# Figma Design System Rules — MindVault Frontend

These rules define how to translate Figma designs into production code for the
MindVault SvelteKit frontend. Follow them for every Figma-driven implementation.

---

## Required Figma-to-Code Flow (do not skip)

1. Run `get_design_context` to fetch the structured representation for the target node(s).
2. If the response is too large or truncated, run `get_metadata` first to get the node map, then re-fetch only the required node(s) with `get_design_context`.
3. Run `get_screenshot` for a visual reference of the node/variant being implemented.
4. Only after you have both design context and screenshot, download any needed assets and start implementation.
5. **Translate the output (React + Tailwind) into Svelte 5 + MindVault conventions.** The Figma MCP returns React/Tailwind code as a design representation — never paste it directly.
6. Validate the final UI against the Figma screenshot for 1:1 visual and behavioral parity before marking complete.

---

## Component Organization

- IMPORTANT: All UI components live in `frontend/src/lib/components/`. Check for existing components before creating new ones.
- Editor sub-components live in `frontend/src/lib/components/editor/`.
- Components are **flat** (no subdirectories by feature or type).
- **Naming**: PascalCase with semantic suffixes:
  - Modals: `*Modal.svelte` (e.g., `QuickCaptureModal`, `TaskFormModal`)
  - Panels: `*Panel.svelte` (e.g., `TaskDetailPanel`, `BacklinksPanel`)
  - List items: `*ListItem.svelte` or `*Card.svelte` (e.g., `TaskListItem`, `KanbanCard`)
  - Layout wrappers: `*Column.svelte`, `*Stack.svelte`
- New route pages go in `frontend/src/routes/<feature>/+page.svelte`.

---

## Styling Rules

### Design Tokens

- IMPORTANT: Never hardcode colors. Use the CSS variables from `frontend/src/lib/styles/tokens.css`.
- Token values are **raw RGB triplets** (no `rgb()` wrapper), allowing Tailwind-style opacity composition:

| Token              | Dark Default       | Purpose            |
|--------------------|--------------------|--------------------|
| `--mv-bg`          | `15 23 42`         | Page background    |
| `--mv-panel`       | `17 25 40`         | Card/panel bg      |
| `--mv-panel-strong`| `15 23 42`         | Strong panel bg    |
| `--mv-text`        | `226 232 240`      | Primary text       |
| `--mv-muted`       | `148 163 184`      | Secondary text     |
| `--mv-border`      | `30 41 59`         | Borders            |
| `--mv-accent`      | `56 189 248`       | Primary accent     |
| `--mv-accent-strong`| `14 116 144`      | Darker accent      |
| `--mv-success`     | `34 197 94`        | Success/green      |
| `--mv-warning`     | `245 158 11`       | Warning/amber      |
| `--mv-danger`      | `239 68 68`        | Danger/red         |
| `--mv-ring`        | `14 165 233`       | Focus ring         |
| `--mv-radius`      | `14px`             | Default radius     |

**Usage pattern:**

```css
background: rgb(var(--mv-panel));
color: rgb(var(--mv-text) / 0.8);  /* with opacity */
```

```html
<div class="bg-[rgb(var(--mv-panel))] text-[rgb(var(--mv-muted))]">...</div>
```

### Tailwind Palette

- Standard Tailwind colors (slate, sky, blue, violet, emerald, amber, red, cyan, orange) are used alongside `--mv-*` tokens.
- Status/priority mappings use Tailwind colors with opacity modifiers: `bg-violet-500/20 text-violet-300`.
- IMPORTANT: Map Figma colors to the closest `--mv-*` token first; fall back to Tailwind palette only for decorative or status-specific colors.

### Typography

- **Font**: `Inter, system-ui, -apple-system, BlinkMacSystemFont, Segoe UI, sans-serif` (set in `app.css`).
- **Sizes**: Standard Tailwind scale (`text-xs`, `text-sm`, `text-base`) plus custom arbitrary values where needed (`text-[11px]`, `text-[13px]`).
- **Weights**: `font-normal`, `font-medium`, `font-semibold`, `font-bold`.
- Do not introduce new font families.

### Spacing

- Use Tailwind's default spacing scale (1 unit = 0.25rem).
- Common patterns: `px-4 py-3` for cards, `gap-2` for flex containers, `mt-2` for section spacing.

### Dark/Light Mode

- Dark mode is the default. Light mode is supported via `prefers-color-scheme` media query and `data-theme` attribute on `:root`.
- IMPORTANT: All Figma implementations must look correct in both dark and light modes. Test both.

---

## Svelte Component Patterns

### Props and Events

```svelte
<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let task: TaskRecord;
  export let active = false;

  const dispatch = createEventDispatcher<{ select: string; complete: string }>();
</script>
```

- Use typed `createEventDispatcher<T>()` for component communication.
- Boolean props default to `false`.
- Use `$:` reactive declarations for computed state.

### Dynamic Classes

```svelte
<div class={`rounded-xl border px-4 py-3 transition ${
  active ? 'border-sky-500/60 bg-sky-500/10' : 'border-slate-900 bg-slate-900/40'
}`}>
```

- Use template literals for conditional Tailwind classes.
- Define color mappings as local `Record<string, string>` constants for status/priority.

### Accessibility

- IMPORTANT: All interactive elements must have `role`, `tabindex="0"`, and keyboard event handling (`on:keydown` for Enter/Space).
- Use `aria-label`, `aria-pressed`, `aria-expanded` where appropriate.
- Prefer semantic HTML (`<button>`, `<input>`) over styled `<div>` click handlers.

### Imports

```typescript
// SvelteKit / Svelte core
import { onMount, tick, createEventDispatcher } from 'svelte';
import { goto } from '$app/navigation';

// API / Types
import type { TaskRecord } from '$lib/db';
import { completeTaskOptimistic } from '$lib/stores/tasks';

// Components (same directory = relative, otherwise $lib)
import KanbanCard from './KanbanCard.svelte';
import AttachmentsPanel from '$lib/components/AttachmentsPanel.svelte';

// Utilities
import { pushToast } from '$lib/stores/toast';
```

- Use `$lib` alias for all imports from `frontend/src/lib/`.
- Use relative imports only for components in the same directory.

---

## Asset Handling

- IMPORTANT: If the Figma MCP server returns a localhost source for an image or SVG, use that source directly to download the asset.
- IMPORTANT: DO NOT install or import new icon packages. All icons are inline SVGs using `fill="currentColor"` and `stroke="currentColor"`.
- IMPORTANT: DO NOT create placeholder images if a Figma asset source is provided.
- Store downloaded image assets in `frontend/src/lib/assets/`.
- SVG icons should be inlined directly in component markup, sized with Tailwind (`h-4 w-4`), colored via `currentColor`.

---

## Figma-to-Svelte Translation Guide

When the Figma MCP returns React + Tailwind code, translate as follows:

| React Pattern | Svelte Equivalent |
|---|---|
| `<div className="...">` | `<div class="...">` |
| `<Component prop={value} />` | `<Component prop={value} />` (same) |
| `onClick={handler}` | `on:click={handler}` |
| `onChange={handler}` | `on:change={handler}` |
| `{condition && <Comp />}` | `{#if condition}<Comp />{/if}` |
| `{items.map(i => <Comp />)}` | `{#each items as i}<Comp />{/each}` |
| `useState` / `useEffect` | `let` variable / `onMount` + `$:` reactive |
| `React.Fragment` / `<>` | No wrapper needed (Svelte supports fragments) |
| `style={{ color: 'red' }}` | `style="color: red"` |

---

## Project-Specific Conventions

- **Package manager**: Use `pnpm` (NOT npm).
- **Offline-first**: The app uses Dexie (IndexedDB) for local caching. UI components read from stores that sync with the backend.
- **Toast notifications**: Use `pushToast(message, variant)` from `$lib/stores/toast` for user feedback.
- **Namespace awareness**: Most data operations are scoped to the active namespace via `$lib/stores/namespace`.
- **Editor**: Rich text uses TipTap (v3) with custom plugins in `frontend/src/lib/editor/`.
- **No Storybook**: There is no component documentation system. Test components by rendering in route pages.
