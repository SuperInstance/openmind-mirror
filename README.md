# openmind-mirror

**Self-reflection and coherence checking for agent muscle memory.**

This crate is the metacognition layer for the OpenMind agent architecture — it watches the agent's own muscle memory and detects when chord shapes are stale, inconsistent, or wrong.

## Why Metacognition?

An AI agent that can't reflect on its own behavior is flying blind. Muscle memory — the agent's compiled reflexes for how to handle function calls — needs to be monitored just like any other system. Things drift. Decisions become stale. Repos diverge. The agent needs to *notice* when something's off.

Openmind-mirror provides five modules for self-reflection:

### Mirror

Compare two muscle memory snapshots and detect drift — chords added, removed, changed, or whose decision source shifted between HARDCODE, MODEL, and HYBRID.

```rust
let report = Mirror::compare(&mm_v1, &mm_v2);
println!("Changed: {:?}", report.changed);
println!("Decision shifts: {:?}", report.decision_shifts);
```

### Coherence

Check if a muscle memory snapshot is internally consistent. Catches orphans (functions nothing calls and that call nothing), broken chains (calls to functions not in memory), safety-critical functions with wrong decisions, test gaps on high-connectivity hubs, and priority inversions.

```rust
let issues = Coherence::check(&mm);
for issue in &issues {
    println!("{:?}: {} — {}", issue.severity, issue.chord, issue.description);
}
```

### Spectral

Detect the *spectral isomorphism* — whether all repos project the same structural pattern. Compares decision vectors across muscle memories using cosine similarity and flags repos that diverge.

```rust
let report = Spectral::analyze(&[mm1, mm2, mm3, mm4, mm5], 0.95);
println!("Mean similarity: {:.3}", report.mean_similarity);
println!("Outliers: {:?}", report.outliers);
```

### Proprioceptor

Real-time monitoring: watch flex calls and detect anomalies — flex misses (calling something not in memory), confidence drops on HARDCODE chords, pattern shifts (agent suddenly using different chords), and muscle fatigue (same chord flexed 1000+ times without refresh).

```rust
let mut prop = Proprioceptor::new(&mm);
prop.observe(&flex_result);
for alert in prop.alerts() {
    println!("{:?}: {}", alert.kind, alert.description);
}
```

### Calibration

Re-tune decisions based on observed behavior. Tracks actual error rates and invocation counts, then recommends upgrades (MODEL→HARDCODE after 100+ clean invocations) or downgrades (HARDCODE→HYBRID when error rate exceeds threshold).

```rust
let cal = Calibration::new();
for adj in cal.calibrate(&mm) {
    println!("{}: {:?} → {:?} ({})", adj.chord, adj.current, adj.recommended, adj.reason);
}
```

## Architecture

The core data types are:

- **`MuscleMemory`** — a complete snapshot of the agent's reflexes, loaded from JSON
- **`ChordShape`** — the agent's knowledge about one function (decision, confidence, call graph edges)
- **`Trit`** — three-valued decision: HARDCODE, MODEL, or HYBRID
- **`FlexResult`** — the result of a single invocation (success, confidence, latency)

No external ML or vector dependencies — pure structural comparison and decision logic.

## The Bigger Picture

This crate is one piece of the OpenMind architecture:

- **openmind** (Python) — the core agent that builds muscle memory
- **openmind-chords** — chord shape definitions and function dispatch
- **openmind-mirror** — *this crate* — self-reflection and coherence checking

An agent that can monitor itself, detect when it's degraded, and recommend its own calibration is an agent that gets *better* over time instead of silently rotting. That's the goal.

## License

MIT
