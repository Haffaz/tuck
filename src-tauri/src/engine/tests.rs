use super::*;
use crate::model::Approval;
#[test]
#[ignore = "Requires Apple Intelligence; synthetic automatic start and fallback"]
fn live_apple_automatic_flow() {
    let (_temp, engine, root) = setup();
    *engine.test_decision.lock().unwrap() = None;
    let mut p = engine.snapshot().preferences;
    p.use_ai = true;
    p.instructions = "Organize documents by their contents. Keep filenames unchanged.".into();
    p.rename = false;
    p.rules
        .retain(|r| matches!(r.folder.as_str(), "Banking & Taxes" | "Employment & Pay"));

    engine.save_preferences(p).unwrap();
    let engine = Engine::new(
        engine.directory.clone(),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/tuck-reader"),
    )
    .unwrap();
    let source = root.join("statement.txt");
    fs::write(&source, "BANK ACCOUNT STATEMENT\nExample Bank\nStatement period: January 2026\nOpening balance: SEK 1000\nDeposits: SEK 2500\nClosing balance: SEK 3500\nThis is a bank account statement showing account transactions. Synthetic test document.").unwrap();
    fs::write(root.join("unreadable.txt"), "").unwrap();
    let hash = files::digest(&source).unwrap();
    engine.map_folder().unwrap();
    assert!(source.exists());
    assert_eq!(engine.snapshot().collection.unwrap().unreadable, 1);
    engine.start_tuck(true).unwrap();
    engine.process_next().unwrap();
    engine.process_next().unwrap();
    let state = engine.snapshot();
    let filed = &state.items[0];
    assert_eq!(filed.status, "organized", "Apple: {}", filed.reason);
    assert_eq!(filed.folder, "Banking & Taxes");
    assert_eq!(files::digest(Path::new(&filed.destination)).unwrap(), hash);
    assert!(!source.exists());
    assert_eq!(state.items[1].status, "review");
    assert!(root.join("unreadable.txt").exists());
    engine.watch_tick().unwrap();
    assert!(engine.state.lock().unwrap().pending.is_empty());
    println!("Synthetic automatic flow: 1 bank statement organized with matching hash; 1 unreadable file retained; own output ignored by watcher.");
}
#[test]
fn mapping_reads_every_document_without_moving_and_start_enables_automatic_filing() {
    let (_temp, engine, root) = setup();
    for n in 0..15 {
        fs::write(
            root.join(format!("source-{n:02}.txt")),
            format!("Synthetic bank statement {n}"),
        )
        .unwrap();
    }
    engine.map_folder().unwrap();
    let state = engine.snapshot();
    let map = state.collection.unwrap();
    assert_eq!(map.documents.len(), 15);
    assert!(engine
        .state
        .lock()
        .unwrap()
        .collection
        .as_ref()
        .unwrap()
        .documents
        .iter()
        .all(|d| !d.text.is_empty()));
    assert!(state.items.is_empty());
    assert!(!state.running);
    assert!(!engine.process_next().unwrap());
    assert_eq!(engine.start_tuck(false).unwrap(), 15);
    assert!(engine.process_next().unwrap());
    let item = engine.snapshot().items[0].clone();
    assert_eq!(item.status, "organized");
    assert!(!Path::new(&item.original).exists());
    assert!(Path::new(&item.destination).exists());
    assert!(root.join("source-01.txt").exists());
    engine.stop_tuck();
    assert!(!engine.process_next().unwrap());
    assert_eq!(engine.snapshot().items[1].status, "skipped");
    assert!(root.join("source-01.txt").exists());
}
#[test]
fn mapping_includes_empty_structure_and_duplicate_edges_and_falls_back() {
    let (_temp, engine, root) = setup();
    fs::write(root.join("a.txt"), "Same contents").unwrap();
    fs::write(root.join("b.txt"), "Same contents").unwrap();
    fs::create_dir(root.join("Education")).unwrap();
    engine.map_folder().unwrap();
    let map = engine.snapshot().collection.unwrap();
    assert_eq!(map.duplicates, 1);
    assert!(map.existing_rules.iter().any(|r| r.folder == "Education"));
    assert!(map.documents[1].duplicate_of.is_some());
    engine.start_tuck(false).unwrap();
    engine.process_next().unwrap();
    engine.process_next().unwrap();
    assert_eq!(engine.snapshot().items[0].status, "organized");
    assert_eq!(engine.snapshot().items[1].status, "review");
    assert!(root.join("b.txt").exists());
}
#[test]
fn start_rejects_stale_map_and_rules_changes_stop_automatic_filing() {
    let (_temp, engine, root) = setup();
    let path = root.join("a.txt");
    fs::write(&path, "original").unwrap();
    engine.map_folder().unwrap();
    fs::write(&path, "new changed contents").unwrap();
    assert!(engine
        .start_tuck(true)
        .unwrap_err()
        .contains("changed since Tuck read the folder"));
    assert!(!engine.snapshot().running);
    engine.map_folder().unwrap();
    engine.start_tuck(true).unwrap();
    fs::write(root.join(".tuck/instructions.md"), "Changed rules").unwrap();
    engine.process_next().unwrap();
    assert!(!engine.snapshot().running);
    assert!(!engine.snapshot().watching);
    assert!(path.exists());
    assert_eq!(engine.snapshot().items[0].status, "review");
}
#[test]
fn watcher_settles_changes_ignores_own_moves_and_never_restarts_itself() {
    let (temp, engine, root) = setup();
    engine.map_folder().unwrap();
    engine.start_tuck(true).unwrap();
    let path = root.join("new.txt");
    fs::write(&path, "new bank statement").unwrap();
    engine.watch_tick().unwrap();
    assert!(engine.snapshot().items.is_empty());
    let age = |engine: &Engine| {
        for (_, since) in engine.state.lock().unwrap().pending.values_mut() {
            *since = Instant::now() - Duration::from_secs(7);
        }
    };
    age(&engine);
    fs::write(&path, "still being copied: more bytes").unwrap();
    engine.watch_tick().unwrap();
    assert!(engine.snapshot().items.is_empty());
    age(&engine);
    engine.watch_tick().unwrap();
    assert_eq!(engine.snapshot().items.len(), 1);
    engine.process_next().unwrap();
    let item = engine.snapshot().items[0].clone();
    assert_eq!(item.status, "organized");
    engine.watch_tick().unwrap();
    age(&engine);
    engine.watch_tick().unwrap();
    assert_eq!(engine.snapshot().items.len(), 1);
    fs::write(&item.destination, "Finder edited contents").unwrap();
    engine.watch_tick().unwrap();
    age(&engine);
    engine.watch_tick().unwrap();
    assert_eq!(
        engine
            .snapshot()
            .items
            .iter()
            .filter(|i| i.status == "queued")
            .count(),
        1
    );
    let restarted = Engine::new(temp.path().join("app"), engine.reader.clone()).unwrap();
    assert!(!restarted.snapshot().running);
    assert!(!restarted.snapshot().watching);
    assert!(restarted.snapshot().collection.is_none());
}
#[test]
fn automatic_uncertainty_and_changed_sources_stay_in_place() {
    let (_temp, engine, root) = setup();
    *engine.test_decision.lock().unwrap() = None;
    fs::write(root.join("unknown.txt"), "Unclear").unwrap();
    engine.map_folder().unwrap();
    engine.start_tuck(false).unwrap();
    engine.process_next().unwrap();
    assert_eq!(engine.snapshot().items[0].status, "review");
    assert!(root.join("unknown.txt").exists());
    assert!(engine.snapshot().items[0].destination.is_empty());
}
#[test]
fn a_session_over_the_document_limit_enqueues_nothing() {
    let (_temp, engine, root) = category_setup(false);
    for n in 0..2001 {
        fs::write(root.join(format!("{n}.txt")), "Synthetic").unwrap();
    }
    // Scanning refuses first; the session limit also holds for files added directly.
    assert!(files::existing_documents(&root)
        .unwrap_err()
        .contains("2,000"));
    let paths = (0..2001).map(|n| root.join(format!("{n}.txt"))).collect();
    assert!(engine.enqueue(paths).unwrap_err().contains("2,000"));
    assert!(engine.snapshot().items.is_empty());
    assert!(engine.snapshot().processing.is_none());
    assert_eq!(fs::read_to_string(root.join("0.txt")).unwrap(), "Synthetic");
}
fn category_setup(live: bool) -> (tempfile::TempDir, Engine, PathBuf) {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::tempdir_in("/private/tmp").unwrap();
    let root = temp.path().join("documents");
    fs::create_dir(&root).unwrap();
    let reader = if live {
        std::env::var_os("TUCK_EVAL_READER")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/tuck-reader")
            })
    } else {
        let helper = temp.path().join("tuck-apple");
        fs::write(&helper, r#"#!/usr/bin/python3
import json, sys
request = json.load(sys.stdin)
from pathlib import Path
with Path(__file__).with_name('requests.jsonl').open('a') as log:
    log.write(json.dumps(request) + '\n')
if request.get('operation') == 'card':
    text = request['text']
    kind = 'rental agreement' if 'rental' in text else 'bank statement'
    print(json.dumps({'kind':kind, 'issuer':'', 'date':'', 'tags':['synthetic'], 'embedding':[0.0, 1.0] if 'rental' in text else [1.0, 0.0]}))
elif request.get('operation') == 'name_group':
    print(json.dumps({'name':'Home' if any('rental' in c['kind'] for c in request['cards']) else 'Banking & Taxes'}))
elif request.get('operation') == 'merge_names':
    print(json.dumps({'merges':[]}))
elif request.get('operation') == 'group_subjects':
    print(json.dumps({'subjects':{}}))
elif request.get('operation') == 'embed':
    print(json.dumps({'embeddings':[[1.0, 0.0] for _ in request['texts']]}))
elif request.get('operation') == 'choose_folder':
    print(json.dumps({'folder':request['candidates'][0]['folder']}))
elif request.get('operation') == 'rule':
    print(json.dumps({'instruction':'Keep immigration applications and residence permits here.'}))
else:
    print(json.dumps({'rules':[{'folder':'Banking & Taxes','instruction':'Keep bank statements and financial records here.'}, {'folder':'Home','instruction':'Keep rental agreements and household documents here.'}]}))
"#).unwrap();
        fs::set_permissions(&helper, fs::Permissions::from_mode(0o700)).unwrap();
        temp.path().join("tuck-reader")
    };
    let engine = Engine::new(temp.path().join("app"), reader).unwrap();
    engine.choose_folder(root.to_str().unwrap()).unwrap();
    (temp, engine, root)
}
fn category_documents(root: &Path) -> Vec<(PathBuf, String)> {
    [
            ("bank.txt", "Bank account statement. Monthly account transactions and closing balance. This is a synthetic financial document."),
            ("rent.txt", "Residential rental agreement. The tenant rents an apartment from the landlord. This is a synthetic housing document."),
            ("work.txt", "Employment contract. This agreement describes the job title, working hours and salary. This is a synthetic employment document."),
        ].into_iter().map(|(name, text)| {
            let path = root.join(name);
            fs::write(&path, text).unwrap();
            let hash = files::digest(&path).unwrap();
            (path, hash)
        }).collect()
}
#[test]
fn saved_example_survives_restart_and_requires_fresh_analysis() {
    let (_temp, engine, root) = category_setup(false);
    let mut p = engine.snapshot().preferences;
    p.rules = Preferences::default().rules;
    p.use_ai = false;
    engine.save_preferences(p).unwrap();
    let document = root.join("example.txt");
    fs::write(&document, "Synthetic document").unwrap();
    engine.enqueue(vec![document.clone()]).unwrap();
    engine.process_next().unwrap();
    let item = engine.snapshot().items[0].clone();
    engine
        .update_review(&item.id, "Banking & Taxes", "Bank Statement")
        .unwrap();
    let item = engine.snapshot().items[0].clone();
    let mut p = engine.snapshot().preferences;
    p.rules.iter_mut().find(|r| r.folder == "Banking & Taxes").unwrap().instruction.push_str("\n\n## Filing example\nBelongs here: An unrelated monthly bank statement.\nDoes not belong here: An attachment in a Cedar application.");
    engine.save_preferences(p).unwrap();
    assert!(engine
        .organize(vec![Approval {
            id: item.id,
            destination: item.destination
        }])
        .unwrap_err()
        .contains("Settings changed"));
    assert!(document.exists());
    let restarted = Engine::new(engine.directory.clone(), engine.reader.clone()).unwrap();
    assert!(restarted
        .snapshot()
        .preferences
        .rules
        .iter()
        .find(|r| r.folder == "Banking & Taxes")
        .unwrap()
        .instruction
        .contains("Does not belong here"));
    assert!(restarted.snapshot().items.is_empty());
}
#[test]
fn category_suggestions_include_every_readable_mapped_document() {
    let (temp, engine, root) = category_setup(false);
    for n in 0..33 {
        fs::write(
            root.join(format!("document-{n:02}.txt")),
            format!("Synthetic document {n} with enough English text for category suggestions."),
        )
        .unwrap();
    }
    fs::write(root.join("unreadable.txt"), "").unwrap();
    engine.map_folder().unwrap();
    let proposal = engine.suggest_categories().unwrap();
    assert_eq!(proposal.sampled, 33);
    assert_eq!(proposal.skipped, 1);
    let requests = fs::read_to_string(temp.path().join("requests.jsonl")).unwrap();
    let mut sent = std::collections::BTreeSet::new();
    for line in requests.lines() {
        let request: serde_json::Value = serde_json::from_str(line).unwrap();
        if request["operation"] == "card" {
            sent.insert(request["text"].as_str().unwrap().to_owned());
        }
    }
    assert_eq!(sent.len(), 33, "every readable document is described once");
    // Similar documents share one folder instead of one folder per batch.
    assert_eq!(proposal.rules.len(), 1);
    assert_eq!(proposal.rules[0].folder, "Banking & Taxes");
    assert!(engine.snapshot().items.is_empty());
    assert_eq!(files::existing_documents(&root).unwrap().len(), 34);
}
#[test]
fn cards_are_reused_across_suggestions_rules_renames_and_filing() {
    let (temp, engine, root) = category_setup(false);
    let documents = category_documents(&root);
    let count = || {
        fs::read_to_string(temp.path().join("requests.jsonl"))
            .unwrap()
            .lines()
            .filter(|line| {
                serde_json::from_str::<serde_json::Value>(line).unwrap()["operation"] == "card"
            })
            .count()
    };
    let proposal = engine.suggest_categories().unwrap();
    assert_eq!(count(), 3);
    let mut p = proposal.preferences;
    p.rules = proposal.rules;
    engine.save_preferences(p).unwrap();
    engine.suggest_categories().unwrap();
    assert_eq!(count(), 3);
    let renamed = root.join("renamed.txt");
    fs::rename(&documents[0].0, &renamed).unwrap();
    engine.suggest_categories().unwrap();
    assert_eq!(count(), 3);
    let text = fs::read_to_string(&renamed).unwrap();
    let decision = crate::filing::decide(
        &text,
        &[],
        &engine.snapshot().preferences,
        Some((&renamed, &engine.reader)),
    );
    assert!(decision.confident, "{}", decision.reason);
    assert_eq!(count(), 3);
    fs::write(
        &renamed,
        "A different readable document about a bank account and its transactions.",
    )
    .unwrap();
    engine.suggest_categories().unwrap();
    assert_eq!(count(), 4);
    assert!(renamed.exists()); // Suggestions and classification themselves do not move files.
}
#[test]
fn approved_suggestions_move_files_in_one_step_and_keep_the_rest_for_review() {
    let (_temp, engine, root) = category_setup(false);
    let documents = category_documents(&root);
    fs::write(root.join("blank.txt"), "").unwrap();
    let proposal = engine.suggest_categories().unwrap();
    assert_eq!(proposal.files.len(), 3);
    assert!(proposal
        .files
        .iter()
        .all(|f| f.title == "Bank Statement" || f.title == "Rental Agreement"));
    let mut approved = proposal.preferences.clone();
    approved.rules = proposal.rules.clone();
    engine.save_preferences(approved).unwrap();
    engine.map_folder().unwrap();
    assert_eq!(
        engine
            .organize_proposal(proposal.files.clone(), false)
            .unwrap(),
        3
    );
    let state = engine.snapshot();
    assert!(state.running && !state.watching);
    for (path, hash) in &documents {
        assert!(!path.exists(), "{} was not moved", path.display());
        let item = state
            .items
            .iter()
            .find(|i| Path::new(&i.original) == path)
            .unwrap();
        assert_eq!(item.status, "organized");
        assert!(item.destination.starts_with(root.to_str().unwrap()));
        assert_eq!(&files::digest(Path::new(&item.destination)).unwrap(), hash);
    }
    let blank = state
        .items
        .iter()
        .find(|i| i.original.ends_with("blank.txt"))
        .unwrap();
    assert_eq!((blank.status.as_str(), blank.confident), ("review", false));
    assert!(root.join("blank.txt").exists());
    assert!(engine
        .organize_proposal(proposal.files, false)
        .unwrap_err()
        .contains("Finish or clear"));
}
#[test]
fn forged_placements_move_nothing_outside_the_plan() {
    let (_temp, engine, root) = category_setup(false);
    let documents = category_documents(&root);
    let mut p = engine.snapshot().preferences;
    p.rules = vec![crate::model::Rule {
        document_types: Vec::new(),
        folder: "Home".into(),
        instruction: "Housing.".into(),
        enabled: true,
    }];
    engine.save_preferences(p).unwrap();
    engine.map_folder().unwrap();
    let outside = root.parent().unwrap().join("outside.txt");
    fs::write(&outside, "Not part of the folder").unwrap();
    let forged = |path: &Path, folder: &str, title: &str| crate::suggest::Placement {
        path: path.to_string_lossy().into_owned(),
        folder: folder.into(),
        title: title.into(),
    };
    let moved = engine
        .organize_proposal(
            vec![
                forged(&outside, "Home", "Outside"),
                forged(&documents[0].0, "Nowhere", "Bank"),
                forged(&documents[1].0, "../escape", "Rent"),
                forged(&documents[2].0, "Home", "../../x"),
            ],
            false,
        )
        .unwrap();
    assert_eq!(moved, 1, "only the in-plan file with a real folder moves");
    assert!(outside.exists());
    assert!(documents[0].0.exists() && documents[1].0.exists());
    let state = engine.snapshot();
    let renamed = state
        .items
        .iter()
        .find(|i| Path::new(&i.original) == documents[2].0)
        .unwrap();
    assert!(renamed
        .destination
        .starts_with(root.join("Home").to_str().unwrap()));
    assert!(!renamed.destination.contains(".."));
    assert!(state
        .items
        .iter()
        .filter(|i| Path::new(&i.original) != documents[2].0)
        .all(|i| i.status == "review" && !i.confident));
}
#[test]
fn category_preview_requires_approval_and_preserves_existing_roots() {
    let (_temp, engine, root) = category_setup(false);
    assert!(engine.snapshot().preferences.rules.is_empty());
    let documents = category_documents(&root);
    let proposal = engine.suggest_categories().unwrap();
    assert_eq!(proposal.sampled, 3);
    assert!(!root.join("Banking & Taxes").exists());
    assert!(!root.join("Home").exists());
    assert!(engine.snapshot().processing.is_none());
    let mut approved = proposal.preferences;
    approved.rules = proposal.rules;
    engine.save_preferences(approved).unwrap();
    assert!(root.join("Banking & Taxes/.tuck/instructions.md").is_file());
    assert!(root.join("Home/.tuck/instructions.md").is_file());
    for (path, hash) in documents {
        assert_eq!(files::digest(&path).unwrap(), hash);
    }
    engine.choose_folder(root.to_str().unwrap()).unwrap();
    assert_eq!(engine.snapshot().preferences.rules.len(), 2);
    assert!(engine.suggest_categories().is_ok());
}
#[test]
fn category_failures_clear_processing_and_stale_proposals_do_not_write() {
    let (_temp, engine, root) = category_setup(false);
    assert!(engine.suggest_categories().is_err());
    assert!(engine.snapshot().processing.is_none());
    category_documents(&root);
    let proposal = engine.suggest_categories().unwrap();
    fs::write(
        root.join(".tuck/instructions.md"),
        "Keep filenames unchanged.",
    )
    .unwrap();
    let mut approved = proposal.preferences;
    approved.rules = proposal.rules;
    assert!(engine.save_preferences(approved).is_err());
    assert!(!root.join("Banking & Taxes").exists());
    assert_eq!(
        fs::read_to_string(root.join(".tuck/instructions.md")).unwrap(),
        "Keep filenames unchanged."
    );
}
#[test]
#[ignore = "Requires Apple Intelligence; synthetic category discovery and folder creation"]
fn live_apple_category_suggestions() {
    let (_temp, engine, root) = category_setup(true);
    let documents = category_documents(&root);
    let proposal = engine.suggest_categories().unwrap();
    assert_eq!(proposal.sampled, 3);
    assert!((1..=6).contains(&proposal.rules.len()));
    println!(
        "Suggested synthetic categories: {:?}",
        proposal.rules.iter().map(|r| &r.folder).collect::<Vec<_>>()
    );
    for rule in &proposal.rules {
        assert!(!root.join(&rule.folder).exists());
    }
    let mut approved = proposal.preferences;
    approved.rules = proposal.rules;
    engine.save_preferences(approved).unwrap();
    for rule in engine.snapshot().preferences.rules {
        assert!(root
            .join(rule.folder)
            .join(".tuck/instructions.md")
            .is_file());
    }
    for (path, hash) in documents {
        assert_eq!(files::digest(&path).unwrap(), hash);
    }
}
fn setup() -> (tempfile::TempDir, Engine, PathBuf) {
    let temp = tempfile::tempdir_in("/private/tmp").unwrap();
    let root = temp.path().join("documents");
    fs::create_dir(&root).unwrap();
    let engine = Engine::new(temp.path().join("app"), PathBuf::new()).unwrap();
    engine.choose_folder(root.to_str().unwrap()).unwrap();
    let mut p = engine.snapshot().preferences;
    p.use_ai = false;
    p.rules = Preferences::default().rules;
    engine.save_preferences(p).unwrap();
    *engine.test_decision.lock().unwrap() = Some(crate::filing::Decision {
        folder: "Banking & Taxes".into(),
        title: "Bank statement".into(),
        confident: true,
        reason: "Synthetic bank statement".into(),
    });
    (temp, engine, root)
}
fn analyze(engine: &Engine, path: &Path) -> ReviewItem {
    engine.enqueue(vec![path.to_path_buf()]).unwrap();
    engine.process_next().unwrap();
    engine.snapshot().items.last().unwrap().clone()
}
fn approval(item: &ReviewItem) -> Approval {
    Approval {
        id: item.id.clone(),
        destination: item.destination.clone(),
    }
}
#[test]
fn review_never_moves_and_approval_moves_only_selected() {
    let (_temp, engine, root) = setup();
    let a = root.join("a.txt");
    let b = root.join("b.txt");
    fs::write(&a, "bank statement a").unwrap();
    fs::write(&b, "bank statement b").unwrap();
    let first = analyze(&engine, &a);
    let second = analyze(&engine, &b);
    assert_eq!(first.status, "review");
    assert!(a.exists() && b.exists());
    assert!(!Path::new(&first.destination).exists());
    engine.organize(vec![approval(&first)]).unwrap();
    assert!(!a.exists());
    assert!(b.exists());
    assert!(Path::new(&first.destination).exists());
    assert_eq!(engine.snapshot().items[1].status, "review");
    // Both initially suggested the same name; the second now requires a new reviewed preview.
    assert!(engine
        .organize(vec![approval(&second)])
        .unwrap_err()
        .contains("occupied"));
    let second = engine.snapshot().items[1].clone();
    assert!(second.destination.contains("(1)"));
    engine.organize(vec![approval(&second)]).unwrap();
    assert_eq!(
        fs::read_to_string(&first.destination).unwrap(),
        "bank statement a"
    );
    assert_eq!(
        fs::read_to_string(&second.destination).unwrap(),
        "bank statement b"
    );
}
#[test]
fn preview_uses_session_identity_and_follows_organized_destination() {
    let (temp, engine, _root) = setup();
    let path = temp.path().join("preview.txt");
    fs::write(&path, "bank statement").unwrap();
    let item = analyze(&engine, &path);
    assert_eq!(engine.preview_path(&item.id).unwrap(), path);
    assert!(engine.preview_path("unknown").is_err());
    engine.organize(vec![approval(&item)]).unwrap();
    assert_eq!(
        engine.preview_path(&item.id).unwrap(),
        PathBuf::from(&item.destination)
    );
    fs::remove_file(&item.destination).unwrap();
    assert!(engine.preview_path(&item.id).is_err());
}
#[test]
fn external_files_are_copied_and_session_is_not_persisted() {
    let (temp, engine, _root) = setup();
    let path = temp.path().join("external.txt");
    fs::write(&path, "statement").unwrap();
    let item = analyze(&engine, &path);
    assert!(item.copied);
    engine.organize(vec![approval(&item)]).unwrap();
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        fs::read_to_string(&item.destination).unwrap()
    );
    let mut app_files: Vec<_> = fs::read_dir(temp.path().join("app"))
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    app_files.sort();
    let mut expected = vec![std::ffi::OsString::from("preferences.json")];
    if cfg!(target_os = "macos") {
        expected.insert(0, "folder-access.json".into());
    }
    assert_eq!(app_files, expected);
    let restarted = Engine::new(temp.path().join("app"), PathBuf::new()).unwrap();
    assert!(restarted.snapshot().items.is_empty());
    assert!(restarted.snapshot().processing.is_none());
}
#[test]
fn uncertain_documents_stay_put_until_user_chooses_folder() {
    let (_temp, engine, root) = setup();
    *engine.test_decision.lock().unwrap() = None;
    let path = root.join("scan.txt");
    fs::write(&path, "unclear").unwrap();
    let item = analyze(&engine, &path);
    assert_eq!(item.status, "review");
    assert!(item.destination.is_empty());
    assert!(engine.organize(vec![approval(&item)]).is_err());
    assert!(path.exists());
    assert!(!root.join("Needs Review").exists());
    engine
        .update_review(&item.id, "Banking & Taxes", "My statement")
        .unwrap();
    let edited = engine.snapshot().items[0].clone();
    engine.organize(vec![approval(&edited)]).unwrap();
    assert!(root.join("Banking & Taxes/My statement.txt").exists());
}
#[test]
fn edits_and_disabled_destinations_are_checked_before_moving() {
    let (_temp, engine, root) = setup();
    let path = root.join("source.txt");
    fs::write(&path, "original").unwrap();
    let item = analyze(&engine, &path);
    assert!(engine
        .update_review(&item.id, "../outside", "name")
        .is_err());
    assert!(engine
        .update_review(&item.id, "Banking & Taxes", "../name")
        .is_err());
    fs::write(&path, "user edits").unwrap();
    assert!(engine
        .organize(vec![approval(&item)])
        .unwrap_err()
        .contains("changed"));
    assert_eq!(fs::read_to_string(&path).unwrap(), "user edits");
    assert!(!Path::new(&item.destination).exists());
    engine.clear();
    let item = analyze(&engine, &path);
    let mut p = engine.snapshot().preferences;
    p.rules
        .iter_mut()
        .find(|r| r.folder == "Banking & Taxes")
        .unwrap()
        .enabled = false;
    engine.save_preferences(p).unwrap();
    assert!(engine
        .organize(vec![approval(&item)])
        .unwrap_err()
        .contains("Settings changed"));
    assert!(path.exists());
}
#[test]
fn switching_models_stops_filing_and_rejects_old_decisions() {
    let (_temp, engine, root) = setup();
    let path = root.join("source.txt");
    fs::write(&path, "statement").unwrap();
    let item = analyze(&engine, &path);
    {
        let mut state = engine.state.lock().unwrap();
        state.running = true;
        state.watching = true;
    }
    let mut p = engine.snapshot().preferences;
    p.ai_model = crate::model::LocalModel::Custom;
    p.connection.model = "local-model".into();
    engine.save_preferences(p).unwrap();
    assert!(!engine.snapshot().running);
    assert!(!engine.snapshot().watching);
    assert!(engine
        .organize(vec![approval(&item)])
        .unwrap_err()
        .contains("Settings changed"));
    assert!(path.exists());
}

#[test]
fn manual_settings_edits_reload_and_invalidate_review_and_stale_saves() {
    let (_temp, engine, root) = setup();
    let path = root.join("source.txt");
    fs::write(&path, "statement").unwrap();
    let item = analyze(&engine, &path);
    let old = engine.snapshot().preferences;
    let settings = root.join(".tuck/settings.json");
    let mut data: serde_json::Value =
        serde_json::from_slice(&fs::read(&settings).unwrap()).unwrap();
    data["rename"] = false.into();
    fs::write(&settings, serde_json::to_vec(&data).unwrap()).unwrap();
    assert!(!engine.snapshot().preferences.rename);
    assert!(engine
        .organize(vec![approval(&item)])
        .unwrap_err()
        .contains("Settings changed"));
    assert!(engine
        .save_preferences(old)
        .unwrap_err()
        .contains("Settings changed"));
    fs::write(&settings, "bad json").unwrap();
    assert!(engine.snapshot().error.is_some());
    assert!(engine.organize(vec![approval(&item)]).is_err());
    assert_eq!(fs::read_to_string(&settings).unwrap(), "bad json");
    assert!(path.exists());
}
#[test]
fn manual_prompt_edits_require_new_analysis() {
    let (_temp, engine, root) = setup();
    let path = root.join("source.txt");
    fs::write(&path, "statement").unwrap();
    let item = analyze(&engine, &path);
    fs::write(root.join(".tuck/instructions.md"), "Keep original names.").unwrap();
    assert!(engine
        .organize(vec![approval(&item)])
        .unwrap_err()
        .contains("Settings changed"));
    assert!(path.exists());
}
#[test]
fn skip_and_clear_leave_files_and_settings_intact() {
    let (_temp, engine, root) = setup();
    let path = root.join("source.txt");
    fs::write(&path, "statement").unwrap();
    engine.enqueue(vec![path.clone(), path.clone()]).unwrap();
    assert_eq!(engine.snapshot().items.len(), 1);
    let item = engine.snapshot().items[0].clone();
    engine.skip(&item.id).unwrap();
    assert!(!engine.process_next().unwrap());
    let settings = fs::read(root.join(".tuck/settings.json")).unwrap();
    engine.clear();
    assert!(engine.snapshot().items.is_empty());
    assert!(path.exists());
    assert_eq!(
        fs::read(root.join(".tuck/settings.json")).unwrap(),
        settings
    );
    assert!(engine
        .enqueue(vec![root.join(".tuck/instructions.md")])
        .is_err());
}
#[test]
fn legacy_watch_queue_and_database_are_never_resumed_or_rewritten() {
    let temp = tempfile::tempdir_in("/private/tmp").unwrap();
    let root = temp.path().join("documents");
    fs::create_dir(&root).unwrap();
    let app = temp.path().join("app");
    fs::create_dir(&app).unwrap();
    let source = root.join("old.txt");
    fs::write(&source, "original").unwrap();
    let old = serde_json::to_vec(&serde_json::json!({"preferences": {"root": root, "rename": false}, "watching": true, "queue": [source], "activity": [{"status":"pending"}]})).unwrap();
    fs::write(app.join("state.json"), &old).unwrap();
    fs::write(app.join("tuck.db"), "existing database bytes").unwrap();
    let engine = Engine::new(app.clone(), PathBuf::new()).unwrap();
    assert!(engine.snapshot().items.is_empty());
    assert!(!engine.snapshot().preferences.rename);
    assert!(!engine.process_next().unwrap());
    assert_eq!(fs::read(app.join("state.json")).unwrap(), old);
    assert_eq!(
        fs::read_to_string(app.join("tuck.db")).unwrap(),
        "existing database bytes"
    );
    assert!(source.exists());
}
#[test]
fn symlink_destinations_and_settings_are_rejected() {
    use std::os::unix::fs::symlink;
    let (temp, engine, root) = setup();
    let path = root.join("source.txt");
    fs::write(&path, "statement").unwrap();
    let item = analyze(&engine, &path);
    // Replacement with a link invalidates instruction loading before any transfer.
    fs::rename(
        root.join("Banking & Taxes"),
        root.join("OldBanking & Taxes"),
    )
    .unwrap();
    symlink(temp.path(), root.join("Banking & Taxes")).unwrap();
    assert!(engine.organize(vec![approval(&item)]).is_err());
    assert!(path.exists());
    fs::remove_file(root.join(".tuck/settings.json")).unwrap();
    let outside = temp.path().join("outside.json");
    fs::write(&outside, "{}").unwrap();
    symlink(&outside, root.join(".tuck/settings.json")).unwrap();
    assert!(engine.snapshot().error.is_some());
    assert_eq!(fs::read_to_string(outside).unwrap(), "{}");
}
#[test]
fn restart_drops_unapproved_batch_and_preserves_settings() {
    let (temp, engine, root) = setup();
    let path = root.join("source.txt");
    fs::write(&path, "statement").unwrap();
    analyze(&engine, &path);
    let restarted = Engine::new(temp.path().join("app"), PathBuf::new()).unwrap();
    assert!(restarted.snapshot().items.is_empty());
    assert!(!restarted.snapshot().preferences.use_ai);
    assert!(path.exists());
}
#[test]
#[ignore = "Requires Apple Intelligence; synthetic end-to-end review and approval"]
fn live_apple_review_flow() {
    let (_temp, engine, root) = setup();
    *engine.test_decision.lock().unwrap() = None;
    let mut p = engine.snapshot().preferences;
    p.use_ai = true;
    engine.save_preferences(p).unwrap();
    let engine = Engine::new(
        engine.directory.clone(),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/tuck-reader"),
    )
    .unwrap();
    let source = root.join("statement.txt");
    fs::write(&source, "BANK ACCOUNT STATEMENT\nExample Bank\nStatement period: January 2026\nOpening balance: SEK 1000\nDeposits: SEK 2500\nClosing balance: SEK 3500\nThis is a bank account statement showing account transactions. Synthetic test document.").unwrap();
    let item = analyze(&engine, &source);
    assert_eq!(item.status, "review");
    assert!(source.exists());
    assert_eq!(
        item.folder, "Banking & Taxes",
        "Apple result: {}",
        item.reason
    );
    assert!(!Path::new(&item.destination).exists());
    engine.organize(vec![approval(&item)]).unwrap();
    assert!(!source.exists());
    assert!(Path::new(&item.destination).is_file());
    assert_eq!(
        files::digest(Path::new(&item.destination)).unwrap(),
        item.hash
    );
}
