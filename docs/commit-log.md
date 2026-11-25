# Commit log

Cadence was built as a sequence of vertical slices. Every commit builds and the
workspace tests pass at every commit; the repository is deliberately free of CI
workflows. The authoritative list is:

```sh
git log --oneline --decorate
```

## Milestones

| Phase | Scope | Representative commits |
| --- | --- | --- |
| Scaffold | workspace, crates, docs skeleton, dev harnesses | `scaffold:` |
| Core model | pitch, duration, marks, attributes, measures, parts, score validation | `core:` |
| MusicXML | bounded XML parser/writer, import/export, fixtures | `import:`, `fixtures:` |
| Engraving | spacing, beams, collision-free layout, pagination | `engrave:` |
| Export | SVG, PDF, MIDI | `export:` |
| Studio | JSON, document editing, HTTP/SSE, embedded UI | `studio:` |
| Quality | fuzz, benchmarks, Playwright, test suites | `test:`, `quality:` |
| Hardening | limits, auth, malformed input, redo/parts/metadata | `fix:`, `test:` |
| Documentation | architecture, subset, API, testing, security, report | `docs:` |

## Slice policy

- A slice adds one coherent capability or one acceptance area, not a mixed bag.
- Tests land with the code they cover; no failing commits.
- Golden export hashes are updated only with an intentional review of output.
- Documentation that states a limit is committed with the limitation, not later.
- No commit adds a dependency, CI workflow, vendored asset or external font.

## Reproducing the history

The repository has at least 90 commits at hand-off. The latest numbers are
recorded in [FINAL_REPORT.md](../FINAL_REPORT.md). To see the full sequence:

```sh
git rev-list --count HEAD
git log --oneline --reverse
```
