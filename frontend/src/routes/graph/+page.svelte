<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { listNodes } from '$lib/api/nodes';
	import { getNeighbors, addRelationship, getNodeRelationships, deleteRelationship, getGraphClusters } from '$lib/api/graph';
	import type { NodeRelationship, GraphCluster } from '$lib/api/graph';
	import type { KnowledgeNode } from '$lib/api/types';
	import { pushToast } from '$lib/stores/toast';
	import { kindColor, kindLabel, ALL_NODE_KINDS, RELATIONSHIP_TYPES, type RelationshipType } from '$lib/utils/kind-helpers';

	type GraphNode = {
		id: string;
		title: string;
		kind: string;
		x: number;
		y: number;
		vx: number;
		vy: number;
	};

	type GraphEdge = {
		source: string;
		target: string;
		kind: string;
	};

	const RELATIONSHIP_COLORS: Record<string, string> = {
		RelatesTo: '#94a3b8',
		DependsOn: '#f97316',
		DerivedFrom: '#a78bfa',
		Supersedes: '#ef4444',
		Contradicts: '#dc2626',
		PartOf: '#22c55e',
		Contains: '#14b8a6',
		References: '#38bdf8',
		SimilarTo: '#8b5cf6',
		FollowsFrom: '#eab308'
	};

	function getRelationshipColor(kind: string): string {
		return RELATIONSHIP_COLORS[kind] ?? '#94a3b8';
	}

	let nodes: GraphNode[] = [];
	let edges: GraphEdge[] = [];
	let loading = true;
	let selectedNodeId: string | null = null;
	let svgEl: SVGSVGElement | null = null;
	let width = 800;
	let height = 600;
	let animFrame: number;
	let dragging: string | null = null;
	let showLabels = true;
	let kindFilter: string | 'all' = 'all';
	let relationshipFilter: string | 'all' = 'all';

	// Relationship creation state
	let creatingRelationship = false;
	let relationshipFromId: string | null = null;
	let selectedRelationshipType: RelationshipType = 'RelatesTo';
	let showRelationshipModal = false;
	let relationshipToId: string | null = null;

	// Selected node relationships
	let selectedNodeRelationships: NodeRelationship[] = [];
	let loadingRelationships = false;
	let deletingRelId: string | null = null;

	// Cluster visualization
	let showClusters = false;
	let clusters: GraphCluster[] = [];
	let loadingClusters = false;
	let nodeClusterMap: Map<string, number> = new Map();

	const CLUSTER_COLORS = [
		'#38bdf8', '#22c55e', '#f97316', '#a78bfa', '#ec4899',
		'#eab308', '#14b8a6', '#ef4444', '#8b5cf6', '#06b6d4'
	];

	function getNodeColor(node: GraphNode): string {
		if (showClusters && nodeClusterMap.has(node.id)) {
			const clusterIdx = nodeClusterMap.get(node.id)!;
			return CLUSTER_COLORS[clusterIdx % CLUSTER_COLORS.length];
		}
		return kindColor(node.kind);
	}

	async function loadClusters() {
		loadingClusters = true;
		try {
			const result = await getGraphClusters({ min_size: 2, max_clusters: 10 });
			clusters = result.clusters;
			// Build node -> cluster mapping
			nodeClusterMap = new Map();
			clusters.forEach((cluster, idx) => {
				cluster.node_ids.forEach((nodeId) => {
					nodeClusterMap.set(nodeId, idx);
				});
			});
		} catch {
			pushToast('Failed to load clusters', 'warning');
		} finally {
			loadingClusters = false;
		}
	}

	async function toggleClusters() {
		showClusters = !showClusters;
		if (showClusters && clusters.length === 0) {
			await loadClusters();
		}
	}

	const KINDS = ALL_NODE_KINDS;

	$: displayedNodes = kindFilter === 'all' ? nodes : nodes.filter((n) => n.kind === kindFilter);
	$: displayedNodeIds = new Set(displayedNodes.map((n) => n.id));
	$: filteredByRelationship = relationshipFilter === 'all'
		? edges
		: edges.filter((e) => e.kind === relationshipFilter);
	$: displayedEdges = filteredByRelationship.filter(
		(e) => displayedNodeIds.has(e.source) && displayedNodeIds.has(e.target)
	);
	$: uniqueRelationshipTypes = [...new Set(edges.map((e) => e.kind))].sort();

	onMount(async () => {
		try {
			const allNodes = await listNodes({ limit: 100 });
			const cx = width / 2;
			const cy = height / 2;

			nodes = allNodes.map((node, i) => {
				const angle = (2 * Math.PI * i) / allNodes.length;
				const radius = 150 + Math.random() * 100;
				return {
					id: node.id,
					title: node.title || 'Untitled',
					kind: node.kind,
					x: cx + Math.cos(angle) * radius,
					y: cy + Math.sin(angle) * radius,
					vx: 0,
					vy: 0
				};
			});

			// Load edges from graph neighbors for first few nodes
			const edgeSet = new Set<string>();
			for (const node of allNodes.slice(0, 30)) {
				try {
					const result = await getNeighbors(node.id, 1);
					for (const neighbor of result.neighbors) {
						const key = [node.id, neighbor.node.id].sort().join('-');
						if (!edgeSet.has(key)) {
							edgeSet.add(key);
							edges = [...edges, {
								source: neighbor.direction === 'outgoing' ? node.id : neighbor.node.id,
								target: neighbor.direction === 'outgoing' ? neighbor.node.id : node.id,
								kind: neighbor.relationship_kind
							}];
						}
					}
				} catch {
					// Node may not have neighbors
				}
			}

			loading = false;
			simulate();
		} catch {
			loading = false;
			pushToast('Failed to load graph', 'danger');
		}
	});

	onDestroy(() => {
		if (animFrame) cancelAnimationFrame(animFrame);
	});

	function simulate() {
		const nodeMap = new Map(nodes.map((n) => [n.id, n]));
		let iterations = 0;

		function tick() {
			if (iterations > 300) return;
			iterations++;

			// Repulsion between all nodes
			for (let i = 0; i < nodes.length; i++) {
				for (let j = i + 1; j < nodes.length; j++) {
					const a = nodes[i];
					const b = nodes[j];
					let dx = b.x - a.x;
					let dy = b.y - a.y;
					const dist = Math.sqrt(dx * dx + dy * dy) || 1;
					const force = 2000 / (dist * dist);
					dx = (dx / dist) * force;
					dy = (dy / dist) * force;
					a.vx -= dx;
					a.vy -= dy;
					b.vx += dx;
					b.vy += dy;
				}
			}

			// Attraction along edges
			for (const edge of edges) {
				const a = nodeMap.get(edge.source);
				const b = nodeMap.get(edge.target);
				if (!a || !b) continue;
				let dx = b.x - a.x;
				let dy = b.y - a.y;
				const dist = Math.sqrt(dx * dx + dy * dy) || 1;
				const force = (dist - 100) * 0.01;
				dx = (dx / dist) * force;
				dy = (dy / dist) * force;
				a.vx += dx;
				a.vy += dy;
				b.vx -= dx;
				b.vy -= dy;
			}

			// Center gravity
			const cx = width / 2;
			const cy = height / 2;
			for (const node of nodes) {
				node.vx += (cx - node.x) * 0.001;
				node.vy += (cy - node.y) * 0.001;
			}

			// Apply velocity with damping
			for (const node of nodes) {
				if (node.id === dragging) continue;
				node.vx *= 0.9;
				node.vy *= 0.9;
				node.x += node.vx;
				node.y += node.vy;
				// Bounds
				node.x = Math.max(20, Math.min(width - 20, node.x));
				node.y = Math.max(20, Math.min(height - 20, node.y));
			}

			nodes = [...nodes];
			animFrame = requestAnimationFrame(tick);
		}

		tick();
	}

	function handleMouseDown(event: MouseEvent, nodeId: string) {
		dragging = nodeId;
		event.preventDefault();
	}

	function handleMouseMove(event: MouseEvent) {
		if (!dragging || !svgEl) return;
		const rect = svgEl.getBoundingClientRect();
		const node = nodes.find((n) => n.id === dragging);
		if (node) {
			node.x = event.clientX - rect.left;
			node.y = event.clientY - rect.top;
			node.vx = 0;
			node.vy = 0;
			nodes = [...nodes];
		}
	}

	function handleMouseUp() {
		dragging = null;
	}

	function nodeLink(node: GraphNode): string {
		if (node.kind === 'task') return `/tasks?task=${node.id}`;
		if (node.kind === 'fact') return `/notes?note=${node.id}`;
		return `/search?q=${encodeURIComponent(node.title)}`;
	}

	function startRelationshipCreation(nodeId: string) {
		if (creatingRelationship && relationshipFromId) {
			// Second click - complete the relationship
			if (nodeId !== relationshipFromId) {
				relationshipToId = nodeId;
				showRelationshipModal = true;
			}
		} else {
			// First click - start creating
			creatingRelationship = true;
			relationshipFromId = nodeId;
			pushToast('Click another node to create relationship', 'info');
		}
	}

	function cancelRelationshipCreation() {
		creatingRelationship = false;
		relationshipFromId = null;
		relationshipToId = null;
		showRelationshipModal = false;
	}

	async function loadSelectedNodeRelationships(nid: string) {
		loadingRelationships = true;
		try {
			const res = await getNodeRelationships(nid);
			selectedNodeRelationships = [...res.incoming, ...res.outgoing];
		} catch {
			selectedNodeRelationships = [];
		} finally {
			loadingRelationships = false;
		}
	}

	async function handleDeleteRelationship(rel: NodeRelationship) {
		deletingRelId = rel.id;
		try {
			await deleteRelationship(rel.id);
			selectedNodeRelationships = selectedNodeRelationships.filter((r) => r.id !== rel.id);
			// Also remove matching edge from the graph display
			edges = edges.filter(
				(e) =>
					!(
						(e.source === rel.from_node_id && e.target === rel.to_node_id && e.kind === rel.kind) ||
						(e.source === rel.to_node_id && e.target === rel.from_node_id && e.kind === rel.kind)
					)
			);
			pushToast('Relationship deleted', 'success');
		} catch {
			pushToast('Failed to delete relationship', 'danger');
		} finally {
			deletingRelId = null;
		}
	}

	// Load relationships when a node is selected
	$: if (selectedNodeId) {
		loadSelectedNodeRelationships(selectedNodeId);
	} else {
		selectedNodeRelationships = [];
	}

	async function confirmCreateRelationship() {
		if (!relationshipFromId || !relationshipToId) return;

		try {
			await addRelationship(relationshipFromId, relationshipToId, selectedRelationshipType);

			// Add to local edges
			edges = [...edges, {
				source: relationshipFromId,
				target: relationshipToId,
				kind: selectedRelationshipType
			}];

			pushToast(`Created ${selectedRelationshipType} relationship`, 'success');
		} catch {
			pushToast('Failed to create relationship', 'danger');
		} finally {
			cancelRelationshipCreation();
		}
	}
</script>

<svelte:window on:mousemove={handleMouseMove} on:mouseup={handleMouseUp} />

<div class="flex flex-col gap-4">
	<div class="flex items-center justify-between">
		<div>
			<h2 class="text-lg font-semibold text-white">Knowledge Graph</h2>
			<p class="text-xs text-slate-400">{nodes.length} nodes, {edges.length} connections</p>
		</div>
		<div class="flex flex-wrap items-center gap-3">
			<label class="flex items-center gap-1.5 text-[10px] text-slate-400">
				<input type="checkbox" bind:checked={showLabels} class="rounded border-slate-600" />
				Labels
			</label>
			<select
				class="rounded-lg border border-slate-700 bg-slate-800 px-2 py-1 text-xs text-white"
				bind:value={kindFilter}
				aria-label="Filter by kind"
			>
				<option value="all">All types</option>
				{#each KINDS as kind}
					<option value={kind}>{kindLabel(kind)}</option>
				{/each}
			</select>
			<select
				class="rounded-lg border border-slate-700 bg-slate-800 px-2 py-1 text-xs text-white"
				bind:value={relationshipFilter}
				aria-label="Filter by relationship"
			>
				<option value="all">All relationships</option>
				{#each uniqueRelationshipTypes as relType}
					<option value={relType}>{relType}</option>
				{/each}
			</select>
			{#if creatingRelationship}
				<button
					class="rounded-lg border border-red-500/40 bg-red-500/10 px-2 py-1 text-[10px] text-red-300"
					on:click={cancelRelationshipCreation}
				>
					Cancel linking
				</button>
			{:else}
				<button
					class="rounded-lg border border-emerald-500/40 bg-emerald-500/10 px-2 py-1 text-[10px] text-emerald-300"
					on:click={() => { creatingRelationship = true; pushToast('Click a node to start', 'info'); }}
				>
					+ Link nodes
				</button>
			{/if}
			<button
				class={`rounded-lg border px-2 py-1 text-[10px] transition ${
					showClusters
						? 'border-sky-500/40 bg-sky-500/10 text-sky-300'
						: 'border-slate-700 text-slate-400 hover:text-white'
				}`}
				on:click={toggleClusters}
				disabled={loadingClusters}
			>
				{loadingClusters ? 'Loading...' : showClusters ? 'Hide Clusters' : 'Show Clusters'}
			</button>
		</div>
	</div>

	{#if loading}
		<div class="flex h-96 items-center justify-center rounded-2xl border border-slate-800 bg-slate-900/40">
			<p class="text-xs text-slate-400">Loading graph...</p>
		</div>
	{:else if nodes.length === 0}
		<div class="flex h-96 flex-col items-center justify-center rounded-2xl border border-dashed border-slate-800 bg-slate-900/20">
			<div class="mb-4 flex h-14 w-14 items-center justify-center rounded-full bg-emerald-500/20 text-xl text-emerald-300">
				<svg class="h-6 w-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13.828 10.172a4 4 0 00-5.656 0l-4 4a4 4 0 105.656 5.656l1.102-1.101m-.758-4.899a4 4 0 005.656 0l4-4a4 4 0 00-5.656-5.656l-1.1 1.1" />
				</svg>
			</div>
			<h3 class="text-sm font-medium text-white">No connections yet</h3>
			<p class="mt-1 max-w-xs text-center text-xs text-slate-500">
				Create notes and tasks first, then link them together to see your knowledge graph.
			</p>
			<div class="mt-4 flex gap-2">
				<a href="/notes" class="rounded-lg border border-sky-500/40 bg-sky-500/10 px-3 py-2 text-xs text-sky-300 hover:bg-sky-500/20">
					Create notes
				</a>
				<a href="/tasks" class="rounded-lg border border-violet-500/40 bg-violet-500/10 px-3 py-2 text-xs text-violet-300 hover:bg-violet-500/20">
					Add tasks
				</a>
			</div>
		</div>
	{:else if displayedNodes.length === 0}
		<div class="flex h-96 flex-col items-center justify-center rounded-2xl border border-dashed border-slate-800 bg-slate-900/20">
			<p class="text-sm text-slate-400">No nodes match your current filter.</p>
			<button
				class="mt-2 text-xs text-sky-400 hover:text-sky-300"
				on:click={() => { kindFilter = 'all'; relationshipFilter = 'all'; }}
			>
				Clear filters
			</button>
		</div>
	{:else}
		<div class="overflow-hidden rounded-2xl border border-slate-800/60 bg-slate-950">
			<svg
				bind:this={svgEl}
				{width}
				{height}
				class="w-full"
				viewBox="0 0 {width} {height}"
			>
				<!-- Edges -->
				{#each displayedEdges as edge}
					{@const source = displayedNodes.find((n) => n.id === edge.source)}
					{@const target = displayedNodes.find((n) => n.id === edge.target)}
					{@const edgeColor = getRelationshipColor(edge.kind)}
					{#if source && target}
							<g class="edge-group">
								<line
									x1={source.x}
									y1={source.y}
								x2={target.x}
								y2={target.y}
								stroke={edgeColor}
								stroke-opacity="0.4"
								stroke-width="1.5"
							/>
								<!-- Arrow marker at midpoint -->
								<polygon
									points="-4,-3 4,0 -4,3"
									fill={edgeColor}
									fill-opacity="0.6"
									transform="translate({(source.x + target.x) / 2}, {(source.y + target.y) / 2}) rotate({Math.atan2(target.y - source.y, target.x - source.x) * (180 / Math.PI)})"
								/>
							</g>
					{/if}
				{/each}

				<!-- Nodes -->
				{#each displayedNodes as node (node.id)}
					<g
						on:mousedown={(e) => handleMouseDown(e, node.id)}
						on:dblclick={() => { window.location.href = nodeLink(node); }}
						style="cursor: {dragging === node.id ? 'grabbing' : 'grab'}"
						role="button"
						tabindex="0"
						on:keydown={(e) => { if (e.key === 'Enter') window.location.href = nodeLink(node); }}
					>
						<circle
							cx={node.x}
							cy={node.y}
							r={selectedNodeId === node.id ? 10 : creatingRelationship && relationshipFromId === node.id ? 10 : 7}
							fill={getNodeColor(node)}
							stroke={creatingRelationship && relationshipFromId === node.id ? '#22c55e' : showClusters && nodeClusterMap.has(node.id) ? getNodeColor(node) : 'none'}
							stroke-width="2"
							opacity={selectedNodeId && selectedNodeId !== node.id ? 0.3 : 0.8}
							role="button"
							tabindex="0"
							on:click={() => {
								if (creatingRelationship) {
									startRelationshipCreation(node.id);
								} else {
									selectedNodeId = selectedNodeId === node.id ? null : node.id;
								}
							}}
							on:keydown={(e) => {
								if (e.key === 'Enter' || e.key === ' ') {
									e.preventDefault();
									if (creatingRelationship) {
										startRelationshipCreation(node.id);
									} else {
										selectedNodeId = selectedNodeId === node.id ? null : node.id;
									}
								}
							}}
						/>
						{#if showLabels}
							<text
								x={node.x}
								y={node.y + 16}
								text-anchor="middle"
								fill="rgba(148, 163, 184, 0.6)"
								font-size="9"
							>
								{node.title.slice(0, 20)}
							</text>
						{/if}
					</g>
				{/each}
			</svg>
		</div>

		<!-- Node Kind Legend -->
		<div class="flex flex-wrap gap-3">
			{#each KINDS as kind}
				<div class="flex items-center gap-1.5">
					<div class="h-3 w-3 rounded-full" style="background-color: {kindColor(kind)}"></div>
					<span class="text-[10px] text-slate-400">{kindLabel(kind)}</span>
				</div>
			{/each}
		</div>

		<!-- Relationship Type Legend -->
		{#if uniqueRelationshipTypes.length > 0}
			<div class="mt-2 flex flex-wrap gap-3">
				<span class="text-[10px] text-slate-500">Relationships:</span>
				{#each uniqueRelationshipTypes as relType}
					<div class="flex items-center gap-1.5">
						<div class="h-0.5 w-4" style="background-color: {getRelationshipColor(relType)}"></div>
						<span class="text-[10px] text-slate-400">{relType}</span>
					</div>
				{/each}
			</div>
		{/if}

		<!-- Cluster Legend -->
		{#if showClusters && clusters.length > 0}
			<div class="mt-2 rounded-lg border border-slate-800 bg-slate-900/40 p-3">
				<div class="flex items-center gap-2">
					<span class="text-[10px] font-medium text-slate-400">Clusters:</span>
					<span class="rounded-full bg-slate-700 px-2 py-0.5 text-[10px] text-slate-300">
						{clusters.length} found
					</span>
				</div>
				<div class="mt-2 flex flex-wrap gap-2">
					{#each clusters as cluster, idx (cluster.id)}
						<div class="flex items-center gap-1.5 rounded-lg border border-slate-700 px-2 py-1">
							<div
								class="h-3 w-3 rounded-full"
								style="background-color: {CLUSTER_COLORS[idx % CLUSTER_COLORS.length]}"
							></div>
							<span class="text-[10px] text-slate-300">
								{cluster.name || `Cluster ${idx + 1}`}
							</span>
							<span class="rounded bg-slate-700 px-1 text-[9px] text-slate-400">
								{cluster.node_ids.length}
							</span>
						</div>
					{/each}
				</div>
			</div>
		{/if}

		{#if selectedNodeId}
			{@const selected = nodes.find((n) => n.id === selectedNodeId)}
			{#if selected}
				<div class="rounded-xl border border-slate-800 bg-slate-900/40 p-4">
					<div class="flex items-center justify-between">
						<div>
							<span class="rounded px-1.5 py-0.5 text-[9px] font-medium" style="background-color: {kindColor(selected.kind)}20; color: {kindColor(selected.kind)}">{kindLabel(selected.kind)}</span>
							<span class="ml-2 text-sm font-medium text-white">{selected.title}</span>
						</div>
						<div class="flex items-center gap-2">
							<button
								class="rounded-lg border border-emerald-500/40 px-3 py-1 text-xs text-emerald-300 hover:bg-emerald-500/10"
								on:click={() => startRelationshipCreation(selected.id)}
							>
								Link from here
							</button>
							<a
								href={nodeLink(selected)}
								class="rounded-lg border border-slate-700 px-3 py-1 text-xs text-slate-300 hover:bg-slate-800"
							>
								Open
							</a>
						</div>
					</div>

					<!-- Relationships list -->
					{#if loadingRelationships}
						<div class="mt-3 text-[10px] text-slate-500">Loading relationships...</div>
					{:else if selectedNodeRelationships.length > 0}
						<div class="mt-3 border-t border-slate-800 pt-3">
							<h4 class="mb-2 text-[10px] font-semibold uppercase tracking-wide text-slate-500">
								Relationships ({selectedNodeRelationships.length})
							</h4>
							<div class="flex flex-col gap-1">
								{#each selectedNodeRelationships as rel (rel.id)}
									{@const otherNodeId = rel.from_node_id === selected.id ? rel.to_node_id : rel.from_node_id}
									{@const otherNode = nodes.find((n) => n.id === otherNodeId)}
									{@const isOutgoing = rel.from_node_id === selected.id}
									<div class="group flex items-center gap-2 rounded-lg px-2 py-1.5 text-xs hover:bg-slate-800/60">
										<span class="text-[9px] text-slate-600">{isOutgoing ? '\u2192' : '\u2190'}</span>
										<span class="text-[9px] font-medium" style="color: {getRelationshipColor(rel.kind)}">{rel.kind}</span>
										<span class="flex-1 truncate text-slate-300">{otherNode?.title ?? otherNodeId.slice(0, 8)}</span>
										<button
											class="flex-shrink-0 rounded p-1 text-slate-600 opacity-0 transition hover:bg-red-500/10 hover:text-red-400 group-hover:opacity-100"
											title="Delete relationship"
											disabled={deletingRelId === rel.id}
											on:click={() => handleDeleteRelationship(rel)}
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
				</div>
			{/if}
		{/if}
	{/if}
</div>

<!-- Relationship Type Modal -->
{#if showRelationshipModal}
	{@const fromNode = nodes.find((n) => n.id === relationshipFromId)}
	{@const toNode = nodes.find((n) => n.id === relationshipToId)}
	<div class="fixed inset-0 z-50 flex items-center justify-center" role="presentation">
		<div
			class="absolute inset-0 bg-black/50"
			on:click={cancelRelationshipCreation}
			on:keydown={(e) => e.key === 'Escape' && cancelRelationshipCreation()}
			role="button"
			tabindex="-1"
			aria-label="Close"
		></div>
		<div class="relative z-10 w-full max-w-md rounded-2xl border border-slate-700 bg-slate-900 p-5 shadow-2xl">
			<h3 class="text-sm font-semibold text-white">Create Relationship</h3>
			<p class="mt-1 text-[11px] text-slate-400">
				Connect "{fromNode?.title ?? 'Node'}" to "{toNode?.title ?? 'Node'}"
			</p>

			<div class="mt-4">
				<label class="text-[10px] uppercase tracking-wide text-slate-500" for="rel-type">
					Relationship Type
				</label>
				<select
					id="rel-type"
					class="mt-1 w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-sm text-white"
					bind:value={selectedRelationshipType}
				>
					{#each RELATIONSHIP_TYPES as relType}
						<option value={relType}>{relType}</option>
					{/each}
				</select>
			</div>

			<div class="mt-3 rounded-lg border border-slate-800 bg-slate-800/50 p-3">
				<div class="flex items-center gap-2 text-xs text-slate-400">
					<span class="rounded bg-slate-700 px-1.5 py-0.5">{fromNode?.title?.slice(0, 20) ?? '?'}</span>
					<span style="color: {getRelationshipColor(selectedRelationshipType)}">→ {selectedRelationshipType} →</span>
					<span class="rounded bg-slate-700 px-1.5 py-0.5">{toNode?.title?.slice(0, 20) ?? '?'}</span>
				</div>
			</div>

			<div class="mt-4 flex justify-end gap-2">
				<button
					class="rounded-lg border border-slate-700 px-3 py-1.5 text-xs text-slate-300 hover:bg-slate-800"
					on:click={cancelRelationshipCreation}
				>
					Cancel
				</button>
				<button
					class="rounded-lg bg-emerald-500 px-3 py-1.5 text-xs font-semibold text-white hover:bg-emerald-400"
					on:click={confirmCreateRelationship}
				>
					Create
				</button>
			</div>
		</div>
	</div>
{/if}
