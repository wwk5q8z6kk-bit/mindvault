# Protocol Boundaries

- **Status:** Required architecture contract
- **Effective:** 2026-07-26
- **Review trigger:** Any supported protocol major version or security-model change

## Principle

MindVault maintains a stable internal domain and supports external standards
through versioned adapters. No protocol object is the canonical database
schema. Translation preserves identity, grants, source authority, provenance,
causation, correlation, and receipts.

## Protocol map

| Need                      | Boundary                                | MindVault role                          |
| ------------------------- | --------------------------------------- | --------------------------------------- |
| AI context and tools      | MCP                                     | Server and client/host                  |
| Agent discovery and work  | A2A                                     | Client and server adapter               |
| Synchronous public API    | OpenAPI HTTP                            | Governed public contract                |
| Event subscriptions       | AsyncAPI                                | Governed event contract                 |
| Event envelope            | CloudEvents                             | External transport-independent envelope |
| Public object validation  | JSON Schema                             | Versioned public schemas                |
| Typed internal transport  | Protocol Buffers                        | Internal/high-performance contract      |
| Human authentication      | OpenID Connect                          | Federated identity                      |
| Delegated authorization   | OAuth                                   | Revocable application and agent grants  |
| Enterprise provisioning   | SCIM                                    | User/group lifecycle adapter            |
| Provenance exchange       | W3C PROV mapping                        | Interchange model                       |
| Telemetry                 | OpenTelemetry                           | Trace, metric, and log correlation      |
| Communication federation  | Matrix bridge                           | Optional messaging compatibility        |
| User-controlled data pods | Solid adapter                           | Optional storage compatibility          |
| Calendar and contacts     | iCalendar/CalDAV, vCard/CardDAV         | Domain adapters                         |
| Email                     | JMAP or IMAP/SMTP                       | Ingestion/action adapters               |
| Files and objects         | Local files, WebDAV, S3-compatible APIs | Storage adapters                        |
| Source code               | Git                                     | Code and change adapter                 |

Version numbers are negotiated at runtime or installation time. Documentation
records verified profiles rather than assuming “latest” behavior forever.

## Internal domain boundary

The internal model owns:

- Actor, Principal, ContextNode, Workspace, Space, Membership;
- SourceBinding, schema identity, materialization, freshness;
- ContextGrant, ToolGrant, delegation, approval, policy decision;
- WorkOrder, AgentRun, Artifact;
- command, event, action receipt, and Trust Ledger records.

Adapters may retain complete external payloads as source artifacts, but they
must map operational behavior to these internal concepts.

## MCP boundary

### Server role

Authorized MCP hosts may discover and use governed resources, prompts, and
tools. Read access compiles a Context Grant; mutation-capable tools create
proposals or commands under a separate Tool Grant. Progress, cancellation,
logging, and elicitation correlate to the originating action envelope.

### Client/host role

MindVault may connect to external MCP servers for resources and tools. Each
server is represented as an Application or Agent Node with:

- pinned identity and endpoint policy;
- negotiated capabilities and protocol version;
- context/action profiles;
- destination and egress restrictions;
- per-call approval and audit policy;
- revocation and health state.

External MCP content and tool descriptions are untrusted. Connecting a server
does not grant it access to the caller’s context, and listing a tool does not
authorize invocation.

## A2A boundary

A2A provides external agent discovery, messages, tasks, artifacts, streaming,
push updates, cancellation, and authentication negotiation.

Mappings are adapters:

| Internal              | A2A                       |
| --------------------- | ------------------------- |
| Agent Profile         | Agent Card                |
| Skill                 | Agent Skill               |
| WorkOrder request     | Initiating message        |
| AgentRun              | Task                      |
| Run status            | Task status               |
| Artifact              | Artifact/part             |
| Progress              | Status update event       |
| Approval/input needed | Input-required state      |
| Credential step       | Auth-required interaction |

Internal WorkOrder and AgentRun state remain authoritative. A2A metadata cannot
widen grants or bypass policy, and remote task identity is bound through a
Source Binding.

## HTTP and event boundaries

- OpenAPI is the authoritative description of supported public synchronous
  HTTP operations; internal gRPC does not silently expose broader authority.
- AsyncAPI describes supported subscriptions, channels, delivery guarantees,
  and consumer responsibilities.
- CloudEvents supplies the external event envelope. MindVault requires
  additional extension attributes or signed data for actor, principal, Space,
  sensitivity, causation, correlation, provenance, retention, and trust.
- CloudEvents `source` plus `id` is the deduplication identity for a producer;
  it is not by itself proof of authenticity or exactly-once delivery.
- State mutations and outbound events are joined through a transactional
  outbox.

## Identity and authorization boundary

OIDC establishes human identity; OAuth delegates bounded access; SCIM
provisions organization users and groups. None directly defines Space
membership, Context Grants, Tool Grants, or resource authorization. The policy
engine maps authenticated external claims to internal actors and evaluates the
requested operation against current internal state.

Run credentials are short-lived, audience-bound, and narrower than the
requesting principal’s standing authority. Devices, services, integrations,
and agents have distinct identity types.

## Provenance boundary

Internal evidence and Trust Ledger records map to W3C PROV concepts for
exchange:

- resource or artifact -> Entity;
- action, derivation, or AgentRun -> Activity;
- human, agent, service, application, or node -> Agent.

The internal action envelope may be stricter than W3C PROV. Export must not
discard principal/acting-actor separation, grants, approvals, source versions,
or provider receipts.

## Protocol negotiation and failure

- Supported versions and extensions are declared explicitly.
- Unknown required capabilities fail before data transfer or action.
- Downgrade is permitted only by policy and is recorded.
- Translation loss is surfaced; security-relevant fields are never silently
  dropped.
- Retries retain stable idempotency and causation identifiers.
- Adapter failure cannot commit false canonical success.

## Current verified references

As of 2026-07-26:

- [MCP specification 2025-11-25](https://modelcontextprotocol.io/specification/2025-11-25)
  defines host/client/server roles, capability negotiation, resources, prompts,
  tools, progress, cancellation, elicitation, and user-control expectations.
- [A2A v1.0 specification](https://a2a-protocol.org/latest/specification/)
  defines Agent Cards, tasks, messages, artifacts, streaming, push
  notifications, protocol negotiation, and standard security schemes.
- [OpenAPI specifications](https://spec.openapis.org/oas/) publish 3.2.0 and
  maintained 3.1/3.0 lines.
- [CloudEvents specification](https://github.com/cloudevents/spec/blob/main/cloudevents/spec.md)
  defines the transport-independent event envelope and `source` + `id`
  uniqueness semantics.
- [AsyncAPI specification](https://www.asyncapi.com/docs/reference/specification/latest)
  governs message-driven API descriptions.
- [W3C PROV overview](https://www.w3.org/TR/prov-overview/) defines an
  interoperable provenance family centered on entities, activities, and agents.

These references validate adapter fit. They do not delegate MindVault’s policy
or domain decisions to the standards.
