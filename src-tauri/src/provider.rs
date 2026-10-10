//! Provider-neutral role contracts. Add another adapter here without giving models file access.
use crate::{apple, cards::Card};
use serde::Serialize;
use serde_json::{json, Value};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

/// A filing destination as the card chooser sees it.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub folder: String,
    pub instruction: String,
    pub document_types: Vec<String>,
}

/// Synchronous by design: one stage completes before the next starts.
/// Adapters return data only. None receive document paths or write tools.
pub trait ModelProvider: Sync {
    fn status(&self) -> Result<String, String>;
    fn suggest_rule(&self, folder: &str, context: &str) -> Result<String, String>;
    /// Identifies the model and helper build; cached cards are keyed by it.
    fn identity(&self) -> Result<String, String>;
    /// One short guided description of a document plus a local embedding.
    fn card(&self, text: &str, file_name: &str, timeout: Duration) -> Result<Card, String>;
    /// Local sentence embeddings; no language model is involved.
    fn embed(&self, texts: &[String], timeout: Duration) -> Result<Vec<Vec<f64>>, String>;
    fn name_group(
        &self,
        cards: &[&Card],
        parent: &str,
        timeout: Duration,
    ) -> Result<String, String>;
    /// A broader subject for each folder name, in the same order; empty when unplaced.
    /// `details` describes what each folder holds.
    fn group_subjects(
        &self,
        names: &[String],
        details: &[String],
        timeout: Duration,
    ) -> Result<Vec<String>, String>;
    /// Pairs (from, into) of names that mean the same thing.
    fn merge_names(
        &self,
        names: &[String],
        timeout: Duration,
    ) -> Result<Vec<(String, String)>, String>;
    /// Index into `candidates`, or None when no folder fits.
    fn choose_folder(
        &self,
        card: &Card,
        excerpt: &str,
        candidates: &[Candidate],
        root_rules: &str,
        timeout: Duration,
    ) -> Result<Option<usize>, String>;
}
pub struct LocalProvider {
    model: crate::model::LocalModel,
    helper: PathBuf,
    preferences: Option<crate::model::Preferences>,
}
/// One selection point for every role. Prompts and validators are shared in the helper.
pub fn selected(reader: &Path, model: crate::model::LocalModel) -> LocalProvider {
    LocalProvider {
        helper: reader.with_file_name("tuck-apple"),
        model,
        preferences: None,
    }
}
pub fn configured(reader: &Path, p: &crate::model::Preferences) -> LocalProvider {
    let mut provider = selected(reader, p.ai_model);
    provider.preferences = Some(p.clone());
    provider
}
impl LocalProvider {
    fn invoke(&self, mut input: Value, timeout: Duration) -> Result<Value, String> {
        input["model"] = json!(self.model.key());
        if let Some(p) = &self.preferences {
            input["connection"] = json!(p.connection);
            input["folderNesting"] = json!(p.folder_nesting);
            input["categoryInstructions"] = json!(p.category_instructions);
        }
        apple::invoke(&self.helper, Some(input), timeout)
    }
}
impl ModelProvider for LocalProvider {
    fn status(&self) -> Result<String, String> {
        self.invoke(json!({"operation":"status"}), Duration::from_secs(90))?["status"]
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| "Invalid model status response".into())
    }
    fn suggest_rule(&self, folder: &str, context: &str) -> Result<String, String> {
        let value = self.invoke(
            json!({"operation":"rule","folder":folder,"context":context}),
            Duration::from_secs(60),
        )?;
        let instruction = value["instruction"].as_str().unwrap_or_default().trim();
        if instruction.is_empty() || instruction.chars().count() > 600 {
            return Err("Invalid folder description. Try again or write a short sentence.".into());
        }
        Ok(instruction.into())
    }
    fn identity(&self) -> Result<String, String> {
        let digest = if self.model == crate::model::LocalModel::Custom {
            serde_json::to_string(
                &self
                    .preferences
                    .as_ref()
                    .ok_or("Missing connection")?
                    .connection,
            )
            .map_err(|e| e.to_string())?
        } else if self.model == crate::model::LocalModel::Apple {
            "on-device".to_owned()
        } else {
            self.invoke(json!({"operation":"identity"}), Duration::from_secs(15))?["digest"]
                .as_str()
                .ok_or("Missing local model identity")?
                .to_owned()
        };
        Ok(format!(
            "{}:cards-v1:{}:{}",
            self.model.key(),
            digest,
            crate::files::digest(&self.helper)?
        ))
    }
    fn card(&self, text: &str, file_name: &str, timeout: Duration) -> Result<Card, String> {
        let value = self.invoke(
            json!({"operation":"card", "text":text, "fileName":file_name}),
            timeout,
        )?;
        serde_json::from_value(value).map_err(|_| "Invalid document description".into())
    }
    fn embed(&self, texts: &[String], timeout: Duration) -> Result<Vec<Vec<f64>>, String> {
        let value = self.invoke(json!({"operation":"embed", "texts":texts}), timeout)?;
        let vectors: Vec<Vec<f64>> = serde_json::from_value(value["embeddings"].clone())
            .map_err(|_| "Invalid embedding response")?;
        if vectors.len() != texts.len() {
            return Err("Invalid embedding response".into());
        }
        Ok(vectors)
    }
    fn name_group(
        &self,
        cards: &[&Card],
        parent: &str,
        timeout: Duration,
    ) -> Result<String, String> {
        let samples: Vec<_> = cards
            .iter()
            .take(10)
            .map(|c| json!({"kind":c.kind, "tags":c.tags}))
            .collect();
        let value = self.invoke(
            json!({"operation":"name_group", "cards":samples, "parent":parent}),
            timeout,
        )?;
        value["name"]
            .as_str()
            .map(|n| n.trim().to_owned())
            .filter(|n| !n.is_empty())
            .ok_or_else(|| "Invalid folder name".into())
    }
    fn group_subjects(
        &self,
        names: &[String],
        details: &[String],
        timeout: Duration,
    ) -> Result<Vec<String>, String> {
        let value = self.invoke(
            json!({"operation":"group_subjects", "names":names, "details":details}),
            timeout,
        )?;
        let subjects = value["subjects"]
            .as_object()
            .ok_or("Invalid subject response")?;
        Ok(names
            .iter()
            .map(|n| {
                subjects
                    .get(n)
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .trim()
                    .to_owned()
            })
            .collect())
    }
    fn merge_names(
        &self,
        names: &[String],
        timeout: Duration,
    ) -> Result<Vec<(String, String)>, String> {
        let value = self.invoke(json!({"operation":"merge_names", "names":names}), timeout)?;
        Ok(value["merges"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|m| {
                Some((
                    m["from"].as_str()?.to_owned(),
                    m["into"].as_str()?.to_owned(),
                ))
            })
            .collect())
    }
    fn choose_folder(
        &self,
        card: &Card,
        excerpt: &str,
        candidates: &[Candidate],
        root_rules: &str,
        timeout: Duration,
    ) -> Result<Option<usize>, String> {
        let summary = json!({"kind":card.kind, "issuer":card.issuer, "tags":card.tags});
        let value = self.invoke(
            json!({"operation":"choose_folder", "card":summary, "excerpt":excerpt, "candidates":candidates, "rootRules":root_rules}),
            timeout,
        )?;
        let folder = value["folder"].as_str().ok_or("Invalid folder choice")?;
        Ok(candidates.iter().position(|c| c.folder == folder))
    }
}
