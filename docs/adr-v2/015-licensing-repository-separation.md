# ADR 015: Licensing and Repository Separation

Status: proposed

## Problem

The owner may rename, make private, relicense, or redesign MindVault while preserving public
obligations, contributor rights, third-party notices, and separate-product boundaries.

## Current state

The public repository/root is Apache-2.0 at the recorded commit, while Tauri/Python declare MIT and
one workspace crate lacks license metadata. Git authorship is largely generic. DevX, Meridian,
CodePark, and DevX Runtime are explicitly separate products.

## Alternatives

1. keep one public Apache repository;
2. private future development retaining published history;
3. dual/commercial license with contributor consent;
4. combine products into a monorepo;
5. separate repositories with versioned shared packages.

## Measurements required

Contributor/rightsholder inventory, dependency obligations, package/release coupling, shared-code
change frequency, CI/release cost, and legal review.

## Security implications

Repository separation reduces credential/data/config bleed but requires signed packages and
dependency-update discipline.

## Privacy implications

Private repositories do not make committed personal data safe; history scanning and access controls
remain required.

## Migration implications

Preserve public history/license, correct package metadata/notices, obtain relicense consent where
needed, and extract shared code only through versioned licensed packages.

## Chosen direction

Preserve the exact public Apache-2.0 boundary. Future MindVault work may be private or renamed only
without obscuring published obligations. Keep DevX, Meridian, CodePark, and DevX Runtime in separate
repositories/products; share code through explicit versioned packages.

## Rejected alternatives

History rewriting cannot erase licenses. Unilateral relicense is invalid without rights. A product
monorepo violates the required boundary and increases accidental coupling.

## Reversal path

Publish or re-open future code under a rights-compatible license; repository history and package
provenance remain independently traceable.

## Acceptance criteria

Verified rightsholder list, consistent first-party package licenses, complete notices/SBOM, no
unknown blockers, preserved public commit, documented shared-package contracts, and legal approval
before relicense.
