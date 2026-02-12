<script lang="ts">
	import { onMount } from 'svelte';
	import {
		getHealthDiagnostics,
		getEmbeddingDiagnostics,
		getPerformanceDiagnostics,
		getPrometheusMetrics,
		type HealthDiagnostics,
		type EmbeddingDiagnostics,
		type PerformanceDiagnostics
	} from '$lib/api/diagnostics';
	import { pushToast } from '$lib/stores/toast';

	let loading = true;
	let metricsLoading = false;
	let health: HealthDiagnostics | null = null;
	let embedding: EmbeddingDiagnostics | null = null;
	let performance: PerformanceDiagnostics | null = null;
	let metricsText = '';

	onMount(() => {
		void refreshDiagnostics();
	});

	async function refreshDiagnostics() {
		loading = true;
		try {
			const [nextHealth, nextEmbedding, nextPerformance] = await Promise.all([
				getHealthDiagnostics(),
				getEmbeddingDiagnostics(),
				getPerformanceDiagnostics()
			]);
			health = nextHealth;
			embedding = nextEmbedding;
			performance = nextPerformance;
		} catch {
			pushToast('Failed to load diagnostics', 'danger');
		} finally {
			loading = false;
		}
	}

	async function loadMetrics() {
		metricsLoading = true;
		try {
			metricsText = await getPrometheusMetrics();
		} catch {
			pushToast('Failed to fetch /metrics', 'warning');
		} finally {
			metricsLoading = false;
		}
	}

	function latencyLabel(): string {
		const p95 = performance?.health_check_latency_ms?.p95;
		if (typeof p95 !== 'number') return 'n/a';
		return `${p95.toFixed(1)} ms`;
	}
</script>

<div class="mx-auto max-w-5xl space-y-6 p-6">
	<div class="flex flex-wrap items-start justify-between gap-3">
		<div>
			<h1 class="text-2xl font-bold text-[rgb(var(--mv-text))]">Diagnostics</h1>
			<p class="text-sm text-[rgb(var(--mv-muted))]">System health, embedding runtime, performance histograms, and Prometheus metrics.</p>
		</div>
		<div class="flex gap-2">
			<a
				href="/settings"
				class="rounded-lg border border-[rgb(var(--mv-border))] px-3 py-1.5 text-xs text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-panel-strong))]"
			>
				Back to Settings
			</a>
			<button
				class="rounded-lg border border-[rgb(var(--mv-border))] px-3 py-1.5 text-xs text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-panel-strong))]"
				on:click={refreshDiagnostics}
			>
				Refresh
			</button>
		</div>
	</div>

	{#if loading}
		<p class="text-sm text-[rgb(var(--mv-muted))]">Loading diagnostics...</p>
	{:else}
		<div class="grid grid-cols-2 gap-3 md:grid-cols-4">
			<div class="rounded-xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/40 p-3">
				<p class="text-[10px] uppercase tracking-wide text-[rgb(var(--mv-muted))]/70">Health</p>
				<p class="mt-1 text-lg font-semibold {health?.status === 'healthy' ? 'text-green-300' : 'text-amber-300'}">{health?.status ?? 'unknown'}</p>
			</div>
			<div class="rounded-xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/40 p-3">
				<p class="text-[10px] uppercase tracking-wide text-[rgb(var(--mv-muted))]/70">DB Latency</p>
				<p class="mt-1 text-lg font-semibold text-[rgb(var(--mv-text))]">{health?.database?.latency_ms ?? 0} ms</p>
			</div>
			<div class="rounded-xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/40 p-3">
				<p class="text-[10px] uppercase tracking-wide text-[rgb(var(--mv-muted))]/70">P95 Health Latency</p>
				<p class="mt-1 text-lg font-semibold text-[rgb(var(--mv-text))]">{latencyLabel()}</p>
			</div>
			<div class="rounded-xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/40 p-3">
				<p class="text-[10px] uppercase tracking-wide text-[rgb(var(--mv-muted))]/70">Uptime</p>
				<p class="mt-1 text-lg font-semibold text-[rgb(var(--mv-text))]">{Math.floor((health?.uptime_seconds ?? 0) / 60)} min</p>
			</div>
		</div>

		<div class="grid gap-4 lg:grid-cols-2">
			<section class="rounded-xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/40 p-4">
				<h2 class="text-sm font-semibold text-[rgb(var(--mv-text))]">Subsystem Health</h2>
				<div class="mt-3 space-y-2 text-xs text-[rgb(var(--mv-muted))]">
					<div class="flex items-center justify-between rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))]/30 px-3 py-2">
						<span>Database</span>
						<span class={health?.database?.status === 'ok' ? 'text-green-300' : 'text-red-300'}>{health?.database?.status ?? 'unknown'}</span>
					</div>
					<div class="flex items-center justify-between rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))]/30 px-3 py-2">
						<span>Version</span>
						<span class="text-[rgb(var(--mv-text))]">{health?.version ?? 'n/a'}</span>
					</div>
				</div>
			</section>

			<section class="rounded-xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/40 p-4">
				<h2 class="text-sm font-semibold text-[rgb(var(--mv-text))]">Embedding Runtime</h2>
				<div class="mt-3 grid gap-2 text-xs text-[rgb(var(--mv-muted))]">
					<div class="rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))]/30 px-3 py-2">
						Configured: <span class="text-[rgb(var(--mv-text))]">{embedding?.configured_provider}:{embedding?.configured_model}</span>
					</div>
					<div class="rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))]/30 px-3 py-2">
						Effective: <span class="text-[rgb(var(--mv-text))]">{embedding?.effective_provider}:{embedding?.effective_model}</span>
					</div>
					<div class="rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))]/30 px-3 py-2">
						Fallback: <span class={embedding?.fallback_to_noop ? 'text-amber-300' : 'text-green-300'}>{embedding?.fallback_to_noop ? 'noop active' : 'not needed'}</span>
					</div>
					{#if embedding?.reason}
						<div class="rounded border border-amber-500/20 bg-amber-500/5 px-3 py-2 text-amber-200">{embedding.reason}</div>
					{/if}
				</div>
			</section>
		</div>

		<section class="rounded-xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/40 p-4">
			<div class="flex items-center justify-between">
				<h2 class="text-sm font-semibold text-[rgb(var(--mv-text))]">Prometheus Metrics</h2>
				<button
					class="rounded-lg border border-[rgb(var(--mv-border))] px-2.5 py-1 text-[10px] text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-panel-strong))] disabled:opacity-60"
					on:click={loadMetrics}
					disabled={metricsLoading}
				>
					{metricsLoading ? 'Loading...' : 'Fetch /metrics'}
				</button>
			</div>
			{#if metricsText}
				<pre class="mt-3 max-h-72 overflow-auto rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))]/30 p-3 text-[10px] leading-relaxed text-[rgb(var(--mv-muted))]">{metricsText}</pre>
			{:else}
				<p class="mt-2 text-xs text-[rgb(var(--mv-muted))]/70">Click \"Fetch /metrics\" to view raw Prometheus output.</p>
			{/if}
		</section>
	{/if}
</div>
