use crate::model::Preferences;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    time::UNIX_EPOCH,
};
use walkdir::WalkDir;

pub const EXTENSIONS: &[&str] = &[
    "pdf", "png", "jpg", "jpeg", "heic", "tiff", "tif", "txt", "md",
];
pub fn supported(path: &Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .is_some_and(|s| EXTENSIONS.contains(&s.to_lowercase().as_str()))
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
pub fn scan(p: &Preferences) -> Result<Vec<PathBuf>, String> {
    let root = Path::new(&p.root);
    if !root.is_dir() {
        return Err(
            "The watched folder is unavailable. Reconnect it or choose another folder.".into(),
        );
    }
    let mut paths = Vec::new();
    let walk = WalkDir::new(root)
        .follow_links(false)
        .max_depth(if p.recursive { usize::MAX } else { 1 });
    for entry in walk
        .into_iter()
        .filter_entry(|e| e.depth() == 0 || !e.file_name().to_string_lossy().starts_with('.'))
    {
        let entry = entry.map_err(|e| format!("Cannot read part of the watched folder: {e}"))?;
        if entry.file_type().is_file() && supported(entry.path()) {
            paths.push(entry.into_path());
        }
    }
    paths.sort();
    Ok(paths)
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
    if !(1..=2).contains(&p.max_depth) {
        return Err("Choose one or two folder levels".into());
    }
    if p.rules.is_empty() || p.rules.len() > 40 {
        return Err("Add between 1 and 40 folder rules".into());
    }
    for rule in &p.rules {
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
    if p.model.trim().is_empty() {
        return Err("Enter a local model name".into());
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
    if !valid_folder(folder, 2) {
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

#[cfg(test)]
mod tests {
    use super::*;
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
        assert!(valid_folder("Migration/AGENCY", 2));
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
}
