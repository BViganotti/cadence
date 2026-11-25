# Architecture

Cadence is a Cargo workspace with five libraries, one product binary, and two
development harnesses.

```
MusicXML ──► cadence-import ──► cadence-core ──► cadence-engrave ──► cadence-export
                                     │                  │                 │
                                     └──────────► cadence-studio ◄────────┘
                                                        │
                                                   browser (loopback)
```

## Data flow

1. `cadence-import::musicxml` parses MusicXML into `cadence_core::Score`.
2. `cadence-studio` edits the score in memory (single user, single document).
3. `cadence-engrave::engrave` turns the score into a page/system/event layout.
4. `cadence-export` renders that layout to SVG or PDF, and renders MIDI events
   directly from the score.
5. The studio sends the same layout to the browser as inline SVG and serves
   byte-identical exports.

## Invariants

- Event durations are written durations; tuplets are not supported.
- A measure resolves attributes (key, time, clef, divisions, staff count) from
  the preceding measure, so engraving never has to search backwards.
- Slur marks are stored on notes and paired during engraving. Ties are stored
  as start/stop flags.
- Event coordinates are stable: measure, voice and index. The studio uses those
  coordinates for edits; they are not persisted as opaque IDs.
- Layout geometry is in PostScript points and never depends on wall clock,
  environment or filesystem state.
- All parsers reject over-limit input with `cadence_core::Error`.

## Determinism

`Score` and `Layout` contain no hash maps with iteration-order dependence.
Engraving uses `Vec` order and deterministic numeric comparisons. Exporters
format floating point with fixed precision and emit objects in fixed order.
There are no timestamps in SVG, PDF or MIDI output.

## Threading

The Studio owns one document behind a `Mutex`. Each HTTP connection runs in its
own thread with bounded request headers and body size. SSE subscribers receive
version notifications from a bounded list; slow clients are dropped instead of
growing memory without limit.
