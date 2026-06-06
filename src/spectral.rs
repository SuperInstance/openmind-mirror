//! Spectral — detect the spectral isomorphism across repos.
//!
//! Compare chord shapes across multiple muscle memories to ensure
//! all repos project the same structure.

use crate::{MuscleMemory, Trit};

/// Result of spectral analysis across multiple muscle memories.
#[derive(Debug, Clone)]
pub struct SpectralReport {
    /// Mean pairwise similarity across all repos.
    pub mean_similarity: f64,
    /// Repos whose similarity drops below the threshold.
    pub outliers: Vec<String>,
    /// Pairwise similarity matrix (flattened upper triangle).
    pub pairwise: Vec<(String, String, f64)>,
}

/// Decision vector for a single muscle memory — encoding of all decisions.
struct DecisionVector {
    /// Ordered list of (function_name, decision_as_f64).
    entries: Vec<(String, f64)>,
}

fn trit_to_f64(t: Trit) -> f64 {
    match t {
        Trit::Hardcode => 1.0,
        Trit::Model => 0.0,
        Trit::Hybrid => 0.5,
    }
}

fn build_decision_vector(mm: &MuscleMemory) -> DecisionVector {
    let mut entries: Vec<(String, f64)> = mm
        .chords
        .iter()
        .map(|(name, chord)| (name.clone(), trit_to_f64(chord.decision)))
        .collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    DecisionVector { entries }
}

/// Compute cosine similarity between two decision vectors.
/// Uses the union of function names — missing functions get a default of 0.5.
fn cosine_similarity(v1: &DecisionVector, v2: &DecisionVector) -> f64 {
    // Build unified index
    let mut all_keys: std::collections::BTreeMap<&str, (f64, f64)> =
        std::collections::BTreeMap::new();

    for (name, val) in &v1.entries {
        all_keys.insert(name.as_str(), (*val, 0.5));
    }
    for (name, val) in &v2.entries {
        all_keys
            .entry(name.as_str())
            .and_modify(|e| e.1 = *val)
            .or_insert((0.5, *val));
    }

    let mut dot = 0.0_f64;
    let mut norm1 = 0.0_f64;
    let mut norm2 = 0.0_f64;

    for (_, (a, b)) in &all_keys {
        dot += a * b;
        norm1 += a * a;
        norm2 += b * b;
    }

    let denom = norm1.sqrt() * norm2.sqrt();
    if denom == 0.0 {
        // Both-zero → identical, but one-zero vs nonzero → maximally different
        if norm1 == 0.0 && norm2 == 0.0 {
            return 1.0;
        }
        return 0.0;
    }
    dot / denom
}

/// Spectral analysis across multiple muscle memory snapshots.
pub struct Spectral;

impl Spectral {
    /// Analyze multiple muscle memories for cross-repo consistency.
    ///
    /// `threshold` is the minimum acceptable similarity (default 0.95).
    /// Repos below this are flagged as outliers.
    pub fn analyze(mems: &[MuscleMemory], threshold: f64) -> SpectralReport {
        let vectors: Vec<(&MuscleMemory, DecisionVector)> =
            mems.iter().map(|mm| (mm, build_decision_vector(mm))).collect();

        let mut pairwise = Vec::new();
        let mut total_sim = 0.0;
        let mut count = 0;

        for i in 0..vectors.len() {
            for j in (i + 1)..vectors.len() {
                let sim = cosine_similarity(&vectors[i].1, &vectors[j].1);
                pairwise.push((
                    vectors[i].0.repo.clone(),
                    vectors[j].0.repo.clone(),
                    sim,
                ));
                total_sim += sim;
                count += 1;
            }
        }

        let mean_similarity = if count > 0 {
            total_sim / count as f64
        } else {
            1.0
        };

        // Find outlier repos: compute each repo's mean similarity to all others
        let mut repo_sims: std::collections::HashMap<&str, (f64, usize)> =
            std::collections::HashMap::new();
        for (repo_a, repo_b, sim) in &pairwise {
            repo_sims
                .entry(repo_a.as_str())
                .and_modify(|e| {
                    e.0 += sim;
                    e.1 += 1;
                })
                .or_insert((*sim, 1));
            repo_sims
                .entry(repo_b.as_str())
                .and_modify(|e| {
                    e.0 += sim;
                    e.1 += 1;
                })
                .or_insert((*sim, 1));
        }

        let outliers: Vec<String> = repo_sims
            .iter()
            .filter_map(|(repo, (total, n))| {
                let mean = *total / *n as f64;
                if mean < threshold {
                    Some(repo.to_string())
                } else {
                    None
                }
            })
            .collect();

        SpectralReport {
            mean_similarity,
            outliers,
            pairwise,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ChordShape, Trit};
    use std::collections::HashMap;

    fn make_chord(name: &str, decision: Trit) -> ChordShape {
        ChordShape {
            name: name.to_string(),
            decision,
            source_hash: "hash".to_string(),
            confidence: 0.95,
            invocation_count: 0,
            error_count: 0,
            avg_latency_ms: 1.0,
            calls: vec![],
            called_by: vec![],
            has_tests: true,
            safety_critical: false,
            priority: 5,
        }
    }

    fn make_mm(repo: &str, chords: Vec<ChordShape>) -> MuscleMemory {
        let map: HashMap<String, ChordShape> = chords
            .into_iter()
            .map(|c| (c.name.clone(), c))
            .collect();
        MuscleMemory {
            repo: repo.to_string(),
            version: "1.0".to_string(),
            timestamp: "now".to_string(),
            chords: map,
        }
    }

    #[test]
    fn test_identical_repos_similarity_one() {
        let mm1 = make_mm(
            "repo1",
            vec![
                make_chord("foo", Trit::Hardcode),
                make_chord("bar", Trit::Model),
            ],
        );
        let mm2 = make_mm(
            "repo2",
            vec![
                make_chord("foo", Trit::Hardcode),
                make_chord("bar", Trit::Model),
            ],
        );
        let report = Spectral::analyze(&[mm1, mm2], 0.95);
        assert!((report.mean_similarity - 1.0).abs() < 1e-9);
        assert!(report.outliers.is_empty());
    }

    #[test]
    fn test_detect_divergent_repo() {
        let mm1 = make_mm("good1", vec![
            make_chord("foo", Trit::Hardcode),
            make_chord("bar", Trit::Hardcode),
            make_chord("baz", Trit::Hardcode),
            make_chord("qux", Trit::Hardcode),
            make_chord("quux", Trit::Hardcode),
        ]);
        let mm2 = make_mm("good2", vec![
            make_chord("foo", Trit::Hardcode),
            make_chord("bar", Trit::Hardcode),
            make_chord("baz", Trit::Hardcode),
            make_chord("qux", Trit::Hardcode),
            make_chord("quux", Trit::Hardcode),
        ]);
        let mm3 = make_mm("divergent", vec![
            make_chord("foo", Trit::Model),
            make_chord("bar", Trit::Model),
            make_chord("baz", Trit::Model),
            make_chord("qux", Trit::Model),
            make_chord("quux", Trit::Model),
        ]);
        let report = Spectral::analyze(&[mm1, mm2, mm3], 0.95);
        assert!(report.outliers.contains(&"divergent".to_string()));
    }

    #[test]
    fn test_single_repo_similarity_is_one() {
        let mm = make_mm("solo", vec![make_chord("foo", Trit::Hardcode)]);
        let report = Spectral::analyze(&[mm], 0.95);
        assert!((report.mean_similarity - 1.0).abs() < 1e-9);
    }
}
