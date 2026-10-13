use crate::{
    config, files, instructions,
    model::{Preferences, ReviewItem, Snapshot},
};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

/// Concurrent document reads while mapping; OCR scaled about 3x up to four.
const READ_WORKERS: usize = 4;

struct Runtime {
    preferences: Preferences,
    items: Vec<ReviewItem>,
    processing: Option<String>,
    error: Option<String>,
    notice: Option<String>,
    generation: u64,
    collection: Option<crate::collection::Collection>,
    running: bool,
    watching: bool,
    run_revision: String,
    seen: BTreeMap<String, String>,
    pending: BTreeMap<String, (String, Instant)>,
}
pub struct Engine {
    state: Mutex<Runtime>,
    directory: PathBuf,
    reader: PathBuf,
    #[cfg(test)]
    test_decision: Mutex<Option<crate::filing::Decision>>,
}
impl Engine {
    pub fn new(directory: PathBuf, reader: PathBuf) -> Result<Self, String> {
        fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        let mut preferences = Preferences::default();
        // Legacy state is read only for preferences. Never resume old watcher work or replay history.
        let pointer = config::read_json(&directory.join("preferences.json"))?;
        if let Some(value) = pointer.as_ref() {
            preferences.root = value["root"]
                .as_str()
                .ok_or("Invalid saved destination")?
                .into();
        } else {
            let registry = config::read_json(&directory.join("folders.json"))?;
            let selected = registry
                .as_ref()
                .and_then(|r| r["selected"].as_str())
                .unwrap_or("primary");
            let legacy = if selected == "primary" || selected.is_empty() {
                directory.join("state.json")
            } else {
                uuid::Uuid::parse_str(selected).map_err(|_| "Invalid legacy folder ID")?;
                directory.join("folders").join(format!("{selected}.json"))
            };
            if let Some(value) = config::read_json(&legacy)? {
                preferences = serde_json::from_value(value["preferences"].clone())
                    .map_err(|e| format!("Invalid legacy preferences: {e}"))?;
            }
        }
        let mut error = None;
        if !preferences.root.is_empty() {
            match crate::native::folder_access(&preferences.root, &directory, true) {
                Ok(path) => preferences.root = path.to_string_lossy().into_owned(),
                Err(e) => error = Some(e),
            }
        }
        if !preferences.root.is_empty() && error.is_none() {
            let result = if pointer.is_some() {
                instructions::recover(Path::new(&preferences.root))
                    .and_then(|_| config::load(&mut preferences))
            } else {
                config::initialize(&mut preferences)
            };
            if let Err(e) = result {
                error = Some(e);
            } else {
                crate::native::folder_access(&preferences.root, &directory, false)?;
                config::atomic_json(
                    &directory.join("preferences.json"),
                    &serde_json::json!({"root":preferences.root}),
                )?;
            }
        }
        Ok(Self {
            state: Mutex::new(Runtime {
                preferences,
                items: vec![],
                processing: None,
                error,
                notice: None,
                generation: 0,
                collection: None,
                running: false,
                watching: false,
                run_revision: String::new(),
                seen: BTreeMap::new(),
                pending: BTreeMap::new(),
            }),
            directory,
            reader,
            #[cfg(test)]
            test_decision: Mutex::new(None),
        })
    }
    fn refresh(state: &mut Runtime) -> Result<(), String> {
        if state.preferences.root.is_empty() {
            return Err("Choose a destination folder first".into());
        }
        let mut p = state.preferences.clone();
        match config::load(&mut p) {
            Ok(()) => {
                if state.running && state.run_revision != config::revision(&p) {
                    state.running = false;
                    state.watching = false;
                    state.notice = Some(
                        "Folder settings changed, so Tuck stopped. Click Done and organize again."
                            .into(),
                    );
                }
                state.preferences = p;
                state.error = None;
                Ok(())
            }
            Err(e) => {
                state.error = Some(e.clone());
                state.running = false;
                state.watching = false;
                Err(e)
            }
        }
    }
    pub fn snapshot(&self) -> Snapshot {
        let mut state = self.state.lock().unwrap();
        if !state.preferences.root.is_empty() {
            let _ = Self::refresh(&mut state);
        }
        Snapshot {
            collection: state.collection.as_ref().map(|m| m.view()),
            running: state.running,
            watching: state.watching,
            preferences: state.preferences.clone(),
            items: state.items.clone(),
            processing: state.processing.clone(),
            error: state.error.clone().or_else(|| state.notice.clone()),
        }
    }
    pub fn preview_path(&self, id: &str) -> Result<PathBuf, String> {
        let state = self.state.lock().unwrap();
        let item = state
            .items
            .iter()
            .find(|item| item.id == id)
            .ok_or("Document is no longer in this session")?;
        let path = PathBuf::from(if item.status == "organized" {
            &item.destination
        } else {
            &item.original
        });
        let metadata =
            fs::symlink_metadata(&path).map_err(|_| "This document is no longer available")?;
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err("Choose a regular document to preview".into());
        }
        Ok(path)
    }
    pub fn choose_folder(&self, root: &str) -> Result<(), String> {
        let root = Path::new(root)
            .canonicalize()
            .map_err(|_| "Choose an existing folder")?;
        if !root.is_dir() || root.parent().is_none() {
            return Err("Choose a document folder".into());
        }
        let mut state = self.state.lock().unwrap();
        if !state.items.is_empty() || state.processing.is_some() {
            return Err("Clear the current batch before changing the destination".into());
        }
        let mut p = Preferences {
            root: root.to_string_lossy().into_owned(),
            rules: vec![],
            ..Preferences::default()
        };
        config::initialize(&mut p)?;
        crate::native::folder_access(&p.root, &self.directory, false)?;
        config::atomic_json(
            &self.directory.join("preferences.json"),
            &serde_json::json!({"root":p.root}),
        )?;
        state.preferences = p;
        state.collection = None;
        state.notice = None;
        state.running = false;
        state.watching = false;
        state.seen.clear();
        state.pending.clear();
        state.error = None;
        Ok(())
    }
    pub fn save_preferences(&self, p: Preferences) -> Result<(), String> {
        self.save_preferences_with_key(p, None)
    }
    pub fn save_preferences_with_key(
        &self,
        mut p: Preferences,
        api_key: Option<String>,
    ) -> Result<(), String> {
        files::validate(&p)?;
        let mut state = self.state.lock().unwrap();
        Self::refresh(&mut state)?;
        if p.root != state.preferences.root {
            return Err("Choose the destination using the folder picker".into());
        }
        if p.settings_revision != state.preferences.settings_revision {
            return Err("Settings changed on disk. Discard your changes and try again.".into());
        }
        if let Some(key) = api_key.filter(|k| !k.is_empty()) {
            crate::files::validate_connection(&p.connection)?;
            crate::helper::invoke(
                &self.reader.with_file_name("tuck-apple"),
                Some(
                    serde_json::json!({"model":"custom","operation":"save_key","connection":p.connection,"apiKey":key}),
                ),
                std::time::Duration::from_secs(30),
            )?;
        }
        instructions::save(&p, &p.instructions_revision)?.apply(&mut p);
        config::save(&p)?;
        config::load(&mut p)?;
        if let Some(map) = state.collection.take() {
            state.collection = Some(crate::collection::build(map.documents, &p));
        }
        state.preferences = p;
        state.running = false;
        state.watching = false;
        Ok(())
    }
    pub fn suggest_categories(&self) -> Result<crate::suggest::Proposal, String> {
        let needs_map = {
            let state = self.state.lock().unwrap();
            let paths = files::existing_documents(Path::new(&state.preferences.root))?;
            state.collection.as_ref().is_none_or(|map| {
                paths.len() != map.documents.len()
                    || paths.iter().zip(&map.documents).any(|(p, d)| {
                        p != Path::new(&d.path)
                            || files::signature(p).ok().as_ref() != Some(&d.signature)
                    })
            })
        };
        if needs_map {
            self.map_folder()?;
        }
        let p = {
            let mut state = self.state.lock().unwrap();
            Self::refresh(&mut state)?;
            if !state.preferences.use_ai {
                return Err("Turn on the AI model in Settings to suggest folders.".into());
            }
            if state.processing.is_some() {
                return Err("Wait for the current document to finish.".into());
            }
            state.processing = Some("Suggesting categories".into());
            state.preferences.clone()
        };
        let collection = self.state.lock().unwrap().collection.clone();
        let result = match collection {
            Some(map) => {
                let generation = self.state.lock().unwrap().generation;
                crate::suggest::suggest(&p, &self.reader, &map, |message| {
                    let mut state = self.state.lock().unwrap();
                    if state.generation != generation {
                        return Err("Suggestions cancelled.".into());
                    }
                    state.processing = Some(message);
                    Ok(())
                })
            }
            None => Err("Choose a folder before asking for suggestions.".into()),
        };
        let mut state = self.state.lock().unwrap();
        state.processing = None;
        Self::refresh(&mut state)?;
        if config::revision(&p) != config::revision(&state.preferences) {
            return Err(
                "Folder settings changed during analysis. Suggest categories again.".into(),
            );
        }
        result
    }
    pub fn map_folder(&self) -> Result<(), String> {
        let (p, generation) = {
            let mut state = self.state.lock().unwrap();
            Self::refresh(&mut state)?;
            if state.processing.is_some() || !state.items.is_empty() || state.running {
                return Err("Stop Tuck and clear the session before mapping again.".into());
            }
            state.collection = None;
            state.notice = None;
            state.processing = Some("Mapping documents".into());
            (state.preferences.clone(), state.generation)
        };
        let result: Result<crate::collection::Collection, String> = (|| {
            let paths = files::existing_documents(Path::new(&p.root))?;
            let total = paths.len();
            // Text recognition runs several documents at once; order is preserved.
            let documents = crate::parallel::map(
                &paths,
                READ_WORKERS,
                |path| crate::collection::read(path, Path::new(&p.root), &self.reader),
                |done| {
                    let mut state = self.state.lock().unwrap();
                    if state.generation != generation {
                        return Err("Mapping cancelled.".into());
                    }
                    state.processing = Some(format!("Reading document {done} of {total}"));
                    Ok(())
                },
            )?;
            Ok(crate::collection::build(documents, &p))
        })();
        let mut state = self.state.lock().unwrap();
        state.processing = None;
        if state.generation != generation {
            return Err("Mapping cancelled.".into());
        }
        Self::refresh(&mut state)?;
        if config::revision(&state.preferences) != config::revision(&p) {
            return Err(
                "Folder rules changed while Tuck was reading. Choose the folder again.".into(),
            );
        }
        state.collection = Some(result?);
        Ok(())
    }
    pub fn start_tuck(&self, watch: bool) -> Result<usize, String> {
        let mut state = self.state.lock().unwrap();
        Self::refresh(&mut state)?;
        if state.processing.is_some() || !state.items.is_empty() {
            return Err("Finish or clear the current session first.".into());
        }
        if !state
            .preferences
            .rules
            .iter()
            .any(|r| state.preferences.rule_enabled(r))
        {
            return Err("Choose a folder structure first.".into());
        }
        let map = Self::unchanged_map(&mut state)?;
        let eligible: Vec<_> = map
            .documents
            .iter()
            .map(|d| PathBuf::from(&d.path))
            .filter(|p| fs::metadata(p).is_ok_and(|m| m.len() <= 100 * 1024 * 1024))
            .collect();
        let count = Self::enqueue_into(&mut state, eligible)?;
        state.seen = map
            .documents
            .iter()
            .map(|d| (d.path.clone(), d.signature.clone()))
            .collect();
        state.pending.clear();
        state.run_revision = config::revision(&state.preferences);
        state.notice = None;
        state.running = true;
        state.watching = watch;
        Ok(count)
    }
    /// The mapped collection, provided the folder has not changed since mapping.
    fn unchanged_map(state: &mut Runtime) -> Result<crate::collection::Collection, String> {
        let map = state
            .collection
            .clone()
            .ok_or("Choose a folder before organizing.")?;
        let paths = files::existing_documents(Path::new(&state.preferences.root))?;
        if paths.len() != map.documents.len()
            || paths.iter().zip(&map.documents).any(|(p, d)| {
                p != Path::new(&d.path) || files::signature(p).ok().as_ref() != Some(&d.signature)
            })
        {
            state.collection = None;
            return Err(
                "Documents changed since Tuck read the folder. Choose the folder again.".into(),
            );
        }
        Ok(map)
    }
    pub fn stop_tuck(&self) {
        let mut state = self.state.lock().unwrap();
        if state
            .processing
            .as_ref()
            .is_some_and(|p| p.starts_with("Mapping"))
        {
            state.generation += 1;
        }
        for item in state.items.iter_mut().filter(|i| i.status == "queued") {
            item.status = "skipped".into();
            item.reason = "Tuck stopped before reading this document.".into();
        }
        state.running = false;
        state.watching = false;
    }
    pub fn import(&self, paths: Vec<PathBuf>) -> Result<usize, String> {
        let mut state = self.state.lock().unwrap();
        Self::refresh(&mut state)?;
        if !state.running {
            return Err("Organize the folder before adding files.".into());
        }
        Self::enqueue_into(&mut state, paths)
    }
    pub fn clear(&self) {
        let mut state = self.state.lock().unwrap();
        state.items.clear();
        state.notice = None;
        state.running = false;
        state.watching = false;
        state.collection = None;
        state.seen.clear();
        state.pending.clear();
        // An in-flight analysis is allowed to finish, but its result cannot enter the new batch.
        state.generation += 1;
    }
    pub fn skip(&self, id: &str) -> Result<(), String> {
        let mut state = self.state.lock().unwrap();
        let item = state
            .items
            .iter_mut()
            .find(|i| i.id == id)
            .ok_or("Document not found")?;
        if item.status == "organized" {
            return Err("This document is already organized".into());
        }
        item.status = "skipped".into();
        Ok(())
    }
    pub fn start(engine: Arc<Self>) {
        std::thread::spawn(move || {
            let mut watcher = crate::watching::FolderWatch::new();
            loop {
                let (root, pending) = {
                    let state = engine.state.lock().unwrap();
                    let root = if state.running && state.watching {
                        state.preferences.root.clone()
                    } else {
                        String::new()
                    };
                    (root, !state.pending.is_empty())
                };
                if watcher.due(&root, pending) {
                    if let Err(e) = engine.watch_tick() {
                        let mut state = engine.state.lock().unwrap();
                        state.running = false;
                        state.watching = false;
                        state.notice = Some(e);
                    }
                }
                if let Err(e) = engine.process_next() {
                    let mut state = engine.state.lock().unwrap();
                    state.running = false;
                    state.watching = false;
                    state.notice = Some(e);
                }
                std::thread::sleep(Duration::from_millis(250));
            }
        });
    }
}

mod organize;
mod queue;
#[cfg(test)]
mod tests;
