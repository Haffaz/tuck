use crate::model::Preferences;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    time::UNIX_EPOCH,
};

pub const EXTENSIONS: &[&str] = &[
    "pdf", "png", "jpg", "jpeg", "heic", "tiff", "tif", "txt", "md",
];
pub fn supported(path: &Path) -> bool {
    !path.components().any(|part| {
        part.as_os_str()
            .to_string_lossy()
            .eq_ignore_ascii_case(".tuck")
    }) && path
        .extension()
        .and_then(|s| s.to_str())
        .is_some_and(|s| EXTENSIONS.contains(&s.to_lowercase().as_str()))
}
/// Collect document paths only after an explicit folder-review request. Never follows links.
pub fn existing_documents(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut pending = vec![(root.to_path_buf(), 0usize)];
    let mut paths = Vec::new();
    let mut entries_seen = 0;
    while let Some((directory, depth)) = pending.pop() {
        let metadata = fs::symlink_metadata(&directory).map_err(|e| e.to_string())?;
        if metadata.file_type().is_symlink()
            || directory.canonicalize().map_err(|e| e.to_string())? != directory
            || !directory.starts_with(root)
        {
            return Err("A folder changed during scanning. Choose the folder again.".into());
        }
        for entry in fs::read_dir(&directory).map_err(|e| e.to_string())? {
            entries_seen += 1;
            if entries_seen > 10_000 {
                return Err("This folder is too large to review at once. Choose files instead, or turn off Include subfolders.".into());
            }
            let entry = entry.map_err(|e| e.to_string())?;
            if entry.file_name().to_string_lossy().starts_with('.') {
                continue;
            }
            let kind = entry.file_type().map_err(|e| e.to_string())?;
            if kind.is_symlink() {
                continue;
            }
            if kind.is_dir() {
                if depth >= 32 {
                    return Err(
                        "This folder has too many nested folders. Choose files instead.".into(),
                    );
                }
                pending.push((entry.path(), depth + 1));
            } else if kind.is_file() && supported(&entry.path()) {
                let path = entry.path();
                if path.canonicalize().map_err(|e| e.to_string())? != path {
                    return Err("A document changed during scanning. Try again.".into());
                }
                paths.push(path);
                if paths.len() > 2000 {
                    return Err("More than 2,000 documents were found. Choose a smaller document folder. Nothing was organized.".into());
                }
            }
        }
    }
    paths.sort();
    Ok(paths)
}
pub fn signature(path: &Path) -> Result<String, String> {
    let m = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !m.is_file() || m.file_type().is_symlink() {
        return Err("Only regular files are supported".into());
    }
    Ok(format!(
        "{}:{}",
        m.len(),
        m.modified()
            .map_err(|e| e.to_string())?
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ))
}
pub fn digest(path: &Path) -> Result<String, String> {
    let mut f = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    let mut buf = [0; 65536];
    loop {
        let n = f.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}
pub fn valid_folder(folder: &str, max_depth: usize) -> bool {
    let parts: Vec<_> = folder.split('/').collect();
    !folder.is_empty()
        && parts.len() <= max_depth
        && parts.iter().all(|s| {
            !s.is_empty()
                && !s.starts_with('.')
                && !s.contains(['\\', ':'])
                && s.chars().all(|c| !c.is_control())
        })
        && Path::new(folder)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
}
pub fn validate(p: &Preferences) -> Result<(), String> {
    if !(1..=crate::model::MAX_FOLDER_DEPTH as u8).contains(&p.folder_nesting)
        || p.category_instructions.chars().count() > 1200
    {
        return Err("Choose one or two folder levels and keep category instructions under 1,200 characters.".into());
    }
    if p.ai_model == crate::model::LocalModel::Custom {
        validate_connection(&p.connection)?;
    }
    if !(1..=crate::model::MAX_FOLDER_DEPTH).contains(&p.max_depth) {
        return Err("Use at most two folder levels".into());
    }
    if p.rules.len() > 40 {
        return Err("Use at most 40 folder rules".into());
    }
    for rule in &p.rules {
        if rule.document_types.len() > 128
            || rule
                .document_types
                .iter()
                .any(|t| t.trim().is_empty() || t.chars().count() > 300)
        {
            return Err("A folder scope exceeds the supported document-type limit".into());
        }
        if rule.enabled && rule.instruction.trim().is_empty() {
            return Err(format!("Describe what belongs in {}", rule.folder));
        }
        if !valid_folder(&rule.folder, p.max_depth) {
            return Err(format!(
                "Invalid folder or too many levels: {}",
                rule.folder
            ));
        }
    }
    let folders: std::collections::HashSet<_> =
        p.rules.iter().map(|r| r.folder.to_lowercase()).collect();
    if folders.len() != p.rules.len() {
        return Err("Each folder should appear only once".into());
    }
    Ok(())
}
pub fn safe_stem(input: &str) -> String {
    input
        .chars()
        .filter(|c| !c.is_control() && !"/\\:*?\"<>|".contains(*c))
        .take(100)
        .collect::<String>()
        .trim_matches([' ', '.'])
        .to_string()
}
pub fn target_directory(root: &Path, folder: &str) -> Result<PathBuf, String> {
    if !valid_folder(folder, crate::model::MAX_FOLDER_DEPTH) {
        return Err("Invalid destination folder".into());
    }
    let mut dir = root.to_path_buf();
    for component in Path::new(folder).components() {
        dir.push(component);
        if let Ok(meta) = fs::symlink_metadata(&dir) {
            if meta.file_type().is_symlink() || !meta.is_dir() {
                return Err("Destination must be a real folder, not a link or file".into());
            }
        } else {
            fs::create_dir(&dir).map_err(|e| e.to_string())?;
        }
        if !dir
            .canonicalize()
            .map_err(|e| e.to_string())?
            .starts_with(root)
        {
            return Err("Destination is outside the selected folder".into());
        }
    }
    Ok(dir)
}
pub fn unique_destination(dir: &Path, stem: &str, extension: &str, source: &Path) -> PathBuf {
    for i in 0..10000 {
        let name = if i == 0 {
            format!("{stem}.{extension}")
        } else {
            format!("{stem} ({i}).{extension}")
        };
        let path = dir.join(name);
        if path == source || fs::symlink_metadata(&path).is_err() {
            return path;
        }
    }
    dir.join(format!("{stem}-{}.{}", uuid::Uuid::new_v4(), extension))
}
// The destination is created exclusively. Existing files are never replaced.
pub fn transfer(
    source: &Path,
    destination: &Path,
    copied: bool,
    expected_hash: &str,
) -> Result<(), String> {
    signature(source)?;
    if source.canonicalize().map_err(|e| e.to_string())? != source {
        return Err("Source path changed; original kept".into());
    }
    let parent = destination.parent().ok_or("Missing destination folder")?;
    if parent.canonicalize().map_err(|e| e.to_string())? != parent {
        return Err("Destination folder changed; original kept".into());
    }
    if digest(source)? != expected_hash {
        return Err("Document changed during processing; retry when it is finished saving".into());
    }
    if copied {
        let mut tmp = tempfile::NamedTempFile::new_in(
            destination.parent().ok_or("Missing destination folder")?,
        )
        .map_err(|e| e.to_string())?;
        let mut source_file = fs::File::open(source).map_err(|e| e.to_string())?;
        std::io::copy(&mut source_file, &mut tmp).map_err(|e| e.to_string())?;
        tmp.flush().map_err(|e| e.to_string())?;
        tmp.as_file().sync_all().map_err(|e| e.to_string())?;
        if digest(tmp.path())? != expected_hash {
            return Err("Document changed while being copied; original kept".into());
        }
        tmp.persist_noclobber(destination)
            .map_err(|e| e.to_string())?;
    } else {
        // Files within the selected root are moved via an exclusive hard link.
        // A failure on another volume leaves the original intact.
        fs::hard_link(source, destination)
            .map_err(|e| format!("Could not move document (original kept): {e}"))?;
        if let Err(error) = fs::remove_file(source) {
            let _ = fs::remove_file(destination);
            return Err(format!("Could not remove original after linking: {error}"));
        }
    }
    Ok(())
}

/// Validate even when a saved connection is not currently reachable.
pub fn validate_connection(c: &crate::model::ModelConnection) -> Result<(), String> {
    if !["ollama", "openai", "anthropic"].contains(&c.kind.as_str())
        || c.model.trim().is_empty()
        || c.model.len() > 200
        || c.model.chars().any(char::is_control)
    {
        return Err("Choose a connection type and enter a model ID.".into());
    }
    let url = tauri::Url::parse(&c.base_url).map_err(|_| "Enter a valid API base URL.")?;
    let local = matches!(
        url.host_str(),
        Some("localhost" | "127.0.0.1" | "[::1]" | "::1")
    );
    if url.host_str().is_none()
        || !(url.scheme() == "https" || (local && url.scheme() == "http"))
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(
            "Use HTTPS, or HTTP for a server on this Mac. Keep credentials out of the URL.".into(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn custom_connections_reject_insecure_or_credential_urls() {
        let mut c = crate::model::ModelConnection {
            model: "arbitrary-model".into(),
            ..Default::default()
        };
        for url in [
            "http://127.0.0.1:11434",
            "http://localhost:1234/v1",
            "http://[::1]:11434",
            "https://api.example.test/v1",
        ] {
            c.base_url = url.into();
            assert!(validate_connection(&c).is_ok(), "{url}");
        }
        for url in [
            "http://api.example.test",
            "file:///tmp/test",
            "https://user:password@example.test",
            "https://example.test?key=secret",
            "https://example.test#key",
            "not a url",
        ] {
            c.base_url = url.into();
            assert!(validate_connection(&c).is_err(), "{url}");
        }
        let mut p = Preferences {
            folder_nesting: 0,
            ..Preferences::default()
        };
        assert!(validate(&p).is_err());
        p.folder_nesting = 4;
        assert!(validate(&p).is_err());
    }
    #[test]
    fn enabled_folders_need_instructions() {
        let mut p = Preferences::default();
        p.rules[0].instruction = "   ".into();
        assert!(validate(&p).unwrap_err().contains("Describe what belongs"));
        p.rules[0].enabled = false;
        assert!(validate(&p).is_ok());
    }
    #[test]
    fn rejects_escape_and_deep_folders() {
        for folder in [
            "../escape",
            "/outside",
            "Migration/../../x",
            "a/b/c",
            "a//b",
            "a\\b",
            ".hidden",
        ] {
            assert!(!valid_folder(folder, 2), "{folder}");
        }
        assert!(valid_folder("Migration/Cedar", 2));
    }
    #[test]
    fn second_level_destinations_are_supported_and_third_levels_fail() {
        let root = tempfile::tempdir_in("/private/tmp").unwrap();
        let path = target_directory(root.path(), "Income/Acme").unwrap();
        assert!(path.is_dir());
        assert!(target_directory(root.path(), "Income/Acme/Payslips").is_err());
        assert!(target_directory(root.path(), "Income/../outside").is_err());
    }
    #[test]
    fn move_and_copy_never_replace_existing_files() {
        let d = tempfile::tempdir_in("/private/tmp").unwrap();
        let a = d.path().join("a.txt");
        let b = d.path().join("b.txt");
        fs::write(&a, "original").unwrap();
        fs::write(&b, "keep me").unwrap();
        let hash = digest(&a).unwrap();
        assert!(transfer(&a, &b, false, &hash).is_err());
        assert!(transfer(&a, &b, true, &hash).is_err());
        assert_eq!(fs::read_to_string(&a).unwrap(), "original");
        assert_eq!(fs::read_to_string(&b).unwrap(), "keep me");
    }
    #[test]
    fn refuses_changed_document() {
        let d = tempfile::tempdir_in("/private/tmp").unwrap();
        let a = d.path().join("a.txt");
        let b = d.path().join("b.txt");
        fs::write(&a, "old").unwrap();
        let hash = digest(&a).unwrap();
        fs::write(&a, "new").unwrap();
        assert!(transfer(&a, &b, false, &hash).is_err());
        assert!(a.exists());
        assert!(!b.exists());
    }
    #[test]
    fn moves_and_copies_keep_contents() {
        let d = tempfile::tempdir_in("/private/tmp").unwrap();
        let a = d.path().join("a.txt");
        let b = d.path().join("b.txt");
        let c = d.path().join("c.txt");
        fs::write(&a, "document").unwrap();
        let hash = digest(&a).unwrap();
        transfer(&a, &b, true, &hash).unwrap();
        assert!(a.exists());
        assert_eq!(digest(&b).unwrap(), hash);
        transfer(&a, &c, false, &hash).unwrap();
        assert!(!a.exists());
        assert_eq!(digest(&c).unwrap(), hash);
    }
    #[cfg(unix)]
    #[test]
    fn refuses_symlink_destination() {
        let d = tempfile::tempdir_in("/private/tmp").unwrap();
        let outside = tempfile::tempdir_in("/private/tmp").unwrap();
        std::os::unix::fs::symlink(outside.path(), d.path().join("Migration")).unwrap();
        assert!(target_directory(d.path(), "Migration").is_err());
    }
    #[test]
    fn metadata_is_never_an_importable_document() {
        assert!(!supported(Path::new("/documents/.tuck/instructions.md")));
        assert!(!supported(Path::new(
            "/documents/Migration/.tuck/instructions.md"
        )));
        assert!(supported(Path::new("/documents/notes.md")));
    }
}
