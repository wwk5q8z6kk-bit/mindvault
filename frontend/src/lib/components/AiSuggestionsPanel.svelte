<script lang="ts">
	import { onMount, createEventDispatcher } from 'svelte';
	import { fade, slide } from 'svelte/transition';
	import { getNode, updateNode } from '$lib/api/nodes';
	import type { KnowledgeNode } from '$lib/api/types';
	import { pushToast } from '$lib/stores/toast';

	export let nodeId: string;

	const dispatch = createEventDispatcher<{ applied: void; dismissed: void }>();

	let node: KnowledgeNode | null = null;
	let loading = true;
	let error: string | null = null;

	onMount(async () => {
		await loadNode();
	});

	async function loadNode() {
		loading = true;
		try {
			node = await getNode(nodeId);
		} catch (e) {
			error = 'Failed to load enriched data';
		} finally {
			loading = false;
		}
	}

	async function applySuggestions() {
		if (!node) return;
		try {
			await updateNode(nodeId, {
				title: node.title,
				tags: node.tags,
				importance: node.importance
			});
			pushToast('AI suggestions applied!', 'success');
			dispatch('applied');
		} catch (e) {
			pushToast('Failed to apply suggestions', 'danger');
		}
	}

	function dismiss() {
		dispatch('dismissed');
	}
</script>

<div class="rounded-xl border border-violet-500/30 bg-violet-500/5 p-4" transition:slide>
	<div class="mb-3 flex items-center justify-between">
		<div class="flex items-center gap-2">
			<div class="flex h-6 w-6 items-center justify-center rounded-full bg-violet-500 text-[10px] text-white">✨</div>
			<span class="text-xs font-semibold text-violet-300">AI Enrichment Discovered</span>
		</div>
		<button class="text-[10px] text-slate-500 hover:text-white" on:click={dismiss}>Dismiss</button>
	</div>

	{#if loading}
		<div class="flex animate-pulse flex-col gap-2">
			<div class="h-4 w-3/4 rounded bg-slate-800"></div>
			<div class="h-3 w-1/2 rounded bg-slate-800"></div>
		</div>
	{:else if node}
		<div class="space-y-4">
			{#if node.title}
				<div>
					<span class="mb-1 block text-[10px] uppercase tracking-wider text-slate-500">Suggested Title</span>
					<div class="rounded-lg bg-slate-900/50 px-3 py-2 text-sm text-white">
						{node.title}
					</div>
				</div>
			{/if}

			{#if node.tags && node.tags.length > 0}
				<div>
					<span class="mb-1 block text-[10px] uppercase tracking-wider text-slate-500">New Tags</span>
					<div class="flex flex-wrap gap-1.5">
						{#each node.tags as tag}
							<span class="rounded bg-violet-500/20 px-2 py-0.5 text-[10px] font-medium text-violet-300">
								#{tag}
							</span>
						{/each}
					</div>
				</div>
			{/if}

			{#if node.importance !== undefined}
				<div class="flex items-center gap-4">
					<div>
						<span class="mb-1 block text-[10px] uppercase tracking-wider text-slate-500">Importance</span>
						<div class="flex items-center gap-2">
							<div class="h-1.5 w-24 overflow-hidden rounded-full bg-slate-800">
								<div class="h-full bg-violet-500" style="width: {node.importance * 100}%"></div>
							</div>
							<span class="text-[10px] font-medium text-violet-300">{Math.round(node.importance * 100)}%</span>
						</div>
					</div>
				</div>
			{/if}

			<div class="flex justify-end gap-2 pt-2">
				<button 
					class="rounded-lg border border-slate-700 px-3 py-1.5 text-[10px] text-slate-400 hover:bg-slate-800 hover:text-white"
					on:click={dismiss}
				>
					Keep Original
				</button>
				<button 
					class="rounded-lg bg-violet-600 px-3 py-1.5 text-[10px] font-bold text-white hover:bg-violet-500"
					on:click={applySuggestions}
				>
					Apply Suggestions
				</button>
			</div>
		</div>
	{/if}
</div>
