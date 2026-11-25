# Limitations

Cadence deliberately ships a sound subset. Everything here is known, tested or
documented; nothing is presented as working when it is not.

## Musical scope

- **Tuplets** are not supported. Importing `time-modification` or
  `notations/tuplet` fails with a clear error instead of approximating the
  rhythm.
- **Grace notes, cue notes, appoggiaturas and tremolo** are not supported.
- **Microtones** and quarter-tone accidentals are not supported; `alter` is
  limited to -2..2.
- **Transposing instruments** are imported and exported at written pitch.
  MIDI export therefore uses written pitch rather than sounding pitch.
- **Multiple staves per part** (piano grand staff, organ, harp) are rejected.
  Model a grand staff as two parts for now.
- **Percussion notation**, tablature, figured bass, chord symbols, harp pedals
  and guitar frames are ignored or rejected depending on whether they affect
  the score.
- **Repeat playback/unfolding** is not performed. Repeats and endings are
  engraved but the MIDI export is a straight reading of the written measures.
- **Ornaments** (trills, turns, mordents) are rejected by import.
- **Articulations outside the listed subset** are rejected rather than dropped.

## Engraving

- Text uses an original 5×7 uppercase bitmap font. Lowercase maps to uppercase
  and unsupported characters render as boxes. This guarantees byte-stable
  output without external fonts but is not typographic-quality text.
- Lyrics are laid out below the lowest note/stem in each measure. Voice 2 uses
  a lane three lyric lines below voice 1. Extremely long lyrics can make a
  measure wider than the page; it is kept bounded but may overflow the content
  box.
- Slurs and ties that cross a system or page break are currently not drawn.
  Within-system slurs/ties are engraved.
- Automatic beaming is a musical heuristic based on the time signature and
  explicit beam marks. It does not attempt publisher-specific beaming rules.
- Beam levels for mixed 16th/8th runs are drawn as contiguous spans; partial
  secondary beams are simplified.
- Clefs are original stylized vector paths, not scanned publisher glyphs.
- Collision checking ignores connectors (stems, beams, slurs, ties, ledger
  lines, staff lines, barlines). Ink symbols are required to be disjoint.
- A single measure is never split across systems. This can leave extra space
  or overflow for pathological text.
- Part names are abbreviated/truncated to ten characters at the left margin.

## Import/export

- Compressed MusicXML (`.mxl`) is not supported; unzip it first.
- `score-timewise` is rejected; only `score-partwise` is accepted.
- Encoding is assumed to be UTF-8. Legacy encodings are rejected.
- DTDs, external entities and unknown entities are rejected.
- Unsupported direction types are ignored when purely expressive, but
  unsupported structural elements are rejected.
- MusicXML round-trip is guaranteed for the documented subset, not for the
  entire MusicXML language.
- MIDI is note-event export only: no audio rendering, no synthesizer, no
  velocity curves beyond written dynamics, no program banks or controllers.

## Studio

- Single user, single document, no collaboration, no file browser, no project
  database. Import replaces the current document; exports download a file.
- Undo is bounded to 32 snapshots. Redo is not implemented.
- The server is loopback-only and has no TLS, user accounts or remote access.
- The browser must support inline SVG, ES2020 and EventSource (current Chrome,
  Edge, Safari or Firefox).
- Playwright tests are a developer tool; the Rust workspace itself has no
  Python or browser dependency.

## Platform

- The launcher attempts `open` (macOS), `xdg-open` (Linux) or `start` (Windows)
  and otherwise prints the URL. It has no terminal UI.
- PDF/SVG output is deterministic on every platform because no host fonts or
  libraries are used.
- `pdfinfo` validation is optional. The structural PDF test always runs; the
  Poppler check is skipped when the tool is absent.
