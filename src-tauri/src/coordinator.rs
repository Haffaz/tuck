//! Deterministic orchestration: reader -> matcher -> verifier -> optional naming. One document at a time.
use crate::{
    classifier::{self, Decision},
    description::ReaderSession,
    model::Preferences,
    provider::{Destination, MatchInput, MatchReply, ModelProvider, NameInput},
};
use std::{
    path::Path,
    sync::{Mutex, MutexGuard},
    time::{Duration, Instant},
};
static PROCESSING: Mutex<()> = Mutex::new(());
/// Serialize entire workflows, including planner calls, across entry points.
pub(crate) fn serial() -> MutexGuard<'static, ()> {
    PROCESSING
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
const DOCUMENT_BUDGET: Duration = Duration::from_secs(90);
fn quote_present(quote: &str, text: &str) -> bool {
    quote.trim().chars().count() >= 5 && text.contains(quote)
}
fn normalize(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// A title may use the translated description, exact source facts and an explicit
/// literal prefix. A model cannot add a new identity/date merely by returning it.
fn supported_title(
    title: &str,
    understanding: &crate::description::Understanding,
    rules: &str,
) -> bool {
    let words = |text: &str| {
        text.split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty())
            .map(str::to_lowercase)
            .collect::<Vec<_>>()
    };
    let mut allowed = words(&understanding.description);
    for evidence in &understanding.evidence {
        allowed.extend(words(&evidence.quote));
    }
    let lower = rules.to_ascii_lowercase();
    let mut prefix_words = 0;
    for marker in [
        "start with",
        "starting with",
        "begin with",
        "beginning with",
        "prefixed with",
        "prefix filenames with",
    ] {
        if let Some(index) = lower.find(marker) {
            let tail = rules[index + marker.len()..].trim_start();
            let literal =
                if let Some(quote) = tail.chars().next().filter(|c| matches!(c, '"' | '“')) {
                    let end = if quote == '“' { '”' } else { '"' };
                    tail[quote.len_utf8()..].split(end).next().unwrap_or("")
                } else {
                    tail.split_whitespace().next().unwrap_or("")
                };
            if literal.chars().count() <= 40 {
                let prefix = words(literal);
                prefix_words = prefix.len();
                allowed.extend(prefix);
            }
            break;
        }
    }
    let proposed = words(title);
    !proposed.is_empty()
        && proposed.len() <= 6 + prefix_words
        && proposed.iter().all(|word| allowed.contains(word))
}

pub fn run(
    name: &str,
    text: &str,
    p: &Preferences,
    path: &Path,
    reader: &Path,
    model: &dyn ModelProvider,
) -> Decision {
    let _serial = serial();
    let started = Instant::now();
    let remaining = || {
        DOCUMENT_BUDGET
            .checked_sub(started.elapsed())
            .filter(|d| !d.is_zero())
            .ok_or("Document time limit reached")
    };
    let result = (|| -> Result<Decision, String> {
        let mut reader_role =
            ReaderSession::new(Path::new(&p.root), path, reader, text, p.use_agent, model)?;
        let mut understanding = reader_role.describe(remaining()?)?;
        let destinations: Vec<_> = p
            .rules
            .iter()
            .filter(|r| p.rule_enabled(r))
            .map(|r| Destination {
                document_types: r.document_types.clone(),
                folder: r.folder.clone(),
                instruction: r.instruction.clone(),
                context: p.destination_instructions(&r.folder),
            })
            .collect();
        let mut source_folders = crate::agent::source_folders(Some(path), &p.root);
        // Keep filename associations available for explicit named-group rules, separately from document facts.
        source_folders.push(name.chars().take(200).collect());
        // One extra-evidence request from the matcher; it shares the reader's three-read budget.
        for attempt in 0..2 {
            let input = MatchInput {
                understanding: &understanding,
                evidence: reader_role.evidence(),
                quotes: understanding
                    .evidence
                    .iter()
                    .map(|e| e.quote.clone())
                    .collect(),
                destinations: destinations.clone(),
                source_folders: source_folders.clone(),
                root_rules: &p.instructions,
                can_read_more: attempt == 0 && reader_role.can_read_more(),
            };
            match model.match_document(&input, remaining()?)? {
                MatchReply::NeedEvidence { page } if input.can_read_more => {
                    reader_role.read_more(page, false, remaining()?)?;
                    understanding = reader_role.describe(remaining()?)?;
                }
                MatchReply::NeedEvidence { .. } => {
                    return Err(
                        "More evidence is needed, but the reading limit was reached.".into(),
                    )
                }
                MatchReply::Uncertain { reason } => return Err(reason.chars().take(600).collect()),
                MatchReply::Finish {
                    selection,
                    reason,
                    quote,
                    source_group,
                } => {
                    let destination = destinations
                        .get(selection)
                        .ok_or("Matcher selected an unavailable folder")?;
                    if destinations.iter().any(|other| {
                        other.folder != destination.folder
                            && Path::new(&other.folder).parent()
                                == Path::new(&destination.folder).parent()
                            && !normalize(&destination.instruction).is_empty()
                            && normalize(&other.instruction) == normalize(&destination.instruction)
                            && crate::model::same_document_types(
                                &other.document_types,
                                &destination.document_types,
                            )
                    }) {
                        return Err("Two folders have the same rule. Give them distinct descriptions or choose a folder yourself.".into());
                    }
                    let group_supported = !source_group.is_empty()
                        && source_folders.contains(&source_group)
                        && normalize(&destination.instruction).contains(&normalize(&source_group));
                    if !(group_supported
                        || understanding.grounded()
                            && quote_present(&quote, reader_role.evidence()))
                    {
                        return Err("The folder match has insufficient source evidence.".into());
                    }
                    if !model.verify_match(&input, selection, remaining()?)? {
                        return Err("The folder check could not confirm this match. Choose a folder for this document.".into());
                    }
                    let mut title = String::new();
                    let mut reason = reason.chars().take(600).collect::<String>();
                    if p.rename && understanding.grounded() {
                        let naming = NameInput {
                            root_rules: &p.instructions,
                            understanding: &understanding,
                            evidence: reader_role.evidence(),
                            rules: &destination.context,
                        };
                        title = remaining()
                            .ok()
                            .and_then(|t| model.name(&naming, t).ok())
                            .filter(|title| supported_title(title, &understanding, &p.instructions))
                            .unwrap_or_default();
                        if title.is_empty() {
                            reason.push_str(" Original filename kept because a new name could not be established.");
                        }
                    }
                    return Ok(Decision {
                        language: None,
                        folder: destination.folder.clone(),
                        title,
                        reason,
                        confident: true,
                    });
                }
            }
        }
        Err("Document still needs attention after reading more evidence.".into())
    })();
    classifier::finalize(result, p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{model::Rule, provider::ReaderReply};
    use serde_json::Value;
    use std::{cell::RefCell, collections::VecDeque, fs};
    struct Fake {
        calls: RefCell<Vec<&'static str>>,
        verification: Result<bool, String>,
        replies: RefCell<VecDeque<MatchReply>>,
        reader_replies: RefCell<VecDeque<ReaderReply>>,
    }
    impl Fake {
        fn new(replies: Vec<MatchReply>) -> Self {
            Self {
                calls: RefCell::new(vec![]),
                verification: Ok(true),
                replies: RefCell::new(replies.into()),
                reader_replies: RefCell::new(VecDeque::new()),
            }
        }
    }
    impl ModelProvider for Fake {
        fn status(&self) -> Result<String, String> {
            Ok("Ready".into())
        }
        fn suggest_rule(&self, _: &str, _: &str) -> Result<String, String> {
            Err("Not used in filing".into())
        }
        fn identity(&self) -> Result<String, String> {
            Ok("fake-model:v1".into())
        }

        fn describe(
            &self,
            _: &str,
            _: &[&str],
            _: &[Value],
            _: Duration,
        ) -> Result<ReaderReply, String> {
            self.calls.borrow_mut().push("describe");
            Ok(self
                .reader_replies
                .borrow_mut()
                .pop_front()
                .unwrap_or(ReaderReply::Finish {
                    description: "Insurance covering a home.".into(),
                    quotes: vec!["Home insurance policy".into()],
                    uncertainty: String::new(),
                }))
        }
        fn categories(&self, _: &[String], _: Duration) -> Result<Vec<Rule>, String> {
            panic!("Filing must not plan categories")
        }
        fn match_document(
            &self,
            input: &MatchInput<'_>,
            _: Duration,
        ) -> Result<MatchReply, String> {
            self.calls.borrow_mut().push("match");
            assert_eq!(input.destinations[0].folder, "Insurance");
            self.replies
                .borrow_mut()
                .pop_front()
                .ok_or("Unexpected extra matching call".into())
        }
        fn verify_match(&self, _: &MatchInput<'_>, _: usize, _: Duration) -> Result<bool, String> {
            self.calls.borrow_mut().push("verify");
            self.verification.clone()
        }
        fn name(&self, _: &NameInput<'_>, _: Duration) -> Result<String, String> {
            self.calls.borrow_mut().push("name");
            Err("Synthetic naming refusal".into())
        }
    }
    fn finish(selection: usize, quote: &str) -> MatchReply {
        MatchReply::Finish {
            selection,
            reason: "Matches the policy rule.".into(),
            quote: quote.into(),
            source_group: String::new(),
        }
    }
    fn scenario(model: &Fake, rename: bool, duplicate: bool) -> Decision {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("scan.txt");
        let text = "Home insurance policy";
        fs::write(&path, format!("{}{}", text, " ".repeat(8100))).unwrap();
        let rule = Rule {
            document_types: Vec::new(),
            folder: "Insurance".into(),
            instruction: "Keep insurance policies here.".into(),
            enabled: true,
        };
        let mut p = Preferences {
            root: root.path().to_string_lossy().into(),
            rename,
            rules: vec![rule.clone()],
            ..Preferences::default()
        };
        if duplicate {
            p.rules.push(Rule {
                document_types: Vec::new(),
                folder: "Policies".into(),
                ..rule
            });
        }
        let decision = run(
            "scan.txt",
            text,
            &p,
            &path,
            &root.path().join("unused-reader"),
            model,
        );
        assert!(path.exists());
        assert!(!root.path().join("Insurance").exists());
        decision
    }
    #[test]
    fn filenames_use_only_verified_facts_and_literal_prefixes() {
        let facts = crate::description::Understanding {
            description: "Home insurance policy".into(),
            uncertainty: String::new(),
            evidence: vec![crate::description::Evidence {
                quote: "Home insurance policy".into(),
                page: None,
            }],
        };
        assert!(supported_title("Home Insurance Policy", &facts, ""));
        assert!(!supported_title("Jane Doe insurance 2099", &facts, ""));
        assert!(!supported_title("Home insurance renewal", &facts, ""));
        assert!(supported_title(
            "Archive Home Insurance Policy",
            &facts,
            "Use filenames starting with Archive."
        ));
        assert!(!supported_title(
            "Archive Home Insurance Policy",
            &facts,
            "Do not use Archive in names."
        ));
    }
    #[test]
    fn roles_run_in_order_and_naming_failure_keeps_supported_match() {
        let fake = Fake::new(vec![finish(0, "Home insurance policy")]);
        let result = scenario(&fake, true, false);
        assert!(result.confident);
        assert_eq!(result.folder, "Insurance");
        assert!(result.title.is_empty());
        assert_eq!(
            *fake.calls.borrow(),
            vec!["describe", "match", "verify", "name"]
        );
    }
    #[test]
    fn invalid_choices_and_quotes_never_reach_naming() {
        for reply in [
            finish(99, "Home insurance policy"),
            finish(0, "Invented evidence"),
            MatchReply::Uncertain {
                reason: "Competing matches".into(),
            },
        ] {
            let fake = Fake::new(vec![reply]);
            let result = scenario(&fake, true, false);
            assert!(!result.confident);
            assert_eq!(result.folder, "Needs Review");
            assert!(!fake.calls.borrow().contains(&"name"));
        }
    }
    #[test]
    fn matcher_delegates_extra_page_to_reader_and_stops_after_one_retry() {
        let fake = Fake::new(vec![
            MatchReply::NeedEvidence { page: Some(2) },
            finish(0, "Home insurance policy"),
        ]);
        assert!(scenario(&fake, false, false).confident);
        assert_eq!(
            *fake.calls.borrow(),
            vec!["describe", "match", "describe", "match", "verify"]
        );
        let fake = Fake::new(vec![
            MatchReply::NeedEvidence { page: Some(2) },
            MatchReply::NeedEvidence { page: Some(3) },
        ]);
        assert!(!scenario(&fake, false, false).confident);
        assert_eq!(
            fake.calls
                .borrow()
                .iter()
                .filter(|c| **c == "match")
                .count(),
            2
        );
    }
    #[test]
    fn reader_and_matcher_share_three_reads() {
        let fake = Fake::new(vec![MatchReply::NeedEvidence { page: Some(4) }]);
        fake.reader_replies
            .borrow_mut()
            .extend([1, 2, 3].map(|p| ReaderReply::ReadPage { page: Some(p) }));
        let result = scenario(&fake, false, false);
        assert!(!result.confident);
        assert!(result.reason.contains("limit"));
        assert_eq!(
            fake.calls
                .borrow()
                .iter()
                .filter(|c| **c == "describe")
                .count(),
            4
        );
    }
    #[test]
    fn verification_disagreement_or_failure_never_reaches_naming() {
        for verification in [Ok(false), Err("Verification unavailable".into())] {
            let mut fake = Fake::new(vec![finish(0, "Home insurance policy")]);
            fake.verification = verification;
            let result = scenario(&fake, true, false);
            assert!(!result.confident);
            assert_eq!(result.folder, "Needs Review");
            assert!(!fake.calls.borrow().contains(&"name"));
        }
    }
    #[test]
    fn complete_short_text_does_not_offer_redundant_reads() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("fragment.txt");
        fs::write(&path, "Reference 47.").unwrap();
        let fake = Fake::new(vec![]);
        let reader = root.path().join("unused");
        let mut session =
            ReaderSession::new(root.path(), &path, &reader, "Reference 47.", true, &fake).unwrap();
        assert!(!session.can_read_more());
        assert!(session
            .read_more(Some(1), true, Duration::from_secs(1))
            .is_err());
    }
    #[test]
    fn duplicate_sibling_rules_need_attention() {
        let fake = Fake::new(vec![finish(0, "Home insurance policy")]);
        let result = scenario(&fake, true, true);
        assert!(!result.confident);
        assert!(result.reason.contains("same rule"));
    }
}
