<script lang="ts">
	import MvEditor from '$lib/components/MvEditor.svelte';
	import { pushToast } from '$lib/stores/toast';
	import {
		currentDailyNote,
		recentDailyNotes,
		loadDailyNote,
		loadRecentDailyNotes,
		saveDailyNote
	} from '$lib/stores/daily-notes';
	import { onMount } from 'svelte';

	let editor: MvEditor;
	let currentDate = todayStr();
	let saving = false;
	let saveTimer: ReturnType<typeof setTimeout> | null = null;

	// Habit tracking
	type Habit = { id: string; name: string; enabled: boolean };
	type HabitCompletions = Record<string, string[]>; // date -> habit ids

	let habits: Habit[] = JSON.parse(localStorage.getItem('mv_habits') ?? '[]');
	let habitCompletions: HabitCompletions = JSON.parse(localStorage.getItem('mv_habit_completions') ?? '{}');
	let showHabitSettings = false;
	let newHabitName = '';

	function saveHabits() {
		localStorage.setItem('mv_habits', JSON.stringify(habits));
	}

	function saveCompletions() {
		localStorage.setItem('mv_habit_completions', JSON.stringify(habitCompletions));
	}

	function addHabit() {
		const name = newHabitName.trim();
		if (!name) return;
		habits = [...habits, { id: Date.now().toString(36), name, enabled: true }];
		newHabitName = '';
		saveHabits();
	}

	function removeHabit(id: string) {
		habits = habits.filter((h) => h.id !== id);
		saveHabits();
	}

	function toggleHabitEnabled(id: string) {
		habits = habits.map((h) => (h.id === id ? { ...h, enabled: !h.enabled } : h));
		saveHabits();
	}

	function toggleHabitCompletion(habitId: string) {
		const completed = habitCompletions[currentDate] ?? [];
		if (completed.includes(habitId)) {
			habitCompletions[currentDate] = completed.filter((id) => id !== habitId);
		} else {
			habitCompletions[currentDate] = [...completed, habitId];
		}
		habitCompletions = { ...habitCompletions };
		saveCompletions();
	}

	function isHabitDone(habitId: string): boolean {
		return (habitCompletions[currentDate] ?? []).includes(habitId);
	}

	function getStreak(habitId: string): number {
		let streak = 0;
		const d = new Date(currentDate + 'T00:00:00');
		while (true) {
			const dateStr = d.toISOString().slice(0, 10);
			if ((habitCompletions[dateStr] ?? []).includes(habitId)) {
				streak++;
				d.setDate(d.getDate() - 1);
			} else {
				break;
			}
		}
		return streak;
	}

	$: enabledHabits = habits.filter((h) => h.enabled);
	$: todayCompleted = (habitCompletions[currentDate] ?? []).length;
	$: todayTotal = enabledHabits.length;

	function todayStr(): string {
		return new Date().toISOString().slice(0, 10);
	}

	function formatDisplayDate(dateStr: string): string {
		const d = new Date(dateStr + 'T00:00:00');
		return d.toLocaleDateString(undefined, { weekday: 'long', year: 'numeric', month: 'long', day: 'numeric' });
	}

	function prevDay() {
		const d = new Date(currentDate + 'T00:00:00');
		d.setDate(d.getDate() - 1);
		currentDate = d.toISOString().slice(0, 10);
		navigateToDate(currentDate);
	}

	function nextDay() {
		const d = new Date(currentDate + 'T00:00:00');
		d.setDate(d.getDate() + 1);
		currentDate = d.toISOString().slice(0, 10);
		navigateToDate(currentDate);
	}

	function goToday() {
		currentDate = todayStr();
		navigateToDate(currentDate);
	}

	async function navigateToDate(date: string) {
		try {
			await loadDailyNote(date);
			if (editor && $currentDailyNote) {
				editor.setMarkdown($currentDailyNote.markdown);
			}
		} catch {
			pushToast('Failed to load daily note', 'danger');
		}
	}

	function onEditorChange(e: CustomEvent<{ markdown: string }>) {
		if (!$currentDailyNote) return;
		if (saveTimer) clearTimeout(saveTimer);
		saveTimer = setTimeout(() => {
			void autoSave(e.detail.markdown);
		}, 1500);
	}

	async function autoSave(content: string) {
		if (!$currentDailyNote) return;
		saving = true;
		try {
			await saveDailyNote($currentDailyNote.id, content);
		} catch {
			pushToast('Auto-save failed', 'danger');
		} finally {
			saving = false;
		}
	}

	function selectRecentNote(note: { id: string; created_at: string }) {
		const dateStr = note.created_at.slice(0, 10);
		currentDate = dateStr;
		navigateToDate(dateStr);
	}

	onMount(async () => {
		try {
			await loadDailyNote();
			await loadRecentDailyNotes();
			if (editor && $currentDailyNote) {
				editor.setMarkdown($currentDailyNote.markdown);
			}
		} catch {
			pushToast('Failed to load daily note', 'danger');
		}
	});
</script>

<div class="flex gap-6">
	<!-- Main editor area -->
	<div class="flex-1">
		<div class="rounded-2xl border border-slate-900 bg-slate-900/40 p-6">
			<!-- Header -->
			<div class="mb-4 flex items-center justify-between">
				<div>
					<h2 class="text-lg font-semibold text-white">Daily Note</h2>
					<p class="text-xs text-slate-400">{formatDisplayDate(currentDate)}</p>
				</div>
				<div class="flex items-center gap-2">
					<button
						class="rounded-lg border border-slate-700 px-2 py-1 text-xs text-slate-300 hover:bg-slate-800"
						on:click={prevDay}
					>
						&larr;
					</button>
					<button
						class="rounded-lg border border-slate-700 px-3 py-1 text-xs text-slate-300 hover:bg-slate-800"
						on:click={goToday}
					>
						Today
					</button>
					<button
						class="rounded-lg border border-slate-700 px-2 py-1 text-xs text-slate-300 hover:bg-slate-800"
						on:click={nextDay}
					>
						&rarr;
					</button>
					<input
						type="date"
						class="ml-2 rounded-lg border border-slate-700 bg-slate-800 px-2 py-1 text-xs text-slate-300"
						bind:value={currentDate}
						on:change={() => navigateToDate(currentDate)}
					/>
					{#if saving}
						<span class="text-[10px] text-amber-300">Saving...</span>
					{/if}
				</div>
			</div>

			<!-- Editor -->
			<MvEditor
				bind:this={editor}
				value={$currentDailyNote?.markdown ?? ''}
				placeholder="What's on your mind today?"
				on:change={onEditorChange}
			/>
		</div>

		<!-- Habit Tracking -->
		{#if enabledHabits.length > 0 || showHabitSettings}
			<div class="mt-4 rounded-2xl border border-slate-900 bg-slate-900/40 p-4">
				<div class="flex items-center justify-between">
					<div class="flex items-center gap-2">
						<h3 class="text-sm font-semibold text-white">Habits</h3>
						{#if todayTotal > 0}
							<span class="rounded-full bg-emerald-500/20 px-2 py-0.5 text-[10px] text-emerald-300">
								{todayCompleted}/{todayTotal}
							</span>
						{/if}
					</div>
					<button
						class="text-[10px] text-slate-500 hover:text-white"
						on:click={() => { showHabitSettings = !showHabitSettings; }}
					>
						{showHabitSettings ? 'Done' : 'Edit'}
					</button>
				</div>

				{#if showHabitSettings}
					<div class="mt-3 flex flex-col gap-2">
						{#each habits as habit (habit.id)}
							<div class="flex items-center gap-2 rounded-lg border border-slate-800 px-3 py-2">
								<button
									class="text-[10px] {habit.enabled ? 'text-emerald-400' : 'text-slate-600'}"
									on:click={() => toggleHabitEnabled(habit.id)}
								>
									{habit.enabled ? 'ON' : 'OFF'}
								</button>
								<span class="flex-1 text-xs text-white">{habit.name}</span>
								<button
									class="text-[10px] text-red-400 hover:text-red-300"
									on:click={() => removeHabit(habit.id)}
								>
									Remove
								</button>
							</div>
						{/each}
						<div class="flex gap-2">
							<input
								class="flex-1 rounded-lg border border-slate-700 bg-slate-800 px-3 py-1.5 text-xs text-white placeholder-slate-500"
								placeholder="New habit name..."
								bind:value={newHabitName}
								on:keydown={(e) => e.key === 'Enter' && addHabit()}
							/>
							<button
								class="rounded-lg bg-sky-500 px-3 py-1.5 text-xs font-semibold text-white hover:bg-sky-400"
								on:click={addHabit}
							>
								Add
							</button>
						</div>
					</div>
				{:else}
					<div class="mt-3 flex flex-col gap-1.5">
						{#each enabledHabits as habit (habit.id)}
							{@const done = isHabitDone(habit.id)}
							{@const streak = getStreak(habit.id)}
							<button
								class="flex items-center gap-3 rounded-lg border px-3 py-2 text-left transition {done
									? 'border-emerald-500/30 bg-emerald-500/5'
									: 'border-slate-800 hover:border-slate-700'}"
								on:click={() => toggleHabitCompletion(habit.id)}
							>
								<div class="flex h-5 w-5 shrink-0 items-center justify-center rounded border {done
									? 'border-emerald-500 bg-emerald-500 text-white'
									: 'border-slate-600'}">
									{#if done}
										<svg class="h-3 w-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
											<path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M5 13l4 4L19 7" />
										</svg>
									{/if}
								</div>
								<span class="flex-1 text-xs {done ? 'text-emerald-200 line-through' : 'text-white'}">{habit.name}</span>
								{#if streak > 1}
									<span class="rounded-full bg-amber-500/20 px-1.5 py-0.5 text-[9px] text-amber-300">
										{streak} day streak
									</span>
								{/if}
							</button>
						{/each}
					</div>
				{/if}
			</div>
		{:else}
			<div class="mt-4 rounded-2xl border border-dashed border-slate-800 p-4 text-center">
				<p class="text-xs text-slate-400">No habits configured.</p>
				<button
					class="mt-2 rounded-lg border border-slate-700 px-3 py-1.5 text-xs text-slate-300 hover:bg-slate-800"
					on:click={() => { showHabitSettings = true; }}
				>
					Add habits
				</button>
			</div>
		{/if}
	</div>

	<!-- Sidebar: Recent daily notes -->
	<aside class="hidden w-56 flex-col gap-2 lg:flex">
		<h3 class="text-xs font-semibold uppercase tracking-wide text-slate-500">Recent</h3>
		<div class="flex flex-col gap-1">
			{#each $recentDailyNotes as note (note.id)}
				{@const noteDate = note.created_at.slice(0, 10)}
				<button
					class={`rounded-lg px-3 py-2 text-left text-xs transition ${
						noteDate === currentDate
							? 'bg-sky-500/20 text-sky-200'
							: 'text-slate-400 hover:bg-slate-800 hover:text-white'
					}`}
					on:click={() => selectRecentNote(note)}
				>
					<div class="font-medium">{noteDate}</div>
					<div class="truncate text-[10px] text-slate-500">
						{note.title || note.markdown.slice(0, 50) || 'Empty'}
					</div>
				</button>
			{/each}
			{#if $recentDailyNotes.length === 0}
				<p class="text-[10px] text-slate-500">No recent daily notes</p>
			{/if}
		</div>
	</aside>
</div>
