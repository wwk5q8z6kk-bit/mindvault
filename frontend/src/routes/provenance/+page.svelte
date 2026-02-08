<script lang="ts">
	import { onMount } from 'svelte';
	import { pushToast } from '$lib/stores/toast';
	import { fetchJson } from '$lib/api/client';

	interface MetricBucket {
		period: string;
		intents_generated: number;
		intents_applied: number;
		intents_dismissed: number;
		nodes_created: number;
		nodes_updated: number;
		proposals_submitted: number;
		proposals_approved: number;
		proposals_rejected: number;
	}

	interface ChronicleEntry {
		id: string;
		step_label: string;
		reasoning: string;
		created_at: string;
		namespace?: string;
	}

	let metrics: MetricBucket[] = [];
	let chronicle: ChronicleEntry[] = [];
	let loading = true;
	let tab: 'metrics' | 'chronicle' = 'metrics';

	onMount(async () => {
		loading = true;
		try {
			const [metricsRes, chronicleRes] = await Promise.all([
				fetchJson<{ buckets: MetricBucket[] }>('/api/v1/metrics/daily?days=7').catch(() => ({
					buckets: []
				})),
				fetchJson<{ entries: ChronicleEntry[] }>('/api/v1/agent/chronicle?limit=50').catch(
					() => ({ entries: [] })
				)
			]);
			metrics = metricsRes.buckets;
			chronicle = chronicleRes.entries;
		} catch {
			pushToast('Failed to load provenance data', 'danger');
		} finally {
			loading = false;
		}
	});
</script>

<div class="mx-auto max-w-4xl space-y-6 p-6">
	<h1 class="text-2xl font-bold text-[rgb(var(--mv-text))]">Provenance & Observability</h1>
	<p class="text-sm text-[rgb(var(--mv-muted))]">
		Audit trail, agent activity metrics, and transparency logs.
	</p>

	<!-- Tabs -->
	<div class="flex gap-4 border-b border-[rgb(var(--mv-border))]">
		<button
			class="border-b-2 px-2 pb-2 text-sm {tab === 'metrics'
				? 'border-blue-500 text-blue-500'
				: 'border-transparent text-[rgb(var(--mv-muted))]'}"
			onclick={() => (tab = 'metrics')}
		>Metrics</button>
		<button
			class="border-b-2 px-2 pb-2 text-sm {tab === 'chronicle'
				? 'border-blue-500 text-blue-500'
				: 'border-transparent text-[rgb(var(--mv-muted))]'}"
			onclick={() => (tab = 'chronicle')}
		>Chronicle</button>
	</div>

	{#if loading}
		<p class="text-[rgb(var(--mv-muted))]">Loading...</p>
	{:else if tab === 'metrics'}
		{#if metrics.length === 0}
			<p class="text-sm text-[rgb(var(--mv-muted))]">
				No metrics data available yet. Metrics accumulate as agents process your vault.
			</p>
		{:else}
			<div class="overflow-x-auto">
				<table class="w-full text-sm">
					<thead>
						<tr class="border-b border-[rgb(var(--mv-border))] text-left text-xs text-[rgb(var(--mv-muted))]">
							<th class="py-2 pr-4">Period</th>
							<th class="py-2 pr-4">Intents</th>
							<th class="py-2 pr-4">Applied</th>
							<th class="py-2 pr-4">Dismissed</th>
							<th class="py-2 pr-4">Nodes Created</th>
							<th class="py-2 pr-4">Proposals</th>
						</tr>
					</thead>
					<tbody>
						{#each metrics as bucket}
							<tr class="border-b border-[rgb(var(--mv-border))]">
								<td class="py-2 pr-4 text-[rgb(var(--mv-text))]">{bucket.period}</td>
								<td class="py-2 pr-4 text-[rgb(var(--mv-text))]">{bucket.intents_generated}</td>
								<td class="py-2 pr-4 text-green-400">{bucket.intents_applied}</td>
								<td class="py-2 pr-4 text-red-400">{bucket.intents_dismissed}</td>
								<td class="py-2 pr-4 text-[rgb(var(--mv-text))]">{bucket.nodes_created}</td>
								<td class="py-2 pr-4 text-[rgb(var(--mv-text))]">
									{bucket.proposals_approved}/{bucket.proposals_submitted}
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		{/if}
	{:else}
		<div class="space-y-2">
			{#if chronicle.length === 0}
				<p class="text-sm text-[rgb(var(--mv-muted))]">No chronicle entries yet.</p>
			{/if}
			{#each chronicle as entry (entry.id)}
				<div class="rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))] p-3">
					<div class="flex items-center justify-between">
						<span class="text-sm font-medium text-[rgb(var(--mv-text))]">{entry.step_label}</span>
						<span class="text-xs text-[rgb(var(--mv-muted))]">
							{new Date(entry.created_at).toLocaleString()}
						</span>
					</div>
					<p class="mt-1 text-xs text-[rgb(var(--mv-muted))]">{entry.reasoning}</p>
				</div>
			{/each}
		</div>
	{/if}
</div>
