//! Proprioceptor — real-time monitoring of flex calls.

use crate::{FlexResult, MuscleMemory, Trit};

/// An alert raised by the proprioceptor.
#[derive(Debug, Clone)]
pub struct Alert {
    /// Type of alert.
    pub kind: AlertKind,
    /// Which chord triggered the alert.
    pub chord: String,
    /// Human-readable description.
    pub description: String,
}

/// Kinds of alerts the proprioceptor can raise.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlertKind {
    /// Agent tried to flex a chord not in memory.
    FlexMiss,
    /// Flex returned low confidence for a HARDCODE chord.
    ConfidenceDrop,
    /// Agent is flexing different chords than usual.
    PatternShift,
    /// Same chord flexed too many times without refresh.
    MuscleFatigue,
}

/// Tracks recent flex history for pattern detection.
struct FlexTracker {
    /// How many times each chord has been flexed (lifetime).
    invocation_counts: std::collections::HashMap<String, u64>,
    /// Recent chord names (for pattern detection).
    recent: Vec<String>,
    /// Baseline pattern: sorted list of most-flexed chords.
    baseline: Vec<String>,
}

impl FlexTracker {
    fn new(mm: &MuscleMemory) -> Self {
        let mut invocation_counts: std::collections::HashMap<String, u64> =
            std::collections::HashMap::new();
        let mut baseline: Vec<String> = mm.chords.keys().cloned().collect();
        for (name, chord) in &mm.chords {
            invocation_counts.insert(name.clone(), chord.invocation_count);
        }
        // Sort baseline by invocation count descending
        let counts = &invocation_counts;
        baseline.sort_by(|a, b| {
            let ca = counts.get(a).copied().unwrap_or(0);
            let cb = counts.get(b).copied().unwrap_or(0);
            cb.cmp(&ca)
        });
        FlexTracker {
            invocation_counts,
            recent: Vec::new(),
            baseline,
        }
    }

    fn record(&mut self, chord_name: &str) {
        *self
            .invocation_counts
            .entry(chord_name.to_string())
            .or_insert(0) += 1;
        self.recent.push(chord_name.to_string());
        // Keep only last 100
        if self.recent.len() > 100 {
            self.recent.remove(0);
        }
    }

    /// Check if the recent pattern diverges from baseline.
    /// Compares the top-10 most-flexed chords from recent history vs baseline.
    fn pattern_shifted(&self) -> bool {
        if self.recent.len() < 20 {
            return false;
        }

        // Count recent frequencies
        let mut freq: std::collections::HashMap<&str, u64> = std::collections::HashMap::new();
        for name in &self.recent {
            *freq.entry(name.as_str()).or_insert(0) += 1;
        }
        let mut recent_ranked: Vec<(&str, u64)> = freq.into_iter().collect();
        recent_ranked.sort_by(|a, b| b.1.cmp(&a.1));
        let recent_top: Vec<&str> = recent_ranked.iter().take(10).map(|(n, _)| *n).collect();

        let baseline_top: Vec<&str> = self.baseline.iter().take(10).map(|s| s.as_str()).collect();

        // If less than 3 overlap, pattern has shifted
        let overlap = recent_top
            .iter()
            .filter(|n| baseline_top.contains(n))
            .count();
        overlap < 3
    }
}

/// Real-time proprioceptor for monitoring flex behavior.
pub struct Proprioceptor {
    muscle_memory: MuscleMemory,
    tracker: FlexTracker,
    alerts: Vec<Alert>,
    /// Threshold for confidence drop alert (0-1).
    pub confidence_threshold: f64,
    /// Threshold for muscle fatigue (invocation count).
    pub fatigue_threshold: u64,
}

impl Proprioceptor {
    /// Create a new proprioceptor monitoring the given muscle memory.
    pub fn new(mm: &MuscleMemory) -> Self {
        Proprioceptor {
            tracker: FlexTracker::new(mm),
            alerts: Vec::new(),
            muscle_memory: mm.clone(),
            confidence_threshold: 0.5,
            fatigue_threshold: 1000,
        }
    }

    /// Observe a flex result and check for anomalies.
    pub fn observe(&mut self, result: &FlexResult) {
        // Check for flex miss
        if !self.muscle_memory.chords.contains_key(&result.chord_name) {
            self.alerts.push(Alert {
                kind: AlertKind::FlexMiss,
                chord: result.chord_name.clone(),
                description: format!(
                    "Agent tried to flex '{}' which is not in muscle memory",
                    result.chord_name
                ),
            });
            return;
        }

        let chord = &self.muscle_memory.chords[&result.chord_name];

        // Check for confidence drop on HARDCODE chords
        if chord.decision == Trit::Hardcode && result.confidence < self.confidence_threshold {
            self.alerts.push(Alert {
                kind: AlertKind::ConfidenceDrop,
                chord: result.chord_name.clone(),
                description: format!(
                    "HARDCODE chord '{}' returned confidence {:.2} (below {:.2})",
                    result.chord_name, result.confidence, self.confidence_threshold
                ),
            });
        }

        // Track invocation
        self.tracker.record(&result.chord_name);

        // Check for muscle fatigue
        let count = self
            .tracker
            .invocation_counts
            .get(&result.chord_name)
            .copied()
            .unwrap_or(0);
        if count >= self.fatigue_threshold {
            self.alerts.push(Alert {
                kind: AlertKind::MuscleFatigue,
                chord: result.chord_name.clone(),
                description: format!(
                    "Chord '{}' flexed {} times without refresh (threshold: {})",
                    result.chord_name, count, self.fatigue_threshold
                ),
            });
        }

        // Check for pattern shift
        if self.tracker.pattern_shifted() {
            self.alerts.push(Alert {
                kind: AlertKind::PatternShift,
                chord: "(global)".to_string(),
                description: "Agent is flexing a significantly different pattern of chords"
                    .to_string(),
            });
        }
    }

    /// Drain and return all pending alerts.
    pub fn alerts(&mut self) -> Vec<Alert> {
        std::mem::take(&mut self.alerts)
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
    fn test_flex_miss() {
        let mm = make_mm(vec![make_chord("foo", Trit::Hardcode)]);
        let mut prop = Proprioceptor::new(&mm);
        prop.observe(&FlexResult {
            chord_name: "nonexistent".to_string(),
            success: true,
            confidence: 0.99,
            latency_ms: 1.0,
        });
        let alerts = prop.alerts();
        assert!(alerts.iter().any(|a| a.kind == AlertKind::FlexMiss));
    }

    #[test]
    fn test_confidence_drop() {
        let mm = make_mm(vec![make_chord("foo", Trit::Hardcode)]);
        let mut prop = Proprioceptor::new(&mm);
        prop.confidence_threshold = 0.8;
        prop.observe(&FlexResult {
            chord_name: "foo".to_string(),
            success: true,
            confidence: 0.3,
            latency_ms: 1.0,
        });
        let alerts = prop.alerts();
        assert!(alerts.iter().any(|a| a.kind == AlertKind::ConfidenceDrop));
    }

    #[test]
    fn test_muscle_fatigue() {
        let mut chord = make_chord("overworked", Trit::Hardcode);
        chord.invocation_count = 999;
        let mm = make_mm(vec![chord]);
        let mut prop = Proprioceptor::new(&mm);
        prop.fatigue_threshold = 1000;
        prop.observe(&FlexResult {
            chord_name: "overworked".to_string(),
            success: true,
            confidence: 0.99,
            latency_ms: 1.0,
        });
        let alerts = prop.alerts();
        assert!(alerts.iter().any(|a| a.kind == AlertKind::MuscleFatigue));
    }

    #[test]
    fn test_no_alerts_on_normal_flex() {
        let mm = make_mm(vec![make_chord("foo", Trit::Hardcode)]);
        let mut prop = Proprioceptor::new(&mm);
        prop.observe(&FlexResult {
            chord_name: "foo".to_string(),
            success: true,
            confidence: 0.99,
            latency_ms: 1.0,
        });
        let alerts = prop.alerts();
        assert!(alerts.is_empty());
    }
}
