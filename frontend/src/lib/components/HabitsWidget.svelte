<script lang="ts">
	import type { BriefingHabit } from '$lib/api/briefing';
	import { toggleHabitCompletionOptimistic } from '$lib/stores/goals';

	export let habits: BriefingHabit[] = [];
	export let loading: boolean = false;

	async function toggleHabit(habit: BriefingHabit) {
		try {
			await toggleHabitCompletionOptimistic(habit.id);
			// Update local state
			habits = habits.map((h) =>
				h.id === habit.id ? { ...h, completed_today: !h.completed_today } : h
			);
		} catch (e) {
			console.error('Failed to toggle habit', e);
		}
	}
</script>

<div class="rounded-xl border border-slate-800 bg-slate-900/40 p-5">
	<h3 class="text-sm font-semibold text-white">Today's Habits</h3>
	<div class="mt-3 space-y-2">
		{#if loading}
			<p class="text-xs text-slate-500">Loading...</p>
		{:else if habits.length === 0}
			<p class="py-4 text-center text-xs text-slate-500">No habits tracked yet.</p>
		{:else}
			{#each habits as habit (habit.id)}
				<button
					on:click={() => toggleHabit(habit)}
					class="flex w-full items-center justify-between rounded-lg border border-slate-800/60 px-3 py-2.5 transition hover:border-slate-700 text-left"
				>
					<div class="flex items-center gap-2.5">
						<div
							class="h-5 w-5 flex-shrink-0 rounded border transition {habit.completed_today
								? 'bg-emerald-500 border-emerald-500'
								: 'border-slate-600 hover:border-emerald-400'}"
						>
							{#if habit.completed_today}
								<svg class="h-5 w-5 text-white" viewBox="0 0 20 20" fill="currentColor">
									<path
										fill-rule="evenodd"
										d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
										clip-rule="evenodd"
									/>
								</svg>
							{/if}
						</div>
						<span class="text-xs font-medium text-white">{habit.name}</span>
					</div>
					<div class="flex items-center gap-2">
						{#if habit.current_streak > 0}
							<span class="text-[10px] text-amber-400">🔥 {habit.current_streak} day streak</span>
						{/if}
					</div>
				</button>
			{/each}
		{/if}
	</div>
</div>
