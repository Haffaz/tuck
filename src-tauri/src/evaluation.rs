//! Synthetic, opt-in evaluation of the actual Apple classification pipeline.
#[cfg(test)]
mod tests {
    use crate::{classifier, model::Preferences};
    use serde_json::{json, Value};
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{Instant, SystemTime, UNIX_EPOCH},
    };

    #[test]
    #[ignore = "Synthetic filing decisions using the evaluated category proposals; never moves files"]
    fn live_frozen_evidence_filing() {
        let corpus = PathBuf::from(std::env::var("TUCK_FROZEN_CORPUS").expect("Synthetic corpus"));
        let categories =
            PathBuf::from(std::env::var("TUCK_FROZEN_CATEGORY_REPORT").expect("Category report"));
        let report = PathBuf::from(std::env::var("TUCK_FROZEN_REPORT").expect("New report file"));
        assert!(!report.exists());
        let reader = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/tuck-reader");
        let inventory: Vec<Value> = ["inventory.json", "reserve-inventory.json"]
            .into_iter()
            .flat_map(|name| {
                serde_json::from_slice::<Vec<Value>>(
                    &fs::read(corpus.join("fixtures").join(name)).unwrap(),
                )
                .unwrap()
            })
            .collect();
        let mut results = Vec::new();
        let mut failures = Vec::new();
        let mut declined = Vec::new();
        let mut clear_count = 0usize;
        let mut correct_clear = 0usize;
        for split in ["dev", "heldout", "reserve"] {
            let proposal: Value = serde_json::from_slice(
                &fs::read(categories.join(format!("{split}-1.json"))).unwrap(),
            )
            .unwrap();
            let manifest: Vec<Value> = serde_json::from_slice(
                &fs::read(corpus.join("fixtures").join(format!("{split}.json"))).unwrap(),
            )
            .unwrap();
            let temp = tempfile::tempdir_in("/private/tmp").unwrap();
            let root = temp.path().canonicalize().unwrap();
            let p = Preferences {
                root: root.to_string_lossy().into_owned(),
                rename: false,
                rules: serde_json::from_value(proposal["rules"].clone()).unwrap(),
                ..Preferences::default()
            };
            for item in manifest {
                let source = corpus.join(item["path"].as_str().unwrap());
                let path = root.join(source.file_name().unwrap());
                fs::copy(&source, &path).unwrap();
                let gold = inventory.iter().find(|g| g["id"] == item["id"]).unwrap();
                let before = crate::files::digest(&path).unwrap();
                let text = classifier::read_document(&path, &reader).unwrap();
                let start = Instant::now();
                let decision = classifier::classify_with_tools(
                    path.file_name().unwrap().to_str().unwrap(),
                    &text,
                    &p,
                    &[],
                    Some((&path, &reader)),
                );
                let expected = gold["subject"].as_str();
                let actual = decision.confident.then(|| decision.folder.to_lowercase());
                let correct = actual.as_deref() == expected;
                if expected.is_some() {
                    clear_count += 1;
                    if correct {
                        correct_clear += 1;
                    }
                }
                if !correct {
                    let detail = format!(
                        "{}: {:?} expected {:?}; {}",
                        gold["id"], actual, expected, decision.reason
                    );
                    if actual.is_none() && expected.is_some() {
                        declined.push(detail);
                    } else {
                        failures.push(detail);
                    }
                }
                assert_eq!(crate::files::digest(&path).unwrap(), before);
                results.push(json!({"id":gold["id"],"expected":expected,"decision":decision,"correct":correct,"seconds":start.elapsed().as_secs_f64()}));
                fs::write(
                    &report,
                    serde_json::to_vec_pretty(&json!({"results":results,"failures":failures,"declined":declined,"clearCorrect":correct_clear,"clearTotal":clear_count}))
                        .unwrap(),
                )
                .unwrap();
            }
            println!("{split} filing decisions checked");
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
        assert!(
            correct_clear * 10 >= clear_count * 9,
            "Clear-document recall below 90%: {correct_clear}/{clear_count}; {}",
            declined.join("\n")
        );
        println!("Filing: {correct_clear}/{clear_count} clear documents correct; {} conservative reviews; no wrong automatic destinations", declined.len());
    }

    #[test]
    #[ignore = "Frozen synthetic evidence categorization; requires Apple Intelligence"]
    fn live_frozen_evidence_categories() {
        let corpus =
            PathBuf::from(std::env::var("TUCK_FROZEN_CORPUS").expect("Synthetic corpus directory"));
        let report =
            PathBuf::from(std::env::var("TUCK_FROZEN_REPORT").expect("New report directory"));
        assert!(!report.exists(), "Retain previous runs");
        fs::create_dir_all(&report).unwrap();
        let reader = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/tuck-reader");
        let inventory: Vec<Value> = ["inventory.json", "reserve-inventory.json"]
            .into_iter()
            .flat_map(|name| {
                serde_json::from_slice::<Vec<Value>>(
                    &fs::read(corpus.join("fixtures").join(name)).unwrap(),
                )
                .unwrap()
            })
            .collect();
        let mut failures = Vec::new();
        let mut runs = Vec::new();
        for split in ["dev", "heldout", "reserve"] {
            let manifest: Vec<Value> = serde_json::from_slice(
                &fs::read(corpus.join("fixtures").join(format!("{split}.json"))).unwrap(),
            )
            .unwrap();
            for repeat in 0..3 {
                let started = Instant::now();
                let temp = tempfile::tempdir_in("/private/tmp").unwrap();
                let root = temp.path().canonicalize().unwrap();
                for item in &manifest {
                    let src = corpus.join(item["path"].as_str().unwrap());
                    let gold = inventory.iter().find(|g| g["id"] == item["id"]).unwrap();
                    assert_eq!(
                        crate::files::digest(&src).unwrap(),
                        gold["sha256"].as_str().unwrap()
                    );
                    fs::copy(&src, root.join(src.file_name().unwrap())).unwrap();
                }
                let p = Preferences {
                    root: root.to_string_lossy().into_owned(),
                    ..Preferences::default()
                };
                let mut paths = crate::files::existing_documents(&root, true).unwrap();
                paths.sort();
                let expected: Vec<_> = paths
                    .iter()
                    .map(|path| {
                        inventory
                            .iter()
                            .find(|g| g["id"].as_str() == path.file_stem().and_then(|s| s.to_str()))
                            .unwrap()
                    })
                    .collect();
                let mut documents: Vec<_> = paths
                    .iter()
                    .map(|path| crate::collection::read(path, &root, &reader))
                    .collect();
                if repeat == 2 {
                    documents.reverse();
                }
                let map = crate::collection::build(documents, &p);
                let proposal =
                    crate::categories::suggest_map(&p, &reader, &map, |_| Ok(())).unwrap();
                let mut decisions = Vec::new();
                for (i, gold) in expected.iter().enumerate() {
                    let assigned = proposal
                        .assignments
                        .iter()
                        .find(|a| a.id == format!("d{i:04}"));
                    let category = assigned
                        .map(|a| a.category.as_str())
                        .filter(|s| *s != "Needs review");
                    let wanted = gold["subject"].as_str();
                    let correct = category.map(str::to_lowercase).as_deref() == wanted;
                    if !correct {
                        failures.push(format!(
                            "{split}-{repeat} {}: {:?} expected {:?}",
                            gold["id"], category, wanted
                        ));
                    }
                    assert_eq!(
                        crate::files::digest(&paths[i]).unwrap(),
                        gold["sha256"].as_str().unwrap()
                    );
                    decisions.push(json!({"id":gold["id"],"category":category,"expected":wanted,"correct":correct}));
                }
                let result = json!({"split":split,"repeat":repeat+1,"reverse":repeat==2,"seconds":started.elapsed().as_secs_f64(),"decisions":decisions,"rules":proposal.rules,"sampled":proposal.sampled,"skipped":proposal.skipped,"limited":proposal.limited});
                fs::write(
                    report.join(format!("{split}-{}.json", repeat + 1)),
                    serde_json::to_vec_pretty(&result).unwrap(),
                )
                .unwrap();
                println!(
                    "{} repeat {}: {} proposed, {} reviewed; {:.2}s",
                    split,
                    repeat + 1,
                    proposal.sampled,
                    proposal.skipped,
                    started.elapsed().as_secs_f64()
                );
                runs.push(result);
            }
        }
        fs::write(
            report.join("summary.json"),
            serde_json::to_vec_pretty(&json!({"runs":runs,"failures":failures})).unwrap(),
        )
        .unwrap();
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    #[test]
    #[ignore = "Read-only category preview for an explicitly selected local folder"]
    fn live_selected_category_preview() {
        let root = PathBuf::from(std::env::var("TUCK_SELECTED_ROOT").expect("Select a root"));
        let reader =
            PathBuf::from(std::env::var("TUCK_EVAL_READER").expect("Select a native reader"));
        let paths = crate::files::existing_documents(&root, true).unwrap();
        let documents = paths
            .iter()
            .map(|path| crate::collection::read(path, &root, &reader))
            .collect();
        let p = Preferences {
            root: root.to_string_lossy().into_owned(),
            ..Preferences::default()
        };
        let map = crate::collection::build(documents, &p);
        // Keep diagnostic descriptions out of the selected folder's saved cache.
        let scratch = tempfile::tempdir_in("/private/tmp").unwrap();
        let p = Preferences {
            root: scratch.path().to_string_lossy().into_owned(),
            ..p
        };
        let before: Vec<_> = paths
            .iter()
            .map(|path| crate::files::digest(path).unwrap())
            .collect();
        let result = crate::categories::suggest_map(&p, &reader, &map, |_| Ok(()));
        for (path, hash) in paths.iter().zip(before) {
            assert_eq!(crate::files::digest(path).unwrap(), hash);
        }
        let proposal = result.unwrap();
        assert!((1..=6).contains(&proposal.rules.len()));
        if std::env::var_os("TUCK_EVAL_SHOW_CATEGORY_NAMES").is_some() {
            println!(
                "Suggested topics: {}",
                proposal
                    .rules
                    .iter()
                    .map(|r| r.folder.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        println!(
            "Category preview: {} described, {} skipped, {} categories; source bytes unchanged",
            proposal.sampled,
            proposal.skipped,
            proposal.rules.len()
        );
    }

    #[test]
    #[ignore = "Read-only local diagnostics for explicitly selected documents; paths supplied by environment"]
    fn live_selected_documents() {
        let root = PathBuf::from(std::env::var("TUCK_SELECTED_ROOT").expect("Select a root"));
        let names: Vec<String> =
            serde_json::from_str(&std::env::var("TUCK_SELECTED_FILES").expect("Select filenames"))
                .unwrap();
        let reader = PathBuf::from(std::env::var("TUCK_EVAL_READER").unwrap_or_else(|_| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("resources/tuck-reader")
                .to_string_lossy()
                .into_owned()
        }));
        let mut preferences = Preferences {
            root: root.to_string_lossy().into_owned(),
            ..Preferences::default()
        };
        crate::config::load(&mut preferences).unwrap();
        for name in names {
            let path = root.join(&name);
            assert!(path
                .canonicalize()
                .unwrap()
                .starts_with(root.canonicalize().unwrap()));
            let hash = crate::files::digest(&path).unwrap();
            let text = classifier::read_document(&path, &reader).unwrap();
            let decision = classifier::classify_with_tools(
                &name,
                &text,
                &preferences,
                &[],
                Some((&path, &reader)),
            );
            assert_eq!(crate::files::digest(&path).unwrap(), hash);
            println!(
                "{}",
                json!({"file":name,"folder":decision.folder,"title":decision.title,"confident":decision.confident,"source_unchanged":true,"image_used":decision.reason.contains("Read using image and text evidence."),"folder_image_used":decision.reason.contains("Folder decision used image and text evidence.")})
            );
        }
    }

    #[test]
    #[ignore = "Requires Apple Intelligence; reports quality on synthetic fixtures without moving documents"]
    fn live_apple_accuracy_eval() {
        let fixture: Value = serde_json::from_str(
            &std::env::var_os("TUCK_EVAL_FIXTURES")
                .map(|p| fs::read_to_string(p).unwrap())
                .unwrap_or_else(|| include_str!("../../evals/documents.json").into()),
        )
        .unwrap();
        let reader = std::env::var_os("TUCK_EVAL_READER")
            .map(PathBuf::from)
            .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/tuck-reader"));
        let model = std::env::var("TUCK_EVAL_MODEL").unwrap_or_else(|_| "apple".into());
        let model = serde_json::from_value(serde_json::json!(model)).expect("Known local model");
        use crate::provider::ModelProvider;
        crate::provider::selected(&reader, model)
            .status()
            .expect("Selected model must be available");
        let temp = tempfile::tempdir_in("/private/tmp").unwrap();
        let text_source = temp.path().join("scan.txt");
        let preferences = Preferences {
            root: temp.path().to_string_lossy().into_owned(),
            ai_model: model,
            ..Preferences::default()
        };
        let mut results = Vec::new();
        for case in fixture["cases"].as_array().unwrap() {
            let text = case["text"].as_str().unwrap();
            let mut preferences = preferences.clone();
            if let Some(rules) = case.get("rules") {
                preferences.rules = serde_json::from_value(rules.clone()).unwrap();
            }
            let text_source = if let Some(folder) = case["source_folder"].as_str() {
                let directory = temp.path().join(folder);
                fs::create_dir_all(&directory).unwrap();
                directory.join("scan.txt")
            } else {
                text_source.clone()
            };
            fs::write(&text_source, text).unwrap();
            let source = if case["render"] == true || case["render_pdf"] == true {
                let image = temp.path().join(if case["render_pdf"] == true {
                    "scan.pdf"
                } else {
                    "scan.png"
                });
                assert!(std::process::Command::new("swift")
                    .arg(
                        Path::new(env!("CARGO_MANIFEST_DIR")).join("../evals/render-fixture.swift")
                    )
                    .arg(&text_source)
                    .arg(&image)
                    .status()
                    .unwrap()
                    .success());
                image
            } else {
                text_source.clone()
            };
            let started = Instant::now();
            let extracted = classifier::read_document(&source, &reader).unwrap();
            let decision = classifier::classify_with_tools(
                source.file_name().unwrap().to_str().unwrap(),
                &extracted,
                &preferences,
                &[],
                Some((&source, &reader)),
            );
            let title = decision.title.to_lowercase();
            let folder_correct = case["folder"].as_str().unwrap() == decision.folder;
            let review_expected = case["folder"] == "Needs Review";
            let forbidden = case["forbidden"]
                .as_array()
                .unwrap()
                .iter()
                .any(|word| title.contains(word.as_str().unwrap()));
            let concepts = case["concepts"].as_array().unwrap().iter().all(|group| {
                group
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|word| title.contains(word.as_str().unwrap()))
            });
            let title_correct = if review_expected {
                decision.title.is_empty() && !decision.confident
            } else if case["keep_name"] == true {
                decision.title.is_empty()
            } else {
                !title.is_empty() && concepts && !forbidden
            };
            let row = json!({"id":case["id"],"expected_folder":case["folder"],"folder_correct":folder_correct,"automated_title_check":title_correct,"forbidden_title_claim":forbidden,"review":decision.folder=="Needs Review","rendered_ocr":case["render"]==true || case["render_pdf"]==true,"image_used":decision.reason.contains("Read using image and text evidence."),"folder_image_used":decision.reason.contains("Folder decision used image and text evidence."),"extracted_characters":extracted.chars().count(),"seconds":started.elapsed().as_secs_f64(),"decision":decision});
            println!("{}", row);
            results.push(row);
        }
        let total = results.len();
        let mut times: Vec<_> = results
            .iter()
            .map(|r| r["seconds"].as_f64().unwrap())
            .collect();
        times.sort_by(f64::total_cmp);
        let median = (times[(total - 1) / 2] + times[total / 2]) / 2.0;
        let summary = json!({"cases":total,"correct_folders":results.iter().filter(|r|r["folder_correct"]==true).count(),"automated_title_checks":results.iter().filter(|r|r["automated_title_check"]==true).count(),"forbidden_title_claims":results.iter().filter(|r|r["forbidden_title_claim"]==true).count(),"review_count":results.iter().filter(|r|r["review"]==true).count(),"median_seconds":median});
        let report = json!({"scope":fixture["description"],"pipeline":std::env::var("TUCK_EVAL_LABEL").unwrap_or_else(|_|"filing-v6".into()),"os":std::process::Command::new("sw_vers").arg("-productVersion").output().map(|o|String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default(),"generated_at_unix":SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),"summary":summary,"results":results});
        let destination = std::env::var_os("TUCK_EVAL_REPORT")
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("tuck-apple-eval.json"));
        fs::write(&destination, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
        println!(
            "Evaluation summary: {}\nReport: {}",
            report["summary"],
            destination.display()
        );
        assert_eq!(results.len(), fixture["cases"].as_array().unwrap().len());
    }

    /// Same harness for the old and new pipelines: only public entry points are used.
    /// TUCK_CORPUS: output of evals/cards/make_corpus.py. TUCK_CORPUS_REPORT: new JSON file.
    #[test]
    #[ignore = "Live corpus comparison; requires Apple Intelligence; never moves files"]
    fn live_corpus_suggest_and_file() {
        let corpus = PathBuf::from(std::env::var("TUCK_CORPUS").expect("Corpus directory"));
        let report = PathBuf::from(std::env::var("TUCK_CORPUS_REPORT").expect("New report file"));
        assert!(!report.exists(), "Keep earlier runs");
        let nesting: u8 = std::env::var("TUCK_CORPUS_NESTING").map_or(2, |n| n.parse().unwrap());
        let reader = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/tuck-reader");
        let gold: Value =
            serde_json::from_slice(&fs::read(corpus.join("gold.json")).unwrap()).unwrap();
        let temp = tempfile::tempdir_in("/private/tmp").unwrap();
        let root = temp.path().canonicalize().unwrap();
        fs::create_dir(root.join(".tuck")).unwrap();
        for doc in gold["documents"].as_array().unwrap() {
            let name = doc["file"].as_str().unwrap();
            fs::copy(corpus.join("files").join(name), root.join(name)).unwrap();
        }
        let p = Preferences {
            root: root.to_string_lossy().into_owned(),
            rules: Vec::new(),
            folder_nesting: nesting,
            ..Preferences::default()
        };
        let mut paths = crate::files::existing_documents(&root, true).unwrap();
        paths.sort();
        let documents: Vec<_> = paths
            .iter()
            .map(|path| crate::collection::read(path, &root, &reader))
            .collect();
        let map = crate::collection::build(documents, &p);
        let gold_for = |path: &str| {
            let name = Path::new(path)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned();
            gold["documents"]
                .as_array()
                .unwrap()
                .iter()
                .find(|d| d["file"] == name.as_str())
                .unwrap()
                .clone()
        };

        let started = Instant::now();
        let proposal = crate::categories::suggest_map(&p, &reader, &map, |_| Ok(()));
        let suggest_seconds = started.elapsed().as_secs_f64();
        let mut ordered: Vec<_> = map.documents.iter().collect();
        ordered.sort_by(|a, b| a.path.cmp(&b.path));
        let predicted: Vec<String> = match &proposal {
            Ok(proposal) => ordered
                .iter()
                .enumerate()
                .map(|(i, _)| {
                    proposal
                        .assignments
                        .iter()
                        .find(|a| a.id == format!("d{i:04}"))
                        .map(|a| a.category.clone())
                        .unwrap_or_else(|| "Needs review".into())
                })
                .collect(),
            Err(_) => vec!["Needs review".into(); ordered.len()],
        };
        let golds: Vec<String> = ordered
            .iter()
            .map(|d| gold_for(&d.path)["folder"].as_str().unwrap().to_owned())
            .collect();
        let top = |c: &str| c.split('/').next().unwrap().to_owned();
        let (mut together, mut together_right, mut gold_pairs, mut gold_pairs_found) = (0, 0, 0, 0);
        for i in 0..ordered.len() {
            for j in i + 1..ordered.len() {
                let placed = predicted[i] != "Needs review" && predicted[j] != "Needs review";
                let same_pred = placed && top(&predicted[i]) == top(&predicted[j]);
                if golds[i] == "Needs Review" || golds[j] == "Needs Review" {
                    continue;
                }
                let same_gold = golds[i] == golds[j];
                together += same_pred as usize;
                together_right += (same_pred && same_gold) as usize;
                gold_pairs += same_gold as usize;
                gold_pairs_found += (same_pred && same_gold) as usize;
            }
        }
        let placed = predicted.iter().filter(|c| *c != "Needs review").count();

        let filing_preferences = Preferences {
            rules: serde_json::from_value(gold["rules"].clone()).unwrap(),
            rename: true,
            ..p.clone()
        };
        let mut rows = Vec::new();
        let (mut correct, mut wrong, mut review) = (0, 0, 0);
        let mut file_seconds = Vec::new();
        for (index, doc) in ordered.iter().enumerate() {
            let path = Path::new(&doc.path);
            let started = Instant::now();
            let decision = match &doc.error {
                Some(error) => crate::classifier::Decision {
                    language: None,
                    folder: "Needs Review".into(),
                    title: String::new(),
                    reason: error.clone(),
                    confident: false,
                },
                None => classifier::classify_with_tools(
                    &path.file_name().unwrap().to_string_lossy(),
                    &doc.text,
                    &filing_preferences,
                    &[],
                    Some((path, &reader)),
                ),
            };
            file_seconds.push(started.elapsed().as_secs_f64());
            let expected = &golds[index];
            if !decision.confident {
                review += 1;
            } else if &decision.folder == expected {
                correct += 1;
            } else {
                wrong += 1;
            }
            let g = gold_for(&doc.path);
            rows.push(json!({"file": g["file"], "format": g["format"], "chars": g["chars"], "gold": expected,
                "suggested": predicted[index], "filed": decision.folder, "confident": decision.confident,
                "title": decision.title, "reason": decision.reason.chars().take(300).collect::<String>()}));
        }
        file_seconds.sort_by(f64::total_cmp);
        let n = ordered.len();
        let summary = json!({
            "documents": n,
            "nesting": nesting,
            "suggest": {
                "error": proposal.as_ref().err(),
                "folders": proposal.as_ref().map(|p| p.rules.iter().map(|r| r.folder.clone()).collect::<Vec<_>>()).unwrap_or_default(),
                "placed": placed,
                "pairPrecision": together_right as f64 / together.max(1) as f64,
                "pairRecall": gold_pairs_found as f64 / gold_pairs.max(1) as f64,
                "seconds": suggest_seconds,
            },
            "filing": {"correct": correct, "wrong": wrong, "review": review, "medianSeconds": file_seconds[n / 2]},
        });
        println!("{}", serde_json::to_string_pretty(&summary).unwrap());
        fs::write(
            &report,
            serde_json::to_vec_pretty(&json!({"summary": summary, "rows": rows})).unwrap(),
        )
        .unwrap();
        for (path, _) in paths.iter().zip(0..) {
            assert!(path.exists(), "Evaluation must not move documents");
        }
    }
}
