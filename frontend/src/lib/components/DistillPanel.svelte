<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { runDistill, type DistillResult } from '$lib/api/distill';
	import { pushToast } from '$lib/stores/toast';

	export let open = false;
	export let defaultNamespace = 'default';

	type DistillMode = 'namespace' | 'temporal' | 'topic_deep_dive';

	const dispatch = createEventDispatcher<{
		close: void;
		completed: DistillResult;
	}>();

	let mode: DistillMode = 'namespace';
	let namespace = defaultNamespace;
	let maxNodes = 50;
	let days = 7;
	let topic = '';
	let loading = false;
	let result: DistillResult | null = null;
	let renderedSummary = '';
	let streamTimer: ReturnType<typeof setInterval> | null = null;

	$: if (defaultNamespace && !namespace) {
		namespace = defaultNamespace;
	}

	function stopStreaming() {
		if (streamTimer) {
			clearInterval(streamTimer);
			streamTimer = null;
		}
	}

	function streamSummary(summary: string) {
		stopStreaming();
		renderedSummary = '';
		let idx = 0;
		streamTimer = setInterval(() => {
			idx = Math.min(summary.length, idx + 14);
			renderedSummary = summary.slice(0, idx);
			if (idx >= summary.length) {
				stopStreaming();
			}
		}, 18);
	}

	async function handleDistill() {
		loading = true;
		result = null;
		renderedSummary = '';
		try {
			const payload =
				mode === 'namespace'
					? { kind: 'namespace' as const, namespace: namespace.trim() || 'default', max_nodes: maxNodes }
					: mode === 'temporal'
						? { kind: 'temporal' as const, days }
						: { kind: 'topic_deep_dive' as const, topic: topic.trim() };
			if (mode === 'topic_deep_dive' && !topic.trim()) {
				pushToast('Enter a topic to summarize', 'warning');
				return;
			}
			result = await runDistill(payload);
			streamSummary(result.summary);
			dispatch('completed', result);
		} catch {
			pushToast('Failed to distill knowledge', 'danger');
		} finally {
			loading = false;
		}
	}

	function closePanel() {
		open = false;
		dispatch('close');
	}
</script>

{#if open}
	<div class="fixed inset-0 z-50 flex items-center justify-center p-4" role="presentation">
		<div
			class="absolute inset-0 bg-black/60"
			role="button"
			tabindex="0"
			aria-label="Close"
			on:click={closePanel}
			on:keydown={(e) => e.key === 'Escape' && closePanel()}
		></div>
		<div class="relative z-10 w-full max-w-3xl rounded-2xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))] p-5 shadow-2xl">
			<div class="flex items-start justify-between gap-3">
				<div>
					<h3 class="text-base font-semibold text-[rgb(var(--mv-text))]">Distill Knowledge</h3>
					<p class="text-xs text-[rgb(var(--mv-muted))]">
						Generate concise summaries by namespace, time range, or topic deep dive.
					</p>
				</div>
				<button
					class="rounded-lg border border-[rgb(var(--mv-border))] px-2 py-1 text-xs text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-panel-strong))]"
					on:click={closePanel}
				>
					Close
				</button>
			</div>

			<div class="mt-4 flex flex-wrap items-center gap-2">
				<button
					class={`rounded-lg border px-3 py-1.5 text-xs ${mode === 'namespace' ? 'border-sky-500/40 bg-sky-500/10 text-sky-200' : 'border-[rgb(var(--mv-border))] text-[rgb(var(--mv-muted))]'}`}
					on:click={() => (mode = 'namespace')}
				>
					Namespace
				</button>
				<button
					class={`rounded-lg border px-3 py-1.5 text-xs ${mode === 'temporal' ? 'border-sky-500/40 bg-sky-500/10 text-sky-200' : 'border-[rgb(var(--mv-border))] text-[rgb(var(--mv-muted))]'}`}
					on:click={() => (mode = 'temporal')}
				>
					Temporal
				</button>
				<button
					class={`rounded-lg border px-3 py-1.5 text-xs ${mode === 'topic_deep_dive' ? 'border-sky-500/40 bg-sky-500/10 text-sky-200' : 'border-[rgb(var(--mv-border))] text-[rgb(var(--mv-muted))]'}`}
					on:click={() => (mode = 'topic_deep_dive')}
				>
					Topic
				</button>
			</div>

			<div class="mt-4 grid gap-3 md:grid-cols-3">
				{#if mode === 'namespace'}
					<div>
						<label for="distill-namespace" class="mb-1 block text-xs text-[rgb(var(--mv-muted))]">Namespace</label>
						<input
							id="distill-namespace"
							class="w-full rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))] px-3 py-2 text-sm text-[rgb(var(--mv-text))]"
							bind:value={namespace}
						/>
					</div>
					<div>
						<label for="distill-max-nodes" class="mb-1 block text-xs text-[rgb(var(--mv-muted))]">Max Nodes</label>
						<input
							id="distill-max-nodes"
							type="number"
							min="5"
							max="500"
							class="w-full rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))] px-3 py-2 text-sm text-[rgb(var(--mv-text))]"
							bind:value={maxNodes}
						/>
					</div>
				{:else if mode === 'temporal'}
					<div>
						<label for="distill-days" class="mb-1 block text-xs text-[rgb(var(--mv-muted))]">Days Back</label>
						<input
							id="distill-days"
							type="number"
							min="1"
							max="365"
							class="w-full rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))] px-3 py-2 text-sm text-[rgb(var(--mv-text))]"
							bind:value={days}
						/>
					</div>
				{:else}
					<div class="md:col-span-2">
						<label for="distill-topic" class="mb-1 block text-xs text-[rgb(var(--mv-muted))]">Topic</label>
						<input
							id="distill-topic"
							class="w-full rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))] px-3 py-2 text-sm text-[rgb(var(--mv-text))]"
							placeholder="e.g. project roadmap, onboarding gaps, database migration"
							bind:value={topic}
						/>
					</div>
				{/if}
			</div>

			<div class="mt-4 flex items-center gap-2">
				<button
					class="rounded-lg bg-sky-500 px-4 py-2 text-xs font-semibold text-white hover:bg-sky-400 disabled:opacity-60"
					disabled={loading}
					on:click={handleDistill}
				>
					{loading ? 'Summarizing…' : 'Run Distillation'}
				</button>
				{#if result}
					<span class="text-[10px] text-[rgb(var(--mv-muted))]">
						{result.source_count} sources • {new Date(result.generated_at).toLocaleTimeString()}
					</span>
				{/if}
			</div>

			<div class="mt-4 max-h-[50vh] overflow-y-auto rounded-xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))]/30 p-4">
				{#if loading}
					<p class="text-sm text-[rgb(var(--mv-muted))]/70">Gathering context and generating summary…</p>
				{:else if result}
					<h4 class="mb-2 text-sm font-semibold text-[rgb(var(--mv-text))]">{result.title}</h4>
					<pre class="whitespace-pre-wrap font-sans text-sm leading-relaxed text-[rgb(var(--mv-text))]/90">{renderedSummary}</pre>
				{:else}
					<p class="text-sm text-[rgb(var(--mv-muted))]/70">Run a distillation to view summary output here.</p>
				{/if}
			</div>
		</div>
	</div>
{/if}
