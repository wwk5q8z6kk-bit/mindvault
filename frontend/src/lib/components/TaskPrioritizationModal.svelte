<script lang="ts">
	import { fade, fly } from 'svelte/transition';
	import { prioritizeTasks, type PrioritizedTaskItem, type PrioritizeTasksRequest } from '$lib/api/ai';
	import type { TaskStatus } from '$lib/api/tasks';
	import { updateTaskOptimistic } from '$lib/stores/tasks';
	import { pushToast } from '$lib/stores/toast';
	import { createEventDispatcher } from 'svelte';

	export let open = false;

	const dispatch = createEventDispatcher<{ close: void; applied: { count: number } }>();

	let loading = false;
	let error: string | null = null;
	let results: PrioritizedTaskItem[] = [];
	let provider: string | null = null;
	let generatedAt: string | null = null;

	// Options
	let limit = 20;
	let includeDone = false;
	let selectedStatuses: Set<TaskStatus> = new Set(['inbox', 'planned', 'in_progress', 'waiting', 'review']);

	// Selection for applying
	let selectedForApply: Set<string> = new Set();
	let applyingPriorities = false;

	const allStatuses: TaskStatus[] = ['inbox', 'planned', 'in_progress', 'waiting', 'review', 'done'];

	function close() {
		open = false;
		results = [];
		error = null;
		selectedForApply = new Set();
		dispatch('close');
	}

	function toggleStatus(status: TaskStatus) {
		const newSet = new Set(selectedStatuses);
		if (newSet.has(status)) {
			if (newSet.size > 1) newSet.delete(status);
		} else {
			newSet.add(status);
		}
		selectedStatuses = newSet;
	}

	function toggleSelectForApply(taskId: string) {
		const newSet = new Set(selectedForApply);
		if (newSet.has(taskId)) {
			newSet.delete(taskId);
		} else {
			newSet.add(taskId);
		}
		selectedForApply = newSet;
	}

	function selectAllForApply() {
		selectedForApply = new Set(results.map((r) => r.task.id));
	}

	function deselectAllForApply() {
		selectedForApply = new Set();
	}

	async function runPrioritization() {
		loading = true;
		error = null;
		results = [];

		try {
			const request: PrioritizeTasksRequest = {
				limit,
				include_done: includeDone,
				statuses: [...selectedStatuses]
			};

			const response = await prioritizeTasks(request);
			results = response.items;
			provider = response.provider ?? null;
			generatedAt = response.generated_at;

			// Auto-select top 5 for apply
			selectedForApply = new Set(results.slice(0, 5).map((r) => r.task.id));
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to prioritize tasks';
		} finally {
			loading = false;
		}
	}

	async function applyPriorities() {
		if (selectedForApply.size === 0) {
			pushToast('Select tasks to apply priorities', 'warning');
			return;
		}

		applyingPriorities = true;
		let applied = 0;

		// Get selected items sorted by rank
		const itemsToApply = results
			.filter((r) => selectedForApply.has(r.task.id))
			.sort((a, b) => a.rank - b.rank);

		for (const item of itemsToApply) {
			try {
				// Map score to priority: 0.8+ = p1, 0.6-0.8 = p2, 0.4-0.6 = p3, <0.4 = p4
				let priority: number;
				if (item.score >= 0.8) priority = 1;
				else if (item.score >= 0.6) priority = 2;
				else if (item.score >= 0.4) priority = 3;
				else priority = 4;

				await updateTaskOptimistic(item.task.id, { priority });
				applied++;
			} catch {
				// continue with others
			}
		}

		applyingPriorities = false;

		if (applied > 0) {
			pushToast(`Applied priority to ${applied} task${applied > 1 ? 's' : ''}`, 'success');
			dispatch('applied', { count: applied });
		}
	}

	function formatScore(score: number): string {
		return (score * 100).toFixed(0) + '%';
	}

	function getScoreColor(score: number): string {
		if (score >= 0.8) return 'text-red-400';
		if (score >= 0.6) return 'text-orange-400';
		if (score >= 0.4) return 'text-yellow-400';
		return 'text-slate-400';
	}

	function getScoreBg(score: number): string {
		if (score >= 0.8) return 'bg-red-500/20';
		if (score >= 0.6) return 'bg-orange-500/20';
		if (score >= 0.4) return 'bg-yellow-500/20';
		return 'bg-slate-500/20';
	}

	function getPriorityLabel(score: number): string {
		if (score >= 0.8) return 'P1 - Critical';
		if (score >= 0.6) return 'P2 - High';
		if (score >= 0.4) return 'P3 - Medium';
		return 'P4 - Low';
	}

	$: if (open && results.length === 0 && !loading && !error) {
		void runPrioritization();
	}
</script>

{#if open}
	<div
		class="fixed inset-0 z-50 flex items-center justify-center p-4"
		role="dialog"
		aria-modal="true"
		aria-labelledby="prioritization-title"
		transition:fade={{ duration: 100 }}
	>
		<div
			class="absolute inset-0 bg-black/60 backdrop-blur-sm"
			on:click={close}
			on:keydown={(e) => e.key === 'Escape' && close()}
			role="button"
			tabindex="-1"
			aria-label="Close"
		></div>

		<div
			class="relative z-10 flex max-h-[85vh] w-full max-w-3xl flex-col overflow-hidden rounded-2xl border border-slate-700 bg-slate-900/95 shadow-2xl"
			transition:fly={{ y: 20, duration: 150 }}
		>
			<!-- Header -->
			<div class="flex items-center justify-between border-b border-slate-800 px-6 py-4">
				<div>
					<h2 id="prioritization-title" class="text-lg font-semibold text-white">
						AI Task Prioritization
					</h2>
					<p class="text-xs text-slate-400">
						Let AI analyze and rank your tasks by importance
					</p>
				</div>
				<button
					class="rounded-lg p-2 text-slate-400 hover:bg-slate-800 hover:text-white"
					on:click={close}
					aria-label="Close"
				>
					<svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
					</svg>
				</button>
			</div>

			<!-- Options -->
			<div class="border-b border-slate-800 px-6 py-3">
				<div class="flex flex-wrap items-center gap-4">
					<div class="flex items-center gap-2">
						<label for="limit-input" class="text-xs text-slate-400">Max tasks:</label>
						<input
							id="limit-input"
							type="number"
							min="5"
							max="50"
							class="w-16 rounded-lg border border-slate-700 bg-slate-800 px-2 py-1 text-xs text-white"
							bind:value={limit}
						/>
					</div>

					<label class="flex items-center gap-2 text-xs text-slate-400">
						<input
							type="checkbox"
							class="rounded border-slate-600 bg-slate-800 text-sky-500"
							bind:checked={includeDone}
						/>
						Include done
					</label>

					<div class="flex items-center gap-1">
						<span class="text-xs text-slate-500">Statuses:</span>
						{#each allStatuses as status}
							<button
								class={`rounded px-2 py-0.5 text-[10px] transition ${
									selectedStatuses.has(status)
										? 'bg-sky-500/20 text-sky-300'
										: 'bg-slate-800 text-slate-500 hover:text-slate-300'
								}`}
								on:click={() => toggleStatus(status)}
							>
								{status.replace('_', ' ')}
							</button>
						{/each}
					</div>

					<button
						class="ml-auto rounded-lg border border-slate-700 px-3 py-1.5 text-xs text-slate-300 hover:bg-slate-800"
						on:click={runPrioritization}
						disabled={loading}
					>
						{loading ? 'Analyzing...' : 'Re-analyze'}
					</button>
				</div>
			</div>

			<!-- Content -->
			<div class="flex-1 overflow-y-auto px-6 py-4">
				{#if loading}
					<div class="flex flex-col items-center justify-center py-12">
						<div class="h-8 w-8 animate-spin rounded-full border-2 border-slate-600 border-t-sky-500"></div>
						<p class="mt-4 text-sm text-slate-400">AI is analyzing your tasks...</p>
						<p class="mt-1 text-xs text-slate-500">This may take a few seconds</p>
					</div>
				{:else if error}
					<div class="rounded-lg border border-red-500/30 bg-red-500/10 p-4 text-center">
						<p class="text-sm text-red-300">{error}</p>
						<button
							class="mt-3 rounded-lg bg-red-500/20 px-4 py-2 text-xs text-red-300 hover:bg-red-500/30"
							on:click={runPrioritization}
						>
							Try again
						</button>
					</div>
				{:else if results.length === 0}
					<div class="py-12 text-center text-sm text-slate-400">
						No tasks found matching your criteria.
					</div>
				{:else}
					<div class="mb-3 flex items-center justify-between">
						<div class="text-xs text-slate-500">
							{results.length} tasks ranked
							{#if provider}
								<span class="text-slate-600">via {provider}</span>
							{/if}
						</div>
						<div class="flex gap-2">
							<button
								class="text-xs text-slate-400 hover:text-white"
								on:click={selectAllForApply}
							>
								Select all
							</button>
							<button
								class="text-xs text-slate-400 hover:text-white"
								on:click={deselectAllForApply}
							>
								Clear
							</button>
						</div>
					</div>

					<div class="space-y-2">
						{#each results as item, i (item.task.id)}
							<div
								class={`flex items-start gap-3 rounded-lg border p-3 transition ${
									selectedForApply.has(item.task.id)
										? 'border-sky-500/50 bg-sky-500/5'
										: 'border-slate-800 bg-slate-800/30 hover:border-slate-700'
								}`}
							>
								<label class="flex items-center pt-1">
									<input
										type="checkbox"
										class="h-4 w-4 rounded border-slate-600 bg-slate-800 text-sky-500"
										checked={selectedForApply.has(item.task.id)}
										on:change={() => toggleSelectForApply(item.task.id)}
									/>
								</label>

								<div class="flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-slate-700 text-xs font-bold text-slate-300">
									{item.rank}
								</div>

								<div class="min-w-0 flex-1">
									<div class="flex items-center gap-2">
										<span class="truncate text-sm font-medium text-white">{item.task.title}</span>
										<span class={`rounded px-1.5 py-0.5 text-[10px] font-medium ${getScoreBg(item.score)} ${getScoreColor(item.score)}`}>
											{formatScore(item.score)}
										</span>
									</div>

									<div class="mt-1 flex items-center gap-2 text-[11px] text-slate-500">
										<span class={`rounded bg-slate-800 px-1.5 py-0.5 ${getScoreColor(item.score)}`}>
											{getPriorityLabel(item.score)}
										</span>
										<span class="rounded bg-slate-800 px-1.5 py-0.5">
											{item.task.status.replace('_', ' ')}
										</span>
										{#if item.task.due_at}
											<span class="rounded bg-slate-800 px-1.5 py-0.5">
												Due: {new Date(item.task.due_at).toLocaleDateString()}
											</span>
										{/if}
									</div>

									{#if item.reason}
										<p class="mt-2 text-xs text-slate-400 italic">
											"{item.reason}"
										</p>
									{/if}
								</div>
							</div>
						{/each}
					</div>
				{/if}
			</div>

			<!-- Footer -->
			{#if results.length > 0}
				<div class="flex items-center justify-between border-t border-slate-800 px-6 py-4">
					<div class="text-xs text-slate-500">
						{selectedForApply.size} task{selectedForApply.size !== 1 ? 's' : ''} selected
					</div>
					<div class="flex gap-2">
						<button
							class="rounded-lg border border-slate-700 px-4 py-2 text-xs text-slate-300 hover:bg-slate-800"
							on:click={close}
						>
							Cancel
						</button>
						<button
							class="rounded-lg bg-sky-500 px-4 py-2 text-xs font-semibold text-white hover:bg-sky-400 disabled:opacity-50"
							on:click={applyPriorities}
							disabled={selectedForApply.size === 0 || applyingPriorities}
						>
							{applyingPriorities ? 'Applying...' : `Apply Priorities (${selectedForApply.size})`}
						</button>
					</div>
				</div>
			{/if}
		</div>
	</div>
{/if}
