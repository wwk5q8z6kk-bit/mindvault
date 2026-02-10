<script lang="ts">
	import { onMount } from 'svelte';
	import { fetchInsights, generateInsights, insights } from '$lib/api/agent';
	import { dismissInsight, fullInsightScan, getEmbeddingClusters } from '$lib/api/insights';
	import { listConflicts, resolveConflict, type ConflictAlert } from '$lib/api/conflicts';
	import { pushToast } from '$lib/stores/toast';
	import type { ProactiveInsight } from '$lib/api/types';

	let loading = true;
	let generating = false;
	let scanning = false;
	let loadingClusters = false;
	let conflicts: ConflictAlert[] = [];
	let clusters: ProactiveInsight[] = [];
	let showClusters = false;
	let filterType: string = 'all';

	onMount(() => {
		void (async () => {
			try {
				await Promise.all([fetchInsights(), loadConflicts()]);
			} catch {
				pushToast('Insights are temporarily unavailable', 'warning');
			} finally {
				loading = false;
			}
		})();
	});

	async function loadConflicts() {
		try {
			conflicts = await listConflicts(false, 50);
		} catch {
			// non-critical
		}
	}

	async function handleGenerate() {
		generating = true;
		try {
			const newInsights = await generateInsights();
			pushToast(`Generated ${newInsights.length} insights`, 'success');
		} catch {
			pushToast('Failed to generate insights', 'danger');
		} finally {
			generating = false;
		}
	}

	async function handleFullScan() {
		scanning = true;
		try {
			const newInsights = await fullInsightScan();
			pushToast(`Full scan found ${newInsights.length} insights`, 'success');
		} catch {
			pushToast('Full scan failed', 'danger');
		} finally {
			scanning = false;
		}
	}

	async function handleLoadClusters() {
		loadingClusters = true;
		showClusters = true;
		try {
			clusters = await getEmbeddingClusters();
		} catch {
			pushToast('Failed to load clusters', 'danger');
		} finally {
			loadingClusters = false;
		}
	}

	async function handleDismiss(id: string) {
		try {
			await dismissInsight(id);
			pushToast('Insight dismissed', 'success');
		} catch {
			pushToast('Failed to dismiss insight', 'danger');
		}
	}

	async function handleResolveConflict(id: string) {
		try {
			await resolveConflict(id);
			conflicts = conflicts.filter((c) => c.id !== id);
			pushToast('Conflict resolved', 'success');
		} catch {
			pushToast('Failed to resolve conflict', 'danger');
		}
	}

	const typeColors: Record<string, string> = {
		connection: 'bg-blue-500/20 text-blue-300',
		trend: 'bg-green-500/20 text-green-300',
		gap: 'bg-yellow-500/20 text-yellow-300',
		stale: 'bg-orange-500/20 text-orange-300',
		cluster: 'bg-purple-500/20 text-purple-300',
		cross_domain: 'bg-cyan-500/20 text-cyan-300',
		unlinked_cluster: 'bg-indigo-500/20 text-indigo-300',
		ambient_link: 'bg-teal-500/20 text-teal-300',
		temporal_pattern: 'bg-pink-500/20 text-pink-300',
		knowledge_gap: 'bg-amber-500/20 text-amber-300',
		conflict: 'bg-red-500/20 text-red-300',
		general: 'bg-slate-500/20 text-slate-300'
	};

	function typeColor(type: string): string {
		return typeColors[type] || typeColors.general;
	}

	$: filteredInsights =
		filterType === 'all'
			? $insights
			: $insights.filter((i) => i.insight_type === filterType);

	$: insightTypes = [...new Set($insights.map((i) => i.insight_type))];
</script>

<svelte:head>
	<title>Insights - MindVault</title>
</svelte:head>

<div class="mx-auto max-w-4xl space-y-6 p-6">
	<div class="flex items-center justify-between">
		<div>
			<h1 class="text-lg font-semibold text-white">Insights</h1>
			<p class="text-xs text-slate-400">
				AI-discovered patterns, connections, and knowledge gaps
			</p>
		</div>
		<div class="flex gap-2">
			<button
				class="rounded-lg border border-slate-700 px-3 py-1.5 text-xs font-medium text-slate-300 transition hover:bg-slate-800 disabled:opacity-50"
				disabled={loadingClusters}
				on:click={handleLoadClusters}
			>
				{loadingClusters ? 'Loading...' : 'Clusters'}
			</button>
			<button
				class="rounded-lg border border-indigo-600 px-3 py-1.5 text-xs font-medium text-indigo-300 transition hover:bg-indigo-600/20 disabled:opacity-50"
				disabled={scanning}
				on:click={handleFullScan}
			>
				{scanning ? 'Scanning...' : 'Full Scan'}
			</button>
			<button
				class="rounded-lg bg-indigo-600 px-3 py-1.5 text-xs font-medium text-white transition hover:bg-indigo-500 disabled:opacity-50"
				disabled={generating}
				on:click={handleGenerate}
			>
				{generating ? 'Generating...' : 'Generate'}
			</button>
		</div>
	</div>

	<!-- Filter chips -->
	{#if insightTypes.length > 1}
		<div class="flex flex-wrap gap-1.5">
			<button
				class="rounded-full px-2.5 py-1 text-[11px] font-medium transition {filterType === 'all'
					? 'bg-white/10 text-white'
					: 'text-slate-400 hover:text-slate-200'}"
				on:click={() => (filterType = 'all')}
			>
				All ({$insights.length})
			</button>
			{#each insightTypes as type}
				<button
					class="rounded-full px-2.5 py-1 text-[11px] font-medium transition {filterType ===
					type
						? 'bg-white/10 text-white'
						: 'text-slate-400 hover:text-slate-200'}"
					on:click={() => (filterType = type)}
				>
					{type.replace(/_/g, ' ')} ({$insights.filter((i) => i.insight_type === type)
						.length})
				</button>
			{/each}
		</div>
	{/if}

	<!-- Conflict Alerts -->
	{#if conflicts.length > 0}
		<div>
			<div class="mb-2 flex items-center gap-2">
				<span class="text-xs font-semibold text-red-300">Conflict Alerts</span>
				<span
					class="rounded-full bg-red-500/20 px-2 py-0.5 text-[10px] font-medium text-red-300"
				>
					{conflicts.length}
				</span>
			</div>
			<div class="space-y-2">
				{#each conflicts as conflict (conflict.id)}
					<div
						class="rounded-xl border border-red-900/40 bg-red-950/20 p-4 transition hover:border-red-800/60"
					>
						<div class="flex items-start justify-between gap-3">
							<div class="min-w-0 flex-1">
								<div class="mb-1 flex items-center gap-2">
									<span class="rounded bg-red-500/20 px-1.5 py-0.5 text-[10px] font-medium text-red-300">
										{conflict.conflict_type}
									</span>
									<span class="text-[10px] text-slate-500">
										score: {conflict.score.toFixed(2)}
									</span>
								</div>
								<p class="text-xs text-slate-300">{conflict.explanation}</p>
							</div>
							<button
								class="shrink-0 rounded-lg border border-slate-700 px-2.5 py-1 text-[11px] text-slate-300 transition hover:bg-slate-800"
								on:click={() => handleResolveConflict(conflict.id)}
							>
								Dismiss
							</button>
						</div>
					</div>
				{/each}
			</div>
		</div>
	{/if}

	<!-- Embedding Clusters -->
	{#if showClusters}
		<div>
			<div class="mb-2 flex items-center justify-between">
				<div class="flex items-center gap-2">
					<span class="text-xs font-semibold text-purple-300">Embedding Clusters</span>
					{#if clusters.length > 0}
						<span
							class="rounded-full bg-purple-500/20 px-2 py-0.5 text-[10px] font-medium text-purple-300"
						>
							{clusters.length}
						</span>
					{/if}
				</div>
				<button
					class="text-[10px] text-slate-500 hover:text-slate-300"
					on:click={() => {
						showClusters = false;
						clusters = [];
					}}
				>
					Hide
				</button>
			</div>
			{#if loadingClusters}
				<div class="rounded-xl border border-slate-800 p-6 text-center text-xs text-slate-400">
					Analyzing embedding space...
				</div>
			{:else if clusters.length === 0}
				<div
					class="rounded-xl border border-dashed border-purple-900/40 p-4 text-center text-xs text-slate-400"
				>
					No unlinked clusters found — nodes are well-connected.
				</div>
			{:else}
				<div class="space-y-2">
					{#each clusters as cluster (cluster.id)}
						<div
							class="rounded-xl border border-purple-900/40 bg-purple-950/20 p-4 transition hover:border-purple-800/60"
						>
							<div class="mb-1 flex items-center gap-2">
								<span
									class="rounded bg-purple-500/20 px-1.5 py-0.5 text-[10px] font-medium text-purple-300"
								>
									{cluster.insight_type.replace(/_/g, ' ')}
								</span>
								{#if cluster.importance >= 0.7}
									<span class="text-[10px] text-amber-400">High priority</span>
								{/if}
							</div>
							<h3 class="text-sm font-medium text-white">{cluster.title}</h3>
							<p class="mt-1 text-xs text-slate-400 leading-relaxed">
								{cluster.content}
							</p>
							{#if cluster.related_node_ids.length > 0}
								<p class="mt-1.5 text-[10px] text-slate-500">
									Nodes in cluster: {cluster.related_node_ids.length}
								</p>
							{/if}
						</div>
					{/each}
				</div>
			{/if}
		</div>
	{/if}

	<!-- Insights -->
	{#if loading}
		<div class="rounded-xl border border-slate-800 p-8 text-center text-xs text-slate-400">
			Loading insights...
		</div>
	{:else if filteredInsights.length === 0 && conflicts.length === 0}
		<div class="rounded-xl border border-dashed border-slate-800 p-8 text-center">
			<div class="flex justify-center">
				<div
					class="flex h-14 w-14 items-center justify-center rounded-2xl bg-indigo-500/10 text-indigo-300"
				>
					<svg class="h-7 w-7" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							stroke-width="1.5"
							d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z"
						/>
					</svg>
				</div>
			</div>
			<h3 class="mt-3 text-sm font-semibold text-white">No insights yet</h3>
			<p class="mt-1 text-xs text-slate-400">
				Click "Generate Insights" to discover patterns in your vault.
			</p>
		</div>
	{:else}
		<div class="space-y-3">
			{#each filteredInsights as insight (insight.id)}
				<div
					class="rounded-xl border border-slate-800/60 bg-slate-900/40 p-4 transition hover:border-slate-700"
				>
					<div class="flex items-start justify-between gap-3">
						<div class="min-w-0 flex-1">
							<div class="mb-1.5 flex items-center gap-2">
								<span
									class="rounded px-1.5 py-0.5 text-[10px] font-medium {typeColor(
										insight.insight_type
									)}"
								>
									{insight.insight_type.replace(/_/g, ' ')}
								</span>
								{#if insight.importance >= 0.7}
									<span class="text-[10px] text-amber-400">High priority</span>
								{/if}
							</div>
							<h3 class="text-sm font-medium text-white">{insight.title}</h3>
							<p class="mt-1 text-xs text-slate-400 leading-relaxed">
								{insight.content}
							</p>
							{#if insight.related_node_ids.length > 0}
								<p class="mt-1.5 text-[10px] text-slate-500">
									Related nodes: {insight.related_node_ids.length}
								</p>
							{/if}
						</div>
						<button
							class="shrink-0 rounded-lg border border-slate-700 px-2.5 py-1 text-[11px] text-slate-300 transition hover:bg-slate-800"
							on:click={() => handleDismiss(insight.id)}
						>
							Dismiss
						</button>
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>
