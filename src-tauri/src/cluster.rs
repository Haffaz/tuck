//! Average-linkage agglomerative clustering on cosine similarity (from the FolderSort prototype).

pub fn cosine(a: &[f64], b: &[f64]) -> f64 {
    if a.is_empty() || a.len() != b.len() {
        return 0.0;
    }
    let (dot, na, nb) = a
        .iter()
        .zip(b)
        .fold((0.0, 0.0, 0.0), |(d, x2, y2), (x, y)| {
            (d + x * y, x2 + x * x, y2 + y * y)
        });
    if na == 0.0 || nb == 0.0 {
        0.0
    } else {
        dot / (na.sqrt() * nb.sqrt())
    }
}

/// Groups item indices. Merging stops when the best average similarity between two
/// groups falls below `threshold`. Items without an embedding stay alone. Groups are
/// returned largest first; ties keep input order so results are deterministic.
pub fn cluster(embeddings: &[Vec<f64>], threshold: f64) -> Vec<Vec<usize>> {
    let n = embeddings.len();
    let mut sim = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in i + 1..n {
            let s = cosine(&embeddings[i], &embeddings[j]);
            sim[i][j] = s;
            sim[j][i] = s;
        }
    }
    let mut groups: Vec<Option<Vec<usize>>> = (0..n).map(|i| Some(vec![i])).collect();
    loop {
        let live: Vec<usize> = (0..n).filter(|&k| groups[k].is_some()).collect();
        let mut best = (threshold, usize::MAX, usize::MAX);
        for (x, &i) in live.iter().enumerate() {
            for &j in &live[x + 1..] {
                if sim[i][j] >= best.0 {
                    best = (sim[i][j], i, j);
                }
            }
        }
        let (_, a, b) = best;
        if a == usize::MAX {
            break;
        }
        let size = |k: usize| groups[k].as_ref().map_or(0.0, |g| g.len() as f64);
        let (size_a, size_b) = (size(a), size(b));
        // Lance-Williams update for average linkage.
        for k in 0..n {
            if k != a && k != b && groups[k].is_some() {
                let s = (size_a * sim[a][k] + size_b * sim[b][k]) / (size_a + size_b);
                sim[a][k] = s;
                sim[k][a] = s;
            }
        }
        let moved = groups[b].take().unwrap_or_default();
        if let Some(group) = groups[a].as_mut() {
            group.extend(moved);
            group.sort_unstable();
        }
    }
    let mut result: Vec<Vec<usize>> = groups.into_iter().flatten().collect();
    result.sort_by_key(|g| (std::cmp::Reverse(g.len()), g[0]));
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn similar_items_group_and_outliers_stay_apart() {
        let e = vec![
            vec![1.0, 0.0, 0.0],
            vec![0.95, 0.05, 0.0],
            vec![0.9, 0.1, 0.0],
            vec![0.0, 1.0, 0.0],
            vec![0.05, 0.98, 0.0],
            vec![0.0, 0.0, 1.0],
            vec![],
        ];
        let groups = cluster(&e, 0.8);
        assert_eq!(groups[0], vec![0, 1, 2]);
        assert_eq!(groups[1], vec![3, 4]);
        assert_eq!(groups.len(), 4);
    }

    #[test]
    fn a_strict_threshold_keeps_everything_separate() {
        let e = vec![vec![1.0, 0.0], vec![0.7, 0.7]];
        assert_eq!(cluster(&e, 0.99).len(), 2);
        assert_eq!(cluster(&e, 0.5).len(), 1);
    }

    #[test]
    fn mismatched_or_empty_vectors_are_dissimilar() {
        assert_eq!(cosine(&[1.0], &[1.0, 0.0]), 0.0);
        assert_eq!(cosine(&[0.0, 0.0], &[1.0, 0.0]), 0.0);
    }
}
