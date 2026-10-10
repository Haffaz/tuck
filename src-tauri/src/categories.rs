//! Bounded category discovery; only rebuildable descriptions are cached. Folder creation uses the existing reviewed settings save.
use crate::{
    files,
    model::{Preferences, Rule},
    provider::ModelProvider,
};
use serde::Serialize;
#[cfg(test)]
use serde_json::json;
use serde_json::Value;
use std::{path::Path, time::Duration};
const MAX_FILES: usize = 32;
const MAX_DESCRIPTION_CHARACTERS: usize = 6000;

// Keep ordinary collections together: repeated summarization can erase minority
// subjects. Large collections remain bounded by both count and text size.
fn description_batches(samples: &[String]) -> Vec<&[String]> {
    let mut batches = Vec::new();
    let mut start = 0;
    let mut characters = 0;
    for (index, sample) in samples.iter().enumerate() {
        let size = sample.chars().count();
        if index > start
            && (index - start == MAX_FILES || characters + size > MAX_DESCRIPTION_CHARACTERS)
        {
            batches.push(&samples[start..index]);
            start = index;
            characters = 0;
        }
        characters += size;
    }
    if start < samples.len() {
        batches.push(&samples[start..]);
    }
    batches
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewFile {
    pub path: String,
    pub reason: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Proposal {
    pub rules: Vec<Rule>,
    #[cfg(test)]
    pub assignments: Vec<crate::category_evidence::Assignment>,
    pub sampled: usize,
    pub skipped: usize,
    pub review_files: Vec<ReviewFile>,
    pub limited: bool,
    pub preferences: Preferences,
}
// Apply only to AI suggestions: existing/user-chosen names remain valid.
pub(crate) fn generic_category_name(name: &str) -> bool {
    const GENERIC: &[&str] = &[
        "a",
        "an",
        "the",
        "and",
        "or",
        "my",
        "all",
        "of",
        "document",
        "documents",
        "documentation",
        "file",
        "files",
        "record",
        "records",
        "paper",
        "papers",
        "paperwork",
        "item",
        "items",
        "stuff",
        "general",
        "other",
        "misc",
        "miscellaneous",
        "personal",
        "important",
        "official",
        "assorted",
        "uncategorized",
        "unclassified",
        "archive",
        "archives",
        "archived",
        "certificate",
        "certificates",
        "form",
        "forms",
        "letter",
        "letters",
    ];
    name.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .all(|s| GENERIC.contains(&s))
}

#[cfg(test)]
fn validate_rules(value: Value) -> Result<Vec<Rule>, String> {
    validate_rules_with_depth(value, 1)
}
pub(crate) fn validate_rules_with_depth(
    value: Value,
    max_depth: usize,
) -> Result<Vec<Rule>, String> {
    if !(1..=crate::model::MAX_FOLDER_DEPTH).contains(&max_depth) {
        return Err("Unsupported folder nesting level".into());
    }
    let rules: Vec<Rule> = serde_json::from_value(value)
        .map_err(|_| "The model returned invalid category suggestions")?;
    if rules.is_empty() || rules.len() > 6 {
        return Err("The model must suggest between one and six categories".into());
    }
    for rule in &rules {
        if !files::valid_folder(&rule.folder, max_depth)
            || rule.folder.trim() != rule.folder
            || rule.folder.chars().count() > 120
            || rule.folder.split('/').any(|part| {
                part.trim() != part || part.chars().count() > 40 || generic_category_name(part)
            })
            || generic_category_name(&rule.folder)
            || !rule.enabled
            || rule.instruction.trim().is_empty()
            || rule.instruction.chars().count() > 600
        {
            return Err("The model returned an invalid category name or rule. Try again or add a rule yourself.".into());
        }
    }
    files::validate(&Preferences {
        rules: rules.clone(),
        ..Preferences::default()
    })?;
    Ok(rules)
}
// Validate at the shared boundary so future adapters cannot bypass safe category paths.
fn validated_categories(
    model: &dyn ModelProvider,
    samples: &[String],
    max_depth: usize,
) -> Result<Vec<Rule>, String> {
    let rules = model.categories(samples, Duration::from_secs(90))?;
    validate_rules_with_depth(
        serde_json::to_value(rules).map_err(|_| "Invalid categories")?,
        max_depth,
    )
}
/// Every readable mapped document contributes. Reduce small batches to fit the local model.
pub fn suggest_map(
    p: &Preferences,
    reader: &Path,
    map: &crate::collection::Collection,
    progress: impl FnMut(String) -> Result<(), String>,
) -> Result<Proposal, String> {
    suggest_map_with_provider(
        p,
        reader,
        map,
        progress,
        &crate::provider::configured(reader, p),
    )
}
fn suggest_map_with_provider(
    p: &Preferences,
    reader: &Path,
    map: &crate::collection::Collection,
    mut progress: impl FnMut(String) -> Result<(), String>,
    model: &dyn ModelProvider,
) -> Result<Proposal, String> {
    let _serial = crate::coordinator::serial();
    let mut samples = Vec::new();
    let mut evidence_samples = Vec::new();
    let mut ordered: Vec<_> = map.documents.iter().collect();
    ordered.sort_by(|a, b| a.path.cmp(&b.path));
    let mut skipped = map.unreadable;
    let mut review_files: Vec<_> = ordered
        .iter()
        .filter_map(|doc| {
            doc.error.as_ref().map(|reason| ReviewFile {
                path: doc.path.clone(),
                reason: reason.clone(),
            })
        })
        .collect();
    let mut first_error = None;
    for (index, doc) in ordered.iter().enumerate() {
        if doc.error.is_some() {
            continue;
        }
        progress(format!(
            "Understanding document {} of {}",
            index + 1,
            map.documents.len()
        ))?;
        let path = Path::new(&doc.path);
        if files::digest(path).ok().as_ref() != Some(&doc.hash) {
            return Err(
                "A document changed. Map the folder again before suggesting categories.".into(),
            );
        }
        match crate::description::ReaderSession::new(
            Path::new(&p.root),
            path,
            reader,
            &doc.text,
            p.use_agent,
            model,
        )
        .and_then(|mut r| r.describe(Duration::from_secs(90)))
        {
            Ok(description) if description.grounded() => {
                if model.uses_evidence_categories() {
                    evidence_samples.push(crate::category_evidence::Sample {
                        id: format!("d{index:04}"),
                        source_text: doc.text.clone(),
                        detail_evidence: if p.folder_nesting >= 2 {
                            description
                                .evidence
                                .iter()
                                .skip(1)
                                .map(|e| e.quote.as_str())
                                .collect::<Vec<_>>()
                                .join("\n")
                        } else {
                            String::new()
                        },
                        description: description.description,
                        quote: description.evidence[0].quote.clone(),
                        page_heads: crate::category_evidence::page_heads(&doc.text),
                    });
                } else {
                    samples.push(description.description);
                }
            }
            Err(error) => {
                review_files.push(ReviewFile {
                    path: doc.path.clone(),
                    reason: error.clone(),
                });
                first_error.get_or_insert(error);
                skipped += 1;
            }
            Ok(description) => {
                review_files.push(ReviewFile {
                    path: doc.path.clone(),
                    reason: if description.uncertainty.is_empty() {
                        "The document type could not be confirmed.".into()
                    } else {
                        description.uncertainty
                    },
                });
                skipped += 1;
            }
        }
    }

    for doc in &map.documents {
        if doc.error.is_none()
            && files::digest(Path::new(&doc.path)).ok().as_ref() != Some(&doc.hash)
        {
            return Err("A document changed during understanding. Map the folder again.".into());
        }
    }
    progress("Grouping your documents".into())?;
    if model.uses_evidence_categories() {
        if evidence_samples.is_empty() {
            return Err(first_error.unwrap_or_else(|| "No documents could be described confidently. Review the files or add folders yourself.".into()));
        }
        let grouped = crate::category_evidence::group(
            model,
            &mut evidence_samples,
            p.folder_nesting as usize,
            &p.category_instructions,
            &mut progress,
        )?;
        // Check cancellation and source identity after the model returns as well.
        progress("Preparing suggested folders".into())?;
        for doc in &map.documents {
            if doc.error.is_none()
                && files::digest(Path::new(&doc.path)).ok().as_ref() != Some(&doc.hash)
            {
                return Err("A document changed during grouping. Map the folder again.".into());
            }
        }
        for assignment in &grouped.assignments {
            if assignment.category == "Needs review" {
                if let Some(doc) = assignment
                    .id
                    .strip_prefix('d')
                    .and_then(|id| id.parse::<usize>().ok())
                    .and_then(|index| ordered.get(index))
                {
                    review_files.push(ReviewFile { path: doc.path.clone(), reason: "No supported folder suggestion could be confirmed. Review this document before choosing a folder.".into() });
                }
            }
        }
        return Ok(Proposal {
            review_files,
            sampled: grouped.assignments.len() - grouped.reviewed,
            skipped: skipped + grouped.reviewed,
            rules: grouped.rules,
            limited: grouped.batched,
            preferences: p.clone(),
            #[cfg(test)]
            assignments: grouped.assignments,
        });
    }
    let sampled = samples.len();
    if samples.is_empty() {
        return Err(first_error.unwrap_or_else(|| "No documents could be described confidently. Check Apple Intelligence, or add folder rules yourself.".into()));
    }
    let mut limited = false;
    loop {
        let batches = description_batches(&samples);
        if batches.len() <= 1 {
            break;
        }
        limited = true;
        let mut reduced = Vec::new();
        for chunk in batches {
            let rules = validated_categories(model, chunk, p.folder_nesting as usize)?;
            for rule in rules {
                reduced.push(format!("{}: {}", rule.folder, rule.instruction));
            }
        }
        if reduced.len() >= samples.len() {
            return Err("The collection could not be grouped within the category limit. Try again or choose categories yourself.".into());
        }
        samples = reduced;
    }
    let rules = validated_categories(model, &samples, p.folder_nesting as usize)?;
    Ok(Proposal {
        #[cfg(test)]
        assignments: vec![],
        rules,
        sampled,
        skipped,
        review_files,
        limited,
        preferences: p.clone(),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    #[test]
    fn batches_keep_small_collections_together_and_bound_large_inputs() {
        let short: Vec<_> = (0..25)
            .map(|n| format!("Document {n} about a distinct subject."))
            .collect();
        assert_eq!(description_batches(&short).len(), 1);
        for samples in [vec!["x".repeat(1200); 17], vec!["short".into(); 70]] {
            let batches = description_batches(&samples);
            assert!(batches.iter().all(|b| b.len() <= MAX_FILES
                && b.iter().map(|s| s.chars().count()).sum::<usize>()
                    <= MAX_DESCRIPTION_CHARACTERS));
            assert_eq!(
                batches.into_iter().flatten().cloned().collect::<Vec<_>>(),
                samples
            );
        }
    }
    #[test]
    fn generated_categories_need_subjects_but_subject_qualified_names_are_allowed() {
        for name in [
            "Documents",
            "PERSONAL DOCUMENTS",
            "General & Other",
            "Official Records",
            "Misc.",
            "Certificates",
        ] {
            assert!(
                validate_rules(json!([{"folder":name,"instruction":"All documents."}])).is_err()
            );
        }
        for name in [
            "Travel",
            "Identification",
            "Medical Records",
            "Employment Documents",
            "Education Certificates",
        ] {
            assert!(validate_rules(
                json!([{"folder":name,"instruction":"Documents on this subject."}])
            )
            .is_ok());
        }
        // This validation belongs to suggestions, not preferences or file operations.
        assert!(files::validate(&Preferences {
            rules: vec![Rule {
                document_types: Vec::new(),
                folder: "Documents".into(),
                instruction: "User choice".into(),
                enabled: true
            }],
            ..Preferences::default()
        })
        .is_ok());
    }
    #[test]
    fn legacy_plans_obey_the_same_nesting_ceiling() {
        for (path, levels) in [
            ("Income", 1),
            ("Income/Acme", 2),
            ("Income/Acme/Payslips", 3),
            ("Income/Acme/Payslips/October", 4),
        ] {
            for limit in 1..=2 {
                let rules = json!([{"folder":path,"instruction":"Salary records."}]);
                assert_eq!(
                    validate_rules_with_depth(rules, limit).is_ok(),
                    levels <= limit
                );
            }
        }
    }
    #[test]
    fn category_names_cannot_escape_or_collide() {
        for folders in [
            vec!["../outside"],
            vec![".tuck"],
            vec!["Home/Rent"],
            vec!["Home", "home"],
        ] {
            let rules: Vec<_> = folders
                .iter()
                .map(|f| json!({"folder":f,"instruction":"Keep housing documents here."}))
                .collect();
            assert!(validate_rules(json!(rules)).is_err());
        }
        assert!(validate_rules(json!([])).is_err());
        assert!(validate_rules(
            json!([{"folder":"Home","instruction":"Keep rental agreements here."}])
        )
        .is_ok());
    }
    #[test]
    fn mapping_skips_links_and_hidden_metadata_but_includes_deep_documents() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        fs::write(root.join("letter.txt"), "Synthetic letter").unwrap();
        fs::create_dir(root.join(".tuck")).unwrap();
        fs::write(root.join(".tuck/instructions.md"), "Private rules").unwrap();
        fs::create_dir_all(root.join("one/two/three")).unwrap();
        fs::write(root.join("one/two/three/deep.txt"), "Too deep").unwrap();
        std::os::unix::fs::symlink(root.join("letter.txt"), root.join("linked.txt")).unwrap();
        let root = root.canonicalize().unwrap();
        let paths = files::existing_documents(&root, true).unwrap();
        assert_eq!(
            paths,
            vec![root.join("letter.txt"), root.join("one/two/three/deep.txt")]
        );
        assert!(!root.join("Finances").exists());
    }
}
