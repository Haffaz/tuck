use crate::{
    category_evidence::{self, Assignment, Sample},
    model::Rule,
    provider::{MatchInput, MatchReply, ModelProvider, NameInput, ReaderReply},
};
use serde_json::{json, Value};
use std::{cell::Cell, time::Duration};
struct PlannedGroups {
    calls: Cell<usize>,
    education: bool,
}
impl ModelProvider for PlannedGroups {
    fn status(&self) -> Result<String, String> {
        unreachable!()
    }
    fn identity(&self) -> Result<String, String> {
        unreachable!()
    }
    fn suggest_rule(&self, _: &str, _: &str) -> Result<String, String> {
        unreachable!()
    }
    fn describe(
        &self,
        _: &str,
        _: &[&str],
        _: &[Value],
        _: Duration,
    ) -> Result<ReaderReply, String> {
        unreachable!()
    }
    fn categories(&self, _: &[String], _: Duration) -> Result<Vec<Rule>, String> {
        unreachable!()
    }
    fn match_document(&self, _: &MatchInput<'_>, _: Duration) -> Result<MatchReply, String> {
        unreachable!()
    }
    fn verify_match(&self, _: &MatchInput<'_>, _: usize, _: Duration) -> Result<bool, String> {
        unreachable!()
    }
    fn name(&self, _: &NameInput<'_>, _: Duration) -> Result<String, String> {
        unreachable!()
    }
    fn group_documents(&self, samples: &[Sample], _: Duration) -> Result<Vec<Assignment>, String> {
        self.calls.set(self.calls.get() + 1);
        Ok(samples
            .iter()
            .map(|s| Assignment {
                id: s.id.clone(),
                category: if self.education {
                    format!("Education/{}", s.description)
                } else if s.id.parse::<usize>().unwrap() < 12 {
                    "Research/Orion".into()
                } else {
                    "Research/Vega".into()
                },
                preserve_depth: 0,
            })
            .collect())
    }
}
#[test]
fn collection_structure_after_batches_and_before_rules() {
    let model = PlannedGroups {
        calls: Cell::new(0),
        education: false,
    };
    let mut samples: Vec<_> = (0..24)
        .map(|i| Sample {
            id: i.to_string(),
            description: "Technical inspection report".into(),
            quote: "Technical inspection report".into(),
            detail_evidence: format!("Project: {}", if i < 12 { "Orion" } else { "Vega" }),
            source_text: String::new(),
            page_heads: vec![],
        })
        .collect();
    let grouped = category_evidence::group(&model, &mut samples, 2, "", |_| Ok(())).unwrap();
    assert!(model.calls.get() > 1);
    assert_eq!(grouped.rules.len(), 2);
    assert_eq!(
        grouped
            .assignments
            .iter()
            .filter(|a| a.category == "Research/Orion")
            .count(),
        12
    );
    assert_eq!(
        grouped
            .assignments
            .iter()
            .filter(|a| a.category == "Research/Vega")
            .count(),
        12
    );
    let model = PlannedGroups {
        calls: Cell::new(0),
        education: true,
    };
    let mut samples: Vec<_> = ["Degree", "Certification", "Transcript"]
        .iter()
        .enumerate()
        .map(|(i, t)| Sample {
            id: i.to_string(),
            description: t.to_string(),
            quote: format!("Academic {t}"),
            detail_evidence: String::new(),
            source_text: String::new(),
            page_heads: vec![],
        })
        .collect();
    let grouped = category_evidence::group(&model, &mut samples, 2, "", |_| Ok(())).unwrap();
    assert_eq!(grouped.rules.len(), 1);
    assert_eq!(grouped.rules[0].folder, "Education");
    for t in ["Degree", "Certification", "Transcript"] {
        assert!(grouped.rules[0].document_types.contains(&t.to_string()));
    }
    let out = std::env::var("TUCK_STRUCTURE_BOUNDARY_REPORT").unwrap();
    std::fs::write(
        out,
        serde_json::to_vec_pretty(
            &json!({"crossBatchCounts":true,"mergedRuleIncludesEveryType":true,"passed":true}),
        )
        .unwrap(),
    )
    .unwrap();
}

#[test]
fn collection_structure_private_replay_when_requested() {
    let Ok(input) = std::env::var("TUCK_STRUCTURE_INPUT") else {
        return;
    };
    let mut rows: Vec<Assignment> = serde_json::from_slice(&std::fs::read(input).unwrap()).unwrap();
    let before = rows.clone();
    category_evidence::compact_assignments(&mut rows);
    let counts = |assignments: &[Assignment]| {
        let mut counts = std::collections::BTreeMap::<String, usize>::new();
        for a in assignments {
            *counts.entry(a.category.clone()).or_default() += 1;
        }
        counts
    };
    std::fs::write(
        std::env::var("TUCK_STRUCTURE_REPLAY_REPORT").unwrap(),
        serde_json::to_vec_pretty(
            &json!({"before":counts(&before),"after":counts(&rows),"assignments":rows}),
        )
        .unwrap(),
    )
    .unwrap();
}

#[test]
fn collection_structure_retains_every_document_type() {
    let model=PlannedGroups {calls:Cell::new(0),education:true};
    let mut samples:Vec<_>=(0..24).map(|i|Sample{id:format!("d{i:04}"),description:format!("Academic qualification {i}"),quote:format!("Academic qualification {i}"),detail_evidence:String::new(),source_text:String::new(),page_heads:vec![]}).collect();
    let result=category_evidence::group(&model,&mut samples,2,"", |_|Ok(())).unwrap();
    assert_eq!(result.rules.len(),1);
    assert_eq!(result.rules[0].document_types.len(),24);
    assert!(result.rules[0].instruction.chars().count()<=600);
    assert_eq!(result.assignments.len(),24);
}
