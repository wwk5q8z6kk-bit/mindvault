<script lang="ts">
	import { searchHybrid, type SearchResultDto } from '$lib/api/search';
	import { addRelationship } from '$lib/api/graph';
	import { pushToast } from '$lib/stores/toast';
	import { kindLabel, kindBadgeClass, RELATIONSHIP_TYPES } from '$lib/utils/kind-helpers';

	export let nodeId: string;
	export let content: string;

	let suggestions: SearchResultDto[] = [];
	let loading = false;
	let loaded = false;
	let expanded = false;
	let selectedRelType: string = 'RelatesTo';

	async function loadSuggestions() {
		if (loaded || loading) return;
		loading = true;
		try {
			const query = content.slice(0, 300);
			if (!query.trim()) {
				suggestions = [];
				loaded = true;
				return;
			}
			const results = await searchHybrid(query, 6);
			suggestions = results.filter((r) => r.node.id !== nodeId);
			loaded = true;
		} catch {
			suggestions = [];
		} finally {
			loading = false;
		}
	}

	function toggle() {
		expanded = !expanded;
		if (expanded && !loaded) {
			void loadSuggestions();
		}
	}

	async function linkNode(targetId: string) {
		try {
			await addRelationship(nodeId, targetId, selectedRelType);
			pushToast(`Connection created (${selectedRelType})`, 'success');
			suggestions = suggestions.filter((s) => s.node.id !== targetId);
		} catch {
			pushToast('Failed to create connection', 'danger');
		}
	}

	function nodeLink(result: SearchResultDto): string {
		if (result.node.kind === 'task') return `/tasks?task=${result.node.id}`;
		if (result.node.kind === 'fact') return `/notes?note=${result.node.id}`;
		return `/search?q=${encodeURIComponent(result.node.title)}`;
	}
</script>

<div class="rounded-2xl border border-slate-800/60 bg-slate-900/30">
	<button
		class="flex w-full items-center justify-between px-4 py-3 text-left"
		on:click={toggle}
	>
		<div class="flex items-center gap-2">
			<svg class="h-4 w-4 text-purple-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
				<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"
					d="M13.828 10.172a4 4 0 00-5.656 0l-4 4a4 4 0 105.656 5.656l1.102-1.101m-.758-4.899a4 4 0 005.656 0l4-4a4 4 0 00-5.656-5.656l-1.1 1.1" />
			</svg>
			<span class="text-xs font-semibold text-white">AI Suggested Connections</span>
			{#if loaded && suggestions.length > 0}
				<span class="rounded-full bg-purple-500/20 px-1.5 py-0.5 text-[9px] text-purple-300">
					{suggestions.length}
				</span>
			{/if}
		</div>
		<svg class="h-4 w-4 text-slate-500 transition {expanded ? 'rotate-180' : ''}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
			<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
		</svg>
	</button>

	{#if expanded}
		<div class="border-t border-slate-800/40 px-4 py-3">
			{#if loading}
				<p class="text-[11px] text-slate-500">Finding related items...</p>
			{:else if suggestions.length === 0}
				<p class="text-[11px] text-slate-500">No similar items found.</p>
			{:else}
				<div class="mb-2 flex items-center gap-2">
					<label class="text-[10px] text-slate-500" for="rel-type">Relationship:</label>
					<select
						id="rel-type"
						class="rounded-lg border border-slate-700 bg-slate-800 px-2 py-1 text-[10px] text-white"
						bind:value={selectedRelType}
					>
						{#each RELATIONSHIP_TYPES as relType}
							<option value={relType}>{relType}</option>
						{/each}
					</select>
				</div>
				<div class="flex flex-col gap-2">
					{#each suggestions as result (result.node.id)}
						<div class="flex items-center gap-2 rounded-lg border border-slate-800/40 bg-slate-900/40 px-3 py-2">
							<span class="rounded px-1.5 py-0.5 text-[9px] font-medium {kindBadgeClass(result.node.kind)}">
								{kindLabel(result.node.kind)}
							</span>
							<a
								href={nodeLink(result)}
								class="min-w-0 flex-1 truncate text-xs text-slate-300 hover:text-white"
							>
								{result.node.title}
							</a>
							<span class="text-[9px] text-slate-600">{(result.score * 100).toFixed(0)}%</span>
							<button
								class="rounded border border-purple-500/30 px-1.5 py-0.5 text-[9px] text-purple-300 transition hover:bg-purple-500/10"
								on:click={() => linkNode(result.node.id)}
								title="Create {selectedRelType} connection"
							>
								Link
							</button>
						</div>
					{/each}
				</div>
			{/if}
		</div>
	{/if}
</div>
