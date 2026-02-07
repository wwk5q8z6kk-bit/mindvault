<script lang="ts">
	import { createEventDispatcher, onMount } from 'svelte';
	import type { KnowledgeNode } from '$lib/api/types';
	import { listTemplates, readTemplateKey, readTemplateTargetKind } from '$lib/api/templates';
	import { searchHybrid } from '$lib/api/search';

	export let kind = 'fact';
	export let namespace: string | undefined = undefined;
	export let label = 'Template';

	const dispatch = createEventDispatcher<{ apply: KnowledgeNode }>();

	let open = false;
	let query = '';
	let templates: KnowledgeNode[] = [];
	let semanticMatches: KnowledgeNode[] = [];
	let loading = false;
	let searchPending = false;
	let searchTimer: ReturnType<typeof setTimeout> | null = null;

	onMount(() => {
		loadTemplates();
	});

	async function loadTemplates() {
		loading = true;
		try {
			templates = await listTemplates({ kind, namespace, limit: 200 });
		} catch {
			templates = [];
		} finally {
			loading = false;
		}
	}

	function scheduleSemanticSearch() {
		if (searchTimer) clearTimeout(searchTimer);
		if (query.trim().length < 3) {
			semanticMatches = [];
			return;
		}
		searchTimer = setTimeout(async () => {
			searchPending = true;
			try {
				const results = await searchHybrid(query.trim(), 8);
				const matches = results
					.map((item) => item.node)
					.filter(
						(node) =>
							(node.kind === 'template' || node.tags?.some((tag) => tag.toLowerCase() === 'template')) &&
							(readTemplateTargetKind(node) ?? '').toLowerCase() === kind.toLowerCase()
					);
				semanticMatches = matches;
			} catch {
				semanticMatches = [];
			} finally {
				searchPending = false;
			}
		}, 250);
	}

	$: scheduleSemanticSearch();

	$: filtered = templates.filter((template) => {
		if (!query.trim()) return true;
		const q = query.trim().toLowerCase();
		return (
			(template.title ?? '').toLowerCase().includes(q) ||
			(template.content ?? "").toLowerCase().includes(q) ||
			template.tags?.some((tag) => tag.toLowerCase().includes(q))
		);
	});

	$: merged = (() => {
		const map = new Map<string, KnowledgeNode>();
		for (const item of filtered) map.set(item.id, item);
		for (const item of semanticMatches) map.set(item.id, item);
		return Array.from(map.values());
	})();

	function applyTemplate(template: KnowledgeNode) {
		dispatch('apply', template);
		open = false;
	}
</script>

<div class="relative">
	<button
		class="rounded-md border border-slate-800 px-2 py-1 text-[11px] text-slate-300 hover:bg-slate-800"
		on:click={() => (open = !open)}
	>
		{label}
	</button>

	{#if open}
		<div class="absolute z-30 mt-2 w-full min-w-[260px] rounded-xl border border-slate-800 bg-slate-950 p-3 shadow-lg">
			<input
				class="w-full rounded-md border border-slate-800 bg-slate-900 px-3 py-2 text-xs text-white"
				placeholder="Search templates"
				bind:value={query}
			/>

			<div class="mt-3 flex flex-col gap-2">
				{#if loading}
					<p class="text-xs text-slate-500">Loading templates…</p>
				{:else if merged.length === 0}
					<p class="text-xs text-slate-500">No templates match.</p>
				{:else}
					{#each merged as template (template.id)}
						<button
							class="rounded-lg border border-slate-800 bg-slate-900/40 px-3 py-2 text-left text-xs text-slate-200 hover:border-slate-700"
							on:click={() => applyTemplate(template)}
						>
							<div class="flex items-center justify-between gap-2">
								<span class="font-semibold">{template.title || 'Untitled template'}</span>
								<span class="rounded bg-slate-800 px-1.5 py-0.5 text-[9px] text-slate-400">
									{readTemplateTargetKind(template) ?? kind}
								</span>
							</div>
							{#if readTemplateKey(template)}
								<p class="mt-1 text-[10px] text-slate-500">key: {readTemplateKey(template)}</p>
							{/if}
							<p class="mt-1 line-clamp-2 text-[10px] text-slate-500">
								{(template.content ?? "").slice(0, 140)}
							</p>
						</button>
					{/each}
				{/if}
			</div>

			{#if searchPending}
				<p class="mt-2 text-[10px] text-slate-500">Searching semantic matches…</p>
			{/if}
		</div>
	{/if}
</div>
