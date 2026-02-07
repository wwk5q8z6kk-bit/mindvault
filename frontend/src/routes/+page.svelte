<script lang="ts">
	import { resolve } from '$app/paths';
	import { goto } from '$app/navigation';
	import { tasksStore, loadTasks } from '$lib/stores/tasks';
	import { notesStore, loadNotes } from '$lib/stores/notes';
	import { shouldShowOnboarding } from '$lib/stores/onboarding';
	import { listDueTasks } from '$lib/api/tasks';
	import type { Task } from '$lib/api/tasks';
	import { onMount } from 'svelte';

	let loaded = false;

	$: allTasks = $tasksStore;
	$: inboxCount = allTasks.filter((t) => t.status === 'inbox').length;
	$: inProgressCount = allTasks.filter((t) => t.status === 'in_progress').length;

	let dueTodayTasks: Task[] = [];
	let dueTodayCount = 0;
	let overdueCount = 0;

	// Fallback: compute due stats locally from the store
	function computeDueStatsLocally() {
		const now = new Date();
		const todayStart = new Date(now.getFullYear(), now.getMonth(), now.getDate());
		const todayEnd = new Date(now.getFullYear(), now.getMonth(), now.getDate() + 1);
		dueTodayTasks = allTasks.filter((t) => {
			if (!t.due_at || t.status === 'done') return false;
			const d = new Date(t.due_at);
			return d >= todayStart && d < todayEnd;
		});
		dueTodayCount = dueTodayTasks.length;
		overdueCount = allTasks.filter((t) => {
			if (!t.due_at || t.status === 'done') return false;
			return new Date(t.due_at) < todayStart;
		}).length;
	}

	$: {
		const weekAgo = new Date();
		weekAgo.setDate(weekAgo.getDate() - 7);
		doneThisWeek = allTasks.filter(
			(t) => t.status === 'done' && t.completed_at && new Date(t.completed_at) >= weekAgo
		).length;
	}
	let doneThisWeek = 0;

	$: recentNotes = $notesStore.slice(0, 5);

	const quickActions = [
		{ label: 'New Task', href: '/tasks', icon: '+', color: 'sky' },
		{ label: 'Goals', href: '/goals', icon: 'G', color: 'emerald' },
		{ label: 'Notes', href: '/notes', icon: 'N', color: 'violet' },
		{ label: 'Daily Note', href: '/daily', icon: 'D', color: 'amber' },
		{ label: 'Search', href: '/search', icon: 'S', color: 'emerald' },
		{ label: 'Kanban', href: '/kanban', icon: 'K', color: 'rose' },
		{ label: 'Calendar', href: '/calendar', icon: 'C', color: 'teal' }
	];

	const colorMap: Record<string, string> = {
		sky: 'bg-sky-500/20 text-sky-200',
		violet: 'bg-violet-500/20 text-violet-200',
		amber: 'bg-amber-500/20 text-amber-200',
		emerald: 'bg-emerald-500/20 text-emerald-200',
		rose: 'bg-rose-500/20 text-rose-200',
		teal: 'bg-teal-500/20 text-teal-200'
	};

	function formatDueTime(due: string): string {
		const d = new Date(due);
		return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
	}

	onMount(() => {
		// Check if onboarding should be shown
		const unsubscribe = shouldShowOnboarding.subscribe((show) => {
			if (show) {
				goto('/onboarding');
			}
		});

		void (async () => {
			await Promise.all([loadTasks(), loadNotes()]);

			// Try to use the dedicated due-tasks API for accurate counts
			try {
				const dueResponse = await listDueTasks({ include_overdue: true });
				overdueCount = dueResponse.overdue_count;
				dueTodayCount = dueResponse.due_today_count;
				// Filter the tasks array for the "Due Today" list display
				const now = new Date();
				const todayStart = new Date(now.getFullYear(), now.getMonth(), now.getDate());
				const todayEnd = new Date(now.getFullYear(), now.getMonth(), now.getDate() + 1);
				dueTodayTasks = dueResponse.tasks.filter((t) =>
					t.due_at ? new Date(t.due_at) >= todayStart && new Date(t.due_at) < todayEnd : false
				);
			} catch {
				// Offline or API unavailable - fall back to local filtering
				computeDueStatsLocally();
			}

			loaded = true;
		})();

		return unsubscribe;
	});
</script>

<div class="mx-auto max-w-5xl space-y-6">
	<!-- Stats row -->
	<div class="grid grid-cols-2 gap-3 md:grid-cols-5">
		<div class="rounded-xl border border-slate-800 bg-slate-900/40 p-4">
			<div class="text-2xl font-bold text-white">{allTasks.length}</div>
			<div class="text-[11px] text-slate-400">Total Tasks</div>
		</div>
		<div class="rounded-xl border border-slate-800 bg-slate-900/40 p-4">
			<div class="text-2xl font-bold text-sky-300">{inboxCount}</div>
			<div class="text-[11px] text-slate-400">Inbox</div>
		</div>
		<div class="rounded-xl border border-slate-800 bg-slate-900/40 p-4">
			<div class="text-2xl font-bold text-amber-300">{dueTodayCount}</div>
			<div class="text-[11px] text-slate-400">Due Today</div>
		</div>
		<div class="rounded-xl border border-slate-800 bg-slate-900/40 p-4">
			<div class="text-2xl font-bold {overdueCount > 0 ? 'text-red-400' : 'text-slate-300'}">
				{overdueCount}
			</div>
			<div class="text-[11px] text-slate-400">Overdue</div>
		</div>
		<div class="rounded-xl border border-slate-800 bg-slate-900/40 p-4">
			<div class="text-2xl font-bold text-emerald-300">{doneThisWeek}</div>
			<div class="text-[11px] text-slate-400">Done This Week</div>
		</div>
	</div>

	<div class="grid gap-6 lg:grid-cols-3">
		<!-- Due Today -->
		<div class="lg:col-span-2">
			<div class="rounded-xl border border-slate-800 bg-slate-900/40 p-5">
				<div class="flex items-center justify-between">
					<h3 class="text-sm font-semibold text-white">Due Today</h3>
					<a
						href={resolve('/tasks')}
						class="text-[11px] text-sky-400 hover:text-sky-300"
					>
						View all tasks
					</a>
				</div>
				<div class="mt-3 space-y-2">
					{#if !loaded}
						<p class="text-xs text-slate-500">Loading...</p>
					{:else if dueTodayTasks.length === 0}
						<p class="py-4 text-center text-xs text-slate-500">No tasks due today. Enjoy!</p>
					{:else}
						{#each dueTodayTasks as task (task.id)}
							<a
								href={resolve('/tasks')}
								class="flex items-center justify-between rounded-lg border border-slate-800/60 px-3 py-2.5 transition hover:border-slate-700"
							>
								<div class="flex items-center gap-2.5 min-w-0">
									<span
										class="h-2 w-2 flex-shrink-0 rounded-full {task.priority <= 1
											? 'bg-red-400'
											: task.priority === 2
												? 'bg-orange-400'
												: task.priority === 3
													? 'bg-yellow-400'
													: 'bg-slate-500'}"
									></span>
									<span class="truncate text-xs font-medium text-white">{task.title}</span>
								</div>
								<div class="flex items-center gap-2 flex-shrink-0">
									{#if task.due_at}
										<span class="text-[10px] text-slate-500">{formatDueTime(task.due_at)}</span>
									{/if}
									<span
										class="rounded-full px-2 py-0.5 text-[9px] uppercase tracking-wide {task.status === 'in_progress'
											? 'bg-blue-500/20 text-blue-300'
											: task.status === 'planned'
												? 'bg-violet-500/20 text-violet-300'
												: 'bg-slate-800 text-slate-400'}"
									>
										{task.status.replace('_', ' ')}
									</span>
								</div>
							</a>
						{/each}
					{/if}
				</div>

				{#if overdueCount > 0}
					<div class="mt-3 rounded-lg border border-red-500/20 bg-red-500/5 px-3 py-2">
						<span class="text-[11px] text-red-300">
							{overdueCount} overdue task{overdueCount > 1 ? 's' : ''} need attention
						</span>
					</div>
				{/if}
			</div>

			<!-- In Progress -->
			{#if inProgressCount > 0}
				<div class="mt-4 rounded-xl border border-slate-800 bg-slate-900/40 p-5">
					<h3 class="text-sm font-semibold text-white">In Progress</h3>
					<div class="mt-3 space-y-2">
						{#each allTasks.filter((t) => t.status === 'in_progress').slice(0, 5) as task (task.id)}
							<div class="flex items-center gap-2.5 rounded-lg border border-slate-800/60 px-3 py-2">
								<span class="h-2 w-2 rounded-full bg-blue-400"></span>
								<span class="truncate text-xs text-white">{task.title}</span>
								{#if task.labels?.length}
									<div class="ml-auto flex gap-1">
										{#each task.labels.slice(0, 2) as label}
											<span class="rounded-full bg-slate-800 px-1.5 py-0.5 text-[9px] text-slate-400">{label}</span>
										{/each}
									</div>
								{/if}
							</div>
						{/each}
					</div>
				</div>
			{/if}
		</div>

		<!-- Right sidebar -->
		<div class="space-y-4">
			<!-- Quick actions -->
			<div class="rounded-xl border border-slate-800 bg-slate-900/40 p-5">
				<h3 class="text-sm font-semibold text-white">Quick Actions</h3>
				<div class="mt-3 grid grid-cols-3 gap-2">
					{#each quickActions as action}
						<a
							href={action.href}
							class="flex flex-col items-center gap-1.5 rounded-lg border border-slate-800/60 px-2 py-3 transition hover:border-slate-700 hover:bg-slate-800/40"
						>
							<div
								class="flex h-8 w-8 items-center justify-center rounded-lg text-xs font-bold {colorMap[action.color]}"
							>
								{action.icon}
							</div>
							<span class="text-[10px] text-slate-400">{action.label}</span>
						</a>
					{/each}
				</div>
			</div>

			<!-- Recent Notes -->
			<div class="rounded-xl border border-slate-800 bg-slate-900/40 p-5">
				<div class="flex items-center justify-between">
					<h3 class="text-sm font-semibold text-white">Recent Notes</h3>
					<a
						href={resolve('/notes')}
						class="text-[11px] text-sky-400 hover:text-sky-300"
					>
						All notes
					</a>
				</div>
				<div class="mt-3 space-y-1.5">
					{#if recentNotes.length === 0}
						<p class="text-[11px] text-slate-500">No notes yet.</p>
					{:else}
						{#each recentNotes as note (note.id)}
							<a
								href={`/notes?note=${note.id}`}
								class="block rounded-lg px-2.5 py-2 transition hover:bg-slate-800/60"
							>
								<div class="flex items-center gap-1.5">
									{#if note.pinned}
										<span class="text-[9px] text-amber-400">pin</span>
									{/if}
									<span class="truncate text-xs font-medium text-slate-200">{note.title || 'Untitled'}</span>
								</div>
								<div class="mt-0.5 truncate text-[10px] text-slate-500">
									{note.markdown.slice(0, 50)}
								</div>
							</a>
						{/each}
					{/if}
				</div>
			</div>

			<!-- Keyboard hint -->
			<div class="rounded-xl border border-slate-800/60 bg-slate-900/20 p-4">
				<p class="text-[11px] text-slate-500">
					<kbd class="rounded border border-slate-700 bg-slate-800 px-1 py-0.5 text-[10px] text-slate-400">Cmd+K</kbd>
					to open command palette
				</p>
			</div>
		</div>
	</div>
</div>
