# Product Constitution

## Purpose

MindVault is a sovereign, local-first context platform through which a human and authorized AI
systems share evidence-backed, temporally accurate, permission-controlled memory without
surrendering data ownership or permitting silent canonical changes.

## Constitutional invariants

1. Human-authored knowledge remains readable and exportable.
2. Original evidence is immutable, addressable, and preserved.
3. SQLite and human-readable files are canonical.
4. Full-text, vector, and graph indexes are derived and rebuildable.
5. AI cannot write canonical knowledge directly.
6. AI changes are typed proposals with evidence, policy evaluation, visible diffs, audit, approval,
   and rollback.
7. Core use works locally without a cloud account.
8. Every cloud disclosure is explicit, classified, minimized, and policy-controlled.
9. Connectors and agents receive least privilege, bounded scope, and revocable credentials.
10. A rewrite requires measured product or operational benefit; language preference is not enough.
11. C++ requires an apples-to-apples benchmark demonstrating material advantage.
12. Existing work and data survive upgrades, reversals, renames, and repository changes.
13. MindVault does not absorb DevX, Meridian, CodePark, or DevX Runtime; product and repository
    boundaries remain explicit.

## Authority order

When requirements conflict:

1. user data safety and explicit user intent;
2. constitutional invariants;
3. privacy and security contracts;
4. published schema/API compatibility;
5. architecture decisions;
6. implementation convenience.

## Change protocol

Every canonical mutation identifies the actor, authority, source evidence, previous version,
proposed version, decision, and rollback reference. Imported human files may update their mirrored
records only through deterministic ingestion rules that preserve the original bytes and provenance.
AI enrichment remains derived until a human or explicit policy approves a proposal.

## Release gate

A release is not constitutionally compliant unless canonical data can be backed up, restored,
exported, and re-indexed; proposal and audit semantics are tested; cloud disclosures are visible;
and migrations preserve a verified pre-migration recovery point.
