use crate::{
    config, files, instructions,
    model::{Approval, Preferences, ReviewItem, Snapshot},
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
                    state.notice = Some("Folder settings changed. Tuck stopped; clear the session and map again to restart.".into());
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
            return Err("Settings changed on disk. Reload preferences before saving.".into());
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
                return Err("Enable local AI in Preferences to suggest categories.".into());
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
            None => Err("Map the folder before asking for suggestions.".into()),
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
            return Err("Rules changed during mapping. Map the folder again.".into());
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
    /// Applies an approved suggestion: files the suggestion placed move straight to their
    /// folders through the same checks as manual filing; the rest wait for a decision.
    /// Tuck then keeps running so dropped (and, with `watch`, new) files are filed too.
    pub fn organize_proposal(
        &self,
        placements: Vec<crate::suggest::Placement>,
        watch: bool,
    ) -> Result<usize, String> {
        let mut state = self.state.lock().unwrap();
        Self::refresh(&mut state)?;
        if state.processing.is_some() || !state.items.is_empty() {
            return Err("Finish or clear the current session first.".into());
        }
        let map = Self::unchanged_map(&mut state)?;
        let p = state.preferences.clone();
        let revision = config::revision(&p);
        let placements: BTreeMap<&str, &crate::suggest::Placement> =
            placements.iter().map(|pl| (pl.path.as_str(), pl)).collect();
        let mut items = Vec::new();
        for doc in &map.documents {
            let placement = placements.get(doc.path.as_str()).copied();
            let original = Path::new(&doc.path);
            let stem = original
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            let mut item = ReviewItem {
                id: uuid::Uuid::new_v4().to_string(),
                original: doc.path.clone(),
                destination: String::new(),
                folder: placement.map(|pl| pl.folder.clone()).unwrap_or_default(),
                title: placement
                    .map(|pl| files::safe_stem(&pl.title))
                    .filter(|t| p.rename && !t.is_empty())
                    .unwrap_or_else(|| files::safe_stem(&stem)),
                reason: doc
                    .error
                    .clone()
                    .unwrap_or_else(|| "Choose a folder for this document.".into()),
                status: "review".into(),
                copied: false,
                confident: false,
                hash: doc.hash.clone(),
                revision: revision.clone(),
            };
            if doc.error.is_none() && placement.is_some() {
                match Self::destination(&p, &item) {
                    Ok(destination) if doc.duplicate_of.is_none() => {
                        item.destination = destination;
                        item.reason = format!("Suggested folder: {}.", item.folder);
                        item.confident = true;
                    }
                    Ok(destination) => {
                        item.destination = destination;
                        item.reason = "Identical content exists elsewhere in this folder. Choose whether to keep this copy.".into();
                    }
                    Err(e) => item.reason = e,
                }
            }
            items.push(item);
        }
        state.items = items;
        state.seen = map
            .documents
            .iter()
            .map(|d| (d.path.clone(), d.signature.clone()))
            .collect();
        state.pending.clear();
        let ready: Vec<String> = state
            .items
            .iter()
            .filter(|i| i.confident)
            .map(|i| i.id.clone())
            .collect();
        // Destinations are recomputed just before each move so documents with the same
        // suggested name get distinct files. A failure stays on its file; others continue.
        let mut organized = 0;
        for id in ready {
            let Some(item) = state.items.iter_mut().find(|i| i.id == id) else {
                continue;
            };
            match Self::destination(&p, item) {
                Ok(destination) => item.destination = destination.clone(),
                Err(e) => {
                    item.confident = false;
                    item.reason = e;
                    continue;
                }
            }
            let approval = Approval {
                id: id.clone(),
                destination: item.destination.clone(),
            };
            if Self::organize_locked(&mut state, vec![approval]).is_ok() {
                organized += 1;
            } else if let Some(item) = state.items.iter_mut().find(|i| i.id == id) {
                // organize_locked already recorded the reason and marked it failed.
                item.confident = false;
            }
        }
        state.run_revision = revision;
        state.notice = None;
        state.running = true;
        state.watching = watch;
        Ok(organized)
    }
    /// The mapped collection, provided the folder has not changed since mapping.
    fn unchanged_map(state: &mut Runtime) -> Result<crate::collection::Collection, String> {
        let map = state
            .collection
            .clone()
            .ok_or("Map the folder before organizing.")?;
        let paths = files::existing_documents(Path::new(&state.preferences.root))?;
        if paths.len() != map.documents.len()
            || paths.iter().zip(&map.documents).any(|(p, d)| {
                p != Path::new(&d.path) || files::signature(p).ok().as_ref() != Some(&d.signature)
            })
        {
            state.collection = None;
            return Err(
                "Documents changed since mapping. Map the folder again before organizing.".into(),
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
            return Err("Choose your structure and click Start Tuck before adding files.".into());
        }
        Self::enqueue_into(&mut state, paths)
    }
    fn watch_tick(&self) -> Result<(), String> {
        let mut state = self.state.lock().unwrap();
        if !state.running || !state.watching || state.processing.is_some() {
            return Ok(());
        }
        Self::refresh(&mut state)?;
        if !state.running {
            return Ok(());
        }
        let paths = files::existing_documents(Path::new(&state.preferences.root))?;
        let current: BTreeMap<_, _> = paths
            .iter()
            .map(|p| Ok((p.to_string_lossy().into_owned(), files::signature(p)?)))
            .collect::<Result<_, String>>()?;
        state.seen.retain(|p, _| current.contains_key(p));
        state.pending.retain(|p, _| current.contains_key(p));
        for (path, signature) in current {
            if state.seen.get(&path) == Some(&signature) {
                continue;
            }
            let stable = match state.pending.get(&path) {
                Some((previous, since)) if previous == &signature => {
                    since.elapsed() >= Duration::from_secs(6)
                }
                _ => {
                    state
                        .pending
                        .insert(path.clone(), (signature.clone(), Instant::now()));
                    false
                }
            };
            if !stable
                || state.items.iter().any(|i| {
                    i.original == path && matches!(i.status.as_str(), "queued" | "reading")
                })
            {
                continue;
            }
            if fs::metadata(&path).map_err(|e| e.to_string())?.len() > 100 * 1024 * 1024 {
                state.seen.insert(path.clone(), signature);
                state.pending.remove(&path);
                continue;
            }
            // A changed document must be read again; old decisions cannot be reused.
            state.items.retain(|i| i.original != path);
            if let Some(map) = state.collection.as_mut() {
                map.documents.retain(|d| d.path != path);
            }
            Self::enqueue_into(&mut state, vec![PathBuf::from(&path)])?;
            state.seen.insert(path.clone(), signature);
            state.pending.remove(&path);
        }
        Ok(())
    }
    #[cfg(test)]
    pub fn enqueue(&self, paths: Vec<PathBuf>) -> Result<usize, String> {
        let mut state = self.state.lock().unwrap();
        Self::refresh(&mut state)?;
        Self::enqueue_into(&mut state, paths)
    }
    fn enqueue_into(state: &mut Runtime, paths: Vec<PathBuf>) -> Result<usize, String> {
        let mut valid = Vec::new();
        for path in paths {
            files::signature(&path)?;
            if !files::supported(&path) {
                return Err("Choose PDF, PNG, JPG, HEIC, TIFF, TXT or Markdown files".into());
            }
            let path = path.canonicalize().map_err(|e| e.to_string())?;
            if !files::supported(&path) {
                return Err("Tuck settings are not documents".into());
            }
            if fs::metadata(&path).map_err(|e| e.to_string())?.len() > 100 * 1024 * 1024 {
                return Err("Choose documents smaller than 100 MB".into());
            }
            if !valid.contains(&path) {
                valid.push(path);
            }
        }
        valid.retain(|path| !state.items.iter().any(|i| Path::new(&i.original) == path));
        if state.items.len() + valid.len() > 2000 {
            return Err(
                "A session supports up to 2,000 documents. Stop and clear the session before adding more.".into(),
            );
        }
        let count = valid.len();
        for path in valid {
            let copied = !path.starts_with(&state.preferences.root);
            state.items.push(ReviewItem {
                id: uuid::Uuid::new_v4().to_string(),
                original: path.to_string_lossy().into_owned(),
                destination: String::new(),
                folder: String::new(),
                title: path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned(),
                reason: String::new(),
                status: "queued".into(),
                copied,
                confident: false,
                hash: String::new(),
                revision: String::new(),
            });
        }
        Ok(count)
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
    pub fn process_next(&self) -> Result<bool, String> {
        let (item, p, generation) = {
            let mut state = self.state.lock().unwrap();
            if state.processing.is_some() || !state.items.iter().any(|i| i.status == "queued") {
                return Ok(false);
            }
            Self::refresh(&mut state)?;
            let item = state
                .items
                .iter_mut()
                .find(|i| i.status == "queued")
                .unwrap();
            item.status = "reading".into();
            let item = item.clone();
            state.processing = Some(item.original.clone());
            (item, state.preferences.clone(), state.generation)
        };
        let result = self.analyze(&item, &p);
        let mut state = self.state.lock().unwrap();
        state.processing = None;
        if generation != state.generation {
            return Ok(true);
        }
        let refreshed = Self::refresh(&mut state);
        let revision = config::revision(&state.preferences);
        if let Some(current) = state
            .items
            .iter_mut()
            .find(|i| i.id == item.id && i.status == "reading")
        {
            match result {
                Ok(proposal) if refreshed.is_ok() && revision == config::revision(&p) => {
                    *current = proposal
                }
                Ok(_) => {
                    current.status = "failed".into();
                    current.reason =
                        "Preferences changed while reading. Analyze this document again.".into();
                }
                Err(e) => {
                    current.status = "failed".into();
                    current.reason = e;
                }
            }
        }
        if state.running && state.run_revision == revision {
            let approval = state
                .items
                .iter()
                .find(|i| {
                    i.id == item.id
                        && i.status == "review"
                        && i.confident
                        && !i.destination.is_empty()
                })
                .map(|i| Approval {
                    id: i.id.clone(),
                    destination: i.destination.clone(),
                });
            if let Some(approval) = approval {
                if let Err(error) = Self::organize_locked(&mut state, vec![approval]) {
                    if let Some(current) = state.items.iter_mut().find(|i| i.id == item.id) {
                        current.confident = false;
                        current.reason = error;
                    }
                }
            }
        }
        Ok(true)
    }
    fn analyze(&self, item: &ReviewItem, p: &Preferences) -> Result<ReviewItem, String> {
        let source = Path::new(&item.original);
        let before = files::signature(source)?;
        if fs::metadata(source).map_err(|e| e.to_string())?.len() > 100 * 1024 * 1024 {
            return Err("Document is now larger than 100 MB; original kept".into());
        }
        let hash = files::digest(source)?;
        let mapped = self
            .state
            .lock()
            .unwrap()
            .collection
            .as_ref()
            .and_then(|m| m.documents.iter().find(|d| d.path == item.original))
            .cloned();
        let already_filed = self
            .state
            .lock()
            .unwrap()
            .items
            .iter()
            .any(|i| i.id != item.id && i.status == "organized" && i.hash == hash);
        let duplicate = already_filed || mapped.as_ref().is_some_and(|d| d.duplicate_of.is_some());
        let reading = match mapped {
            Some(doc) if doc.hash == hash => match doc.error {
                Some(error) => Err(error),
                None => Ok(crate::reader::Reading {
                    text: doc.text,
                    clues: doc.clues,
                }),
            },
            _ => crate::reader::read(source, &self.reader),
        };
        let decision = match reading {
            Ok(reading) => crate::filing::decide(
                &reading.text,
                &reading.clues,
                p,
                Some((source, &self.reader)),
            ),
            Err(e) => crate::filing::Decision {
                folder: "Needs Review".into(),
                title: String::new(),
                reason: e,
                confident: false,
            },
        };
        #[cfg(test)]
        let decision = self
            .test_decision
            .lock()
            .unwrap()
            .clone()
            .map(|d| crate::filing::finalize(Ok(d), p))
            .unwrap_or(decision);
        if files::signature(source)? != before || files::digest(source)? != hash {
            return Err("Document changed during reading. Analyze it again.".into());
        }
        let mut result = item.clone();
        result.status = "review".into();
        result.hash = hash;
        result.revision = config::revision(p);
        result.reason = decision.reason;
        result.confident = decision.confident;
        result.folder = if decision.confident {
            decision.folder
        } else {
            String::new()
        };
        if p.rename && decision.confident && !decision.title.is_empty() {
            let title = Path::new(&decision.title);
            result.title = if title.extension().is_some_and(|e| {
                source
                    .extension()
                    .is_some_and(|ext| e.eq_ignore_ascii_case(ext))
            }) {
                title
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned()
            } else {
                decision.title
            };
        }
        if !result.folder.is_empty() {
            match Self::destination(p, &result) {
                Ok(path) => result.destination = path,
                Err(e) => {
                    result.reason = format!("{} {e}", result.reason);
                    result.confident = false;
                }
            }
        }
        if duplicate {
            result.confident = false;
            result.reason = format!("Identical content exists elsewhere in this folder. Choose whether to keep this copy. {}", result.reason);
        }
        Ok(result)
    }
    fn destination(p: &Preferences, item: &ReviewItem) -> Result<String, String> {
        if !p
            .rules
            .iter()
            .any(|r| r.folder == item.folder && p.rule_enabled(r))
            || !files::valid_folder(&item.folder, p.max_depth)
        {
            return Err("Choose an enabled destination folder".into());
        }
        if item.title.is_empty() || files::safe_stem(&item.title) != item.title {
            return Err(
                "Use a filename without extension, special characters, or more than 100 characters"
                    .into(),
            );
        }
        let source = Path::new(&item.original);
        let directory = Path::new(&p.root).join(&item.folder);
        Ok(files::unique_destination(
            &directory,
            &item.title,
            &source.extension().unwrap_or_default().to_string_lossy(),
            source,
        )
        .to_string_lossy()
        .into_owned())
    }
    pub fn update_review(&self, id: &str, folder: &str, title: &str) -> Result<(), String> {
        let mut state = self.state.lock().unwrap();
        Self::refresh(&mut state)?;
        let p = state.preferences.clone();
        let item = state
            .items
            .iter_mut()
            .find(|i| i.id == id)
            .ok_or("Document not found")?;
        if item.status != "review" {
            return Err("Only documents awaiting review can be edited".into());
        }
        if item.revision != config::revision(&p) {
            return Err(
                "Preferences changed. Analyze this document again before organizing.".into(),
            );
        }
        let mut next = item.clone();
        next.folder = folder.into();
        next.title = title.trim().into();
        next.destination = Self::destination(&p, &next)?;
        *item = next;
        Ok(())
    }
    pub fn organize(&self, approvals: Vec<Approval>) -> Result<(), String> {
        let mut state = self.state.lock().unwrap();
        Self::refresh(&mut state)?;
        Self::organize_locked(&mut state, approvals)
    }
    fn organize_locked(state: &mut Runtime, approvals: Vec<Approval>) -> Result<(), String> {
        let p = state.preferences.clone();
        for approval in approvals {
            let item = state
                .items
                .iter_mut()
                .find(|i| i.id == approval.id)
                .ok_or("Document not found")?;
            if item.status != "review" || item.destination.is_empty() {
                return Err("Review the destination before organizing".into());
            }
            if item.destination != approval.destination {
                return Err("The preview changed. Review it before organizing.".into());
            }
            if item.revision != config::revision(&p) {
                return Err(
                    "Preferences changed. Analyze this document again before organizing.".into(),
                );
            }
            let fresh = Self::destination(&p, item)?;
            if fresh != item.destination {
                item.destination = fresh;
                return Err("A destination is now occupied. The preview has been updated; review it and organize again.".into());
            }
            let result = (|| {
                let source = Path::new(&item.original);
                files::signature(source)?;
                if source.canonicalize().map_err(|e| e.to_string())? != source
                    || files::digest(source)? != item.hash
                {
                    return Err("Source changed or moved. Analyze it again; original kept.".into());
                }
                let root = Path::new(&p.root);
                if root.canonicalize().map_err(|e| e.to_string())? != root {
                    return Err("Destination folder changed location. Choose it again.".into());
                }
                files::target_directory(root, &item.folder)?;
                if source != Path::new(&item.destination) {
                    files::transfer(
                        source,
                        Path::new(&item.destination),
                        item.copied,
                        &item.hash,
                    )?;
                }
                Ok::<(), String>(())
            })();
            match result {
                Ok(()) => {
                    item.status = "organized".into();
                    state.seen.remove(&item.original);
                    state.pending.remove(&item.original);
                    if let Ok(signature) = files::signature(Path::new(&item.destination)) {
                        state.seen.insert(item.destination.clone(), signature);
                        state.pending.remove(&item.destination);
                    }
                }
                Err(e) => {
                    item.status = "failed".into();
                    item.reason = e.clone();
                    return Err(e);
                }
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "Requires Apple Intelligence; synthetic automatic start and fallback"]
    fn live_apple_automatic_flow() {
        let (_temp, engine, root) = setup();
        *engine.test_decision.lock().unwrap() = None;
        let mut p = engine.snapshot().preferences;
        p.use_ai = true;
        p.instructions = "Organize documents by their contents. Keep filenames unchanged.".into();
        p.rename = false;
        p.rules
            .retain(|r| matches!(r.folder.as_str(), "Banking & Taxes" | "Employment & Pay"));

        engine.save_preferences(p).unwrap();
        let engine = Engine::new(
            engine.directory.clone(),
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/tuck-reader"),
        )
        .unwrap();
        let source = root.join("statement.txt");
        fs::write(&source, "BANK ACCOUNT STATEMENT\nExample Bank\nStatement period: January 2026\nOpening balance: SEK 1000\nDeposits: SEK 2500\nClosing balance: SEK 3500\nThis is a bank account statement showing account transactions. Synthetic test document.").unwrap();
        fs::write(root.join("unreadable.txt"), "").unwrap();
        let hash = files::digest(&source).unwrap();
        engine.map_folder().unwrap();
        assert!(source.exists());
        assert_eq!(engine.snapshot().collection.unwrap().unreadable, 1);
        engine.start_tuck(true).unwrap();
        engine.process_next().unwrap();
        engine.process_next().unwrap();
        let state = engine.snapshot();
        let filed = &state.items[0];
        assert_eq!(filed.status, "organized", "Apple: {}", filed.reason);
        assert_eq!(filed.folder, "Banking & Taxes");
        assert_eq!(files::digest(Path::new(&filed.destination)).unwrap(), hash);
        assert!(!source.exists());
        assert_eq!(state.items[1].status, "review");
        assert!(root.join("unreadable.txt").exists());
        engine.watch_tick().unwrap();
        assert!(engine.state.lock().unwrap().pending.is_empty());
        println!("Synthetic automatic flow: 1 bank statement organized with matching hash; 1 unreadable file retained; own output ignored by watcher.");
    }
    #[test]
    fn mapping_reads_every_document_without_moving_and_start_enables_automatic_filing() {
        let (_temp, engine, root) = setup();
        for n in 0..15 {
            fs::write(
                root.join(format!("source-{n:02}.txt")),
                format!("Synthetic bank statement {n}"),
            )
            .unwrap();
        }
        engine.map_folder().unwrap();
        let state = engine.snapshot();
        let map = state.collection.unwrap();
        assert_eq!(map.documents.len(), 15);
        assert!(engine
            .state
            .lock()
            .unwrap()
            .collection
            .as_ref()
            .unwrap()
            .documents
            .iter()
            .all(|d| !d.text.is_empty()));
        assert!(state.items.is_empty());
        assert!(!state.running);
        assert!(!engine.process_next().unwrap());
        assert_eq!(engine.start_tuck(false).unwrap(), 15);
        assert!(engine.process_next().unwrap());
        let item = engine.snapshot().items[0].clone();
        assert_eq!(item.status, "organized");
        assert!(!Path::new(&item.original).exists());
        assert!(Path::new(&item.destination).exists());
        assert!(root.join("source-01.txt").exists());
        engine.stop_tuck();
        assert!(!engine.process_next().unwrap());
        assert_eq!(engine.snapshot().items[1].status, "skipped");
        assert!(root.join("source-01.txt").exists());
    }
    #[test]
    fn mapping_includes_empty_structure_and_duplicate_edges_and_falls_back() {
        let (_temp, engine, root) = setup();
        fs::write(root.join("a.txt"), "Same contents").unwrap();
        fs::write(root.join("b.txt"), "Same contents").unwrap();
        fs::create_dir(root.join("Education")).unwrap();
        engine.map_folder().unwrap();
        let map = engine.snapshot().collection.unwrap();
        assert_eq!(map.duplicates, 1);
        assert!(map.existing_rules.iter().any(|r| r.folder == "Education"));
        assert!(map.documents[1].duplicate_of.is_some());
        engine.start_tuck(false).unwrap();
        engine.process_next().unwrap();
        engine.process_next().unwrap();
        assert_eq!(engine.snapshot().items[0].status, "organized");
        assert_eq!(engine.snapshot().items[1].status, "review");
        assert!(root.join("b.txt").exists());
    }
    #[test]
    fn start_rejects_stale_map_and_rules_changes_stop_automatic_filing() {
        let (_temp, engine, root) = setup();
        let path = root.join("a.txt");
        fs::write(&path, "original").unwrap();
        engine.map_folder().unwrap();
        fs::write(&path, "new changed contents").unwrap();
        assert!(engine
            .start_tuck(true)
            .unwrap_err()
            .contains("changed since mapping"));
        assert!(!engine.snapshot().running);
        engine.map_folder().unwrap();
        engine.start_tuck(true).unwrap();
        fs::write(root.join(".tuck/instructions.md"), "Changed rules").unwrap();
        engine.process_next().unwrap();
        assert!(!engine.snapshot().running);
        assert!(!engine.snapshot().watching);
        assert!(path.exists());
        assert_eq!(engine.snapshot().items[0].status, "review");
    }
    #[test]
    fn watcher_settles_changes_ignores_own_moves_and_never_restarts_itself() {
        let (temp, engine, root) = setup();
        engine.map_folder().unwrap();
        engine.start_tuck(true).unwrap();
        let path = root.join("new.txt");
        fs::write(&path, "new bank statement").unwrap();
        engine.watch_tick().unwrap();
        assert!(engine.snapshot().items.is_empty());
        let age = |engine: &Engine| {
            for (_, since) in engine.state.lock().unwrap().pending.values_mut() {
                *since = Instant::now() - Duration::from_secs(7);
            }
        };
        age(&engine);
        fs::write(&path, "still being copied: more bytes").unwrap();
        engine.watch_tick().unwrap();
        assert!(engine.snapshot().items.is_empty());
        age(&engine);
        engine.watch_tick().unwrap();
        assert_eq!(engine.snapshot().items.len(), 1);
        engine.process_next().unwrap();
        let item = engine.snapshot().items[0].clone();
        assert_eq!(item.status, "organized");
        engine.watch_tick().unwrap();
        age(&engine);
        engine.watch_tick().unwrap();
        assert_eq!(engine.snapshot().items.len(), 1);
        fs::write(&item.destination, "Finder edited contents").unwrap();
        engine.watch_tick().unwrap();
        age(&engine);
        engine.watch_tick().unwrap();
        assert_eq!(
            engine
                .snapshot()
                .items
                .iter()
                .filter(|i| i.status == "queued")
                .count(),
            1
        );
        let restarted = Engine::new(temp.path().join("app"), engine.reader.clone()).unwrap();
        assert!(!restarted.snapshot().running);
        assert!(!restarted.snapshot().watching);
        assert!(restarted.snapshot().collection.is_none());
    }
    #[test]
    fn automatic_uncertainty_and_changed_sources_stay_in_place() {
        let (_temp, engine, root) = setup();
        *engine.test_decision.lock().unwrap() = None;
        fs::write(root.join("unknown.txt"), "Unclear").unwrap();
        engine.map_folder().unwrap();
        engine.start_tuck(false).unwrap();
        engine.process_next().unwrap();
        assert_eq!(engine.snapshot().items[0].status, "review");
        assert!(root.join("unknown.txt").exists());
        assert!(engine.snapshot().items[0].destination.is_empty());
    }
    #[test]
    fn a_session_over_the_document_limit_enqueues_nothing() {
        let (_temp, engine, root) = category_setup(false);
        for n in 0..2001 {
            fs::write(root.join(format!("{n}.txt")), "Synthetic").unwrap();
        }
        // Scanning refuses first; the session limit also holds for files added directly.
        assert!(files::existing_documents(&root)
            .unwrap_err()
            .contains("2,000"));
        let paths = (0..2001).map(|n| root.join(format!("{n}.txt"))).collect();
        assert!(engine.enqueue(paths).unwrap_err().contains("2,000"));
        assert!(engine.snapshot().items.is_empty());
        assert!(engine.snapshot().processing.is_none());
        assert_eq!(fs::read_to_string(root.join("0.txt")).unwrap(), "Synthetic");
    }
    fn category_setup(live: bool) -> (tempfile::TempDir, Engine, PathBuf) {
        use std::os::unix::fs::PermissionsExt;
        let temp = tempfile::tempdir_in("/private/tmp").unwrap();
        let root = temp.path().join("documents");
        fs::create_dir(&root).unwrap();
        let reader = if live {
            std::env::var_os("TUCK_EVAL_READER")
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/tuck-reader")
                })
        } else {
            let helper = temp.path().join("tuck-apple");
            fs::write(&helper, r#"#!/usr/bin/python3
import json, sys
request = json.load(sys.stdin)
from pathlib import Path
with Path(__file__).with_name('requests.jsonl').open('a') as log:
    log.write(json.dumps(request) + '\n')
if request.get('operation') == 'card':
    text = request['text']
    kind = 'rental agreement' if 'rental' in text else 'bank statement'
    print(json.dumps({'kind':kind, 'issuer':'', 'date':'', 'tags':['synthetic'], 'embedding':[0.0, 1.0] if 'rental' in text else [1.0, 0.0]}))
elif request.get('operation') == 'name_group':
    print(json.dumps({'name':'Home' if any('rental' in c['kind'] for c in request['cards']) else 'Banking & Taxes'}))
elif request.get('operation') == 'merge_names':
    print(json.dumps({'merges':[]}))
elif request.get('operation') == 'group_subjects':
    print(json.dumps({'subjects':{}}))
elif request.get('operation') == 'embed':
    print(json.dumps({'embeddings':[[1.0, 0.0] for _ in request['texts']]}))
elif request.get('operation') == 'choose_folder':
    print(json.dumps({'folder':request['candidates'][0]['folder']}))
elif request.get('operation') == 'rule':
    print(json.dumps({'instruction':'Keep immigration applications and residence permits here.'}))
else:
    print(json.dumps({'rules':[{'folder':'Banking & Taxes','instruction':'Keep bank statements and financial records here.'}, {'folder':'Home','instruction':'Keep rental agreements and household documents here.'}]}))
"#).unwrap();
            fs::set_permissions(&helper, fs::Permissions::from_mode(0o700)).unwrap();
            temp.path().join("tuck-reader")
        };
        let engine = Engine::new(temp.path().join("app"), reader).unwrap();
        engine.choose_folder(root.to_str().unwrap()).unwrap();
        (temp, engine, root)
    }
    fn category_documents(root: &Path) -> Vec<(PathBuf, String)> {
        [
            ("bank.txt", "Bank account statement. Monthly account transactions and closing balance. This is a synthetic financial document."),
            ("rent.txt", "Residential rental agreement. The tenant rents an apartment from the landlord. This is a synthetic housing document."),
            ("work.txt", "Employment contract. This agreement describes the job title, working hours and salary. This is a synthetic employment document."),
        ].into_iter().map(|(name, text)| {
            let path = root.join(name);
            fs::write(&path, text).unwrap();
            let hash = files::digest(&path).unwrap();
            (path, hash)
        }).collect()
    }
    #[test]
    fn saved_example_survives_restart_and_requires_fresh_analysis() {
        let (_temp, engine, root) = category_setup(false);
        let mut p = engine.snapshot().preferences;
        p.rules = Preferences::default().rules;
        p.use_ai = false;
        engine.save_preferences(p).unwrap();
        let document = root.join("example.txt");
        fs::write(&document, "Synthetic document").unwrap();
        engine.enqueue(vec![document.clone()]).unwrap();
        engine.process_next().unwrap();
        let item = engine.snapshot().items[0].clone();
        engine
            .update_review(&item.id, "Banking & Taxes", "Bank Statement")
            .unwrap();
        let item = engine.snapshot().items[0].clone();
        let mut p = engine.snapshot().preferences;
        p.rules.iter_mut().find(|r| r.folder == "Banking & Taxes").unwrap().instruction.push_str("\n\n## Filing example\nBelongs here: An unrelated monthly bank statement.\nDoes not belong here: An attachment in a Cedar application.");
        engine.save_preferences(p).unwrap();
        assert!(engine
            .organize(vec![Approval {
                id: item.id,
                destination: item.destination
            }])
            .unwrap_err()
            .contains("Preferences changed"));
        assert!(document.exists());
        let restarted = Engine::new(engine.directory.clone(), engine.reader.clone()).unwrap();
        assert!(restarted
            .snapshot()
            .preferences
            .rules
            .iter()
            .find(|r| r.folder == "Banking & Taxes")
            .unwrap()
            .instruction
            .contains("Does not belong here"));
        assert!(restarted.snapshot().items.is_empty());
    }
    #[test]
    fn category_suggestions_include_every_readable_mapped_document() {
        let (temp, engine, root) = category_setup(false);
        for n in 0..33 {
            fs::write(
                root.join(format!("document-{n:02}.txt")),
                format!(
                    "Synthetic document {n} with enough English text for category suggestions."
                ),
            )
            .unwrap();
        }
        fs::write(root.join("unreadable.txt"), "").unwrap();
        engine.map_folder().unwrap();
        let proposal = engine.suggest_categories().unwrap();
        assert_eq!(proposal.sampled, 33);
        assert_eq!(proposal.skipped, 1);
        let requests = fs::read_to_string(temp.path().join("requests.jsonl")).unwrap();
        let mut sent = std::collections::BTreeSet::new();
        for line in requests.lines() {
            let request: serde_json::Value = serde_json::from_str(line).unwrap();
            if request["operation"] == "card" {
                sent.insert(request["text"].as_str().unwrap().to_owned());
            }
        }
        assert_eq!(sent.len(), 33, "every readable document is described once");
        // Similar documents share one folder instead of one folder per batch.
        assert_eq!(proposal.rules.len(), 1);
        assert_eq!(proposal.rules[0].folder, "Banking & Taxes");
        assert!(engine.snapshot().items.is_empty());
        assert_eq!(files::existing_documents(&root).unwrap().len(), 34);
    }
    #[test]
    fn cards_are_reused_across_suggestions_rules_renames_and_filing() {
        let (temp, engine, root) = category_setup(false);
        let documents = category_documents(&root);
        let count = || {
            fs::read_to_string(temp.path().join("requests.jsonl"))
                .unwrap()
                .lines()
                .filter(|line| {
                    serde_json::from_str::<serde_json::Value>(line).unwrap()["operation"] == "card"
                })
                .count()
        };
        let proposal = engine.suggest_categories().unwrap();
        assert_eq!(count(), 3);
        let mut p = proposal.preferences;
        p.rules = proposal.rules;
        engine.save_preferences(p).unwrap();
        engine.suggest_categories().unwrap();
        assert_eq!(count(), 3);
        let renamed = root.join("renamed.txt");
        fs::rename(&documents[0].0, &renamed).unwrap();
        engine.suggest_categories().unwrap();
        assert_eq!(count(), 3);
        let text = fs::read_to_string(&renamed).unwrap();
        let decision = crate::filing::decide(
            &text,
            &[],
            &engine.snapshot().preferences,
            Some((&renamed, &engine.reader)),
        );
        assert!(decision.confident, "{}", decision.reason);
        assert_eq!(count(), 3);
        fs::write(
            &renamed,
            "A different readable document about a bank account and its transactions.",
        )
        .unwrap();
        engine.suggest_categories().unwrap();
        assert_eq!(count(), 4);
        assert!(renamed.exists()); // Suggestions and classification themselves do not move files.
    }
    #[test]
    fn approved_suggestions_move_files_in_one_step_and_keep_the_rest_for_review() {
        let (_temp, engine, root) = category_setup(false);
        let documents = category_documents(&root);
        fs::write(root.join("blank.txt"), "").unwrap();
        let proposal = engine.suggest_categories().unwrap();
        assert_eq!(proposal.files.len(), 3);
        assert!(proposal
            .files
            .iter()
            .all(|f| f.title == "Bank Statement" || f.title == "Rental Agreement"));
        let mut approved = proposal.preferences.clone();
        approved.rules = proposal.rules.clone();
        engine.save_preferences(approved).unwrap();
        engine.map_folder().unwrap();
        assert_eq!(
            engine
                .organize_proposal(proposal.files.clone(), false)
                .unwrap(),
            3
        );
        let state = engine.snapshot();
        assert!(state.running && !state.watching);
        for (path, hash) in &documents {
            assert!(!path.exists(), "{} was not moved", path.display());
            let item = state
                .items
                .iter()
                .find(|i| Path::new(&i.original) == path)
                .unwrap();
            assert_eq!(item.status, "organized");
            assert!(item.destination.starts_with(root.to_str().unwrap()));
            assert_eq!(&files::digest(Path::new(&item.destination)).unwrap(), hash);
        }
        let blank = state
            .items
            .iter()
            .find(|i| i.original.ends_with("blank.txt"))
            .unwrap();
        assert_eq!((blank.status.as_str(), blank.confident), ("review", false));
        assert!(root.join("blank.txt").exists());
        assert!(engine
            .organize_proposal(proposal.files, false)
            .unwrap_err()
            .contains("Finish or clear"));
    }
    #[test]
    fn forged_placements_move_nothing_outside_the_plan() {
        let (_temp, engine, root) = category_setup(false);
        let documents = category_documents(&root);
        let mut p = engine.snapshot().preferences;
        p.rules = vec![crate::model::Rule {
            document_types: Vec::new(),
            folder: "Home".into(),
            instruction: "Housing.".into(),
            enabled: true,
        }];
        engine.save_preferences(p).unwrap();
        engine.map_folder().unwrap();
        let outside = root.parent().unwrap().join("outside.txt");
        fs::write(&outside, "Not part of the folder").unwrap();
        let forged = |path: &Path, folder: &str, title: &str| crate::suggest::Placement {
            path: path.to_string_lossy().into_owned(),
            folder: folder.into(),
            title: title.into(),
        };
        let moved = engine
            .organize_proposal(
                vec![
                    forged(&outside, "Home", "Outside"),
                    forged(&documents[0].0, "Nowhere", "Bank"),
                    forged(&documents[1].0, "../escape", "Rent"),
                    forged(&documents[2].0, "Home", "../../x"),
                ],
                false,
            )
            .unwrap();
        assert_eq!(moved, 1, "only the in-plan file with a real folder moves");
        assert!(outside.exists());
        assert!(documents[0].0.exists() && documents[1].0.exists());
        let state = engine.snapshot();
        let renamed = state
            .items
            .iter()
            .find(|i| Path::new(&i.original) == documents[2].0)
            .unwrap();
        assert!(renamed
            .destination
            .starts_with(root.join("Home").to_str().unwrap()));
        assert!(!renamed.destination.contains(".."));
        assert!(state
            .items
            .iter()
            .filter(|i| Path::new(&i.original) != documents[2].0)
            .all(|i| i.status == "review" && !i.confident));
    }
    #[test]
    fn category_preview_requires_approval_and_preserves_existing_roots() {
        let (_temp, engine, root) = category_setup(false);
        assert!(engine.snapshot().preferences.rules.is_empty());
        let documents = category_documents(&root);
        let proposal = engine.suggest_categories().unwrap();
        assert_eq!(proposal.sampled, 3);
        assert!(!root.join("Banking & Taxes").exists());
        assert!(!root.join("Home").exists());
        assert!(engine.snapshot().processing.is_none());
        let mut approved = proposal.preferences;
        approved.rules = proposal.rules;
        engine.save_preferences(approved).unwrap();
        assert!(root.join("Banking & Taxes/.tuck/instructions.md").is_file());
        assert!(root.join("Home/.tuck/instructions.md").is_file());
        for (path, hash) in documents {
            assert_eq!(files::digest(&path).unwrap(), hash);
        }
        engine.choose_folder(root.to_str().unwrap()).unwrap();
        assert_eq!(engine.snapshot().preferences.rules.len(), 2);
        assert!(engine.suggest_categories().is_ok());
    }
    #[test]
    fn category_failures_clear_processing_and_stale_proposals_do_not_write() {
        let (_temp, engine, root) = category_setup(false);
        assert!(engine.suggest_categories().is_err());
        assert!(engine.snapshot().processing.is_none());
        category_documents(&root);
        let proposal = engine.suggest_categories().unwrap();
        fs::write(
            root.join(".tuck/instructions.md"),
            "Keep filenames unchanged.",
        )
        .unwrap();
        let mut approved = proposal.preferences;
        approved.rules = proposal.rules;
        assert!(engine.save_preferences(approved).is_err());
        assert!(!root.join("Banking & Taxes").exists());
        assert_eq!(
            fs::read_to_string(root.join(".tuck/instructions.md")).unwrap(),
            "Keep filenames unchanged."
        );
    }
    #[test]
    #[ignore = "Requires Apple Intelligence; synthetic category discovery and folder creation"]
    fn live_apple_category_suggestions() {
        let (_temp, engine, root) = category_setup(true);
        let documents = category_documents(&root);
        let proposal = engine.suggest_categories().unwrap();
        assert_eq!(proposal.sampled, 3);
        assert!((1..=6).contains(&proposal.rules.len()));
        println!(
            "Suggested synthetic categories: {:?}",
            proposal.rules.iter().map(|r| &r.folder).collect::<Vec<_>>()
        );
        for rule in &proposal.rules {
            assert!(!root.join(&rule.folder).exists());
        }
        let mut approved = proposal.preferences;
        approved.rules = proposal.rules;
        engine.save_preferences(approved).unwrap();
        for rule in engine.snapshot().preferences.rules {
            assert!(root
                .join(rule.folder)
                .join(".tuck/instructions.md")
                .is_file());
        }
        for (path, hash) in documents {
            assert_eq!(files::digest(&path).unwrap(), hash);
        }
    }
    fn setup() -> (tempfile::TempDir, Engine, PathBuf) {
        let temp = tempfile::tempdir_in("/private/tmp").unwrap();
        let root = temp.path().join("documents");
        fs::create_dir(&root).unwrap();
        let engine = Engine::new(temp.path().join("app"), PathBuf::new()).unwrap();
        engine.choose_folder(root.to_str().unwrap()).unwrap();
        let mut p = engine.snapshot().preferences;
        p.use_ai = false;
        p.rules = Preferences::default().rules;
        engine.save_preferences(p).unwrap();
        *engine.test_decision.lock().unwrap() = Some(crate::filing::Decision {
            folder: "Banking & Taxes".into(),
            title: "Bank statement".into(),
            confident: true,
            reason: "Synthetic bank statement".into(),
        });
        (temp, engine, root)
    }
    fn analyze(engine: &Engine, path: &Path) -> ReviewItem {
        engine.enqueue(vec![path.to_path_buf()]).unwrap();
        engine.process_next().unwrap();
        engine.snapshot().items.last().unwrap().clone()
    }
    fn approval(item: &ReviewItem) -> Approval {
        Approval {
            id: item.id.clone(),
            destination: item.destination.clone(),
        }
    }
    #[test]
    fn review_never_moves_and_approval_moves_only_selected() {
        let (_temp, engine, root) = setup();
        let a = root.join("a.txt");
        let b = root.join("b.txt");
        fs::write(&a, "bank statement a").unwrap();
        fs::write(&b, "bank statement b").unwrap();
        let first = analyze(&engine, &a);
        let second = analyze(&engine, &b);
        assert_eq!(first.status, "review");
        assert!(a.exists() && b.exists());
        assert!(!Path::new(&first.destination).exists());
        engine.organize(vec![approval(&first)]).unwrap();
        assert!(!a.exists());
        assert!(b.exists());
        assert!(Path::new(&first.destination).exists());
        assert_eq!(engine.snapshot().items[1].status, "review");
        // Both initially suggested the same name; the second now requires a new reviewed preview.
        assert!(engine
            .organize(vec![approval(&second)])
            .unwrap_err()
            .contains("occupied"));
        let second = engine.snapshot().items[1].clone();
        assert!(second.destination.contains("(1)"));
        engine.organize(vec![approval(&second)]).unwrap();
        assert_eq!(
            fs::read_to_string(&first.destination).unwrap(),
            "bank statement a"
        );
        assert_eq!(
            fs::read_to_string(&second.destination).unwrap(),
            "bank statement b"
        );
    }
    #[test]
    fn preview_uses_session_identity_and_follows_organized_destination() {
        let (temp, engine, _root) = setup();
        let path = temp.path().join("preview.txt");
        fs::write(&path, "bank statement").unwrap();
        let item = analyze(&engine, &path);
        assert_eq!(engine.preview_path(&item.id).unwrap(), path);
        assert!(engine.preview_path("unknown").is_err());
        engine.organize(vec![approval(&item)]).unwrap();
        assert_eq!(
            engine.preview_path(&item.id).unwrap(),
            PathBuf::from(&item.destination)
        );
        fs::remove_file(&item.destination).unwrap();
        assert!(engine.preview_path(&item.id).is_err());
    }
    #[test]
    fn external_files_are_copied_and_session_is_not_persisted() {
        let (temp, engine, _root) = setup();
        let path = temp.path().join("external.txt");
        fs::write(&path, "statement").unwrap();
        let item = analyze(&engine, &path);
        assert!(item.copied);
        engine.organize(vec![approval(&item)]).unwrap();
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            fs::read_to_string(&item.destination).unwrap()
        );
        let mut app_files: Vec<_> = fs::read_dir(temp.path().join("app"))
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        app_files.sort();
        let mut expected = vec![std::ffi::OsString::from("preferences.json")];
        if cfg!(target_os = "macos") {
            expected.insert(0, "folder-access.json".into());
        }
        assert_eq!(app_files, expected);
        let restarted = Engine::new(temp.path().join("app"), PathBuf::new()).unwrap();
        assert!(restarted.snapshot().items.is_empty());
        assert!(restarted.snapshot().processing.is_none());
    }
    #[test]
    fn uncertain_documents_stay_put_until_user_chooses_folder() {
        let (_temp, engine, root) = setup();
        *engine.test_decision.lock().unwrap() = None;
        let path = root.join("scan.txt");
        fs::write(&path, "unclear").unwrap();
        let item = analyze(&engine, &path);
        assert_eq!(item.status, "review");
        assert!(item.destination.is_empty());
        assert!(engine.organize(vec![approval(&item)]).is_err());
        assert!(path.exists());
        assert!(!root.join("Needs Review").exists());
        engine
            .update_review(&item.id, "Banking & Taxes", "My statement")
            .unwrap();
        let edited = engine.snapshot().items[0].clone();
        engine.organize(vec![approval(&edited)]).unwrap();
        assert!(root.join("Banking & Taxes/My statement.txt").exists());
    }
    #[test]
    fn edits_and_disabled_destinations_are_checked_before_moving() {
        let (_temp, engine, root) = setup();
        let path = root.join("source.txt");
        fs::write(&path, "original").unwrap();
        let item = analyze(&engine, &path);
        assert!(engine
            .update_review(&item.id, "../outside", "name")
            .is_err());
        assert!(engine
            .update_review(&item.id, "Banking & Taxes", "../name")
            .is_err());
        fs::write(&path, "user edits").unwrap();
        assert!(engine
            .organize(vec![approval(&item)])
            .unwrap_err()
            .contains("changed"));
        assert_eq!(fs::read_to_string(&path).unwrap(), "user edits");
        assert!(!Path::new(&item.destination).exists());
        engine.clear();
        let item = analyze(&engine, &path);
        let mut p = engine.snapshot().preferences;
        p.rules
            .iter_mut()
            .find(|r| r.folder == "Banking & Taxes")
            .unwrap()
            .enabled = false;
        engine.save_preferences(p).unwrap();
        assert!(engine
            .organize(vec![approval(&item)])
            .unwrap_err()
            .contains("Preferences changed"));
        assert!(path.exists());
    }
    #[test]
    fn switching_models_stops_filing_and_rejects_old_decisions() {
        let (_temp, engine, root) = setup();
        let path = root.join("source.txt");
        fs::write(&path, "statement").unwrap();
        let item = analyze(&engine, &path);
        {
            let mut state = engine.state.lock().unwrap();
            state.running = true;
            state.watching = true;
        }
        let mut p = engine.snapshot().preferences;
        p.ai_model = crate::model::LocalModel::Custom;
        p.connection.model = "local-model".into();
        engine.save_preferences(p).unwrap();
        assert!(!engine.snapshot().running);
        assert!(!engine.snapshot().watching);
        assert!(engine
            .organize(vec![approval(&item)])
            .unwrap_err()
            .contains("Preferences changed"));
        assert!(path.exists());
    }

    #[test]
    fn manual_settings_edits_reload_and_invalidate_review_and_stale_saves() {
        let (_temp, engine, root) = setup();
        let path = root.join("source.txt");
        fs::write(&path, "statement").unwrap();
        let item = analyze(&engine, &path);
        let old = engine.snapshot().preferences;
        let settings = root.join(".tuck/settings.json");
        let mut data: serde_json::Value =
            serde_json::from_slice(&fs::read(&settings).unwrap()).unwrap();
        data["rename"] = false.into();
        fs::write(&settings, serde_json::to_vec(&data).unwrap()).unwrap();
        assert!(!engine.snapshot().preferences.rename);
        assert!(engine
            .organize(vec![approval(&item)])
            .unwrap_err()
            .contains("Preferences changed"));
        assert!(engine
            .save_preferences(old)
            .unwrap_err()
            .contains("Settings changed"));
        fs::write(&settings, "bad json").unwrap();
        assert!(engine.snapshot().error.is_some());
        assert!(engine.organize(vec![approval(&item)]).is_err());
        assert_eq!(fs::read_to_string(&settings).unwrap(), "bad json");
        assert!(path.exists());
    }
    #[test]
    fn manual_prompt_edits_require_new_analysis() {
        let (_temp, engine, root) = setup();
        let path = root.join("source.txt");
        fs::write(&path, "statement").unwrap();
        let item = analyze(&engine, &path);
        fs::write(root.join(".tuck/instructions.md"), "Keep original names.").unwrap();
        assert!(engine
            .organize(vec![approval(&item)])
            .unwrap_err()
            .contains("Preferences changed"));
        assert!(path.exists());
    }
    #[test]
    fn skip_and_clear_leave_files_and_settings_intact() {
        let (_temp, engine, root) = setup();
        let path = root.join("source.txt");
        fs::write(&path, "statement").unwrap();
        engine.enqueue(vec![path.clone(), path.clone()]).unwrap();
        assert_eq!(engine.snapshot().items.len(), 1);
        let item = engine.snapshot().items[0].clone();
        engine.skip(&item.id).unwrap();
        assert!(!engine.process_next().unwrap());
        let settings = fs::read(root.join(".tuck/settings.json")).unwrap();
        engine.clear();
        assert!(engine.snapshot().items.is_empty());
        assert!(path.exists());
        assert_eq!(
            fs::read(root.join(".tuck/settings.json")).unwrap(),
            settings
        );
        assert!(engine
            .enqueue(vec![root.join(".tuck/instructions.md")])
            .is_err());
    }
    #[test]
    fn legacy_watch_queue_and_database_are_never_resumed_or_rewritten() {
        let temp = tempfile::tempdir_in("/private/tmp").unwrap();
        let root = temp.path().join("documents");
        fs::create_dir(&root).unwrap();
        let app = temp.path().join("app");
        fs::create_dir(&app).unwrap();
        let source = root.join("old.txt");
        fs::write(&source, "original").unwrap();
        let old = serde_json::to_vec(&serde_json::json!({"preferences": {"root": root, "rename": false}, "watching": true, "queue": [source], "activity": [{"status":"pending"}]})).unwrap();
        fs::write(app.join("state.json"), &old).unwrap();
        fs::write(app.join("tuck.db"), "existing database bytes").unwrap();
        let engine = Engine::new(app.clone(), PathBuf::new()).unwrap();
        assert!(engine.snapshot().items.is_empty());
        assert!(!engine.snapshot().preferences.rename);
        assert!(!engine.process_next().unwrap());
        assert_eq!(fs::read(app.join("state.json")).unwrap(), old);
        assert_eq!(
            fs::read_to_string(app.join("tuck.db")).unwrap(),
            "existing database bytes"
        );
        assert!(source.exists());
    }
    #[test]
    fn symlink_destinations_and_settings_are_rejected() {
        use std::os::unix::fs::symlink;
        let (temp, engine, root) = setup();
        let path = root.join("source.txt");
        fs::write(&path, "statement").unwrap();
        let item = analyze(&engine, &path);
        // Replacement with a link invalidates instruction loading before any transfer.
        fs::rename(
            root.join("Banking & Taxes"),
            root.join("OldBanking & Taxes"),
        )
        .unwrap();
        symlink(temp.path(), root.join("Banking & Taxes")).unwrap();
        assert!(engine.organize(vec![approval(&item)]).is_err());
        assert!(path.exists());
        fs::remove_file(root.join(".tuck/settings.json")).unwrap();
        let outside = temp.path().join("outside.json");
        fs::write(&outside, "{}").unwrap();
        symlink(&outside, root.join(".tuck/settings.json")).unwrap();
        assert!(engine.snapshot().error.is_some());
        assert_eq!(fs::read_to_string(outside).unwrap(), "{}");
    }
    #[test]
    fn restart_drops_unapproved_batch_and_preserves_settings() {
        let (temp, engine, root) = setup();
        let path = root.join("source.txt");
        fs::write(&path, "statement").unwrap();
        analyze(&engine, &path);
        let restarted = Engine::new(temp.path().join("app"), PathBuf::new()).unwrap();
        assert!(restarted.snapshot().items.is_empty());
        assert!(!restarted.snapshot().preferences.use_ai);
        assert!(path.exists());
    }
    #[test]
    #[ignore = "Requires Apple Intelligence; synthetic end-to-end review and approval"]
    fn live_apple_review_flow() {
        let (_temp, engine, root) = setup();
        *engine.test_decision.lock().unwrap() = None;
        let mut p = engine.snapshot().preferences;
        p.use_ai = true;
        engine.save_preferences(p).unwrap();
        let engine = Engine::new(
            engine.directory.clone(),
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/tuck-reader"),
        )
        .unwrap();
        let source = root.join("statement.txt");
        fs::write(&source, "BANK ACCOUNT STATEMENT\nExample Bank\nStatement period: January 2026\nOpening balance: SEK 1000\nDeposits: SEK 2500\nClosing balance: SEK 3500\nThis is a bank account statement showing account transactions. Synthetic test document.").unwrap();
        let item = analyze(&engine, &source);
        assert_eq!(item.status, "review");
        assert!(source.exists());
        assert_eq!(
            item.folder, "Banking & Taxes",
            "Apple result: {}",
            item.reason
        );
        assert!(!Path::new(&item.destination).exists());
        engine.organize(vec![approval(&item)]).unwrap();
        assert!(!source.exists());
        assert!(Path::new(&item.destination).is_file());
        assert_eq!(
            files::digest(Path::new(&item.destination)).unwrap(),
            item.hash
        );
    }
}
