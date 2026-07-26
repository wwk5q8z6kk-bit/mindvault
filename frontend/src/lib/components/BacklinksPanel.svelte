<script lang="ts">
	import {
		getNodeBacklinks,
		deleteRelationship,
		type NodeBacklinkEdge
	} from '$lib/api/graph';
	import { goto } from '$app/navigation';
	import { kindLabel, kindBadgeClass } from '$lib/utils/kind-helpers';
	import { pushToast } from '$lib/stores/toast';

	export let nodeId: string;

	let backlinks: NodeBacklinkEdge[] = [];
	let totalBacklinks = 0;
	let hasMore = false;
	let loading = true;
	let error = false;
	let deletingId: string | null = null;
	let includeAuto = true;
	let includeManual = true;
	let loadingMore = false;

	const PAGE_SIZE = 20;

	function kindBadge(kind: string): { label: string; color: string } {
		return { label: kindLabel(kind), color: kindBadgeClass(kind) };
	}

	function navigateToBacklink(edge: NodeBacklinkEdge) {
		if (edge.related_node_kind === 'task') {
			goto(`/tasks?task=${edge.related_node_id}`);
		} else {
			goto(`/notes?note=${edge.related_node_id}`);
		}
	}

	async function handleDelete(edge: NodeBacklinkEdge) {
		deletingId = edge.relationship_id;
		try {
			await deleteRelationship(edge.relationship_id);
			backlinks = backlinks.filter((b) => b.relationship_id !== edge.relationship_id);
			totalBacklinks = Math.max(0, totalBacklinks - 1);
			pushToast('Backlink removed', 'success');
		} catch {
			pushToast('Failed to delete backlink', 'danger');
		} finally {
			deletingId = null;
		}
	}

	async function loadBacklinks(options: { reset?: boolean; soft?: boolean } = {}) {
		const reset = options.reset ?? true;
		const soft = options.soft ?? false;
		const requestNodeId = nodeId;

		if (reset) {
			if (!soft) {
				loading = true;
				backlinks = [];
				hasMore = false;
				totalBacklinks = 0;
			}
			error = false;
		} else {
			loadingMore = true;
		}

		try {
			const offset = reset ? 0 : backlinks.length;
			const res = await getNodeBacklinks(requestNodeId, {
				limit: PAGE_SIZE,
				offset,
				include_auto: includeAuto,
				include_manual: includeManual
			});
			// Ignore stale responses after the selected note changes.
			if (requestNodeId !== nodeId) return;
			backlinks = reset ? res.backlinks : [...backlinks, ...res.backlinks];
			totalBacklinks = res.total_backlinks;
			hasMore = res.has_more;
		} catch {
			if (requestNodeId !== nodeId) return;
			if (reset) {
				error = true;
				backlinks = [];
				totalBacklinks = 0;
				hasMore = false;
			} else {
				pushToast('Failed to load more backlinks', 'danger');
			}
		} finally {
			if (requestNodeId === nodeId) {
				loading = false;
				loadingMore = false;
			}
		}
	}

	function setFilter(nextAuto: boolean, nextManual: boolean) {
		if (!nextAuto && !nextManual) return;
		includeAuto = nextAuto;
		includeManual = nextManual;
		void loadBacklinks({ reset: true, soft: true });
	}

	let loadedFor: string | null = null;
	$: if (nodeId && nodeId !== loadedFor) {
		loadedFor = nodeId;
		void loadBacklinks({ reset: true, soft: false });
	}
</script>

{#if loading}
	<div class="text-[10px] text-slate-500">Loading backlinks...</div>
{:else if error}
	<div class="flex items-center justify-between gap-2 text-[10px] text-slate-500">
		<span>Could not load backlinks</span>
		<button
			class="rounded border border-slate-700 px-1.5 py-0.5 text-[10px] text-slate-400 hover:text-white"
			on:click={() => loadBacklinks({ reset: true })}
		>
			Retry
		</button>
	</div>
{:else}
	<div class="rounded-xl border border-slate-800 bg-slate-900/40 p-4">
		<div class="mb-2 flex items-center justify-between gap-2">
			<h4 class="text-xs font-semibold uppercase tracking-wide text-slate-500">
				Backlinks ({totalBacklinks})
			</h4>
			<div class="flex rounded-md border border-slate-800 text-[9px]" role="group" aria-label="Backlink filters">
				<button
					class={`px-1.5 py-0.5 ${includeAuto && includeManual ? 'bg-slate-800 text-slate-200' : 'text-slate-500 hover:text-slate-300'}`}
					aria-pressed={includeAuto && includeManual}
					on:click={() => setFilter(true, true)}
				>
					All
				</button>
				<button
					class={`px-1.5 py-0.5 ${includeAuto && !includeManual ? 'bg-slate-800 text-slate-200' : 'text-slate-500 hover:text-slate-300'}`}
					aria-pressed={includeAuto && !includeManual}
					title="Wiki-link / auto-generated references"
					on:click={() => setFilter(true, false)}
				>
					Auto
				</button>
				<button
					class={`px-1.5 py-0.5 ${!includeAuto && includeManual ? 'bg-slate-800 text-slate-200' : 'text-slate-500 hover:text-slate-300'}`}
					aria-pressed={!includeAuto && includeManual}
					title="Manually created references"
					on:click={() => setFilter(false, true)}
				>
					Manual
				</button>
			</div>
		</div>

		{#if backlinks.length === 0}
			<p class="text-[11px] text-slate-500">
				No reference backlinks yet. Link notes with <code class="text-slate-400">[[wiki-links]]</code> to build this graph.
			</p>
		{:else}
			<div class="flex flex-col gap-1">
				{#each backlinks as edge (edge.relationship_id)}
					{@const badge = kindBadge(edge.related_node_kind)}
					<div class="group flex items-center gap-1">
						<button
							class="flex flex-1 items-center gap-2 rounded-lg px-2 py-1.5 text-left text-xs transition hover:bg-slate-800"
							on:click={() => navigateToBacklink(edge)}
						>
							<span class={`rounded-full px-1.5 py-0.5 text-[9px] font-medium ${badge.color}`}>
								{badge.label}
							</span>
							<span class="flex-1 truncate text-slate-300">
								{edge.related_node_title || 'Untitled'}
							</span>
							{#if edge.auto_managed}
								<span
									class="rounded bg-sky-500/10 px-1 py-0.5 text-[9px] text-sky-300"
									title={edge.auto_source ? `Source: ${edge.auto_source}` : 'Auto-managed backlink'}
								>
									auto
								</span>
							{:else}
								<span class="rounded bg-slate-800 px-1 py-0.5 text-[9px] text-slate-500">manual</span>
							{/if}
							<span class="text-[9px] text-slate-600">←</span>
						</button>
						<button
							class="flex-shrink-0 rounded p-1 text-slate-600 opacity-0 transition hover:bg-red-500/10 hover:text-red-400 group-hover:opacity-100"
							title="Remove backlink"
							disabled={deletingId === edge.relationship_id}
							on:click|stopPropagation={() => handleDelete(edge)}
						>
							<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2">
								<path d="M18 6L6 18M6 6l12 12" />
							</svg>
						</button>
					</div>
				{/each}
			</div>
			{#if hasMore}
				<button
					class="mt-2 w-full rounded-lg border border-slate-800 px-2 py-1.5 text-[10px] text-slate-400 hover:bg-slate-800 hover:text-slate-200 disabled:opacity-50"
					disabled={loadingMore}
					on:click={() => loadBacklinks({ reset: false })}
				>
					{loadingMore ? 'Loading…' : `Show more (${backlinks.length} of ${totalBacklinks})`}
				</button>
			{/if}
		{/if}
	</div>
{/if}
