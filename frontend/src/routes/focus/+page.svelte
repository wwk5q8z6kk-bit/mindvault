<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { goto } from '$app/navigation';
	import { tasksStore, loadTasks, completeTaskOptimistic, updateTaskOptimistic } from '$lib/stores/tasks';
	import { prioritizeTasks, type PrioritizedTaskItem } from '$lib/api/ai';
	import { pushToast } from '$lib/stores/toast';
	import { selectedTaskId } from '$lib/stores/ui';
	import TimeTracker from '$lib/components/TimeTracker.svelte';
	import type { TaskRecord } from '$lib/db';

	// Focus session state
	let focusItems: PrioritizedTaskItem[] = [];
	let currentIndex = 0;
	let loading = true;
	let sessionActive = false;
	let sessionStartTime: Date | null = null;

	// Timer state
	let timerMode: 'pomodoro' | 'stopwatch' = 'pomodoro';
	let timerSeconds = 25 * 60; // 25 min default
	let timerRunning = false;
	let timerInterval: ReturnType<typeof setInterval> | null = null;
	let pomodoroCount = 0;
	let totalFocusTime = 0;

	// Pomodoro presets
	const pomodoroPresets = [
		{ label: '25 min', seconds: 25 * 60 },
		{ label: '50 min', seconds: 50 * 60 },
		{ label: '15 min', seconds: 15 * 60 }
	];

	// Session history
	let completedInSession: string[] = [];

	onMount(async () => {
		await loadFocusList();
	});

	onDestroy(() => {
		if (timerInterval) clearInterval(timerInterval);
	});

	async function loadFocusList() {
		loading = true;
		try {
			await loadTasks();
			const response = await prioritizeTasks({
				limit: 10,
				include_done: false,
				statuses: ['inbox', 'planned', 'in_progress']
			});
			focusItems = response.items;
		} catch {
			// Fallback to manual priority sort
			const activeTasks = $tasksStore.filter((t) => t.status !== 'done');
			focusItems = activeTasks
				.sort((a, b) => (a.priority ?? 4) - (b.priority ?? 4))
				.slice(0, 10)
				.map((task, i) => ({
					task: {
						id: task.id,
						title: task.title,
						description: task.description ?? '',
						status: task.status,
						priority: task.priority ?? 4,
						due_at: task.due_at ?? null,
						labels: task.labels ?? [],
						dependencies: task.dependencies ?? [],
						metadata: task.metadata ?? {},
						created_at: task.created_at,
						updated_at: task.updated_at
					},
					score: 1 - i * 0.1,
					rank: i + 1,
					reason: null
				}));
		} finally {
			loading = false;
		}
	}

	function startSession() {
		sessionActive = true;
		sessionStartTime = new Date();
		currentIndex = 0;
		completedInSession = [];
	}

	function endSession() {
		sessionActive = false;
		stopTimer();
		if (timerInterval) clearInterval(timerInterval);
		pushToast(`Session ended. ${completedInSession.length} tasks completed.`, 'success');
	}

	function startTimer() {
		if (timerRunning) return;
		timerRunning = true;
		timerInterval = setInterval(() => {
			if (timerMode === 'pomodoro') {
				timerSeconds--;
				totalFocusTime++;
				if (timerSeconds <= 0) {
					pomodoroCount++;
					pushToast('Pomodoro complete! Take a break.', 'success');
					stopTimer();
					timerSeconds = 5 * 60; // 5 min break
				}
			} else {
				timerSeconds++;
				totalFocusTime++;
			}
		}, 1000);
	}

	function stopTimer() {
		timerRunning = false;
		if (timerInterval) {
			clearInterval(timerInterval);
			timerInterval = null;
		}
	}

	function resetTimer() {
		stopTimer();
		timerSeconds = timerMode === 'pomodoro' ? 25 * 60 : 0;
	}

	function setPomodoroDuration(seconds: number) {
		timerSeconds = seconds;
		timerMode = 'pomodoro';
	}

	async function completeCurrentTask() {
		const current = focusItems[currentIndex];
		if (!current) return;

		try {
			await completeTaskOptimistic(current.task.id);
			completedInSession.push(current.task.id);
			pushToast(`Completed: ${current.task.title}`, 'success');

			// Move to next task
			if (currentIndex < focusItems.length - 1) {
				currentIndex++;
			} else {
				pushToast('All focus tasks completed!', 'success');
				endSession();
			}
		} catch {
			pushToast('Failed to complete task', 'danger');
		}
	}

	function skipTask() {
		if (currentIndex < focusItems.length - 1) {
			currentIndex++;
		}
	}

	function previousTask() {
		if (currentIndex > 0) {
			currentIndex--;
		}
	}

	function openTaskDetail(taskId: string) {
		selectedTaskId.set(taskId);
		goto(`/tasks?task=${taskId}`);
	}

	function formatTime(seconds: number): string {
		const mins = Math.floor(Math.abs(seconds) / 60);
		const secs = Math.abs(seconds) % 60;
		return `${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
	}

	function formatDuration(seconds: number): string {
		const hours = Math.floor(seconds / 3600);
		const mins = Math.floor((seconds % 3600) / 60);
		if (hours > 0) return `${hours}h ${mins}m`;
		return `${mins}m`;
	}

	// Per-task time tracking
	let taskTimeSpent: Map<string, number> = new Map();

	async function handleTimeUpdate(event: CustomEvent<{ taskId: string; timeSpentMin: number }>) {
		const { taskId, timeSpentMin } = event.detail;
		taskTimeSpent = new Map(taskTimeSpent).set(taskId, timeSpentMin);
		try {
			await updateTaskOptimistic(taskId, {
				metadata: { time_spent_min: timeSpentMin }
			});
		} catch {
			// Silently fail — time is tracked locally even if persist fails
		}
	}

	$: currentTask = focusItems[currentIndex] ?? null;
	$: currentTaskTimeSpent = currentTask ? (taskTimeSpent.get(currentTask.task.id) ?? (currentTask.task.metadata?.time_spent_min as number ?? 0)) : 0;
	$: currentTaskEstimate = currentTask ? (currentTask.task.metadata?.estimate_min as number ?? null) : null;
	$: progress = focusItems.length > 0 ? ((currentIndex + completedInSession.length) / focusItems.length) * 100 : 0;
</script>

<div class="mx-auto max-w-4xl space-y-6">
	<div class="flex items-center justify-between">
		<div>
			<h1 class="text-lg font-semibold text-white">Focus Mode</h1>
			<p class="text-xs text-slate-400">
				{#if sessionActive}
					Session in progress • {completedInSession.length} completed
				{:else}
					Deep work with AI-prioritized tasks
				{/if}
			</p>
		</div>
		<div class="flex gap-2">
			{#if !sessionActive}
				<button
					class="rounded-lg border border-slate-700 px-3 py-2 text-xs text-slate-300 hover:bg-slate-800"
					on:click={loadFocusList}
					disabled={loading}
				>
					Refresh
				</button>
				<button
					class="rounded-lg bg-sky-500 px-4 py-2 text-xs font-semibold text-white hover:bg-sky-400"
					on:click={startSession}
					disabled={loading || focusItems.length === 0}
				>
					Start Focus Session
				</button>
			{:else}
				<button
					class="rounded-lg border border-red-500/30 px-3 py-2 text-xs text-red-300 hover:bg-red-500/10"
					on:click={endSession}
				>
					End Session
				</button>
			{/if}
		</div>
	</div>

	{#if loading}
		<div class="flex items-center justify-center py-12">
			<div class="h-8 w-8 animate-spin rounded-full border-2 border-slate-600 border-t-sky-500"></div>
		</div>
	{:else if focusItems.length === 0}
		<div class="rounded-2xl border border-dashed border-slate-800 p-12 text-center">
			<h3 class="text-sm font-medium text-white">No tasks to focus on</h3>
			<p class="mt-1 text-xs text-slate-400">Add some tasks to get started</p>
			<button
				class="mt-4 rounded-lg bg-sky-500 px-4 py-2 text-xs font-semibold text-white hover:bg-sky-400"
				on:click={() => goto('/tasks')}
			>
				Go to Tasks
			</button>
		</div>
	{:else if sessionActive && currentTask}
		<!-- Active Focus Session -->
		<div class="grid gap-6 lg:grid-cols-2">
			<!-- Current Task -->
			<div class="rounded-2xl border border-slate-700 bg-slate-900/80 p-6">
				<div class="flex items-center justify-between">
					<span class="text-xs uppercase tracking-wide text-slate-500">
						Task {currentIndex + 1} of {focusItems.length}
					</span>
					<div class="flex items-center gap-2">
						<span class="rounded bg-sky-500/20 px-2 py-0.5 text-[10px] font-medium text-sky-300">
							P{currentTask.task.priority}
						</span>
						<span class="rounded bg-slate-800 px-2 py-0.5 text-[10px] text-slate-400">
							Score: {(currentTask.score * 100).toFixed(0)}%
						</span>
					</div>
				</div>

				<h2 class="mt-4 text-xl font-bold text-white">{currentTask.task.title}</h2>

				{#if currentTask.reason}
					<p class="mt-2 text-xs italic text-slate-400">"{currentTask.reason}"</p>
				{/if}

				{#if currentTask.task.description}
					<p class="mt-3 text-sm text-slate-300">{currentTask.task.description}</p>
				{/if}

				{#if currentTask.task.due_at}
					<div class="mt-3 text-xs text-slate-500">
						Due: {new Date(currentTask.task.due_at).toLocaleString()}
					</div>
				{/if}

				<div class="mt-6 flex gap-2">
					<button
						class="flex-1 rounded-lg bg-emerald-500 py-3 text-sm font-semibold text-white hover:bg-emerald-400"
						on:click={completeCurrentTask}
					>
						Complete
					</button>
					<button
						class="rounded-lg border border-slate-700 px-4 py-3 text-sm text-slate-300 hover:bg-slate-800"
						on:click={skipTask}
					>
						Skip
					</button>
					<button
						class="rounded-lg border border-slate-700 px-4 py-3 text-sm text-slate-300 hover:bg-slate-800"
						on:click={() => openTaskDetail(currentTask.task.id)}
					>
						Details
					</button>
				</div>

				<!-- Navigation -->
				<div class="mt-4 flex justify-between">
					<button
						class="text-xs text-slate-500 hover:text-white disabled:opacity-50"
						on:click={previousTask}
						disabled={currentIndex === 0}
					>
						Previous
					</button>
					<button
						class="text-xs text-slate-500 hover:text-white disabled:opacity-50"
						on:click={skipTask}
						disabled={currentIndex >= focusItems.length - 1}
					>
						Next
					</button>
				</div>
			</div>

			<!-- Task-level Time Tracking -->
			<div class="lg:col-span-2">
				<TimeTracker
					taskId={currentTask.task.id}
					estimateMin={currentTaskEstimate}
					timeSpentMin={currentTaskTimeSpent}
					on:timeUpdate={handleTimeUpdate}
				/>
			</div>

			<!-- Timer -->
			<div class="rounded-2xl border border-slate-700 bg-slate-900/80 p-6">
				<div class="flex items-center justify-between">
					<span class="text-xs uppercase tracking-wide text-slate-500">Timer</span>
					<div class="flex gap-1">
						<button
							class={`rounded px-2 py-1 text-[10px] ${timerMode === 'pomodoro' ? 'bg-sky-500/20 text-sky-300' : 'text-slate-500'}`}
							on:click={() => { timerMode = 'pomodoro'; resetTimer(); }}
						>
							Pomodoro
						</button>
						<button
							class={`rounded px-2 py-1 text-[10px] ${timerMode === 'stopwatch' ? 'bg-sky-500/20 text-sky-300' : 'text-slate-500'}`}
							on:click={() => { timerMode = 'stopwatch'; resetTimer(); }}
						>
							Stopwatch
						</button>
					</div>
				</div>

				<div class="mt-6 text-center">
					<div class="text-6xl font-bold tabular-nums text-white">
						{formatTime(timerSeconds)}
					</div>

					{#if timerMode === 'pomodoro'}
						<div class="mt-4 flex justify-center gap-2">
							{#each pomodoroPresets as preset}
								<button
									class="rounded-lg border border-slate-700 px-3 py-1 text-xs text-slate-400 hover:bg-slate-800"
									on:click={() => setPomodoroDuration(preset.seconds)}
								>
									{preset.label}
								</button>
							{/each}
						</div>
					{/if}

					<div class="mt-6 flex justify-center gap-2">
						{#if timerRunning}
							<button
								class="rounded-lg bg-amber-500 px-6 py-2 text-sm font-semibold text-white hover:bg-amber-400"
								on:click={stopTimer}
							>
								Pause
							</button>
						{:else}
							<button
								class="rounded-lg bg-emerald-500 px-6 py-2 text-sm font-semibold text-white hover:bg-emerald-400"
								on:click={startTimer}
							>
								Start
							</button>
						{/if}
						<button
							class="rounded-lg border border-slate-700 px-4 py-2 text-sm text-slate-300 hover:bg-slate-800"
							on:click={resetTimer}
						>
							Reset
						</button>
					</div>
				</div>

				<div class="mt-6 grid grid-cols-2 gap-4 border-t border-slate-800 pt-4">
					<div class="text-center">
						<div class="text-2xl font-bold text-white">{pomodoroCount}</div>
						<div class="text-[10px] text-slate-500">Pomodoros</div>
					</div>
					<div class="text-center">
						<div class="text-2xl font-bold text-white">{formatDuration(totalFocusTime)}</div>
						<div class="text-[10px] text-slate-500">Total Focus</div>
					</div>
				</div>
			</div>
		</div>

		<!-- Progress -->
		<div class="rounded-xl border border-slate-800 bg-slate-900/60 p-4">
			<div class="flex items-center justify-between text-xs text-slate-400">
				<span>Session Progress</span>
				<span>{completedInSession.length} / {focusItems.length} tasks</span>
			</div>
			<div class="mt-2 h-2 overflow-hidden rounded-full bg-slate-800">
				<div
					class="h-full bg-gradient-to-r from-sky-500 to-emerald-500 transition-all duration-300"
					style="width: {progress}%"
				></div>
			</div>
		</div>

		<!-- Up Next -->
		{#if focusItems.length > currentIndex + 1}
			<div class="rounded-xl border border-slate-800 bg-slate-900/40 p-4">
				<h3 class="text-xs font-semibold uppercase tracking-wide text-slate-500">Up Next</h3>
				<div class="mt-3 space-y-2">
					{#each focusItems.slice(currentIndex + 1, currentIndex + 4) as item, i}
						<div class="flex items-center gap-3 text-xs">
							<span class="flex h-5 w-5 items-center justify-center rounded-full bg-slate-800 text-[10px] text-slate-500">
								{currentIndex + i + 2}
							</span>
							<span class="text-slate-300">{item.task.title}</span>
							<span class="ml-auto text-slate-600">P{item.task.priority}</span>
						</div>
					{/each}
				</div>
			</div>
		{/if}
	{:else}
		<!-- Pre-session: Show task list -->
		<div class="rounded-2xl border border-slate-800 bg-slate-900/60 p-6">
			<h3 class="text-sm font-semibold text-white">Focus Queue</h3>
			<p class="text-xs text-slate-400">AI-ranked tasks for your focus session</p>

			<div class="mt-4 space-y-2">
				{#each focusItems as item, i (item.task.id)}
					<div class="flex items-center gap-3 rounded-lg border border-slate-800 bg-slate-900/40 p-3">
						<div class="flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-slate-800 text-sm font-bold text-slate-300">
							{i + 1}
						</div>
						<div class="min-w-0 flex-1">
							<div class="flex items-center gap-2">
								<span class="truncate text-sm font-medium text-white">{item.task.title}</span>
								<span class="shrink-0 rounded bg-sky-500/20 px-1.5 py-0.5 text-[9px] text-sky-300">
									{(item.score * 100).toFixed(0)}%
								</span>
							</div>
							{#if item.reason}
								<p class="mt-0.5 truncate text-[11px] text-slate-400">{item.reason}</p>
							{/if}
						</div>
						<span class="text-[10px] text-slate-500">P{item.task.priority}</span>
					</div>
				{/each}
			</div>
		</div>
	{/if}
</div>
