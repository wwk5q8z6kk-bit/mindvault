<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import type { RemoteCursor } from './types';

	/** Map of remote cursors by client ID */
	export let cursors: Map<string, RemoteCursor> = new Map();

	/** Editor element to position cursors relative to */
	export let editorElement: HTMLElement | null = null;

	interface CursorDisplay {
		clientId: string;
		name: string;
		color: string;
		top: number;
		left: number;
		visible: boolean;
	}

	let cursorDisplays: CursorDisplay[] = [];

	$: {
		if (editorElement) {
			cursorDisplays = Array.from(cursors.values()).map((cursor) => ({
				clientId: cursor.clientId,
				name: cursor.clientInfo.name,
				color: cursor.clientInfo.color,
				top: 0,
				left: 0,
				visible: true
			}));
		}
	}
</script>

{#each cursorDisplays as cursor (cursor.clientId)}
	{#if cursor.visible}
		<div
			class="pointer-events-none absolute z-50"
			style="top: {cursor.top}px; left: {cursor.left}px"
		>
			<!-- Cursor line -->
			<div
				class="h-5 w-0.5"
				style="background-color: {cursor.color}"
			></div>
			<!-- Name label -->
			<div
				class="absolute -top-5 left-0 whitespace-nowrap rounded px-1 py-0.5 text-[10px] text-white"
				style="background-color: {cursor.color}"
			>
				{cursor.name}
			</div>
		</div>
	{/if}
{/each}

<style>
	/* Remote selection highlight */
	:global(.mv-remote-selection) {
		position: absolute;
		pointer-events: none;
		opacity: 0.3;
	}
</style>
