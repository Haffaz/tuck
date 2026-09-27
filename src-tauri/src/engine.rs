use crate::{
    classifier, files,
    model::{Activity, Preferences, Snapshot, Store},
};
use std::{
    collections::HashMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

pub struct Runtime {
    pub store: Store,
    pub settling: HashMap<String, String>,
    pub processing: Option<String>,
    pub error: Option<String>,
    pub revision: u64,
}
pub struct Engine {
    pub state: Mutex<Runtime>,
    pub storage: PathBuf,
    pub reader: PathBuf,
}
impl Engine {
    pub fn new(storage: PathBuf, reader: PathBuf) -> Result<Self, String> {
        fs::create_dir_all(storage.parent().ok_or("Missing app data folder")?)
            .map_err(|e| e.to_string())?;
        let mut store: Store = if storage.exists() {
            serde_json::from_slice(&fs::read(&storage).map_err(|e| e.to_string())?).map_err(
                |e| format!("Could not read Tuck's saved state. Original state preserved: {e}"),
            )?
        } else {
            Store::default()
        };
        let mut recovered = false;
        for record in &mut store.activity {
            if record.status == "undo_pending" {
                if !Path::new(&record.destination).exists()
                    && files::digest(Path::new(&record.original)).ok().as_deref()
                        == Some(&record.hash)
                {
                    record.status = "undone".into();
                    if let Ok(sig) = files::signature(Path::new(&record.original)) {
                        store.seen.insert(record.original.clone(), sig);
                    }
                } else if files::digest(Path::new(&record.destination))
                    .ok()
                    .as_deref()
                    == Some(&record.hash)
                {
                    record.status = if record.folder == "Needs Review" {
                        "review"
                    } else {
                        "sorted"
                    }
                    .into();
                } else {
                    record.status = "failed".into();
                    record.reason = "Undo was interrupted. All remaining copies were kept; please inspect their locations.".into();
                }
                store.watching = false;
                recovered = true;
            }
            if record.status == "pending" {
                let destination = Path::new(&record.destination);
                if files::digest(destination).ok().as_deref() == Some(&record.hash)
                    && (record.copied || !Path::new(&record.original).exists())
                {
                    record.status = if record.folder == "Needs Review" {
                        "review"
                    } else {
                        "sorted"
                    }
                    .into();
                    if let Ok(sig) = files::signature(destination) {
                        store.seen.insert(record.destination.clone(), sig);
                    }
                } else {
                    record.status = "failed".into();
                    record.reason = "An operation was interrupted. Any remaining originals and copies have been kept; inspect them before retrying.".into();
                    store.watching = false;
                }
                recovered = true;
            }
        }
        let engine = Self {
            state: Mutex::new(Runtime {
                store,
                settling: HashMap::new(),
                processing: None,
                error: None,
                revision: 0,
            }),
            storage,
            reader,
        };
        if recovered {
            engine.persist(&engine.state.lock().unwrap().store)?;
        }
        Ok(engine)
    }
    pub fn persist(&self, store: &Store) -> Result<(), String> {
        let data = serde_json::to_vec_pretty(store).map_err(|e| e.to_string())?;
        let mut file = tempfile::NamedTempFile::new_in(self.storage.parent().unwrap())
            .map_err(|e| e.to_string())?;
        file.write_all(&data).map_err(|e| e.to_string())?;
        file.as_file().sync_all().map_err(|e| e.to_string())?;
        file.persist(&self.storage)
            .map_err(|e| format!("Could not save Tuck's state: {e}"))?;
        Ok(())
    }
    pub fn snapshot(&self) -> Snapshot {
        let state = self.state.lock().unwrap();
        Snapshot {
            preferences: state.store.preferences.clone(),
            watching: state.store.watching,
            activity: state
                .store
                .activity
                .iter()
                .rev()
                .take(500)
                .cloned()
                .collect(),
            pending: state.store.queue.len(),
            processing: state.processing.clone(),
            error: state.error.clone(),
        }
    }
    pub fn save_preferences(&self, mut preferences: Preferences) -> Result<(), String> {
        files::validate(&preferences)?;
        if !preferences.root.is_empty() {
            let path = Path::new(&preferences.root)
                .canonicalize()
                .map_err(|_| "Choose an existing folder")?;
            if !path.is_dir() || path.parent().is_none() {
                return Err("Choose a document folder".into());
            }
            preferences.root = path.to_string_lossy().into_owned();
        }
        let mut state = self.state.lock().unwrap();
        if state.processing.is_some() {
            return Err("Wait for the current document to finish before saving preferences".into());
        }
        let mut next = state.store.clone();
        if next.preferences.root != preferences.root
            || next.preferences.recursive != preferences.recursive
        {
            next.watching = false;
            next.seen.clear();
            // Existing files are a baseline. Initial cleanup is a separate explicit action.
            if !preferences.root.is_empty() {
                for path in files::scan(&preferences)? {
                    if let Ok(sig) = files::signature(&path) {
                        next.seen.insert(path.to_string_lossy().into_owned(), sig);
                    }
                }
            }
            next.queue.clear();
            state.settling.clear();
        }
        next.preferences = preferences;
        self.persist(&next)?;
        state.store = next;
        state.revision += 1;
        state.error = None;
        Ok(())
    }
    pub fn watch(&self, enabled: bool) -> Result<(), String> {
        let mut state = self.state.lock().unwrap();
        if enabled && state.store.preferences.root.is_empty() {
            return Err("Choose a folder first".into());
        }
        let mut next = state.store.clone();
        next.watching = enabled;
        self.persist(&next)?;
        state.store = next;
        state.revision += 1;
        state.error = None;
        Ok(())
    }
    pub fn enqueue(&self, paths: Vec<PathBuf>) -> Result<usize, String> {
        let mut valid = Vec::new();
        for path in paths {
            files::signature(&path)?;
            if !files::supported(&path) {
                return Err("Choose PDF, PNG, JPG, HEIC, TIFF, TXT or Markdown files".into());
            }
            valid.push(path.canonicalize().map_err(|e| e.to_string())?);
        }
        let mut state = self.state.lock().unwrap();
        if state.store.preferences.root.is_empty() {
            return Err("Choose a destination folder first".into());
        }
        let mut count = 0;
        for path in valid {
            if !state.store.queue.contains(&path) && state.processing.as_deref() != path.to_str() {
                state.store.queue.push_back(path);
                count += 1;
            }
        }
        let mut next = state.store.clone();
        next.watching = true;
        self.persist(&next)?;
        state.store = next;
        state.error = None;
        Ok(count)
    }
    pub fn organize_existing(&self) -> Result<usize, String> {
        let preferences = self.state.lock().unwrap().store.preferences.clone();
        self.enqueue(files::scan(&preferences)?)
    }
    pub fn discover(&self) -> Result<(), String> {
        let mut state = self.state.lock().unwrap();
        if !state.store.watching || state.store.preferences.root.is_empty() {
            return Ok(());
        }
        let paths = files::scan(&state.store.preferences)?;
        let mut current = HashMap::new();
        for path in paths {
            let Ok(sig) = files::signature(&path) else {
                continue;
            };
            let key = path.to_string_lossy().into_owned();
            if state.store.seen.get(&key) != Some(&sig) {
                if state.settling.get(&key) == Some(&sig)
                    && !state.store.queue.contains(&path)
                    && state.processing.as_deref() != Some(&key)
                {
                    state.store.queue.push_back(path);
                }
                current.insert(key, sig);
            }
        }
        state.settling = current;
        Ok(())
    }
    pub fn process_next(&self) -> Result<bool, String> {
        let (path, preferences, revision) = {
            let mut state = self.state.lock().unwrap();
            if !state.store.watching {
                return Ok(false);
            }
            let Some(path) = state.store.queue.pop_front() else {
                return Ok(false);
            };
            state.processing = Some(path.to_string_lossy().into_owned());
            (path, state.store.preferences.clone(), state.revision)
        };
        let result = self.process(&path, &preferences, revision);
        let mut state = self.state.lock().unwrap();
        state.processing = None;
        if let Err(error) = result {
            state.store.activity.push(Activity {
                id: uuid::Uuid::new_v4().to_string(),
                timestamp: now(),
                original: path.to_string_lossy().into_owned(),
                destination: String::new(),
                folder: String::new(),
                reason: error,
                status: "failed".into(),
                hash: String::new(),
                copied: false,
                duplicate: false,
            });
            if let Ok(sig) = files::signature(&path) {
                state
                    .store
                    .seen
                    .insert(path.to_string_lossy().into_owned(), sig);
            }
            self.persist(&state.store)?;
        }
        Ok(true)
    }
    fn process(&self, source: &Path, p: &Preferences, revision: u64) -> Result<(), String> {
        let before = files::signature(source)?;
        if fs::metadata(source).map_err(|e| e.to_string())?.len() > 100 * 1024 * 1024 {
            return Err("Files larger than 100 MB need manual review; original kept".into());
        }
        let hash = files::digest(source)?;
        let filename = source.file_name().unwrap_or_default().to_string_lossy();
        let text = classifier::read_document(source, &self.reader);
        let decision = match text {
            Ok(text) => classifier::classify(&filename, &text, p),
            Err(error) => {
                classifier::rule_match(&filename, "", p).unwrap_or(classifier::Decision {
                    folder: "Needs Review".into(),
                    title: String::new(),
                    reason: error,
                    confident: false,
                })
            }
        };
        let mut state = self.state.lock().unwrap();
        if !state.store.watching || state.revision != revision {
            state.store.queue.push_front(source.to_path_buf());
            self.persist(&state.store)?;
            return Ok(());
        }
        if files::signature(source)? != before {
            return Err(
                "File changed during reading; original kept. Retry after it finishes saving."
                    .into(),
            );
        }
        let root = Path::new(&p.root);
        if root.canonicalize().map_err(|e| e.to_string())? != root {
            return Err("Selected folder changed location; please choose it again".into());
        }
        let directory = files::target_directory(root, &decision.folder)?;
        let extension = source.extension().unwrap_or_default().to_string_lossy();
        let old_stem = source.file_stem().unwrap_or_default().to_string_lossy();
        let stem: &str = if p.rename && !decision.title.is_empty() && decision.confident {
            &decision.title
        } else {
            &old_stem
        };
        let destination = files::unique_destination(&directory, stem, &extension, source);
        let copied = !source.starts_with(root);
        let duplicate = state.store.activity.iter().any(|r| {
            r.hash == hash
                && r.destination != source.to_string_lossy()
                && matches!(r.status.as_str(), "sorted" | "review")
                && Path::new(&r.destination).exists()
        });
        if destination == source {
            state
                .store
                .seen
                .insert(source.to_string_lossy().into_owned(), before);
            self.persist(&state.store)?;
            return Ok(());
        }
        let record = Activity {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: now(),
            original: source.to_string_lossy().into_owned(),
            destination: destination.to_string_lossy().into_owned(),
            folder: decision.folder.clone(),
            reason: decision.reason,
            status: "pending".into(),
            hash: hash.clone(),
            copied,
            duplicate,
        };
        state.store.activity.push(record);
        // Write the undo information before touching the document.
        self.persist(&state.store)?;
        let result = files::transfer(source, &destination, copied, &hash);
        let record = state.store.activity.last_mut().unwrap();
        match result {
            Ok(()) => {
                record.status = if decision.folder == "Needs Review" {
                    "review"
                } else {
                    "sorted"
                }
                .into()
            }
            Err(error) => {
                record.status = "failed".into();
                record.reason = error;
            }
        }
        if let Ok(sig) = files::signature(&destination) {
            state
                .store
                .seen
                .insert(destination.to_string_lossy().into_owned(), sig);
        }
        if let Ok(sig) = files::signature(source) {
            state
                .store
                .seen
                .insert(source.to_string_lossy().into_owned(), sig);
        }
        self.persist(&state.store)?;
        Ok(())
    }
    pub fn undo(&self, id: &str) -> Result<(), String> {
        let mut state = self.state.lock().unwrap();
        if state.processing.is_some() {
            return Err("Wait for the current document to finish before undoing".into());
        }
        let index = state
            .store
            .activity
            .iter()
            .position(|r| r.id == id)
            .ok_or("Activity not found")?;
        let record = state.store.activity[index].clone();
        if !matches!(record.status.as_str(), "sorted" | "review") {
            return Err("This action cannot be undone".into());
        }
        let destination = Path::new(&record.destination);
        files::signature(destination)?;
        if files::digest(destination)? != record.hash {
            return Err("This document has been edited since sorting. Undo stopped to preserve your changes.".into());
        }
        let original = Path::new(&record.original);
        if record.copied && files::digest(original).ok().as_deref() != Some(&record.hash) {
            return Err(
                "The original copy is missing or changed. Keeping the organized copy.".into(),
            );
        }
        if !record.copied && !original.parent().is_some_and(Path::is_dir) {
            return Err("Original folder is missing. Restore it before undoing.".into());
        }
        state.store.activity[index].status = "undo_pending".into();
        self.persist(&state.store)?;
        let result = if record.copied {
            fs::remove_file(destination).map_err(|e| e.to_string())
        } else {
            files::transfer(destination, original, false, &record.hash)
        };
        if let Err(error) = result {
            state.store.activity[index].status = record.status;
            self.persist(&state.store)?;
            return Err(error);
        }
        if let Ok(sig) = files::signature(original) {
            state.store.seen.insert(record.original.clone(), sig);
        }
        state.store.activity[index].status = "undone".into();
        state
            .store
            .queue
            .retain(|p| p != destination && p != original);
        self.persist(&state.store)?;
        Ok(())
    }
    pub fn start(engine: Arc<Self>) {
        std::thread::spawn(move || loop {
            let result = engine
                .discover()
                .and_then(|_| engine.process_next().map(|_| ()));
            if let Err(error) = result {
                let mut state = engine.state.lock().unwrap();
                state.error = Some(error);
                state.store.watching = false;
                let _ = engine.persist(&state.store);
            }
            std::thread::sleep(std::time::Duration::from_secs(2));
        });
    }
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn setup() -> (tempfile::TempDir, Engine) {
        let dir = tempfile::tempdir_in("/private/tmp").unwrap();
        let root = dir.path().join("documents");
        fs::create_dir(&root).unwrap();
        let engine = Engine::new(dir.path().join("state.json"), PathBuf::new()).unwrap();
        let p = Preferences {
            root: root.to_string_lossy().into_owned(),
            use_ai: false,
            ..Preferences::default()
        };
        engine.save_preferences(p).unwrap();
        (dir, engine)
    }
    #[test]
    fn watch_waits_for_stable_file_and_does_not_loop_after_move_or_undo() {
        let (_dir, engine) = setup();
        engine.watch(true).unwrap();
        let root = PathBuf::from(engine.snapshot().preferences.root);
        let source = root.join("scan.txt");
        fs::write(&source, "AGENCY residence permit").unwrap();
        engine.discover().unwrap();
        assert_eq!(engine.snapshot().pending, 0);
        engine.discover().unwrap();
        assert_eq!(engine.snapshot().pending, 1);
        engine.process_next().unwrap();
        let record = engine.snapshot().activity[0].clone();
        assert_eq!(record.folder, "Migration/AGENCY");
        assert!(!source.exists());
        assert!(Path::new(&record.destination).exists());
        engine.discover().unwrap();
        engine.discover().unwrap();
        assert_eq!(engine.snapshot().pending, 0);
        engine.undo(&record.id).unwrap();
        assert!(source.exists());
        engine.discover().unwrap();
        engine.discover().unwrap();
        assert_eq!(engine.snapshot().pending, 0);
    }
    #[test]
    fn existing_documents_wait_for_explicit_cleanup() {
        let (dir, engine) = setup();
        let other = dir.path().join("other");
        fs::create_dir(&other).unwrap();
        fs::write(other.join("old.txt"), "tax return").unwrap();
        let mut p = engine.snapshot().preferences;
        p.root = other.to_string_lossy().into_owned();
        engine.save_preferences(p).unwrap();
        engine.watch(true).unwrap();
        engine.discover().unwrap();
        engine.discover().unwrap();
        assert_eq!(engine.snapshot().pending, 0);
        assert_eq!(engine.organize_existing().unwrap(), 1);
        engine.process_next().unwrap();
        assert!(other.join("Finances/old.txt").exists());
    }
    #[test]
    fn undo_refuses_edited_files_and_occupied_original_paths() {
        let (_dir, engine) = setup();
        let root = PathBuf::from(engine.snapshot().preferences.root);
        let source = root.join("tax.txt");
        fs::write(&source, "tax return").unwrap();
        engine.enqueue(vec![source.clone()]).unwrap();
        engine.process_next().unwrap();
        let record = engine.snapshot().activity[0].clone();
        fs::write(&source, "another document").unwrap();
        assert!(engine.undo(&record.id).is_err());
        assert_eq!(fs::read_to_string(&source).unwrap(), "another document");
        fs::remove_file(&source).unwrap();
        fs::write(&record.destination, "edited").unwrap();
        assert!(engine.undo(&record.id).is_err());
        assert!(!source.exists());
    }
    #[test]
    fn external_import_preserves_original_and_undo_removes_only_copy() {
        let (dir, engine) = setup();
        let original = dir.path().join("external.txt");
        fs::write(&original, "unknown content").unwrap();
        engine.enqueue(vec![original.clone()]).unwrap();
        engine.process_next().unwrap();
        let record = engine.snapshot().activity[0].clone();
        assert!(record.copied);
        assert_eq!(record.status, "review");
        assert!(original.exists());
        engine.undo(&record.id).unwrap();
        assert!(original.exists());
        assert!(!Path::new(&record.destination).exists());
    }
    #[test]
    fn persistence_survives_restart_and_retains_duplicate_copies() {
        let (dir, engine) = setup();
        let original = dir.path().join("one.txt");
        let other = dir.path().join("two.txt");
        fs::write(&original, "insurance policy").unwrap();
        fs::write(&other, "insurance policy").unwrap();
        engine.enqueue(vec![original, other]).unwrap();
        engine.process_next().unwrap();
        engine.process_next().unwrap();
        let restarted = Engine::new(engine.storage.clone(), PathBuf::new()).unwrap();
        assert_eq!(restarted.snapshot().activity.len(), 2);
        assert!(restarted.snapshot().activity[0].duplicate);
        assert!(restarted
            .snapshot()
            .activity
            .iter()
            .all(|r| Path::new(&r.destination).exists()));
    }
    #[test]
    fn paused_queue_does_not_move_documents() {
        let (dir, engine) = setup();
        let original = dir.path().join("one.txt");
        fs::write(&original, "tax return").unwrap();
        engine.enqueue(vec![original.clone()]).unwrap();
        engine.watch(false).unwrap();
        assert!(!engine.process_next().unwrap());
        assert!(original.exists());
        assert_eq!(engine.snapshot().pending, 1);
    }
    #[test]
    fn queued_imports_survive_restart() {
        let (dir, engine) = setup();
        let original = dir.path().join("queued.txt");
        fs::write(&original, "tax return").unwrap();
        engine.enqueue(vec![original.clone()]).unwrap();
        let restarted = Engine::new(engine.storage.clone(), PathBuf::new()).unwrap();
        assert_eq!(restarted.snapshot().pending, 1);
        restarted.process_next().unwrap();
        assert_eq!(restarted.snapshot().activity[0].status, "sorted");
        assert!(original.exists());
    }
    #[test]
    fn journal_recovers_completed_move_after_interruption() {
        let (_dir, engine) = setup();
        let root = PathBuf::from(engine.snapshot().preferences.root);
        let original = root.join("bank.txt");
        fs::write(&original, "bank statement").unwrap();
        engine.enqueue(vec![original]).unwrap();
        engine.process_next().unwrap();
        {
            let mut state = engine.state.lock().unwrap();
            state.store.activity[0].status = "pending".into();
            engine.persist(&state.store).unwrap();
        }
        let restarted = Engine::new(engine.storage.clone(), PathBuf::new()).unwrap();
        assert_eq!(restarted.snapshot().activity[0].status, "sorted");
        restarted.discover().unwrap();
        restarted.discover().unwrap();
        assert_eq!(restarted.snapshot().pending, 0);
    }
    #[test]
    fn changing_file_waits_for_another_matching_scan() {
        let (_dir, engine) = setup();
        engine.watch(true).unwrap();
        let root = PathBuf::from(engine.snapshot().preferences.root);
        let source = root.join("download.txt");
        fs::write(&source, "part one").unwrap();
        engine.discover().unwrap();
        fs::write(&source, "part one and part two").unwrap();
        engine.discover().unwrap();
        assert_eq!(engine.snapshot().pending, 0);
        engine.discover().unwrap();
        assert_eq!(engine.snapshot().pending, 1);
    }
}
