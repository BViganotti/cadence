# Design decisions

## Standard library only

No crates, no vendored code, no CDN. This keeps the supply chain empty and the
binary self-contained. The cost is that XML, JSON, HTTP, PDF, SVG, MIDI and the
font are original code. That cost is paid once and covered by tests.

## Strict subset over silent loss

MusicXML is enormous. The importer rejects constructs that would change
musical meaning (tuplets, grace notes, multiple staves, composite meters,
inconsistent durations). Presentational hints are ignored because Cadence
engraves from the score model. The subset is documented and tested; it is not
presented as full MusicXML support.

## Resolved measure attributes

Every measure stores key, time, clef and divisions resolved from the preceding
measure. This removes backward searches from engraving and export and makes a
measure independently inspectable. Changes are still marked so the exporter can
emit them only when needed.

## Event coordinates, not opaque IDs

The Studio addresses notes by `(part, measure, voice, event, note)` indexes.
These are stable for a single document version and avoid a parallel ID
allocation layer. Edits validate and re-resolve after every change.

## Column spacing shared by voices

Horizontal positions come from a tick boundary set shared by all voices. This
keeps simultaneous events aligned and makes horizontal collisions structurally
impossible inside an event slot. Multi-voice lanes reserve extra width rather
than moving glyphs after the fact.

## Deterministic glyphs, not host fonts

Text is a 5×7 bitmap font emitted as rectangles; music glyphs are vector paths.
No host font is referenced, so SVG/PDF bytes are identical across machines.
Lowercase maps to uppercase and unknown characters render as boxes. The
tradeoff is visual quality, documented in LIMITATIONS.md.

## Original PDF writer

The PDF writer emits a minimal 1.4 file with a catalog, pages tree, page
objects, uncompressed content streams and an exact xref table. Every glyph is a
path or filled rectangle. No timestamps or font objects are embedded. Output is
validated structurally and with `pdfinfo` when available.

## One document, bounded history

The Studio is single-user and single-document. A `Mutex<Document>` owns the
score, edits are validated before commit, and undo/redo history is capped at 32
snapshots. There is no database, CRDT, collaboration protocol or crash
analysis.

## Loopback token, not accounts

A random token is generated at launch, placed in the initial URL, and exchanged
for an `HttpOnly` cookie. API requests need the token through a cookie or
header. CSP blocks external resources. There is no TLS, user account system or
remote binding because the product is explicitly local.

## Bounded parsing everywhere

Every parser has byte, depth and node limits. DTDs and external entities are
rejected. The fuzz suite mutates real MusicXML 1,000 ways and requires zero
panics. Soundness beats accepting another malformed file.

## Tests as the release gate

No CI workflows. The gate is `cargo build --workspace --release`,
`cargo test --workspace` in debug and release, the fuzz binary, the benchmark
binary and the Playwright Studio flow. Golden hashes pin export bytes so
accidental drift is caught.
