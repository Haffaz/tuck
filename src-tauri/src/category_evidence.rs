//! Apple category proposals retain exact quotes and page context through grouping.
use crate::{
    files,
    model::{Preferences, Rule},
    provider::ModelProvider,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    time::Duration,
};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Sample {
    pub id: String,
    pub description: String,
    pub quote: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub source_text: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub page_heads: Vec<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub detail_evidence: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Assignment {
    pub id: String,
    pub category: String,
    #[serde(default, rename = "preserveDepth")]
    pub preserve_depth: usize,
}

// The evaluated reader consumes complete short documents. Do not turn a clipped
// or partially read file into a confident claim about the whole document.
pub fn source_issue(text: &str) -> Option<&'static str> {
    if text.chars().count() > 5000
        || text.contains("[Earlier excerpt]")
        || text.contains("[Text truncated]")
    {
        return Some("This document exceeds the supported reading limit. Choose a folder after reviewing it.");
    }
    if let Some(count) = text
        .lines()
        .next()
        .and_then(|s| s.strip_prefix("[Document has "))
        .and_then(|s| s.strip_suffix(" pages.]"))
        .and_then(|s| s.parse::<usize>().ok())
    {
        let pages: BTreeSet<_> = text
            .lines()
            .filter_map(|s| {
                s.strip_prefix("[Page ")?
                    .strip_suffix(']')?
                    .parse::<usize>()
                    .ok()
            })
            .collect();
        if count == 0 || count > 12 || pages != (1..=count).collect() {
            return Some(
                "Not every page was read. Review the whole document before choosing a folder.",
            );
        }
    }
    if source_lines(text)
        .join("\n")
        .chars()
        .filter(|c| c.is_alphabetic())
        .count()
        < 25
    {
        return Some("There is not enough readable text to identify this document.");
    }
    None
}
pub fn source_lines(text: &str) -> Vec<&str> {
    text.lines()
        .filter(|s| !s.is_empty() && !s.starts_with("[Page ") && !s.starts_with("[Document has "))
        .collect()
}
pub fn page_heads(text: &str) -> Vec<String> {
    let mut pages = Vec::new();
    let mut current: Option<Vec<&str>> = None;
    for line in text.lines() {
        if line.starts_with("[Document has ") {
            continue;
        }
        if line.starts_with("[Page ") {
            if let Some(lines) = current.take() {
                pages.push(lines.join("\n"));
            }
            current = Some(Vec::new());
        } else if let Some(lines) = current.as_mut() {
            lines.push(line);
        }
    }
    if let Some(lines) = current {
        pages.push(lines.join("\n"));
    }
    if pages.len() < 2 {
        return vec![];
    }
    pages
        .into_iter()
        .map(|s| s.trim().chars().take(240).collect())
        .collect()
}
#[cfg(test)]
fn validate_assignments(samples: &[Sample], assignments: &[Assignment]) -> Result<(), String> {
    validate_assignments_with_depth(samples, assignments, 1)
}
pub fn validate_assignments_with_depth(
    samples: &[Sample],
    assignments: &[Assignment],
    max_depth: usize,
) -> Result<(), String> {
    if !(1..=crate::model::MAX_FOLDER_DEPTH).contains(&max_depth) {
        return Err("Unsupported folder nesting level".into());
    }
    let expected: BTreeSet<_> = samples.iter().map(|s| s.id.as_str()).collect();
    let actual: BTreeSet<_> = assignments.iter().map(|a| a.id.as_str()).collect();
    if assignments.len() != samples.len() || expected.len() != samples.len() || actual != expected {
        return Err("The model did not assign every document exactly once. Try again.".into());
    }
    for a in assignments {
        if a.category == "Needs review" {
            continue;
        }
        if a.preserve_depth > max_depth
            || !files::valid_folder(&a.category, max_depth)
            || a.category.trim() != a.category
            || a.category.chars().count() > 120
            || a.category.split('/').any(|part| {
                part.trim() != part
                    || part.chars().count() > 48
                    || crate::categories::generic_category_name(part)
            })
            || crate::categories::generic_category_name(&a.category)
            || [
                "needs review",
                "review",
                "unknown",
                "finance",
                "financial",
                "financial documents",
                "legal",
            ]
            .contains(&a.category.to_lowercase().as_str())
        {
            return Err("The model returned an unsafe or overly broad category. Try again or add a folder yourself.".into());
        }
    }
    Ok(())
}
// Run once after every model batch, before generating folder rules. Only remove
// redundant path components: never invent labels, merge subjects or move review files.
// Three records make a useful automatic subgroup; a split also needs another
// substantial child or at least three records remaining directly in its parent.
pub fn compact_assignments(assignments: &mut [Assignment]) {
    const MIN_GROUP: usize = 3;
    fn compact(parent: &str, indices: &[usize], rows: &mut [Assignment]) {
        let level = parent.split('/').count();
        let mut children: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        let mut direct = 0;
        for &i in indices {
            let parts: Vec<_> = rows[i].category.split('/').collect();
            if parts.len() <= level {
                direct += 1;
            } else {
                children
                    .entry(parts[..=level].join("/"))
                    .or_default()
                    .push(i);
            }
        }
        let substantial = children
            .values()
            .filter(|ids| ids.len() >= MIN_GROUP)
            .count();
        let remaining = direct
            + children
                .values()
                .filter(|ids| ids.len() < MIN_GROUP)
                .map(Vec::len)
                .sum::<usize>();
        let useful_split = substantial + usize::from(remaining >= MIN_GROUP) >= 2;
        for (child, ids) in children {
            let requested = ids.iter().any(|&i| rows[i].preserve_depth > level);
            if requested || (useful_split && ids.len() >= MIN_GROUP) {
                compact(&child, &ids, rows);
            } else {
                for i in ids {
                    rows[i].category = parent.to_owned();
                }
            }
        }
    }
    let mut roots: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i, row) in assignments.iter().enumerate() {
        if row.category != "Needs review" {
            roots
                .entry(row.category.split('/').next().unwrap().to_owned())
                .or_default()
                .push(i);
        }
    }
    for (root, indices) in roots {
        compact(&root, &indices, assignments);
    }
}

pub struct Grouping {
    pub rules: Vec<Rule>,
    pub reviewed: usize,
    pub batched: bool,
    pub assignments: Vec<Assignment>,
}
pub fn group(
    model: &dyn ModelProvider,
    samples: &mut [Sample],
    max_depth: usize,
    preferences: &str,
    mut progress: impl FnMut(String) -> Result<(), String>,
) -> Result<Grouping, String> {
    samples.sort_by(|a, b| a.id.cmp(&b.id));
    let mut batches = Vec::new();
    let mut start = 0;
    let mut size = 0;
    for (index, sample) in samples.iter().enumerate() {
        // Complete source is checked in a separate per-record model call.
        // It does not consume the grouping prompt budget.
        let mut grouping_sample =
            serde_json::to_value(sample).map_err(|_| "Invalid document evidence")?;
        grouping_sample
            .as_object_mut()
            .unwrap()
            .remove("sourceText");
        let len = serde_json::to_string(&grouping_sample)
            .map_err(|_| "Invalid document evidence")?
            .chars()
            .count();
        if len > 6500 {
            return Err("Document evidence exceeds the grouping limit".into());
        }
        if index > start && (index - start >= 4 || size + len > 6500) {
            batches.push(start..index);
            start = index;
            size = 0;
        }
        size += len;
    }
    if start < samples.len() {
        batches.push(start..samples.len());
    }
    let mut assignments = Vec::new();
    let mut subjects = BTreeSet::new();
    for (i, range) in batches.iter().enumerate() {
        progress(format!(
            "Grouping documents · {} of {}",
            i + 1,
            batches.len()
        ))?;
        let chunk = &samples[range.clone()];
        let result = model.group_documents_with_context(
            chunk,
            &subjects.iter().cloned().collect::<Vec<_>>(),
            Duration::from_secs(90),
        )?;
        validate_assignments_with_depth(chunk, &result, max_depth)?;
        for assignment in &result {
            if assignment.category != "Needs review" {
                subjects.insert(assignment.category.split('/').next().unwrap().to_owned());
            }
        }
        if subjects.len() > 40 {
            return Err(
                "This collection needs more than 40 subjects; organize it in smaller groups."
                    .into(),
            );
        }
        assignments.extend(result);
    }
    validate_assignments_with_depth(samples, &assignments, max_depth)?;
    compact_assignments(&mut assignments);
    validate_assignments_with_depth(samples, &assignments, max_depth)?;
    let by_id: BTreeMap<_, _> = samples.iter().map(|s| (s.id.as_str(), s)).collect();
    let mut groups: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut reviewed = 0;
    for a in &assignments {
        if a.category == "Needs review" {
            reviewed += 1;
            continue;
        }
        groups
            .entry(a.category.clone())
            .or_default()
            .insert(by_id[a.id.as_str()].description.clone());
    }
    let rules: Vec<_> = groups
        .into_iter()
        .map(|(folder, descriptions)| {
            // The observed document types define this draft scope. Equivalent type
            // names are allowed; matching an entity alone is insufficient.
            let scope = if folder.contains('/') {
                "Each subgroup must match a distinguishing source fact, such as a named subject, organization, project, product, location or document type. Names require source evidence; document type synonyms are allowed."
            } else {
                "Use a supported more specific child folder when available; otherwise use this broad subject."
            };
            let grouped_samples: Vec<_> = assignments.iter().filter(|a| a.category == folder)
                .map(|a| by_id[a.id.as_str()]).collect();
            let details = subgroup_details(&folder, &grouped_samples, preferences);
            let document_types: Vec<_> = descriptions.into_iter().collect();
            let instruction = format!(
                "File documents about {folder} here. Match the saved document types, including equivalent names. {details}{scope}"
            );
            Rule { document_types,
                folder,
                instruction,
                enabled: true,
            }
        })
        .collect();
    if rules.iter().any(|r| {
        r.instruction.chars().count() > 600 || r.instruction.contains("Required document scope: .")
    }) {
        return Err("These categories include too many distinct document types for a concise rule. Add or edit folders yourself.".into());
    }
    files::validate(&Preferences {
        rules: rules.clone(),
        ..Preferences::default()
    })?;
    Ok(Grouping {
        rules,
        reviewed,
        batched: batches.len() > 1,
        assignments,
    })
}

// Store the source field behind a named subgroup, so a shared person or
// document type cannot substitute for a different project or organization.
fn contains_label(label: &str, text: &str) -> bool {
    let words = |s: &str| {
        s.split(|c: char| !c.is_alphanumeric())
            .filter(|w| !w.is_empty())
            .map(str::to_lowercase)
            .collect::<Vec<_>>()
    };
    let label = words(label);
    let text = words(text);
    !label.is_empty() && text.windows(label.len()).any(|part| part == label)
}
fn subgroup_details(folder: &str, samples: &[&Sample], preferences: &str) -> String {
    let mut conditions = Vec::new();
    for part in folder.split('/').skip(1) {
        let mut facts = BTreeSet::new();
        for sample in samples {
            for line in sample.detail_evidence.lines() {
                let Some((key, value)) = line.split_once(':') else {
                    continue;
                };
                let (key, value) = (key.trim(), value.trim());
                if key.is_empty()
                    || value.is_empty()
                    || key.chars().count() > 40
                    || value.chars().count() > 100
                {
                    continue;
                }
                let alias = preferences
                    .split(['.', '\n'])
                    .any(|clause| contains_label(part, clause) && contains_label(value, clause));
                if contains_label(part, value) || alias {
                    facts.insert(format!("{key}: {value}"));
                }
            }
        }
        if !facts.is_empty() {
            conditions.push(format!(
                "({})",
                facts.into_iter().collect::<Vec<_>>().join(" OR ")
            ));
        }
    }
    if conditions.is_empty() {
        String::new()
    } else {
        format!("Required source details: {}. ", conditions.join(" AND "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample(id: &str) -> Sample {
        Sample {
            detail_evidence: String::new(),
            source_text: String::new(),
            id: id.into(),
            description: "Payslip".into(),
            quote: "Monthly payslip".into(),
            page_heads: vec![],
        }
    }
    #[test]
    fn collection_structure_matches_frozen_examples() {
        #[derive(Deserialize)]
        struct Case {
            name: String,
            groups: Vec<Group>,
        }
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Group {
            path: String,
            count: usize,
            preserve_depth: usize,
            expected: String,
        }
        let cases: Vec<Case> =
            serde_json::from_str(include_str!("../../evals/structure/fixtures.json")).unwrap();
        let mut report = Vec::new();
        for case in cases {
            let mut rows = Vec::new();
            let mut expected = BTreeMap::new();
            for group in case.groups {
                for _ in 0..group.count {
                    let id = format!("d{}", rows.len());
                    expected.insert(id.clone(), group.expected.clone());
                    rows.push(Assignment {
                        id,
                        category: group.path.clone(),
                        preserve_depth: group.preserve_depth,
                    });
                }
            }
            let before: BTreeSet<_> = rows.iter().map(|a| a.category.clone()).collect();
            let mut forward = None;
            for reverse in [false, true] {
                let mut actual = rows.clone();
                if reverse {
                    actual.reverse();
                }
                compact_assignments(&mut actual);
                let paths: BTreeMap<_, _> = actual
                    .iter()
                    .map(|a| (a.id.clone(), a.category.clone()))
                    .collect();
                assert_eq!(paths, expected, "{} reversed={reverse}", case.name);
                compact_assignments(&mut actual);
                assert_eq!(
                    actual
                        .iter()
                        .map(|a| (a.id.clone(), a.category.clone()))
                        .collect::<BTreeMap<_, _>>(),
                    expected,
                    "{} must be stable",
                    case.name
                );
                if !reverse {
                    forward = Some(paths);
                }
            }
            let after: BTreeSet<_> = expected.values().cloned().collect();
            report.push(serde_json::json!({"name":case.name,"documents":rows.len(),"before":before,"after":after,"assignments":forward,"passed":true}));
        }
        if let Ok(path) = std::env::var("TUCK_STRUCTURE_REPORT") {
            std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
        }
        println!(
            "{} frozen structure cases passed in both input orders",
            report.len()
        );
    }

    #[test]
    fn subgroup_conditions_preserve_generic_fields_and_aliases() {
        let project = Sample {
            detail_evidence: "Project: Orion\nSupplier: Harbor Office".into(),
            ..sample("a")
        };
        assert_eq!(
            subgroup_details("Work/Orion/Harbor", &[&project], ""),
            "Required source details: (Project: Orion) AND (Supplier: Harbor Office). "
        );
        assert_eq!(
            subgroup_details(
                "Work/Client One",
                &[&project],
                "Use Client One for project Orion."
            ),
            "Required source details: (Project: Orion). "
        );
        assert_eq!(subgroup_details("Work/Invented", &[&project], ""), "");
        let clinical = Sample {
            detail_evidence: "Clinic: Meadow Clinic".into(),
            ..sample("b")
        };
        assert_eq!(
            subgroup_details("Health/Harbor Clinic", &[&clinical], ""),
            ""
        );
    }
    #[test]
    fn nesting_is_a_ceiling_and_invalid_paths_still_fail() {
        let input = [sample("a")];
        for (path, levels) in [
            ("Income", 1),
            ("Income/Acme", 2),
            ("Income/Acme/Payslips", 3),
            ("Income/Acme/Payslips/October", 4),
        ] {
            let assignments = [Assignment {
                id: "a".into(),
                category: path.into(),
                preserve_depth: 0,
            }];
            for limit in 1..=2 {
                assert_eq!(
                    validate_assignments_with_depth(&input, &assignments, limit).is_ok(),
                    levels <= limit,
                    "{path} at {limit}"
                );
            }
        }
        for path in [
            "Income/../outside",
            "Income/.hidden",
            "Income//Acme",
            "Income/Documents",
            "Income/ Acme",
        ] {
            let assignments = [Assignment {
                id: "a".into(),
                category: path.into(),
                preserve_depth: 0,
            }];
            assert!(
                validate_assignments_with_depth(&input, &assignments, 2).is_err(),
                "{path}"
            );
        }
    }
    #[test]
    fn coverage_and_safe_labels_are_required() {
        let input = vec![sample("a"), sample("b")];
        let a = |id: &str, category: &str| Assignment {
            id: id.into(),
            category: category.into(),
            preserve_depth: 0,
        };
        assert!(validate_assignments(&input, &[a("a", "Income"), a("b", "Needs review")]).is_ok());
        for bad in [
            vec![a("a", "Income")],
            vec![a("a", "Income"), a("a", "Income")],
            vec![a("a", "Income"), a("c", "Income")],
            vec![a("a", "../escape"), a("b", "Income")],
            vec![a("a", "Documents"), a("b", "Income")],
            vec![a("a", "Needs Review"), a("b", "Income")],
        ] {
            assert!(validate_assignments(&input, &bad).is_err());
        }
    }
    #[test]
    fn incomplete_sources_cannot_be_confident_and_page_context_survives() {
        assert!(source_issue(&"word ".repeat(1100)).is_some());
        assert!(source_issue("Readable but clipped document contents. [Text truncated]").is_some());
        assert!(source_issue(
            "[Document has 2 pages.]\n[Page 1]\nA complete readable payslip with monthly salary."
        )
        .is_some());
        let full = "[Document has 2 pages.]\n[Page 1]\nMonthly salary payment statement.\n[Page 2]\nResidence permit decision and approval.";
        assert!(source_issue(full).is_none());
        assert_eq!(
            page_heads(full),
            [
                "Monthly salary payment statement.",
                "Residence permit decision and approval."
            ]
        );
        assert_eq!(source_lines(full).len(), 2);
        assert_eq!(
            page_heads("[Document has 2 pages.]\n\n[Page 1]\n\n[Page 2]\nPayslip"),
            ["", "Payslip"]
        );
        assert!(source_issue("[Document has 1 pages.]\n[Page 1]\n42").is_some());
    }
}
