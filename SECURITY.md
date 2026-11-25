# Security

Cadence is a local, single-user tool. Its threat model is a loopback web page
and a score parser that may receive untrusted MusicXML.

## Network surface

- Binds only `127.0.0.1`. There is no option to bind another interface.
- A random 256-bit token is generated from `/dev/urandom` (with a deterministic
  fallback only if the OS source is unavailable).
- The token is required for the page and every API route. It is accepted as a
  query parameter for the initial navigation, then stored in an `HttpOnly`,
  `SameSite=Strict` cookie.
- All responses include `X-Content-Type-Options: nosniff` and
  `Referrer-Policy: no-referrer`.
- The page CSP is `default-src 'self'`, with `script-src 'self'`,
  `connect-src 'self'`, `frame-ancestors 'none'`, `base-uri 'none'` and
  `form-action 'none'`. There are no external scripts, styles, fonts, images or
  CDNs.
- The server does not implement CORS, TLS, remote access, WebSockets, chunked
  uploads or plugins.

## Resource limits

| Resource | Limit |
| --- | --- |
| HTTP headers | 32 KiB, 128 headers |
| HTTP body | 8 MiB |
| Concurrent sockets | 64 |
| SSE subscribers | 32 |
| XML input | 8 MiB |
| XML depth | 64 |
| XML nodes | 500,000 |
| JSON input | 1 MiB |
| JSON depth | 64 |
| Score parts | 64 |
| Measures per part | 10,000 |
| Events per score | 1,000,000 |
| Notes per chord | 24 |
| Lyric characters | 512 |

## Parser safety

- No `unsafe` code in the workspace.
- No external crates, so no transitive C/FFI attack surface.
- The XML parser rejects DOCTYPE declarations, external entities and unknown
  entities before any URL or file could be opened.
- All recursion is depth-limited.
- All numeric parsing uses checked integer or finite floating-point parsing.
- Malformed input is returned as `cadence_core::Error`; the fuzz suite runs
  1,000 hostile mutations and requires zero panics.

## Local data

The document lives in memory. The server does not read user files at runtime
and does not write files; exports are produced as HTTP responses and saved by
the browser. No telemetry, analytics, remote fonts or network requests are
made by the binary or the UI.
