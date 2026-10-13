//! Folder suggestions. Only rebuildable cards are cached; folders are created later through the reviewed settings save.
#[cfg(test)]
use crate::files;
use crate::{
    model::{Preferences, Rule},
    provider::ModelProvider,
};
use serde::{Deserialize, Serialize};
use std::path::Path;
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewFile {
    pub path: String,
    pub reason: String,
}

#[cfg(test)]
#[derive(Serialize, Debug, Clone)]
pub struct Assignment {
    pub id: String,
    pub category: String,
}
/// Where a suggestion puts one document. `title` is empty when renaming is off.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Placement {
    pub path: String,
    pub folder: String,
    pub title: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Proposal {
    pub rules: Vec<Rule>,
    /// Every document that got a folder; the rest are in `review_files`.
    pub files: Vec<Placement>,
    /// Evaluation hook: `d{index}` (path order, all mapped documents) to folder or "Needs review".
    #[cfg(test)]
    pub assignments: Vec<Assignment>,
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

/// Every readable mapped document contributes a card; similar cards become folders.
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
    _reader: &Path,
    map: &crate::collection::Collection,
    progress: impl FnMut(String) -> Result<(), String>,
    model: &dyn ModelProvider,
) -> Result<Proposal, String> {
    crate::card_suggest::suggest(p, map, progress, model)
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
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
            assert!(generic_category_name(name), "{name}");
        }
        for name in [
            "Travel",
            "Identification",
            "Medical Records",
            "Employment Documents",
            "Education Certificates",
        ] {
            assert!(!generic_category_name(name), "{name}");
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
