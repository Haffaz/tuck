//! A session-only document map. Text stays on this Mac and is never written to settings.
use crate::{
    classifier, files,
    model::{Preferences, Rule},
};
use serde::Serialize;
use std::{collections::BTreeMap, fs, path::Path};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Document {
    pub path: String,
    pub folder: String,
    pub duplicate_of: Option<String>,
    pub error: Option<String>,
    #[serde(skip)]
    pub text: String,
    /// Local clues outside the text: download source, PDF properties, faces, barcodes.
    #[serde(skip)]
    pub clues: Vec<String>,
    #[serde(skip)]
    pub hash: String,
    #[serde(skip)]
    pub signature: String,
}
#[derive(Clone, Serialize)]
pub struct Folder {
    pub path: String,
    pub documents: usize,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Collection {
    pub documents: Vec<Document>,
    pub folders: Vec<Folder>,
    pub existing_rules: Vec<Rule>,
    pub unreadable: usize,
    pub duplicates: usize,
}
pub fn read(path: &Path, root: &Path, reader: &Path) -> Document {
    let mut document = Document {
        path: path.to_string_lossy().into_owned(),
        folder: path
            .parent()
            .and_then(|p| p.strip_prefix(root).ok())
            .unwrap_or(Path::new(""))
            .to_string_lossy()
            .into_owned(),
        duplicate_of: None,
        error: None,
        text: String::new(),
        clues: Vec::new(),
        hash: String::new(),
        signature: String::new(),
    };
    let result = (|| {
        if path.canonicalize().map_err(|e| e.to_string())? != path || !path.starts_with(root) {
            return Err("Document moved during mapping. Map the folder again.".into());
        }
        document.signature = files::signature(path)?;
        if fs::metadata(path).map_err(|e| e.to_string())?.len() > 100 * 1024 * 1024 {
            return Err("Document exceeds 100 MB; original kept.".into());
        }
        document.hash = files::digest(path)?;
        let reading = classifier::read(path, reader)?;
        document.text = reading.text;
        document.clues = reading.clues;
        if files::signature(path)? != document.signature || files::digest(path)? != document.hash {
            return Err("Document changed during mapping. Map the folder again.".into());
        }
        if document.text.trim().is_empty() {
            return Err("No readable text found.".into());
        }
        Ok(())
    })();
    if let Err(error) = result {
        document.error = Some(error);
        document.text.clear();
    }
    document
}
pub fn build(documents: Vec<Document>, p: &Preferences) -> Collection {
    let mut documents = documents;
    let mut hashes: BTreeMap<String, String> = BTreeMap::new();
    let mut counts = BTreeMap::new();
    for doc in &mut documents {
        if !doc.hash.is_empty() {
            if let Some(first) = hashes.get(&doc.hash) {
                doc.duplicate_of = Some(first.clone());
            } else {
                hashes.insert(doc.hash.clone(), doc.path.clone());
            }
        }
        if !doc.folder.is_empty() {
            *counts.entry(doc.folder.clone()).or_insert(0) += 1;
        }
    }
    // Include empty destination folders as well as folders containing documents.
    let root = Path::new(&p.root);
    let mut pending = vec![(root.to_path_buf(), 0)];
    while let Some((directory, depth)) = pending.pop() {
        if depth >= p.max_depth {
            continue;
        }
        if let Ok(entries) = fs::read_dir(directory) {
            for entry in entries.flatten() {
                if entry.file_name().to_string_lossy().starts_with('.') {
                    continue;
                }
                if entry
                    .file_type()
                    .is_ok_and(|t| t.is_dir() && !t.is_symlink())
                {
                    let path = entry.path();
                    if path.canonicalize().ok().as_ref() != Some(&path) {
                        continue;
                    }
                    if let Ok(relative) = path.strip_prefix(root) {
                        counts
                            .entry(relative.to_string_lossy().into_owned())
                            .or_insert(0);
                        pending.push((path, depth + 1));
                    }
                }
            }
        }
    }
    let folders: Vec<_> = counts
        .into_iter()
        .map(|(path, documents)| Folder { path, documents })
        .collect();
    let mut existing_rules = p.rules.clone();
    for folder in &folders {
        if files::valid_folder(&folder.path, p.max_depth)
            && !existing_rules.iter().any(|r| r.folder == folder.path)
        {
            existing_rules.push(Rule { document_types: Vec::new(), folder: folder.path.clone(), instruction: format!("Keep documents about {} here. Confirm this topic from document content; existing placement alone is not evidence.", folder.path), enabled: true });
        }
    }
    let unreadable = documents.iter().filter(|d| d.error.is_some()).count();
    let duplicates = documents
        .iter()
        .filter(|d| d.duplicate_of.is_some())
        .count();
    Collection {
        documents,
        folders,
        existing_rules,
        unreadable,
        duplicates,
    }
}

impl Collection {
    pub fn view(&self) -> Self {
        Self {
            documents: self
                .documents
                .iter()
                .map(|d| Document {
                    path: d.path.clone(),
                    folder: d.folder.clone(),
                    duplicate_of: d.duplicate_of.clone(),
                    error: d.error.clone(),
                    text: String::new(),
                    clues: Vec::new(),
                    hash: String::new(),
                    signature: String::new(),
                })
                .collect(),
            folders: self.folders.clone(),
            existing_rules: self.existing_rules.clone(),
            unreadable: self.unreadable,
            duplicates: self.duplicates,
        }
    }
}
