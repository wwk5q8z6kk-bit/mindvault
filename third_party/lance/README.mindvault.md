# Vendored lance 0.19.2 (MindVault patch)

Patched `src/lib.rs` with `#![recursion_limit = "512"]` so rustc can compile
deeply nested async layouts in `lance::index` (query depth overflow on current stable).

Wired via workspace `[patch.crates-io]` in the root `Cargo.toml`.
Do not upgrade this tree casually — re-vendor from crates.io and re-apply the attribute.
