# Knowledge Workspace Contract Fixtures

These files are acceptance inputs and expected outputs for the file-first
workspace design. They are test inputs, not runtime data or active migrations.

## Portable paths

`portable-path-cases.json` is consumed directly by the `mv-core` path-policy
test. It proves that ordinary portable paths pass, safe-but-nonportable names
remain readable, NFC normalization is deterministic, and traversal or reserved
MindVault paths are rejected.

## Byte preservation

`markdown-preservation.md` combines YAML comments and scalar styles, Unicode,
Obsidian extensions, Markdown links, HTML, math, tables, footnotes, and fenced
content.

Accepted invariant:

- source SHA-256:
  `628e2dbc62f79ceeefa10e4564f56dd06d3da8ee494ee08905ec5d2717da1977`;
- read-only ingest, projection, indexing, tree refresh, and no-op rendering
  leave the SHA-256 unchanged;
- unsupported syntax produces diagnostics without rewriting the file.

Line-ending and BOM variants must be generated in tests from this source so the
repository does not rely on Git preserving those variants:

- UTF-8 with BOM;
- CRLF without BOM;
- CRLF with BOM.

Each generated variant must also remain byte-identical after a no-op cycle.

## Legacy migration

- `legacy-nodes.json` is the immutable database-source fixture.
- `expected-migration-plan.json` is the accepted deterministic dry-run plan.
- `migration-expected/` contains the exact staged Markdown bytes.
- `migration-report.schema.json` is the report contract.
- `expected-migration-report.json` is a report instance that must validate
  against the schema.

The expected plan file SHA-256 is
`42a44eca2822e7e4dc96e74f908caf7128ea6f0507f8cc4f31101da8dc9b2a9b`.
Apply must reject any plan whose exact bytes do not match that hash.

The fixture proves:

- ordinary title preservation;
- deterministic duplicate-title suffixing;
- reserved-device-name handling;
- source-node-to-path mapping outside document frontmatter;
- exact staged-byte verification;
- explicit warnings without silent data loss.

## Runtime test placement

Rust tests may read these fixtures directly or copy them into a temporary
directory. Tests must not mutate the source fixture directory.
