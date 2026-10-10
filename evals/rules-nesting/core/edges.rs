use crate::model::{Preferences, Rule};
use serde_json::json;
use std::{fs, path::Path};
fn rule(folder: &str, instruction: &str) -> Rule {
    Rule { document_types: Vec::new(),
        folder: folder.into(),
        instruction: instruction.into(),
        enabled: true,
    }
}
#[test]
fn live_rule_scope_edges() {
    let out = std::path::PathBuf::from(std::env::var("TUCK_MATRIX_OUT").unwrap());
    let temp = tempfile::tempdir_in("/private/tmp").unwrap();
    let root = temp.path().canonicalize().unwrap();
    fs::create_dir(root.join(".tuck")).unwrap();
    let reader = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/tuck-reader");
    let cases=[
  ("opaque-alias","Meeting minutes\nProject: Orion\nThe team agreed the next project tasks and owners.",vec![rule("Work/Orion","Design review reports for project Orion only. Exclude meeting minutes, which belong in Work/Client One."),rule("Work/Client One","Meeting minutes for project Orion. The folder name is this user's alias for those records.")],"Work/Client One"),
  ("parent-fallback","Technical inspection report\nProject: Orion\nThe engineer completed an inspection of the project equipment.",vec![rule("Work","Technical inspection reports for any project."),rule("Work/Orion","Meeting minutes for project Orion only. Exclude inspection reports, which belong in the broad Work folder.")],"Work"),
  ("shared-name","Laboratory test report\nClinic: Lark Clinic\nThis report lists completed blood test measurements and reference intervals.",vec![rule("Health","Laboratory reports from clinics other than Harbor Clinic and Meadow Clinic."),rule("Health/Harbor Clinic","Laboratory reports issued by Harbor Clinic only."),rule("Health/Meadow Clinic","Laboratory reports issued by Meadow Clinic only.")],"Health"),
  ("mixed-types","[Document has 2 pages.]\n[Page 1]\nLaboratory test report\nClinic: Harbor Clinic\nBlood test measurements and reference intervals.\n[Page 2]\nProduct warranty certificate\nProduct: Bicycle\nFive-year coverage for defects in the frame. A separate unrelated record.",vec![rule("Health","Clinical reports."),rule("Warranties","Product warranty certificates.")],"Needs Review"),
 ];
    let mut rows = vec![];
    for (id, text, rules, expected) in cases {
        let p = Preferences {
            root: root.to_string_lossy().into_owned(),
            folder_nesting: 2,
            instructions: "Use clear English names without inventing facts.".into(),
            rules,
            ..Preferences::default()
        };
        let path = root.join(format!("{id}.txt"));
        fs::write(&path, text).unwrap();
        let before = crate::files::digest(&path).unwrap();
        let decision = crate::classifier::classify_with_tools(
            path.file_name().unwrap().to_str().unwrap(),
            text,
            &p,
            &[],
            Some((&path, &reader)),
        );
        let correct = if expected == "Needs Review" {
            !decision.confident
        } else {
            decision.confident && decision.folder == expected
        };
        assert_eq!(before, crate::files::digest(&path).unwrap());
        rows.push(json!({"id":id,"expected":expected,"correct":correct,"decision":decision,"sourceUnchanged":true}));
        fs::write(
            out.join("scope-edges.json"),
            serde_json::to_vec_pretty(&rows).unwrap(),
        )
        .unwrap();
        println!("Scope edge {id}: {correct}");
    }
}
