<script lang="ts">
	import { createEventDispatcher, onMount } from 'svelte';
	import {
		getNodeRelationships,
		getGraphClusters,
		findPaths,
		searchGraph
	} from '$lib/api/graph';
	import type {
		NodeRelationship,
		GraphCluster,
		GraphPath,
		GraphSearchResult
	} from '$lib/api/graph';
	import type { KnowledgeNode } from '$lib/api/types';
	import { pushToast } from '$lib/stores/toast';
	import { goto } from '$app/navigation';

	export let nodeId: string | null = null;
	export let mode: 'relationships' | 'clusters' | 'paths' | 'search' = 'relationships';

	const dispatch = createEventDispatcher<{
		nodeSelect: KnowledgeNode;
	}>();

	// Relationships mode
	let incoming: NodeRelationship[] = [];
	let outgoing: NodeRelationship[] = [];
	let loadingRelationships = false;

	// Clusters mode
	let clusters: GraphCluster[] = [];
	let loadingClusters = false;
	let selectedCluster: GraphCluster | null = null;

	// Paths mode
	let pathStartId = '';
	let pathEndId = '';
	let paths: GraphPath[] = [];
	let loadingPaths = false;

	// Search mode
	let searchStartId = '';
	let searchResults: GraphSearchResult[] = [];
	let loadingSearch = false;
	let maxDepth = 3;
	let relationshipKinds: string[] = [];

	$: if (nodeId && mode === 'relationships') {
		loadRelationships();
	}

	onMount(() => {
		if (mode === 'clusters') {
			loadClusters();
		}
	});

	async function loadRelationships() {
		if (!nodeId) return;
		loadingRelationships = true;
		try {
			const response = await getNodeRelationships(nodeId);
			incoming = response.incoming;
			outgoing = response.outgoing;
		} catch (err) {
			pushToast('Failed to load relationships', 'danger');
		} finally {
			loadingRelationships = false;
		}
	}

	async function loadClusters() {
		loadingClusters = true;
		try {
			const response = await getGraphClusters({ min_size: 2, max_clusters: 20 });
			clusters = response.clusters;
		} catch (err) {
			pushToast('Failed to load clusters', 'danger');
		} finally {
			loadingClusters = false;
		}
	}

	async function searchPaths() {
		if (!pathStartId || !pathEndId) {
			pushToast('Please enter both start and end node IDs', 'warning');
			return;
		}
		loadingPaths = true;
		try {
			const response = await findPaths(pathStartId, pathEndId, { max_depth: 5, max_paths: 5 });
			paths = response.paths;
			if (paths.length === 0) {
				pushToast('No paths found between these nodes', 'info');
			}
		} catch (err) {
			pushToast('Failed to find paths', 'danger');
		} finally {
			loadingPaths = false;
		}
	}

	async function performGraphSearch() {
		if (!searchStartId) {
			pushToast('Please enter a starting node ID', 'warning');
			return;
		}
		loadingSearch = true;
		try {
			const response = await searchGraph({
				start_id: searchStartId,
				max_depth: maxDepth,
				relationship_kinds: relationshipKinds.length > 0 ? relationshipKinds : undefined
			});
			searchResults = response.results;
			if (searchResults.length === 0) {
				pushToast('No connected nodes found', 'info');
			}
		} catch (err) {
			pushToast('Failed to search graph', 'danger');
		} finally {
			loadingSearch = false;
		}
	}

	function navigateToNode(id: string) {
		goto(`/notes/${id}`);
	}

	function getRelationshipColor(kind: string): string {
		const colors: Record<string, string> = {
			'references': '#3b82f6',
			'related': '#8b5cf6',
			'parent': '#22c55e',
			'child': '#22c55e',
			'blocks': '#ef4444',
			'depends_on': '#f59e0b'
		};
		return colors[kind] || '#6b7280';
	}
</script>

<div class="graph-explorer">
	<div class="mode-tabs">
		<button class:active={mode === 'relationships'} on:click={() => (mode = 'relationships')}>
			Relationships
		</button>
		<button class:active={mode === 'clusters'} on:click={() => (mode = 'clusters')}>
			Clusters
		</button>
		<button class:active={mode === 'paths'} on:click={() => (mode = 'paths')}>
			Paths
		</button>
		<button class:active={mode === 'search'} on:click={() => (mode = 'search')}>
			Search
		</button>
	</div>

	<div class="explorer-content">
		{#if mode === 'relationships'}
			<div class="relationships-view">
				{#if !nodeId}
					<div class="empty-state">
						<p>Select a note to view its relationships</p>
					</div>
				{:else if loadingRelationships}
					<div class="loading">Loading relationships...</div>
				{:else}
					<div class="relationship-section">
						<h3>Incoming ({incoming.length})</h3>
						{#if incoming.length === 0}
							<p class="empty">No incoming relationships</p>
						{:else}
							<ul class="relationship-list">
								{#each incoming as rel (rel.id)}
									<li>
										<span
											class="rel-kind"
											style="background: {getRelationshipColor(rel.kind)}"
										>
											{rel.kind}
										</span>
										<button
											class="rel-node"
											on:click={() => navigateToNode(rel.from_node_id)}
										>
											{rel.from_node_id}
										</button>
									</li>
								{/each}
							</ul>
						{/if}
					</div>

					<div class="relationship-section">
						<h3>Outgoing ({outgoing.length})</h3>
						{#if outgoing.length === 0}
							<p class="empty">No outgoing relationships</p>
						{:else}
							<ul class="relationship-list">
								{#each outgoing as rel (rel.id)}
									<li>
										<span
											class="rel-kind"
											style="background: {getRelationshipColor(rel.kind)}"
										>
											{rel.kind}
										</span>
										<button
											class="rel-node"
											on:click={() => navigateToNode(rel.to_node_id)}
										>
											{rel.to_node_id}
										</button>
									</li>
								{/each}
							</ul>
						{/if}
					</div>
				{/if}
			</div>
		{:else if mode === 'clusters'}
			<div class="clusters-view">
				{#if loadingClusters}
					<div class="loading">Analyzing clusters...</div>
				{:else if clusters.length === 0}
					<div class="empty-state">
						<p>No clusters found. Add more notes and relationships to see clusters.</p>
					</div>
				{:else}
					<div class="clusters-list">
						{#each clusters as cluster (cluster.id)}
							<button
								class="cluster-item"
								class:selected={selectedCluster?.id === cluster.id}
								on:click={() => (selectedCluster = cluster)}
							>
								<span class="cluster-name">{cluster.name || `Cluster ${cluster.id}`}</span>
								<span class="cluster-size">{cluster.node_ids.length} notes</span>
								<span class="cluster-density">
									Density: {(cluster.density * 100).toFixed(0)}%
								</span>
							</button>
						{/each}
					</div>
					{#if selectedCluster}
						<div class="cluster-detail">
							<h3>{selectedCluster.name || `Cluster ${selectedCluster.id}`}</h3>
							<p>Center: <button class="link" on:click={() => navigateToNode(selectedCluster?.center_node_id || '')}>{selectedCluster.center_node_id}</button></p>
							<ul class="cluster-nodes">
								{#each selectedCluster.node_ids as id}
									<li>
										<button class="link" on:click={() => navigateToNode(id)}>{id}</button>
									</li>
								{/each}
							</ul>
						</div>
					{/if}
				{/if}
			</div>
		{:else if mode === 'paths'}
			<div class="paths-view">
				<div class="paths-form">
					<div class="form-row">
						<label>
							From:
							<input type="text" bind:value={pathStartId} placeholder="Node ID" />
						</label>
						<label>
							To:
							<input type="text" bind:value={pathEndId} placeholder="Node ID" />
						</label>
					</div>
					<button class="btn-search" on:click={searchPaths} disabled={loadingPaths}>
						{loadingPaths ? 'Finding...' : 'Find Paths'}
					</button>
				</div>

				{#if paths.length > 0}
					<div class="paths-results">
						<h3>Found {paths.length} path(s)</h3>
						{#each paths as path, i}
							<div class="path-item">
								<span class="path-number">Path {i + 1}</span>
								<span class="path-weight">Weight: {path.total_weight.toFixed(2)}</span>
								<div class="path-nodes">
									{#each path.nodes as nodeIdInPath, j}
										<button class="link" on:click={() => navigateToNode(nodeIdInPath)}>
											{nodeIdInPath}
										</button>
										{#if j < path.nodes.length - 1}
											<span class="path-arrow">→</span>
										{/if}
									{/each}
								</div>
							</div>
						{/each}
					</div>
				{/if}
			</div>
		{:else if mode === 'search'}
			<div class="search-view">
				<div class="search-form">
					<label>
						Starting Node:
						<input type="text" bind:value={searchStartId} placeholder="Node ID" />
					</label>
					<label>
						Max Depth:
						<input type="number" bind:value={maxDepth} min="1" max="10" />
					</label>
					<button class="btn-search" on:click={performGraphSearch} disabled={loadingSearch}>
						{loadingSearch ? 'Searching...' : 'Search Graph'}
					</button>
				</div>

				{#if searchResults.length > 0}
					<div class="search-results">
						<h3>Found {searchResults.length} connected nodes</h3>
						<ul class="result-list">
							{#each searchResults as result (result.node.id)}
								<li class="result-item">
									<button
										class="result-title"
										on:click={() => navigateToNode(result.node.id)}
									>
										{result.node.title}
									</button>
									<div class="result-meta">
										<span>Distance: {result.path_length}</span>
										<span>Score: {result.score.toFixed(2)}</span>
									</div>
									{#if result.via_relationships.length > 0}
										<div class="result-path">
											via: {result.via_relationships.join(' → ')}
										</div>
									{/if}
								</li>
							{/each}
						</ul>
					</div>
				{/if}
			</div>
		{/if}
	</div>
</div>

<style>
	.graph-explorer {
		display: flex;
		flex-direction: column;
		height: 100%;
		background: var(--surface-color, #fff);
		border-radius: 12px;
		overflow: hidden;
	}

	.mode-tabs {
		display: flex;
		border-bottom: 1px solid var(--border-color, #e0e0e0);
	}

	.mode-tabs button {
		flex: 1;
		padding: 0.75rem;
		border: none;
		background: transparent;
		font-size: 0.875rem;
		cursor: pointer;
		color: var(--text-muted, #6b7280);
		border-bottom: 2px solid transparent;
		margin-bottom: -1px;
	}

	.mode-tabs button.active {
		color: var(--primary-color, #3b82f6);
		border-bottom-color: var(--primary-color, #3b82f6);
	}

	.explorer-content {
		flex: 1;
		overflow: auto;
		padding: 1rem;
	}

	.empty-state, .loading {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100%;
		color: var(--text-muted, #6b7280);
	}

	.relationship-section {
		margin-bottom: 1.5rem;
	}

	.relationship-section h3 {
		font-size: 0.875rem;
		margin: 0 0 0.5rem;
		color: var(--text-muted, #6b7280);
	}

	.relationship-list {
		list-style: none;
		padding: 0;
		margin: 0;
	}

	.relationship-list li {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.5rem 0;
		border-bottom: 1px solid var(--border-color, #e0e0e0);
	}

	.rel-kind {
		padding: 0.125rem 0.5rem;
		border-radius: 4px;
		font-size: 0.75rem;
		color: white;
	}

	.rel-node, .link {
		background: none;
		border: none;
		color: var(--primary-color, #3b82f6);
		cursor: pointer;
		font-size: 0.875rem;
	}

	.rel-node:hover, .link:hover {
		text-decoration: underline;
	}

	.empty {
		color: var(--text-muted, #6b7280);
		font-size: 0.875rem;
	}

	/* Clusters */
	.clusters-view {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 1rem;
	}

	.clusters-list {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.cluster-item {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		padding: 0.75rem;
		background: var(--bg-muted, #f9fafb);
		border: 1px solid var(--border-color, #e0e0e0);
		border-radius: 8px;
		text-align: left;
		cursor: pointer;
	}

	.cluster-item.selected {
		border-color: var(--primary-color, #3b82f6);
		background: var(--primary-bg, #eff6ff);
	}

	.cluster-name {
		font-weight: 500;
	}

	.cluster-size, .cluster-density {
		font-size: 0.75rem;
		color: var(--text-muted, #6b7280);
	}

	.cluster-detail h3 {
		margin: 0 0 0.5rem;
	}

	.cluster-nodes {
		list-style: none;
		padding: 0;
		margin: 0;
	}

	.cluster-nodes li {
		padding: 0.25rem 0;
	}

	/* Paths */
	.paths-form, .search-form {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		margin-bottom: 1rem;
	}

	.form-row {
		display: flex;
		gap: 0.75rem;
	}

	.form-row label {
		flex: 1;
	}

	label {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		font-size: 0.875rem;
	}

	input {
		padding: 0.5rem;
		border: 1px solid var(--border-color, #e0e0e0);
		border-radius: 6px;
		font-size: 0.875rem;
	}

	.btn-search {
		padding: 0.5rem 1rem;
		background: var(--primary-color, #3b82f6);
		color: white;
		border: none;
		border-radius: 6px;
		font-size: 0.875rem;
		cursor: pointer;
	}

	.btn-search:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.paths-results h3, .search-results h3 {
		font-size: 0.875rem;
		margin: 0 0 0.75rem;
	}

	.path-item {
		padding: 0.75rem;
		background: var(--bg-muted, #f9fafb);
		border-radius: 8px;
		margin-bottom: 0.5rem;
	}

	.path-number {
		font-weight: 500;
		margin-right: 0.5rem;
	}

	.path-weight {
		font-size: 0.75rem;
		color: var(--text-muted, #6b7280);
	}

	.path-nodes {
		margin-top: 0.5rem;
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.25rem;
	}

	.path-arrow {
		color: var(--text-muted, #6b7280);
	}

	/* Search results */
	.result-list {
		list-style: none;
		padding: 0;
		margin: 0;
	}

	.result-item {
		padding: 0.75rem;
		border-bottom: 1px solid var(--border-color, #e0e0e0);
	}

	.result-title {
		font-weight: 500;
		margin-bottom: 0.25rem;
	}

	.result-meta {
		display: flex;
		gap: 1rem;
		font-size: 0.75rem;
		color: var(--text-muted, #6b7280);
	}

	.result-path {
		font-size: 0.75rem;
		color: var(--text-secondary, #4b5563);
		margin-top: 0.25rem;
	}
</style>
