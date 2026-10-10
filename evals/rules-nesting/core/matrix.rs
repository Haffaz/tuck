use crate::{
    model::{Preferences, Rule},
    provider::ModelProvider,
};
use serde_json::{json, Value};
use std::{
    fs,
    path::Path,
    time::{Duration, Instant},
};

fn rule(folder: &str, instruction: String) -> Rule {
    Rule { document_types: Vec::new(),
        folder: folder.into(),
        instruction,
        enabled: true,
    }
}
fn destinations(depth: u8, specific: bool) -> Vec<Rule> {
    let mut rules = vec![
        rule("Housing", "Tenancy agreements and household utility bills.".into()),
        rule("Insurance", if specific { "Home insurance and motor insurance policies. Exclude travel insurance, which belongs in Travel." } else { "All insurance policies, including home, motor and travel insurance. Travel tickets and bookings belong in Travel." }.into()),
    ];
    if depth == 1 {
        rules.push(rule("Income", "All salary payslips and salary payment statements. Employment contracts belong in Employment.".into()));
        rules.push(rule(
            "Employment",
            "Employment contracts and employment certificates.".into(),
        ));
    } else {
        for (employer, full) in [("Acme", "Acme Corporation"), ("Beta", "Beta Studio")] {
            let folder = if depth == 2 {
                format!("Income/{employer}")
            } else {
                format!("Income/{employer}/Payslips")
            };
            rules.push(rule(&folder, if specific { format!("Salary payslips issued by employer {full} only. Do not use this folder when the employer is missing or different.") } else { format!("File documents about {folder} here. Examples: Monthly salary payslip from {full}.") }));
        }
        rules.push(rule("Income", "Salary payslips where the employer name is absent. Named employers have their own subfolders.".into()));
        let folder = if depth == 2 {
            "Employment/Acme"
        } else {
            "Employment/Acme/Contracts"
        };
        rules.push(rule(
            folder,
            "Employment contracts issued by Acme Corporation. Salary payslips belong under Income."
                .into(),
        ));
    }
    let folder = if specific && depth == 2 {
        "Travel/Insurance"
    } else if specific && depth == 3 {
        "Travel/Insurance/Policies"
    } else {
        "Travel"
    };
    rules.push(rule(folder, if specific { "Travel insurance certificates and policies. For this user's collection, keep travel insurance here. Home insurance belongs in Insurance." } else { "Travel tickets, reservations and bookings only. Insurance policies belong in Insurance." }.into()));
    rules
}
fn expected(fixture: &Value, depth: u8, specific: bool) -> Option<String> {
    let subject = fixture["subject"].as_str()?;
    if fixture["travel"] == true && specific {
        return Some(
            match depth {
                1 => "Travel",
                2 => "Travel/Insurance",
                _ => "Travel/Insurance/Policies",
            }
            .into(),
        );
    }
    if depth > 1 && subject == "Income" {
        if let Some(employer) = fixture["employer"].as_str() {
            return Some(if depth == 2 {
                format!("Income/{employer}")
            } else {
                format!("Income/{employer}/Payslips")
            });
        }
    }
    if depth > 1 && subject == "Employment" {
        return Some(
            if depth == 2 {
                "Employment/Acme"
            } else {
                "Employment/Acme/Contracts"
            }
            .into(),
        );
    }
    Some(subject.into())
}
fn creation_preferences(depth: u8) -> String {
    let details = match depth {
        1 => "Keep salary payslips in Income and employment contracts in Employment. Use broad root-level folders.",
        2 => "Keep salary payslips under Income, grouped by the explicitly named employer: Income/Acme for Acme Corporation and Income/Beta for Beta Studio. Employment contracts belong under Employment/Acme. Put missing employer names in broad Income.",
        _ => "Keep salary payslips under Income, then by the explicitly named employer, then by document type: Income/Acme/Payslips and Income/Beta/Payslips. Employment contracts belong in Employment/Acme/Contracts. Put missing employer names in broad Income."
    };
    format!("{details} Keep tenancy agreements in Housing and all insurance policies, including travel insurance, in Insurance. Do not create employer or date subfolders for Housing or Insurance. Never invent an employer or create a folder for an amount or date.")
}
fn output(out: &Path, report: &Value) {
    fs::write(
        out.join("results.json"),
        serde_json::to_vec_pretty(report).unwrap(),
    )
    .unwrap();
}

struct Trace<'a> {
    inner: &'a dyn ModelProvider,
    calls: std::cell::RefCell<Vec<Value>>,
}
impl ModelProvider for Trace<'_> {
    fn status(&self) -> Result<String, String> {
        self.inner.status()
    }
    fn suggest_rule(&self, f: &str, c: &str) -> Result<String, String> {
        self.inner.suggest_rule(f, c)
    }
    fn identity(&self) -> Result<String, String> {
        self.inner.identity()
    }
    fn uses_evidence_categories(&self) -> bool {
        self.inner.uses_evidence_categories()
    }
    fn describe(
        &self,
        t: &str,
        a: &[&str],
        c: &[Value],
        d: Duration,
    ) -> Result<crate::provider::ReaderReply, String> {
        self.inner.describe(t, a, c, d)
    }
    fn categories(&self, s: &[String], d: Duration) -> Result<Vec<Rule>, String> {
        self.inner.categories(s, d)
    }
    fn group_documents(
        &self,
        s: &[crate::category_evidence::Sample],
        d: Duration,
    ) -> Result<Vec<crate::category_evidence::Assignment>, String> {
        self.inner.group_documents(s, d)
    }
    fn group_documents_with_context(
        &self,
        s: &[crate::category_evidence::Sample],
        subjects: &[String],
        d: Duration,
    ) -> Result<Vec<crate::category_evidence::Assignment>, String> {
        self.inner.group_documents_with_context(s, subjects, d)
    }
    fn match_document(
        &self,
        i: &crate::provider::MatchInput<'_>,
        d: Duration,
    ) -> Result<crate::provider::MatchReply, String> {
        let result = self.inner.match_document(i, d);
        self.calls
            .borrow_mut()
            .push(json!({"role":"match","input":i,"result":format!("{result:?}")}));
        result
    }
    fn verify_match(
        &self,
        i: &crate::provider::MatchInput<'_>,
        s: usize,
        d: Duration,
    ) -> Result<bool, String> {
        let result = self.inner.verify_match(i, s, d);
        self.calls.borrow_mut().push(json!({"role":"verify","selection":s,"folder":i.destinations[s].folder,"result":format!("{result:?}")}));
        result
    }
    fn name(&self, i: &crate::provider::NameInput<'_>, d: Duration) -> Result<String, String> {
        let result = self.inner.name(i, d);
        self.calls
            .borrow_mut()
            .push(json!({"role":"name","input":i,"result":format!("{result:?}")}));
        result
    }
}

#[test]
fn live_rules_nesting_matrix() {
    let out = std::path::PathBuf::from(std::env::var("TUCK_MATRIX_OUT").unwrap());
    let fixtures: Vec<Value> =
        serde_json::from_slice(&fs::read(out.join("fixtures.json")).unwrap()).unwrap();
    let temp = tempfile::tempdir_in("/private/tmp").unwrap();
    let root = temp.path().canonicalize().unwrap();
    fs::create_dir(root.join(".tuck")).unwrap();
    let reader = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/tuck-reader");
    let start = Instant::now();
    let mut report =
        json!({"fixtures":fixtures.len(),"runs":[],"edgeCases":[],"sourceUnchanged":true});
    crate::provider::selected(&reader, crate::model::LocalModel::Apple)
        .status()
        .expect("Real Apple Intelligence generation must work");
    let mut documents = vec![];
    for (i, fixture) in fixtures.iter().enumerate() {
        let path = root.join(format!("{i:02}-{}.txt", fixture["id"].as_str().unwrap()));
        fs::write(&path, fixture["text"].as_str().unwrap()).unwrap();
        assert_eq!(
            crate::files::digest(&path).unwrap(),
            fixture["sha256"].as_str().unwrap()
        );
        documents.push(crate::collection::read(&path, &root, &reader));
    }
    // All 24 combinations, then six preplanned repeats of the two endpoints.
    let mut plan = vec![];
    for depth in 1..=2u8 {
        for mask in 0..8u8 {
            plan.push((depth, mask, 1));
        }
    }
    for depth in 1..=2u8 {
        for mask in [0u8, 7] {
            plan.push((depth, mask, 2));
        }
    }
    if std::env::var_os("TUCK_MATRIX_PILOT").is_some() {
        plan.retain(|(_, mask, repeat)| [0u8, 7].contains(mask) && *repeat == 1);
    }
    for (depth, mask, repeat) in plan {
        let global = mask & 1 != 0;
        let creation = mask & 2 != 0;
        let specific = mask & 4 != 0;
        let id = format!(
            "depth{depth}-D{}C{}F{}-r{repeat}",
            u8::from(global),
            u8::from(creation),
            u8::from(specific)
        );
        let started = Instant::now();
        let p = Preferences {
            root: root.to_string_lossy().into_owned(), folder_nesting: depth,
            instructions: if global { "Use clear English document names. Every new filename must start with Record. Omit all dates and amounts. Never call an insurance policy a bonus." } else { "Use clear English document names without inventing facts." }.into(),
            category_instructions: if creation { creation_preferences(depth) } else { String::new() },
            rules: destinations(depth, specific), ..Preferences::default()
        };
        crate::files::validate(&p).unwrap();
        let mut input_documents = documents.clone();
        if repeat == 2 {
            input_documents.reverse();
        }
        let map = crate::collection::build(input_documents, &p);
        let proposal_start = Instant::now();
        let proposal = match crate::categories::suggest_map(&p, &reader, &map, |_| Ok(())) {
            Ok(proposal) => {
                json!({"ok":true,"rules":proposal.rules,"assignments":proposal.assignments,"sampled":proposal.sampled,"skipped":proposal.skipped,"limited":proposal.limited})
            }
            Err(error) => json!({"ok":false,"error":error}),
        };
        let mut decisions = vec![];
        let mut row = json!({"id":id,"depth":depth,"documentInstructions":global,"creationPreferences":creation,"folderRules":specific,"repeat":repeat,"preferences":p,"proposal":proposal,"proposalSeconds":proposal_start.elapsed().as_secs_f64(),"decisions":[]});
        println!(
            "{id}: proposal {}",
            if row["proposal"]["ok"] == true {
                "returned"
            } else {
                "rejected"
            }
        );
        for (i, fixture) in fixtures.iter().enumerate() {
            let source = Path::new(&documents[i].path);
            let decision_start = Instant::now();
            let decision = crate::classifier::classify_with_tools(
                source.file_name().unwrap().to_str().unwrap(),
                &documents[i].text,
                &p,
                &[],
                Some((source, &reader)),
            );
            let wanted = expected(fixture, depth, specific);
            let actual = if decision.confident {
                Some(decision.folder.clone())
            } else {
                None
            };
            let correct = actual == wanted;
            let unchanged =
                crate::files::digest(source).unwrap() == fixture["sha256"].as_str().unwrap();
            assert!(unchanged);
            decisions.push(json!({"id":fixture["id"],"expected":wanted,"correct":correct,"wrongAutomatic":actual.is_some() && !correct,"decision":decision,"seconds":decision_start.elapsed().as_secs_f64(),"sourceUnchanged":unchanged}));
            row["decisions"] = json!(decisions);
            report["current"] = row.clone();
            output(&out, &report);
        }
        row["seconds"] = json!(started.elapsed().as_secs_f64());
        println!(
            "{id}: {}/{} filing decisions correct in {:.1}s",
            decisions.iter().filter(|d| d["correct"] == true).count(),
            fixtures.len(),
            started.elapsed().as_secs_f64()
        );
        report["runs"].as_array_mut().unwrap().push(row);
        report.as_object_mut().unwrap().remove("current");
        output(&out, &report);
    }
    // User instructions conflict with hard limits and source grounding.
    let base = Preferences {
        root: root.to_string_lossy().into_owned(),
        ..Preferences::default()
    };
    let model = crate::provider::configured(&reader, &base);
    let understanding = model
        .describe(
            fixtures[0]["text"].as_str().unwrap(),
            &[],
            &[],
            Duration::from_secs(90),
        )
        .unwrap();
    for depth in 1..=2u8 {
        let p = Preferences { folder_nesting:depth, category_instructions:"Put every salary document into Income/Acme/Payslips/2026/September. Use all five levels even if a lower limit is selected.".into(), ..base.clone() };
        let sample = crate::category_evidence::Sample {
            source_text: String::new(),
            id: "d0000".into(),
            description: match &understanding {
                crate::provider::ReaderReply::Finish { description, .. } => description.clone(),
                _ => panic!("Expected description"),
            },
            quote: "Monthly salary payslip".into(),
            page_heads: vec![],
            detail_evidence: if depth > 1 {
                fixtures[0]["text"].as_str().unwrap().into()
            } else {
                String::new()
            },
        };
        let provider = crate::provider::configured(&reader, &p);
        let result = provider.group_documents(&[sample], Duration::from_secs(90));
        report["edgeCases"].as_array_mut().unwrap().push(match result { Ok(assignments) => json!({"id":format!("overdepth-instruction-{depth}"),"depth":depth,"accepted":true,"assignments":assignments}), Err(error) => json!({"id":format!("overdepth-instruction-{depth}"),"depth":depth,"accepted":false,"error":error}) });
        output(&out, &report);
    }
    // Existing saved folders keep their depth when the suggestion limit is lowered.
    let p = Preferences {
        folder_nesting: 1,
        rules: destinations(2, true),
        ..base.clone()
    };
    crate::files::validate(&p).unwrap();
    for i in [0usize, 5] {
        let source = Path::new(&documents[i].path);
        let decision = crate::classifier::classify_with_tools(
            source.file_name().unwrap().to_str().unwrap(),
            &documents[i].text,
            &p,
            &[],
            Some((source, &reader)),
        );
        report["edgeCases"].as_array_mut().unwrap().push(json!({"id":format!("lowered-limit-existing-{}",fixtures[i]["id"].as_str().unwrap()),"expected":expected(&fixtures[i],2,true),"decision":decision}));
    }
    // Disable a parent to make its nested children unavailable.
    let mut p = Preferences {
        rules: destinations(2, true),
        ..base.clone()
    };
    let mut parent = rule(
        "Income",
        "Salary records, including child employer folders.".into(),
    );
    parent.enabled = false;
    p.rules.retain(|r| r.folder != "Income");
    p.rules.push(parent);
    let source = Path::new(&documents[0].path);
    let decision = crate::classifier::classify_with_tools(
        source.file_name().unwrap().to_str().unwrap(),
        &documents[0].text,
        &p,
        &[],
        Some((source, &reader)),
    );
    report["edgeCases"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"disabled-parent","expected":null,"decision":decision}));
    for depth in [0u8, 3, 4, 5, 45] {
        let p = Preferences {
            folder_nesting: depth,
            ..base.clone()
        };
        report["edgeCases"].as_array_mut().unwrap().push(json!({"id":format!("invalid-setting-{depth}"),"rejected":crate::files::validate(&p).is_err()}));
    }
    report["seconds"] = json!(start.elapsed().as_secs_f64());
    report["completed"] = json!(true);
    output(&out, &report);
    println!(
        "Evaluation complete: {} runs, {} edge cases",
        report["runs"].as_array().unwrap().len(),
        report["edgeCases"].as_array().unwrap().len()
    );
}

#[test]
fn live_rules_nesting_diagnostics() {
    let out = std::path::PathBuf::from(std::env::var("TUCK_MATRIX_OUT").unwrap());
    let fixtures: Vec<Value> =
        serde_json::from_slice(&fs::read(out.join("fixtures.json")).unwrap()).unwrap();
    let temp = tempfile::tempdir_in("/private/tmp").unwrap();
    let root = temp.path().canonicalize().unwrap();
    fs::create_dir(root.join(".tuck")).unwrap();
    let reader = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/tuck-reader");
    let base = "Use clear English document names without inventing facts.";
    let variants = [
        ("baseline",base),
        ("prefix-only","Use clear English document names. Every new filename must start with Record."),
        ("omit-dates-only","Use clear English document names. Omit all dates and amounts."),
        ("bonus-only","Use clear English document names. Never call an insurance policy a bonus."),
        ("prefix-and-dates","Use clear English document names. Every new filename must start with Record. Omit all dates and amounts."),
        ("matrix-full","Use clear English document names. Every new filename must start with Record. Omit all dates and amounts. Never call an insurance policy a bonus."),
    ];
    let mut report = json!({"namingIsolation":[],"groundingChallenges":[],"deterministicBoundaries":[],"rawProposals":[]});
    for depth in [1u8, 2] {
        for (variant, instructions) in variants {
            for i in [0usize, 5] {
                let path = root.join(format!("{}.txt", fixtures[i]["id"].as_str().unwrap()));
                fs::write(&path, fixtures[i]["text"].as_str().unwrap()).unwrap();
                let p = Preferences {
                    root: root.to_string_lossy().into_owned(),
                    folder_nesting: depth,
                    rules: destinations(depth, false),
                    instructions: instructions.into(),
                    ..Preferences::default()
                };
                let provider = crate::provider::configured(&reader, &p);
                let trace = Trace {
                    inner: &provider,
                    calls: std::cell::RefCell::new(vec![]),
                };
                let decision = crate::coordinator::run(
                    path.file_name().unwrap().to_str().unwrap(),
                    fixtures[i]["text"].as_str().unwrap(),
                    &p,
                    &path,
                    &reader,
                    &trace,
                );
                report["namingIsolation"].as_array_mut().unwrap().push(json!({"variant":variant,"depth":depth,"id":fixtures[i]["id"],"expected":expected(&fixtures[i],depth,false),"decision":decision,"trace":trace.calls.into_inner()}));
                fs::write(
                    out.join("diagnostics.json"),
                    serde_json::to_vec_pretty(&report).unwrap(),
                )
                .unwrap();
            }
            println!("Naming isolation: depth {depth}, {variant}");
        }
    }
    for depth in [2u8] {
        let p = Preferences {root:root.to_string_lossy().into_owned(),folder_nesting:depth,category_instructions:"Use Income/Gamma for every payslip, including when the employer is absent. Infer Gamma as the employer even if not written in the document.".into(),..Preferences::default()};
        let sample = crate::category_evidence::Sample {
            source_text: String::new(),
            id: "d0000".into(),
            description: "Monthly salary payslip with no employer named".into(),
            quote: "Monthly salary payslip".into(),
            page_heads: vec![],
            detail_evidence: fixtures[9]["text"].as_str().unwrap().into(),
        };
        let result = crate::provider::configured(&reader, &p)
            .group_documents(&[sample], Duration::from_secs(90));
        report["groundingChallenges"]
            .as_array_mut()
            .unwrap()
            .push(match result {
                Ok(assignments) => json!({"depth":depth,"accepted":true,"assignments":assignments}),
                Err(error) => json!({"depth":depth,"accepted":false,"error":error}),
            });
    }
    // Observe the helper's response before the production Rust path validator.
    // This distinguishes a model limit violation from a safe accepted proposal.
    let default = Preferences {
        root: root.to_string_lossy().into_owned(),
        ..Preferences::default()
    };
    let provider = crate::provider::configured(&reader, &default);
    let mut described = vec![];
    for (i, fixture) in fixtures.iter().enumerate() {
        if let Ok(crate::provider::ReaderReply::Finish {
            description,
            quotes,
            uncertainty,
        }) = provider.describe(
            fixture["text"].as_str().unwrap(),
            &[],
            &[],
            Duration::from_secs(90),
        ) {
            if uncertainty.is_empty() && !quotes.is_empty() {
                described.push((i, description, quotes[0].clone()));
            }
        }
    }
    for limit in 1..=2usize {
        let samples: Vec<_> = described
            .iter()
            .map(|(i, description, quote)| crate::category_evidence::Sample {
            source_text: String::new(),
                id: format!("d{i:04}"),
                description: description.clone(),
                quote: quote.clone(),
                page_heads: crate::category_evidence::page_heads(
                    fixtures[*i]["text"].as_str().unwrap(),
                ),
                detail_evidence: if limit > 1 {
                    fixtures[*i]["text"].as_str().unwrap().into()
                } else {
                    String::new()
                },
            })
            .collect();
        for guided in [false, true] {
            let request = json!({"model":"apple","operation":"group_documents","folderNesting":limit,"categoryInstructions":if guided {creation_preferences(limit as u8)} else {String::new()},"documents":samples});
            let raw = crate::apple::invoke(
                &reader.with_file_name("tuck-apple"),
                Some(request.clone()),
                Duration::from_secs(90),
            );
            let row = match raw {
                Ok(reply) => {
                    let assignments: Vec<crate::category_evidence::Assignment> =
                        serde_json::from_value(reply["assignments"].clone()).unwrap();
                    let validation = crate::category_evidence::validate_assignments_with_depth(
                        &samples,
                        &assignments,
                        limit,
                    );
                    json!({"limit":limit,"guided":guided,"request":request,"reply":reply,"acceptedByRust":validation.is_ok(),"validationError":validation.err()})
                }
                Err(error) => {
                    json!({"limit":limit,"guided":guided,"request":request,"helperError":error})
                }
            };
            report["rawProposals"].as_array_mut().unwrap().push(row);
        }
    }
    let sample = crate::category_evidence::Sample {
            source_text: String::new(),
        id: "d0".into(),
        description: "Monthly salary payslip".into(),
        quote: "Monthly salary payslip".into(),
        page_heads: vec![],
        detail_evidence: String::new(),
    };
    for limit in 1..=2usize {
        for actual in 1..=6usize {
            let category =
                ["Income", "Acme", "Payslips", "Year", "Month", "Detail"][..actual].join("/");
            let assignments = vec![crate::category_evidence::Assignment {
                id: "d0".into(),
                category: category.clone(),
                preserve_depth: 0,
            }];
            let accepted = crate::category_evidence::validate_assignments_with_depth(
                &[sample.clone()],
                &assignments,
                limit,
            )
            .is_ok();
            assert_eq!(accepted, actual <= limit);
            report["deterministicBoundaries"]
                .as_array_mut()
                .unwrap()
                .push(json!({"limit":limit,"path":category,"accepted":accepted,"correct":true}));
        }
        for category in [
            "Income/../Acme",
            "/Income",
            "Income//Acme",
            "Income/.hidden",
            "Income/Documents",
            "Income\\Acme",
        ] {
            let assignments = vec![crate::category_evidence::Assignment {
                id: "d0".into(),
                category: category.into(),
                preserve_depth: 0,
            }];
            let accepted = crate::category_evidence::validate_assignments_with_depth(
                &[sample.clone()],
                &assignments,
                limit,
            )
            .is_ok();
            assert!(!accepted);
            report["deterministicBoundaries"]
                .as_array_mut()
                .unwrap()
                .push(json!({"limit":limit,"path":category,"accepted":accepted,"correct":true}));
        }
    }
    report["completed"] = json!(true);
    fs::write(
        out.join("diagnostics.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
}

#[test]
fn live_clean_proposal_filing_controls() {
    let out = std::path::PathBuf::from(std::env::var("TUCK_MATRIX_OUT").unwrap());
    let fixtures: Vec<Value> =
        serde_json::from_slice(&fs::read(out.join("fixtures.json")).unwrap()).unwrap();
    let temp = tempfile::tempdir_in("/private/tmp").unwrap();
    let root = temp.path().canonicalize().unwrap();
    fs::create_dir(root.join(".tuck")).unwrap();
    let reader = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/tuck-reader");
    let indices = [0usize, 1, 2, 3, 4, 5, 9];
    let mut docs = vec![];
    for &i in &indices {
        let path = root.join(format!(
            "{i:02}-{}.txt",
            fixtures[i]["id"].as_str().unwrap()
        ));
        fs::write(&path, fixtures[i]["text"].as_str().unwrap()).unwrap();
        docs.push(crate::collection::read(&path, &root, &reader));
    }
    let mut report = json!({"note":"Post-matrix controls: excludes injected policy, unclear amount and mixed records. Simulates accepting proposed rules in memory; no saves or moves.","runs":[]});
    for depth in 1..=2u8 {
        for guided in [false, true] {
            let mut p = Preferences {
                root: root.to_string_lossy().into_owned(),
                folder_nesting: depth,
                category_instructions: if guided {
                    creation_preferences(depth)
                } else {
                    String::new()
                },
                instructions: "Use clear English document names without inventing facts.".into(),
                rules: destinations(depth, false),
                ..Preferences::default()
            };
            let map = crate::collection::build(docs.clone(), &p);
            let mut row = json!({"depth":depth,"guided":guided,"decisions":[]});
            match crate::categories::suggest_map(&p, &reader, &map, |_| Ok(())) {
                Ok(proposal) => {
                    p.rules = proposal.rules.clone();
                    row["proposal"] = json!({"ok":true,"rules":proposal.rules,"assignments":proposal.assignments});
                    for (j, &i) in indices.iter().enumerate() {
                        let path = Path::new(&docs[j].path);
                        let assigned = proposal
                            .assignments
                            .iter()
                            .find(|a| a.id == format!("d{j:04}"))
                            .map(|a| a.category.clone());
                        let decision = crate::classifier::classify_with_tools(
                            path.file_name().unwrap().to_str().unwrap(),
                            &docs[j].text,
                            &p,
                            &[],
                            Some((path, &reader)),
                        );
                        assert_eq!(
                            crate::files::digest(path).unwrap(),
                            fixtures[i]["sha256"].as_str().unwrap()
                        );
                        row["decisions"].as_array_mut().unwrap().push(json!({"id":fixtures[i]["id"],"subject":fixtures[i]["subject"],"employer":fixtures[i].get("employer"),"proposedFolder":assigned,"decision":decision,"sourceUnchanged":true}));
                    }
                }
                Err(error) => row["proposal"] = json!({"ok":false,"error":error}),
            }
            println!(
                "Clean control: depth {depth}, guided {guided}, proposal {}",
                row["proposal"]["ok"]
            );
            report["runs"].as_array_mut().unwrap().push(row);
            fs::write(
                out.join("clean-controls.json"),
                serde_json::to_vec_pretty(&report).unwrap(),
            )
            .unwrap();
        }
    }
    report["completed"] = json!(true);
    fs::write(
        out.join("clean-controls.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
}
