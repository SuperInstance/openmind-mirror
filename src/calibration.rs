//! Calibration — re-tune decisions based on observed behavior.
//!
//! Tracks actual latencies and error rates of flex calls, then
//! recommends decision adjustments.

use crate::{MuscleMemory, Trit};

/// A recommended calibration adjustment.
#[derive(Debug, Clone)]
pub struct CalibrationAdjustment {
    /// Which chord to adjust.
    pub chord: String,
    /// Current decision.
    pub current: Trit,
    /// Recommended new decision.
    pub recommended: Trit,
    /// Reason for the adjustment.
    pub reason: String,
}

/// Tracks observed flex statistics for calibration.
pub struct Calibration {
    /// Error rate threshold for downgrading (0-1).
    pub error_rate_threshold: f64,
    /// Minimum successful invocations before upgrading MODEL→HARDCODE.
    pub success_upgrade_threshold: u64,
}

impl Calibration {
    /// Create a new calibrator with default thresholds.
    pub fn new() -> Self {
        Calibration {
            error_rate_threshold: 0.1,
            success_upgrade_threshold: 100,
        }
    }

    /// Analyze muscle memory and recommend calibration adjustments.
    ///
    /// - MODEL→HARDCODE: if 100+ successful invocations and error_rate < threshold
    /// - HARDCODE→HYBRID: if error_rate > threshold
    /// - HYBRID→HARDCODE: if 100+ successful invocations and error_rate < threshold
    /// - MODEL→HYBRID: if error_rate > threshold (softer downgrade)
    pub fn calibrate(&self, mm: &MuscleMemory) -> Vec<CalibrationAdjustment> {
        let mut adjustments = Vec::new();

        for (name, chord) in &mm.chords {
            let total = chord.invocation_count;
            let errors = chord.error_count;
            let error_rate = if total > 0 {
                errors as f64 / total as f64
            } else {
                0.0
            };

            // Don't adjust safety-critical functions
            if chord.safety_critical {
                continue;
            }

            match chord.decision {
                Trit::Model => {
                    if total >= self.success_upgrade_threshold
                        && error_rate < self.error_rate_threshold
                    {
                        adjustments.push(CalibrationAdjustment {
                            chord: name.clone(),
                            current: Trit::Model,
                            recommended: Trit::Hardcode,
                            reason: format!(
                                "{} successful invocations with error rate {:.3} (< {:.3})",
                                total, error_rate, self.error_rate_threshold
                            ),
                        });
                    } else if error_rate > self.error_rate_threshold && total >= 10 {
                        // Already MODEL with high errors — suggest HYBRID for fallback
                        adjustments.push(CalibrationAdjustment {
                            chord: name.clone(),
                            current: Trit::Model,
                            recommended: Trit::Hybrid,
                            reason: format!(
                                "High error rate {:.3} (> {:.3}) with {} invocations",
                                error_rate, self.error_rate_threshold, total
                            ),
                        });
                    }
                }
                Trit::Hardcode => {
                    if error_rate > self.error_rate_threshold && total >= 10 {
                        adjustments.push(CalibrationAdjustment {
                            chord: name.clone(),
                            current: Trit::Hardcode,
                            recommended: Trit::Hybrid,
                            reason: format!(
                                "HARDCODE error rate {:.3} exceeds threshold {:.3}",
                                error_rate, self.error_rate_threshold
                            ),
                        });
                    }
                }
                Trit::Hybrid => {
                    if total >= self.success_upgrade_threshold
                        && error_rate < self.error_rate_threshold
                    {
                        adjustments.push(CalibrationAdjustment {
                            chord: name.clone(),
                            current: Trit::Hybrid,
                            recommended: Trit::Hardcode,
                            reason: format!(
                                "HYBRID performing well: {} invocations, error rate {:.3}",
                                total, error_rate
                            ),
                        });
                    }
                }
            }
        }

        adjustments
    }
}

impl Default for Calibration {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ChordShape;
    use std::collections::HashMap;

    fn make_chord(name: &str, decision: Trit, invocations: u64, errors: u64) -> ChordShape {
        ChordShape {
            name: name.to_string(),
            decision,
            source_hash: "hash".to_string(),
            confidence: 0.95,
            invocation_count: invocations,
            error_count: errors,
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
    fn test_upgrade_model_to_hardcode() {
        let mm = make_mm(vec![make_chord("foo", Trit::Model, 150, 0)]);
        let cal = Calibration::new();
        let adj = cal.calibrate(&mm);
        assert_eq!(adj.len(), 1);
        assert_eq!(adj[0].recommended, Trit::Hardcode);
        assert_eq!(adj[0].current, Trit::Model);
    }

    #[test]
    fn test_downgrade_hardcode_to_hybrid() {
        let mm = make_mm(vec![make_chord("bar", Trit::Hardcode, 50, 10)]);
        let cal = Calibration::new();
        let adj = cal.calibrate(&mm);
        assert_eq!(adj.len(), 1);
        assert_eq!(adj[0].recommended, Trit::Hybrid);
        assert_eq!(adj[0].current, Trit::Hardcode);
    }

    #[test]
    fn test_no_adjustment_safety_critical() {
        let mut chord = make_chord("critical", Trit::Model, 200, 0);
        chord.safety_critical = true;
        let mm = make_mm(vec![chord]);
        let cal = Calibration::new();
        let adj = cal.calibrate(&mm);
        assert!(adj.is_empty());
    }

    #[test]
    fn test_upgrade_hybrid_to_hardcode() {
        let mm = make_mm(vec![make_chord("baz", Trit::Hybrid, 120, 2)]);
        let cal = Calibration::new();
        let adj = cal.calibrate(&mm);
        assert_eq!(adj.len(), 1);
        assert_eq!(adj[0].recommended, Trit::Hardcode);
    }

    #[test]
    fn test_no_adjustment_insufficient_data() {
        let mm = make_mm(vec![make_chord("qux", Trit::Model, 10, 0)]);
        let cal = Calibration::new();
        let adj = cal.calibrate(&mm);
        assert!(adj.is_empty());
    }
}
