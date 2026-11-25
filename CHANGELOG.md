# Changelog

All notable changes to Cadence are recorded here. Every entry corresponds to a
committed vertical slice in the repository; no dates are generated at runtime.

## [0.1.0] - 2026-11-25

### Scaffold

- Cargo workspace with zero external dependencies; root product binary.
- Original embedded assets, no CDN or network runtime.
- MIT license, README, Makefile, documentation skeleton.
- `fuzz/`, `bench/` and `fixtures/` development members.

### Score model

- Scientific pitch notation (C4 = MIDI 60), spelling, range validation and
  MIDI conversion.
- Written durations from whole to 128th with 0–3 dots, exact tick conversion.
- Accidentals, articulations, dynamics, lyrics, ties, slurs and beam marks.
- Key, time, clef and resolved measure attributes; pickup-aware timelines.
- Notes, chords, rests, voices, measures, repeats, endings, tempo and parts.
- Structural validation plus musical inspection (overfull, underfull, ties,
  duplicate IDs, measure numbering).
- Slur/tie resolution, beam-run analysis and lyric line counting.
- Two-part demonstration score with lyrics, ties, slurs, dynamics, chords,
  key change and multi-voice rests.

### MusicXML

- Original bounded XML parser: entities, CDATA, namespaces, comments, strict
  limits and DOCTYPE/external-entity rejection.
- Deterministic pretty writer.
- Import mapping for notes/chords/rests, voices, backup/forward, ties, slurs,
  beams, lyrics, articulations, dynamics, tempo, repeats, endings, barlines and
  print breaks.
- Strict errors for tuplets, grace/cue notes, multiple staves, composite time
  signatures and inconsistent durations.
- Export of the supported subset with stable element order.
- 100 deterministic fixtures; two exports per fixture are byte-identical and
  re-import without losing notation.

### Engraving

- Shared tick columns across voices with duration and glyph requirements.
- Clef/key/time prefixes, key/time change system breaks and explicit breaks.
- Automatic and explicit beaming, stem directions, flags, beams and ledger
  lines.
- Chord second stacking, accidentals with measure state, dots and lyrics lanes.
- Dynamic/articulation placement, rehearsal/tempo/measure headings.
- Deterministic system/page pagination with headers and page numbers.
- Collision-visible symbol list; 300 generated scores required to have zero
  overlapping ink.
- Part extraction layouts and Studio hit regions.

### Export

- Self-contained SVG with outlined text, stable formatting and optional hit
  rectangles.
- Original vector PDF 1.4 writer with exact xref offsets, stream lengths and
  no external fonts/timestamps.
- Standard MIDI format 1 with conductor track, tempo/time/key meta, tracks per
  part and merged ties.
- Byte-stability tests for all three formats; optional `pdfinfo` open test.

### Studio

- Bounded JSON parser/serializer with depth, node and byte limits.
- In-memory document with bounded undo, validation rollback and import/reset.
- Full edit command set (notes, rests, chords, pitch, duration, dynamics,
  lyrics, tempo, key, transpose, measures, part selection).
- Loopback HTTP/SSE server with random token, strict cookie, CSP, limits and
  connection caps.
- Embedded original HTML/CSS/JS: part/full-score preview, note entry, inspector,
  diagnostics, import, undo and four download formats.
- Playwright acceptance test covering import, edit, preview and export.

### Hardening

- 1,000 deterministic malformed-input mutations with zero panics.
- Honest dependency-free benchmark harness.
- Documentation for architecture, subset, engraving, API, testing, security and
  limitations.
- Release report with LOC, commits, tests, coverage, benchmarks and gaps.
