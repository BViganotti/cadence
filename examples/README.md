# Examples

## Demo score

- `demo.musicxml` — two-part demonstration score generated from `Score::demo()`.
- `demo.svg` — the engraved demo, stacked pages, self-contained.
- `demo.pdf` — the same layout as vector PDF (all text outlined).
- `demo.mid` — standard MIDI file with conductor and one track per part.

Regenerate the binary examples with:

```sh
cargo run --release --example export_demo
```

Regenerate the MusicXML demo and fixtures with:

```sh
cargo run --release -p cadence-fixtures
```

## Fixtures

`fixtures/fixture_000.musicxml` … `fixture_099.musicxml` are deterministic
scores covering:

- seven key signatures and five time signatures;
- one- and two-part layouts;
- chords, rests, ties, slurs, lyrics, dynamics, articulations and accidentals;
- repeats, endings, tempo, barline styles and multi-voice measures.

Every fixture is imported, validated, exported twice and re-imported by
`tests/fixtures.rs`; byte stability is checked in the same test. The layout test
transposes each fixture three ways and requires zero collisions across the
resulting 300 scores.

## Notes

The fixtures are generated source, not hand-copied publisher scores. They exist
to exercise the supported subset, not to claim coverage of the whole MusicXML
language. See [../docs/musicxml-subset.md](../docs/musicxml-subset.md).
