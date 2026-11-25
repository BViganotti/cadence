# Cadence

Cadence is a local browser music notation and engraving workbench. Import
MusicXML, edit a score, engrave it with deterministic collision-free spacing,
and export publishable SVG/PDF plus MIDI note events.

Cadence is a typesetting engine, not a sequencer and not a DAW. Everything runs
on your machine: the binary embeds the studio UI and serves it on
`127.0.0.1` only. There are no external crates, no runtime downloads, no CDN
assets, no external fonts, and no telemetry.

## Quick start

```sh
cargo run --release -p cadence
```

A browser opens at a loopback URL with a random access token. If it does not,
copy the printed URL. Stop the server with `Ctrl-C` (the only terminal
interaction the launcher needs).

For automated checks:

```sh
cargo test --workspace
cargo build --release
```

## Studio

The Studio is a single-page local workbench:

- import MusicXML with a file picker (the document is replaced atomically);
- switch between parts or preview the full score;
- click any notehead/rest to select it, then change pitch, duration, dots,
  dynamic, lyric, add a chord note or delete the event;
- add, duplicate or delete measures; add or delete parts;
- set title, subtitle, composer, tempo and key; transpose one or all parts;
- edit individual chord notes and their lyrics;
- see note/chord/rest counts and live diagnostics;
- zoom, page through the preview, and use keyboard shortcuts;
- download SVG, PDF, MIDI or MusicXML for the selected part or the full score;
- receive live updates over SSE and undo/redo the last 32 edits.

The UI is embedded with `include_str!`; the server is loopback-only, token
protected, CSP-restricted and never reads the filesystem at runtime. The
[Studio API guide](docs/studio-api.md) lists every endpoint and edit command.

## What works

- **Score model.** Pitches, durations, dots, voices, chords, rests, ties,
  slurs, beams, lyrics, articulations, dynamics, repeats, endings, tempo, key,
  time, clefs and multiple parts.
- **MusicXML.** Import and export of a documented subset with bounded parsing
  and clean errors on malformed input.
- **Engraving.** Duration-aware horizontal spacing, automatic and explicit
  beaming, stem directions, chord notehead stacking, accidentals, dots,
  ledger lines, lyrics, dynamics, slurs, ties, systems, pages, headers and
  footers. Layout is deterministic and collision checked.
- **Export.** Original SVG writer, original vector PDF writer (all glyphs are
  paths, so output is byte-stable across systems), and Standard MIDI File
  export.
- **Studio.** Import, part selection, note entry and editing, live layout
  preview, diagnostics and export buttons in the browser.

The supported MusicXML subset is listed in [docs/musicxml-subset.md](docs/musicxml-subset.md).
Known gaps are tracked in [LIMITATIONS.md](LIMITATIONS.md). The layout model is
documented in [docs/engraving.md](docs/engraving.md).

## Tests and benchmarks

```sh
cargo test --workspace
cargo test --workspace --release
cargo run --release -p cadence-fuzz        # 1,000 malformed cases
cargo run --release -p cadence-bench       # honest wall-clock medians
python3 tests/browser/studio_playwright.py # import/edit/preview/export
```

The fixture suite imports, exports twice and re-imports 100 checked-in
MusicXML scores. The layout suite engraves 300 generated scores and requires
zero overlapping ink symbols. The fuzz suite requires zero panics across 1,000
hostile mutations. There is no CI workflow by design.

## Workspace

| Crate | Purpose |
| --- | --- |
| `cadence-core` | score model, notation invariants |
| `cadence-import` | original XML parser, MusicXML reader/writer |
| `cadence-engrave` | spacing, beaming, collisions, pagination |
| `cadence-export` | deterministic SVG, PDF and MIDI |
| `cadence-studio` | loopback HTTP/SSE server and embedded UI |
| `fuzz` | deterministic malformed-input harness |
| `bench` | dependency-free timing harness |

## Design rules

1. Standard library only.
2. Every parser has explicit byte, depth and node limits.
3. Malformed input returns an error; it never panics or aborts.
4. Layout and exports depend only on the score and engraving options.
5. Generated exports contain no timestamps, random values or host paths.

## Documentation

- [Architecture](docs/architecture.md)
- [MusicXML subset](docs/musicxml-subset.md)
- [Engraving model](docs/engraving.md)
- [Studio API](docs/studio-api.md)
- [Testing](docs/testing.md)
- [Design decisions](docs/design-decisions.md)
- [Performance](docs/performance.md)
- [Accessibility](docs/accessibility.md)
- [Release checklist](docs/release-checklist.md)
- [Security](SECURITY.md)
- [Limitations](LIMITATIONS.md)

## License

MIT. See [LICENSE](LICENSE).
