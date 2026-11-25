# Fuzz harness

`cadence-fuzz` is a development tool, not part of the product. It mutates
`examples/demo.musicxml` deterministically and verifies that parsing, layout and
export never panic on hostile input.

```sh
cargo run --release -p cadence-fuzz        # 1,000 cases
cargo run --release -p cadence-fuzz 5000   # more cases
cargo test -p cadence-fuzz                 # the required 1,000-case test
```

## Mutation families

- Prefix truncation at arbitrary character boundaries.
- Insertion of `<`, `>`, `&`, quotes, processing instructions, comments,
  CDATA markers, unknown entities, DOCTYPEs and control characters.
- Random byte-range deletion and chunk duplication.
- Deep element nesting beyond the parser depth limit.
- Attribute bombs with hundreds of attributes and long values.
- Pure ASCII soup.
- System-entity DOCTYPE (rejected before any URL is opened).
- Closing-tag corruption, numeric-value corruption and tuplet injection.

All slices are made on UTF-8 character boundaries so the harness itself cannot
panic. The runner uses `catch_unwind`, reports the first panic indices and
counts accepted/rejected documents. Accepted documents are also engraved and
exported inside `catch_unwind`.

The workspace test requires:

- 1,000 cases executed,
- zero panics,
- at least 900 clean rejections.

The exact counts are printed by the binary and recorded in FINAL_REPORT.md.
