<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { onDragStateChange, getDragState, startDrag, endDrag, cancelDrag } from './index';

	let isVisible = false;
	let position = { top: 0, left: 0 };
	let hoveredBlock: HTMLElement | null = null;
	let isDragging = false;

	let unsubscribe: (() => void) | null = null;

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
		hoveredBlock = state.hoveredBlock;

		if (state.handlePosition && !isDragging) {
			isVisible = true;
			position = state.handlePosition;
		} else {
			isVisible = false;
		}
	}

	function handleMouseDown(event: MouseEvent) {
		event.preventDefault();
		event.stopPropagation();

		if (hoveredBlock) {
			startDrag(hoveredBlock);

			// Add global event listeners
			window.addEventListener('mouseup', handleMouseUp);
			window.addEventListener('keydown', handleKeyDown);
		}
	}

	function handleMouseUp() {
		endDrag();
		cleanup();
	}

	function handleKeyDown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			cancelDrag();
			cleanup();
		}
	}

	function cleanup() {
		window.removeEventListener('mouseup', handleMouseUp);
		window.removeEventListener('keydown', handleKeyDown);
	}
</script>

{#if isVisible}
	<button
		class="absolute z-20 flex h-6 w-6 cursor-grab items-center justify-center rounded opacity-0 transition-opacity hover:bg-slate-800 group-hover:opacity-60"
		style="top: {position.top}px; left: {position.left}px"
		on:mousedown={handleMouseDown}
		aria-label="Drag to reorder"
		type="button"
	>
		<svg
			class="h-4 w-4 text-slate-500"
			fill="none"
			viewBox="0 0 24 24"
			stroke="currentColor"
		>
			<path
				stroke-linecap="round"
				stroke-linejoin="round"
				stroke-width="2"
				d="M4 8h16M4 16h16"
			/>
		</svg>
	</button>
{/if}

<style>
	/* Show handle when hovering over editor */
	:global(.mv-editor-content:hover) button {
		opacity: 0.6;
	}

	/* Dragging styles */
	:global(.mv-block-dragging) {
		opacity: 0.5;
		outline: 2px dashed rgb(56, 189, 248);
		outline-offset: 2px;
	}
</style>
