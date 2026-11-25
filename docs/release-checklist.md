# Release checklist

Cadence has no CI workflow. Run this list manually before tagging or handing off.

## Build

```sh
cargo build --workspace --release
```

- Must finish with zero warnings.
- Must not fetch external crates (all dependencies are workspace paths).

## Tests

```sh
cargo test --workspace
cargo test --workspace --release
```

- 100+ fixtures round-trip.
- 300 generated layouts have zero overlapping ink.
- All 100 fixtures engrave and export SVG/PDF/MIDI.
- Golden demo hashes match.
- PDF xref/stream tests and optional `pdfinfo` check pass.
- Server auth, limits, SSE and edit tests pass.

## Fuzz and browser

```sh
cargo run --release -p cadence-fuzz 1000
cargo run --release -p cadence-bench
python3 tests/browser/studio_playwright.py
```

- Fuzz: zero panics.
- Benchmark runs and prints medians (no golden numbers).
- Playwright: import, edit, preview, export, keyboard shortcuts, no console
  errors.

## Repository hygiene

- `git status` is clean.
- No `.github/workflows`, `.gitlab-ci.yml` or `.circleci`.
- `LICENSE`, `README.md`, `CHANGELOG.md`, `LIMITATIONS.md`, `SECURITY.md`,
  `CONTRIBUTING.md`, `FINAL_REPORT.md` and `docs/` exist.
- No new external dependency, CDN, vendored asset or host font.
- Commit count and LOC recorded in `FINAL_REPORT.md`.
- Remote is public and pushed, or `PUSH.md` exists.

## Known release risks

- Engraving is deterministic but not publisher-parity.
- Text is a bitmap font; keep expectations documented.
- Cross-system slurs/ties are not drawn.
- Not every MusicXML file is in the supported subset; rejection is intentional.
- Studio is single-user and loopback-only by design.
