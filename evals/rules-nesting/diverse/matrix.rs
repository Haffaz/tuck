use crate::model::{Preferences, Rule};
use serde_json::{json, Value};
use std::{fs, path::Path};
fn output(out: &Path, report: &Value) {
    fs::write(
        out.join("results.json"),
        serde_json::to_vec_pretty(report).unwrap(),
    )
    .unwrap();
}
#[test]
fn live_diverse_matrix() {
    let out = std::path::PathBuf::from(std::env::var("TUCK_MATRIX_OUT").unwrap());
    let fixtures: Vec<Value> =
        serde_json::from_slice(&fs::read(out.join("fixtures.json")).unwrap()).unwrap();
    let temp = tempfile::tempdir_in("/private/tmp").unwrap();
    let root = temp.path().canonicalize().unwrap();
    fs::create_dir(root.join(".tuck")).unwrap();
    let reader = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/tuck-reader");
    let mut docs = vec![];
    for (i, f) in fixtures.iter().enumerate() {
        let path = root.join(format!("{i:02}-{}.txt", f["id"].as_str().unwrap()));
        fs::write(&path, f["text"].as_str().unwrap()).unwrap();
        docs.push(crate::collection::read(&path, &root, &reader));
    }
    let mut report = json!({"runs":[]});
    for depth in 1..=2u8 {
        for customized in [false, true] {
            let mut rules = vec![];
            let mut expected = vec![];
            for f in &fixtures {
                let Some(subject) = f["subject"].as_str() else {
                    expected.push(None);
                    continue;
                };
                let entity = f["entity"].as_str();
                let kind = f["kind"].as_str();
                let folder = if depth == 1 || entity.is_none() {
                    subject.to_string()
                } else if depth == 2 {
                    format!("{subject}/{}", entity.unwrap())
                } else {
                    format!("{subject}/{}/{}", entity.unwrap(), kind.unwrap())
                };
                expected.push(Some(folder.clone()));
                if !rules.iter().any(|r: &Rule| r.folder == folder) {
                    let instruction = if depth == 1 {
                        format!(
                            "File {} documents here. Other subjects have their own folders.",
                            subject
                        )
                    } else if let Some(entity) = entity {
                        format!("File {} documents whose project, supplier, clinic, institution, product or destination is explicitly {}. Other named entities do not belong here.",f["text"].as_str().unwrap().lines().next().unwrap(),entity)
                    } else {
                        "Meeting notes where no project is identified. Named projects belong in their own child folders.".to_string()
                    };
                    rules.push(Rule { document_types: Vec::new(),
                        folder,
                        instruction,
                        enabled: true,
                    });
                }
            }
            let preferences = if customized {
                "Keep project notes in Work, supplier invoices in Invoices, clinical reports in Health, academic transcripts in Education, product warranties in Warranties and travel reservations in Travel. When supported, group by project, supplier, clinic, institution, product or destination, then document type within the selected nesting ceiling. Keep missing details in a broad subject."
            } else {
                ""
            };
            let p = Preferences {
                root: root.to_string_lossy().into_owned(),
                folder_nesting: depth,
                rules,
                instructions: if customized {
                    "Use clear English filenames starting with Record. Omit dates and amounts."
                } else {
                    "Use clear English names without inventing facts."
                }
                .into(),
                category_instructions: preferences.into(),
                ..Preferences::default()
            };
            crate::files::validate(&p).unwrap();
            let map = crate::collection::build(docs.clone(), &p);
            let proposal = match crate::categories::suggest_map(&p, &reader, &map, |_| Ok(())) {
                Ok(x) => json!({"ok":true,"rules":x.rules,"assignments":x.assignments}),
                Err(e) => json!({"ok":false,"error":e}),
            };
            let mut row =
                json!({"depth":depth,"customized":customized,"proposal":proposal,"decisions":[]});
            for (i, f) in fixtures.iter().enumerate() {
                let path = Path::new(&docs[i].path);
                let d = crate::classifier::classify_with_tools(
                    path.file_name().unwrap().to_str().unwrap(),
                    &docs[i].text,
                    &p,
                    &[],
                    Some((path, &reader)),
                );
                let actual = if d.confident {
                    Some(d.folder.clone())
                } else {
                    None
                };
                assert_eq!(
                    crate::files::digest(path).unwrap(),
                    f["sha256"].as_str().unwrap()
                );
                row["decisions"].as_array_mut().unwrap().push(json!({"id":f["id"],"expected":expected[i],"correct":actual==expected[i],"wrongAutomatic":actual.is_some()&&actual!=expected[i],"decision":d,"sourceUnchanged":true}));
                report["current"] = row.clone();
                output(&out, &report);
            }
            println!(
                "Diverse depth{depth} customized{customized}: {}/{}",
                row["decisions"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|d| d["correct"] == true)
                    .count(),
                fixtures.len()
            );
            report["runs"].as_array_mut().unwrap().push(row);
            report.as_object_mut().unwrap().remove("current");
            output(&out, &report);
        }
    }
    report["completed"] = json!(true);
    output(&out, &report);
}
