//! The filing queue: documents added by dropping, by Organize new files or by the folder
//! watcher are read and filed one at a time; clear decisions are organized automatically.
use super::{Engine, Runtime};
use crate::{
    config, files,
    model::{Approval, Preferences, ReviewItem},
};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

impl Engine {
    pub(super) fn watch_tick(&self) -> Result<(), String> {
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
    pub(super) fn enqueue_into(state: &mut Runtime, paths: Vec<PathBuf>) -> Result<usize, String> {
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
                        "Settings changed while Tuck was reading. Try this document again.".into();
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
    pub(super) fn analyze(&self, item: &ReviewItem, p: &Preferences) -> Result<ReviewItem, String> {
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
            return Err("The document changed while Tuck was reading it. Try again.".into());
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
}
