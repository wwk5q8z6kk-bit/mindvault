# UI, UX, and Accessibility Audit

## Baseline

The Svelte interface is broad and visually developed, with notes, search, graph, chat/agent,
settings, keychain, plans, habits, calendar, plugins, connectors, and other surfaces. It has
component/unit specs and some semantic labels. It has not been demonstrated as a coherent,
accessible, production desktop flow.

## Product UX issues

1. Core and optional modules are presented in one dense product surface, obscuring the evidence →
   retrieval → proposal → decision loop.
2. The desktop depends on a separately running localhost server but does not provide one reliable
   lifecycle, authentication, recovery, or version-mismatch experience.
3. Security concepts—sealed state, local/cloud execution, proposal authority, connector scope,
   backup completeness—lack a unified, comprehensible status model.
4. AI citations are not a durable evidence inspector: users need source version, exact span,
   timestamp, extraction provenance, and stale/contradiction indicators.
5. Approval must show structured before/after, evidence, policy, actor/model, uncertainty, base
   version, and rollback consequence. Existing proposal UI does not prove all of these.
6. Settings permits an AI API key in localStorage, conflicting with the keychain surface.
7. Failure states are fragmented across network health, WebSocket status, provider errors, and
   local server availability.

## Accessibility risk

No evidence of a WCAG 2.2 AA audit, screen-reader walkthrough, reduced-motion review, forced-colors
support, 200–400% zoom/reflow verification, complete focus management, or automated axe gate was
found. Small text utility classes, icon-only controls, custom editors/graphs/canvas, popovers, and
drag interactions are high-risk areas.

## Required test matrix

- keyboard-only traversal for ingestion, search, evidence inspection, proposal approve/reject, and
  rollback;
- VoiceOver names, roles, states, errors, live regions, and reading order;
- visible focus and focus restoration across dialogs/popovers;
- 200% and 400% zoom, narrow window, text spacing, localization expansion;
- light/dark/high-contrast and non-color status cues;
- reduced motion and animation cancellation;
- editor, graph, canvas, media, and diff alternatives;
- destructive-action confirmation and undo announcements;
- offline/server-mismatch/auth/sealed/cloud-disclosure error recovery;
- axe/Playwright automated checks plus manual assistive-technology evidence.

## Recommended first-slice UX

Limit navigation to Sources, Search, Evidence, Proposals, Audit, and Settings. The slice is complete
when a keyboard/screen-reader user can import a fixture vault, inspect preserved evidence, retrieve
an exact and semantic result, inspect citations/contradictions, approve or reject a proposal, and
verify rollback.
