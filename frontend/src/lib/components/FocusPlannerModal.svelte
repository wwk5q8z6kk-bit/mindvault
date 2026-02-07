<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { focusPlannerState, selectedTaskId } from '$lib/stores/ui';

	$: state = $focusPlannerState;

	function close() {
		focusPlannerState.set({ open: false, generatedAt: state.generatedAt, items: [] });
	}

	function formatDue(value?: string | null) {
		if (!value) return 'No due date';
		const date = new Date(value);
		if (Number.isNaN(date.getTime())) return 'No due date';
		return date.toLocaleString();
	}

	async function openTask(taskId: string) {
		selectedTaskId.set(taskId);
		close();
		await goto(resolve('/tasks'));
	}
</script>

{#if state.open}
	<div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4" role="presentation">
		<div
			class="w-full max-w-3xl rounded-2xl border border-slate-800 bg-slate-950 p-6"
			role="dialog"
			aria-modal="true"
			aria-labelledby="focus-planner-title"
		>
			<div class="flex items-center justify-between">
				<div>
					<h3 id="focus-planner-title" class="text-lg font-semibold text-white">
						Focus list
					</h3>
					<p class="text-xs text-slate-400">
						Generated {state.generatedAt ? new Date(state.generatedAt).toLocaleString() : 'now'}
					</p>
				</div>
				<button
					class="rounded-lg border border-slate-800 px-2 py-1 text-xs text-slate-300"
					on:click={close}
				>
					Close
				</button>
			</div>

			<div class="mt-6 grid gap-3">
				{#each state.items as item (item.task.id)}
					<div class="rounded-xl border border-slate-800 bg-slate-900/60 p-4">
						<div class="flex items-start justify-between gap-4">
							<div>
								<div class="text-xs uppercase tracking-wide text-slate-500">
									#{item.rank} • score {item.score.toFixed(2)} • P{item.task.priority}
								</div>
								<div class="mt-1 text-base font-semibold text-white">
									{item.task.title}
								</div>
								<div class="mt-1 text-xs text-slate-400">
									{item.reason ?? 'Balanced priority'}
								</div>
								<div class="mt-2 text-xs text-slate-500">{formatDue(item.task.due_at)}</div>
							</div>
							<button
								class="rounded-lg border border-slate-800 px-3 py-2 text-xs text-slate-300"
								on:click={() => openTask(item.task.id)}
							>
								Open
							</button>
						</div>
					</div>
				{/each}
			</div>
		</div>
	</div>
{/if}
