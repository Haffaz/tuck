//! Provider-neutral role contracts. Add another adapter here without giving models file access.
use crate::{apple, description::Understanding, model::Rule};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum ReaderReply {
    Finish {
        description: String,
        quotes: Vec<String>,
        uncertainty: String,
    },
    ReadPage {
        #[serde(default)]
        page: Option<usize>,
    },
    RetryOcr {
        #[serde(default)]
        page: Option<usize>,
    },
}
#[derive(Clone, Serialize)]
pub struct Destination {
    #[serde(rename = "documentTypes")]
    pub document_types: Vec<String>,
    pub folder: String,
    pub instruction: String,
    pub context: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchInput<'a> {
    pub understanding: &'a Understanding,
    pub evidence: &'a str,
    pub quotes: Vec<String>,
    pub destinations: Vec<Destination>,
    pub source_folders: Vec<String>,
    pub root_rules: &'a str,
    pub can_read_more: bool,
}
#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum MatchReply {
    Finish {
        selection: usize,
        reason: String,
        quote: String,
        #[serde(default)]
        source_group: String,
    },
    NeedEvidence {
        #[serde(default)]
        page: Option<usize>,
    },
    Uncertain {
        reason: String,
    },
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NameInput<'a> {
    pub root_rules: &'a str,
    pub understanding: &'a Understanding,
    pub evidence: &'a str,
    pub rules: &'a str,
}

/// Synchronous by design: the coordinator completes one stage before starting the next.
/// Adapters return data only. None receive document paths or write tools.
pub trait ModelProvider {
    fn status(&self) -> Result<String, String>;
    fn suggest_rule(&self, folder: &str, context: &str) -> Result<String, String>;
    fn identity(&self) -> Result<String, String>;
    fn describe(
        &self,
        text: &str,
        actions: &[&str],
        completed: &[Value],
        timeout: Duration,
    ) -> Result<ReaderReply, String>;
    fn uses_evidence_categories(&self) -> bool {
        false
    }
    fn group_documents(
        &self,
        _: &[crate::category_evidence::Sample],
        _: Duration,
    ) -> Result<Vec<crate::category_evidence::Assignment>, String> {
        Err("Evidence grouping is unavailable for this provider".into())
    }
    fn group_documents_with_context(
        &self,
        samples: &[crate::category_evidence::Sample],
        subjects: &[String],
        timeout: Duration,
    ) -> Result<Vec<crate::category_evidence::Assignment>, String> {
        let _ = subjects;
        self.group_documents(samples, timeout)
    }
    fn categories(&self, descriptions: &[String], timeout: Duration) -> Result<Vec<Rule>, String>;
    fn match_document(
        &self,
        input: &MatchInput<'_>,
        timeout: Duration,
    ) -> Result<MatchReply, String>;
    fn verify_match(
        &self,
        input: &MatchInput<'_>,
        selection: usize,
        timeout: Duration,
    ) -> Result<bool, String>;
    fn name(&self, input: &NameInput<'_>, timeout: Duration) -> Result<String, String>;
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
#[cfg(test)]
pub fn local(reader: &Path) -> LocalProvider {
    selected(reader, crate::model::LocalModel::Apple)
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
            "{}:roles-v7-evidence:{}:{}",
            self.model.key(),
            digest,
            crate::files::digest(&self.helper)?
        ))
    }

    fn describe(
        &self,
        text: &str,
        actions: &[&str],
        completed: &[Value],
        timeout: Duration,
    ) -> Result<ReaderReply, String> {
        serde_json::from_value(self.invoke(
            if self.uses_evidence_categories() {
                json!({"operation":"describe", "text":text, "quotes":crate::category_evidence::source_lines(text)})
            } else { apple::description_request(text, actions, completed) },
            timeout,
        )?)
        .map_err(|_| "Invalid reader response".into())
    }
    fn uses_evidence_categories(&self) -> bool {
        matches!(
            self.model,
            crate::model::LocalModel::Apple | crate::model::LocalModel::Custom
        )
    }
    fn group_documents(
        &self,
        samples: &[crate::category_evidence::Sample],
        timeout: Duration,
    ) -> Result<Vec<crate::category_evidence::Assignment>, String> {
        self.group_documents_with_context(samples, &[], timeout)
    }
    fn group_documents_with_context(
        &self,
        samples: &[crate::category_evidence::Sample],
        subjects: &[String],
        timeout: Duration,
    ) -> Result<Vec<crate::category_evidence::Assignment>, String> {
        let value = self.invoke(
            json!({"operation":"group_documents", "documents":samples, "includeLayoutHints":true,"establishedSubjects":subjects}),
            timeout,
        )?;
        let assignments: Vec<crate::category_evidence::Assignment> =
            serde_json::from_value(value["assignments"].clone())
                .map_err(|_| "Invalid document assignments")?;
        crate::category_evidence::validate_assignments_with_depth(
            samples,
            &assignments,
            self.preferences
                .as_ref()
                .map_or(2, |p| p.folder_nesting as usize),
        )?;
        Ok(assignments)
    }
    fn categories(&self, descriptions: &[String], timeout: Duration) -> Result<Vec<Rule>, String> {
        let value = self.invoke(
            json!({"operation":"categories","samples":descriptions}),
            timeout,
        )?;
        crate::categories::validate_rules_with_depth(
            value["rules"].clone(),
            self.preferences
                .as_ref()
                .map_or(2, |p| p.folder_nesting as usize),
        )
    }
    fn match_document(
        &self,
        input: &MatchInput<'_>,
        timeout: Duration,
    ) -> Result<MatchReply, String> {
        let value = self.invoke(json!({"operation":"match", "input":input}), timeout)?;
        serde_json::from_value(value).map_err(|_| "Invalid matcher response".into())
    }
    fn verify_match(
        &self,
        input: &MatchInput<'_>,
        selection: usize,
        timeout: Duration,
    ) -> Result<bool, String> {
        let value = self.invoke(
            json!({"operation":"verify_match", "input":{"match":input,"selection":selection}}),
            timeout,
        )?;
        value["supported"]
            .as_bool()
            .ok_or_else(|| "Invalid verification response".into())
    }
    fn name(&self, input: &NameInput<'_>, timeout: Duration) -> Result<String, String> {
        let value = self.invoke(json!({"operation":"name", "input":input}), timeout)?;
        let title = value["title"].as_str().ok_or("Invalid naming response")?;
        if title.chars().count() > 120 {
            return Err("Suggested name is too long".into());
        }
        Ok(title.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn page_is_optional_for_read_requests_and_absent_for_finish() {
        assert!(matches!(
            serde_json::from_value::<ReaderReply>(json!({"action":"read_page"})).unwrap(),
            ReaderReply::ReadPage { page: None }
        ));
        assert!(matches!(
            serde_json::from_value::<ReaderReply>(
                json!({"action":"finish","description":"Policy","quotes":[],"uncertainty":""})
            )
            .unwrap(),
            ReaderReply::Finish { .. }
        ));
        assert!(matches!(
            serde_json::from_value::<MatchReply>(json!({"action":"need_evidence"})).unwrap(),
            MatchReply::NeedEvidence { page: None }
        ));
    }
    #[test]
    fn providers_cannot_return_arbitrary_tools_or_folder_paths() {
        assert!(
            serde_json::from_value::<ReaderReply>(json!({"action":"move_file","page":1})).is_err()
        );
        assert!(serde_json::from_value::<MatchReply>(
            json!({"action":"finish","folder":"../escape","quote":"evidence","reason":"match"})
        )
        .is_err());
    }
}
