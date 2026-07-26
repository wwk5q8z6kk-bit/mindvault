# Product Boundaries

## Core product

The supported core is:

- context and human-readable documents;
- immutable evidence and attachments;
- memory records and temporal versions;
- exact, full-text, semantic, graph, and hybrid retrieval;
- provenance and citations;
- policy, identity, authorization, and local/cloud routing;
- proposals, approval, rejection, audit, and rollback;
- canonical storage, migrations, backup, restore, and export;
- least-privilege agent access through a narrow supported API.

Core must work locally and remain useful when no LLM is configured.

## Optional modules

Tasks, goals, habits, calendar, relay, public sharing, federation, messaging, social/commenting,
model fine-tuning, marketplace discovery, and rich collaboration are optional modules. Each must
depend on stable core ports; core may not import an optional module.

## External systems

Obsidian and filesystem vaults are user-owned sources, not subordinate databases. Cloud models,
calendar providers, webhooks, MCP servers, and future sync relays are untrusted external systems.
Their absence must not corrupt or make core knowledge unreadable.

DevX, Meridian, CodePark, and DevX Runtime are separate products. Shared libraries require an
explicitly versioned package and license; no source-tree coupling, shared mutable database, or
implicit credential sharing is allowed.

## Non-goals for the next vertical slice

- replacing every UI or transport;
- team tenancy;
- public social features;
- general-purpose messaging;
- automatic canonical AI memory;
- a C++ subsystem;
- bidirectional sync before canonical versioning is proven.
