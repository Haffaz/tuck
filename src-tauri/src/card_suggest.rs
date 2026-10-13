//! Folder suggestions from document cards. Embeddings group documents of the same kind;
//! the model names those groups and, in one call, places them under broader subjects.
//! A failed model reply degrades a name or a subject, never a document's placement.
use crate::{
    cards::Card,
    categories::{generic_category_name, Placement, Proposal, ReviewFile},
    cluster, files,
    model::{Preferences, Rule},
    provider::ModelProvider,
};
use std::{collections::BTreeMap, path::Path, time::Duration};

/// Cosine threshold for average-linkage clustering of card embeddings. Measured with
/// NLEmbedding: the same document kind from different issuers scores about 0.8, while
/// different kinds score 0.55-0.73. Broader subjects are not separable this way.
const KIND_THRESHOLD: f64 = 0.75;
const MIN_GROUP: usize = 2;
const MAX_RULES: usize = 40;
/// Subfolders need three documents so small collections are not fragmented.
const MIN_CHILD: usize = 3;
const MAX_KINDS: usize = 128;
const MAX_INSTRUCTION: usize = 600;
const STEP_TIMEOUT: Duration = Duration::from_secs(60);
/// Apple's on-device model serves one request at a time, but overlapping a few cuts
/// wall time by about a quarter; more than three gained nothing in measurements.
const CARD_WORKERS: usize = 3;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Group {
    pub name: String,
    /// Documents filed directly in this folder.
    pub members: Vec<usize>,
    pub children: Vec<Group>,
}

impl Group {
    fn all_members(&self) -> Vec<usize> {
        let mut all = self.members.clone();
        for child in &self.children {
            all.extend(child.all_members());
        }
        all
    }
}

/// Clusters `items` (indices into `cards`) by kind. Returns groups of at least
/// `min_size` and the remaining single files. Singles are not attached to the nearest
/// group: at kind level that put visas with rental agreements; the subject step places them.
pub fn form_groups(
    cards: &[Card],
    items: &[usize],
    threshold: f64,
    min_size: usize,
) -> (Vec<Vec<usize>>, Vec<usize>) {
    let embeddings: Vec<Vec<f64>> = items.iter().map(|&i| cards[i].embedding.clone()).collect();
    let (groups, small): (Vec<Vec<usize>>, Vec<Vec<usize>>) =
        cluster::cluster(&embeddings, threshold)
            .into_iter()
            .map(|g| g.into_iter().map(|k| items[k]).collect::<Vec<_>>())
            .partition(|g| g.len() >= min_size);
    (groups, small.into_iter().flatten().collect())
}

fn title_case(text: &str) -> String {
    text.split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().chain(chars).collect()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn usable_name(name: &str) -> bool {
    let name = name.trim();
    !name.is_empty()
        && name.chars().count() <= 40
        && !generic_category_name(name)
        && files::valid_folder(name, 1)
}

fn most_common_kind(cards: &[Card], members: &[usize]) -> String {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for &m in members {
        *counts.entry(cards[m].kind.as_str()).or_default() += 1;
    }
    counts
        .into_iter()
        .max_by_key(|(kind, count)| (*count, std::cmp::Reverse(*kind)))
        .map(|(kind, _)| title_case(kind))
        .unwrap_or_default()
}

/// The model names the group; a usable most-common kind is the fallback.
fn name_for(
    cards: &[Card],
    members: &[usize],
    parent: &str,
    model: &dyn ModelProvider,
) -> Option<String> {
    let samples: Vec<&Card> = members.iter().take(10).map(|&m| &cards[m]).collect();
    model
        .name_group(&samples, parent, STEP_TIMEOUT)
        .ok()
        .map(|n| n.trim().to_owned())
        .filter(|n| usable_name(n) && !n.eq_ignore_ascii_case(parent))
        .or_else(|| {
            Some(most_common_kind(cards, members))
                .filter(|n| usable_name(n) && !n.eq_ignore_ascii_case(parent))
        })
}

/// Same-named groups are combined; with `model`, synonyms are paired too (Banking + Bank
/// Statements). Subjects pass no model: they come from one call and are already consistent,
/// and the model once merged Finance into Home.
fn merge_named(groups: Vec<Group>, model: Option<&dyn ModelProvider>) -> Vec<Group> {
    let mut merged: Vec<Group> = Vec::new();
    for group in groups {
        match merged
            .iter_mut()
            .find(|g| g.name.eq_ignore_ascii_case(&group.name))
        {
            Some(existing) => {
                existing.members.extend(group.members);
                existing.children.extend(group.children);
            }
            None => merged.push(group),
        }
    }
    let Some(model) = model.filter(|_| merged.len() >= 2) else {
        return merged;
    };
    let names: Vec<String> = merged.iter().map(|g| g.name.clone()).collect();
    let pairs = model.merge_names(&names, STEP_TIMEOUT).unwrap_or_default();
    let mut gone = vec![false; merged.len()];
    for (from, into) in pairs {
        let (Some(f), Some(t)) = (
            names.iter().position(|n| *n == from),
            names.iter().position(|n| *n == into),
        ) else {
            continue;
        };
        if f == t || gone[f] || gone[t] {
            continue;
        }
        let moved = std::mem::take(&mut merged[f]);
        merged[t].members.extend(moved.members);
        merged[t].children.extend(moved.children);
        gone[f] = true;
    }
    merged
        .into_iter()
        .zip(gone)
        .filter_map(|(g, gone)| (!gone).then_some(g))
        .collect()
}

/// Kind-level groups, each named. A file unlike every group becomes its own kind, so it
/// can still join a subject.
fn kind_groups(cards: &[Card], model: &dyn ModelProvider) -> (Vec<Group>, Vec<usize>) {
    let items: Vec<usize> = (0..cards.len()).collect();
    let (clusters, singles) = form_groups(cards, &items, KIND_THRESHOLD, MIN_GROUP);
    let clusters = clusters
        .into_iter()
        .chain(singles.into_iter().map(|i| vec![i]));
    let mut unplaced = Vec::new();
    let mut groups = Vec::new();
    for members in clusters {
        match name_for(cards, &members, "", model) {
            Some(name) => groups.push(Group {
                name,
                members,
                children: Vec::new(),
            }),
            None => unplaced.extend(members),
        }
    }
    (merge_named(groups, Some(model)), unplaced)
}

/// The distinct kinds and most common tags in a group, e.g. "payslip; salary, tax".
fn holds(cards: &[Card], members: &[usize]) -> String {
    let mut kinds: Vec<&str> = Vec::new();
    let mut tags: BTreeMap<&str, usize> = BTreeMap::new();
    for &m in members {
        if !kinds.contains(&cards[m].kind.as_str()) {
            kinds.push(&cards[m].kind);
        }
        for tag in &cards[m].tags {
            *tags.entry(tag).or_default() += 1;
        }
    }
    let mut tags: Vec<_> = tags.into_iter().collect();
    tags.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
    let tags: Vec<&str> = tags.into_iter().take(4).map(|(t, _)| t).collect();
    format!("{}; {}", kinds.join(", "), tags.join(", "))
}

/// Places kinds under subjects in one model call. A kind the model could not place, or
/// placed under a generic name, becomes its own subject.
fn subject_groups(cards: &[Card], kinds: Vec<Group>, model: &dyn ModelProvider) -> Vec<Group> {
    let names: Vec<String> = kinds.iter().map(|g| g.name.clone()).collect();
    let details: Vec<String> = kinds.iter().map(|g| holds(cards, &g.members)).collect();
    let placed = model
        .group_subjects(&names, &details, STEP_TIMEOUT)
        .unwrap_or_default();
    let subjects = kinds
        .into_iter()
        .enumerate()
        .map(|(index, kind)| Group {
            name: placed
                .get(index)
                .map(|s| s.trim().to_owned())
                .filter(|s| usable_name(s))
                .unwrap_or_else(|| kind.name.clone()),
            members: Vec::new(),
            children: vec![kind],
        })
        .collect();
    merge_named(subjects, None)
}

/// Keeps a kind as a subfolder only when it has enough documents and its subject keeps a
/// meaningful alternative (another subfolder, or enough documents of its own). Everything
/// else is filed directly in the subject, so small collections are not fragmented.
fn compact(subject: Group, nesting: usize) -> Group {
    let mut members = subject.members;
    let mut kept = Vec::new();
    let mut loose = Vec::new();
    for child in subject.children {
        if nesting >= 2
            && child.members.len() >= MIN_CHILD
            && !child.name.eq_ignore_ascii_case(&subject.name)
        {
            kept.push(child);
        } else {
            loose.extend(child.all_members());
        }
    }
    let alternative =
        kept.len() >= 2 || (kept.len() == 1 && loose.len() + members.len() >= MIN_CHILD);
    if !alternative {
        loose.extend(kept.drain(..).flat_map(|c| c.all_members()));
    }
    members.extend(loose);
    members.sort_unstable();
    Group {
        name: subject.name,
        members,
        children: kept,
    }
}

/// Builds the folder tree. Returns groups and the indices that got no folder: only those
/// whose name came out generic, or that exceed the rule limit.
pub fn plan(cards: &[Card], nesting: usize, model: &dyn ModelProvider) -> (Vec<Group>, Vec<usize>) {
    let (kinds, mut unplaced) = kind_groups(cards, model);
    let mut groups: Vec<Group> = subject_groups(cards, kinds, model)
        .into_iter()
        .map(|subject| compact(subject, nesting))
        .collect();
    groups.sort_by_key(|g| {
        (
            std::cmp::Reverse(g.all_members().len()),
            g.name.to_lowercase(),
        )
    });
    // Largest folders first within the settings limit.
    let mut kept = Vec::new();
    let mut rule_count = 0;
    for group in groups {
        let needed = 1 + group.children.len();
        if rule_count + needed <= MAX_RULES {
            rule_count += needed;
            kept.push(group);
        } else {
            unplaced.extend(group.all_members());
        }
    }
    unplaced.sort_unstable();
    (kept, unplaced)
}

/// A deterministic rule from the kinds seen in the group, most frequent first.
fn rule_for(folder: String, cards: &[Card], members: &[usize]) -> Rule {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for &m in members {
        *counts.entry(cards[m].kind.to_lowercase()).or_default() += 1;
    }
    let mut kinds: Vec<(String, usize)> = counts.into_iter().collect();
    kinds.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let document_types: Vec<String> = kinds.into_iter().take(MAX_KINDS).map(|(k, _)| k).collect();
    let mut instruction = String::from("Documents such as ");
    for (index, kind) in document_types.iter().enumerate() {
        let piece = if index == 0 {
            kind.clone()
        } else {
            format!(", {kind}")
        };
        if instruction.chars().count() + piece.chars().count() + 1 > MAX_INSTRUCTION {
            break;
        }
        instruction.push_str(&piece);
    }
    instruction.push('.');
    Rule {
        document_types,
        folder,
        instruction,
        enabled: true,
    }
}

pub fn rules(groups: &[Group], cards: &[Card]) -> Vec<Rule> {
    let mut rules = Vec::new();
    for group in groups {
        rules.push(rule_for(group.name.clone(), cards, &group.all_members()));
        for child in &group.children {
            rules.push(rule_for(
                format!("{}/{}", group.name, child.name),
                cards,
                &child.members,
            ));
        }
    }
    rules
}

/// Each placed document with its folder path and suggested name.
fn placements(
    groups: &[Group],
    described: &[(String, Card, usize)],
    title: impl Fn(&Card, usize) -> String,
) -> Vec<Placement> {
    let mut placed = Vec::new();
    for group in groups {
        let mut add = |members: &[usize], folder: String| {
            for &m in members {
                placed.push(Placement {
                    path: described[m].0.clone(),
                    folder: folder.clone(),
                    title: files::safe_stem(&title(&described[m].1, described[m].2)),
                });
            }
        };
        add(&group.members, group.name.clone());
        for child in &group.children {
            add(&child.members, format!("{}/{}", group.name, child.name));
        }
    }
    placed.sort_by(|a, b| a.path.cmp(&b.path));
    placed
}

#[cfg(test)]
fn assignments(
    groups: &[Group],
    described: &[(String, Card, usize)],
    total: usize,
) -> Vec<crate::categories::Assignment> {
    let mut categories = vec!["Needs review".to_string(); total];
    for group in groups {
        for &m in &group.members {
            categories[described[m].2] = group.name.clone();
        }
        for child in &group.children {
            for &m in &child.members {
                categories[described[m].2] = format!("{}/{}", group.name, child.name);
            }
        }
    }
    categories
        .into_iter()
        .enumerate()
        .map(|(i, category)| crate::categories::Assignment {
            id: format!("d{i:04}"),
            category,
        })
        .collect()
}

/// Describes every readable mapped document, then groups the cards.
pub fn suggest(
    p: &Preferences,
    map: &crate::collection::Collection,
    mut progress: impl FnMut(String) -> Result<(), String>,
    model: &dyn ModelProvider,
) -> Result<Proposal, String> {
    let _serial = crate::coordinator::serial();
    let mut ordered: Vec<_> = map.documents.iter().collect();
    ordered.sort_by(|a, b| a.path.cmp(&b.path));
    let mut review_files: Vec<ReviewFile> = ordered
        .iter()
        .filter_map(|doc| {
            doc.error.as_ref().map(|reason| ReviewFile {
                path: doc.path.clone(),
                reason: reason.clone(),
            })
        })
        .collect();
    let readable: Vec<(usize, &&crate::collection::Document)> = ordered
        .iter()
        .enumerate()
        .filter(|(_, d)| d.error.is_none())
        .collect();
    let total = readable.len();
    // A digest mismatch means the folder changed after mapping; stop instead of carding.
    if readable
        .iter()
        .any(|(_, doc)| files::digest(Path::new(&doc.path)).ok().as_ref() != Some(&doc.hash))
    {
        return Err(
            "A document changed. Map the folder again before suggesting categories.".into(),
        );
    }
    let results = crate::parallel::map(
        &readable,
        CARD_WORKERS,
        |(_, doc)| {
            crate::cards::describe(
                Path::new(&p.root),
                Path::new(&doc.path),
                &doc.text,
                &doc.clues,
                model,
                Duration::from_secs(90),
            )
        },
        |done| progress(format!("Reading document {done} of {total}")),
    )?;
    let mut described = Vec::new();
    let mut first_error = None;
    for ((position, doc), result) in readable.iter().zip(results) {
        match result {
            Ok(card) => described.push((doc.path.clone(), card, *position)),
            Err(error) => {
                first_error.get_or_insert_with(|| error.clone());
                review_files.push(ReviewFile {
                    path: doc.path.clone(),
                    reason: error,
                });
            }
        }
    }
    if described.is_empty() {
        return Err(first_error.unwrap_or_else(|| {
            "No documents could be read. Review the files or add folders yourself.".into()
        }));
    }
    progress("Grouping similar documents".into())?;
    let cards: Vec<Card> = described.iter().map(|(_, c, _)| c.clone()).collect();
    let (groups, unplaced) = plan(&cards, p.folder_nesting as usize, model);
    for &index in &unplaced {
        review_files.push(ReviewFile {
            path: described[index].0.clone(),
            reason: format!(
                "No specific folder could be suggested for this {}. Choose a folder for it.",
                cards[index].kind
            ),
        });
    }
    if groups.is_empty() {
        return Err("No specific folders could be suggested. Add folders yourself.".into());
    }
    let rules = rules(&groups, &cards);
    files::validate(&Preferences {
        rules: rules.clone(),
        ..Preferences::default()
    })?;
    progress("Preparing suggested folders".into())?;
    for doc in &map.documents {
        if doc.error.is_none()
            && files::digest(Path::new(&doc.path)).ok().as_ref() != Some(&doc.hash)
        {
            return Err("A document changed during grouping. Map the folder again.".into());
        }
    }
    let files = placements(&groups, &described, |card, position| {
        if p.rename {
            crate::card_filing::title(card, &ordered[position].text)
        } else {
            String::new()
        }
    });
    Ok(Proposal {
        files,
        sampled: described.len() - unplaced.len(),
        skipped: review_files.len(),
        review_files,
        rules,
        limited: false,
        preferences: p.clone(),
        #[cfg(test)]
        assignments: assignments(&groups, &described, ordered.len()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Names groups after their first card's kind, merges "Bankings" into "Bank Statements"
    /// and puts bank statements and payslips under Finance.
    struct Namer;
    impl ModelProvider for Namer {
        fn status(&self) -> Result<String, String> {
            Ok("ok".into())
        }
        fn identity(&self) -> Result<String, String> {
            Ok("namer".into())
        }
        fn card(&self, _: &str, _: &str, _: &[String], _: Duration) -> Result<Card, String> {
            unreachable!()
        }
        fn embed(&self, _: &[String], _: Duration) -> Result<Vec<Vec<f64>>, String> {
            unreachable!()
        }
        fn choose_folder(
            &self,
            _: &Card,
            _: &str,
            _: &[crate::provider::Candidate],
            _: &str,
            _: Duration,
        ) -> Result<Option<usize>, String> {
            unreachable!()
        }
        fn name_group(&self, cards: &[&Card], _: &str, _: Duration) -> Result<String, String> {
            match cards[0].kind.as_str() {
                "misc" => Ok("Documents".into()),
                kind => Ok(format!("{}s", title_case(kind))),
            }
        }
        fn group_subjects(
            &self,
            names: &[String],
            _: &[String],
            _: Duration,
        ) -> Result<Vec<String>, String> {
            Ok(names
                .iter()
                .map(|n| match n.as_str() {
                    "Bank Statements" | "Payslips" => "Finance".into(),
                    "Photos" => "Misc".into(),
                    _ => String::new(),
                })
                .collect())
        }
        fn merge_names(
            &self,
            names: &[String],
            _: Duration,
        ) -> Result<Vec<(String, String)>, String> {
            Ok(if names.iter().any(|n| n == "Bankings") {
                vec![("Bankings".into(), "Bank Statements".into())]
            } else {
                Vec::new()
            })
        }
    }

    fn card(kind: &str, embedding: Vec<f64>) -> Card {
        Card {
            kind: kind.into(),
            tags: vec!["t".into()],
            embedding,
            ..Card::default()
        }
    }

    #[test]
    fn kinds_join_subjects_and_unplaced_kinds_keep_their_own_name() {
        let cards = vec![
            card("bank statement", vec![1.0, 0.0, 0.0]),
            card("bank statement", vec![0.97, 0.05, 0.0]),
            card("payslip", vec![0.0, 1.0, 0.0]),
            card("payslip", vec![0.05, 0.98, 0.0]),
            card("photo", vec![0.0, 0.0, 1.0]),
        ];
        let (groups, unplaced) = plan(&cards, 1, &Namer);
        let names: Vec<_> = groups.iter().map(|g| g.name.as_str()).collect();
        // A generic subject ("Misc") is ignored and the kind name kept.
        assert_eq!(names, ["Finance", "Photos"]);
        assert!(unplaced.is_empty());
        assert!(groups[0].children.is_empty(), "one level requested");
        let rules = rules(&groups, &cards);
        assert_eq!(rules[0].document_types, vec!["bank statement", "payslip"]);
        assert_eq!(
            rules[0].instruction,
            "Documents such as bank statement, payslip."
        );
    }

    #[test]
    fn folders_beyond_the_rule_limit_need_review() {
        let cards: Vec<_> = (0..45)
            .map(|i| {
                let mut v = vec![0.0; 45];
                v[i] = 1.0;
                card(&format!("kind{i}"), v)
            })
            .collect();
        let (groups, unplaced) = plan(&cards, 1, &Namer);
        assert_eq!(groups.len(), MAX_RULES);
        assert_eq!(unplaced.len(), 5);
    }

    #[test]
    fn a_single_unlike_file_is_not_glued_to_the_nearest_group() {
        let cards = vec![
            card("rental agreement", vec![1.0, 0.0]),
            card("rental agreement", vec![0.99, 0.05]),
            card("residence permit", vec![0.8, 0.6]),
        ];
        let (groups, singles) = form_groups(&cards, &[0, 1, 2], 0.95, 2);
        assert_eq!(groups, vec![vec![0, 1]]);
        assert_eq!(singles, vec![2]);
    }

    #[test]
    fn generic_model_names_fall_back_to_the_common_kind_and_synonyms_merge() {
        let cards = vec![
            card("misc", vec![1.0, 0.0]),
            card("misc", vec![0.99, 0.02]),
            card("banking", vec![0.0, 1.0]),
            card("banking", vec![0.02, 0.99]),
            card("bank statement", vec![0.7, -0.7]),
            card("bank statement", vec![0.69, -0.72]),
        ];
        let (groups, unplaced) = plan(&cards, 1, &Namer);
        let names: Vec<_> = groups.iter().map(|g| g.name.as_str()).collect();
        // "Documents" and the fallback "Misc" are both generic, so those files need review.
        assert_eq!(names, ["Finance"]);
        assert_eq!(
            groups[0].all_members().len(),
            4,
            "Banking merged into Bank Statements"
        );
        assert_eq!(unplaced, vec![0, 1]);
    }

    #[test]
    fn two_levels_keep_only_substantial_kinds_as_subfolders() {
        let kinds = |payslips: usize, statements: usize| {
            let mut cards = Vec::new();
            for i in 0..payslips {
                cards.push(card("payslip", vec![1.0, 0.0, 0.01 * i as f64]));
            }
            for i in 0..statements {
                cards.push(card("bank statement", vec![0.0, 1.0, 0.01 * i as f64]));
            }
            cards
        };
        let cards = kinds(3, 3);
        let (groups, unplaced) = plan(&cards, 2, &Namer);
        assert!(unplaced.is_empty());
        assert_eq!(groups.len(), 1);
        let children: Vec<_> = groups[0].children.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(children, ["Payslips", "Bank Statements"]);
        assert!(groups[0].members.is_empty());
        let rules = rules(&groups, &cards);
        let folders: Vec<_> = rules.iter().map(|r| r.folder.as_str()).collect();
        assert_eq!(
            folders,
            ["Finance", "Finance/Payslips", "Finance/Bank Statements"]
        );
        // One substantial kind plus enough other documents still earns a subfolder.
        let (groups, _) = plan(&kinds(3, 3)[..], 2, &Namer);
        assert_eq!(groups[0].children.len(), 2);
        let (groups, _) = plan(&kinds(4, 2), 2, &Namer);
        assert!(
            groups[0].children.is_empty(),
            "two leftover documents are no alternative"
        );
        let (groups, _) = plan(&kinds(2, 2), 2, &Namer);
        assert!(groups[0].children.is_empty());
        assert_eq!(groups[0].members.len(), 4);
    }

    #[test]
    fn one_subfolder_is_kept_beside_enough_loose_documents() {
        let subject = Group {
            name: "Finance".into(),
            members: Vec::new(),
            children: vec![
                Group {
                    name: "Payslips".into(),
                    members: vec![0, 1, 2],
                    children: Vec::new(),
                },
                Group {
                    name: "Receipts".into(),
                    members: vec![3, 4],
                    children: Vec::new(),
                },
                Group {
                    name: "Tax".into(),
                    members: vec![5],
                    children: Vec::new(),
                },
            ],
        };
        let compacted = compact(subject.clone(), 2);
        assert_eq!(compacted.children.len(), 1);
        assert_eq!(compacted.members, vec![3, 4, 5]);
        assert!(compact(subject, 1).children.is_empty());
    }
}
