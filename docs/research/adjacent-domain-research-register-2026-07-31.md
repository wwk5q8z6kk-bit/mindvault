# MindVault Next — Repository and Adjacent-Domain Research Register

**Date:** July 31, 2026  
**Purpose:** Trace the primary sources used to broaden the master plan beyond its original examples.  
**Evidence rule:** A source can inspire a pattern, compatibility profile, or benchmark. It does not become a core dependency or establish that MindVault already implements the capability.

## 1. Repository Evidence Reviewed

| Source | Repository fact used | Planning implication |
|---|---|---|
| [INTEROPERABILITY_CONSTITUTION.md](https://raw.githubusercontent.com/wwk5q8z6kk-bit/mindvault/main/INTEROPERABILITY_CONSTITUTION.md) | Ratified law defines a sovereign interoperable context fabric, logically central and physically decentralized; mandates source authority, grants, provenance, portability, no direct extension DB access, and fail-closed behavior | Treat interoperability as constitutional; extend through public contracts and Domain Packs rather than vendor-specific core types |
| [IMPLEMENTATION_BACKLOG.md](https://raw.githubusercontent.com/wwk5q8z6kk-bit/mindvault/main/IMPLEMENTATION_BACKLOG.md) | Evidence-gated backlog distinguishes code presence from verified behavior and identifies current P0 gaps | Expansion work must not displace current truth, authority, reliable-effects, promotion-boundary, and executor work |
| [Repository README](https://github.com/wwk5q8z6kk-bit/mindvault) | Current Rust workspace already contains structured storage, indexes, graph, API surfaces, Tauri/Svelte UI, MCP, connectors, and plugin foundations | Evolve and benchmark the current architecture rather than assuming a clean rewrite |
| ADR 008 — file-first knowledge workspace | Human Markdown and structured SQLite state have distinct canonical roles; indexes are projections | Preserve ordinary files and make Domain Pack state exportable and reconstructable |
| ADR 010 — personal Vault and collaborative Spaces | Current intended model separates private personal state from shared collaboration | All new packs and agents must preserve explicit private-to-shared transfer |
| ADR 012 — governed agent execution graph | Governed schema and tests exist, but no executor currently drives work | Durable execution and an actual Run executor are prerequisite to broad agent use cases |
| Collaborative Spaces baseline | Identifies reliable effects, action envelope, promotion boundary, and isolation contracts | Do not begin with Slack UI breadth; close authority and effect boundaries first |

## 2. Local-First and Malleable Software

| Primary source | Pattern considered | MindVault adaptation |
|---|---|---|
| [Ink & Switch — Local-first software](https://www.inkandswitch.com/local-first/) | Offline operation, user ownership, privacy, longevity, multi-device collaboration | Personal Vault remains usable locally; cloud and federation are optional layers |
| [Ink & Switch — Malleable software](https://www.inkandswitch.com/essay/malleable-software/) | Users reshape tools; gentle slope from user to creator; compose tools rather than sealed apps | Domain Pack Studio, universal views, formulas, workflows, and code escape hatches |
| [Potluck](https://www.inkandswitch.com/potluck/) | Gradual enrichment from ordinary text into interactive structured tools | Let users progressively structure notes and objects rather than requiring up-front schemas |
| [Embark](https://www.inkandswitch.com/embark/) | Dynamic views and formulas over shared data; practical schema-interoperability challenges | Add schema lenses, mapping review, and view formulas without pretending schemas align automatically |
| [Automerge](https://automerge.org/) | Local-first versioned synchronization and conflict-free data structures | Candidate document collaboration backend behind an owned interface; not a global data model |
| [Yjs](https://docs.yjs.dev/) | Shared types and awareness for collaborative applications | Candidate for selected collaborative document objects after measured need and CRDT ADR |

## 3. Interoperability and Agent Protocols

| Primary source | Current fact or pattern | MindVault adaptation |
|---|---|---|
| [MCP 2026-07-28 release](https://blog.modelcontextprotocol.io/posts/2026-07-28/) | Current specification is generally available; stateless core, discovery, task extension, authorization hardening, and SDK updates | Implement versioned bidirectional MCP adapters; do not store protocol transport semantics as the internal domain model |
| [A2A specification](https://a2a-protocol.org/latest/specification/) | Agent Cards, tasks, messages, artifacts, streaming, asynchronous state, and cancellation | Map external agents to internal Agent identities, Work Orders, Runs, artifacts, and grants |
| [OpenAPI](https://spec.openapis.org/oas/latest.html) | Language-independent HTTP interface description and client/tool generation | Public synchronous API contract and alternate-client conformance |
| [AsyncAPI](https://www.asyncapi.com/docs/reference/specification/latest) | Event-driven interface description | Public event/subscription contract |
| [CloudEvents](https://cloudevents.io/) | Transport-neutral event envelope and source/id identity | External event envelope, correlation, causation, schema, and idempotency conventions |
| [Matrix](https://matrix.org/) | Open decentralized communication and federation | Optional communication bridge; never the internal Work/Knowledge/Agent model |
| [Solid](https://solidproject.org/TR/protocol) | User-controlled data pods and permissioned application access | Optional data-node adapter and design influence; not a mandatory storage core |
| [ActivityPub](https://www.w3.org/TR/activitypub/) | Decentralized social client/server and federation | Optional public/community federation profile |

## 4. Policy, Identity, Consent, and Credentials

| Primary source | Pattern considered | MindVault adaptation |
|---|---|---|
| [Open Policy Agent](https://www.openpolicyagent.org/docs) | Domain-agnostic policy-as-code; policy decision separated from enforcement | `PolicyBackend` adapter option behind owned principal/action/resource/context/purpose contracts |
| [Cedar](https://www.cedarpolicy.com/en) | Explicit principal, action, resource, and context authorization model | Influence the internal decision envelope and explainable policy tests |
| [SPIFFE](https://spiffe.io/docs/latest/spiffe-about/overview/) | Short-lived workload identity and federated trust domains | Candidate workload identity profile for Agent Runs, connectors, and services |
| [W3C Verifiable Credentials 2.0](https://www.w3.org/TR/vc-data-model-2.0/) | Issuer–holder–verifier, tamper-evident credentials, selective presentation patterns | Credential Domain Pack/profile; credentials remain scoped claims, not universal truth |
| [User-Managed Access](https://docs.kantarainitiative.org/uma/wg/rec-oauth-uma-grant-2.0.html) | Resource-owner controlled, asynchronous authorization for sharing data/services | Input to consent and purpose-limited cross-node data access |

## 5. Provenance, Lineage, and Authenticity

| Primary source | Pattern considered | MindVault adaptation |
|---|---|---|
| [W3C PROV](https://www.w3.org/TR/prov-overview/) | Entities, activities, agents, derivation, attribution, and interoperable provenance | General Trust Ledger export profile |
| [OpenLineage](https://openlineage.io/docs/) | Dataset, job, run, and extensible-facet lineage | Data/ML pipeline profile generated from internal lineage |
| [SLSA](https://slsa.dev/spec/) | Software build provenance and supply-chain integrity levels | Software release and build-attestation profile |
| [in-toto](https://in-toto.io/) | Authorized supply-chain steps and artifact verification | Software workflow attestation and verification adapter |
| [C2PA](https://c2pa.org/) | Media origin and edit history through Content Credentials | Creative/media provenance profile |
| [RO-Crate 1.3](https://www.researchobject.org/ro-crate/specification) | Packaged research data, contextual entities, workflows, scripts, and provenance | Research Domain Pack export/import profile |
| [SHACL](https://www.w3.org/TR/shacl12-core/) | Validation of graph-shaped data and constraints | Candidate semantic-profile validation/export; not a required internal graph representation |

## 6. Durable Work, Observability, and Resource Governance

| Primary source | Pattern considered | MindVault adaptation |
|---|---|---|
| [Temporal human-in-the-loop agent pattern](https://docs.temporal.io/ai-cookbook/human-in-the-loop-python) | Durable waiting, signals, timers, approval, and complete history across disruption | Owned durable workflow contract; evaluate external engine only behind an adapter |
| [OpenTelemetry](https://opentelemetry.io/docs/specs/semconv/) | Portable traces, metrics, logs, and evolving GenAI semantic conventions | Operational observability export with content redaction and local-only modes |
| [FinOps Framework](https://www.finops.org/framework/) | Allocation, governance, optimization, and connection of technology use to value | Hierarchical model/token/compute/storage/API/money budgets and outcome linkage |

## 7. Domain Interoperability Stress Sources

| Domain | Primary source | Architecture pressure tested |
|---|---|---|
| Healthcare | [HL7 FHIR](https://hl7.org/fhir/) | field-level sensitivity, consent, authoritative clinical sources, temporality, regulated exchange |
| Research | [RO-Crate](https://www.researchobject.org/ro-crate/specification) | raw/derived separation, reproducibility, contextual entities, portable packaging |
| Education | [1EdTech LTI](https://www.1edtech.org/standards/lti) | platform/tool roles, institution identity, course and assessment integration |
| Industrial systems | [OPC UA](https://opcfoundation.org/about/opc-technologies/opc-ua/) | semantic device/asset profiles, telemetry, commands, industrial interoperability |
| Consumer devices | [Matter](https://csa-iot.org/all-solutions/matter/) | cross-vendor IP device interoperability and capability models |
| Robotics | [ROS 2](https://docs.ros.org/en/rolling/Concepts/Basic.html) | distributed nodes, topics, services/actions, real-time-adjacent boundaries |
| Legal discovery | [EDRM](https://edrm.net/resources/frameworks-and-standards/edrm-model/) | evidence lifecycle, preservation, review, production, and defensibility |
| Legal rules | [LegalRuleML](https://docs.oasis-open.org/legalruleml/legalruleml-core-spec/v1.0/) | formal norms, obligations, permissions, prohibitions, and jurisdiction context |
| Financial data consent | [Open Banking standards](https://standards.openbanking.org.uk/) | revocable consent, scoped financial data access, external authoritative execution |
| Media authenticity | [C2PA](https://c2pa.org/) | origin, edits, signatures, and consumer-visible provenance |
| Civic/social federation | [ActivityPub](https://www.w3.org/TR/activitypub/) and [Matrix](https://matrix.org/) | decentralized identity, federation, moderation, abuse, node trust, portable participation |

## 8. Research Conclusions

1. The adjacent domains do not justify placing their business objects in the universal kernel.
2. They do justify additional domain-general planes: durable workflow, purpose/consent policy, workload identity, provenance profiles, resource governance, records continuity, device/edge state, simulation branching, moderation, and malleable views.
3. Standards should be implemented as profiles, adapters, mappings, validators, and export formats behind MindVault-owned contracts.
4. Different object classes require different collaboration and consistency semantics; one global CRDT or event-sourcing decision would be inappropriate.
5. The platform needs an explicit unknown-domain challenge to prevent the reference examples from becoming hidden architecture boundaries.
6. Current repository P0 work remains the execution priority; expansion contracts and fixtures can proceed in parallel, but broad domain feature code should wait for authority and reliable-effects gates.

