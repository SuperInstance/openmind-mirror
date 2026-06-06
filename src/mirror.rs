//! Mirror — compare two muscle memory snapshots and detect drift.

use crate::{MuscleMemory, Trit};

/// A chord that changed its decision source between versions.
#[derive(Debug, Clone)]
pub struct DecisionShift {
    pub chord: String,
    pub from: Trit,
    pub to: Trit,
}

/// A chord whose source hash changed (implementation modified).
#[derive(Debug, Clone)]
pub struct ChangedChord {
    pub chord: String,
    pub old_hash: String,
    pub new_hash: String,
}

/// Full diff report between two muscle memory snapshots.
#[derive(Debug, Clone)]
pub struct MirrorReport {
    /// Chords whose source_hash changed.
    pub changed: Vec<ChangedChord>,
    /// New chords present in v2 but not v1.
    pub added: Vec<String>,
    /// Chords present in v1 but missing from v2.
    pub removed: Vec<String>,
    /// Chords whose decision source shifted.
    pub decision_shifts: Vec<DecisionShift>,
}

/// Compare two muscle memory snapshots.
pub struct Mirror;

impl Mirror {
    /// Produce a full diff report between two versions.
    pub fn compare(v1: &MuscleMemory, v2: &MuscleMemory) -> MirrorReport {
        let mut changed = Vec::new();
        let mut added = Vec::new();
        let mut removed = Vec::new();
        let mut decision_shifts = Vec::new();

        // Find removed chords (in v1 but not v2)
        for name in v1.chords.keys() {
            if !v2.chords.contains_key(name) {
                removed.push(name.clone());
            }
        }

        // Find added chords and changed chords
        for (name, chord_v2) in &v2.chords {
            match v1.chords.get(name) {
                None => {
                    added.push(name.clone());
                }
                Some(chord_v1) => {
                    if chord_v1.source_hash != chord_v2.source_hash {
                        changed.push(ChangedChord {
                            chord: name.clone(),
                            old_hash: chord_v1.source_hash.clone(),
                            new_hash: chord_v2.source_hash.clone(),
                        });
                    }
                    if chord_v1.decision != chord_v2.decision {
                        decision_shifts.push(DecisionShift {
                            chord: name.clone(),
                            from: chord_v1.decision,
                            to: chord_v2.decision,
                        });
                    }
                }
            }
        }

        MirrorReport {
            changed,
            added,
            removed,
            decision_shifts,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ChordShape;
    use crate::Trit;
    use std::collections::HashMap;

    fn make_chord(name: &str, decision: Trit, hash: &str) -> ChordShape {
        ChordShape {
            name: name.to_string(),
            decision,
            source_hash: hash.to_string(),
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
    fn test_detect_added_chords() {
        let v1 = make_mm(vec![make_chord("foo", Trit::Hardcode, "aaa")]);
        let v2 = make_mm(vec![
            make_chord("foo", Trit::Hardcode, "aaa"),
            make_chord("bar", Trit::Model, "bbb"),
        ]);
        let report = Mirror::compare(&v1, &v2);
        assert_eq!(report.added, vec!["bar"]);
        assert!(report.removed.is_empty());
        assert!(report.changed.is_empty());
    }

    #[test]
    fn test_detect_removed_chords() {
        let v1 = make_mm(vec![
            make_chord("foo", Trit::Hardcode, "aaa"),
            make_chord("baz", Trit::Model, "ccc"),
        ]);
        let v2 = make_mm(vec![make_chord("foo", Trit::Hardcode, "aaa")]);
        let report = Mirror::compare(&v1, &v2);
        assert_eq!(report.removed, vec!["baz"]);
        assert!(report.added.is_empty());
    }

    #[test]
    fn test_detect_changed_chords() {
        let v1 = make_mm(vec![make_chord("foo", Trit::Hardcode, "aaa")]);
        let v2 = make_mm(vec![make_chord("foo", Trit::Hardcode, "bbb")]);
        let report = Mirror::compare(&v1, &v2);
        assert_eq!(report.changed.len(), 1);
        assert_eq!(report.changed[0].chord, "foo");
        assert_eq!(report.changed[0].old_hash, "aaa");
        assert_eq!(report.changed[0].new_hash, "bbb");
    }

    #[test]
    fn test_detect_decision_shift() {
        let v1 = make_mm(vec![make_chord("foo", Trit::Hardcode, "aaa")]);
        let v2 = make_mm(vec![make_chord("foo", Trit::Model, "aaa")]);
        let report = Mirror::compare(&v1, &v2);
        assert_eq!(report.decision_shifts.len(), 1);
        assert_eq!(report.decision_shifts[0].from, Trit::Hardcode);
        assert_eq!(report.decision_shifts[0].to, Trit::Model);
    }

    #[test]
    fn test_identical_snapshots() {
        let v1 = make_mm(vec![
            make_chord("foo", Trit::Hardcode, "aaa"),
            make_chord("bar", Trit::Model, "bbb"),
        ]);
        let v2 = make_mm(vec![
            make_chord("foo", Trit::Hardcode, "aaa"),
            make_chord("bar", Trit::Model, "bbb"),
        ]);
        let report = Mirror::compare(&v1, &v2);
        assert!(report.changed.is_empty());
        assert!(report.added.is_empty());
        assert!(report.removed.is_empty());
        assert!(report.decision_shifts.is_empty());
    }
}
