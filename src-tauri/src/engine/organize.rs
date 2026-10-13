//! Organizing: applying an approved suggestion and moving reviewed files. Every move goes
//! through `organize_locked`, which re-checks the source hash, the destination and the root.
use super::{Engine, Runtime};
use crate::{
    config, files,
    model::{Approval, Preferences, ReviewItem},
};
use std::{collections::BTreeMap, path::Path};

impl Engine {
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
    pub(super) fn destination(p: &Preferences, item: &ReviewItem) -> Result<String, String> {
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
            return Err("Settings changed. Suggest again before organizing.".into());
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
    pub(super) fn organize_locked(
        state: &mut Runtime,
        approvals: Vec<Approval>,
    ) -> Result<(), String> {
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
                return Err("Settings changed. Suggest again before organizing.".into());
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
                    return Err(
                        "The file changed or moved since Tuck read it, so it was left where it is."
                            .into(),
                    );
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
}
