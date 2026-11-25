# Cadence — Implementation Plan

Status: ready for one-shot implementation.

## Mission

Cadence is a local browser music notation and engraving workbench: import
MusicXML, edit a score, engrave it with collision-free spacing, and export a
publishable PDF/SVG plus MIDI note events. It is a typesetting engine, not a
sequencer and not a DAW.

## Product shape

- `cadence-core`: score model, durations, voices, beams, slurs, ties, lyrics.
- `cadence-import`: MusicXML parser and writer.
- `cadence-engrave`: spacing, beaming, collisions, page layout, parts.
- `cadence-export`: PDF and SVG writers, MIDI event export.
- `cadence-studio`: local loopback HTTP/SSE Studio with embedded assets.
- `examples/`: scores and imported fixtures.
- `tests/`, `fuzz/`, `bench/`.

The user-facing product is the Studio and the library. A CLI may exist only for
tests, benchmarks and batch export.

## Scope

### In v1
- Pitch/duration/voice/measure model, chords, rests, accidentals, ties, slurs.
- Lyrics, articulations, dynamics, repeats, endings, tempo markings.
- MusicXML import/export for the supported subset.
- Automatic engraving: horizontal spacing, beams, stem directions, collision
  avoidance, system/page breaks, headers/footers.
- Multi-part scores and part extraction.
- PDF and SVG export from the same layout model.
- MIDI note-event export.
- Browser editor: note entry, score canvas, layout preview, part selector,
  export buttons.
- Deterministic layout and byte-stable exports for fixed inputs.

### Out of v1
- Playback, audio, synthesis, DAW features.
- Full MusicXML (all extensions), MusicBrainz, real-time collaboration.
- Databases, CRDTs, crash analysis, observability.

## Architecture

```
cadence/
  Cargo.toml
  crates/
    cadence-core/
      src/score/        # pitches, durations, voices, measures
      src/notation/     # beams, slurs, ties, articulations, lyrics
    cadence-import/
      src/musicxml.rs
      src/midi_out.rs
    cadence-engrave/
      src/spacing.rs
      src/beaming.rs
      src/collision.rs
      src/pagination.rs
    cadence-export/
      src/pdf.rs
      src/svg.rs
    cadence-studio/
      src/server.rs
      src/json.rs
      src/assets.rs
  assets/
  examples/
  tests/ fuzz/ bench/
```

Rust std only. Own XML parser/writer, JSON, HTTP/SSE, PDF writer and UI assets.

## LOC budget (target 13–15k)

| Module | LOC |
|---|---:|
| Score/notation model | 2,200 |
| MusicXML import/export | 2,400 |
| Engraving/spacing/beaming | 2,500 |
| Collision/pagination/parts | 1,600 |
| PDF/SVG export | 1,800 |
| MIDI export | 600 |
| Studio server/API | 1,000 |
| UI assets | 1,500 |
| Tests/fuzz/bench | 1,500 |

## Milestones and commit ranges

### P0 Scaffold (commits 1–8)
Cargo workspace, test harness, README, LICENSE, Makefile, no CI, no crates.

### P1 Score model (commits 9–22)
Pitch/duration → measures/voices → chords/rests → accidentals → ties/slurs →
lyrics/articulations → invariants → fixtures.

### P2 MusicXML (commits 23–38)
XML parser → score mapping → export → round-trip → 100 fixtures → malformed XML
fuzz.

### P3 Engraving (commits 39–56)
Horizontal spacing → beaming → stem directions → collisions → systems/pages →
parts → deterministic layout → 300 generated scores.

### P4 Export (commits 57–68)
SVG → PDF writer → fonts/positions → MIDI events → byte-stable exports →
open/render checks.

### P5 Studio (commits 69–82)
HTTP/JSON/SSE → embedded assets → score canvas → note entry → part selector →
layout preview → export buttons.

### P6 Hardening (commits 83–96)
1,000 malformed MusicXML fuzz → determinism → performance → docs →
FINAL_REPORT.

## Testing

- 100 MusicXML fixtures round-trip without lost notes.
- 300 generated scores have no overlapping symbols.
- PDFs open cleanly and match SVG layout bounds.
- MIDI note events match the score.
- Layout is deterministic and exports are byte-stable.
- 1,000 malformed XML files fail cleanly.
- Studio Playwright: import, edit, preview, export.

## Acceptance criteria

- Zero warnings; tests pass.
- Round-trip preserves notes, durations, voices, lyrics and ties.
- No overlapping symbols on generated scores.
- PDF/SVG exports deterministic and visually aligned.
- MIDI events match the score.
- Studio runs the full flow.
- No external crates; no CI; no CLI-only product.

## Risks and fallbacks

- Full MusicXML is huge: document the supported subset and reject the rest
  cleanly.
- PDF typography: use a built-in deterministic glyph layout; do not rely on
  system fonts for byte stability.
- Engraving perfection: prioritize collision-free, deterministic output over
  publisher parity.
