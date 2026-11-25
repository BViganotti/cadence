# Examples

- `demo.musicxml` — the two-part demonstration score used by the Studio,
  generated from `Score::demo()`.
- `fixtures/fixture_000.musicxml` … `fixture_099.musicxml` — deterministic
  MusicXML fixtures covering seven key signatures, five time signatures, one
  and two parts, chords, rests, ties, slurs, lyrics, dynamics, articulations,
  accidentals, repeats, endings, tempo and part extraction.

Regenerate them with:

```sh
cargo run --release -p cadence-fixtures
```

Every fixture is imported, validated, exported twice and re-imported by
`tests/fixtures.rs`. Byte stability is checked as part of that test.
