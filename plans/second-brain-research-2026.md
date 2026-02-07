# MindVault 2026 Research Additions

Last updated: 2026-02-06

## High-ROI feature signals from primary sources

| Feature candidate | Why it matters for MindVault | Primary source |
|---|---|---|
| JSON Canvas round-trip support | Open interoperable canvas format reduces lock-in and enables migration from other PKM tools. | https://obsidian.md/blog/json-canvas/ |
| Canvas cards with mixed file types | Confirms demand for visual workspaces that include notes, images, PDFs, and media together. | https://help.obsidian.md/plugins/canvas |
| Workflow action buttons | One-click, repeatable automations reduce friction for routine task/note workflows. | https://www.notion.com/help/template-buttons |
| Timeline + dependency planning views | Project tracking value increases when tasks are represented on a temporal dependency graph. | https://www.notion.com/help/timelines |
| Recurring issue/task templates | Native recurrence with cadence rules is now standard in planning tools. | https://linear.app/docs/creating-issues |
| Explicit issue dependencies | Blocking/blocked graph edges directly improve prioritization and execution sequencing. | https://linear.app/docs/issue-relations |
| Natural-language recurring dates | Fast language-driven scheduling lowers capture friction for daily use. | https://www.todoist.com/help/articles/introduction-to-recurring-dates-YUYVJJAV |
| Global shortcuts for quick capture | Desktop capture speed improves with OS-level shortcuts and direct command triggering. | https://v2.tauri.app/ko/plugin/global-shortcut/ |
| Native reminder notifications | Local notification actions close the loop for reminders and task follow-through. | https://v2.tauri.app/fr/plugin/notification/ |
| Payload-aware vector filtering + snapshots | Semantic retrieval quality and operational durability improve with filtered recall and snapshots. | https://qdrant.tech/documentation/concepts/filtering/ |
| Snapshot backup/restore for vectors | Recovery and migration paths are required for production-scale memory systems. | https://qdrant.tech/documentation/concepts/snapshots/ |
| Mermaid Gantt support | Structured project visualization with milestones/dependencies can be generated from notes. | https://mermaid.js.org/syntax/gantt.html |
| Importer parity for external migration | Migration from existing vault tools is a key adoption requirement for second-brain apps. | https://help.obsidian.md/plugins/importer |
| Structured database views inside notes | Typed fields, formulas, and table-like organization reduce fragmentation between docs and planning. | https://help.obsidian.md/plugins/bases |
| Dependency links for planning tasks | Blocking relationships are now baseline for execution-focused PM workflows. | https://www.notion.com/help/guides/task-dependencies-for-project-management |
| Auto-shifting blocked timelines | Schedule adjustments based on dependencies reduce manual project maintenance load. | https://www.notion.com/help/guides/auto-shifting-dates-based-on-dependent-tasks |
| Query routing for mixed retrieval strategies | Router-style query engines support MoM-like dispatch to the best retrieval strategy. | https://docs.llamaindex.ai/en/stable/module_guides/querying/router/ |

## Recommended integration order

1. JSON Canvas import/export + canvas workspace persistence.
2. Tauri quick capture (global shortcut + notification actions).
3. Dependency-aware timeline planning (Mermaid + native calendar/task graph).
4. Recurring templates with natural-language date parsing.
5. Qdrant payload filter presets + snapshot orchestration for durability.
