# Supported MusicXML subset

Cadence reads and writes MusicXML 4.0 **score-partwise** documents. The goal is
a well-defined, sound subset rather than silent partial support. Constructs that
would change the musical meaning are rejected with an error; purely
presentational hints that Cadence computes itself are ignored.

## Imported and exported

| Area | Supported |
| --- | --- |
| Root | `score-partwise` |
| Metadata | `work/work-title`, `movement-title`, `identification/creator` (composer, lyricist, poet, arranger), `rights`, `source`, a fallback `credit-words` title |
| Parts | `part-list/score-part`, `part-name`, `part-abbreviation`, one `midi-instrument` with program and channel |
| Attributes | `divisions`, `key/fifths`, `key/mode`, simple `time` (one beats/beat-type pair), common/cut symbols, one `clef` |
| Measures | `number`, notes, rests, directions, barlines, print system/page breaks, sound tempo |
| Pitches | `step`, `alter` (-2..2), `octave`; written pitch only |
| Durations | whole through 128th, zero to three dots |
| Events | single notes, chords (`<chord/>`), rests, whole-measure rests |
| Voices | up to 16 voices per measure, `backup` and `forward` stream positioning |
| Accidentals | natural, sharp, flat, double sharp, double flat |
| Ties | `tie` and `notations/tied` start/stop |
| Slurs | `notations/slur` start/stop/continue with numbers |
| Beams | `beam` begin/continue/end, forward/backward hooks |
| Articulations | accent, staccato, staccatissimo, tenuto, strong-accent, fermata, breath-mark, up/down bow, harmonic, open-string, pizzicato, snap-pizzicato |
| Dynamics | ppp…fff, sf, sfp, fp, rf, rfz, sfz, sffz, fz |
| Lyrics | numbered lyric lines, syllabic values, extension, up to eight lines and 512 characters |
| Directions | dynamics, metronome, words/rehearsal, sound tempo, offsets |
| Repeats | forward/backward, 2–9 times |
| Endings | start/stop/discontinue with number |
| Tempo | measure-level metronome or sound tempo |

## Rejected (clean error, no partial import)

- `score-timewise` and any other root.
- Tuplets (`time-modification`, `notations/tuplet`) and grace/cue notes.
- Multiple staves per part (`staves > 1`, multiple clefs). Each part is one
  staff in v1.
- Composite/additive time signatures and dotted metronome units.
- Unknown dynamics, articulations, note types, accidental names, beam values,
  bar styles and direction types that would be lost.
- Overfull measures, notes whose written type and `<duration>` disagree,
  chord durations that disagree, and unrepresentable tick gaps.
- Any document with a DOCTYPE, unknown entity, or malformed tag structure.

## Ignored (presentation only)

Stem direction, notehead shape, default x/y, print spacing, page layout,
instrument sounds, MIDI velocity, technical fingering, harmony symbols,
figured bass, and expressive directions outside the engraved subset (wedge,
pedal, octave shift, segno, coda, …). Cadence engraves these itself or does not
claim them.

## Round-trip guarantees

For documents in the supported subset:

- notes, chords, pitches, written durations, dots, voices and measures survive;
- ties, slurs, beams, articulations, lyrics, dynamics and accidentals survive;
- key/time/clef changes, repeats, endings, tempo and barline styles survive;
- part names, abbreviations, MIDI program/channel and metadata survive;
- export is deterministic and importing an export yields identical notation.

This is verified against 100 fixtures in `examples/fixtures` by
`tests/fixtures.rs`, including two-part, multi-voice and mid-score key change
cases.

## Notable limitations

Transposing instruments are imported at written pitch; MIDI export uses written
pitch. Grace notes, tuplets, cross-staff notation, percussion notation,
microtones and compressed MusicXML (`.mxl`) are outside v1. See
[LIMITATIONS.md](../LIMITATIONS.md).
