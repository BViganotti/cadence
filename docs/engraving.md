# Engraving model

Cadence engraves from the resolved score model, not from MusicXML presentation
hints. Layout is a pure function of the score and `Options`; it contains no
randomness, timestamps, environment lookups or hash-map iteration order.

## Coordinate system

Layout uses PostScript points with the origin at the top-left of the page and
`y` growing downward. PDF export flips `y` once; SVG uses the layout directly.
A4 is the default (595.276 × 841.89 pt). Staff space defaults to 8 pt, so a
five-line staff is 32 pt tall.

## Horizontal spacing

1. Every event start and end tick in a measure becomes a boundary; voices share
   the boundary set, so simultaneous events stay vertically aligned.
2. Each segment gets `min_event_width + duration_scale × beats^0.58`.
3. A segment is widened to the largest glyph requirement starting there:
   accidentals and left padding, notehead, dots, flags, chord seconds, lyrics,
   dynamics and multi-voice lanes.
4. Lower voices get a reserved horizontal lane so unison/adjacent voices cannot
   collide. Multi-voice rests get an additional lane.
5. The current clef, key signature and time signature occupy a prefix at the
   start of every system.
6. A key/time/clef change starts a new system so the change is always shown at
   a system start. Explicit `<print new-system/new-page>` hints are honored.

A measure is never split. If a single measure is wider than the page, the
engraver keeps it bounded and deterministic rather than looping; extreme text
can overflow the content box and is documented as a limitation.

## Vertical spacing

- Pitch placement uses diatonic staff steps for the measure clef. C4 in treble
  is one ledger line below the staff.
- Chord notes are sorted low to high. Adjacent seconds alternate a horizontal
  offset so notehead boxes do not overlap.
- Stem direction is up for the upper voice and down for lower voices; a single
  voice uses its average pitch relative to the middle line.
- Ledger lines are emitted for every line outside the staff.
- Accidentals are computed against the key and measure accidental state, then
  placed in a left column per chord. Dots are placed after the widest chord
  notehead, staggered for chord notes.
- Lyrics start below the lowest notehead/stem in the measure. Voice 2 lyric
  lines start three lines lower. Dynamics follow the last lyric line.
- Stems, beams, slurs and ties are connectors and may cross ink; they are
  excluded from collision checks. Beams connect stem tips at the group's beam
  line and stack additional levels toward the noteheads.
- Ledger lines, staff lines and barlines are connectors and are also excluded.

## Systems and pages

- Measure widths are shared across parts so staves align.
- Systems break when the next measure would exceed the content width, when an
  explicit system break appears, when attributes change, or when
  `max_measures_per_system` is reached.
- Staff extents are computed from the actual pitches, articulations, lyrics and
  dynamics in the system. The next staff starts below the previous staff's
  extent, so cross-staff collisions are avoided by construction.
- Page capacity is measured from the header on page 1 (title block) and the
  full content box on later pages. Page numbers live in the bottom margin.
- Every page records systems, staff tops, hits and ink symbols.

## Glyphs and fonts

Cadence ships an original 5×7 bitmap font for text and vector paths for clefs,
accidentals, rests, flags, beams, articulations and the common/cut time symbol.
No host font is referenced. Lowercase text maps to uppercase glyphs; unsupported
Unicode renders as a box. This is a deliberate determinism tradeoff documented
in LIMITATIONS.

## Collision checking

`collision::find_collisions` checks every pair of ink symbols on a page:
noteheads, accidentals, dots, rests, lyrics, dynamics, articulations, clefs,
key/time signatures, measure numbers, rehearsal/tempo text, titles and part
names. Connectors are excluded. The workspace test engraves 300 generated
scores (100 fixtures × three transpositions) and requires zero collisions.

## Determinism

- No floating-point formatting is done in the engraver; exporters format with a
  fixed three-decimal helper.
- Element order follows score order: pages, systems, staves, measures, voices,
  events, notes.
- SVG/PDF/MusicXML/MIDI have no timestamps, host paths, random IDs or font
  dependencies.
- Re-engraving the same score produces a structurally equal `Layout`; re-export
  produces identical bytes.

## Part extraction

A part layout is produced by extracting the part into a one-part score and
engraving it. Studio hit regions are remapped to the original part index so
edits target the right part. Full-score preview engraves all parts together.
