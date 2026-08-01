# MindVault Frontend UX Audit (Feb 12, 2026)

> **Superseded for systemic judgment, IA, competitive strategy, roadmap, and
> engineering backlog by
> [`UI_UX_PRODUCT_EXPERIENCE_AUDIT.md`](UI_UX_PRODUCT_EXPERIENCE_AUDIT.md)
> (2026-07-31).** Keep this file as a historical page-level defect log; several
> findings (token adoption, `confirm`/`prompt`, focus rings) remain open and are
> restated with fresh evidence in the superseding audit.

## Executive Summary

Systematic review of all 25+ pages and key components. The app has a strong dark-themed foundation with good component modularity. However, three systemic issues recur across the entire surface:

1. **Design token adoption is incomplete** — `--mv-*` CSS variables exist but many components still use hardcoded Tailwind slate/sky colors
2. **Accessibility is inconsistent** — ARIA labels, focus indicators, and keyboard navigation are present on some pages but missing on others
3. **No shared UI primitives** — buttons, modals, badges, and empty states are styled inline per-component with no reusable abstractions

**Pages by polish level:**

| Tier | Pages |
|------|-------|
| Production-ready | Graph, Tags, Bookmarks, Flashcards, Timeline |
| Good (minor polish) | Dashboard, Tasks, Notes, Kanban, Calendar, Daily, Goals, Templates, Review, Canvas, Trash, Plan |
| Needs work | Chat, Search, Settings, Inbox, PDF Viewer, Insights |

---

## Systemic Issues (affect all pages)

### S1. Hardcoded colors instead of design tokens

`tokens.css` defines `--mv-bg`, `--mv-panel`, `--mv-border`, etc. but components mix these with raw Tailwind:

```
border-slate-800      (should be border-[rgb(var(--mv-border))])
bg-slate-900/40       (should be bg-[rgb(var(--mv-panel))]/40)
text-slate-400        (should be text-[rgb(var(--mv-muted))])
```

**Affected:** Dashboard stats, Inbox items, Calendar custom CSS vars, Task list items, Notes sidebar, Settings forms, Kanban columns.

**Impact:** Light mode will look wrong. Theme changes require touching every file.

**Fix:** Global find-replace of the ~8 most common hardcoded values to token equivalents.

### S2. No shared button component

At least 5 distinct button patterns exist with no abstraction:
- Primary: `bg-sky-500 hover:bg-sky-400 text-white`
- Secondary: `border border-slate-700 text-slate-300 hover:border-slate-600`
- Danger: `bg-red-500/20 text-red-200 hover:bg-red-500/30`
- Ghost: `text-slate-400 hover:text-white`
- Disabled: sometimes `opacity-50`, sometimes `opacity-40`

**Fix:** Create `Button.svelte` with `variant` prop or define Tailwind `@apply` classes.

### S3. No shared modal component

Modals use different patterns:
- Graph: `backdrop-blur`
- Bookmarks: `backdrop-blur-sm`
- Trash: no blur
- TaskFormModal: separate component
- Tags: uses `prompt()`/`confirm()`

**Fix:** Create `Modal.svelte` with consistent backdrop, focus trap, and Escape handling.

### S4. Inconsistent empty states

- Graph: rich icon + gradient + CTA buttons
- Tags: bare "No tags yet."
- Bookmarks: "No bookmarks"
- Notes: icon + button
- Tasks: two different styles for "no tasks" vs "no filter match"

**Fix:** Create `EmptyState.svelte` with icon, title, description, and optional CTA.

### S5. Missing focus indicators

Most interactive elements lack visible `:focus-visible` rings. Keyboard-only users can't tell where focus is.

**Fix:** Add `focus-visible:ring-2 focus-visible:ring-[rgb(var(--mv-ring))]` to all buttons, links, and interactive elements.

### S6. `prompt()` and `confirm()` usage

Native browser dialogs used for:
- Inbox: snooze custom date, tag note
- Tags: rename, delete
- Settings: delete operations

**Fix:** Replace with custom modals using the shared Modal component.

### S7. Inconsistent loading states

Some pages show "Loading...", some use skeleton, some show nothing. No spinners anywhere.

**Fix:** Create `LoadingSpinner.svelte` and `SkeletonCard.svelte`, use consistently.

---

## Per-Page Findings

### Dashboard (`/`)

| Issue | Severity | Fix |
|-------|----------|-----|
| Stats grid missing `sm:grid-cols-3` breakpoint | Medium | Add intermediate breakpoint |
| Stat card colors have no consistent principle (sky/amber/red/emerald) | Low | Document color semantics or unify |
| "In Progress" truncates at 5 with no "show more" link | Medium | Add overflow indicator |
| Quick Actions have no hover feedback | Low | Add `hover:border-{color}/60` |
| No error handling if `loadTasks()`/`loadNotes()` fails | Medium | Add try/catch + toast |
| Stats should use `<dl>/<dt>/<dd>` for screen readers | Medium | Semantic HTML fix |
| Keyboard hint hardcodes `Cmd+K` (Mac only) | Low | Detect platform |

### Inbox (`/inbox`)

| Issue | Severity | Fix |
|-------|----------|-----|
| `prompt()` for snooze custom date — no date picker | High | Replace with modal + date input |
| `prompt()` for tag note | High | Replace with modal |
| AI triage buttons (Triage/Apply/Clear) lack visual hierarchy | Medium | Primary/secondary/ghost differentiation |
| Conflict alerts use `border-red-900/40` — too dark to see | Medium | Use `border-red-500/30` |
| Checkbox styling is `h-3.5 w-3.5` — hard to click | Medium | Increase to `h-5 w-5` |
| Virtual list `max-height: 80vh` doesn't account for header | Medium | Use `calc(100vh - header)` |
| Action buttons wrap awkwardly on mobile | Medium | Reduce count or use overflow menu |

### Tasks (`/tasks`)

| Issue | Severity | Fix |
|-------|----------|-----|
| Status colors duplicated in TaskListItem AND TaskDetailPanel | High | Extract to shared constants |
| Priority dot has no ARIA label or title | Medium | Add `title="Priority {n}"` |
| Quick-add preview fails silently on invalid input (e.g., "p6") | Medium | Show validation feedback |
| Bulk action bar not dismissible via Escape | Medium | Add keydown handler |
| Empty state has two inconsistent designs | Low | Unify |
| Overdue tasks not highlighted distinctly in list | Medium | Add red accent border |
| `max-height: 70vh` on task list is inflexible | Low | Use flex-1 min-h-0 |
| Delete dialog missing `role="alertdialog"` | Medium | Add ARIA role |

### Notes (`/notes`)

| Issue | Severity | Fix |
|-------|----------|-----|
| Right sidebar is overwhelmingly long (editor+buttons+autotag+backlinks+suggestions+AI+attachments+share+comments) | High | Group into collapsible sections or tabs |
| j/k keyboard navigation has no visible focus ring | Medium | Add focus styling |
| Kind badges inconsistent between list (`text-[9px]`) and detail | Low | Standardize |
| Bulk mode not visually distinct from normal mode | Medium | Add banner/highlight |
| Virtual scroll overscan too conservative (5) | Low | Increase to 10-15 |

### Kanban (`/kanban`)

| Issue | Severity | Fix |
|-------|----------|-----|
| No visual feedback when dragging card (no opacity/scale change) | Medium | Add `opacity: 0.7` during drag |
| Column `max-height: calc(100vh - 220px)` is brittle | Medium | Use CSS clamp or flex |
| `aria-pressed` on draggable button is wrong semantic | Medium | Use `aria-selected` |
| No keyboard navigation between columns (arrow keys) | Medium | Add keydown handler |
| No transition effect when card moves between columns | Low | Add animate:flip |
| Horizontal scroll not discoverable | Low | Add scroll indicator |

### Calendar (`/calendar`)

| Issue | Severity | Fix |
|-------|----------|-----|
| Uses custom CSS variables disconnected from `--mv-*` tokens | High | Migrate to token system |
| Loading state only dims content — no spinner | Medium | Add spinner component |
| Month view "+N more" is a span, not clickable | Medium | Make it a button |
| Week view items truncated at `text-[11px]` with no tooltip | Low | Add title attribute |
| iCal import `<label>` wrapping `<input type="file">` — keyboard inaccessible | Medium | Fix keyboard access |
| No transition between day/week/month views | Low | Add fade transition |

### Daily Notes (`/daily`)

| Issue | Severity | Fix |
|-------|----------|-----|
| Habit ON/OFF buttons are `text-[10px]` — low contrast, hard to click | High | Use toggle switch or checkbox |
| Habit "checkboxes" are `<button>`, not `<input type="checkbox">` | Medium | Use proper semantic element |
| Arrow nav buttons use `&larr;`/`&rarr;` with no ARIA labels | Medium | Add `aria-label` |
| Date input has no `<label>` | Medium | Add label or aria-label |
| Sidebar hidden on mobile with no alternative | Medium | Add dropdown menu |
| Auto-save failure shows toast but no retry suggestion | Low | Add retry action |
| No transition between days | Low | Add slide transition |

### Chat (`/chat`)

| Issue | Severity | Fix |
|-------|----------|-----|
| No indication system is searching knowledge base during loading | Medium | Show "Searching your knowledge..." |
| Source citations not obviously clickable | Medium | Add underline + arrow icon |
| Chat history silently limited to last 10 messages | Medium | Show indicator |
| Error messages are generic | Low | Parse error type |
| Loading animation has no ARIA role/label | Medium | Add `role="status"` |
| No smooth scroll to bottom | Low | Add `behavior: 'smooth'` |

### Search (`/search`)

| Issue | Severity | Fix |
|-------|----------|-----|
| Filters scattered across multiple rows | Medium | Group into collapsible panel |
| Tag filter is text input, no autocomplete | Medium | Add combobox with available tags |
| Selected result state too subtle | Medium | Add left accent bar |
| Kind dropdown uses custom UI but sort uses native `<select>` | Low | Standardize |
| No "clear filters" suggestion when filtered results are empty | Medium | Add hint |

### Settings (`/settings`)

| Issue | Severity | Fix |
|-------|----------|-----|
| 1800+ line page with no section navigation | High | Add sticky sidebar with anchors |
| Uses `confirm()` for destructive actions | Medium | Replace with custom modal |
| Toggle switches lack transition animation | Low | Add `transition-all` |
| Feature toggle descriptions are `text-[10px]` — too small | Medium | Change to `text-xs` |
| AI provider selection requires separate "Save" click | Medium | Auto-save or show "unsaved" badge |
| Keyboard shortcuts section is a wall of text | Medium | Group by category |

### Graph (`/graph`)

| Issue | Severity | Fix |
|-------|----------|-----|
| Relationship creation has too many reactive states | Low | Extract to state machine |
| Modal missing `aria-modal="true"` | Medium | Add attribute |
| Complex ternary for node radius sizing | Low | Extract to helper |

### Goals (`/goals`)

| Issue | Severity | Fix |
|-------|----------|-----|
| No progress bar visualization | Medium | Add progress bar |
| Streak display not visually differentiated | Low | Add color badges |
| Goal/habit forms duplicate field structure | Low | Extract shared form |

### Templates (`/templates`)

| Issue | Severity | Fix |
|-------|----------|-----|
| Version diff shows line counts only, no visual diff | Medium | Add syntax-highlighted diff |
| Pack install shows no progress | Low | Add progress indicator |

### Flashcards (`/flashcards`)

| Issue | Severity | Fix |
|-------|----------|-----|
| Rating buttons too small and close together | Medium | Increase size + spacing |
| No progress bar for review session | Low | Add bar |
| Card parsing fails on multiline answers | Low | Improve regex |

### Canvas (`/canvas`)

| Issue | Severity | Fix |
|-------|----------|-----|
| No touch support for pan/drag | Medium | Add touchmove handlers |
| SVG grid pattern recalculates every render | Low | Cache pattern |

### PDF Viewer (`/pdf`) — NEEDS WORK

| Issue | Severity | Fix |
|-------|----------|-----|
| No actual PDF.js integration — uses `<object>` tag | High | Integrate PDF.js |
| Annotations exist in data but never overlay on PDF | High | Implement annotation layer |
| Manual page number entry instead of navigation | Medium | Add prev/next buttons |
| No touch zoom support | Medium | Add gesture handlers |

### Insights (`/insights`) — NEEDS WORK

| Issue | Severity | Fix |
|-------|----------|-----|
| Clicking an insight does nothing | High | Add detail modal |
| Cluster info shows "N nodes" but can't view them | Medium | Add expandable node list |
| No conflict resolution actions beyond "Dismiss" | Medium | Add merge/view options |

### Trash (`/trash`)

| Issue | Severity | Fix |
|-------|----------|-----|
| No bulk select for restore/delete | Medium | Add multi-select |
| No search/filter in trash | Low | Add search input |

### Plan (`/plan`)

| Issue | Severity | Fix |
|-------|----------|-----|
| "Apply to calendar" just navigates — doesn't populate events | Medium | Implement calendar integration |
| Workday hours hardcoded, no settings UI | Medium | Add settings panel |

### Timeline (`/timeline`)

| Issue | Severity | Fix |
|-------|----------|-----|
| Filter state resets on page refresh | Low | Persist to localStorage |
| No export functionality | Low | Add CSV/JSON export |

---

## Layout (`+layout.svelte`)

| Issue | Severity | Fix |
|-------|----------|-----|
| 32 nav items in flat list — no grouping | High | Group into sections (Planning, Capture, Views, Settings) |
| Logo uses hardcoded `bg-sky-500/20` not `--mv-accent` | Low | Use token |
| "Offline" and "Pending sync" use same amber color | Medium | Use red for offline |
| Mobile nav duplicates desktop sidebar code | Medium | Extract to NavLinks component |
| No skip-to-content link | Medium | Add for accessibility |
| `<title>` hardcoded to "MindVault" | Low | Update per route |

---

## Priority Action Plan

### Phase 1: Systemic fixes (highest ROI)

1. **Token adoption sweep** — Replace top 8 hardcoded color patterns across all files
2. **Create shared components**: `Button.svelte`, `Modal.svelte`, `EmptyState.svelte`, `LoadingSpinner.svelte`
3. **Sidebar nav grouping** — Group 32 items into 5-6 labeled sections
4. **Replace all `prompt()`/`confirm()`** with custom modals
5. **Add focus-visible rings** globally

### Phase 2: Page-specific high-priority fixes

6. **Settings page navigation** — Add sticky sidebar with section anchors
7. **Notes right sidebar** — Group into collapsible sections
8. **Calendar token migration** — Replace custom CSS vars with `--mv-*`
9. **Inbox date picker** — Replace prompt() with modal
10. **PDF viewer** — Integrate PDF.js for actual rendering

### Phase 3: Polish and accessibility

11. Status color constants → shared file (TaskListItem + TaskDetailPanel)
12. Keyboard navigation: kanban columns, daily notes, flashcard review
13. ARIA improvements: graph modal, kanban cards, daily note inputs
14. Transition effects: calendar view switching, daily note navigation
15. Loading states: standardize across all pages

### Phase 4: Incomplete features

16. PDF Viewer: full PDF.js integration + annotation overlay
17. Insights: detail modal + cluster node viewing
18. Canvas: touch support
19. Plan: actual calendar integration for "apply to calendar"
