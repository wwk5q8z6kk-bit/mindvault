<script lang="ts">
	import { onMount } from 'svelte';
	import { get } from 'svelte/store';
	import { goto } from '$app/navigation';
	import { pushToast } from '$lib/stores/toast';
	import { activeNamespace } from '$lib/stores/namespace';
	import { prioritizeTasks, type PrioritizedTaskItem } from '$lib/api/ai';
	import { suggestTimeBlocks, type TimeBlockSuggestion } from '$lib/api/time-blocks';
	import { formatCalendarDate, getCalendarItems, type CalendarItem } from '$lib/api/calendar';
	import { completeTaskOptimistic, updateTaskOptimistic, tasksStore, loadTasks } from '$lib/stores/tasks';

	let loading = true;
	let prioritizedTasks: PrioritizedTaskItem[] = [];
	let timeBlockSuggestions: TimeBlockSuggestion[] = [];
	let calendarItems: CalendarItem[] = [];
	let error: string | null = null;

	// Settings
	let workdayStartHour = 9;
	let workdayEndHour = 18;
	let focusBlockMinutes = 45;
	let maxSuggestions = 6;

	// Date
	let planDate = new Date();
	$: dateStr = formatCalendarDate(planDate);
	$: isToday = dateStr === formatCalendarDate(new Date());

	// Computed
	$: totalFocusMinutes = timeBlockSuggestions.reduce((sum, s) => sum + s.durationMinutes, 0);
	$: availableHours = workdayEndHour - workdayStartHour;
	$: busyMinutes = calendarItems
		.filter((item) => item.kind === 'event')
		.reduce((sum, item) => {
			const start = new Date(item.start).getTime();
			const end = item.end ? new Date(item.end).getTime() : start + 30 * 60 * 1000;
			return sum + Math.round((end - start) / 60000);
		}, 0);

	onMount(async () => {
		await loadDayPlan();
	});

	async function loadDayPlan() {
		loading = true;
		error = null;
		try {
			const namespace = get(activeNamespace);

			// Load all data in parallel
			const [prioritized, timeBlocks, calendar] = await Promise.all([
				prioritizeTasks({
					namespace: namespace ?? undefined,
					limit: 10,
					include_done: false,
					statuses: ['inbox', 'planned', 'in_progress']
				}),
				suggestTimeBlocks({
					date: dateStr,
					namespace,
					limit: maxSuggestions,
					workdayStartHour,
					workdayEndHour,
					defaultBlockMinutes: focusBlockMinutes
				}),
				getCalendarItems({
					date: dateStr,
					view: 'day',
					include_tasks: true,
					namespace: namespace ?? undefined
				})
			]);

			prioritizedTasks = prioritized.items;
			timeBlockSuggestions = timeBlocks;
			calendarItems = calendar.items;
		} catch {
			error = 'Failed to load daily plan';
		} finally {
			loading = false;
		}
	}

	function formatTime(isoString: string): string {
		return new Date(isoString).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
	}

	function formatMinutes(min: number): string {
		if (min < 60) return `${min}m`;
		const h = Math.floor(min / 60);
		const m = min % 60;
		return m > 0 ? `${h}h ${m}m` : `${h}h`;
	}

	function prevDay() {
		const d = new Date(planDate);
		d.setDate(d.getDate() - 1);
		planDate = d;
		loadDayPlan();
	}

	function nextDay() {
		const d = new Date(planDate);
		d.setDate(d.getDate() + 1);
		planDate = d;
		loadDayPlan();
	}

	function goToToday() {
		planDate = new Date();
		loadDayPlan();
	}

	async function startTask(taskId: string) {
		try {
			await updateTaskOptimistic(taskId, { status: 'in_progress' });
			pushToast('Task started', 'success');
			goto(`/focus?task=${taskId}`);
		} catch {
			pushToast('Failed to start task', 'danger');
		}
	}

	async function completeTask(taskId: string) {
		try {
			await completeTaskOptimistic(taskId);
			pushToast('Task completed', 'success');
			// Refresh plan
			await loadDayPlan();
		} catch {
			pushToast('Failed to complete task', 'danger');
		}
	}

	function getPriorityColor(priority: number): string {
		if (priority <= 1) return 'text-red-400';
		if (priority <= 2) return 'text-orange-400';
		if (priority <= 3) return 'text-amber-400';
		return 'text-slate-400';
	}

	function getScoreColor(score: number): string {
		if (score >= 0.8) return 'bg-emerald-500';
		if (score >= 0.6) return 'bg-sky-500';
		if (score >= 0.4) return 'bg-amber-500';
		return 'bg-slate-500';
	}
</script>

<div class="mx-auto max-w-4xl">
	<!-- Header -->
	<div class="flex items-center justify-between">
		<div>
			<h2 class="text-lg font-semibold text-white">Daily Plan</h2>
			<p class="text-xs text-slate-400">AI-powered daily planning and focus suggestions</p>
		</div>
		<div class="flex items-center gap-2">
			<button
				class="rounded-lg border border-slate-700 px-2 py-1 text-xs text-slate-300 hover:bg-slate-800"
				on:click={prevDay}
			>
				&larr;
			</button>
			{#if !isToday}
				<button
					class="rounded-lg border border-sky-500/30 px-2 py-1 text-xs text-sky-300 hover:bg-sky-500/10"
					on:click={goToToday}
				>
					Today
				</button>
			{/if}
			<span class="min-w-[140px] text-center text-sm font-medium text-white">
				{planDate.toLocaleDateString(undefined, { weekday: 'short', month: 'short', day: 'numeric' })}
			</span>
			<button
				class="rounded-lg border border-slate-700 px-2 py-1 text-xs text-slate-300 hover:bg-slate-800"
				on:click={nextDay}
			>
				&rarr;
			</button>
		</div>
	</div>

	{#if loading}
		<div class="mt-8 flex items-center justify-center">
			<div class="flex flex-col items-center gap-3">
				<div class="h-8 w-8 animate-spin rounded-full border-2 border-sky-500 border-t-transparent"></div>
				<p class="text-sm text-slate-400">Generating your daily plan...</p>
			</div>
		</div>
	{:else if error}
		<div class="mt-8 rounded-xl border border-red-500/30 bg-red-500/10 p-6 text-center">
			<p class="text-sm text-red-300">{error}</p>
			<button
				class="mt-3 rounded-lg bg-red-500/20 px-4 py-2 text-xs text-red-200 hover:bg-red-500/30"
				on:click={loadDayPlan}
			>
				Retry
			</button>
		</div>
	{:else}
		<!-- Day Overview -->
		<div class="mt-6 grid gap-4 sm:grid-cols-3">
			<div class="rounded-xl border border-slate-800 bg-slate-900/40 p-4">
				<div class="text-[10px] uppercase tracking-wider text-slate-500">Available Time</div>
				<div class="mt-1 text-2xl font-bold text-white">{availableHours}h</div>
				<div class="mt-1 text-[10px] text-slate-400">
					{workdayStartHour}:00 - {workdayEndHour}:00
				</div>
			</div>
			<div class="rounded-xl border border-slate-800 bg-slate-900/40 p-4">
				<div class="text-[10px] uppercase tracking-wider text-slate-500">Scheduled</div>
				<div class="mt-1 text-2xl font-bold text-amber-400">{formatMinutes(busyMinutes)}</div>
				<div class="mt-1 text-[10px] text-slate-400">
					{calendarItems.filter((i) => i.kind === 'event').length} events
				</div>
			</div>
			<div class="rounded-xl border border-slate-800 bg-slate-900/40 p-4">
				<div class="text-[10px] uppercase tracking-wider text-slate-500">Focus Time</div>
				<div class="mt-1 text-2xl font-bold text-emerald-400">{formatMinutes(totalFocusMinutes)}</div>
				<div class="mt-1 text-[10px] text-slate-400">
					{timeBlockSuggestions.length} blocks suggested
				</div>
			</div>
		</div>

		<div class="mt-6 grid gap-6 lg:grid-cols-2">
			<!-- Priority Tasks -->
			<div class="rounded-xl border border-slate-800 bg-slate-900/40 p-5">
				<div class="flex items-center justify-between">
					<h3 class="text-sm font-semibold text-white">Top Priorities</h3>
					<span class="text-[10px] text-slate-500">AI-ranked by urgency & importance</span>
				</div>

				{#if prioritizedTasks.length === 0}
					<div class="mt-4 rounded-lg border border-dashed border-slate-700 p-4 text-center">
						<p class="text-xs text-slate-400">No tasks to prioritize</p>
						<a href="/tasks" class="mt-2 inline-block text-xs text-sky-400 hover:text-sky-300">
							Add tasks
						</a>
					</div>
				{:else}
					<div class="mt-4 flex flex-col gap-2">
						{#each prioritizedTasks.slice(0, 5) as item, idx (item.task.id)}
							<div class="group flex items-start gap-3 rounded-lg border border-slate-800/60 bg-slate-950/30 p-3 transition hover:border-slate-700">
								<div class="flex h-6 w-6 flex-shrink-0 items-center justify-center rounded-full {getScoreColor(item.score)} text-[10px] font-bold text-white">
									{idx + 1}
								</div>
								<div class="min-w-0 flex-1">
									<div class="flex items-center gap-2">
										<span class="truncate text-sm text-white">{item.task.title}</span>
										<span class="text-[10px] {getPriorityColor(item.task.priority)}">P{item.task.priority}</span>
									</div>
									{#if item.reason}
										<p class="mt-1 text-[10px] text-slate-500 line-clamp-1">{item.reason}</p>
									{/if}
									{#if item.task.due_at}
										<p class="mt-1 text-[10px] text-amber-400">
											Due: {new Date(item.task.due_at).toLocaleDateString()}
										</p>
									{/if}
								</div>
								<div class="flex gap-1 opacity-0 transition group-hover:opacity-100">
									<button
										class="rounded-md bg-emerald-500/20 px-2 py-1 text-[10px] text-emerald-300 hover:bg-emerald-500/30"
										on:click={() => startTask(item.task.id)}
									>
										Start
									</button>
									<button
										class="rounded-md bg-slate-700 px-2 py-1 text-[10px] text-slate-300 hover:bg-slate-600"
										on:click={() => completeTask(item.task.id)}
									>
										Done
									</button>
								</div>
							</div>
						{/each}
					</div>
				{/if}
			</div>

			<!-- Focus Block Suggestions -->
			<div class="rounded-xl border border-slate-800 bg-slate-900/40 p-5">
				<div class="flex items-center justify-between">
					<h3 class="text-sm font-semibold text-white">Suggested Focus Blocks</h3>
					<span class="text-[10px] text-slate-500">Based on calendar gaps</span>
				</div>

				{#if timeBlockSuggestions.length === 0}
					<div class="mt-4 rounded-lg border border-dashed border-slate-700 p-4 text-center">
						<p class="text-xs text-slate-400">No open slots found for focus blocks</p>
						<p class="mt-1 text-[10px] text-slate-500">Your calendar might be full today</p>
					</div>
				{:else}
					<div class="mt-4 flex flex-col gap-2">
						{#each timeBlockSuggestions as block (block.taskId + block.start)}
							<div class="rounded-lg border border-violet-500/20 bg-violet-500/5 p-3">
								<div class="flex items-center justify-between">
									<span class="text-xs font-medium text-white">{block.taskTitle}</span>
									<span class="text-[10px] text-violet-300">{block.durationMinutes}m</span>
								</div>
								<div class="mt-1 flex items-center gap-2 text-[10px] text-slate-400">
									<span>{formatTime(block.start)} - {formatTime(block.end)}</span>
									<span class="rounded bg-violet-500/20 px-1.5 py-0.5 text-violet-300">
										{Math.round(block.score * 100)}% match
									</span>
								</div>
								{#if block.reason}
									<p class="mt-1 text-[10px] text-slate-500">{block.reason}</p>
								{/if}
							</div>
						{/each}
					</div>
				{/if}

				<a
					href="/calendar"
					class="mt-4 inline-flex items-center gap-1.5 text-[10px] text-sky-400 hover:text-sky-300"
				>
					<span>Apply to calendar</span>
					<span>&rarr;</span>
				</a>
			</div>
		</div>

		<!-- Today's Schedule -->
		<div class="mt-6 rounded-xl border border-slate-800 bg-slate-900/40 p-5">
			<h3 class="text-sm font-semibold text-white">Today's Schedule</h3>
			{#if calendarItems.length === 0}
				<div class="mt-4 rounded-lg border border-dashed border-slate-700 p-4 text-center">
					<p class="text-xs text-slate-400">No events or tasks scheduled</p>
				</div>
			{:else}
				<div class="mt-4 flex flex-col gap-2">
					{#each calendarItems.slice(0, 10) as item (item.id)}
						<div class="flex items-center gap-3 rounded-lg border border-slate-800/60 px-3 py-2">
							<div class="w-16 text-[10px] text-slate-400">
								{formatTime(item.start)}
							</div>
							<div class="h-2 w-2 rounded-full {item.kind === 'event' ? 'bg-sky-500' : 'bg-violet-500'}"></div>
							<span class="flex-1 truncate text-xs text-white">{item.title}</span>
							<span class="text-[10px] text-slate-500">{item.kind}</span>
						</div>
					{/each}
				</div>
			{/if}
		</div>

		<!-- Quick Actions -->
		<div class="mt-6 flex flex-wrap gap-3">
			<a
				href="/focus"
				class="inline-flex items-center gap-2 rounded-lg bg-emerald-500 px-4 py-2 text-sm font-semibold text-white hover:bg-emerald-400"
			>
				<span>Start Focus Session</span>
			</a>
			<a
				href="/tasks"
				class="inline-flex items-center gap-2 rounded-lg border border-slate-700 px-4 py-2 text-sm text-slate-300 hover:bg-slate-800"
			>
				<span>Manage Tasks</span>
			</a>
			<a
				href="/calendar"
				class="inline-flex items-center gap-2 rounded-lg border border-slate-700 px-4 py-2 text-sm text-slate-300 hover:bg-slate-800"
			>
				<span>View Calendar</span>
			</a>
			<button
				class="inline-flex items-center gap-2 rounded-lg border border-slate-700 px-4 py-2 text-sm text-slate-300 hover:bg-slate-800"
				on:click={loadDayPlan}
			>
				<span>Refresh Plan</span>
			</button>
		</div>
	{/if}
</div>
