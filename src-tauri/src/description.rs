//! Rebuildable, content-addressed document understanding; never a filing decision.
use crate::{classifier, files, instructions};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
const VERSION: u32 = 1;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Evidence {
    pub quote: String,
    pub page: Option<usize>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Understanding {
    pub description: String,
    pub evidence: Vec<Evidence>,
    pub uncertainty: String,
}
impl Understanding {
    pub fn grounded(&self) -> bool {
        !self.description.trim().is_empty()
            && self.uncertainty.trim().is_empty()
            && !self.evidence.is_empty()
    }
    fn valid(&self, text: &str) -> bool {
        self.description.chars().count() <= 1200
            && self.uncertainty.chars().count() <= 600
            && self.evidence.len() <= 7
            && self.evidence.iter().all(|e| {
                e.quote.chars().count() >= 5
                    && e.quote.chars().count() <= 500
                    && text.contains(&e.quote)
                    && !e.quote.contains("[Text truncated]")
                    && !e.quote.contains("[Earlier excerpt]")
                    && !e.quote.contains("[Middle of excerpt omitted;")
                    && e.page.is_none_or(|p| (1..=50).contains(&p))
            })
    }
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Cached {
    version: u32,
    content_hash: String,
    input_hash: String,
    helper_hash: String,
    understanding: Understanding,
}
fn digest(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}
fn cache_path(root: &Path, hash: &str, create: bool) -> Option<PathBuf> {
    if hash.len() != 64 || !hash.bytes().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let metadata = instructions::settings_path(root)
        .ok()?
        .parent()?
        .to_path_buf();
    if !fs::symlink_metadata(&metadata).ok()?.is_dir() {
        return None;
    }
    let cache = metadata.join("cache");
    if create && !cache.exists() {
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        if builder.create(&cache).is_err() {
            return None;
        }
    }
    let info = fs::symlink_metadata(&cache).ok()?;
    if !info.is_dir() || info.file_type().is_symlink() {
        return None;
    }
    Some(cache.join(format!("{hash}.json")))
}
fn load(root: &Path, hash: &str, text: &str, helper_hash: &str) -> Option<Understanding> {
    let path = cache_path(root, hash, false)?;
    let info = fs::symlink_metadata(&path).ok()?;
    if !info.is_file() || info.file_type().is_symlink() || info.len() > 32_768 {
        return None;
    }
    let cached: Cached = serde_json::from_slice(&fs::read(path).ok()?).ok()?;
    (cached.version == VERSION
        && cached.content_hash == hash
        && cached.input_hash == digest(text)
        && cached.helper_hash == helper_hash
        && cached.understanding.grounded()
        && cached.understanding.valid(text))
    .then_some(cached.understanding)
}
fn save(root: &Path, hash: &str, text: &str, helper_hash: &str, understanding: &Understanding) {
    // Invalid/cache links are ignored. Cache failure must never block document handling.
    if !understanding.grounded() || !understanding.valid(text) {
        return;
    }
    if let Some(path) = cache_path(root, hash, true) {
        let record = Cached {
            version: VERSION,
            content_hash: hash.into(),
            input_hash: digest(text),
            helper_hash: helper_hash.into(),
            understanding: understanding.clone(),
        };
        // Unlike user settings, corrupt cache entries may be replaced safely.
        if fs::symlink_metadata(&path).is_ok_and(|m| !m.is_file() || m.file_type().is_symlink()) {
            return;
        }
        if let (Ok(mut file), Ok(bytes)) = (
            tempfile::NamedTempFile::new_in(path.parent().unwrap()),
            serde_json::to_vec_pretty(&record),
        ) {
            use std::io::Write;
            if file.write_all(&bytes).is_ok() && file.as_file().sync_all().is_ok() {
                let _ = file.persist(path);
            }
        }
    }
}
fn page_for(text: &str, quote: &str) -> Option<usize> {
    let before = &text[..text.find(quote)?];
    let mut page = None;
    for line in before.lines() {
        if line == "[Earlier excerpt]" {
            page = None;
        }
        if let Some(number) = line
            .strip_prefix("[Page ")
            .and_then(|s| s.strip_suffix(']'))
            .and_then(|s| s.parse().ok())
        {
            page = Some(number);
        }
    }
    page
}
// Resolve a single explicit cross-reference only when a read is requested. This
// cannot open another file: all page access remains scoped to the current source.
fn referenced_page(text: &str) -> Option<usize> {
    let source = text
        .lines()
        .filter(|l| !l.trim_start().starts_with('['))
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    let tokens: Vec<_> = source
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .collect();
    let mut pages = tokens
        .windows(2)
        .filter_map(|p| {
            if !matches!(p[0], "page" | "chunk" | "sida") {
                return None;
            }
            p[1].parse::<usize>().ok().filter(|n| (1..=50).contains(n))
        })
        .collect::<Vec<_>>();
    pages.sort_unstable();
    pages.dedup();
    (pages.len() == 1).then(|| pages[0])
}
/// A single document's read state. Only this role can execute page/OCR tools.
pub struct ReaderSession<'a> {
    root: &'a Path,
    path: &'a Path,
    reader: &'a Path,
    model: &'a dyn crate::provider::ModelProvider,
    initial: String,
    ledger: String,
    hash: String,
    identity: String,
    tools: bool,
    completed: Vec<Value>,
    cached: Option<Understanding>,
}
impl<'a> ReaderSession<'a> {
    pub fn new(
        root: &'a Path,
        path: &'a Path,
        reader: &'a Path,
        text: &str,
        tools: bool,
        model: &'a dyn crate::provider::ModelProvider,
    ) -> Result<Self, String> {
        let hash = files::digest(path)?;
        let identity = model.identity()?;
        let cached = load(root, &hash, text, &identity);
        Ok(Self {
            root,
            path,
            reader,
            model,
            initial: text.into(),
            ledger: text.chars().take(14000).collect(),
            hash,
            identity,
            tools,
            completed: vec![],
            cached,
        })
    }
    pub fn evidence(&self) -> &str {
        &self.ledger
    }
    pub fn can_read_more(&self) -> bool {
        self.tools
            && self.completed.len() < 3
            && (self.page_read_available() || self.ocr_available())
    }
    fn extension(&self) -> String {
        self.path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default()
            .to_lowercase()
    }
    fn page_read_available(&self) -> bool {
        match self.extension().as_str() {
            "pdf" => true,
            "txt" | "md" => {
                self.initial.chars().count() > 6000
                    || fs::metadata(self.path).is_ok_and(|m| m.len() > self.initial.len() as u64)
            }
            _ => false,
        }
    }
    fn ocr_available(&self) -> bool {
        matches!(
            self.extension().as_str(),
            "pdf" | "png" | "jpg" | "jpeg" | "heic" | "tif" | "tiff"
        )
    }
    fn actions(&self) -> Vec<&'static str> {
        let mut actions = vec!["finish"];
        if self.can_read_more() {
            if self.page_read_available() {
                actions.push("read_page");
            }
            if self.ocr_available() {
                actions.push("retry_ocr");
            }
        }
        actions
    }
    fn next_page(&self) -> Option<usize> {
        let extension = self
            .path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default()
            .to_lowercase();
        if !matches!(extension.as_str(), "pdf" | "txt" | "md") {
            return None;
        }
        let pdf = self
            .path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("pdf"));
        let last = if pdf {
            self.initial
                .lines()
                .filter_map(|l| {
                    l.strip_prefix("[Page ")?
                        .strip_suffix(']')?
                        .parse::<usize>()
                        .ok()
                })
                .max()
                .unwrap_or(0)
        } else {
            self.initial.chars().count().div_ceil(4000)
        };
        let clipped = self.initial.chars().count() >= 13900;
        let first = if clipped { last.max(1) } else { last + 1 };
        let maximum = if pdf {
            self.initial
                .lines()
                .next()
                .and_then(|l| {
                    l.strip_prefix("[Document has ")?
                        .strip_suffix(" pages.]")?
                        .parse::<usize>()
                        .ok()
                })
                .unwrap_or(50)
                .min(50)
        } else if clipped {
            50
        } else {
            last
        };
        (first..=maximum).find(|page| {
            !self
                .completed
                .iter()
                .any(|r| r["action"] == "read_page" && r["page"] == *page)
        })
    }
    pub fn read_more(
        &mut self,
        page: Option<usize>,
        ocr: bool,
        timeout: Duration,
    ) -> Result<(), String> {
        if !self.can_read_more() {
            return Err("Document read budget exhausted; needs attention".into());
        }
        if (ocr && !self.ocr_available()) || (!ocr && !self.page_read_available()) {
            return Err("This document has no available read of that kind".into());
        }
        let reference = if !ocr && page.unwrap_or(0) <= 1 {
            referenced_page(&self.ledger).filter(|p| {
                *p > 1
                    && !self
                        .completed
                        .iter()
                        .any(|r| r["action"] == "read_page" && r["page"] == *p)
            })
        } else {
            None
        };
        let page = reference
            .or(page)
            .filter(|p| *p != 0)
            .or_else(|| if ocr { Some(1) } else { self.next_page() })
            .ok_or("No unread page is available; needs attention")?;
        let action = if ocr { "retry_ocr" } else { "read_page" };
        if !(1..=50).contains(&page)
            || self
                .completed
                .iter()
                .any(|r| r["action"] == action && r["page"] == page)
        {
            return Err("Invalid or repeated document read".into());
        }
        self.completed.push(json!({"action":action,"page":page}));
        let text = classifier::read_page(self.path, self.reader, page, ocr, timeout)?;
        self.ledger = format!(
            "[Page {page}]\n{}\n[Earlier excerpt]\n{}",
            text.chars().take(4000).collect::<String>(),
            self.ledger.chars().take(8000).collect::<String>()
        );
        self.cached = None;
        Ok(())
    }
    pub fn describe(&mut self, budget: Duration) -> Result<Understanding, String> {
        if self.model.uses_evidence_categories() {
            if let Some(reason) = crate::category_evidence::source_issue(&self.initial) {
                return Ok(Understanding {
                    description: "Unclear".into(),
                    evidence: vec![],
                    uncertainty: reason.into(),
                });
            }
        }
        if let Some(cached) = self.cached.take() {
            return Ok(cached);
        }
        let started = Instant::now();
        let remaining = || {
            budget
                .checked_sub(started.elapsed())
                .filter(|d| !d.is_zero())
                .ok_or("Document understanding reached its time limit")
        };
        loop {
            let actions = self.actions();
            match self
                .model
                .describe(&self.ledger, &actions, &self.completed, remaining()?)?
            {
                crate::provider::ReaderReply::ReadPage { page } => {
                    self.read_more(page, false, remaining()?)?
                }
                crate::provider::ReaderReply::RetryOcr { page } => {
                    self.read_more(page, true, remaining()?)?
                }
                crate::provider::ReaderReply::Finish {
                    description,
                    quotes,
                    mut uncertainty,
                } => {
                    let evidence = quotes
                        .into_iter()
                        .filter(|q| !q.is_empty())
                        .map(|quote| Evidence {
                            page: page_for(&self.ledger, &quote),
                            quote,
                        })
                        .collect::<Vec<_>>();
                    if evidence.is_empty() && uncertainty.is_empty() {
                        uncertainty = "No supporting passage was found.".into();
                    }
                    let understanding = Understanding {
                        description,
                        evidence,
                        uncertainty,
                    };
                    if !understanding.valid(&self.ledger) {
                        return Err("Document description evidence could not be verified".into());
                    }
                    if !self.model.uses_evidence_categories()
                        && !understanding.grounded()
                        && self.can_read_more()
                    {
                        if self.next_page().is_some() {
                            self.read_more(None, false, remaining()?)?;
                            continue;
                        }
                        if self.ocr_available()
                            && self.ledger.chars().filter(|c| c.is_alphabetic()).count() < 60
                            && !self.completed.iter().any(|r| r["action"] == "retry_ocr")
                        {
                            self.read_more(Some(1), true, remaining()?)?;
                            continue;
                        }
                    }
                    if files::digest(self.path)? != self.hash {
                        return Err("Document changed while being described; try again".into());
                    }
                    save(
                        self.root,
                        &self.hash,
                        &self.initial,
                        &self.identity,
                        &understanding,
                    );
                    return Ok(understanding);
                }
            }
        }
    }
}
/// Shared by the planner and compatibility callers; the active production provider is selected centrally.
#[cfg(test)]
pub fn understand(
    root: &Path,
    path: &Path,
    reader: &Path,
    text: &str,
    tools: bool,
    budget: Duration,
) -> Result<Understanding, String> {
    let model = crate::provider::local(reader);
    ReaderSession::new(root, path, reader, text, tools, &model)?.describe(budget)
}
#[cfg(test)]
mod tests {
    #[test]
    fn references_are_unique_bounded_and_ignore_application_page_markers() {
        assert_eq!(
            super::referenced_page("[Page 1]\nSee text chunk 5. Read chunk 5."),
            Some(5)
        );
        assert_eq!(super::referenced_page("See page 2 or page 4."), None);
        assert_eq!(super::referenced_page("See page 999."), None);
        assert_eq!(super::referenced_page("[Page 5]"), None);
        assert_eq!(
            super::page_for(
                "[Page 5]\nMaintenance report\n[Earlier excerpt]\nCover sheet",
                "Cover sheet"
            ),
            None
        );
    }

    use super::*;
    fn understanding() -> Understanding {
        Understanding {
            description: "Insurance covering a home.".into(),
            evidence: vec![Evidence {
                quote: "Home insurance policy".into(),
                page: None,
            }],
            uncertainty: String::new(),
        }
    }
    #[test]
    fn cache_is_content_based_rebuildable_and_excludes_rules() {
        let root = tempfile::tempdir_in("/private/tmp").unwrap();
        fs::create_dir(root.path().join(".tuck")).unwrap();
        let text = "Home insurance policy";
        let hash = digest(text);
        save(root.path(), &hash, text, "model-v1", &understanding());
        assert!(load(root.path(), &hash, text, "model-v1").is_some());
        assert!(load(root.path(), &digest("changed bytes"), text, "model-v1").is_none());
        assert!(load(root.path(), &hash, "different extraction", "model-v1").is_none());
        assert!(load(root.path(), &hash, text, "model-v2").is_none());
        let path = cache_path(root.path(), &hash, false).unwrap();
        fs::write(&path, "broken").unwrap();
        assert!(load(root.path(), &hash, text, "model-v1").is_none());
        save(root.path(), &hash, text, "model-v1", &understanding());
        assert!(load(root.path(), &hash, text, "model-v1").is_some());
        fs::remove_file(path).unwrap();
        assert!(load(root.path(), &hash, text, "model-v1").is_none());
    }
    #[test]
    #[ignore = "Requires Apple Intelligence; synthetic open-vocabulary description and filing"]
    fn live_description_first_unknown_type_and_cache() {
        let root = tempfile::tempdir_in("/private/tmp").unwrap();
        let mut p = crate::model::Preferences {
            root: root.path().to_string_lossy().into_owned(),
            rename: false,
            rules: vec![crate::model::Rule {
                document_types: Vec::new(),
                folder: "Equipment".into(),
                instruction: "Keep equipment servicing, inspections and maintenance reports here."
                    .into(),
                enabled: true,
            }],
            ..crate::model::Preferences::default()
        };
        crate::config::initialize(&mut p).unwrap();
        let path = root.path().join("scan.txt");
        let text = "Equipment maintenance report. A technician inspected a ventilation fan, replaced its worn drive belt, cleaned the air filters and confirmed normal operation.";
        fs::write(&path, text).unwrap();
        let reader = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/tuck-reader");
        let understanding = understand(
            root.path(),
            &path,
            &reader,
            text,
            true,
            Duration::from_secs(90),
        )
        .unwrap();
        println!("Description: {}", understanding.description);
        assert!(understanding.grounded(), "{}", understanding.uncertainty);
        assert!(understanding
            .description
            .to_lowercase()
            .contains("maintenance"));
        let cache = cache_path(root.path(), &files::digest(&path).unwrap(), false).unwrap();
        let written = fs::metadata(&cache).unwrap().modified().unwrap();
        let decision = crate::classifier::classify_with_tools(
            "scan.txt",
            text,
            &p,
            &[],
            Some((&path, &reader)),
        );
        println!("Folder: {}; reason: {}", decision.folder, decision.reason);
        assert_eq!(decision.folder, "Equipment");
        assert!(decision.confident);
        assert_eq!(fs::metadata(cache).unwrap().modified().unwrap(), written);
        assert_eq!(fs::read_to_string(path).unwrap(), text);
    }
    #[test]
    fn extra_reads_are_bounded_and_cannot_invent_evidence() {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir_in("/private/tmp").unwrap();
        let path = root.path().join("document.txt");
        fs::write(&path, format!("{}Home insurance policy", "x".repeat(4000))).unwrap();
        let helper = root.path().join("tuck-apple");
        fs::write(&helper, r#"#!/usr/bin/python3
import json,sys
r=json.load(sys.stdin)
if r['operation']=='language':
 print(json.dumps({'text':r['text'],'blocked':False,'language':{'name':'English','code':'en','status':'supported','message':''}}))
elif not r['completed']:
 print(json.dumps({'action':'read_page','page':2}))
else:
 print(json.dumps({'action':'finish','page':0,'description':'Insurance covering a home.','quotes':['Home insurance policy'],'uncertainty':''}))
"#).unwrap();
        fs::set_permissions(&helper, fs::Permissions::from_mode(0o700)).unwrap();
        let reader = root.path().join("tuck-reader");
        let model = crate::provider::local(&reader);
        let mut session = ReaderSession::new(
            root.path(),
            &path,
            &reader,
            "Cover sheet: see chunk 2.",
            true,
            &model,
        )
        .unwrap();
        session
            .read_more(Some(2), false, Duration::from_secs(10))
            .unwrap();
        assert!(session.evidence().contains("Home insurance policy"));
        assert_eq!(
            page_for(session.evidence(), "Home insurance policy"),
            Some(2)
        );
        assert!(session
            .read_more(Some(2), false, Duration::from_secs(10))
            .is_err());
        assert!(session
            .read_more(Some(51), false, Duration::from_secs(10))
            .is_err());
        let mut disabled =
            ReaderSession::new(root.path(), &path, &reader, "Cover", false, &model).unwrap();
        assert!(disabled
            .read_more(Some(2), false, Duration::from_secs(10))
            .unwrap_err()
            .contains("budget"));
        // The Apple reader does not infer a document type from a sparse cover.
        let unclear = disabled.describe(Duration::from_secs(10)).unwrap();
        assert!(!unclear.grounded());
        assert!(unclear.uncertainty.contains("readable text"));
    }
    #[test]
    fn detailed_evidence_has_a_bounded_source_grounded_budget() {
        let text = (0..8)
            .map(|i| format!("Observed detail number {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut u = Understanding {
            description: "Project report".into(),
            uncertainty: String::new(),
            evidence: text
                .lines()
                .take(7)
                .map(|line| Evidence {
                    quote: line.into(),
                    page: None,
                })
                .collect(),
        };
        assert!(u.valid(&text));
        u.evidence.push(Evidence {
            quote: "Observed detail number 7".into(),
            page: None,
        });
        assert!(!u.valid(&text));
        u.evidence.pop();
        u.evidence[1].quote = "Invented project detail".into();
        assert!(!u.valid(&text));
    }
    #[test]
    fn quotes_are_grounded_and_cache_cannot_follow_links() {
        let mut u = understanding();
        assert!(!u.valid("Passport"));
        u.uncertainty = "Missing coverage details".into();
        assert!(!u.grounded());
        assert_eq!(
            page_for("[Page 2]\nHome insurance policy", "Home insurance policy"),
            Some(2)
        );
        let root = tempfile::tempdir_in("/private/tmp").unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join(".tuck")).unwrap();
        std::os::unix::fs::symlink(outside.path(), root.path().join(".tuck/cache")).unwrap();
        save(
            root.path(),
            &digest("x"),
            "Home insurance policy",
            "v1",
            &understanding(),
        );
        assert_eq!(fs::read_dir(outside.path()).unwrap().count(), 0);
    }
}
