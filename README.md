# OpenMind Mirror

**Metacognition for AI agents** — self-reflection and coherence checking for agent muscle memory. OpenMind Mirror watches an agent's compiled reflexes (its "muscle memory"), detects when they go stale or inconsistent, and recommends calibration adjustments based on observed performance.

## Why It Matters

As AI agents take on autonomous long-running tasks, they need **metacognitive awareness** — the ability to monitor their own capabilities and detect when their internal models are wrong:

- **Drift detection** — Did the codebase change underneath the agent's cached knowledge?
- **Coherence checking** — Are there broken call chains or orphaned functions in memory?
- **Calibration** — Should a MODEL-based decision be promoted to HARDCODE after 100 successful calls?
- **Proprioception** — Is the agent calling functions it doesn't have in memory?

This is the **self-monitoring layer** for the SuperInstance ecosystem's agent muscle memory system.

## How It Works

### Chord Shapes

Each function the agent knows about is a **ChordShape** — a compiled reflex with:

| Field | Description |
|-------|-------------|
| `decision` | How to resolve: `HARDCODE` (direct), `MODEL` (LLM), `HYBRID` (hardcode→model fallback) |
| `source_hash` | Content hash of the source code this chord was built from |
| `invocation_count` | How many times this chord has been flexed |
| `error_count` | How many flexes resulted in errors |
| `calls` / `called_by` | Call graph edges (outgoing / incoming) |
| `safety_critical` | If true, must remain HARDCODE |
| `priority` | Importance rank (higher = more critical) |

### The Trit Decision Model

Each chord uses a **ternary decision** (γ + η = C):

| Trit | Value | Meaning |
|------|-------|---------|
| `HARDCODE` | +1 | Always use cached path — fastest, no model call |
| `MODEL` | 0 | Always delegate to LLM — most flexible, slowest |
| `HYBRID` | −1 | Try hardcode first, fall back to model on error |

The sum γ + η = C: the total invocation space C is partitioned into **(γ) hardcoded calls** and **(η) model-delegated calls**. The calibration system optimizes to maximize γ (speed) while maintaining accuracy.

### Module: Coherence

Detects five classes of internal inconsistency:

| Issue | Severity | Condition |
|-------|----------|-----------|
| Orphan chord | Info | `calls.is_empty() && called_by.is_empty()` |
| Broken chain | Critical | Calls a function not in muscle memory |
| Safety violation | Critical | `safety_critical && decision != HARDCODE` |
| Test gap | Warning | High connectivity (≥5 edges) + `!has_tests` |
| Priority inversion | Warning | `called_by.len() >= 5 && priority < 3` |

### Module: Mirror (Version Diff)

Compares two muscle memory snapshots and reports:

- **Changed chords** — `source_hash` differs (implementation modified)
- **Added chords** — new in v2
- **Removed chords** — deleted from v2
- **Decision shifts** — Trit changed (e.g., MODEL → HARDCODE)

### Module: Calibration

Recommends Trit transitions based on observed statistics:

| Current | Condition | Recommended |
|---------|-----------|-------------|
| MODEL | ≥100 successes, error_rate < 0.1 | HARDCODE |
| MODEL | error_rate > 0.1, ≥10 calls | HYBRID |
| HARDCODE | error_rate > 0.1, ≥10 calls | HYBRID |
| HYBRID | ≥100 successes, error_rate < 0.1 | HARDCODE |

Safety-critical functions are never adjusted.

### Module: Proprioceptor

Real-time flex monitoring that raises alerts:

- **FlexMiss** — calling a chord not in memory
- **ConfidenceDrop** — HARDCODE chord returning low confidence
- **PatternShift** — agent flexing unusual chord patterns
- **MuscleFatigue** — same chord flexed excessively without refresh

### Complexity

| Operation | Time |
|-----------|------|
| Coherence check | O(V + E) where V = chords, E = call edges |
| Mirror diff | O(V₁ + V₂) |
| Calibration analysis | O(V) |
| Proprioceptor check | O(1) per flex, O(W) for pattern shift (W = window size) |

## Quick Start

```rust
use openmind_mirror::{MuscleMemory, ChordShape, Trit, Severity};
use openmind_mirror::coherence::Coherence;
use openmind_mirror::mirror::Mirror;
use openmind_mirror::calibration::Calibration;

// Load muscle memory
let mm = MuscleMemory::load("muscle-memory.json")?;

// Check coherence
let issues = Coherence::check(&mm);
for issue in &issues {
    println!("[{:?}] {}: {}", issue.severity, issue.chord, issue.description);
}

// Calibrate
let cal = Calibration::new();
for adj in cal.calibrate(&mm) {
    println!("{:?} → {:?}: {}", adj.current, adj.recommended, adj.reason);
}

// Compare with previous version
let old = MuscleMemory::load("muscle-memory-old.json")?;
let report = Mirror::compare(&old, &mm);
println!("Changed: {}, Added: {}, Removed: {}", report.changed.len(), report.added.len(), report.removed.len());
```

## API

### Core Types
- `MuscleMemory` — serializable snapshot of all chord shapes (load/save JSON)
- `ChordShape` — per-function metadata and decision
- `Trit` — `Hardcode` | `Model` | `Hybrid`
- `Issue` / `Severity` — coherence findings

### Modules
- `coherence::Coherence::check(&mm) -> Vec<Issue>`
- `mirror::Mirror::compare(&v1, &v2) -> MirrorReport`
- `calibration::Calibration::new().calibrate(&mm) -> Vec<CalibrationAdjustment>`
- `proprioceptor::Proprioceptor` — real-time flex monitoring
- `spectral::Spectral` — spectral analysis of chord patterns

## Architecture Notes

The muscle memory is a **directed graph** where nodes are chord shapes and edges are call relationships. The coherence checker performs a single graph traversal, and the mirror diff uses hash-based set operations for O(V) comparison.

The **γ + η = C** link is explicit and central: the Trit enum is the ternary decision model. HARDCODE (+1) is the γ term (deterministic, cached), MODEL (0) and HYBRID (−1) are the η terms (non-deterministic, model-invoking). The calibration system's goal is to maximize γ/C (the fraction of calls served by hardcode) subject to accuracy constraints.

## References

1. Flavell, J. H. (1979). "Metacognition and cognitive monitoring: A new area of cognitive-developmental inquiry." *American Psychologist*, 34(10), 906–911. — Origin of metacognition theory.
2. Anderson, J. R. (1996). "ACT: A simple theory of complex cognition." *American Psychologist*, 51(4), 355–365. — Procedural memory (production rules) in cognitive architectures.
3. Laird, J. E. (2012). *The Soar Cognitive Architecture*. MIT Press. — Chunking and procedural learning.
4. Lake, B. M. et al. (2017). "Building machines that learn and think like people." *Behavioral and Brain Sciences*, 40, e253.
5. Kahneman, D. (2011). *Thinking, Fast and Slow*. — System 1 (hardcode) vs. System 2 (model) analogy.

## License

MIT
