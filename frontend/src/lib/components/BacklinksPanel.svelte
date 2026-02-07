<script lang="ts">
	import { getNeighbors, getNodeRelationships, deleteRelationship, type GraphNeighbor, type NodeRelationship } from '$lib/api/graph';
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { kindLabel, kindBadgeClass } from '$lib/utils/kind-helpers';
	import { pushToast } from '$lib/stores/toast';

	export let nodeId: string;

	let neighbors: GraphNeighbor[] = [];
	let relationships: NodeRelationship[] = [];
	let loading = true;
	let error = false;
	let deletingId: string | null = null;

	function kindBadge(kind: string): { label: string; color: string } {
		return { label: kindLabel(kind), color: kindBadgeClass(kind) };
	}

	function navigateToNode(neighbor: GraphNeighbor) {
		const node = neighbor.node;
		if (node.kind === 'task') {
			goto(`/tasks?task=${node.id}`);
		} else {
			goto(`/notes?note=${node.id}`);
		}
	}

	/** Find the relationship ID that corresponds to a given neighbor. */
	function findRelationshipId(neighbor: GraphNeighbor): string | null {
		const match = relationships.find((r) => {
			if (neighbor.direction === 'incoming') {
				return r.from_node_id === neighbor.node.id && r.to_node_id === nodeId && r.kind === neighbor.relationship_kind;
			} else {
				return r.from_node_id === nodeId && r.to_node_id === neighbor.node.id && r.kind === neighbor.relationship_kind;
			}
		});
		return match?.id ?? null;
	}

	async function handleDelete(neighbor: GraphNeighbor) {
		const relId = findRelationshipId(neighbor);
		if (!relId) {
			pushToast('Could not find relationship to delete', 'danger');
			return;
		}
		deletingId = relId;
		try {
			await deleteRelationship(relId);
			neighbors = neighbors.filter((n) => n !== neighbor);
			relationships = relationships.filter((r) => r.id !== relId);
			pushToast('Relationship removed', 'success');
		} catch {
			pushToast('Failed to delete relationship', 'danger');
		} finally {
			deletingId = null;
		}
	}

	async function loadNeighbors() {
		loading = true;
		error = false;
		try {
			const [neighborsRes, relsRes] = await Promise.all([
				getNeighbors(nodeId),
				getNodeRelationships(nodeId)
			]);
			neighbors = neighborsRes.neighbors;
			relationships = [...relsRes.incoming, ...relsRes.outgoing];
		} catch {
			error = true;
			neighbors = [];
			relationships = [];
		} finally {
			loading = false;
		}
	}

	onMount(() => {
		loadNeighbors();
	});

	$: if (nodeId) {
		loadNeighbors();
	}
</script>

{#if loading}
	<div class="text-[10px] text-slate-500">Loading backlinks...</div>
{:else if error}
	<div class="text-[10px] text-slate-500">Could not load backlinks</div>
{:else if neighbors.length > 0}
	<div class="rounded-xl border border-slate-800 bg-slate-900/40 p-4">
		<h4 class="mb-2 text-xs font-semibold uppercase tracking-wide text-slate-500">
			Backlinks & Connections ({neighbors.length})
		</h4>
		<div class="flex flex-col gap-1">
			{#each neighbors as neighbor (neighbor.node.id)}
				{@const badge = kindBadge(neighbor.node.kind)}
				{@const relId = findRelationshipId(neighbor)}
				<div class="group flex items-center gap-1">
					<button
						class="flex flex-1 items-center gap-2 rounded-lg px-2 py-1.5 text-left text-xs transition hover:bg-slate-800"
						on:click={() => navigateToNode(neighbor)}
					>
						<span class={`rounded-full px-1.5 py-0.5 text-[9px] font-medium ${badge.color}`}>
							{badge.label}
						</span>
						<span class="flex-1 truncate text-slate-300">{neighbor.node.title}</span>
						<span class="text-[9px] text-slate-600">{neighbor.relationship_kind}</span>
						<span class="text-[9px] text-slate-600">
							{neighbor.direction === 'incoming' ? '\u2190' : '\u2192'}
						</span>
					</button>
					<button
						class="flex-shrink-0 rounded p-1 text-slate-600 opacity-0 transition hover:bg-red-500/10 hover:text-red-400 group-hover:opacity-100"
						title="Remove relationship"
						disabled={deletingId === relId}
						on:click|stopPropagation={() => handleDelete(neighbor)}
					>
						<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2">
							<path d="M18 6L6 18M6 6l12 12"/>
						</svg>
					</button>
				</div>
			{/each}
		</div>
	</div>
{/if}
