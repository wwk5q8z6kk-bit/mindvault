<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { page } from '$app/stores';
	import { listNodes } from '$lib/api/nodes';
	import {
		getNeighbors,
		addRelationship,
		getNodeRelationships,
		deleteRelationship,
		updateRelationship,
		getGraphClusters,
		searchGraph
	} from '$lib/api/graph';
	import type { NodeRelationship, GraphCluster, GraphSearchResult } from '$lib/api/graph';
	import {
		getConceptMap,
		getKnowledgeGaps,
		getCrossNamespaceInsights,
		type ConceptMapResponse
	} from '$lib/api/insights';
	import type { KnowledgeNode, ProactiveInsight } from '$lib/api/types';
	import { pushToast } from '$lib/stores/toast';
	import {
		kindColor,
		kindLabel,
		ALL_NODE_KINDS,
		RELATIONSHIP_TYPES,
		type RelationshipType
	} from '$lib/utils/kind-helpers';

	type GraphNode = {
		id: string;
		title: string;
		kind: string;
		namespace: string | null;
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
	let appliedRequestedNodeId: string | null = null;
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
	let editingRelId: string | null = null;
	let editingWeight: number = 1;

	// Cluster visualization
	let showClusters = false;
	let clusters: GraphCluster[] = [];
	let loadingClusters = false;
	let nodeClusterMap: Map<string, number> = new Map();

	// Cluster detail state
	let selectedClusterId: string | null = null;
	$: selectedCluster = clusters.find((c) => c.id === selectedClusterId) ?? null;
	$: selectedClusterNodes = selectedCluster
		? nodes.filter((n) => selectedCluster!.node_ids.includes(n.id))
		: [];

	// Graph search state
	let showGraphSearch = false;
	let graphSearchResults: GraphSearchResult[] = [];
	let graphSearching = false;
	let searchMaxDepth = 3;
	let searchMinScore = 0.1;

	// Graph depth control
	let graphDepth = 1;

	// Intelligence features
	let showIntelligence = false;
	let conceptMap: ConceptMapResponse | null = null;
	let conceptMapLoading = false;
	let knowledgeGaps: ProactiveInsight[] = [];
	let gapsLoading = false;
	let crossNamespaceInsights: ProactiveInsight[] = [];
	let crossNamespaceLoading = false;
	let crossNamespaceInput = '';

	// Local multi-hop pathfinding (BFS over neighbors endpoint)
	let pathFromId = '';
	let pathToId = '';
	let pathLoading = false;
	let pathResult: string[] = [];
	let pathError = '';

	const CLUSTER_COLORS = [
		'#38bdf8',
		'#22c55e',
		'#f97316',
		'#a78bfa',
		'#ec4899',
		'#eab308',
		'#14b8a6',
		'#ef4444',
		'#8b5cf6',
		'#06b6d4'
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
	$: filteredByRelationship =
		relationshipFilter === 'all' ? edges : edges.filter((e) => e.kind === relationshipFilter);
	$: displayedEdges = filteredByRelationship.filter(
		(e) => displayedNodeIds.has(e.source) && displayedNodeIds.has(e.target)
	);
	$: uniqueRelationshipTypes = [...new Set(edges.map((e) => e.kind))].sort();
	$: availableNamespaces = [...new Set(nodes.map((node) => node.namespace).filter(Boolean))];
	$: if (!crossNamespaceInput && availableNamespaces.length >= 2) {
		crossNamespaceInput = availableNamespaces.slice(0, 2).join(', ');
	}
	$: if (!pathFromId && nodes.length > 0) {
		pathFromId = nodes[0].id;
	}
	$: if (!pathToId && nodes.length > 1) {
		pathToId = nodes[1].id;
	}
	$: requestedNodeId = $page.url.searchParams.get('node');
	$: if (
		requestedNodeId &&
		requestedNodeId !== appliedRequestedNodeId &&
		nodes.some((node) => node.id === requestedNodeId) &&
		selectedNodeId !== requestedNodeId
	) {
		selectedNodeId = requestedNodeId;
		appliedRequestedNodeId = requestedNodeId;
	}

	async function loadGraph(depth: number = 1) {
		loading = true;
		edges = [];
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
					namespace: node.namespace ?? null,
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
					const result = await getNeighbors(node.id, depth);
					for (const neighbor of result.neighbors) {
						const key = [node.id, neighbor.node.id].sort().join('-');
						if (!edgeSet.has(key)) {
							edgeSet.add(key);
							edges = [
								...edges,
								{
									source: neighbor.direction === 'outgoing' ? node.id : neighbor.node.id,
									target: neighbor.direction === 'outgoing' ? neighbor.node.id : node.id,
									kind: neighbor.relationship_kind
								}
							];
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
	}

	async function reloadWithDepth() {
		if (animFrame) cancelAnimationFrame(animFrame);
		await loadGraph(graphDepth);
	}

	onMount(async () => {
		await loadGraph(graphDepth);
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

	function startEditRelationship(rel: NodeRelationship) {
		editingRelId = rel.id;
		editingWeight = rel.weight ?? 1;
	}

	async function saveRelationshipWeight(relId: string) {
		try {
			const updated = await updateRelationship(relId, { weight: editingWeight });
			selectedNodeRelationships = selectedNodeRelationships.map((r) =>
				r.id === relId ? updated : r
			);
			pushToast('Relationship weight updated', 'success');
		} catch {
			pushToast('Failed to update relationship', 'danger');
		} finally {
			editingRelId = null;
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
			edges = [
				...edges,
				{
					source: relationshipFromId,
					target: relationshipToId,
					kind: selectedRelationshipType
				}
			];

			pushToast(`Created ${selectedRelationshipType} relationship`, 'success');
		} catch {
			pushToast('Failed to create relationship', 'danger');
		} finally {
			cancelRelationshipCreation();
		}
	}

	async function runGraphSearch() {
		if (!selectedNodeId || graphSearching) return;
		graphSearching = true;
		showGraphSearch = true;
		try {
			const result = await searchGraph({
				start_id: selectedNodeId,
				max_depth: searchMaxDepth,
				min_score: searchMinScore
			});
			graphSearchResults = result.results;
			if (result.results.length === 0) {
				pushToast('No reachable nodes found from this starting point', 'info');
			}
		} catch {
			pushToast('Graph search failed', 'danger');
		} finally {
			graphSearching = false;
		}
	}

	async function loadConceptMapData() {
		conceptMapLoading = true;
		try {
			conceptMap = await getConceptMap(undefined, 8);
		} catch {
			pushToast('Failed to load concept map', 'warning');
			conceptMap = null;
		} finally {
			conceptMapLoading = false;
		}
	}

	async function loadKnowledgeGapsData() {
		gapsLoading = true;
		try {
			knowledgeGaps = await getKnowledgeGaps();
		} catch {
			pushToast('Failed to load knowledge gap analysis', 'warning');
			knowledgeGaps = [];
		} finally {
			gapsLoading = false;
		}
	}

	async function loadCrossNamespaceData() {
		const namespaces = crossNamespaceInput
			.split(',')
			.map((item) => item.trim())
			.filter(Boolean);
		if (namespaces.length < 2) {
			pushToast('Enter at least two comma-separated namespaces', 'info');
			return;
		}
		crossNamespaceLoading = true;
		try {
			crossNamespaceInsights = await getCrossNamespaceInsights(namespaces, 2);
		} catch {
			pushToast('Failed to load cross-namespace insights', 'warning');
			crossNamespaceInsights = [];
		} finally {
			crossNamespaceLoading = false;
		}
	}

	async function toggleIntelligence() {
		showIntelligence = !showIntelligence;
		if (!showIntelligence) return;
		if (!conceptMap) {
			await loadConceptMapData();
		}
		if (knowledgeGaps.length === 0) {
			await loadKnowledgeGapsData();
		}
	}

	async function findMultiHopPath() {
		if (!pathFromId || !pathToId) return;
		if (pathFromId === pathToId) {
			pathResult = [pathFromId];
			pathError = '';
			return;
		}
		pathLoading = true;
		pathError = '';
		pathResult = [];
		try {
			const maxDepth = 5;
			const queue: string[][] = [[pathFromId]];
			const visited = new Set<string>([pathFromId]);

			while (queue.length > 0) {
				const currentPath = queue.shift()!;
				const current = currentPath[currentPath.length - 1];
				if (currentPath.length - 1 >= maxDepth) continue;

				const neighbors = await getNeighbors(current, 1);
				for (const neighbor of neighbors.neighbors) {
					const nextId = neighbor.node.id;
					if (visited.has(nextId)) continue;
					visited.add(nextId);
					const nextPath = [...currentPath, nextId];
					if (nextId === pathToId) {
						pathResult = nextPath;
						pathError = '';
						return;
					}
					queue.push(nextPath);
				}
			}

			pathError = 'No path found within 5 hops.';
		} catch {
			pathError = 'Pathfinding failed.';
		} finally {
			pathLoading = false;
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
			<div class="flex items-center gap-1.5">
				<label class="text-[10px] text-slate-400" for="graph-depth">Depth</label>
				<select
					id="graph-depth"
					class="rounded-lg border border-slate-700 bg-slate-800 px-2 py-1 text-xs text-white"
					bind:value={graphDepth}
					on:change={reloadWithDepth}
				>
					<option value={1}>1 hop</option>
					<option value={2}>2 hops</option>
					<option value={3}>3 hops</option>
				</select>
			</div>
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
					on:click={() => {
						creatingRelationship = true;
						pushToast('Click a node to start', 'info');
					}}
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
			<button
				class={`rounded-lg border px-2 py-1 text-[10px] transition ${
					showIntelligence
						? 'border-violet-500/40 bg-violet-500/10 text-violet-300'
						: 'border-slate-700 text-slate-400 hover:text-white'
				}`}
				on:click={toggleIntelligence}
			>
				{showIntelligence ? 'Hide Intelligence' : 'Intelligence'}
			</button>
		</div>
	</div>

	{#if loading}
		<div
			class="flex h-96 items-center justify-center rounded-2xl border border-slate-800 bg-slate-900/40"
		>
			<p class="text-xs text-slate-400">Loading graph...</p>
		</div>
	{:else if nodes.length === 0}
		<div
			class="flex h-96 flex-col items-center justify-center rounded-2xl border border-dashed border-slate-800 bg-slate-900/20"
		>
			<div
				class="mb-4 flex h-14 w-14 items-center justify-center rounded-full bg-emerald-500/20 text-xl text-emerald-300"
			>
				<svg class="h-6 w-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
					<path
						stroke-linecap="round"
						stroke-linejoin="round"
						stroke-width="2"
						d="M13.828 10.172a4 4 0 00-5.656 0l-4 4a4 4 0 105.656 5.656l1.102-1.101m-.758-4.899a4 4 0 005.656 0l4-4a4 4 0 00-5.656-5.656l-1.1 1.1"
					/>
				</svg>
			</div>
			<h3 class="text-sm font-medium text-white">No connections yet</h3>
			<p class="mt-1 max-w-xs text-center text-xs text-slate-500">
				Create notes and tasks first, then link them together to see your knowledge graph.
			</p>
			<div class="mt-4 flex gap-2">
				<a
					href="/notes"
					class="rounded-lg border border-sky-500/40 bg-sky-500/10 px-3 py-2 text-xs text-sky-300 hover:bg-sky-500/20"
				>
					Create notes
				</a>
				<a
					href="/tasks"
					class="rounded-lg border border-violet-500/40 bg-violet-500/10 px-3 py-2 text-xs text-violet-300 hover:bg-violet-500/20"
				>
					Add tasks
				</a>
			</div>
		</div>
	{:else if displayedNodes.length === 0}
		<div
			class="flex h-96 flex-col items-center justify-center rounded-2xl border border-dashed border-slate-800 bg-slate-900/20"
		>
			<p class="text-sm text-slate-400">No nodes match your current filter.</p>
			<button
				class="mt-2 text-xs text-sky-400 hover:text-sky-300"
				on:click={() => {
					kindFilter = 'all';
					relationshipFilter = 'all';
				}}
			>
				Clear filters
			</button>
		</div>
	{:else}
		<div class="overflow-hidden rounded-2xl border border-slate-800/60 bg-slate-950">
			<svg bind:this={svgEl} {width} {height} class="w-full" viewBox="0 0 {width} {height}">
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
								transform="translate({(source.x + target.x) / 2}, {(source.y + target.y) /
									2}) rotate({Math.atan2(target.y - source.y, target.x - source.x) *
									(180 / Math.PI)})"
							/>
						</g>
					{/if}
				{/each}

				<!-- Nodes -->
				{#each displayedNodes as node (node.id)}
					<g
						on:mousedown={(e) => handleMouseDown(e, node.id)}
						on:dblclick={() => {
							window.location.href = nodeLink(node);
						}}
						style="cursor: {dragging === node.id ? 'grabbing' : 'grab'}"
						role="button"
						tabindex="0"
						on:keydown={(e) => {
							if (e.key === 'Enter') window.location.href = nodeLink(node);
						}}
					>
						<circle
							cx={node.x}
							cy={node.y}
							r={selectedNodeId === node.id
								? 10
								: creatingRelationship && relationshipFromId === node.id
									? 10
									: 7}
							fill={getNodeColor(node)}
							stroke={creatingRelationship && relationshipFromId === node.id
								? '#22c55e'
								: showClusters && nodeClusterMap.has(node.id)
									? getNodeColor(node)
									: 'none'}
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

		<!-- Cluster Legend + Details -->
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
						<button
							class="flex items-center gap-1.5 rounded-lg border px-2 py-1 transition {selectedClusterId ===
							cluster.id
								? 'border-sky-500/40 bg-sky-500/10'
								: 'border-slate-700 hover:border-slate-600'}"
							on:click={() => {
								selectedClusterId = selectedClusterId === cluster.id ? null : cluster.id;
							}}
						>
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
						</button>
					{/each}
				</div>

				<!-- Cluster detail panel -->
				{#if selectedCluster}
					{@const clusterIdx = clusters.indexOf(selectedCluster)}
					<div class="mt-3 border-t border-slate-800 pt-3">
						<div class="flex items-center gap-2">
							<div
								class="h-4 w-4 rounded-full"
								style="background-color: {CLUSTER_COLORS[clusterIdx % CLUSTER_COLORS.length]}"
							></div>
							<h4 class="text-xs font-semibold text-white">
								{selectedCluster.name || `Cluster ${clusterIdx + 1}`}
							</h4>
							<span class="ml-auto text-[10px] text-slate-500">
								Density: {selectedCluster.density.toFixed(2)}
							</span>
						</div>
						<div class="mt-2 flex flex-col gap-1 max-h-40 overflow-y-auto">
							{#each selectedClusterNodes as cNode (cNode.id)}
								<a
									href={nodeLink(cNode)}
									class="flex items-center gap-2 rounded-lg px-2 py-1.5 text-xs hover:bg-slate-800/60 transition {cNode.id ===
									selectedCluster.center_node_id
										? 'border border-sky-500/30 bg-sky-500/5'
										: ''}"
								>
									<div
										class="h-2.5 w-2.5 rounded-full flex-shrink-0"
										style="background-color: {kindColor(cNode.kind)}"
									></div>
									<span class="truncate text-slate-300">{cNode.title}</span>
									{#if cNode.id === selectedCluster.center_node_id}
										<span class="ml-auto text-[9px] text-sky-400">center</span>
									{/if}
								</a>
							{/each}
						</div>
					</div>
				{/if}
			</div>
		{/if}

		{#if showIntelligence}
			<div class="mt-3 rounded-xl border border-violet-500/20 bg-violet-500/5 p-4 space-y-3">
				<div class="flex items-center justify-between gap-2">
					<h3 class="text-sm font-semibold text-violet-200">Graph Intelligence</h3>
					<div class="flex gap-1.5">
						<button
							class="rounded border border-slate-700 px-2 py-0.5 text-[10px] text-slate-300 hover:bg-slate-800"
							on:click={loadConceptMapData}
							disabled={conceptMapLoading}
						>
							{conceptMapLoading ? 'Loading...' : 'Concept Map'}
						</button>
						<button
							class="rounded border border-slate-700 px-2 py-0.5 text-[10px] text-slate-300 hover:bg-slate-800"
							on:click={loadKnowledgeGapsData}
							disabled={gapsLoading}
						>
							{gapsLoading ? 'Loading...' : 'Gap Analysis'}
						</button>
					</div>
				</div>

				<div class="grid gap-3 lg:grid-cols-2">
					<div class="rounded-lg border border-slate-800 bg-slate-900/40 p-3">
						<div class="mb-2 flex items-center justify-between">
							<h4 class="text-[11px] font-semibold uppercase tracking-wide text-slate-400">
								Concept Map
							</h4>
							{#if conceptMap}
								<span class="text-[10px] text-slate-500">{conceptMap.total_nodes} nodes</span>
							{/if}
						</div>
						{#if !conceptMap}
							<p class="text-xs text-slate-500">Generate to view dominant concept clusters.</p>
						{:else}
							<div class="space-y-1.5 max-h-40 overflow-y-auto">
								{#each conceptMap.clusters as cluster (cluster.topic)}
									<div class="rounded border border-slate-700 px-2 py-1 text-xs text-slate-300">
										<span class="font-medium text-violet-300">{cluster.topic}</span>
										<span class="ml-2 text-slate-500">{cluster.count} nodes</span>
									</div>
								{/each}
							</div>
						{/if}
					</div>

					<div class="rounded-lg border border-slate-800 bg-slate-900/40 p-3">
						<h4 class="mb-2 text-[11px] font-semibold uppercase tracking-wide text-slate-400">
							Knowledge Gaps
						</h4>
						{#if knowledgeGaps.length === 0}
							<p class="text-xs text-slate-500">No major unanswered questions detected.</p>
						{:else}
							<div class="space-y-1.5 max-h-40 overflow-y-auto">
								{#each knowledgeGaps.slice(0, 8) as gap (gap.id)}
									<div class="rounded border border-amber-500/20 bg-amber-500/5 px-2 py-1 text-xs">
										<p class="font-medium text-amber-200">{gap.title}</p>
										<p class="mt-0.5 text-slate-300">{gap.content}</p>
									</div>
								{/each}
							</div>
						{/if}
					</div>
				</div>

				<div class="rounded-lg border border-slate-800 bg-slate-900/40 p-3">
					<div class="flex flex-wrap items-end gap-2">
						<div class="flex-1 min-w-[220px]">
							<label
								for="cross-ns-input"
								class="mb-1 block text-[10px] uppercase tracking-wide text-slate-500"
							>
								Cross-Namespace Insights
							</label>
							<input
								id="cross-ns-input"
								bind:value={crossNamespaceInput}
								class="w-full rounded border border-slate-700 bg-slate-800 px-2 py-1.5 text-xs text-white"
								placeholder="namespace-a, namespace-b"
							/>
						</div>
						<button
							class="rounded border border-violet-500/40 px-2.5 py-1.5 text-[10px] text-violet-300 hover:bg-violet-500/10 disabled:opacity-60"
							on:click={loadCrossNamespaceData}
							disabled={crossNamespaceLoading}
						>
							{crossNamespaceLoading ? 'Analyzing...' : 'Analyze'}
						</button>
					</div>
					{#if crossNamespaceInsights.length > 0}
						<div class="mt-2 space-y-1.5 max-h-36 overflow-y-auto">
							{#each crossNamespaceInsights as insight (insight.id)}
								<div
									class="rounded border border-violet-500/20 bg-violet-500/5 px-2 py-1 text-xs text-slate-200"
								>
									<p class="font-medium text-violet-200">{insight.title}</p>
									<p class="mt-0.5 text-slate-300">{insight.content}</p>
								</div>
							{/each}
						</div>
					{/if}
				</div>

				<div class="rounded-lg border border-slate-800 bg-slate-900/40 p-3">
					<h4 class="mb-2 text-[11px] font-semibold uppercase tracking-wide text-slate-400">
						Multi-Hop Pathfinding
					</h4>
					<div class="grid gap-2 md:grid-cols-[1fr_1fr_auto]">
						<select
							class="rounded border border-slate-700 bg-slate-800 px-2 py-1.5 text-xs text-white"
							bind:value={pathFromId}
						>
							{#each nodes as node (node.id)}
								<option value={node.id}>{node.title.slice(0, 48)}</option>
							{/each}
						</select>
						<select
							class="rounded border border-slate-700 bg-slate-800 px-2 py-1.5 text-xs text-white"
							bind:value={pathToId}
						>
							{#each nodes as node (node.id)}
								<option value={node.id}>{node.title.slice(0, 48)}</option>
							{/each}
						</select>
						<button
							class="rounded border border-sky-500/40 px-2.5 py-1.5 text-[10px] text-sky-300 hover:bg-sky-500/10 disabled:opacity-60"
							on:click={findMultiHopPath}
							disabled={pathLoading || !pathFromId || !pathToId}
						>
							{pathLoading ? 'Searching...' : 'Find Path'}
						</button>
					</div>
					{#if pathResult.length > 0}
						<div class="mt-2 flex flex-wrap items-center gap-1 text-xs text-slate-300">
							{#each pathResult as nodeId, idx (nodeId)}
								{@const node = nodes.find((n) => n.id === nodeId)}
								<span class="rounded border border-slate-700 px-2 py-0.5"
									>{node?.title || nodeId.slice(0, 8)}</span
								>
								{#if idx < pathResult.length - 1}
									<span class="text-slate-500">→</span>
								{/if}
							{/each}
						</div>
					{:else if pathError}
						<p class="mt-2 text-xs text-red-300">{pathError}</p>
					{/if}
				</div>
			</div>
		{/if}

		{#if selectedNodeId}
			{@const selected = nodes.find((n) => n.id === selectedNodeId)}
			{#if selected}
				<div class="rounded-xl border border-slate-800 bg-slate-900/40 p-4">
					<div class="flex items-center justify-between">
						<div>
							<span
								class="rounded px-1.5 py-0.5 text-[9px] font-medium"
								style="background-color: {kindColor(selected.kind)}20; color: {kindColor(
									selected.kind
								)}">{kindLabel(selected.kind)}</span
							>
							<span class="ml-2 text-sm font-medium text-white">{selected.title}</span>
						</div>
						<div class="flex items-center gap-2">
							<button
								class="rounded-lg border border-emerald-500/40 px-3 py-1 text-xs text-emerald-300 hover:bg-emerald-500/10"
								on:click={() => startRelationshipCreation(selected.id)}
							>
								Link from here
							</button>
							<button
								class="rounded-lg border border-violet-500/40 px-3 py-1 text-xs text-violet-300 hover:bg-violet-500/10 disabled:opacity-50"
								on:click={runGraphSearch}
								disabled={graphSearching}
							>
								{graphSearching ? 'Searching...' : 'Explore from here'}
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
									{@const otherNodeId =
										rel.from_node_id === selected.id ? rel.to_node_id : rel.from_node_id}
									{@const otherNode = nodes.find((n) => n.id === otherNodeId)}
									{@const isOutgoing = rel.from_node_id === selected.id}
									<div class="group rounded-lg px-2 py-1.5 text-xs hover:bg-slate-800/60">
										<div class="flex items-center gap-2">
											<span class="text-[9px] text-slate-600"
												>{isOutgoing ? '\u2192' : '\u2190'}</span
											>
											<span
												class="text-[9px] font-medium"
												style="color: {getRelationshipColor(rel.kind)}">{rel.kind}</span
											>
											<span class="flex-1 truncate text-slate-300"
												>{otherNode?.title ?? otherNodeId.slice(0, 8)}</span
											>
											{#if rel.weight != null}
												<span class="text-[9px] text-slate-600" title="Weight">{rel.weight}</span>
											{/if}
											<button
												class="flex-shrink-0 rounded p-1 text-slate-600 opacity-0 transition hover:bg-sky-500/10 hover:text-sky-400 group-hover:opacity-100"
												title="Edit weight"
												on:click={() => startEditRelationship(rel)}
											>
												<svg
													viewBox="0 0 24 24"
													width="14"
													height="14"
													fill="none"
													stroke="currentColor"
													stroke-width="2"
												>
													<path d="M11 4H4a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2v-7" /><path
														d="M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4 9.5-9.5z"
													/>
												</svg>
											</button>
											<button
												class="flex-shrink-0 rounded p-1 text-slate-600 opacity-0 transition hover:bg-red-500/10 hover:text-red-400 group-hover:opacity-100"
												title="Delete relationship"
												disabled={deletingRelId === rel.id}
												on:click={() => handleDeleteRelationship(rel)}
											>
												<svg
													viewBox="0 0 24 24"
													width="14"
													height="14"
													fill="none"
													stroke="currentColor"
													stroke-width="2"
												>
													<path d="M18 6L6 18M6 6l12 12" />
												</svg>
											</button>
										</div>
										{#if editingRelId === rel.id}
											<div class="mt-1.5 flex items-center gap-2">
												<label class="flex items-center gap-1 text-[9px] text-slate-500">
													Weight
													<input
														type="number"
														min="0"
														max="10"
														step="0.1"
														class="w-16 rounded border border-slate-700 bg-slate-800 px-1.5 py-0.5 text-[10px] text-white"
														bind:value={editingWeight}
														on:keydown={(e) => e.key === 'Enter' && saveRelationshipWeight(rel.id)}
													/>
												</label>
												<button
													class="rounded bg-sky-600 px-2 py-0.5 text-[9px] text-white hover:bg-sky-500"
													on:click={() => saveRelationshipWeight(rel.id)}
												>
													Save
												</button>
												<button
													class="text-[9px] text-slate-500 hover:text-white"
													on:click={() => {
														editingRelId = null;
													}}
												>
													Cancel
												</button>
											</div>
										{/if}
									</div>
								{/each}
							</div>
						</div>
					{/if}

					<!-- Graph Search Results -->
					{#if showGraphSearch}
						<div class="mt-3 border-t border-slate-800 pt-3">
							<div class="flex items-center justify-between">
								<h4 class="text-[10px] font-semibold uppercase tracking-wide text-slate-500">
									Graph Exploration
								</h4>
								<button
									class="text-[10px] text-slate-500 hover:text-slate-300"
									on:click={() => {
										showGraphSearch = false;
										graphSearchResults = [];
									}}
								>
									Close
								</button>
							</div>
							<div class="mt-2 flex items-center gap-2">
								<label class="text-[10px] text-slate-500">
									Depth
									<input
										type="number"
										min="1"
										max="5"
										class="ml-1 w-12 rounded border border-slate-700 bg-slate-800 px-1.5 py-0.5 text-[10px] text-white"
										bind:value={searchMaxDepth}
									/>
								</label>
								<label class="text-[10px] text-slate-500">
									Min score
									<input
										type="number"
										min="0"
										max="1"
										step="0.1"
										class="ml-1 w-14 rounded border border-slate-700 bg-slate-800 px-1.5 py-0.5 text-[10px] text-white"
										bind:value={searchMinScore}
									/>
								</label>
								<button
									class="rounded border border-violet-500/40 px-2 py-0.5 text-[10px] text-violet-300 hover:bg-violet-500/10 disabled:opacity-50"
									on:click={runGraphSearch}
									disabled={graphSearching}
								>
									{graphSearching ? '...' : 'Search'}
								</button>
							</div>
							{#if graphSearchResults.length > 0}
								<div class="mt-2 flex flex-col gap-1 max-h-48 overflow-y-auto">
									{#each graphSearchResults as result (result.node.id)}
										<a
											href={nodeLink({
												id: result.node.id,
												title: result.node.title || 'Untitled',
												kind: result.node.kind,
												namespace: result.node.namespace ?? null,
												x: 0,
												y: 0,
												vx: 0,
												vy: 0
											})}
											class="flex items-center gap-2 rounded-lg px-2 py-1.5 text-xs hover:bg-slate-800/60 transition"
										>
											<div
												class="h-2.5 w-2.5 rounded-full flex-shrink-0"
												style="background-color: {kindColor(result.node.kind)}"
											></div>
											<span class="flex-1 truncate text-slate-300"
												>{result.node.title || 'Untitled'}</span
											>
											<span class="text-[9px] text-slate-500"
												>{result.path_length} hop{result.path_length !== 1 ? 's' : ''}</span
											>
											<span class="text-[9px] text-violet-400">{result.score.toFixed(2)}</span>
										</a>
									{/each}
								</div>
							{:else if !graphSearching}
								<p class="mt-2 text-[10px] text-slate-500">
									No results yet. Click Search to explore.
								</p>
							{/if}
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
		<div
			class="relative z-10 w-full max-w-md rounded-2xl border border-slate-700 bg-slate-900 p-5 shadow-2xl"
		>
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
					<span class="rounded bg-slate-700 px-1.5 py-0.5"
						>{fromNode?.title?.slice(0, 20) ?? '?'}</span
					>
					<span style="color: {getRelationshipColor(selectedRelationshipType)}"
						>→ {selectedRelationshipType} →</span
					>
					<span class="rounded bg-slate-700 px-1.5 py-0.5"
						>{toNode?.title?.slice(0, 20) ?? '?'}</span
					>
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
