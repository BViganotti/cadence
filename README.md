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
Known gaps are tracked in [LIMITATIONS.md](LIMITATIONS.md).

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

## License

MIT. See [LICENSE](LICENSE).
