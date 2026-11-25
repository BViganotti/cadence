# Cadence — Final Report

## Status: complete

Cadence is a local browser music notation and engraving workbench built with
Rust's standard library only. It imports MusicXML, edits a score in a loopback
Studio, engraves a deterministic collision-free layout, and exports SVG, PDF,
MIDI and MusicXML. No external crates, vendored code, CDN, host fonts,
telemetry, CI workflows or network runtime are used.

Repository: <https://github.com/BViganotti/cadence> (public, `master`)

## Definition of Done

| # | Requirement | Evidence | Status |
| --- | --- | --- | --- |
| 1 | `cargo build --release` succeeds with zero warnings | `cargo build --workspace --release` → finished, no warnings | ✅ |
| 2 | `cargo test` and `cargo test --release` pass | debug: 293 tests pass; release: 293 tests pass | ✅ |
| 3 | Project-specific acceptance tests | 100 fixtures round-trip; 300 layouts collision-free; PDF/SVG/MIDI checks; Playwright flow | ✅ |
| 4 | 1,000 malformed inputs fail cleanly, never panic | fuzz run: total 1000, rejected 988, accepted 12, panics 0 | ✅ |
| 5 | Browser Studio demo works via Playwright | `tests/browser/studio_playwright.py` passed against the release binary | ✅ |
| 6 | README, docs, CHANGELOG, LICENSE, LIMITATIONS, FINAL_REPORT | all present; docs index links verified | ✅ |
| 7 | FINAL_REPORT lists LOC, commits, tests, coverage, benchmarks, gaps | this document | ✅ |
| 8 | Repo committed and pushed | public GitHub repository, pushed | ✅ |
| 9 | No external crate, no CI, no CLI-only product | dependency invariant test, no `.github/workflows`, Studio is the product | ✅ |

## Deliverables

### Crates

| Crate | Purpose |
| --- | --- |
| `cadence-core` | score model, notation analysis, validation |
| `cadence-import` | bounded XML parser/writer, MusicXML import/export |
| `cadence-engrave` | spacing, beaming, collisions, systems, pages |
| `cadence-export` | SVG, vector PDF, MIDI |
| `cadence-studio` | bounded JSON, document editing, loopback HTTP/SSE, UI |
| `cadence` (root) | product launcher binary |
| `fuzz` | deterministic malformed-input harness (dev tool) |
| `bench` | dependency-free timing harness (dev tool) |
| `fixtures` | deterministic fixture generator (dev tool) |

### Product features

- Notes, chords, rests, voices, measures, parts; ties, slurs, beams, lyrics,
  articulations, dynamics, accidentals, repeats, endings, tempo, key/time/clef
  changes.
- MusicXML 4.0 `score-partwise` import/export for the documented subset, with
  strict rejection of unsupported constructs.
- Deterministic engraving with shared voice columns, chord seconds, accidental
  state, beams/flags, lyric lanes, dynamics, ledger lines, systems and pages.
- Original SVG, vector PDF and Standard MIDI writers; byte-stable output.
- Studio with full-score/part preview, note entry, chord-note editing, lyric and
  dynamic editing, metadata, tempo/key, transpose, parts, measures, zoom,
  page navigation, drag-and-drop import, keyboard shortcuts, undo/redo,
  diagnostics and four export formats.
- Loopback-only token-protected HTTP/SSE server with CSP and hard limits.

## Size

| Area | LOC |
| --- | ---: |
| Product Rust (`src/` + `crates/`) | 13,337 |
| Embedded UI assets (`assets/`) | 1,869 |
| Workspace tests (`tests/`) | 3,944 |
| Dev harnesses (`fuzz/`, `bench/`, `fixtures/`) | 634 |
| Documentation (`docs/` + root Markdown) | 1,149 |
| **Rust + assets total** | **19,784** |

Product code is 13.3k LOC, inside the 13–15k target. Test and harness code is
reported separately rather than hidden inside the product count.

## Commits

- **96 commits** at hand-off (this report is part of the final commit).
- Every commit builds and passes the workspace tests.
- History is a sequence of vertical slices: core model, MusicXML, engraving,
  export, Studio, hardening, tests and documentation.
- Pushed publicly; no CI workflow was created.

## Tests

- **293 tests** across unit, integration and cross-crate suites.
- Debug and release test runs both pass with zero failures.
- Key acceptance suites:
  - `tests/fixtures.rs` — 100 MusicXML fixtures import, validate, export twice
    and re-import without lost notes or marks.
  - `tests/layout.rs` — 300 generated scores (100 fixtures × 3 transpositions)
    engrave with zero overlapping ink symbols.
  - `tests/fixtures_export.rs` — all 100 fixtures engrave and export to
    SVG/PDF/MIDI.
  - `tests/golden.rs` — FNV-1a hashes pin SVG/PDF/MIDI/MusicXML demo bytes.
  - `tests/pdf.rs` — xref offsets, stream lengths, MediaBox, page count and an
    optional `pdfinfo` open check.
  - `tests/midi.rs` — track structure, balanced note-ons/offs, merged ties,
    channels and determinism.
  - `tests/server*.rs` — token auth, cookie, edit/import/export, SSE, 405,
    malformed requests and resource limits.
  - `tests/browser/studio_playwright.py` — real binary + Chromium: import,
    edit, keyboard entry/undo/redo, zoom, page navigation, preview, export.

## Coverage

Measured with `cargo llvm-cov --workspace --summary-only` on this workspace:

| Metric | Coverage |
| --- | ---: |
| Lines | **83.89%** (10,557 instrumented, 1,701 missed) |
| Regions | 82.97% |
| Functions | 79.23% |

Coverage includes the dev harness and product launcher entry points, which are
not exercised by `cargo test` and report 0%; product modules are generally
higher (engraver layout 94.8% lines, MIDI 96.9%, PDF 96.2%, SVG 98.8%, XML
parser 93.1%). Branch coverage is not reported by this tool version.

## Fuzz

`cargo run --release -p cadence-fuzz 1000`:

```text
FuzzReport { total: 1000, rejected: 988, accepted: 12, panics: 0, panic_samples: [] }
```

The twelve accepted mutations are still-valid documents; they are engraved and
exported inside the same panic guard. No parser, layout or export panic
occurred.

## Benchmarks

Environment: Apple silicon macOS (arm64), rustc 1.99.1, release profile with
thin LTO. Synthetic score: 120 measures, 1 part, 520 notes, 208,287 bytes of
MusicXML. Medians of 20 iterations after one warmup:

| Stage | Median | Output |
| --- | ---: | ---: |
| MusicXML import | 2.956 ms | 520 notes |
| Engrave layout | 0.546 ms | 15 pages, 60 systems, 939 symbols |
| Render SVG | 7.745 ms | 904,935 bytes |
| Render PDF | 10.455 ms | 404,463 bytes |
| Render MIDI | 0.044 ms | 4,726 bytes |
| Full pipeline | 21.746 ms | — |

No benchmark number is stored as a test golden value. The harness prints its
own machine-dependent results.

## Verification log

| Check | Result |
| --- | --- |
| `cargo build --workspace --release` | pass, zero warnings |
| `cargo test --workspace` | 293 pass |
| `cargo test --workspace --release` | 293 pass |
| `cargo run --release -p cadence-fuzz 1000` | 0 panics |
| `cargo run --release -p cadence-bench` | table above |
| `python3 tests/browser/studio_playwright.py` | pass against release binary |
| `pdfinfo examples/demo.pdf` | PDF 1.4, 2 pages, A4 595.276 × 841.89 pt |
| Golden export hashes | match |
| 300-layout collision check | zero collisions |
| 100-fixture round-trip | zero lost notes |

## Documented gaps

These are real and tracked in [LIMITATIONS.md](LIMITATIONS.md):

- Tuplets, grace/cue notes, microtones, ornaments and multiple staves per part
  are rejected rather than approximated.
- Transposing instruments use written pitch, including in MIDI.
- Repeat playback/unfolding is not performed; repeats are engraved only.
- Text uses an original uppercase 5×7 bitmap font; unknown characters are
  boxes. This is the price of no external fonts and byte-stable output.
- Cross-system/cross-page slurs and ties are not drawn (same-system curves are).
- Automatic beaming is a deterministic heuristic, not publisher parity.
- The Studio is single-user, loopback-only, with a single in-memory document
  and 32-step undo/redo. There is no database, CRDT, collaboration or file
  browser.
- Playwright is a developer tool; the Rust workspace has no Python dependency.
- Coverage is line/region/function only; no branch coverage report is produced.

No result in this report is fabricated: every number comes from a command run
against the committed workspace.
