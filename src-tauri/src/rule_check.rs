//! Exact, scoped rule checks. Broad semantic auditing remains unshipped after live failures.
use crate::model::Preferences;
use serde::Serialize;

#[derive(Clone, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RuleIssue {
    pub folder: String,
    pub other_folder: String,
    pub issue: String,
    pub clarification: String,
}

pub fn check(p: &Preferences) -> Vec<RuleIssue> {
    let normalize = |text: &str| {
        text.split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase()
    };
    let rules: Vec<_> = p.rules.iter().filter(|r| p.rule_enabled(r)).collect();
    let mut issues = Vec::new();
    for (index, rule) in rules.iter().enumerate() {
        for other in rules.iter().skip(index + 1) {
            // Identical children in different branches can inherit different meanings.
            if rule.folder.rsplit_once('/').map(|(parent, _)| parent)
                != other.folder.rsplit_once('/').map(|(parent, _)| parent)
            {
                continue;
            }
            if !rule.instruction.trim().is_empty()
                && normalize(&rule.instruction) == normalize(&other.instruction)
                && crate::model::same_document_types(&rule.document_types, &other.document_types)
            {
                issues.push(RuleIssue { folder: rule.folder.clone(), other_folder: other.folder.clone(), issue: "These folders have the same description. Add a distinction to their rules so Tuck can tell them apart.".into(), clarification: String::new() });
            }
        }
    }
    issues
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Rule;
    fn rule(folder: &str, instruction: &str, enabled: bool) -> Rule {
        Rule {
            document_types: Vec::new(),
            folder: folder.into(),
            instruction: instruction.into(),
            enabled,
        }
    }
    #[test]
    fn distinct_saved_types_disambiguate_identical_descriptions() {
        let mut p = Preferences {
            rules: vec![
                rule("School", "Keep accepted records here.", true),
                rule("Research", "Keep accepted records here.", true),
            ],
            ..Preferences::default()
        };
        p.rules[0].document_types = vec!["Academic transcript".into()];
        p.rules[1].document_types = vec!["Research report".into()];
        assert!(check(&p).is_empty());
        p.rules[1].document_types = vec!["ACADEMIC  transcript".into()];
        assert_eq!(check(&p).len(), 1);
    }

    #[test]
    fn duplicates_are_scoped_and_disabled_branches_are_excluded() {
        let mut p = Preferences {
            rules: vec![
                rule("Work", "Keep documents here.", true),
                rule("Home", "KEEP  documents\nhere.", true),
            ],
            ..Preferences::default()
        };
        assert_eq!(check(&p).len(), 1);
        p.rules[1].instruction = "Keep rental agreements here.".into();
        assert!(check(&p).is_empty());
        p.rules = vec![
            rule("Work", "Work context", false),
            rule("Work/Receipts", "Keep receipts here.", true),
            rule("Work/Invoices", "Keep receipts here.", true),
            rule("Home/Receipts", "Keep receipts here.", true),
        ];
        assert!(check(&p).is_empty());
        p.rules[0].enabled = true;
        let issues = check(&p);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].folder, "Work/Receipts");
        assert_eq!(issues[0].other_folder, "Work/Invoices");
    }
}
