<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import {
		onAIAssistStateChange,
		getAIAssistState,
		hideTransformMenu,
		transformText,
		applyTransformResult
	} from './index';

	let isVisible = false;
	let position = { x: 0, y: 0 };
	let isTransforming = false;
	let transformResult: string | null = null;

	let unsubscribe: (() => void) | null = null;

	onMount(() => {
		const state = getAIAssistState();
		syncState(state);

		unsubscribe = onAIAssistStateChange((state) => {
			syncState(state);
		});
	});

	onDestroy(() => {
		unsubscribe?.();
	});

	function syncState(state: ReturnType<typeof getAIAssistState>) {
		isVisible = state.showTransformMenu;
		position = state.transformMenuPosition;
		isTransforming = state.isTransforming;
		transformResult = state.transformResult;
	}

	async function handleTransform(mode: 'summarize' | 'action_items' | 'refine') {
		await transformText(mode);
	}

	function handleApply() {
		applyTransformResult();
	}

	function handleCancel() {
		hideTransformMenu();
	}
</script>

{#if isVisible}
	<div
		class="absolute z-40 -translate-x-1/2 -translate-y-full"
		style="left: {position.x}px; top: {position.y}px"
	>
		{#if transformResult}
			<!-- Result preview -->
			<div class="w-80 rounded-lg border border-slate-700 bg-slate-900 p-3 shadow-xl">
				<div class="mb-2 text-[10px] uppercase tracking-wide text-slate-500">
					AI Result
				</div>
				<div class="mb-3 max-h-40 overflow-y-auto rounded border border-slate-800 bg-slate-950 p-2 text-xs text-slate-300">
					{transformResult}
				</div>
				<div class="flex justify-end gap-2">
					<button
						class="rounded px-2 py-1 text-xs text-slate-400 hover:bg-slate-800 hover:text-white"
						on:click={handleCancel}
					>
						Cancel
					</button>
					<button
						class="rounded bg-sky-500 px-2 py-1 text-xs font-medium text-white hover:bg-sky-400"
						on:click={handleApply}
					>
						Apply
					</button>
				</div>
			</div>
		{:else}
			<!-- Transform options -->
			<div class="flex gap-1 rounded-lg border border-slate-700 bg-slate-900 p-1 shadow-xl">
				<button
					class="flex items-center gap-1.5 rounded px-2 py-1 text-xs text-slate-300 hover:bg-slate-800 disabled:opacity-50"
					disabled={isTransforming}
					on:click={() => handleTransform('summarize')}
				>
					<span>📝</span>
					Summarize
				</button>
				<button
					class="flex items-center gap-1.5 rounded px-2 py-1 text-xs text-slate-300 hover:bg-slate-800 disabled:opacity-50"
					disabled={isTransforming}
					on:click={() => handleTransform('action_items')}
				>
					<span>✅</span>
					Actions
				</button>
				<button
					class="flex items-center gap-1.5 rounded px-2 py-1 text-xs text-slate-300 hover:bg-slate-800 disabled:opacity-50"
					disabled={isTransforming}
					on:click={() => handleTransform('refine')}
				>
					<span>✨</span>
					Refine
				</button>
				{#if isTransforming}
					<div class="flex items-center px-2">
						<div class="h-3 w-3 animate-spin rounded-full border-2 border-slate-600 border-t-sky-500"></div>
					</div>
				{/if}
			</div>
		{/if}
	</div>
{/if}
