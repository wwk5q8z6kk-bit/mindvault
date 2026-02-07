<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { onDragStateChange, getDragState } from './index';

	/** Editor container element */
	export let editorContainer: HTMLElement;

	let isVisible = false;
	let position = { top: 0 };
	let isDragging = false;
	let dropIndex: number | null = null;
	let draggedIndex = -1;

	let unsubscribe: (() => void) | null = null;

	// Block selector
	const BLOCK_SELECTOR = 'p, h1, h2, h3, h4, h5, h6, ul, ol, blockquote, pre, hr, table, .mv-math-block';

	onMount(() => {
		const state = getDragState();
		syncState(state);

		unsubscribe = onDragStateChange((state) => {
			syncState(state);
		});
	});

	onDestroy(() => {
		unsubscribe?.();
	});

	function syncState(state: ReturnType<typeof getDragState>) {
		isDragging = state.isDragging;
		dropIndex = state.dropIndex;
		draggedIndex = state.draggedIndex;

		if (isDragging && dropIndex !== null && dropIndex !== draggedIndex) {
			isVisible = true;
			position = calculateIndicatorPosition(dropIndex);
		} else {
			isVisible = false;
		}
	}

	function calculateIndicatorPosition(index: number): { top: number } {
		const editorEl = editorContainer.querySelector('.mv-editor-content');
		if (!editorEl) return { top: 0 };

		const blocks = Array.from(editorEl.querySelectorAll(`:scope > ${BLOCK_SELECTOR}`));
		const editorRect = editorEl.getBoundingClientRect();

		if (blocks.length === 0) {
			return { top: 0 };
		}

		if (index >= blocks.length) {
			// After the last block
			const lastBlock = blocks[blocks.length - 1];
			const rect = lastBlock.getBoundingClientRect();
			return { top: rect.bottom - editorRect.top + 4 };
		}

		// Before the target block
		const targetBlock = blocks[index];
		const rect = targetBlock.getBoundingClientRect();
		return { top: rect.top - editorRect.top - 4 };
	}
</script>

{#if isVisible}
	<div
		class="absolute left-0 right-0 z-20 h-0.5 bg-sky-500"
		style="top: {position.top}px"
	>
		<div class="absolute -left-1 -top-1 h-2 w-2 rounded-full bg-sky-500"></div>
		<div class="absolute -right-1 -top-1 h-2 w-2 rounded-full bg-sky-500"></div>
	</div>
{/if}
