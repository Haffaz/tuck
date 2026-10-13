use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase", from = "StoredRule")]
pub struct Rule {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub document_types: Vec<String>,
    pub folder: String,
    pub instruction: String,
    #[serde(default = "enabled_by_default")]
    pub enabled: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredRule {
    #[serde(default)]
    document_types: Vec<String>,
    folder: String,
    instruction: Option<String>,
    keywords: Option<String>,
    #[serde(default = "enabled_by_default")]
    enabled: bool,
}
impl From<StoredRule> for Rule {
    fn from(old: StoredRule) -> Self {
        let instruction = old.instruction.unwrap_or_else(|| {
            let topics = old.keywords.unwrap_or_default();
            if topics.trim().is_empty() {
                format!("Keep documents related to {} here.", old.folder)
            } else {
                format!("Keep documents related to these topics here: {}. Consider the document text, filename and source folders.", topics.trim())
            }
        });
        Self {
            document_types: old.document_types,
            folder: old.folder,
            instruction,
            enabled: old.enabled,
        }
    }
}

/// Compare saved scopes without treating ordering or casing as a distinction.
pub fn same_document_types(left: &[String], right: &[String]) -> bool {
    let normalized = |types: &[String]| -> std::collections::BTreeSet<String> {
        types
            .iter()
            .map(|s| {
                s.split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .to_lowercase()
            })
            .collect()
    };
    normalized(left) == normalized(right)
}

fn enabled_by_default() -> bool {
    true
}

/// Legacy selections remain readable; new connections use the custom adapter.
#[derive(Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum LocalModel {
    #[default]
    Apple,
    GemmaE2b,
    GemmaE4b,
    Custom,
}
impl LocalModel {
    pub fn key(self) -> &'static str {
        match self {
            Self::Apple => "apple",
            Self::GemmaE2b => "gemmaE2b",
            Self::GemmaE4b => "gemmaE4b",
            Self::Custom => "custom",
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct ModelConnection {
    pub kind: String,
    pub base_url: String,
    pub model: String,
}
impl Default for ModelConnection {
    fn default() -> Self {
        Self {
            kind: "ollama".into(),
            base_url: "http://127.0.0.1:11434".into(),
            model: String::new(),
        }
    }
}
pub fn default_folder_nesting() -> u8 {
    2
}
pub const MAX_FOLDER_DEPTH: usize = 2;
// Read and recover old metadata without making old destinations active.
pub const LEGACY_FOLDER_DEPTH: usize = 3;

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Preferences {
    pub root: String,
    pub recursive: bool,
    pub rename: bool,
    pub max_depth: usize,
    pub use_ai: bool,
    pub ai_model: LocalModel,
    pub connection: ModelConnection,
    pub folder_nesting: u8,
    pub category_instructions: String,

    pub use_agent: bool,
    pub use_memory: bool,
    pub instructions: String,
    pub rules: Vec<Rule>,
    pub legacy_folders: Vec<String>,
    pub instructions_revision: String,
    pub settings_revision: String,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            root: String::new(), settings_revision: String::new(), instructions_revision: String::new(), recursive: true, rename: true, max_depth: MAX_FOLDER_DEPTH,
            use_ai: true, ai_model: LocalModel::default(), connection: ModelConnection::default(), folder_nesting: default_folder_nesting(), category_instructions: String::new(), use_agent: true, use_memory: true,
            instructions: "Use clear English document names. Describe the document accurately without inventing names or dates.".into(),
            legacy_folders: vec![],
            rules: vec![
                Rule { document_types: Vec::new(), enabled: true, folder: "Identity & Immigration".into(), instruction: "Identity documents, residence permits, visas and immigration decisions or applications.".into() },
                Rule { document_types: Vec::new(), enabled: true, folder: "Banking & Taxes".into(), instruction: "Bank statements, account records and tax documents.".into() },
                Rule { document_types: Vec::new(), enabled: true, folder: "Employment & Pay".into(), instruction: "Employment contracts, employment certificates, payslips and other employment or pay records.".into() },
                Rule { document_types: Vec::new(), enabled: true, folder: "Housing & Utilities".into(), instruction: "Rental agreements and household utility bills, including electricity bills.".into() },
                Rule { document_types: Vec::new(), enabled: true, folder: "Insurance".into(), instruction: "Insurance policies and certificates of cover, including home and travel insurance.".into() },
                Rule { document_types: Vec::new(), enabled: true, folder: "Medical Records".into(), instruction: "Medical certificates, reports, prescriptions and other clinical records.".into() },
                Rule { document_types: Vec::new(), enabled: true, folder: "Maintenance & Repairs".into(), instruction: "Equipment inspections, maintenance reports and repair records.".into() },
            ],
        }
    }
}

impl Preferences {
    pub fn rule_enabled(&self, rule: &Rule) -> bool {
        rule.enabled
            && !self.rules.iter().any(|parent| {
                !parent.enabled && rule.folder.starts_with(&format!("{}/", parent.folder))
            })
    }
    pub fn destination_instructions(&self, folder: &str) -> String {
        let mut result = vec![self.instructions.clone()];
        let mut ancestors: Vec<_> = self
            .rules
            .iter()
            .filter(|r| r.folder == folder || folder.starts_with(&format!("{}/", r.folder)))
            .collect();
        ancestors.sort_by_key(|r| r.folder.split('/').count());
        result.extend(
            ancestors
                .into_iter()
                .map(|r| format!("{}: {}", r.folder, r.instruction)),
        );
        result.join("\n\n")
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct LanguageInfo {
    pub name: String,
    pub code: String,
    pub status: String,
    pub message: String,
}

#[derive(Clone, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ReviewItem {
    pub language: Option<LanguageInfo>,
    pub id: String,
    pub original: String,
    pub destination: String,
    pub folder: String,
    pub title: String,
    pub reason: String,
    pub status: String,
    pub copied: bool,
    pub confident: bool,
    #[serde(skip)]
    pub hash: String,
    #[serde(skip)]
    pub revision: String,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub collection: Option<crate::collection::Collection>,
    pub running: bool,
    pub watching: bool,
    pub preferences: Preferences,
    pub items: Vec<ReviewItem>,
    pub processing: Option<String>,
    pub error: Option<String>,
}
#[derive(Deserialize)]
pub struct Approval {
    pub id: String,
    pub destination: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn legacy_keywords_migrate_once_preserving_order_and_switches() {
        let p: Preferences = serde_json::from_str(r#"{"rules":[{"folder":"Custom","keywords":"acme, Swedish letters","enabled":false},{"folder":"Migration","keywords":"cedar"}]}"#).unwrap();
        assert_eq!(p.rules[0].folder, "Custom");
        assert!(!p.rules[0].enabled);
        assert!(p.rules[0].instruction.contains("acme, Swedish letters"));
        assert!(p.rules[1].enabled);
        let saved = serde_json::to_string(&p).unwrap();
        assert!(!saved.contains("keywords"));
        let loaded: Preferences = serde_json::from_str(&saved).unwrap();
        assert_eq!(loaded.rules[0].instruction, p.rules[0].instruction);
        let explicit: Rule = serde_json::from_str(
            r#"{"folder":"Migration","keywords":"old","instruction":"My prompt"}"#,
        )
        .unwrap();
        assert_eq!(explicit.instruction, "My prompt");
    }
    #[test]
    fn legacy_provider_settings_are_ignored_without_losing_preferences() {
        let p: Preferences = serde_json::from_str(r#"{"provider":"ollama","model":"qwen3:4b","useAi":false,"rename":false,"root":"/example"}"#).unwrap();
        assert!(!p.use_ai);
        assert!(!p.rename);
        assert_eq!(p.root, "/example");
        let saved = serde_json::to_value(&p).unwrap();
        assert!(saved.get("provider").is_none());
        assert!(saved.get("model").is_none());
    }
}
