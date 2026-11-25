# Contributing

Cadence is intentionally small and dependency-free. Contributions should keep
the following invariants:

1. `cargo build --workspace --release` has zero warnings.
2. `cargo test --workspace` passes in debug and release.
3. No external crates, vendored code, network calls at runtime, CDNs or
   external fonts/assets.
4. No `.github/workflows` and no CI.
5. Every parser has explicit byte, depth and node limits.
6. Malformed input returns an error and never panics.
7. Layout and export stay deterministic and byte-stable.
8. Documentation is updated when the supported MusicXML subset or Studio API
   changes.

## Reporting a bug

Include:

- the Cadence commit and Rust version;
- the platform;
- the smallest MusicXML file that reproduces the problem (or the edit command
  sequence);
- whether the issue affects import, layout, SVG, PDF, MIDI or the Studio;
- expected versus actual behaviour.

Please do not paste copyrighted scores; reduce the case to a few measures first.

## Development loop

```sh
cargo build --workspace
cargo test --workspace
cargo run --release -p cadence
cargo run --release -p cadence-fuzz
cargo run --release -p cadence-bench
python3 tests/browser/studio_playwright.py
```

There is no CI; these commands are the gate. Keep the working tree building at
every commit.
