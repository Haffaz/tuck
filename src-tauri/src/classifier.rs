use crate::{files, model::Preferences};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Read,
    path::Path,
    process::{Command, Stdio},
    time::Duration,
};
use wait_timeout::ChildExt;

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct Decision {
    #[serde(default)]
    pub language: Option<crate::model::LanguageInfo>,
    pub folder: String,
    pub title: String,
    pub reason: String,
    pub confident: bool,
}
pub fn read_document(path: &Path, reader: &Path) -> Result<String, String> {
    let ext = path
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    if ext == "txt" || ext == "md" {
        let data = fs::read(path).map_err(|e| e.to_string())?;
        return Ok(String::from_utf8_lossy(&data).chars().take(14000).collect());
    }
    run_reader(path, reader, &[], Duration::from_secs(60))
}
fn run_reader(
    path: &Path,
    reader: &Path,
    args: &[String],
    timeout: Duration,
) -> Result<String, String> {
    let mut child = Command::new(reader)
        .arg(path)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Local document reader unavailable: {e}"))?;
    let mut stdout = child.stdout.take().ok_or("Reader output unavailable")?;
    let drain = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = stdout.read_to_end(&mut bytes);
        bytes
    });
    if child
        .wait_timeout(timeout)
        .map_err(|e| e.to_string())?
        .is_none()
    {
        let _ = child.kill();
        let _ = child.wait();
        return Err("Local text recognition timed out".into());
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    let output = drain.join().map_err(|_| "Reader failed")?;
    if !status.success() {
        return Err("Could not read this document locally".into());
    }
    let value: serde_json::Value = serde_json::from_slice(&output).map_err(|e| e.to_string())?;
    Ok(value["text"].as_str().unwrap_or_default().to_string())
}
#[cfg(test)]
pub fn classify(name: &str, text: &str, p: &Preferences) -> Decision {
    classify_with_tools(name, text, p, &[], None)
}

pub fn classify_with_tools(
    name: &str,
    text: &str,
    p: &Preferences,
    _memories: &[crate::model::Correction],
    source: Option<(&Path, &Path)>,
) -> Decision {
    if !p.rules.iter().any(|r| p.rule_enabled(r)) {
        return review(
            "All folder rules are disabled. Enable a folder to resume automatic classification.",
        );
    }
    if !p.use_ai {
        return review("Local AI is off. Enable it to interpret your folder rules.");
    }
    let Some((path, reader)) = source else {
        return review("Local model helper unavailable");
    };
    let _ = name;
    crate::card_filing::run(text, p, path, &crate::provider::configured(reader, p))
}

pub(crate) fn finalize(result: Result<Decision, String>, p: &Preferences) -> Decision {
    match result {
        Ok(mut decision) => {
            if !decision.confident {
                return review(&format!(
                    "The local model was uncertain. {}",
                    decision.reason
                ));
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
        language: None,
        folder: "Needs Review".into(),
        title: String::new(),
        reason: reason.into(),
        confident: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn disabled_ai_and_disabled_instructions_require_review() {
        let mut p = Preferences {
            use_ai: false,
            ..Preferences::default()
        };
        let d = classify("AGENCY insurance policy.txt", "bank statement", &p);
        assert_eq!(d.folder, "Needs Review");
        assert!(d.title.is_empty());
        assert!(d.reason.contains("off"));
        p.use_ai = true;
        for rule in &mut p.rules {
            rule.enabled = false;
        }
        let d = classify("scan.txt", "insurance policy", &p);
        assert_eq!(d.folder, "Needs Review");
        assert!(d.reason.contains("disabled"));
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
            language: None,
            folder: "Insurance".into(),
            title: "Policy".into(),
            reason: "AI suggestion".into(),
            confident: true,
        };
        assert_eq!(finalize(Ok(suggestion), &p).folder, "Needs Review");
    }
    #[test]
    fn uncertain_decisions_keep_original_names_and_require_review() {
        let p = Preferences::default();
        let uncertain = Decision {
            language: None,
            folder: "Home".into(),
            title: "Made up name".into(),
            reason: "Not sure".into(),
            confident: false,
        };
        let decision = finalize(Ok(uncertain), &p);
        assert_eq!(decision.folder, "Needs Review");
        assert!(decision.title.is_empty());
        assert!(!decision.confident);
    }
    #[test]
    #[ignore = "Requires Apple Intelligence enabled; uses only synthetic documents"]
    fn live_apple_agent_smoke() {
        let reader = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/tuck-reader");
        println!(
            "{}",
            crate::apple::check(&reader.with_file_name("tuck-apple")).unwrap()
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
            ("[unreadable scan; no recognizable text]", "Needs Review"),
        ] {
            fs::write(&path, text).unwrap();
            let started = std::time::Instant::now();
            let d = classify_with_tools("scan.txt", text, &p, &[], Some((&path, &reader)));
            println!(
                "Apple synthetic: {:.2}s {}",
                started.elapsed().as_secs_f64(),
                serde_json::to_string(&d).unwrap()
            );
            assert_eq!(d.folder, expected);
            if expected != "Needs Review" {
                assert!(d.confident);
                // Naming is now independent: a refused/uncertain naming call
                // must retain the original filename without blocking a valid folder.
            }
        }
        let index =
            "Document index: the actual certificate is in text chunk 5. This is a cover sheet. ";
        let mut data = format!("{index}{}", "Cover sheet. ".repeat(1400));
        data.truncate(16000);
        data.push_str(
            "FÖRSÄKRINGSBREV. Home insurance certificate. Building and contents coverage.",
        );
        fs::write(&path, &data).unwrap();
        let text = read_document(&path, &reader).unwrap();
        let started = std::time::Instant::now();
        let d = classify_with_tools("packet.txt", &text, &p, &[], Some((&path, &reader)));
        println!(
            "Apple late evidence: {:.2}s {}",
            started.elapsed().as_secs_f64(),
            serde_json::to_string(&d).unwrap()
        );
        assert_eq!(d.folder, "Insurance");
        assert_eq!(fs::read_to_string(&path).unwrap(), data);
        assert!(d.confident);
    }

    #[test]
    fn model_failure_requires_review_without_keyword_override() {
        let p = Preferences::default();
        let d = finalize(Err("Apple is unavailable".into()), &p);
        assert_eq!(d.folder, "Needs Review");
        assert!(d.title.is_empty());
        assert!(d.reason.contains("unavailable"));
    }
    #[test]
    #[ignore = "Requires Apple Intelligence; synthetic semantic instructions and folder context"]
    fn live_apple_folder_instructions() {
        let reader = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/tuck-reader");
        let dir = tempfile::tempdir_in("/private/tmp").unwrap();
        let parent = dir.path().join("AGENCY").join("Supporting documents");
        fs::create_dir_all(&parent).unwrap();
        let path = parent.join("scan.txt");
        let text = "Employment contract. The employee works as an engineer for Acme.";
        fs::write(&path, text).unwrap();
        let mut p = Preferences {
            root: dir.path().to_string_lossy().into_owned(),
            rename: false,
            ..Preferences::default()
        };
        p.rules = vec![
            crate::model::Rule { document_types: Vec::new(), folder: "Migration".into(), instruction: "All my migration documents should go here. Include files from AGENCY and all supporting documents in AGENCY folders, even employment contracts.".into(), enabled: true },
            crate::model::Rule { document_types: Vec::new(), folder: "Work".into(), instruction: "Keep employment contracts here unless they belong to a migration application bundle.".into(), enabled: true },
        ];
        let d = classify_with_tools("scan.txt", text, &p, &[], Some((&path, &reader)));
        println!("Bundle context: {}", serde_json::to_string(&d).unwrap());
        assert_eq!(d.folder, "Migration");
        p.rules[0].enabled = false;
        p.rules[1].instruction = "Keep all employment contracts here.".into();
        let d = classify_with_tools("scan.txt", text, &p, &[], Some((&path, &reader)));
        println!(
            "Disabled bundle instruction: {}",
            serde_json::to_string(&d).unwrap()
        );
        assert_eq!(d.folder, "Work");
        p.rules =
            vec![crate::model::Rule { document_types: Vec::new(),
            folder: "Place to live".into(),
            instruction:
                "Agreements that let me occupy a home in exchange for monthly payment belong here."
                    .into(),
            enabled: true,
        }];
        let text = "HYRESAVTAL. Tenant and landlord agree to a twelve month tenancy of the apartment. Rent is paid monthly.";
        fs::write(&path, text).unwrap();
        let d = classify_with_tools("scan.txt", text, &p, &[], Some((&path, &reader)));
        println!(
            "Semantic description: {}",
            serde_json::to_string(&d).unwrap()
        );
        assert_eq!(d.folder, "Place to live");
        // The migrated seven-folder setup must retain bundle intent without matching unrelated parents.
        p = Preferences {
            root: dir.path().to_string_lossy().into_owned(),
            ..Preferences::default()
        };
        p.rules[0] =
            serde_json::from_str(r#"{"folder":"Migration/AGENCY","keywords":"agency"}"#).unwrap();
        let text = "Employment contract. The employee works as an engineer for Acme.";
        fs::write(&path, text).unwrap();
        let d = classify_with_tools("scan.txt", text, &p, &[], Some((&path, &reader)));
        println!(
            "Migrated bundle instruction: {}",
            serde_json::to_string(&d).unwrap()
        );
        assert_eq!(d.folder, "Migration/AGENCY");
        let unrelated = dir.path().join("Incoming");
        fs::create_dir(&unrelated).unwrap();
        let bank = unrelated.join("scan.txt");
        let text = "Bank account statement. Opening balance, transactions and closing balance.";
        fs::write(&bank, text).unwrap();
        let d = classify_with_tools("scan.txt", text, &p, &[], Some((&bank, &reader)));
        println!(
            "Unrelated parent folder: {}",
            serde_json::to_string(&d).unwrap()
        );
        assert_eq!(d.folder, "Finances");
        let named = dir.path().join("AGENCY employment contract.txt");
        let text = "Employment contract. The employee works as an engineer for Acme.";
        fs::write(&named, text).unwrap();
        let d = classify_with_tools(
            "AGENCY employment contract.txt",
            text,
            &p,
            &[],
            Some((&named, &reader)),
        );
        println!(
            "Filename association: {}",
            serde_json::to_string(&d).unwrap()
        );
        assert_eq!(d.folder, "Migration/AGENCY");
    }
    #[test]
    #[ignore = "Requires Apple Intelligence; synthetic hierarchy and inherited naming"]
    fn live_apple_hierarchical_instructions() {
        use crate::model::Rule;
        let dir = tempfile::tempdir_in("/private/tmp").unwrap();
        let path = dir.path().join("scan.txt");
        let reader = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/tuck-reader");
        let mut p=Preferences {instructions:"Use short English document names without dates.".into(), rules:vec![
            Rule { document_types: Vec::new(),folder:"Home".into(),instruction:"Housing and property documents belong here.".into(),enabled:true},
            Rule { document_types: Vec::new(),folder:"Home/Rental".into(),instruction:"Tenancy agreements between a landlord and tenant belong here. Use the document type as the name.".into(),enabled:true},
            Rule { document_types: Vec::new(),folder:"Finances".into(),instruction:"Bank account statements belong here.".into(),enabled:true},
        ],..Preferences::default()};
        let text="HYRESAVTAL. Tenant and landlord agree to a twelve month tenancy of the apartment. Rent is paid monthly.";
        fs::write(&path, text).unwrap();
        let d = classify_with_tools("scan.txt", text, &p, &[], Some((&path, &reader)));
        println!(
            "Hierarchical content routing: {}",
            serde_json::to_string(&d).unwrap()
        );
        assert_eq!(d.folder, "Home/Rental");
        assert!(!d.title.is_empty());
        assert!(!d.title.chars().any(|c| c.is_ascii_digit()));
        p.rules[0].enabled = false;
        fs::write(&path, text).unwrap();
        let d = classify_with_tools("scan.txt", text, &p, &[], Some((&path, &reader)));
        println!("Disabled parent: {}", serde_json::to_string(&d).unwrap());
        // This verifies the disabled branch, not semantic abstention: the current model
        // may still choose an unrelated enabled destination (recorded in ai-accuracy.md).
        assert!(!d.folder.starts_with("Home"));
        assert!(
            d.folder == "Needs Review"
                || p.rules
                    .iter()
                    .any(|r| p.rule_enabled(r) && r.folder == d.folder)
        );
    }
}
