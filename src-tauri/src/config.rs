//! Small, user-editable settings. No queue, document index or history is persisted.
use crate::{files, instructions, model::Preferences};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fs, io::Write, path::Path};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Settings {
    version: u32,
    rename: bool,
    max_depth: usize,
    use_ai: bool,
    #[serde(default)]
    ai_model: crate::model::LocalModel,
    use_agent: bool,
    #[serde(default)]
    connection: crate::model::ModelConnection,
    #[serde(default = "crate::model::default_folder_nesting")]
    folder_nesting: u8,
    #[serde(default, rename = "categorySpecificity", skip_serializing)]
    _legacy_specificity: Option<u8>,
    #[serde(default)]
    category_instructions: String,
}
fn settings(p: &Preferences) -> Settings {
    Settings {
        version: 1,
        rename: p.rename,
        max_depth: p.max_depth,
        use_ai: p.use_ai,
        ai_model: p.ai_model,
        use_agent: p.use_agent,
        connection: p.connection.clone(),
        folder_nesting: p.folder_nesting,
        _legacy_specificity: None,
        category_instructions: p.category_instructions.clone(),
    }
}
pub fn fingerprint(p: &Preferences) -> String {
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&settings(p)).unwrap())
    )
}
pub fn revision(p: &Preferences) -> String {
    format!("{}:{}:{}", p.root, fingerprint(p), p.instructions_revision)
}
pub fn read_json(path: &Path) -> Result<Option<serde_json::Value>, String> {
    match fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.to_string()),
        Ok(m) => {
            if !m.is_file() || m.file_type().is_symlink() || m.len() > 1024 * 1024 {
                return Err(format!(
                    "Expected a regular settings file under 1 MB: {}",
                    path.display()
                ));
            }
            let bytes = fs::read(path).map_err(|e| e.to_string())?;
            serde_json::from_slice(&bytes)
                .map(Some)
                .map_err(|e| format!("Invalid {}: {e}. Original file kept.", path.display()))
        }
    }
}
pub fn atomic_json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    // Validate existing files instead of following links or replacing corrupt configuration.
    read_json(path)?;
    let mut file = tempfile::NamedTempFile::new_in(path.parent().ok_or("Missing settings folder")?)
        .map_err(|e| e.to_string())?;
    file.write_all(&serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    file.as_file().sync_all().map_err(|e| e.to_string())?;
    file.persist(path).map_err(|e| e.to_string())?;
    Ok(())
}
pub fn load(p: &mut Preferences) -> Result<(), String> {
    let path = instructions::settings_path(Path::new(&p.root))?;
    let value = read_json(&path)?
        .ok_or(".tuck/settings.json is missing. Restore it or choose the folder again.")?;
    let stored: Settings =
        serde_json::from_value(value).map_err(|e| format!("Invalid .tuck/settings.json: {e}"))?;
    if stored.version != 1 {
        return Err("Unsupported settings version; original file kept".into());
    }
    p.rename = stored.rename;
    if !(1..=crate::model::LEGACY_FOLDER_DEPTH).contains(&stored.max_depth) {
        return Err("Unsupported saved folder depth; original settings kept".into());
    }
    p.max_depth = crate::model::MAX_FOLDER_DEPTH;
    p.use_ai = stored.use_ai;
    p.ai_model = stored.ai_model;
    p.use_agent = stored.use_agent;
    p.connection = stored.connection;
    if !(1..=3).contains(&stored.folder_nesting) {
        return Err("Unsupported saved nesting preference; original settings kept".into());
    }
    p.folder_nesting = stored
        .folder_nesting
        .min(crate::model::MAX_FOLDER_DEPTH as u8);
    p.category_instructions = stored.category_instructions;
    p.use_memory = false;
    p.settings_revision = fingerprint(p);
    instructions::load(p)?.apply(p);
    files::validate(p)
}
pub fn save(p: &Preferences) -> Result<(), String> {
    atomic_json(
        &instructions::settings_path(Path::new(&p.root))?,
        &settings(p),
    )
}
pub fn initialize(p: &mut Preferences) -> Result<(), String> {
    // Parse any settings before creating anything. Existing settings always win.
    let exists = read_json(&instructions::settings_path(Path::new(&p.root))?)?.is_some();
    if exists {
        instructions::recover(Path::new(&p.root))?;
        load(p)
    } else {
        instructions::initialize(p)?.apply(p);
        save(p)?;
        load(p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::LocalModel;
    #[test]
    fn legacy_third_level_rules_are_preserved_without_becoming_destinations() {
        let dir = tempfile::tempdir_in("/private/tmp").unwrap();
        let mut p = Preferences {
            root: dir.path().to_string_lossy().into(),
            ..Preferences::default()
        };
        initialize(&mut p).unwrap();
        let legacy = dir.path().join("Education/University/Transcripts/.tuck");
        fs::create_dir_all(&legacy).unwrap();
        let bytes = "---\nenabled: true\npriority: 0\n---\n\nAcademic transcripts.\n";
        fs::write(legacy.join("instructions.md"), bytes).unwrap();
        let path = instructions::settings_path(dir.path()).unwrap();
        let mut stored = read_json(&path).unwrap().unwrap();
        stored["maxDepth"] = serde_json::json!(3);
        stored["folderNesting"] = serde_json::json!(3);
        fs::write(&path, serde_json::to_vec(&stored).unwrap()).unwrap();
        load(&mut p).unwrap();
        assert_eq!(p.max_depth, 2);
        assert_eq!(p.folder_nesting, 2);
        assert_eq!(p.legacy_folders, vec!["Education/University/Transcripts"]);
        assert!(p.rules.iter().all(|r| r.folder.split('/').count() <= 2));
        p.instructions.push_str(" Keep source dates.");
        instructions::save(&p, &p.instructions_revision)
            .unwrap()
            .apply(&mut p);
        save(&p).unwrap();
        load(&mut p).unwrap();
        assert_eq!(
            fs::read_to_string(legacy.join("instructions.md")).unwrap(),
            bytes
        );
        assert_eq!(p.legacy_folders.len(), 1);
    }
    #[test]
    fn connection_and_nesting_roundtrip_without_secrets() {
        let dir = tempfile::tempdir().unwrap();
        let mut p = Preferences {
            root: dir
                .path()
                .canonicalize()
                .unwrap()
                .to_string_lossy()
                .into_owned(),
            ..Preferences::default()
        };
        initialize(&mut p).unwrap();
        let before = revision(&p);
        p.ai_model = LocalModel::Custom;
        p.connection = crate::model::ModelConnection {
            kind: "openai".into(),
            base_url: "https://example.test/v1".into(),
            model: "any-model-id".into(),
        };
        p.folder_nesting = 3;
        p.category_instructions = "Separate employers".into();
        save(&p).unwrap();
        load(&mut p).unwrap();
        assert_eq!(p.connection.model, "any-model-id");
        assert_eq!(p.folder_nesting, 2);
        assert_eq!(p.category_instructions, "Separate employers");
        assert_ne!(revision(&p), before);
        let json = serde_json::to_value(settings(&p)).unwrap();
        assert!(json["connection"].get("apiKey").is_none());
        let current = revision(&p);
        p.connection.base_url = "https://another.test/v1".into();
        assert_ne!(revision(&p), current);
        let current = revision(&p);
        p.folder_nesting = 1;
        assert_ne!(revision(&p), current);
    }
    #[test]
    fn earlier_specificity_settings_load_and_lower_nesting_keeps_existing_rules() {
        let dir = tempfile::tempdir_in("/private/tmp").unwrap();
        let mut p = Preferences {
            root: dir.path().to_string_lossy().into(),
            rules: vec![crate::model::Rule {
                document_types: Vec::new(),
                folder: "Income/Acme".into(),
                instruction: "Acme payslips.".into(),
                enabled: true,
            }],
            ..Preferences::default()
        };
        initialize(&mut p).unwrap();
        let path = instructions::settings_path(dir.path()).unwrap();
        let mut stored: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        stored.as_object_mut().unwrap().remove("folderNesting");
        stored["categorySpecificity"] = serde_json::json!(5);
        stored["maxDepth"] = serde_json::json!(2);
        fs::write(&path, serde_json::to_vec(&stored).unwrap()).unwrap();
        load(&mut p).unwrap();
        assert_eq!(p.folder_nesting, 2);
        assert_eq!(p.max_depth, crate::model::MAX_FOLDER_DEPTH);
        p.folder_nesting = 1;
        save(&p).unwrap();
        load(&mut p).unwrap();
        assert_eq!(p.folder_nesting, 1);
        assert_eq!(p.rules[0].folder, "Income/Acme");
        let current: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        assert!(current.get("categorySpecificity").is_none());
    }
    #[test]
    fn legacy_settings_default_to_apple_and_unknown_models_fail() {
        let legacy = serde_json::json!({"version":1,"rename":true,"maxDepth":2,"useAi":true,"useAgent":true});
        let stored: Settings = serde_json::from_value(legacy.clone()).unwrap();
        assert_eq!(stored.ai_model, LocalModel::Apple);
        let mut invalid = legacy;
        invalid["aiModel"] = serde_json::json!("cloud-or-arbitrary-tag");
        assert!(serde_json::from_value::<Settings>(invalid).is_err());
    }
    #[test]
    fn model_selection_roundtrips_and_invalidates_decisions() {
        let dir = tempfile::tempdir().unwrap();
        let mut p = Preferences {
            root: dir
                .path()
                .canonicalize()
                .unwrap()
                .to_string_lossy()
                .into_owned(),
            ..Preferences::default()
        };
        initialize(&mut p).unwrap();
        let original = revision(&p);
        p.ai_model = LocalModel::GemmaE2b;
        save(&p).unwrap();
        p.ai_model = LocalModel::Apple;
        load(&mut p).unwrap();
        assert_eq!(p.ai_model, LocalModel::GemmaE2b);
        assert_ne!(revision(&p), original);
        let e2b = revision(&p);
        p.ai_model = LocalModel::GemmaE4b;
        assert_ne!(revision(&p), e2b);
    }
}
