<script lang="ts">
	import { createEventDispatcher, onMount, onDestroy } from 'svelte';
	import SlashCommandMenu from './SlashCommandMenu.svelte';

	/** Placeholder text */
	export let placeholder = 'Start writing...';
	/** Whether the editor is read-only */
	export let readonly = false;
	/** Initial HTML content */
	export let content = '';
	/** Whether the editor is empty */
	export let isEmpty = true;
	/** Slash menu items */
	export let slashItems: SlashItem[] = [];

	interface SlashItem {
		id: string;
		label: string;
		description: string;
		icon?: string;
		action: () => void;
	}

	const dispatch = createEventDispatcher<{
		input: void;
		keydown: KeyboardEvent;
		paste: ClipboardEvent;
		selectionchange: void;
	}>();

	let editorEl: HTMLDivElement;
	let showSlashMenu = false;
	let slashMenuX = 0;
	let slashMenuY = 0;
	let slashActiveIndex = 0;
	let slashQuery = '';
	let slashRange: Range | null = null;
	let filteredSlashItems: SlashItem[] = [];

	// Reactive filtering
	$: filteredSlashItems = slashQuery
		? slashItems.filter(
				(item) =>
					item.label.toLowerCase().includes(slashQuery.toLowerCase()) ||
					item.description.toLowerCase().includes(slashQuery.toLowerCase())
			)
		: slashItems;

	export function getElement(): HTMLDivElement {
		return editorEl;
	}

	export function setContent(html: string): void {
		if (editorEl) {
			editorEl.innerHTML = html;
		}
	}

	export function getContent(): string {
		return editorEl?.innerHTML ?? '';
	}

	export function focus(): void {
		editorEl?.focus();
	}

	function openSlashMenu() {
		const sel = window.getSelection();
		if (!sel || sel.rangeCount === 0) return;

		const range = sel.getRangeAt(0);
		slashRange = range.cloneRange();
		const rect = range.getBoundingClientRect();
		const editorRect = editorEl.getBoundingClientRect();

		slashMenuX = rect.left - editorRect.left;
		slashMenuY = rect.bottom - editorRect.top + 4;
		slashQuery = '';
		slashActiveIndex = 0;
		showSlashMenu = true;
	}

	function closeSlashMenu() {
		showSlashMenu = false;
		slashRange = null;
	}

	function executeSlashItem(item: SlashItem) {
		// Remove the "/" and query text from the editor
		if (slashRange) {
			const sel = window.getSelection();
			if (sel) {
				const range = slashRange.cloneRange();
				const startOffset = Math.max(0, range.startOffset - 1 - slashQuery.length);
				range.setStart(range.startContainer, startOffset);
				sel.removeAllRanges();
				sel.addRange(range);
				document.execCommand('delete');
			}
		}
		closeSlashMenu();
		item.action();
	}

	function handleInput() {
		isEmpty = isEditorEmpty();
		dispatch('input');

		// Slash command detection
		if (showSlashMenu) {
			const sel = window.getSelection();
			if (sel && sel.rangeCount > 0) {
				const range = sel.getRangeAt(0);
				const textNode = range.startContainer;
				if (textNode.nodeType === Node.TEXT_NODE) {
					const text = textNode.textContent ?? '';
					const cursorPos = range.startOffset;
					const slashIdx = text.lastIndexOf('/', cursorPos);
					if (slashIdx >= 0) {
						slashQuery = text.substring(slashIdx + 1, cursorPos);
					} else {
						closeSlashMenu();
					}
				}
			}
		}
	}

	function handleKeyDown(event: KeyboardEvent) {
		// Slash menu navigation
		if (showSlashMenu) {
			if (event.key === 'ArrowDown') {
				event.preventDefault();
				slashActiveIndex = (slashActiveIndex + 1) % filteredSlashItems.length;
				return;
			}
			if (event.key === 'ArrowUp') {
				event.preventDefault();
				slashActiveIndex = (slashActiveIndex - 1 + filteredSlashItems.length) % filteredSlashItems.length;
				return;
			}
			if (event.key === 'Enter' || event.key === 'Tab') {
				event.preventDefault();
				if (filteredSlashItems[slashActiveIndex]) {
					executeSlashItem(filteredSlashItems[slashActiveIndex]);
				}
				return;
			}
			if (event.key === 'Escape') {
				event.preventDefault();
				closeSlashMenu();
				return;
			}
		}

		// Open slash menu on "/" at start of line or after space
		const mod = event.metaKey || event.ctrlKey;
		if (event.key === '/' && !mod && !showSlashMenu) {
			const sel = window.getSelection();
			if (sel && sel.rangeCount > 0) {
				const range = sel.getRangeAt(0);
				const textNode = range.startContainer;
				const offset = range.startOffset;
				const text = textNode.textContent ?? '';
				const charBefore = offset > 0 ? text[offset - 1] : '';
				if (offset === 0 || charBefore === ' ' || charBefore === '\n') {
					setTimeout(() => openSlashMenu(), 10);
				}
			}
		}

		dispatch('keydown', event);
	}

	function handlePaste(event: ClipboardEvent) {
		dispatch('paste', event);
	}

	function isEditorEmpty(): boolean {
		if (!editorEl) return true;
		const text = editorEl.textContent?.trim() ?? '';
		return text === '' || editorEl.innerHTML === '<p><br></p>';
	}

	function handleSlashSelect(event: CustomEvent<any>) {
		executeSlashItem(event.detail);
	}

	onMount(() => {
		if (content) {
			editorEl.innerHTML = content;
		}
		isEmpty = isEditorEmpty();
	});
</script>

<svelte:document on:selectionchange={() => dispatch('selectionchange')} />

<div class="relative">
	<div
		bind:this={editorEl}
		class="mv-editor-content prose prose-invert prose-sm min-h-[200px] max-w-none px-4 py-3 text-sm text-slate-200 outline-none"
		contenteditable={!readonly}
		role="textbox" tabindex="0"
		aria-multiline="true"
		aria-placeholder={placeholder}
		on:input={handleInput}
		on:keydown={handleKeyDown}
		on:paste={handlePaste}
	></div>

	{#if isEmpty && !readonly}
		<div class="pointer-events-none absolute left-4 top-3 text-sm text-slate-600">
			{placeholder}
		</div>
	{/if}

	{#if showSlashMenu && filteredSlashItems.length > 0}
		<SlashCommandMenu
			items={filteredSlashItems}
			activeIndex={slashActiveIndex}
			x={slashMenuX}
			y={slashMenuY}
			on:select={handleSlashSelect}
			on:close={closeSlashMenu}
		/>
	{/if}
</div>

<style>
	.mv-editor-content :global(h1) {
		font-size: 1.5rem;
		font-weight: 700;
		margin: 0.75rem 0 0.5rem;
		color: white;
	}
	.mv-editor-content :global(h2) {
		font-size: 1.25rem;
		font-weight: 600;
		margin: 0.75rem 0 0.5rem;
		color: white;
	}
	.mv-editor-content :global(h3) {
		font-size: 1.1rem;
		font-weight: 600;
		margin: 0.5rem 0 0.25rem;
		color: white;
	}
	.mv-editor-content :global(h4) {
		font-size: 1rem;
		font-weight: 600;
		margin: 0.5rem 0 0.25rem;
		color: rgb(203 213 225);
	}
	.mv-editor-content :global(p) {
		margin: 0.25rem 0;
	}
	.mv-editor-content :global(blockquote) {
		border-left: 3px solid rgb(56 189 248);
		padding-left: 1rem;
		margin: 0.5rem 0;
		color: rgb(148 163 184);
	}
	.mv-editor-content :global(pre) {
		background: rgb(15 23 42);
		border: 1px solid rgb(30 41 59);
		border-radius: 8px;
		padding: 0.75rem 1rem;
		margin: 0.5rem 0;
		overflow-x: auto;
		font-size: 0.8rem;
	}
	.mv-editor-content :global(code) {
		font-family: 'SF Mono', 'Fira Code', 'Cascadia Code', monospace;
		font-size: 0.85em;
	}
	.mv-editor-content :global(:not(pre) > code) {
		background: rgb(30 41 59);
		padding: 0.15rem 0.4rem;
		border-radius: 4px;
		color: rgb(248 113 113);
	}
	.mv-editor-content :global(ul),
	.mv-editor-content :global(ol) {
		padding-left: 1.5rem;
		margin: 0.25rem 0;
	}
	.mv-editor-content :global(li) {
		margin: 0.15rem 0;
	}
	.mv-editor-content :global(ul[data-type='taskList']) {
		list-style: none;
		padding-left: 0;
	}
	.mv-editor-content :global(li[data-type='taskItem']) {
		display: flex;
		align-items: flex-start;
		gap: 0.5rem;
	}
	.mv-editor-content :global(li[data-type='taskItem'] label) {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		cursor: pointer;
	}
	.mv-editor-content :global(li[data-type='taskItem'] input[type='checkbox']) {
		accent-color: rgb(56 189 248);
		cursor: pointer;
	}
	.mv-editor-content :global(li[data-type='taskItem'][data-checked='true'] span) {
		text-decoration: line-through;
		color: rgb(100 116 139);
	}
	.mv-editor-content :global(a) {
		color: rgb(56 189 248);
		text-decoration: underline;
		text-decoration-color: rgb(56 189 248 / 0.3);
	}
	.mv-editor-content :global(a:hover) {
		text-decoration-color: rgb(56 189 248);
	}
	.mv-editor-content :global(a.mv-wikilink) {
		color: rgb(192 132 252);
		text-decoration-color: rgb(192 132 252 / 0.3);
	}
	.mv-editor-content :global(hr) {
		border: none;
		border-top: 1px solid rgb(51 65 85);
		margin: 1rem 0;
	}
	.mv-editor-content :global(s),
	.mv-editor-content :global(del) {
		color: rgb(100 116 139);
	}
	.mv-editor-content :global(table) {
		width: 100%;
		border-collapse: collapse;
		margin: 0.5rem 0;
		font-size: 0.85rem;
	}
	.mv-editor-content :global(th),
	.mv-editor-content :global(td) {
		border: 1px solid rgb(51 65 85);
		padding: 0.4rem 0.75rem;
		text-align: left;
		min-width: 80px;
	}
	.mv-editor-content :global(th) {
		background: rgb(30 41 59);
		font-weight: 600;
		color: rgb(203 213 225);
	}
	.mv-editor-content :global(td) {
		background: rgb(15 23 42 / 0.5);
	}
	.mv-editor-content :global(img) {
		max-width: 100%;
		border-radius: 8px;
		margin: 0.5rem 0;
	}
</style>
