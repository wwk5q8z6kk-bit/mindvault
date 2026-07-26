# Dependency and License Audit

## Baseline

- Root license file: Apache-2.0.
- Most Rust workspace packages: Apache-2.0.
- `frontend/src-tauri/Cargo.toml`: MIT.
- `python/pyproject.toml`: MIT.
- `mv-mcp`: no Cargo license metadata in the generated inventory.
- Public repository at baseline: GitHub reports `PUBLIC`; its detected license field is empty,
  while the exact commit's root `LICENSE` text is Apache-2.0.

This inconsistency must be resolved before relicense, private split, binary distribution, or public
claims about the repository-wide license.

## Inventory

The generated CycloneDX 1.6 SBOM at `audit/SBOM.cdx.json` contains 1,843 components and 1,844
dependency relationships. Scanner evidence at `audit/evidence/trivy-licenses.json` covers:

| Ecosystem | Components observed |
|---|---:|
| Root Cargo workspace | 855 |
| Tauri Cargo graph | 455 |
| Frontend pnpm | 223 |
| Python uv | 130 |
| Connector npm | 91 |
| Web npm | 70 |
| Plugin example Cargo | 12 |

Counts overlap across graphs and are not a unique legal-component count.

## License exceptions requiring review

- MPL-2.0 appears for `colored`, `option-ext`, and Tauri transitive CSS/selectors packages.
- `htmlescape` offers Apache/MIT/MPL alternatives; choose a compatible option in notices.
- `r-efi` offers MIT/Apache/LGPL alternatives; select a permissive option rather than treating it as
  an LGPL-only dependency.
- `mv-mcp` has unknown package-license metadata.
- npm/Python packages with missing or non-SPDX metadata must be resolved from their source
  distributions before release.

MPL dependencies are generally file-level copyleft, but exact distribution obligations and any
modified MPL files require legal review. This document is engineering evidence, not legal advice.

## Authorship, copied code, generated code

Git records 138 commits: 133 use a generic contributor identity and five use a GitHub identity.
This is inadequate evidence for a clean relicense. No contributor agreement or provenance ledger
was found. Generated protobuf/OpenAPI/bundled frontend assets must carry generator, input, version,
and license metadata. Large vendored/generated files and copied examples need source-specific
attribution review; `ATTRIBUTIONS.md` is not complete enough.

## Supply-chain gaps

- no `cargo-deny` policy;
- the timestamped Trivy scan reports 217 lockfile occurrences (203 unique IDs): 3 critical, 56
  high, 114 medium, and 44 low; all 55 unique critical/high advisory IDs now have a call-path
  disposition in
  [VULNERABILITY_REACHABILITY_AND_DISPOSITION.md](VULNERABILITY_REACHABILITY_AND_DISPOSITION.md),
  but no finding has yet been remediated or accepted;
- no provenance/signature/SLSA policy for release artifacts;
- no lock/toolchain policy covering every nested package;
- plugin and connector packages do not have a unified SBOM/signature admission process;
- local model files and extraction binaries are outside the current SBOM.

## Required controls

1. Choose and document the first-party license per package and repository.
2. Populate missing `license`, `repository`, authorship, and notice metadata.
3. Establish contributor provenance/CLA or obtain explicit relicense consent.
4. Generate SPDX/CycloneDX per release, including nested connectors, desktop, Python, tools, and
   model artifacts.
5. Run license allow/deny and vulnerability scans from timestamped databases.
6. Execute the critical/high closure plan: patch or isolate each shipped path, then document any
   residual acceptance with a named owner, rationale, expiry, and verification evidence.
7. Block prohibited/unknown licenses unless reviewed.
8. Include source offers/notices where required.
9. Preserve the public commit and its Apache-2.0 history through any rename or repository split.

## Acceptance

No blocker-level unknown license, complete third-party notices, signed SBOM attached to artifacts,
reproducible dependency resolution, and documented disposition for every copied/generated
component.
