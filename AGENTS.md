## Learned User Preferences

- Prefer full in-repo master product specs over condensed summaries that only link out to existing ADRs.
- Treat discussed verticals (notes/Obsidian, meetings, interview tools, Slack-like Spaces, coding agents, scheduled reports) as conformance floors and proof slices—not the product ceiling; expand via adjacent-domain and competitor research.
- Use evidence-first planning: do not equate code, schemas, routes, or documentation presence with verified implementation.
- Do not copy stealth or live interview-copilot behaviors; treat that category as competitive/ethical risk context only.
- Finish P0 foundation work (truth/docs, identity and grants, reliable effects, agent executor, communication-to-knowledge promotion) before broad domain vertical implementation.
- When implementing from an attached Cursor plan, follow the plan as specified and do not edit the plan file itself.

## Learned Workspace Facts

- Product direction is a sovereign interoperable context fabric: Private Personal Vaults plus Governed Shared Spaces, superseding the older single-owner no-shared-state assumption.
- `docs/MINDVAULT_NEXT_MASTER_PLAN.md` is the superseding product and architecture plan; `INTEROPERABILITY_CONSTITUTION.md` is binding interop law; `IMPLEMENTATION_BACKLOG.md` is the sole execution-status authority.
- Domain Packs carry domain-specific schemas, workflows, policies, and connectors; the kernel stays a domain-agnostic capability algebra.
- SPACE-004 is a live P0: the relay currently inserts every allowed message into the knowledge graph and violates the communication-to-knowledge promotion boundary.
- Shared Actor/Workspace/Space/Membership authorization, collaborative WorkOrder/AgentRun/Artifact state machines, durable inbox/outbox effects, and an Agent Run executor remain unfinished ahead of vertical packs.
- Broad CRDT editing, Kafka-scale decomposition, Matrix federation, and Solid integration stay parked until measured requirements justify them.
- Dual-track execution is intended: Track A finishes foundation truth, safety, and execution; Track B formalizes Domain Pack, workflow, and policy contracts without shipping vertical features yet.
- Extensions are fail-closed and must not access canonical storage directly; context grants and tool grants remain separate.
- MCP `2026-07-28` is the current protocol version to target via versioned adapters, without folding MCP transport details into MindVault’s internal object model.

## Cursor Cloud specific instructions

Standard build/lint/test/run commands live in `README.md`, `CLAUDE.md`, and `scripts/`. This section only covers non-obvious cloud-environment caveats.

### System dependencies (already provisioned in the VM snapshot)

These are installed in the base image, not by the update script. If a server run panics or a build fails on a fresh/rebuilt VM, re-provision them:

- `protobuf-compiler` (protoc), `libssl-dev`, `pkg-config` — required to build the workspace (`tonic`/`prost` and `openssl-sys`).
- ONNX Runtime shared library — required at runtime whenever the binary is built with the `local-embeddings` feature (fastembed uses `ort` in `ort-load-dynamic` mode). Without it the server panics at startup with `Failed to load ONNX Runtime dylib`. `ort 2.0.0-rc.11` requires ONNX Runtime **>= 1.23.x**. It is installed to `/usr/local/lib/libonnxruntime.so*` (via `ldconfig`) so `dlopen("libonnxruntime.so")` resolves without setting `ORT_DYLIB_PATH`. To reinstall: download `onnxruntime-linux-x64-1.23.0.tgz` from the microsoft/onnxruntime GitHub releases, copy `lib/libonnxruntime.so*` to `/usr/local/lib/`, then run `sudo ldconfig`.
- First server/smoke-test run downloads the `bge-small-en-v1.5` embedding model from HuggingFace (needs network); it is cached afterward.

### Running the stack

- Backend (REST 9470 / gRPC 50051): build once with `cargo build -p mv-cli --features local-embeddings`, then run the compiled binary `./target/debug/mv server start --foreground --config <toml>`. Omitting `MINDVAULT_AUTH_TOKEN` leaves auth open (health returns 200 with no token) — convenient for local UI work. The debug binary is large (~1.4 GB) and the full build takes several minutes; `scripts/smoke_test.sh` is the fastest end-to-end check (store + hybrid recall).
- Web admin UI (`web/`, Vite dev on 5173): non-obvious gotcha — the SPA's default API base is `window.location.origin` when served over http, so a Vite-hosted UI on `:5173` sends API calls to `:5173` (Vite returns HTML → the UI shows "Degraded" / "Invalid JSON response from API"). Fix without code changes: open the **Stats** tab and set **API Base** to `http://127.0.0.1:9470`, click **Apply**, reload. Also start the server with `[server].cors_allowed_origins` including `http://localhost:5173` (and `http://127.0.0.1:5173`). `mv-server` does not serve the web build itself.
- Desktop app (`frontend/`, Tauri) and connectors are out of scope for the default setup; the Tauri build additionally needs GTK/WebKit system libraries that are not installed.

### Lint note

`cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings` (the CI gates) may report pre-existing formatting/lint findings on a feature branch — that reflects branch code state, not a broken toolchain.
