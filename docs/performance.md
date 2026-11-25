# Performance

Cadence prioritizes soundness and determinism over throughput. The numbers here
are measured, not estimated, and are not used as test golden values.

## Environment

- Apple silicon macOS (arm64)
- `rustc 1.99.1` / Cargo 1.99.1
- release profile: `opt-level = 3`, thin LTO, one codegen unit
- benchmark binary: `cargo run --release -p cadence-bench`

## Synthetic workload

The harness builds a deterministic single-part score with 120 measures, 4/4
time, 520 notes, chords, lyrics, dynamics, a second voice of rests and a final
barline. MusicXML size is 208,287 bytes.

Measured medians of 20 iterations after one warmup:

| Stage | Median | Output |
| --- | ---: | ---: |
| MusicXML import | 3.040 ms | 520 notes |
| Engrave layout | 0.521 ms | 15 pages, 60 systems, 939 symbols |
| Render SVG | 7.824 ms | 904,935 bytes |
| Render PDF | 10.491 ms | 404,463 bytes |
| Render MIDI | 0.045 ms | 4,726 bytes |
| Full pipeline (sum) | 21.920 ms | — |

SVG is the largest output because every text run is emitted as vector
rectangles and every glyph is a path. PDF is smaller because it emits filled
rectangles and paths directly; MIDI is tiny.

## Larger scores

The harness accepts a measure count argument:

```sh
cargo run --release -p cadence-bench 500
```

Expect roughly linear import/spacing work with pagination and file size growth.
The exact results depend on the machine; record them locally rather than
assuming the table above.

## What is not optimized

- No incremental layout or caching: every edit re-engraves.
- No parallelism inside the engraver or exporters; the Studio uses threads only
  for connections.
- SVG/PDF text is outlined, which trades size for portability and byte
  stability.
- Collision checking is O(n²) over ink symbols. It is fast enough for the
  fixture corpus and is used by tests, not by the normal export path.
- The browser re-renders the full SVG on each document version.

## Why this is acceptable

- Local single-user workbench, not a server or batch typesetter.
- 120 measures engrave and export all formats in about 22 ms on the reference
  machine.
- Determinism and bounded parsing matter more than beating a publisher
  engraver. No performance claim is made beyond the measured table.
