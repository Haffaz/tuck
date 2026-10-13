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
    pub folder: String,
    pub title: String,
    pub reason: String,
    pub confident: bool,
}
/// Text plus local clues that are not in the text (download source, PDF properties,
/// faces, barcodes, passport lines). See TuckClues.swift.
#[derive(Clone, Debug, Default)]
pub struct Reading {
    pub text: String,
    pub clues: Vec<String>,
}

pub const UNREADABLE_SCAN: &str =
    "Tuck can't read the text in this scan yet (its script or quality isn't supported). Choose a folder for it.";

pub fn read(path: &Path, reader: &Path) -> Result<Reading, String> {
    let ext = path
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    if ext == "txt" || ext == "md" {
        let data = fs::read(path).map_err(|e| e.to_string())?;
        // Text files only have download clues; failing to read them is not an error.
        let clues = run_reader(path, reader, &["--clues".into()], Duration::from_secs(10))
            .map(|value| clues_of(&value))
            .unwrap_or_default();
        return Ok(Reading {
            text: String::from_utf8_lossy(&data).chars().take(14000).collect(),
            clues,
        });
    }
    let value = run_reader(path, reader, &[], Duration::from_secs(60))?;
    if value["unreadable"].as_bool() == Some(true) {
        return Err(UNREADABLE_SCAN.into());
    }
    Ok(Reading {
        text: value["text"].as_str().unwrap_or_default().to_string(),
        clues: clues_of(&value),
    })
}
fn clues_of(value: &serde_json::Value) -> Vec<String> {
    value["clues"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|c| c.as_str())
        .map(|c| c.chars().take(160).collect())
        .take(12)
        .collect()
}
fn run_reader(
    path: &Path,
    reader: &Path,
    args: &[String],
    timeout: Duration,
) -> Result<serde_json::Value, String> {
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
    serde_json::from_slice(&output).map_err(|e| e.to_string())
}
#[cfg(test)]
pub fn classify(name: &str, text: &str, p: &Preferences) -> Decision {
    classify_with_tools(name, text, p, &[], None)
}

pub fn classify_with_tools(
    name: &str,
    text: &str,
    p: &Preferences,
    clues: &[String],
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
    crate::card_filing::run(
        text,
        clues,
        p,
        path,
        &crate::provider::configured(reader, p),
    )
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
        folder: "Needs Review".into(),
        title: String::new(),
        reason: reason.into(),
        confident: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// A stand-in reader that prints `output` as its JSON reply.
    fn fake_reader(dir: &Path, output: &str) -> std::path::PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let reader = dir.join("tuck-reader");
        fs::write(&reader, format!("#!/bin/sh\nprintf '%s' '{output}'\n")).unwrap();
        fs::set_permissions(&reader, fs::Permissions::from_mode(0o700)).unwrap();
        reader
    }
    #[test]
    fn unreadable_scans_need_review_and_clues_are_kept() {
        let dir = tempfile::tempdir().unwrap();
        let scan = dir.path().join("scan.png");
        fs::write(&scan, "image").unwrap();
        let reader = fake_reader(
            dir.path(),
            r#"{"text":"@ .12,450","clues":[],"unreadable":true}"#,
        );
        assert_eq!(read(&scan, &reader).unwrap_err(), UNREADABLE_SCAN);
        let reader = fake_reader(
            dir.path(),
            r#"{"text":"Passport","clues":["Has a photo of one person","Machine-readable zone: passport"],"unreadable":false}"#,
        );
        let reading = read(&scan, &reader).unwrap();
        assert_eq!(reading.text, "Passport");
        assert_eq!(
            reading.clues,
            [
                "Has a photo of one person",
                "Machine-readable zone: passport"
            ]
        );
    }
    #[test]
    fn text_files_get_download_clues_and_survive_a_failing_reader() {
        let dir = tempfile::tempdir().unwrap();
        let note = dir.path().join("note.txt");
        fs::write(&note, "Kontoutdrag mars").unwrap();
        let reader = fake_reader(
            dir.path(),
            r#"{"text":"","clues":["Downloaded from or sent by: nordea.se"],"unreadable":false}"#,
        );
        let reading = read(&note, &reader).unwrap();
        assert_eq!(reading.text, "Kontoutdrag mars");
        assert_eq!(reading.clues, ["Downloaded from or sent by: nordea.se"]);
        let missing = read(&note, &dir.path().join("no-reader")).unwrap();
        assert!(missing.clues.is_empty());
    }
    #[test]
    fn disabled_ai_and_disabled_instructions_require_review() {
        let mut p = Preferences {
            use_ai: false,
            ..Preferences::default()
        };
        let d = classify("Cedar insurance policy.txt", "bank statement", &p);
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
    fn live_apple_filing_smoke() {
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
    }

    #[test]
    fn model_failure_requires_review_without_keyword_override() {
        let p = Preferences::default();
        let d = finalize(Err("Apple is unavailable".into()), &p);
        assert_eq!(d.folder, "Needs Review");
        assert!(d.title.is_empty());
        assert!(d.reason.contains("unavailable"));
    }
}
