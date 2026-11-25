# Benchmark harness

`cadence-bench` is a dependency-free wall-clock harness. It is not a
microbenchmark framework and does not invent numbers: it builds a deterministic
synthetic score, runs each stage 20 times after one warmup, and prints the
median.

```sh
cargo run --release -p cadence-bench          # 120 measures
cargo run --release -p cadence-bench 500      # larger score
```

Stages measured independently:

1. MusicXML import of a ~200 KiB document.
2. Layout with systems, pages and collision-visible symbols.
3. SVG rendering.
4. PDF rendering.
5. MIDI rendering.

The output includes score size, page/system/symbol counts and output byte
sizes. Numbers depend on the machine; FINAL_REPORT.md records the machine used
and the measured medians. No performance result is stored as a golden value.

The harness is a workspace binary so `cargo test --workspace` still builds it
with zero warnings. It has no effect on the product binary or dependency graph.
