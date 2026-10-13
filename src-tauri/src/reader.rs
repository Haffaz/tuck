//! Reads a document with the Swift reader (`tuck-reader`): PDF text, Vision OCR for scans and
//! images, photo labels, and local clues that are not in the text. Unreadable scans are an
//! error with a reason the user sees, so they wait for a decision instead of being guessed.
use std::{
    fs,
    io::Read,
    path::Path,
    process::{Command, Stdio},
    time::Duration,
};
use wait_timeout::ChildExt;

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
}
