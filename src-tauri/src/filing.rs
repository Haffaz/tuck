//! Filing one document: its cached card, then one model choice among a short,
//! embedding-ranked list of enabled folders whose rules are shown to the model.
//! Anything uncertain becomes a `NEEDS_REVIEW` decision; Rust still checks every move.
use crate::{
    cards::{title_case, Card},
    cluster, files,
    model::Preferences,
    provider::{Candidate, ModelProvider},
};
use serde::{Deserialize, Serialize};
use std::{
    path::Path,
    time::{Duration, Instant},
};

/// The folder name a decision carries when it needs the user.
pub const NEEDS_REVIEW: &str = "Needs Review";

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct Decision {
    pub folder: String,
    pub title: String,
    pub reason: String,
    pub confident: bool,
}

/// Decides where a document goes. `source` is the document and the reader binary, whose
/// sibling is the model helper; without it nothing can be decided.
pub fn decide(
    text: &str,
    clues: &[String],
    p: &Preferences,
    source: Option<(&Path, &Path)>,
) -> Decision {
    if !p.rules.iter().any(|r| p.rule_enabled(r)) {
        return review("All folders are turned off. Turn a folder on to file documents.");
    }
    if !p.use_ai {
        return review("The AI model is off. Turn it on in Settings to file documents.");
    }
    let Some((path, reader)) = source else {
        return review("The model helper is unavailable.");
    };
    run(
        text,
        clues,
        p,
        path,
        &crate::provider::configured(reader, p),
    )
}

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
    let mut parts = vec![title_case(&card.kind)];
    if grounded(&card.issuer, text) {
        parts.push(card.issuer.trim().to_owned());
    }
    if card.date.len() >= 4 && text.contains(&card.date[..4]) {
        parts.push(card.date.clone());
    }
    parts.join(" - ")
}

fn run(
    text: &str,
    clues: &[String],
    p: &Preferences,
    path: &Path,
    model: &dyn ModelProvider,
) -> Decision {
    let _serial = crate::model_lock::hold();
    let started = Instant::now();
    let remaining = || {
        DOCUMENT_BUDGET
            .checked_sub(started.elapsed())
            .filter(|d| !d.is_zero())
            .ok_or_else(|| "Document time limit reached".to_string())
    };
    let result = (|| -> Result<Decision, String> {
        let card =
            crate::cards::describe(Path::new(&p.root), path, text, clues, model, remaining()?)?;
        let ordered = ranked(&card, candidates(p), model, remaining()?);
        let excerpt: String = text.chars().take(EXCERPT_CHARS).collect();
        for batch in ordered.chunks(CHOICES_PER_CALL) {
            if let Some(index) =
                model.choose_folder(&card, &excerpt, batch, &p.instructions, remaining()?)?
            {
                let folder = batch[index].folder.clone();
                return Ok(Decision {
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
    finalize(result, p)
}

pub(crate) fn finalize(result: Result<Decision, String>, p: &Preferences) -> Decision {
    match result {
        Ok(mut decision) => {
            if !decision.confident {
                return review(&format!("The model was not sure. {}", decision.reason));
            }
            if !p
                .rules
                .iter()
                .any(|r| p.rule_enabled(r) && r.folder == decision.folder)
            {
                return review("The model suggested a folder outside your enabled folders.");
            }
            decision.title = files::safe_stem(&decision.title);
            decision
        }
        Err(error) => review(&error),
    }
}
fn review(reason: &str) -> Decision {
    Decision {
        folder: NEEDS_REVIEW.into(),
        title: String::new(),
        reason: reason.into(),
        confident: false,
    }
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
        fn card(&self, _: &str, _: &str, _: &[String], _: Duration) -> Result<Card, String> {
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
        run(text, &[], &p, &path, model)
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
        assert_eq!(decision.folder, NEEDS_REVIEW);
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
    #[test]
    fn disabled_ai_and_disabled_instructions_require_review() {
        let mut p = Preferences {
            use_ai: false,
            ..Preferences::default()
        };
        let d = classify("bank statement", &p);
        assert_eq!(d.folder, NEEDS_REVIEW);
        assert!(d.title.is_empty());
        assert!(d.reason.contains("off"));
        p.use_ai = true;
        for rule in &mut p.rules {
            rule.enabled = false;
        }
        let d = classify("insurance policy", &p);
        assert_eq!(d.folder, NEEDS_REVIEW);
        assert!(d.reason.contains("turned off"));
    }
    #[test]
    fn ai_cannot_route_to_disabled_folder() {
        let mut p = Preferences::default();
        p.rules
            .iter_mut()
            .find(|r| r.folder == "Insurance")
            .unwrap()
            .enabled = false;
        let suggestion = Decision {
            folder: "Insurance".into(),
            title: "Policy".into(),
            reason: "AI suggestion".into(),
            confident: true,
        };
        assert_eq!(finalize(Ok(suggestion), &p).folder, NEEDS_REVIEW);
    }
    #[test]
    fn uncertain_decisions_keep_original_names_and_require_review() {
        let p = Preferences::default();
        let uncertain = Decision {
            folder: "Home".into(),
            title: "Made up name".into(),
            reason: "Not sure".into(),
            confident: false,
        };
        let decision = finalize(Ok(uncertain), &p);
        assert_eq!(decision.folder, NEEDS_REVIEW);
        assert!(decision.title.is_empty());
        assert!(!decision.confident);
    }
    #[test]
    #[ignore = "Requires Apple Intelligence enabled; uses only synthetic documents"]
    fn live_apple_filing_smoke() {
        let reader = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/tuck-reader");
        println!(
            "{}",
            crate::helper::check(&reader.with_file_name("tuck-apple")).unwrap()
        );
        let dir = tempfile::tempdir_in("/private/tmp").unwrap();
        let path = dir.path().join("scan.txt");
        let p = Preferences {
            root: dir.path().to_string_lossy().into_owned(),
            ..Preferences::default()
        };
        for (text, expected) in [
            (
                "KONTOUTDRAG. Banken. Transaktioner: lön in, hyra ut. Saldo 100 kronor.",
                "Banking & Taxes",
            ),
            (
                "This certificate confirms cover for damage to your building and its contents.",
                "Insurance",
            ),
            ("[unreadable scan; no recognizable text]", NEEDS_REVIEW),
        ] {
            fs::write(&path, text).unwrap();
            let started = std::time::Instant::now();
            let d = decide(text, &[], &p, Some((&path, &reader)));
            println!(
                "Apple synthetic: {:.2}s {}",
                started.elapsed().as_secs_f64(),
                serde_json::to_string(&d).unwrap()
            );
            assert_eq!(d.folder, expected);
            if expected != NEEDS_REVIEW {
                assert!(d.confident);
                // Naming is now independent: a refused/uncertain naming call
                // must retain the original filename without blocking a valid folder.
            }
        }
    }

    #[test]
    fn model_failure_requires_review_without_keyword_override() {
        let p = Preferences::default();
        let d = finalize(Err("Apple is unavailable".into()), &p);
        assert_eq!(d.folder, NEEDS_REVIEW);
        assert!(d.title.is_empty());
        assert!(d.reason.contains("unavailable"));
    }

    fn classify(text: &str, p: &Preferences) -> Decision {
        decide(text, &[], p, None)
    }
}
