//! One short, cached description per document. Shared by folder suggestions and filing.
use crate::{files, instructions, provider::ModelProvider};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

const VERSION: u32 = 2;
pub const UNIDENTIFIED: &str =
    "Tuck couldn't tell what kind of document this is. Choose a folder for it.";
const MAX_CACHE_BYTES: u64 = 64 * 1024;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    pub kind: String,
    #[serde(default)]
    pub issuer: String,
    #[serde(default)]
    pub date: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub embedding: Vec<f64>,
}

impl Card {
    /// The model could not tell what the document is ("unknown", "document", "scan").
    /// Such a card would otherwise become an "Unknown" folder.
    pub fn unidentified(&self) -> bool {
        const VAGUE: &[&str] = &[
            "unknown",
            "unidentified",
            "unclear",
            "unreadable",
            "document",
            "documents",
            "file",
            "scan",
            "scanned",
            "image",
            "page",
            "text",
            "other",
            "misc",
            "miscellaneous",
            "blank",
        ];
        self.kind
            .split_whitespace()
            .all(|word| VAGUE.contains(&word))
    }

    /// Rejects replies that cannot be grouped or shown. The helper already cleans fields.
    pub fn valid(&self) -> bool {
        let words = self.kind.split_whitespace().count();
        (1..=6).contains(&words)
            && self.kind.chars().count() <= 60
            && self.issuer.chars().count() <= 80
            && self.tags.len() <= 3
            && self
                .tags
                .iter()
                .all(|t| !t.trim().is_empty() && t.chars().count() <= 40)
            && !self.embedding.is_empty()
            && self.embedding.iter().all(|v| v.is_finite())
    }
}

/// "bank statement" -> "Bank Statement", for folder names and filenames.
pub fn title_case(text: &str) -> String {
    text.split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().chain(chars).collect()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Cached {
    version: u32,
    content_hash: String,
    input_hash: String,
    identity: String,
    card: Card,
}

fn digest(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

fn cache_path(root: &Path, hash: &str, create: bool) -> Option<PathBuf> {
    if hash.len() != 64 || !hash.bytes().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let settings = instructions::settings_path(root)
        .ok()?
        .parent()?
        .to_path_buf();
    if !fs::symlink_metadata(&settings).ok()?.is_dir() {
        return None;
    }
    let mut directory = settings;
    for part in ["cache", "cards"] {
        directory = directory.join(part);
        if create && !directory.exists() {
            let mut builder = fs::DirBuilder::new();
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                builder.mode(0o700);
            }
            // Parallel card workers may race to create it; losing that race is fine.
            if let Err(error) = builder.create(&directory) {
                if error.kind() != std::io::ErrorKind::AlreadyExists {
                    return None;
                }
            }
        }
        let info = fs::symlink_metadata(&directory).ok()?;
        if !info.is_dir() || info.file_type().is_symlink() {
            return None;
        }
    }
    Some(directory.join(format!("{hash}.json")))
}

fn load(root: &Path, hash: &str, text: &str, identity: &str) -> Option<Card> {
    let path = cache_path(root, hash, false)?;
    let info = fs::symlink_metadata(&path).ok()?;
    if !info.is_file() || info.file_type().is_symlink() || info.len() > MAX_CACHE_BYTES {
        return None;
    }
    let cached: Cached = serde_json::from_slice(&fs::read(path).ok()?).ok()?;
    (cached.version == VERSION
        && cached.content_hash == hash
        && cached.input_hash == digest(text)
        && cached.identity == identity
        && cached.card.valid())
    .then_some(cached.card)
}

/// Cache failure never blocks document handling; corrupt entries are simply replaced.
fn save(root: &Path, hash: &str, text: &str, identity: &str, card: &Card) {
    let Some(path) = cache_path(root, hash, true) else {
        return;
    };
    if fs::symlink_metadata(&path).is_ok_and(|m| !m.is_file() || m.file_type().is_symlink()) {
        return;
    }
    let record = Cached {
        version: VERSION,
        content_hash: hash.into(),
        input_hash: digest(text),
        identity: identity.into(),
        card: card.clone(),
    };
    let Some(parent) = path.parent() else {
        return;
    };
    if let (Ok(mut file), Ok(bytes)) = (
        tempfile::NamedTempFile::new_in(parent),
        serde_json::to_vec(&record),
    ) {
        use std::io::Write;
        if file.write_all(&bytes).is_ok() && file.as_file().sync_all().is_ok() {
            let _ = file.persist(path);
        }
    }
}

/// Returns the cached card or asks the model for one. Text is read from the start;
/// long documents are no longer rejected for being long.
pub fn describe(
    root: &Path,
    path: &Path,
    text: &str,
    clues: &[String],
    model: &dyn ModelProvider,
    timeout: Duration,
) -> Result<Card, String> {
    let hash = files::digest(path)?;
    let identity = model.identity()?;
    // Clues are part of what the model saw, so they are part of the cache key.
    let input = format!("{text}\n[clues]\n{}", clues.join("\n"));
    if let Some(card) = load(root, &hash, &input, &identity) {
        return Ok(card);
    }
    let name: String = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .chars()
        .take(200)
        .collect();
    let card = model.card(text, &name, clues, timeout)?;
    if !card.valid() {
        return Err("The model could not describe this document.".into());
    }
    if card.unidentified() {
        return Err(UNIDENTIFIED.into());
    }
    if files::digest(path)? != hash {
        return Err("Document changed while it was being read. Analyze it again.".into());
    }
    save(root, &hash, &input, &identity, &card);
    Ok(card)
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn card(kind: &str, embedding: Vec<f64>) -> Card {
        Card {
            kind: kind.into(),
            issuer: String::new(),
            date: String::new(),
            tags: vec!["tag".into()],
            embedding,
        }
    }

    #[test]
    fn vague_kinds_are_unidentified_but_real_kinds_are_not() {
        for kind in [
            "unknown",
            "document",
            "scanned document",
            "unknown image",
            "blank page",
        ] {
            assert!(card(kind, vec![1.0]).unidentified(), "{kind}");
        }
        for kind in [
            "photo",
            "bank statement",
            "passport",
            "unknown sender letter",
        ] {
            assert!(!card(kind, vec![1.0]).unidentified(), "{kind}");
        }
    }

    #[test]
    fn invalid_cards_are_rejected() {
        assert!(card("bank statement", vec![1.0]).valid());
        assert!(!card("", vec![1.0]).valid());
        assert!(!card("one two three four five six seven", vec![1.0]).valid());
        assert!(!card("payslip", vec![]).valid());
        assert!(!card("payslip", vec![f64::NAN]).valid());
        let mut tags = card("payslip", vec![1.0]);
        tags.tags = vec!["a".into(), "b".into(), "c".into(), "d".into()];
        assert!(!tags.valid());
    }

    #[test]
    fn cache_round_trips_and_invalidates_on_text_or_identity() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let settings = instructions::settings_path(&root).unwrap();
        fs::create_dir_all(settings.parent().unwrap()).unwrap();
        let hash = "a".repeat(64);
        let value = card("payslip", vec![0.5, 0.5]);
        save(&root, &hash, "text", "model:v1", &value);
        assert_eq!(load(&root, &hash, "text", "model:v1"), Some(value));
        assert_eq!(load(&root, &hash, "changed", "model:v1"), None);
        assert_eq!(load(&root, &hash, "text", "model:v2"), None);
        assert_eq!(load(&root, "../escape", "text", "model:v1"), None);
    }
}
