<script lang="ts">
	import { onMount } from 'svelte';
	import { get } from 'svelte/store';
	import { page } from '$app/stores';
	import type { GoalStatus, HabitFrequency } from '$lib/api/goals';
	import {
		activeHabits,
		createGoalOptimistic,
		createHabitOptimistic,
		deleteGoalOptimistic,
		deleteHabitOptimistic,
		filteredGoals,
		goalsFilter,
		goalsStore,
		habitsStore,
		loadGoalsAndHabits,
		toggleHabitCompletionOptimistic,
		updateGoalOptimistic,
		updateHabitOptimistic
	} from '$lib/stores/goals';
	import { toDateOnly } from '$lib/goals/habits';
	import { assistTransform } from '$lib/api/assist';
	import { pushToast } from '$lib/stores/toast';

	let selectedGoalId: string | null = null;
	let selectedHabitId: string | null = null;
	let loading = false;
	let savingGoal = false;
	let savingHabit = false;
	let generatingReview = false;
	let reviewText = '';

	let goalQuery = '';
	let goalStatusFilter: GoalStatus | 'all' = 'all';

	const goalStatusOptions: GoalStatus[] = ['active', 'on_track', 'at_risk', 'paused', 'completed'];
	const habitFrequencyOptions: HabitFrequency[] = ['daily', 'weekdays', 'weekly'];
	const today = toDateOnly(new Date());

	let newGoal = {
		title: '',
		description: '',
		target_date: '',
		priority: 3,
		status: 'active' as GoalStatus,
		tags: ''
	};

	let newHabit = {
		name: '',
		description: '',
		frequency: 'daily' as HabitFrequency,
		target_per_period: 1,
		tags: ''
	};

	let goalEditorId = '';
	let goalEditor = {
		title: '',
		description: '',
		target_date: '',
		priority: 3,
		progress: 0,
		status: 'active' as GoalStatus,
		tags: ''
	};

	let habitEditorId = '';
	let habitEditor = {
		name: '',
		description: '',
		frequency: 'daily' as HabitFrequency,
		target_per_period: 1,
		enabled: true,
		tags: ''
	};

	function parseTags(input: string): string[] {
		return [...new Set(input.split(/[,\s]+/).map((tag) => tag.trim().toLowerCase()).filter(Boolean))];
	}

	function setGoalFilter() {
		goalsFilter.set({ query: goalQuery, status: goalStatusFilter });
	}

	function openGoal(id: string) {
		selectedGoalId = id;
	}

	function openHabit(id: string) {
		selectedHabitId = id;
	}

	$: selectedGoal = $goalsStore.find((goal) => goal.id === selectedGoalId) ?? null;
	$: selectedHabit = $habitsStore.find((habit) => habit.id === selectedHabitId) ?? null;

	$: if (selectedGoal && selectedGoal.id !== goalEditorId) {
		goalEditorId = selectedGoal.id;
		goalEditor = {
			title: selectedGoal.title,
			description: selectedGoal.description ?? '',
			target_date: selectedGoal.target_date ?? '',
			priority: selectedGoal.priority,
			progress: selectedGoal.progress,
			status: selectedGoal.status,
			tags: selectedGoal.tags.join(' ')
		};
	}

	$: if (selectedHabit && selectedHabit.id !== habitEditorId) {
		habitEditorId = selectedHabit.id;
		habitEditor = {
			name: selectedHabit.name,
			description: selectedHabit.description ?? '',
			frequency: selectedHabit.frequency,
			target_per_period: selectedHabit.target_per_period,
			enabled: selectedHabit.enabled,
			tags: selectedHabit.tags.join(' ')
		};
	}

	$: activeGoalCount = $goalsStore.filter((goal) => goal.status !== 'completed').length;
	$: averageProgress =
		$goalsStore.length === 0
			? 0
			: Math.round($goalsStore.reduce((acc, goal) => acc + goal.progress, 0) / $goalsStore.length);
	$: completedHabitsToday = $habitsStore.filter((habit) => habit.checkins.includes(today)).length;
	$: bestHabitStreak = Math.max(0, ...$habitsStore.map((habit) => habit.best_streak));

	onMount(async () => {
		loading = true;
		const filter = get(goalsFilter);
		goalQuery = filter.query;
		goalStatusFilter = filter.status;
		try {
			await loadGoalsAndHabits();
			const goalFromUrl = get(page).url.searchParams.get('goal');
			const habitFromUrl = get(page).url.searchParams.get('habit');
			selectedGoalId = goalFromUrl ?? get(filteredGoals)[0]?.id ?? null;
			selectedHabitId = habitFromUrl ?? get(activeHabits)[0]?.id ?? get(habitsStore)[0]?.id ?? null;
		} catch {
			pushToast('Failed to load goals and habits.', 'danger');
		} finally {
			loading = false;
		}
	});

	async function createGoal() {
		if (!newGoal.title.trim()) {
			pushToast('Goal title is required.', 'warning');
			return;
		}
		savingGoal = true;
		try {
			const created = await createGoalOptimistic({
				title: newGoal.title.trim(),
				description: newGoal.description.trim() || null,
				target_date: newGoal.target_date || null,
				priority: newGoal.priority,
				status: newGoal.status,
				progress: 0,
				tags: parseTags(newGoal.tags)
			});
			selectedGoalId = created.id;
			newGoal = {
				title: '',
				description: '',
				target_date: '',
				priority: 3,
				status: 'active',
				tags: ''
			};
			pushToast('Goal created.', 'success');
		} catch {
			pushToast('Could not create goal.', 'danger');
		} finally {
			savingGoal = false;
		}
	}

	async function saveSelectedGoal() {
		if (!selectedGoal) return;
		savingGoal = true;
		try {
			await updateGoalOptimistic(selectedGoal.id, {
				title: goalEditor.title.trim(),
				description: goalEditor.description.trim() || null,
				target_date: goalEditor.target_date || null,
				priority: goalEditor.priority,
				progress: goalEditor.progress,
				status: goalEditor.status,
				tags: parseTags(goalEditor.tags)
			});
			pushToast('Goal updated.', 'success');
		} catch {
			pushToast('Unable to update goal.', 'danger');
		} finally {
			savingGoal = false;
		}
	}

	async function markGoalComplete() {
		if (!selectedGoal) return;
		try {
			await updateGoalOptimistic(selectedGoal.id, { status: 'completed', progress: 100 });
			pushToast('Goal marked complete.', 'success');
		} catch {
			pushToast('Unable to complete goal.', 'danger');
		}
	}

	async function removeSelectedGoal() {
		if (!selectedGoal) return;
		if (!confirm(`Delete goal "${selectedGoal.title}"?`)) return;
		try {
			await deleteGoalOptimistic(selectedGoal.id);
			selectedGoalId = get(filteredGoals)[0]?.id ?? null;
			pushToast('Goal deleted.', 'success');
		} catch {
			pushToast('Unable to delete goal.', 'danger');
		}
	}

	async function createHabit() {
		if (!newHabit.name.trim()) {
			pushToast('Habit name is required.', 'warning');
			return;
		}
		savingHabit = true;
		try {
			const created = await createHabitOptimistic({
				name: newHabit.name.trim(),
				description: newHabit.description.trim() || null,
				frequency: newHabit.frequency,
				target_per_period: newHabit.target_per_period,
				tags: parseTags(newHabit.tags),
				enabled: true
			});
			selectedHabitId = created.id;
			newHabit = {
				name: '',
				description: '',
				frequency: 'daily',
				target_per_period: 1,
				tags: ''
			};
			pushToast('Habit created.', 'success');
		} catch {
			pushToast('Could not create habit.', 'danger');
		} finally {
			savingHabit = false;
		}
	}

	async function saveSelectedHabit() {
		if (!selectedHabit) return;
		savingHabit = true;
		try {
			await updateHabitOptimistic(selectedHabit.id, {
				name: habitEditor.name.trim(),
				description: habitEditor.description.trim() || null,
				frequency: habitEditor.frequency,
				target_per_period: habitEditor.target_per_period,
				enabled: habitEditor.enabled,
				tags: parseTags(habitEditor.tags)
			});
			pushToast('Habit updated.', 'success');
		} catch {
			pushToast('Unable to update habit.', 'danger');
		} finally {
			savingHabit = false;
		}
	}

	async function removeSelectedHabit() {
		if (!selectedHabit) return;
		if (!confirm(`Delete habit "${selectedHabit.name}"?`)) return;
		try {
			await deleteHabitOptimistic(selectedHabit.id);
			selectedHabitId = get(habitsStore)[0]?.id ?? null;
			pushToast('Habit deleted.', 'success');
		} catch {
			pushToast('Unable to delete habit.', 'danger');
		}
	}

	async function toggleToday(habitId: string) {
		try {
			await toggleHabitCompletionOptimistic(habitId, today);
		} catch {
			pushToast('Unable to update habit check-in.', 'danger');
		}
	}

	async function generateReview() {
		const goals = get(goalsStore);
		const habits = get(habitsStore);
		if (goals.length === 0 && habits.length === 0) {
			pushToast('Add goals or habits before generating a review.', 'info');
			return;
		}
		generatingReview = true;
		reviewText = '';
		try {
			const goalLines = goals
				.map(
					(goal) =>
						`- ${goal.title} [${goal.status}] progress:${goal.progress}% target:${goal.target_date ?? 'none'}`
				)
				.join('\n');
			const habitLines = habits
				.map(
					(habit) =>
						`- ${habit.name} (${habit.frequency}) streak:${habit.current_streak} best:${habit.best_streak} today:${habit.checkins.includes(today) ? 'done' : 'pending'}`
				)
				.join('\n');

			const prompt = [
				'Create a concise weekly execution review with these sections:',
				'1. Goal momentum',
				'2. Habit consistency',
				'3. Risks and blockers',
				'4. Top 5 actions for next week',
				'',
				'Goals:',
				goalLines || '(none)',
				'',
				'Habits:',
				habitLines || '(none)'
			].join('\n');

			const response = await assistTransform({ text: prompt, mode: 'summarize' });
			reviewText = response.transformed_text;
			pushToast('Review generated.', 'success');
		} catch {
			pushToast('Review generation failed.', 'danger');
		} finally {
			generatingReview = false;
		}
	}
</script>

<div class="grid gap-6 lg:grid-cols-12">
	<section class="lg:col-span-5">
		<div class="grid grid-cols-2 gap-3">
			<div class="rounded-xl border border-slate-800 bg-slate-900/40 p-3">
				<p class="text-[10px] uppercase tracking-wide text-slate-500">Active goals</p>
				<p class="mt-1 text-xl font-semibold text-white">{activeGoalCount}</p>
			</div>
			<div class="rounded-xl border border-slate-800 bg-slate-900/40 p-3">
				<p class="text-[10px] uppercase tracking-wide text-slate-500">Avg progress</p>
				<p class="mt-1 text-xl font-semibold text-sky-300">{averageProgress}%</p>
			</div>
			<div class="rounded-xl border border-slate-800 bg-slate-900/40 p-3">
				<p class="text-[10px] uppercase tracking-wide text-slate-500">Habits done today</p>
				<p class="mt-1 text-xl font-semibold text-emerald-300">{completedHabitsToday}</p>
			</div>
			<div class="rounded-xl border border-slate-800 bg-slate-900/40 p-3">
				<p class="text-[10px] uppercase tracking-wide text-slate-500">Best streak</p>
				<p class="mt-1 text-xl font-semibold text-amber-300">{bestHabitStreak}</p>
			</div>
		</div>

		<div class="mt-4 rounded-2xl border border-slate-900 bg-slate-900/40 p-4">
			<h3 class="text-sm font-semibold text-white">Create Goal</h3>
			<div class="mt-3 space-y-2">
				<input
					class="w-full rounded-lg border border-slate-800 bg-slate-900 px-3 py-2 text-xs text-white"
					placeholder="Goal title"
					bind:value={newGoal.title}
				/>
				<textarea
					class="h-20 w-full rounded-lg border border-slate-800 bg-slate-900 px-3 py-2 text-xs text-white"
					placeholder="Goal description"
					bind:value={newGoal.description}
				></textarea>
				<div class="grid grid-cols-2 gap-2">
					<input
						type="date"
						class="rounded-lg border border-slate-800 bg-slate-900 px-3 py-2 text-xs text-white"
						bind:value={newGoal.target_date}
					/>
					<select
						class="rounded-lg border border-slate-800 bg-slate-900 px-3 py-2 text-xs text-white"
						bind:value={newGoal.status}
					>
						{#each goalStatusOptions as option (option)}
							<option value={option}>{option.replace('_', ' ')}</option>
						{/each}
					</select>
				</div>
				<div class="grid grid-cols-2 gap-2">
					<input
						type="number"
						min="1"
						max="5"
						class="rounded-lg border border-slate-800 bg-slate-900 px-3 py-2 text-xs text-white"
						bind:value={newGoal.priority}
					/>
					<input
						class="rounded-lg border border-slate-800 bg-slate-900 px-3 py-2 text-xs text-white"
						placeholder="tags e.g. health career"
						bind:value={newGoal.tags}
					/>
				</div>
				<button
					class="w-full rounded-lg bg-sky-500 px-3 py-2 text-xs font-semibold text-white hover:bg-sky-400 disabled:opacity-50"
					on:click={createGoal}
					disabled={savingGoal}
				>
					{savingGoal ? 'Creating...' : 'Create Goal'}
				</button>
			</div>
		</div>

		<div class="mt-4 rounded-2xl border border-slate-900 bg-slate-900/40 p-4">
			<div class="flex items-center gap-2">
				<input
					class="flex-1 rounded-lg border border-slate-800 bg-slate-900 px-3 py-2 text-xs text-white"
					placeholder="Search goals"
					bind:value={goalQuery}
					on:input={setGoalFilter}
				/>
				<select
					class="rounded-lg border border-slate-800 bg-slate-900 px-2 py-2 text-xs text-white"
					bind:value={goalStatusFilter}
					on:change={setGoalFilter}
				>
					<option value="all">all</option>
					{#each goalStatusOptions as option (option)}
						<option value={option}>{option}</option>
					{/each}
				</select>
			</div>
			<div class="mt-3 space-y-2">
				{#if loading}
					<p class="text-xs text-slate-500">Loading goals...</p>
				{:else if $filteredGoals.length === 0}
					<p class="text-xs text-slate-500">No goals match this filter.</p>
				{:else}
					{#each $filteredGoals as goal (goal.id)}
						<button
							class="w-full rounded-lg border px-3 py-2 text-left text-xs transition {selectedGoalId === goal.id
								? 'border-sky-500 bg-sky-500/10 text-sky-100'
								: 'border-slate-800 bg-slate-900/30 text-slate-200 hover:border-slate-700'}"
							on:click={() => openGoal(goal.id)}
						>
							<div class="flex items-center justify-between gap-2">
								<span class="truncate font-semibold">{goal.title}</span>
								<span class="text-[10px] text-slate-400">{goal.progress}%</span>
							</div>
							<div class="mt-1 text-[10px] text-slate-500">
								{goal.status}
								{#if goal.target_date}
									· due {goal.target_date}
								{/if}
							</div>
						</button>
					{/each}
				{/if}
			</div>
		</div>
	</section>

	<section class="lg:col-span-7">
		<div class="rounded-2xl border border-slate-900 bg-slate-900/40 p-4">
			<h3 class="text-sm font-semibold text-white">Goal Detail</h3>
			{#if selectedGoal}
				<div class="mt-3 space-y-2">
					<input
						class="w-full rounded-lg border border-slate-800 bg-slate-900 px-3 py-2 text-xs text-white"
						bind:value={goalEditor.title}
					/>
					<textarea
						class="h-24 w-full rounded-lg border border-slate-800 bg-slate-900 px-3 py-2 text-xs text-white"
						bind:value={goalEditor.description}
					></textarea>
					<div class="grid grid-cols-2 gap-2">
						<input
							type="date"
							class="rounded-lg border border-slate-800 bg-slate-900 px-3 py-2 text-xs text-white"
							bind:value={goalEditor.target_date}
						/>
						<select
							class="rounded-lg border border-slate-800 bg-slate-900 px-3 py-2 text-xs text-white"
							bind:value={goalEditor.status}
						>
							{#each goalStatusOptions as option (option)}
								<option value={option}>{option.replace('_', ' ')}</option>
							{/each}
						</select>
					</div>
					<div class="grid grid-cols-2 gap-2">
						<input
							type="number"
							min="1"
							max="5"
							class="rounded-lg border border-slate-800 bg-slate-900 px-3 py-2 text-xs text-white"
							bind:value={goalEditor.priority}
						/>
						<input
							type="number"
							min="0"
							max="100"
							class="rounded-lg border border-slate-800 bg-slate-900 px-3 py-2 text-xs text-white"
							bind:value={goalEditor.progress}
						/>
					</div>
					<input
						class="w-full rounded-lg border border-slate-800 bg-slate-900 px-3 py-2 text-xs text-white"
						placeholder="tags"
						bind:value={goalEditor.tags}
					/>
					<div class="flex flex-wrap gap-2 pt-1">
						<button
							class="rounded-lg bg-sky-500 px-3 py-2 text-xs font-semibold text-white hover:bg-sky-400 disabled:opacity-50"
							on:click={saveSelectedGoal}
							disabled={savingGoal}
						>
							Save goal
						</button>
						<button
							class="rounded-lg border border-emerald-500/30 px-3 py-2 text-xs text-emerald-300 hover:bg-emerald-500/10"
							on:click={markGoalComplete}
						>
							Mark complete
						</button>
						<button
							class="rounded-lg border border-red-500/30 px-3 py-2 text-xs text-red-300 hover:bg-red-500/10"
							on:click={removeSelectedGoal}
						>
							Delete
						</button>
					</div>
				</div>
			{:else}
				<p class="mt-2 text-xs text-slate-500">Select a goal to edit.</p>
			{/if}
		</div>

		<div class="mt-4 rounded-2xl border border-slate-900 bg-slate-900/40 p-4">
			<h3 class="text-sm font-semibold text-white">Habits</h3>
			<div class="mt-3 grid gap-2 md:grid-cols-2">
				<input
					class="rounded-lg border border-slate-800 bg-slate-900 px-3 py-2 text-xs text-white"
					placeholder="Habit name"
					bind:value={newHabit.name}
				/>
				<select
					class="rounded-lg border border-slate-800 bg-slate-900 px-3 py-2 text-xs text-white"
					bind:value={newHabit.frequency}
				>
					{#each habitFrequencyOptions as option (option)}
						<option value={option}>{option}</option>
					{/each}
				</select>
				<input
					class="rounded-lg border border-slate-800 bg-slate-900 px-3 py-2 text-xs text-white"
					placeholder="Description"
					bind:value={newHabit.description}
				/>
				<input
					type="number"
					min="1"
					max="31"
					class="rounded-lg border border-slate-800 bg-slate-900 px-3 py-2 text-xs text-white"
					bind:value={newHabit.target_per_period}
				/>
				<input
					class="md:col-span-2 rounded-lg border border-slate-800 bg-slate-900 px-3 py-2 text-xs text-white"
					placeholder="tags"
					bind:value={newHabit.tags}
				/>
				<button
					class="md:col-span-2 rounded-lg bg-emerald-500 px-3 py-2 text-xs font-semibold text-white hover:bg-emerald-400 disabled:opacity-50"
					on:click={createHabit}
					disabled={savingHabit}
				>
					{savingHabit ? 'Creating...' : 'Create Habit'}
				</button>
			</div>

			<div class="mt-4 space-y-2">
				{#if $habitsStore.length === 0}
					<p class="text-xs text-slate-500">No habits yet.</p>
				{:else}
					{#each $habitsStore as habit (habit.id)}
						<div class="rounded-lg border border-slate-800 bg-slate-900/40 p-3">
							<div class="flex items-center justify-between gap-2">
								<button
									class="min-w-0 text-left"
									on:click={() => openHabit(habit.id)}
								>
									<div class="truncate text-xs font-semibold text-white">{habit.name}</div>
									<div class="text-[10px] text-slate-500">
										{habit.frequency} · streak {habit.current_streak} · best {habit.best_streak}
									</div>
								</button>
								<button
									class="rounded-lg border px-2 py-1 text-[10px] {habit.checkins.includes(today)
										? 'border-emerald-500/40 bg-emerald-500/10 text-emerald-300'
										: 'border-slate-700 text-slate-300 hover:bg-slate-800'}"
									on:click={() => toggleToday(habit.id)}
								>
									{habit.checkins.includes(today) ? 'Done today' : 'Mark today'}
								</button>
							</div>
						</div>
					{/each}
				{/if}
			</div>
		</div>

		<div class="mt-4 rounded-2xl border border-slate-900 bg-slate-900/40 p-4">
			<h3 class="text-sm font-semibold text-white">Habit Detail</h3>
			{#if selectedHabit}
				<div class="mt-3 grid gap-2 md:grid-cols-2">
					<input
						class="rounded-lg border border-slate-800 bg-slate-900 px-3 py-2 text-xs text-white"
						bind:value={habitEditor.name}
					/>
					<select
						class="rounded-lg border border-slate-800 bg-slate-900 px-3 py-2 text-xs text-white"
						bind:value={habitEditor.frequency}
					>
						{#each habitFrequencyOptions as option (option)}
							<option value={option}>{option}</option>
						{/each}
					</select>
					<input
						class="md:col-span-2 rounded-lg border border-slate-800 bg-slate-900 px-3 py-2 text-xs text-white"
						bind:value={habitEditor.description}
					/>
					<input
						type="number"
						min="1"
						max="31"
						class="rounded-lg border border-slate-800 bg-slate-900 px-3 py-2 text-xs text-white"
						bind:value={habitEditor.target_per_period}
					/>
					<label class="flex items-center gap-2 rounded-lg border border-slate-800 bg-slate-900 px-3 py-2 text-xs text-slate-300">
						<input type="checkbox" bind:checked={habitEditor.enabled} />
						Enabled
					</label>
					<input
						class="md:col-span-2 rounded-lg border border-slate-800 bg-slate-900 px-3 py-2 text-xs text-white"
						placeholder="tags"
						bind:value={habitEditor.tags}
					/>
				</div>
				<div class="mt-3 flex flex-wrap gap-2">
					<button
						class="rounded-lg bg-sky-500 px-3 py-2 text-xs font-semibold text-white hover:bg-sky-400 disabled:opacity-50"
						on:click={saveSelectedHabit}
						disabled={savingHabit}
					>
						Save habit
					</button>
					<button
						class="rounded-lg border border-red-500/30 px-3 py-2 text-xs text-red-300 hover:bg-red-500/10"
						on:click={removeSelectedHabit}
					>
						Delete
					</button>
				</div>
			{:else}
				<p class="mt-2 text-xs text-slate-500">Select a habit to edit.</p>
			{/if}
		</div>

		<div class="mt-4 rounded-2xl border border-slate-900 bg-slate-900/40 p-4">
			<div class="flex items-center justify-between gap-2">
				<h3 class="text-sm font-semibold text-white">AI Weekly Review</h3>
				<button
					class="rounded-lg border border-violet-500/30 px-3 py-1.5 text-xs text-violet-300 hover:bg-violet-500/10 disabled:opacity-50"
					on:click={generateReview}
					disabled={generatingReview}
				>
					{generatingReview ? 'Generating...' : 'Generate'}
				</button>
			</div>
			{#if reviewText}
				<pre class="mt-3 whitespace-pre-wrap rounded-lg border border-slate-800 bg-slate-950 p-3 text-xs text-slate-200">{reviewText}</pre>
			{:else}
				<p class="mt-2 text-xs text-slate-500">
					Generate a concise momentum report with next actions based on your goals and habits.
				</p>
			{/if}
		</div>
	</section>
</div>
