<script lang="ts">
	import { onMount } from 'svelte';
	import { tasksStore, loadTasks } from '$lib/stores/tasks';
	import { notesStore, loadNotes } from '$lib/stores/notes';
	import { pushToast } from '$lib/stores/toast';

	type DayStats = {
		date: string;
		tasksCreated: number;
		tasksCompleted: number;
		notesCreated: number;
	};

	type WeekComparison = {
		thisWeek: { tasks: number; completed: number; notes: number };
		lastWeek: { tasks: number; completed: number; notes: number };
	};

	type TagStats = {
		name: string;
		count: number;
		taskCount: number;
		noteCount: number;
	};

	type HourStats = {
		hour: number;
		count: number;
	};

	let loading = true;
	let timeRange: '7d' | '30d' | '90d' | 'all' = '30d';
	let dailyStats: DayStats[] = [];
	let weekComparison: WeekComparison = {
		thisWeek: { tasks: 0, completed: 0, notes: 0 },
		lastWeek: { tasks: 0, completed: 0, notes: 0 }
	};
	let tagStats: TagStats[] = [];
	let hourlyActivity: HourStats[] = [];
	let currentStreak = 0;
	let longestStreak = 0;
	let totalTasksCompleted = 0;
	let avgTasksPerDay = 0;
	let mostProductiveDay = '';
	let mostProductiveHour = 0;

	onMount(async () => {
		await Promise.all([loadTasks(), loadNotes()]);
		computeStats();
		loading = false;
	});

	$: {
		// Recompute when stores or timeRange change
		$tasksStore;
		$notesStore;
		timeRange;
		if (!loading) computeStats();
	}

	function getDateRange(): { start: Date; end: Date } {
		const end = new Date();
		const start = new Date();
		if (timeRange === '7d') {
			start.setDate(end.getDate() - 7);
		} else if (timeRange === '30d') {
			start.setDate(end.getDate() - 30);
		} else if (timeRange === '90d') {
			start.setDate(end.getDate() - 90);
		} else {
			start.setFullYear(start.getFullYear() - 10); // "all time"
		}
		start.setHours(0, 0, 0, 0);
		return { start, end };
	}

	function computeStats() {
		const { start, end } = getDateRange();
		const days = Math.ceil((end.getTime() - start.getTime()) / (1000 * 60 * 60 * 24));

		// Initialize daily stats map
		const statsMap = new Map<string, DayStats>();
		for (let i = 0; i < days; i++) {
			const d = new Date(start);
			d.setDate(d.getDate() + i);
			const key = d.toISOString().split('T')[0];
			statsMap.set(key, { date: key, tasksCreated: 0, tasksCompleted: 0, notesCreated: 0 });
		}

		// Hourly activity
		const hourCounts = new Array(24).fill(0);

		// Process tasks
		let completedCount = 0;
		for (const task of $tasksStore) {
			const createdDate = new Date(task.created_at);
			if (createdDate >= start && createdDate <= end) {
				const key = createdDate.toISOString().split('T')[0];
				const stat = statsMap.get(key);
				if (stat) stat.tasksCreated++;
				hourCounts[createdDate.getHours()]++;
			}

			if (task.completed_at) {
				const completedDate = new Date(task.completed_at);
				if (completedDate >= start && completedDate <= end) {
					const key = completedDate.toISOString().split('T')[0];
					const stat = statsMap.get(key);
					if (stat) stat.tasksCompleted++;
					completedCount++;
				}
			}
		}

		// Process notes
		for (const note of $notesStore) {
			const createdDate = new Date(note.created_at);
			if (createdDate >= start && createdDate <= end) {
				const key = createdDate.toISOString().split('T')[0];
				const stat = statsMap.get(key);
				if (stat) stat.notesCreated++;
				hourCounts[createdDate.getHours()]++;
			}
		}

		dailyStats = [...statsMap.values()].sort((a, b) => a.date.localeCompare(b.date));
		totalTasksCompleted = completedCount;
		avgTasksPerDay = days > 0 ? +(completedCount / days).toFixed(1) : 0;

		// Hourly activity
		hourlyActivity = hourCounts.map((count, hour) => ({ hour, count }));
		mostProductiveHour = hourCounts.indexOf(Math.max(...hourCounts));

		// Most productive day
		const maxCompletedDay = dailyStats.reduce(
			(max, d) => (d.tasksCompleted > max.tasksCompleted ? d : max),
			{ date: '', tasksCompleted: 0, tasksCreated: 0, notesCreated: 0 }
		);
		mostProductiveDay = maxCompletedDay.date
			? new Date(maxCompletedDay.date).toLocaleDateString(undefined, { weekday: 'long', month: 'short', day: 'numeric' })
			: 'N/A';

		// Compute streaks
		computeStreaks();

		// Week comparison
		computeWeekComparison();

		// Tag stats
		computeTagStats();
	}

	function computeStreaks() {
		const today = new Date();
		today.setHours(0, 0, 0, 0);

		// Get all days with activity
		const activeDays = new Set<string>();
		for (const task of $tasksStore) {
			if (task.completed_at) {
				const d = new Date(task.completed_at);
				d.setHours(0, 0, 0, 0);
				activeDays.add(d.toISOString().split('T')[0]);
			}
		}
		for (const note of $notesStore) {
			const d = new Date(note.created_at);
			d.setHours(0, 0, 0, 0);
			activeDays.add(d.toISOString().split('T')[0]);
		}

		// Current streak (from today backwards)
		let streak = 0;
		const checkDate = new Date(today);
		while (activeDays.has(checkDate.toISOString().split('T')[0])) {
			streak++;
			checkDate.setDate(checkDate.getDate() - 1);
		}
		currentStreak = streak;

		// Longest streak
		const sortedDays = [...activeDays].sort();
		let maxStreak = 0;
		let tempStreak = 0;
		let prevDate: Date | null = null;

		for (const dayStr of sortedDays) {
			const d = new Date(dayStr);
			if (prevDate) {
				const diff = (d.getTime() - prevDate.getTime()) / (1000 * 60 * 60 * 24);
				if (diff === 1) {
					tempStreak++;
				} else {
					maxStreak = Math.max(maxStreak, tempStreak);
					tempStreak = 1;
				}
			} else {
				tempStreak = 1;
			}
			prevDate = d;
		}
		longestStreak = Math.max(maxStreak, tempStreak);
	}

	function computeWeekComparison() {
		const now = new Date();
		const thisWeekStart = new Date(now);
		thisWeekStart.setDate(now.getDate() - now.getDay());
		thisWeekStart.setHours(0, 0, 0, 0);

		const lastWeekStart = new Date(thisWeekStart);
		lastWeekStart.setDate(lastWeekStart.getDate() - 7);

		const lastWeekEnd = new Date(thisWeekStart);

		let thisWeek = { tasks: 0, completed: 0, notes: 0 };
		let lastWeek = { tasks: 0, completed: 0, notes: 0 };

		for (const task of $tasksStore) {
			const created = new Date(task.created_at);
			if (created >= thisWeekStart) {
				thisWeek.tasks++;
			} else if (created >= lastWeekStart && created < lastWeekEnd) {
				lastWeek.tasks++;
			}

			if (task.completed_at) {
				const completed = new Date(task.completed_at);
				if (completed >= thisWeekStart) {
					thisWeek.completed++;
				} else if (completed >= lastWeekStart && completed < lastWeekEnd) {
					lastWeek.completed++;
				}
			}
		}

		for (const note of $notesStore) {
			const created = new Date(note.created_at);
			if (created >= thisWeekStart) {
				thisWeek.notes++;
			} else if (created >= lastWeekStart && created < lastWeekEnd) {
				lastWeek.notes++;
			}
		}

		weekComparison = { thisWeek, lastWeek };
	}

	function computeTagStats() {
		const tagMap = new Map<string, TagStats>();

		for (const task of $tasksStore) {
			for (const label of task.labels ?? []) {
				const existing = tagMap.get(label) ?? { name: label, count: 0, taskCount: 0, noteCount: 0 };
				existing.count++;
				existing.taskCount++;
				tagMap.set(label, existing);
			}
		}

		for (const note of $notesStore) {
			for (const tag of note.tags ?? []) {
				const existing = tagMap.get(tag) ?? { name: tag, count: 0, taskCount: 0, noteCount: 0 };
				existing.count++;
				existing.noteCount++;
				tagMap.set(tag, existing);
			}
		}

		tagStats = [...tagMap.values()].sort((a, b) => b.count - a.count).slice(0, 15);
	}

	function formatHour(hour: number): string {
		if (hour === 0) return '12am';
		if (hour === 12) return '12pm';
		return hour < 12 ? `${hour}am` : `${hour - 12}pm`;
	}

	function getComparisonColor(current: number, previous: number): string {
		if (current > previous) return 'text-emerald-400';
		if (current < previous) return 'text-red-400';
		return 'text-slate-400';
	}

	function getComparisonIcon(current: number, previous: number): string {
		if (current > previous) return '↑';
		if (current < previous) return '↓';
		return '→';
	}

	function getBarHeight(value: number, max: number): number {
		return max > 0 ? Math.max(4, (value / max) * 100) : 4;
	}

	$: maxDailyCompleted = Math.max(...dailyStats.map((d) => d.tasksCompleted), 1);
	$: maxDailyCreated = Math.max(...dailyStats.map((d) => d.tasksCreated + d.notesCreated), 1);
	$: maxHourly = Math.max(...hourlyActivity.map((h) => h.count), 1);
	$: maxTagCount = Math.max(...tagStats.map((t) => t.count), 1);
</script>

<div class="space-y-6">
	<div class="flex flex-wrap items-center justify-between gap-3">
		<div>
			<h2 class="text-lg font-semibold text-white">Productivity Stats</h2>
			<p class="text-xs text-slate-400">Track your progress and identify patterns</p>
		</div>
		<div class="flex rounded-lg border border-slate-700 text-[10px]">
			<button
				class={`px-3 py-1.5 transition ${timeRange === '7d' ? 'bg-slate-700 text-white' : 'text-slate-400 hover:text-white'}`}
				on:click={() => (timeRange = '7d')}
			>
				7 days
			</button>
			<button
				class={`px-3 py-1.5 transition ${timeRange === '30d' ? 'bg-slate-700 text-white' : 'text-slate-400 hover:text-white'}`}
				on:click={() => (timeRange = '30d')}
			>
				30 days
			</button>
			<button
				class={`px-3 py-1.5 transition ${timeRange === '90d' ? 'bg-slate-700 text-white' : 'text-slate-400 hover:text-white'}`}
				on:click={() => (timeRange = '90d')}
			>
				90 days
			</button>
			<button
				class={`px-3 py-1.5 transition ${timeRange === 'all' ? 'bg-slate-700 text-white' : 'text-slate-400 hover:text-white'}`}
				on:click={() => (timeRange = 'all')}
			>
				All time
			</button>
		</div>
	</div>

	{#if loading}
		<div class="flex h-64 items-center justify-center rounded-xl border border-slate-800">
			<p class="text-xs text-slate-400">Loading stats...</p>
		</div>
	{:else}
		<!-- Key Metrics -->
		<div class="grid grid-cols-2 gap-3 md:grid-cols-4 lg:grid-cols-6">
			<div class="rounded-xl border border-slate-800 bg-slate-900/40 p-4">
				<div class="text-2xl font-bold text-emerald-400">{totalTasksCompleted}</div>
				<div class="text-[11px] text-slate-400">Tasks Completed</div>
			</div>
			<div class="rounded-xl border border-slate-800 bg-slate-900/40 p-4">
				<div class="text-2xl font-bold text-sky-400">{avgTasksPerDay}</div>
				<div class="text-[11px] text-slate-400">Avg/Day</div>
			</div>
			<div class="rounded-xl border border-slate-800 bg-slate-900/40 p-4">
				<div class="text-2xl font-bold text-amber-400">{currentStreak}</div>
				<div class="text-[11px] text-slate-400">Current Streak</div>
			</div>
			<div class="rounded-xl border border-slate-800 bg-slate-900/40 p-4">
				<div class="text-2xl font-bold text-violet-400">{longestStreak}</div>
				<div class="text-[11px] text-slate-400">Longest Streak</div>
			</div>
			<div class="rounded-xl border border-slate-800 bg-slate-900/40 p-4">
				<div class="text-sm font-bold text-white truncate">{mostProductiveDay || 'N/A'}</div>
				<div class="text-[11px] text-slate-400">Best Day</div>
			</div>
			<div class="rounded-xl border border-slate-800 bg-slate-900/40 p-4">
				<div class="text-2xl font-bold text-teal-400">{formatHour(mostProductiveHour)}</div>
				<div class="text-[11px] text-slate-400">Peak Hour</div>
			</div>
		</div>

		<!-- Week Comparison -->
		<div class="rounded-xl border border-slate-800 bg-slate-900/40 p-4">
			<h3 class="text-xs font-semibold uppercase tracking-wide text-slate-500">This Week vs Last Week</h3>
			<div class="mt-3 grid grid-cols-3 gap-4">
				<div>
					<div class="flex items-baseline gap-2">
						<span class="text-xl font-bold text-white">{weekComparison.thisWeek.tasks}</span>
						<span class={`text-xs ${getComparisonColor(weekComparison.thisWeek.tasks, weekComparison.lastWeek.tasks)}`}>
							{getComparisonIcon(weekComparison.thisWeek.tasks, weekComparison.lastWeek.tasks)}
							{Math.abs(weekComparison.thisWeek.tasks - weekComparison.lastWeek.tasks)}
						</span>
					</div>
					<div class="text-[11px] text-slate-500">Tasks Created</div>
				</div>
				<div>
					<div class="flex items-baseline gap-2">
						<span class="text-xl font-bold text-emerald-400">{weekComparison.thisWeek.completed}</span>
						<span class={`text-xs ${getComparisonColor(weekComparison.thisWeek.completed, weekComparison.lastWeek.completed)}`}>
							{getComparisonIcon(weekComparison.thisWeek.completed, weekComparison.lastWeek.completed)}
							{Math.abs(weekComparison.thisWeek.completed - weekComparison.lastWeek.completed)}
						</span>
					</div>
					<div class="text-[11px] text-slate-500">Completed</div>
				</div>
				<div>
					<div class="flex items-baseline gap-2">
						<span class="text-xl font-bold text-violet-400">{weekComparison.thisWeek.notes}</span>
						<span class={`text-xs ${getComparisonColor(weekComparison.thisWeek.notes, weekComparison.lastWeek.notes)}`}>
							{getComparisonIcon(weekComparison.thisWeek.notes, weekComparison.lastWeek.notes)}
							{Math.abs(weekComparison.thisWeek.notes - weekComparison.lastWeek.notes)}
						</span>
					</div>
					<div class="text-[11px] text-slate-500">Notes Created</div>
				</div>
			</div>
		</div>

		<div class="grid gap-6 lg:grid-cols-2">
			<!-- Daily Activity Chart -->
			<div class="rounded-xl border border-slate-800 bg-slate-900/40 p-4">
				<h3 class="text-xs font-semibold uppercase tracking-wide text-slate-500">Daily Activity</h3>
				<div class="mt-4 flex h-32 items-end gap-0.5 overflow-x-auto pb-6">
					{#each dailyStats.slice(-30) as day}
						<div class="group relative flex flex-1 min-w-[8px] flex-col items-center gap-0.5">
							<!-- Tasks completed bar -->
							<div
								class="w-full rounded-t bg-emerald-500/60 transition-all group-hover:bg-emerald-400"
								style="height: {getBarHeight(day.tasksCompleted, maxDailyCompleted)}%"
								title="{day.tasksCompleted} completed"
							></div>
							<!-- Tasks + notes created bar -->
							<div
								class="w-full rounded-b bg-sky-500/40 transition-all group-hover:bg-sky-400/60"
								style="height: {getBarHeight(day.tasksCreated + day.notesCreated, maxDailyCreated) * 0.5}%"
								title="{day.tasksCreated} tasks, {day.notesCreated} notes created"
							></div>
							<!-- Date label (show for first, last, and every 7th) -->
							{#if dailyStats.slice(-30).indexOf(day) % 7 === 0 || dailyStats.slice(-30).indexOf(day) === dailyStats.slice(-30).length - 1}
								<span class="absolute -bottom-5 text-[8px] text-slate-600 whitespace-nowrap">
									{new Date(day.date).toLocaleDateString(undefined, { month: 'short', day: 'numeric' })}
								</span>
							{/if}
						</div>
					{/each}
				</div>
				<div class="mt-2 flex justify-center gap-4 text-[10px]">
					<div class="flex items-center gap-1.5">
						<div class="h-2 w-2 rounded bg-emerald-500"></div>
						<span class="text-slate-400">Completed</span>
					</div>
					<div class="flex items-center gap-1.5">
						<div class="h-2 w-2 rounded bg-sky-500/60"></div>
						<span class="text-slate-400">Created</span>
					</div>
				</div>
			</div>

			<!-- Hourly Activity -->
			<div class="rounded-xl border border-slate-800 bg-slate-900/40 p-4">
				<h3 class="text-xs font-semibold uppercase tracking-wide text-slate-500">Activity by Hour</h3>
				<div class="mt-4 flex h-32 items-end gap-0.5">
					{#each hourlyActivity as hourData}
						<div class="group relative flex flex-1 min-w-[10px] flex-col items-center">
							<div
								class="w-full rounded-t transition-all {hourData.hour === mostProductiveHour ? 'bg-amber-500' : 'bg-violet-500/60 group-hover:bg-violet-400'}"
								style="height: {getBarHeight(hourData.count, maxHourly)}%"
								title="{hourData.count} items at {formatHour(hourData.hour)}"
							></div>
						</div>
					{/each}
				</div>
				<div class="mt-2 flex justify-between text-[9px] text-slate-600">
					<span>12am</span>
					<span>6am</span>
					<span>12pm</span>
					<span>6pm</span>
					<span>11pm</span>
				</div>
			</div>
		</div>

		<!-- Tag Usage -->
		<div class="rounded-xl border border-slate-800 bg-slate-900/40 p-4">
			<h3 class="text-xs font-semibold uppercase tracking-wide text-slate-500">Top Tags</h3>
			{#if tagStats.length === 0}
				<p class="mt-3 text-xs text-slate-500">No tags used yet.</p>
			{:else}
				<div class="mt-3 space-y-2">
					{#each tagStats as tag}
						<div class="flex items-center gap-3">
							<span class="w-24 truncate text-xs text-slate-300">{tag.name}</span>
							<div class="flex-1 h-4 rounded-full bg-slate-800 overflow-hidden">
								<div class="flex h-full">
									<div
										class="bg-violet-500/70"
										style="width: {(tag.taskCount / maxTagCount) * 100}%"
										title="{tag.taskCount} tasks"
									></div>
									<div
										class="bg-sky-500/70"
										style="width: {(tag.noteCount / maxTagCount) * 100}%"
										title="{tag.noteCount} notes"
									></div>
								</div>
							</div>
							<span class="w-8 text-right text-[10px] text-slate-500">{tag.count}</span>
						</div>
					{/each}
				</div>
				<div class="mt-3 flex justify-center gap-4 text-[10px]">
					<div class="flex items-center gap-1.5">
						<div class="h-2 w-2 rounded bg-violet-500"></div>
						<span class="text-slate-400">Tasks</span>
					</div>
					<div class="flex items-center gap-1.5">
						<div class="h-2 w-2 rounded bg-sky-500"></div>
						<span class="text-slate-400">Notes</span>
					</div>
				</div>
			{/if}
		</div>

		<!-- Activity Heatmap -->
		<div class="rounded-xl border border-slate-800 bg-slate-900/40 p-4">
			<h3 class="text-xs font-semibold uppercase tracking-wide text-slate-500">Activity Heatmap (Last 12 Weeks)</h3>
			<div class="mt-3 overflow-x-auto">
				<div class="flex gap-0.5">
					{#each { length: 7 } as _, dayOfWeek}
						<div class="flex flex-col gap-0.5">
							{#each { length: 12 } as _, weekOffset}
								{@const date = new Date()}
								{@const shiftedDateValue = date.setDate(date.getDate() - date.getDay() - (11 - weekOffset) * 7 + dayOfWeek)}
								{@const dateStr = date.toISOString().split('T')[0]}
								{@const dayData = dailyStats.find((d) => d.date === dateStr)}
								{@const activity = (dayData?.tasksCompleted ?? 0) + (dayData?.tasksCreated ?? 0) + (dayData?.notesCreated ?? 0)}
								<div
									class="h-3 w-3 rounded-sm transition-colors {activity === 0
										? 'bg-slate-800'
										: activity <= 2
											? 'bg-emerald-900'
											: activity <= 5
												? 'bg-emerald-700'
												: activity <= 10
													? 'bg-emerald-500'
													: 'bg-emerald-300'}"
									title="{date.toLocaleDateString()}: {activity} items"
								></div>
							{/each}
						</div>
					{/each}
				</div>
				<div class="mt-2 flex items-center justify-between text-[9px] text-slate-600">
					<div class="flex items-center gap-1">
						<span>Less</span>
						<div class="h-2 w-2 rounded-sm bg-slate-800"></div>
						<div class="h-2 w-2 rounded-sm bg-emerald-900"></div>
						<div class="h-2 w-2 rounded-sm bg-emerald-700"></div>
						<div class="h-2 w-2 rounded-sm bg-emerald-500"></div>
						<div class="h-2 w-2 rounded-sm bg-emerald-300"></div>
						<span>More</span>
					</div>
					<span>Sun-Sat by week</span>
				</div>
			</div>
		</div>
	{/if}
</div>
