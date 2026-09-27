use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Rule {
    pub folder: String,
    pub keywords: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Preferences {
    pub root: String,
    pub recursive: bool,
    pub rename: bool,
    pub max_depth: usize,
    pub use_ai: bool,
    pub model: String,
    pub instructions: String,
    pub rules: Vec<Rule>,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            root: String::new(), recursive: true, rename: true, max_depth: 2,
            use_ai: true, model: "qwen3:4b".into(),
            instructions: "Use clear English document names. Keep application bundles together. Never infer an expiry date or call a residence permit a visa.".into(),
            rules: vec![
                Rule { folder: "Migration/AGENCY".into(), keywords: "agency".into() },
                Rule { folder: "Migration".into(), keywords: "residence permit, migrationsverket, uppehållstillstånd, immigration".into() },
                Rule { folder: "Finances".into(), keywords: "bank statement, tax return, kontoutdrag, skatteverket".into() },
                Rule { folder: "Work".into(), keywords: "payslip, employment contract, lönespecifikation, anställningsavtal".into() },
                Rule { folder: "Home".into(), keywords: "rental agreement, tenancy, hyresavtal".into() },
                Rule { folder: "Insurance".into(), keywords: "insurance policy, försäkringsbrev".into() },
                Rule { folder: "Health".into(), keywords: "medical report, prescription, läkarintyg".into() },
            ],
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Activity {
    pub id: String,
    pub timestamp: u64,
    pub original: String,
    pub destination: String,
    pub folder: String,
    pub reason: String,
    pub status: String,
    pub hash: String,
    pub copied: bool,
    pub duplicate: bool,
}

#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Store {
    pub preferences: Preferences,
    pub watching: bool,
    pub activity: Vec<Activity>,
    pub seen: HashMap<String, String>,
    pub queue: std::collections::VecDeque<std::path::PathBuf>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub preferences: Preferences,
    pub watching: bool,
    pub activity: Vec<Activity>,
    pub pending: usize,
    pub processing: Option<String>,
    pub error: Option<String>,
}
