//! Coherence — check if muscle memory is internally consistent.

use crate::{Issue, MuscleMemory, Severity};

/// Coherence checker for muscle memory.
pub struct Coherence;

impl Coherence {
    /// Check a muscle memory snapshot for internal consistency issues.
    pub fn check(mm: &MuscleMemory) -> Vec<Issue> {
        let mut issues = Vec::new();

        let all_names: std::collections::HashSet<&str> =
            mm.chords.keys().map(|s| s.as_str()).collect();

        for (name, chord) in &mm.chords {
            // 1. Orphan chords: calls nothing and nothing calls them
            if chord.calls.is_empty() && chord.called_by.is_empty() {
                issues.push(Issue {
                    severity: Severity::Info,
                    chord: name.clone(),
                    description: "Orphan chord: calls nothing and nothing calls it".to_string(),
                });
            }

            // 2. Broken chains: calls a function not in memory
            for called in &chord.calls {
                if !all_names.contains(called.as_str()) {
                    issues.push(Issue {
                        severity: Severity::Critical,
                        chord: name.clone(),
                        description: format!(
                            "Broken chain: calls '{}' which is not in muscle memory",
                            called
                        ),
                    });
                }
            }

            // 3. Decision contradictions: safety-critical function not HARDCODE
            if chord.safety_critical && chord.decision != crate::Trit::Hardcode {
                issues.push(Issue {
                    severity: Severity::Critical,
                    chord: name.clone(),
                    description: format!(
                        "Safety-critical function has decision {:?} (should be HARDCODE)",
                        chord.decision
                    ),
                });
            }

            // 4. Test gaps: high-connectivity function (5+ callers or callees) with no tests
            let connectivity = chord.calls.len() + chord.called_by.len();
            if connectivity >= 5 && !chord.has_tests {
                issues.push(Issue {
                    severity: Severity::Warning,
                    chord: name.clone(),
                    description: format!(
                        "High-connectivity function ({} edges) has no tests",
                        connectivity
                    ),
                });
            }

            // 5. Priority inversions: called by many but ranked low
            if chord.called_by.len() >= 5 && chord.priority < 3 {
                issues.push(Issue {
                    severity: Severity::Warning,
                    chord: name.clone(),
                    description: format!(
                        "Priority inversion: called by {} functions but priority is {}",
                        chord.called_by.len(),
                        chord.priority
                    ),
                });
            }
        }

        issues
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

    fn make_mm(chords: Vec<ChordShape>) -> MuscleMemory {
        let map: HashMap<String, ChordShape> = chords
            .into_iter()
            .map(|c| (c.name.clone(), c))
            .collect();
        MuscleMemory {
            repo: "test".to_string(),
            version: "1.0".to_string(),
            timestamp: "now".to_string(),
            chords: map,
        }
    }

    #[test]
    fn test_find_orphans() {
        let mm = make_mm(vec![make_chord("lonely", Trit::Hardcode)]);
        let issues = Coherence::check(&mm);
        assert!(issues.iter().any(|i| i.chord == "lonely" && matches!(i.severity, Severity::Info)));
    }

    #[test]
    fn test_broken_chain() {
        let mut chord = make_chord("caller", Trit::Hardcode);
        chord.calls = vec!["nonexistent".to_string()];
        let mm = make_mm(vec![chord]);
        let issues = Coherence::check(&mm);
        assert!(issues
            .iter()
            .any(|i| i.chord == "caller" && matches!(i.severity, Severity::Critical)));
    }

    #[test]
    fn test_safety_critical_not_hardcode() {
        let mut chord = make_chord("dangerous", Trit::Model);
        chord.safety_critical = true;
        let mm = make_mm(vec![chord]);
        let issues = Coherence::check(&mm);
        assert!(issues.iter().any(|i| i.chord == "dangerous"
            && matches!(i.severity, Severity::Critical)
            && i.description.contains("Safety-critical")));
    }

    #[test]
    fn test_gap_high_connectivity_no_tests() {
        let mut chord = make_chord("hub", Trit::Hardcode);
        chord.calls = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        chord.called_by = vec!["d".to_string(), "e".to_string()];
        chord.has_tests = false;
        let mm = make_mm(vec![chord]);
        let issues = Coherence::check(&mm);
        assert!(issues.iter().any(|i| i.chord == "hub"
            && matches!(i.severity, Severity::Warning)
            && i.description.contains("no tests")));
    }

    #[test]
    fn test_priority_inversion() {
        let mut chord = make_chord("important", Trit::Hardcode);
        chord.called_by = vec!["a".to_string(), "b".to_string(), "c".to_string(), "d".to_string(), "e".to_string()];
        chord.priority = 1;
        let mm = make_mm(vec![chord]);
        let issues = Coherence::check(&mm);
        assert!(issues.iter().any(|i| i.chord == "important"
            && matches!(i.severity, Severity::Warning)
            && i.description.contains("Priority inversion")));
    }

    #[test]
    fn test_clean_memory_no_issues() {
        let mut a = make_chord("a", Trit::Hardcode);
        a.calls = vec!["b".to_string()];
        a.has_tests = true;
        let mut b = make_chord("b", Trit::Model);
        b.called_by = vec!["a".to_string()];
        b.has_tests = true;
        let mm = make_mm(vec![a, b]);
        let issues = Coherence::check(&mm);
        // Should have no critical or warning issues (may have info for 'a' having callers)
        assert!(!issues
            .iter()
            .any(|i| matches!(i.severity, Severity::Critical | Severity::Warning)));
    }
}
