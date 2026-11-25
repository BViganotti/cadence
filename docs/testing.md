# Testing

Cadence uses only the Rust standard library. The test suite is split across
unit tests in each crate, end-to-end tests in `tests/`, a deterministic
malformed-input harness in `fuzz/`, and a Playwright browser test.

## Commands

```sh
cargo build --workspace --release
cargo test --workspace
cargo test --workspace --release
cargo run --release -p cadence-fuzz 1000
cargo run --release -p cadence-bench
python3 tests/browser/studio_playwright.py
```

No CI workflow exists by design. The commands above are the release gate.

## What is covered

### `cadence-core`

- Pitch spelling, MIDI numbers, range rejection and scientific notation.
- Exact tick conversion for every note value and dot count; unrepresentable
  tick values are rejected.
- Key signatures, tonic spelling, clef staff mapping and beam groups.
- Chord sorting/deduplication, voice creation, measure editing.
- Pickup timeline, tie/slur resolution, beam runs.
- Structural validation and musical inspection (overfull/underfull, ties,
  duplicate part IDs, transpose bounds).

### `cadence-import`

- XML parsing of elements, attributes, entities, CDATA, namespaces and comments.
- Hard limits: depth, node count, attributes, names, entity values and input
  bytes.
- Rejection of DOCTYPE, external entities, unknown entities, mismatched tags,
  duplicate attributes and trailing content.
- MusicXML mapping for notes, chords, rests, voices, backup/forward, ties,
  slurs, beams, lyrics, articulations, dynamics, tempo, repeats, endings and
  barline styles.
- Explicit rejection of tuplets, grace/cue notes, multiple staves, composite
  time signatures and inconsistent durations.
- Deterministic export and byte-stable re-export.
- 100 checked-in fixtures round-trip without lost notes or marks.

### `cadence-engrave`

- Duration-aware columns, voice alignment, lyric widening and prefix widths.
- Automatic/explicit beam groups, beat splitting and rests.
- System and page breaking, forced breaks and height pagination.
- Deterministic layout equality.
- Zero overlapping ink across 300 generated scores.
- Part extraction layouts and hit geometry.

### `cadence-export`

- SVG is self-contained, deterministic, contains no `<text>` or external
  references, and carries hit regions for the Studio.
- PDF has valid object offsets, stream lengths, trailer and page count; `pdfinfo`
  opens the demo PDF when Poppler is installed.
- MIDI header, track count, PPQ, tempo/time/key meta, balanced note-on/off
  events, merged ties and deterministic bytes.

### `cadence-studio`

- JSON parsing/serialization, nesting and byte limits, duplicate keys,
  surrogates and non-finite numbers.
- Every edit command, undo bounds, validation rollback, import/reset, summary
  and event JSON.
- HTTP auth, state, edit, layout, exports, SSE initial event, filename safety,
  percent decoding and connection limits.

### Browser

`tests/browser/studio_playwright.py` starts the real binary on a free loopback
port and verifies: authenticated page load, token removal from the URL, demo
diagnostics, full-score preview, part selection, note editing, measure/note
creation, MusicXML import, and SVG/PDF/MIDI/MusicXML downloads with signature
checks. It also fails on browser console errors.

## Fuzzing

The fuzz harness is deterministic: a fixed xorshift seed derives 1,000
mutations from `examples/demo.musicxml` (truncation, byte replacement, tag
injection, depth bombs, attribute bombs, entity bombs, DOCTYPE and numeric
corruption). Every mutation is parsed inside `catch_unwind`. Accepted mutations
are also engraved and exported inside `catch_unwind`. The test requires zero
panics and at least 900 clean rejections.

## Coverage

Coverage is measured with `cargo llvm-cov` in the release report. It is not part
of the build because the workspace itself must depend only on the standard
library; coverage tooling is a developer tool, not a dependency.
