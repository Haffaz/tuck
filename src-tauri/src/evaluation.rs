//! Opt-in evaluations against Apple Intelligence. They are `#[ignore]`d, read synthetic
//! corpora (see evals/cards) or an explicitly chosen folder, and never move documents.
#[cfg(test)]
mod tests {
    use crate::{classifier, model::Preferences};
    use serde_json::{json, Value};
    use std::{
        fs,
        path::{Path, PathBuf},
        time::Instant,
    };

    #[test]
    #[ignore = "Read-only category preview for an explicitly selected local folder"]
    fn live_selected_category_preview() {
        let root = PathBuf::from(std::env::var("TUCK_SELECTED_ROOT").expect("Select a root"));
        let reader =
            PathBuf::from(std::env::var("TUCK_EVAL_READER").expect("Select a native reader"));
        let paths = crate::files::existing_documents(&root).unwrap();
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
        assert!(!proposal.rules.is_empty());
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
        let mut paths = crate::files::existing_documents(&root).unwrap();
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
