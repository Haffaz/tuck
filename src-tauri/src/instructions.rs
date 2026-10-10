//! Folder-owned prompts. The app store holds only the root and an on-disk marker.
use crate::{
    files,
    model::{Preferences, Rule},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

const LIMIT: u64 = 64 * 1024;
const JOURNAL: &str = "pending-edit.json";

pub struct Loaded {
    pub instructions: String,
    pub rules: Vec<Rule>,
    pub legacy_folders: Vec<String>,
    pub revision: String,
    files: BTreeMap<String, String>,
}
impl Loaded {
    pub fn apply(self, p: &mut Preferences) {
        p.instructions = self.instructions;
        p.rules = self.rules;
        p.legacy_folders = self.legacy_folders;
        p.instructions_revision = self.revision;
    }
}
#[derive(Serialize, Deserialize)]
struct Edit {
    folder: String,
    before: Option<String>,
    after: Option<String>,
}

fn config_path(root: &Path, folder: &str, create: bool) -> Result<PathBuf, String> {
    if !folder.is_empty() && !files::valid_folder(folder, crate::model::LEGACY_FOLDER_DEPTH) {
        return Err("Invalid instruction folder".into());
    }
    if root
        .canonicalize()
        .map_err(|_| "The instruction root is unavailable")?
        != root
    {
        return Err("The instruction root changed location; choose it again".into());
    }
    let mut path = root.to_path_buf();
    for part in folder
        .split('/')
        .filter(|p| !p.is_empty())
        .chain(std::iter::once(".tuck"))
    {
        path.push(part);
        match fs::symlink_metadata(&path) {
            Ok(m) if m.is_dir() && !m.file_type().is_symlink() => {}
            Ok(_) => {
                return Err(format!(
                    "Instruction folders must be real directories: {}",
                    path.display()
                ))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                if create {
                    fs::create_dir(&path).map_err(|e| e.to_string())?;
                }
            }
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(path.join("instructions.md"))
}
pub(crate) fn settings_path(root: &Path) -> Result<PathBuf, String> {
    Ok(config_path(root, "", false)?.with_file_name("settings.json"))
}
fn read_file(path: &Path, limit: u64) -> Result<Option<String>, String> {
    match fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.to_string()),
        Ok(m) => {
            if !m.is_file() || m.file_type().is_symlink() || m.len() > limit {
                return Err(format!(
                    "Instruction file must be a regular file smaller than {} KB: {}",
                    limit / 1024,
                    path.display()
                ));
            }
            fs::read_to_string(path)
                .map(Some)
                .map_err(|e| format!("Cannot read {}: {e}", path.display()))
        }
    }
}
fn raw(root: &Path, folder: &str) -> Result<Option<String>, String> {
    read_file(&config_path(root, folder, false)?, LIMIT)
}
fn atomic(path: &Path, content: &str) -> Result<(), String> {
    let mut temp =
        tempfile::NamedTempFile::new_in(path.parent().ok_or("Missing instruction directory")?)
            .map_err(|e| e.to_string())?;
    temp.write_all(content.as_bytes())
        .map_err(|e| e.to_string())?;
    temp.as_file().sync_all().map_err(|e| e.to_string())?;
    temp.persist(path).map_err(|e| e.to_string())?;
    Ok(())
}
fn put(root: &Path, folder: &str, content: &Option<String>) -> Result<(), String> {
    let path = config_path(root, folder, content.is_some())?;
    if let Some(text) = content {
        if text.len() as u64 > LIMIT {
            return Err("An instruction file exceeds 64 KB".into());
        }
        atomic(&path, text)
    } else {
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}
fn journal_path(root: &Path, create: bool) -> Result<PathBuf, String> {
    Ok(config_path(root, "", create)?.with_file_name(JOURNAL))
}
/// Roll back an interrupted editor save only where the exact before/after bytes agree.
pub fn recover(root: &Path) -> Result<(), String> {
    let path = journal_path(root, false)?;
    let Some(text) = read_file(&path, 12 * 1024 * 1024)? else {
        return Ok(());
    };
    let edits: Vec<Edit> = serde_json::from_str(&text).map_err(|_| {
        "Invalid instruction-save journal; preserve it and restore the instruction files"
    })?;
    if edits.len() > 81 {
        return Err("Invalid instruction-save journal size".into());
    }
    for edit in &edits {
        let current = raw(root, &edit.folder)?;
        if current != edit.before && current != edit.after {
            return Err("An interrupted instruction save conflicts with a manual edit. Preserve pending-edit.json and resolve the changed instruction file before sorting.".into());
        }
    }
    for edit in edits.iter().rev() {
        if raw(root, &edit.folder)? == edit.after {
            put(root, &edit.folder, &edit.before)?;
        }
    }
    fs::remove_file(path).map_err(|e| e.to_string())
}
fn commit(root: &Path, edits: Vec<Edit>) -> Result<(), String> {
    if edits.is_empty() {
        return Ok(());
    }
    if edits.len() > 81
        || edits
            .iter()
            .any(|e| e.after.as_ref().is_some_and(|s| s.len() as u64 > LIMIT))
    {
        return Err("Too many instruction changes or a file exceeds 64 KB".into());
    }
    let journal = journal_path(root, true)?;
    let mut temp =
        tempfile::NamedTempFile::new_in(journal.parent().unwrap()).map_err(|e| e.to_string())?;
    temp.write_all(&serde_json::to_vec(&edits).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    temp.as_file().sync_all().map_err(|e| e.to_string())?;
    temp.persist_noclobber(&journal)
        .map_err(|_| "An instruction save is already pending; restart Tuck to recover it")?;
    let result = (|| {
        for edit in &edits {
            if raw(root, &edit.folder)? != edit.before {
                return Err("Rules changed during saving. Reload them before trying again.".into());
            }
            put(root, &edit.folder, &edit.after)?;
        }
        Ok(())
    })();
    if let Err(error) = result {
        recover(root).map_err(|recovery| format!("{error} {recovery}"))?;
        return Err(error);
    }
    fs::remove_file(journal).map_err(|e| e.to_string())
}
fn parse(folder: &str, raw: &str) -> Result<(Rule, usize), String> {
    let text = raw.replace("\r\n", "\n");
    let (mut enabled, mut priority) = (true, usize::MAX);
    let mut document_types: Vec<String> = Vec::new();
    let mut scope_hash = String::new();
    let mut body = text.as_str();
    if let Some(rest) = text.strip_prefix("---\n") {
        let (header, remainder) = rest
            .split_once("\n---\n")
            .ok_or_else(|| format!("Unclosed front matter in {folder}/.tuck/instructions.md"))?;
        let mut seen = std::collections::HashSet::new();
        for line in header
            .lines()
            .filter(|l| !l.trim().is_empty() && !l.trim().starts_with('#'))
        {
            let (key, value) = line.split_once(':').ok_or(
                "Use enabled: true/false and priority: a whole number in instruction front matter",
            )?;
            if !seen.insert(key.trim()) {
                return Err(format!("Duplicate instruction field in {folder}"));
            }
            match key.trim() {
                "document-types" => document_types = serde_json::from_str(value.trim()).map_err(|_| format!("Invalid document-types in {folder}"))?,
                "scope-hash" => scope_hash = value.trim().to_owned(),
                "enabled" => enabled = value.trim().parse().map_err(|_| format!("Use enabled: true or false in {folder}"))?,
                "priority" => priority = value.trim().parse().map_err(|_| format!("Use a non-negative priority number in {folder}"))?,
                _ => return Err(format!("Unknown instruction field '{}' in {folder}; supported fields are enabled, priority, document-types and scope-hash", key.trim())),
            }
        }
        body = remainder;
    }
    if enabled && body.trim().is_empty() {
        return Err(format!(
            "Describe what belongs in {folder}/.tuck/instructions.md"
        ));
    }
    if scope_hash != format!("{:x}", Sha256::digest(body.trim().as_bytes())) {
        document_types.clear();
    }
    Ok((
        Rule {
            document_types,
            folder: folder.into(),
            instruction: body.trim().into(),
            enabled,
        },
        priority,
    ))
}
fn render(rule: &Rule, priority: usize) -> String {
    let scope = if rule.document_types.is_empty() {
        String::new()
    } else {
        format!(
            "document-types: {}\nscope-hash: {:x}\n",
            serde_json::to_string(&rule.document_types).unwrap(),
            Sha256::digest(rule.instruction.trim().as_bytes())
        )
    };
    format!(
        "---\nenabled: {}\npriority: {}\n{}---\n\n{}\n",
        rule.enabled,
        priority,
        scope,
        rule.instruction.trim()
    )
}
fn folders(
    root: &Path,
    current: &Path,
    depth: usize,
    result: &mut Vec<String>,
) -> Result<(), String> {
    if depth == crate::model::LEGACY_FOLDER_DEPTH {
        return Ok(());
    }
    for entry in fs::read_dir(current).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.file_name().to_string_lossy().starts_with('.')
            || !entry.file_type().map_err(|e| e.to_string())?.is_dir()
        {
            continue;
        }
        let path = entry.path();
        result.push(
            path.strip_prefix(root)
                .map_err(|e| e.to_string())?
                .to_string_lossy()
                .into_owned(),
        );
        folders(root, &path, depth + 1, result)?;
    }
    Ok(())
}
pub fn load(p: &Preferences) -> Result<Loaded, String> {
    let root = Path::new(&p.root);
    if read_file(&journal_path(root, false)?, 12 * 1024 * 1024)?.is_some() {
        return Err("An instruction save is pending. Restart Tuck to recover it.".into());
    }
    let instructions = raw(root, "")?.ok_or("Root .tuck/instructions.md is missing. Restore it before sorting; Tuck will not recreate deleted rules.")?;
    let mut found = Vec::new();
    folders(root, root, 0, &mut found)?;
    found.sort();
    let mut source = BTreeMap::from([(String::new(), instructions.clone())]);
    let mut rules = Vec::new();
    let mut legacy_folders = Vec::new();
    for folder in found {
        if let Some(text) = raw(root, &folder)? {
            let parsed = parse(&folder, &text)?;
            if files::valid_folder(&folder, crate::model::MAX_FOLDER_DEPTH) {
                rules.push(parsed);
            } else {
                legacy_folders.push(folder.clone());
            }
            source.insert(folder, text);
        }
    }
    rules.sort_by(|(a, pa), (b, pb)| pa.cmp(pb).then_with(|| a.folder.cmp(&b.folder)));
    let rules: Vec<_> = rules.into_iter().map(|(r, _)| r).collect();
    let candidate = Preferences {
        rules: rules.clone(),
        ..p.clone()
    };
    files::validate(&candidate)?;
    let mut hash = Sha256::new();
    for (folder, text) in &source {
        hash.update((folder.len() as u64).to_le_bytes());
        hash.update(folder.as_bytes());
        hash.update((text.len() as u64).to_le_bytes());
        hash.update(text.as_bytes());
    }
    Ok(Loaded {
        instructions,
        rules,
        legacy_folders,
        revision: format!("{:x}", hash.finalize()),
        files: source,
    })
}
/// Root file is the opt-in marker. Existing root instructions take precedence over legacy settings.
pub fn initialize(p: &Preferences) -> Result<Loaded, String> {
    files::validate(p)?;
    let root = Path::new(&p.root);
    recover(root)?;
    if raw(root, "")?.is_none() {
        let mut edits = Vec::new();
        for (index, rule) in p.rules.iter().enumerate() {
            if raw(root, &rule.folder)?.is_none() {
                edits.push(Edit {
                    folder: rule.folder.clone(),
                    before: None,
                    after: Some(render(rule, index)),
                });
            }
        }
        edits.push(Edit {
            folder: String::new(),
            before: None,
            after: Some(p.instructions.clone()),
        });
        commit(root, edits)?;
    }
    load(p)
}
pub fn save(p: &Preferences, expected: &str) -> Result<Loaded, String> {
    files::validate(p)?;
    let previous = load(&Preferences {
        max_depth: crate::model::MAX_FOLDER_DEPTH,
        ..p.clone()
    })?;
    if previous.revision != expected {
        return Err("Rules changed on disk. Reload preferences before saving your edits.".into());
    }
    let mut desired = BTreeMap::from([(String::new(), p.instructions.clone())]);
    for folder in &previous.legacy_folders {
        if let Some(text) = previous.files.get(folder) {
            desired.insert(folder.clone(), text.clone());
        }
    }
    for (index, rule) in p.rules.iter().enumerate() {
        desired.insert(rule.folder.clone(), render(rule, index));
    }
    let mut keys: Vec<_> = previous
        .files
        .keys()
        .chain(desired.keys())
        .cloned()
        .collect();
    keys.sort();
    keys.dedup();
    let edits = keys
        .into_iter()
        .filter_map(|folder| {
            let before = previous.files.get(&folder).cloned();
            let after = desired.get(&folder).cloned();
            (before != after).then_some(Edit {
                folder,
                before,
                after,
            })
        })
        .collect();
    commit(Path::new(&p.root), edits)?;
    load(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn setup() -> (tempfile::TempDir, Preferences) {
        let dir = tempfile::tempdir_in("/private/tmp").unwrap();
        let p = Preferences {
            root: dir.path().to_str().unwrap().into(),
            ..Preferences::default()
        };
        (dir, p)
    }
    #[test]
    fn typed_scope_roundtrips_and_manual_scope_edits_clear_it() {
        let (dir, mut p) = setup();
        initialize(&p).unwrap().apply(&mut p);
        let types = (0..24)
            .map(|i| format!("Distinct academic record type {i}"))
            .collect::<Vec<_>>();
        p.rules[0].document_types = types.clone();
        let revision = load(&p).unwrap().revision;
        save(&p, &revision).unwrap().apply(&mut p);
        assert_eq!(p.rules[0].document_types, types);
        let path = config_path(dir.path(), &p.rules[0].folder, false).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        fs::write(
            path,
            text.replace(&p.rules[0].instruction, "A new user-defined scope."),
        )
        .unwrap();
        load(&p).unwrap().apply(&mut p);
        assert!(p.rules[0].document_types.is_empty());
    }
    #[test]
    fn migration_preserves_existing_prompts_and_switches_and_is_idempotent() {
        let (_dir, mut p) = setup();
        p.rules[1].enabled = false;
        put(
            Path::new(&p.root),
            &p.rules[0].folder,
            &Some("---\nenabled: true\npriority: 0\n---\n\nMy manual prompt\n".into()),
        )
        .unwrap();
        let first = initialize(&p).unwrap();
        assert_eq!(first.rules[0].instruction, "My manual prompt");
        assert!(!first.rules[1].enabled);
        assert_eq!(first.rules.len(), p.rules.len());
        assert_eq!(first.instructions, p.instructions);
        p.instructions = "Replacement must not win".into();
        assert_eq!(initialize(&p).unwrap().revision, first.revision);
    }
    #[test]
    fn manual_edits_invalidate_saves_and_removing_rule_keeps_documents() {
        let (_dir, mut p) = setup();
        initialize(&p).unwrap().apply(&mut p);
        let root = Path::new(&p.root);
        fs::write(root.join(".tuck/instructions.md"), "Manual root edit").unwrap();
        assert!(save(&p, &p.instructions_revision)
            .err()
            .unwrap()
            .contains("changed on disk"));
        assert_eq!(load(&p).unwrap().instructions, "Manual root edit");
        load(&p).unwrap().apply(&mut p);
        let removed = p.rules.remove(0);
        let document = Path::new(&p.root).join(&removed.folder).join("keep.txt");
        fs::write(&document, "keep").unwrap();
        let saved = save(&p, &p.instructions_revision).unwrap();
        assert!(!saved.rules.iter().any(|r| r.folder == removed.folder));
        assert_eq!(fs::read_to_string(document).unwrap(), "keep");
    }
    #[test]
    fn invalid_frontmatter_and_missing_root_fail_closed() {
        let (_dir, p) = setup();
        initialize(&p).unwrap();
        let root = Path::new(&p.root);
        for text in [
            "---\nenabled: yes\n---\nx",
            "---\nunknown: true\n---\nx",
            "---\npriority: -1\n---\nx",
            "---\nenabled: true\nenabled: false\n---\nx",
            "---\nenabled: true",
            "",
        ] {
            put(root, &p.rules[0].folder, &Some(text.into())).unwrap();
            assert!(load(&p).is_err(), "{text}");
        }
        fs::remove_file(root.join(".tuck/instructions.md")).unwrap();
        assert!(load(&p).err().unwrap().contains("missing"));
        assert!(!root.join(".tuck/instructions.md").exists());
    }
    #[test]
    fn recovery_restores_exact_bytes_but_preserves_conflicting_manual_edits() {
        let (_dir, p) = setup();
        initialize(&p).unwrap();
        let root = Path::new(&p.root);
        let before = raw(root, "").unwrap();
        let edits = vec![Edit {
            folder: "".into(),
            before: before.clone(),
            after: Some("half saved".into()),
        }];
        fs::write(
            journal_path(root, false).unwrap(),
            serde_json::to_vec(&edits).unwrap(),
        )
        .unwrap();
        put(root, "", &Some("half saved".into())).unwrap();
        assert!(load(&p).is_err());
        recover(root).unwrap();
        assert_eq!(raw(root, "").unwrap(), before);
        fs::write(
            journal_path(root, false).unwrap(),
            serde_json::to_vec(&edits).unwrap(),
        )
        .unwrap();
        put(root, "", &Some("manual edit".into())).unwrap();
        assert!(recover(root).is_err());
        assert_eq!(raw(root, "").unwrap().unwrap(), "manual edit");
        assert!(journal_path(root, false).unwrap().exists());
    }
    #[test]
    fn symlink_metadata_cannot_redirect_writes() {
        use std::os::unix::fs::symlink;
        let (_dir, p) = setup();
        let outside = tempfile::tempdir_in("/private/tmp").unwrap();
        symlink(outside.path(), Path::new(&p.root).join(".tuck")).unwrap();
        assert!(initialize(&p).is_err());
        assert!(!outside.path().join("instructions.md").exists());
        fs::remove_file(Path::new(&p.root).join(".tuck")).unwrap();
        initialize(&p).unwrap();
        let target = outside.path().join("sentinel");
        fs::write(&target, "untouched").unwrap();
        let config = Path::new(&p.root).join(".tuck/instructions.md");
        fs::remove_file(&config).unwrap();
        symlink(&target, &config).unwrap();
        assert!(load(&p).is_err());
        assert_eq!(fs::read_to_string(target).unwrap(), "untouched");
    }
}
