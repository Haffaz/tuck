//! Filing with document cards: one cached card, then one model choice among a short,
//! embedding-ranked list of enabled folders whose rules are shown to the model.
use crate::{
    cards::Card,
    classifier::{self, Decision},
    cluster,
    model::Preferences,
    provider::{Candidate, ModelProvider},
};
use std::{
    path::Path,
    time::{Duration, Instant},
};

const DOCUMENT_BUDGET: Duration = Duration::from_secs(90);
/// Small enough that every folder rule fits the on-device model's context.
const CHOICES_PER_CALL: usize = 6;
/// Reading input is cheap for the on-device model; generating text is what costs time.
const EXCERPT_CHARS: usize = 1200;

/// A folder's rule together with its parents' rules, which children inherit.
fn candidates(p: &Preferences) -> Vec<Candidate> {
    p.rules
        .iter()
        .filter(|r| p.rule_enabled(r))
        .map(|rule| {
            let mut ancestors: Vec<_> = p
                .rules
                .iter()
                .filter(|r| {
                    r.folder == rule.folder || rule.folder.starts_with(&format!("{}/", r.folder))
                })
                .collect();
            ancestors.sort_by_key(|r| r.folder.split('/').count());
            Candidate {
                folder: rule.folder.clone(),
                instruction: ancestors
                    .iter()
                    .map(|r| r.instruction.trim())
                    .collect::<Vec<_>>()
                    .join(" "),
                document_types: rule.document_types.clone(),
            }
        })
        .collect()
}

/// Orders folders by how close their description is to the card. Rule-text similarity
/// is a weak signal, so it only decides which folders the model compares first.
fn ranked(
    card: &Card,
    candidates: Vec<Candidate>,
    model: &dyn ModelProvider,
    timeout: Duration,
) -> Vec<Candidate> {
    if candidates.len() <= CHOICES_PER_CALL {
        return candidates;
    }
    let texts: Vec<String> = candidates
        .iter()
        .map(|c| {
            let leaf = c.folder.rsplit('/').next().unwrap_or(&c.folder);
            format!("{leaf}. {} {}", c.instruction, c.document_types.join(", "))
        })
        .collect();
    let Ok(vectors) = model.embed(&texts, timeout) else {
        return candidates;
    };
    let mut scored: Vec<(f64, Candidate)> = vectors
        .iter()
        .map(|v| cluster::cosine(&card.embedding, v))
        .zip(candidates)
        .collect();
    scored.sort_by(|a, b| b.0.total_cmp(&a.0));
    scored.into_iter().map(|(_, c)| c).collect()
}

/// Words of `value` that appear in the document; the rest are dropped.
fn grounded(value: &str, text: &str) -> bool {
    let text = text.to_lowercase();
    let words: Vec<_> = value
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.chars().count() >= 2)
        .collect();
    !words.is_empty() && words.iter().all(|w| text.contains(&w.to_lowercase()))
}

/// "Bank Statement - Nordea Bank Abp - 2026-03". Issuer and date are included only when
/// the document itself contains them, so a model cannot invent names into filenames.
pub fn title(card: &Card, text: &str) -> String {
    let kind = card
        .kind
        .split_whitespace()
        .map(|w| {
            let mut c = w.chars();
            c.next()
                .map_or_else(String::new, |f| f.to_uppercase().chain(c).collect())
        })
        .collect::<Vec<_>>()
        .join(" ");
    let mut parts = vec![kind];
    if grounded(&card.issuer, text) {
        parts.push(card.issuer.trim().to_owned());
    }
    if card.date.len() >= 4 && text.contains(&card.date[..4]) {
        parts.push(card.date.clone());
    }
    parts.join(" - ")
}

pub fn run(text: &str, p: &Preferences, path: &Path, model: &dyn ModelProvider) -> Decision {
    let _serial = crate::coordinator::serial();
    let started = Instant::now();
    let remaining = || {
        DOCUMENT_BUDGET
            .checked_sub(started.elapsed())
            .filter(|d| !d.is_zero())
            .ok_or_else(|| "Document time limit reached".to_string())
    };
    let result = (|| -> Result<Decision, String> {
        let card = crate::cards::describe(Path::new(&p.root), path, text, model, remaining()?)?;
        let ordered = ranked(&card, candidates(p), model, remaining()?);
        let excerpt: String = text.chars().take(EXCERPT_CHARS).collect();
        for batch in ordered.chunks(CHOICES_PER_CALL) {
            if let Some(index) =
                model.choose_folder(&card, &excerpt, batch, &p.instructions, remaining()?)?
            {
                let folder = batch[index].folder.clone();
                return Ok(Decision {
                    language: None,
                    reason: format!("{} matches {folder}.", title(&card, text)),
                    folder,
                    title: if p.rename {
                        title(&card, text)
                    } else {
                        String::new()
                    },
                    confident: true,
                });
            }
        }
        Err(format!(
            "No folder fits this {}. Choose a folder for it.",
            card.kind
        ))
    })();
    classifier::finalize(result, p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Rule;
    use std::{fs, sync::Mutex};

    struct Chooser {
        pick: Option<&'static str>,
        seen: Mutex<Vec<Vec<String>>>,
    }
    impl ModelProvider for Chooser {
        fn status(&self) -> Result<String, String> {
            Ok("ok".into())
        }
        fn suggest_rule(&self, _: &str, _: &str) -> Result<String, String> {
            unreachable!()
        }
        fn identity(&self) -> Result<String, String> {
            Ok("chooser".into())
        }
        fn name_group(&self, _: &[&Card], _: &str, _: Duration) -> Result<String, String> {
            unreachable!()
        }
        fn group_subjects(
            &self,
            _: &[String],
            _: &[String],
            _: Duration,
        ) -> Result<Vec<String>, String> {
            unreachable!()
        }
        fn merge_names(&self, _: &[String], _: Duration) -> Result<Vec<(String, String)>, String> {
            unreachable!()
        }
        fn card(&self, _: &str, _: &str, _: Duration) -> Result<Card, String> {
            Ok(Card {
                kind: "electricity bill".into(),
                issuer: "Vattenfall".into(),
                date: "2026-03".into(),
                tags: vec!["energy".into()],
                embedding: vec![1.0, 0.0],
            })
        }
        fn embed(&self, texts: &[String], _: Duration) -> Result<Vec<Vec<f64>>, String> {
            Ok(texts
                .iter()
                .map(|t| {
                    if t.starts_with("Home") {
                        vec![1.0, 0.0]
                    } else {
                        vec![0.0, 1.0]
                    }
                })
                .collect())
        }
        fn choose_folder(
            &self,
            _: &Card,
            _: &str,
            candidates: &[Candidate],
            _: &str,
            _: Duration,
        ) -> Result<Option<usize>, String> {
            self.seen
                .lock()
                .unwrap()
                .push(candidates.iter().map(|c| c.folder.clone()).collect());
            Ok(self
                .pick
                .and_then(|f| candidates.iter().position(|c| c.folder == f)))
        }
    }

    fn rule(folder: &str, instruction: &str) -> Rule {
        Rule {
            document_types: Vec::new(),
            folder: folder.into(),
            instruction: instruction.into(),
            enabled: true,
        }
    }

    fn file(model: &Chooser, rules: Vec<Rule>) -> Decision {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("scan.txt");
        let text = "Vattenfall AB invoice for electricity, March 2026";
        fs::write(&path, text).unwrap();
        let p = Preferences {
            root: root.path().to_string_lossy().into(),
            rules,
            ..Preferences::default()
        };
        run(text, &p, &path, model)
    }

    #[test]
    fn the_chosen_folder_is_filed_with_a_grounded_title() {
        let model = Chooser {
            pick: Some("Home/Utilities"),
            seen: Mutex::new(vec![]),
        };
        let decision = file(
            &model,
            vec![
                rule("Finances", "Bank records."),
                rule("Home", "Housing."),
                rule("Home/Utilities", "Utility bills."),
            ],
        );
        assert!(decision.confident, "{}", decision.reason);
        assert_eq!(decision.folder, "Home/Utilities");
        assert_eq!(decision.title, "Electricity Bill - Vattenfall - 2026-03");
    }

    #[test]
    fn no_fit_needs_review() {
        let model = Chooser {
            pick: None,
            seen: Mutex::new(vec![]),
        };
        let decision = file(&model, vec![rule("Finances", "Bank records.")]);
        assert!(!decision.confident);
        assert_eq!(decision.folder, "Needs Review");
        assert!(decision.reason.contains("electricity bill"));
    }

    #[test]
    fn many_folders_are_compared_in_ranked_batches() {
        let model = Chooser {
            pick: Some("Work 7"),
            seen: Mutex::new(vec![]),
        };
        let mut rules: Vec<_> = (1..=8)
            .map(|i| rule(&format!("Work {i}"), "Work papers."))
            .collect();
        rules.push(rule("Home", "Utility bills."));
        let decision = file(&model, rules);
        assert_eq!(decision.folder, "Work 7");
        let seen = model.seen.lock().unwrap();
        assert_eq!(seen.len(), 2);
        assert_eq!(seen[0][0], "Home", "closest folder is compared first");
        assert!(seen.iter().all(|batch| batch.len() <= CHOICES_PER_CALL));
    }

    #[test]
    fn children_inherit_parent_rules_and_disabled_folders_are_skipped() {
        let mut off = rule("Old", "Archive.");
        off.enabled = false;
        let p = Preferences {
            rules: vec![
                rule("Home", "Housing."),
                rule("Home/Utilities", "Utility bills."),
                off,
            ],
            ..Preferences::default()
        };
        let list = candidates(&p);
        assert_eq!(list.len(), 2);
        assert_eq!(list[1].instruction, "Housing. Utility bills.");
    }

    #[test]
    fn titles_drop_details_missing_from_the_document() {
        let card = Card {
            kind: "payslip".into(),
            issuer: "Acme AB".into(),
            date: "2031-01".into(),
            ..Card::default()
        };
        assert_eq!(title(&card, "Salary from Acme AB"), "Payslip - Acme AB");
        assert_eq!(title(&card, "Salary 2031"), "Payslip - 2031-01");
    }
}
