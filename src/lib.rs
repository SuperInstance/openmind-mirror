//! # openmind-mirror
//!
//! Self-reflection and coherence checking for agent muscle memory.
//! This is the agent's **metacognition** — it watches its own muscle memory
//! and detects when chord shapes are stale, inconsistent, or wrong.

pub mod calibration;
pub mod coherence;
pub mod mirror;
pub mod proprioceptor;
pub mod spectral;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// The decision source for a chord — how the agent resolves a function call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Trit {
    /// Always use the hardcoded path — no model invocation.
    Hardcode,
    /// Always delegate to the model.
    Model,
    /// Try hardcoded first, fall back to model on failure.
    Hybrid,
}

/// A single chord shape — the agent's knowledge about one function.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChordShape {
    /// Fully qualified function/method name.
    pub name: String,
    /// Decision source for this chord.
    pub decision: Trit,
    /// Hash of the source code this chord was built from.
    pub source_hash: String,
    /// Confidence score [0, 1].
    pub confidence: f64,
    /// Number of times this chord has been flexed (invoked).
    pub invocation_count: u64,
    /// Number of errors encountered during flex.
    pub error_count: u64,
    /// Average latency in milliseconds.
    pub avg_latency_ms: f64,
    /// Functions this chord calls (outgoing edges).
    pub calls: Vec<String>,
    /// Functions that call this chord (incoming edges).
    pub called_by: Vec<String>,
    /// Whether this chord has associated tests.
    pub has_tests: bool,
    /// Whether this function is safety-critical.
    pub safety_critical: bool,
    /// Priority rank (higher = more important).
    pub priority: u32,
}

/// A complete muscle memory snapshot — the agent's compiled reflexes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MuscleMemory {
    /// Source repository this memory was built from.
    pub repo: String,
    /// Version tag or commit hash.
    pub version: String,
    /// Timestamp when this snapshot was taken.
    pub timestamp: String,
    /// All chord shapes keyed by function name.
    pub chords: HashMap<String, ChordShape>,
}

impl MuscleMemory {
    /// Load muscle memory from a JSON file.
    pub fn load(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let data = fs::read_to_string(Path::new(path))?;
        let mm: MuscleMemory = serde_json::from_str(&data)?;
        Ok(mm)
    }

    /// Load from a JSON string.
    pub fn from_json(json: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let mm: MuscleMemory = serde_json::from_str(json)?;
        Ok(mm)
    }

    /// Save to a JSON file.
    pub fn save(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(Path::new(path), json)?;
        Ok(())
    }
}

/// Result of a single flex (invocation) call.
#[derive(Debug, Clone)]
pub struct FlexResult {
    /// Which chord was flexed.
    pub chord_name: String,
    /// Whether the flex succeeded.
    pub success: bool,
    /// Confidence returned by the flex.
    pub confidence: f64,
    /// Latency in milliseconds.
    pub latency_ms: f64,
}

/// Severity level for coherence issues.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Critical,
    Warning,
    Info,
}

/// A single coherence issue found during checking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    /// How severe this issue is.
    pub severity: Severity,
    /// Which chord has the issue.
    pub chord: String,
    /// Human-readable description.
    pub description: String,
}
