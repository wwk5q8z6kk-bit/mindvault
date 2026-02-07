<script lang="ts">
	import { canUndo, canRedo, lastUndoDescription, lastRedoDescription, popUndo, popRedo } from '$lib/stores/undo';
	import { fade, fly } from 'svelte/transition';

	let showDetails = false;
</script>

{#if $canUndo || $canRedo}
	<div
		class="fixed bottom-20 left-4 z-40 md:bottom-4"
		transition:fly={{ y: 20, duration: 200 }}
		role="region"
		aria-label="Undo/Redo actions"
	>
		<div
			class="flex items-center gap-1 rounded-xl border border-slate-700/50 bg-slate-900/95 p-1 shadow-lg backdrop-blur-sm"
			on:mouseenter={() => (showDetails = true)}
			on:mouseleave={() => (showDetails = false)}
			role="group"
		>
			{#if $canUndo}
				<button
					class="flex items-center gap-1.5 rounded-lg px-2.5 py-1.5 text-xs text-slate-300 transition hover:bg-slate-800 hover:text-white"
					on:click={() => popUndo()}
					title={$lastUndoDescription ? `Undo: ${$lastUndoDescription}` : 'Undo'}
				>
					<svg class="h-3.5 w-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 10h10a8 8 0 018 8v2M3 10l6 6m-6-6l6-6" />
					</svg>
					<span class="hidden sm:inline">Undo</span>
				</button>
			{/if}

			{#if $canRedo}
				<button
					class="flex items-center gap-1.5 rounded-lg px-2.5 py-1.5 text-xs text-slate-300 transition hover:bg-slate-800 hover:text-white"
					on:click={() => popRedo()}
					title={$lastRedoDescription ? `Redo: ${$lastRedoDescription}` : 'Redo'}
				>
					<svg class="h-3.5 w-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 10h-10a8 8 0 00-8 8v2M21 10l-6 6m6-6l-6-6" />
					</svg>
					<span class="hidden sm:inline">Redo</span>
				</button>
			{/if}
		</div>

		{#if showDetails && ($lastUndoDescription || $lastRedoDescription)}
			<div
				class="mt-2 rounded-lg border border-slate-700/50 bg-slate-900/95 px-3 py-2 text-xs text-slate-400 shadow-lg backdrop-blur-sm"
				transition:fade={{ duration: 150 }}
			>
				{#if $lastUndoDescription}
					<div class="flex items-center gap-2">
						<span class="text-slate-500">Undo:</span>
						<span class="text-slate-300">{$lastUndoDescription}</span>
						<kbd class="rounded bg-slate-800 px-1.5 py-0.5 text-[10px] text-slate-500">Cmd+Z</kbd>
					</div>
				{/if}
				{#if $lastRedoDescription}
					<div class="mt-1 flex items-center gap-2">
						<span class="text-slate-500">Redo:</span>
						<span class="text-slate-300">{$lastRedoDescription}</span>
						<kbd class="rounded bg-slate-800 px-1.5 py-0.5 text-[10px] text-slate-500">Cmd+Shift+Z</kbd>
					</div>
				{/if}
			</div>
		{/if}
	</div>
{/if}
