# Studio API

The Studio is a loopback-only HTTP server with Server-Sent Events. It binds
`127.0.0.1` (never a public interface), uses a random 256-bit token, sets an
`HttpOnly`, `SameSite=Strict` cookie on the initial authenticated page load,
and serves an embedded CSP that blocks external resources.

There is no terminal interaction in normal use: `cadence` starts the server,
opens the browser, and stays resident. `Ctrl-C` stops it.

## Authentication

The launcher prints a URL such as:

```
http://127.0.0.1:49321/?token=5f0c...
```

Opening it validates the token and sets `cadence_token`. Subsequent API
requests may authenticate with any of:

- `Cookie: cadence_token=…`
- `X-Cadence-Token: …`
- `Authorization: Bearer …`
- `?token=…` (used for the initial page load and downloadable exports)

`GET /healthz` is the only unauthenticated endpoint and returns `ok`. Static
assets are served from memory and contain no user data.

## Endpoints

| Method | Path | Purpose |
| --- | --- | --- |
| GET | `/` | Embedded Studio HTML; requires a valid token/cookie |
| GET | `/assets/app.css`, `/assets/app.js`, `/favicon.svg` | Embedded assets |
| GET | `/healthz` | Liveness check |
| GET | `/api/state` | Score summary, parts, diagnostics, version, undo state |
| GET | `/api/event?part&measure&voice&event` | One event with notes, lyrics, dynamic, key and tempo |
| GET | `/api/layout.svg?part=all\|N&hits=1` | Engraved SVG; hit rects carry `data-part/measure/voice/event` |
| GET | `/api/export/svg?part=all\|N` | Download SVG |
| GET | `/api/export/pdf?part=all\|N` | Download vector PDF |
| GET | `/api/export/midi?part=all\|N` | Download MIDI |
| GET | `/api/export/musicxml?part=all\|N` | Download MusicXML |
| GET | `/api/events` | SSE stream (`update` events with `{version}`) |
| POST | `/api/import` | Raw MusicXML body (UTF-8, ≤ 8 MiB) |
| POST | `/api/edit` | JSON edit command (≤ 1 MiB) |
| POST | `/api/undo` | Undo the last edit |
| POST | `/api/reset` | Restore the built-in demo |
| POST | `/api/select` | `{ "part": N }` selection without creating an undo step |

Errors are JSON on API routes:

```json
{"ok":false,"error":"measure is full for that duration"}
```

Status codes: `400` invalid command/operation, `401` missing token,
`403` invalid page token, `404` unknown route, `413` oversized body,
`503` too many connections/subscribers.

## Edit commands

All commands are objects with an `op` string. Coordinates are zero-based;
`voice` is 1-based. Unknown operations are rejected.

| `op` | Fields |
| --- | --- |
| `add_note` | `part, measure, voice, pitch ("C4", "Bb3", "F##2"), duration, dots, accidental?` |
| `add_rest` | `part, measure, voice, duration, dots` |
| `add_chord_note` | `part, measure, voice, event, pitch` |
| `delete_event` | `part, measure, voice, event` |
| `set_pitch` | `part, measure, voice, event, note, pitch` |
| `set_duration` | `part, measure, voice, event, duration, dots` |
| `set_dynamic` | `part, measure, voice, event, dynamic` (`""` removes) |
| `set_lyric` | `part, measure, voice, event, text, syllabic` |
| `set_tempo` | `part, measure, bpm` |
| `set_title` | `title` |
| `set_key` | `part, measure, fifths` (-7..7) |
| `transpose` | `part, semitones` (-128..127) |
| `add_measure` | `part` |
| `delete_measure` | `part, measure` |
| `select_part` | `part` |

Duration names: `whole`, `half`, `quarter`, `eighth`, `16th`, `32nd`, `64th`,
`128th`. Dots: 0–3.

Every edit is validated before it is kept. Overfull measures, out-of-range
pitches, invalid durations and unsupported dynamics are rejected; the document
is restored and no version is published. Successful edits push one bounded undo
snapshot (32 entries).

## SSE

`/api/events` sends an initial `update` event and then one event per document
version. A comment heartbeat every 15 seconds keeps idle connections alive.
The server caps subscribers at 32 and drops disconnected channels during the
next notification. The browser re-renders on each version.

## Limits

- Request headers: 32 KiB, 128 headers.
- Request body: 8 MiB (MusicXML and JSON share the bound; JSON also has its own
  1 MiB parser limit).
- Concurrent sockets: 64.
- Chunked request bodies are rejected.
- No keep-alive: each non-SSE request closes its socket, which keeps the
  implementation small and predictable.

## Security model

Cadence is a single-user local tool. It does not expose remote access,
authentication providers, TLS, CORS or plugins. The token prevents other local
processes from casually reading or editing the score. The CSP blocks external
scripts, styles, images and connections. The server performs no filesystem
reads at runtime and never writes user files except through browser downloads.
