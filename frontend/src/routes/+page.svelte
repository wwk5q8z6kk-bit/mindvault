<script lang="ts">
	import { goto } from '$app/navigation';
	import { tasksStore, loadTasks } from '$lib/stores/tasks';
	import { notesStore, loadNotes } from '$lib/stores/notes';
	import { shouldShowOnboarding } from '$lib/stores/onboarding';
	import { fetchBriefing, type BriefingResponse } from '$lib/api/briefing';
	import AiBriefingWidget from '$lib/components/AiBriefingWidget.svelte';
	import DueTasksWidget from '$lib/components/DueTasksWidget.svelte';
	import HabitsWidget from '$lib/components/HabitsWidget.svelte';
	import WhatsNextWidget from '$lib/components/WhatsNextWidget.svelte';
	import InsightsDashboard from '$lib/components/InsightsDashboard.svelte';
	import RecentNotesWidget from '$lib/components/RecentNotesWidget.svelte';
	import IntentInbox from '$lib/components/IntentInbox.svelte';
	import AgentStream from '$lib/components/AgentStream.svelte';
	import { onMount } from 'svelte';

	let loaded = false;
	let briefing: BriefingResponse | null = null;
	let briefingLoading = true;
	let isMac = false;

	// Knowledge graph stats
	let graphNodes = 0;
	let graphEdges = 0;
	let graphLoaded = false;

	$: allTasks = $tasksStore;
	$: allNotes = $notesStore;
	$: inboxCount = allTasks.filter((t) => t.status === 'inbox').length;
	$: inProgressCount = allTasks.filter((t) => t.status === 'in_progress').length;

	// Activity feed: merge recent tasks and notes, sorted by updated_at
	$: activityItems = [
		...allTasks.slice(0, 10).map((t) => ({
			id: t.id,
			type: 'task' as const,
			title: t.title,
			updated_at: t.updated_at,
			status: t.status
		})),
		...allNotes.slice(0, 10).map((n) => ({
			id: n.id,
			type: 'note' as const,
			title: n.title ?? 'Untitled',
			updated_at: n.updated_at,
			status: null
		}))
	]
		.sort((a, b) => new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime())
		.slice(0, 5);

	let dueTodayCount = 0;
	let overdueCount = 0;

	// Fallback: compute due stats locally from the store
	function computeDueStatsLocally() {
		const now = new Date();
		const todayStart = new Date(now.getFullYear(), now.getMonth(), now.getDate());
		const todayEnd = new Date(now.getFullYear(), now.getMonth(), now.getDate() + 1);
		dueTodayCount = allTasks.filter((t) => {
			if (!t.due_at || t.status === 'done') return false;
			const d = new Date(t.due_at);
			return d >= todayStart && d < todayEnd;
		}).length;
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

	const topActions = [
		{
			label: 'New Note',
			href: '/notes',
			icon: 'M12 4v16m-8-8h16',
			color: 'violet'
		},
		{
			label: 'New Task',
			href: '/tasks',
			icon: 'M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z',
			color: 'sky'
		},
		{
			label: 'Voice Capture',
			href: '/notes?voice=1',
			icon: 'M19 11a7 7 0 01-7 7m0 0a7 7 0 01-7-7m7 7v4m0 0H8m4 0h4M12 15a3 3 0 003-3V5a3 3 0 00-6 0v7a3 3 0 003 3z',
			color: 'amber'
		},
		{
			label: 'AI Chat',
			href: '/chat',
			icon: 'M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z',
			color: 'emerald'
		}
	];

	const topColorMap: Record<string, string> = {
		sky: 'bg-sky-500/15 text-sky-300 hover:bg-sky-500/25',
		violet: 'bg-violet-500/15 text-violet-300 hover:bg-violet-500/25',
		amber: 'bg-amber-500/15 text-amber-300 hover:bg-amber-500/25',
		emerald: 'bg-emerald-500/15 text-emerald-300 hover:bg-emerald-500/25'
	};

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

	function formatRelativeTime(dateStr: string): string {
		const diff = Date.now() - new Date(dateStr).getTime();
		const mins = Math.floor(diff / 60000);
		if (mins < 1) return 'just now';
		if (mins < 60) return `${mins}m ago`;
		const hours = Math.floor(mins / 60);
		if (hours < 24) return `${hours}h ago`;
		const days = Math.floor(hours / 24);
		return `${days}d ago`;
	}

	onMount(() => {
		isMac = navigator.platform?.startsWith('Mac') ?? false;

		// Check if onboarding should be shown
		const unsubscribe = shouldShowOnboarding.subscribe((show) => {
			if (show) {
				goto('/onboarding');
			}
		});

		void (async () => {
			await Promise.all([loadTasks(), loadNotes()]);

			// Try the briefing API for all dashboard data in one call
			try {
				briefing = await fetchBriefing();
				overdueCount = briefing.overdue.length;
				dueTodayCount = briefing.due_today.length;
			} catch {
				// Offline or API unavailable — fall back to local filtering
				computeDueStatsLocally();
			} finally {
				briefingLoading = false;
			}

			// Fetch graph stats (non-blocking)
			try {
				const res = await fetch('/api/v1/graph/stats');
				if (res.ok) {
					const stats = await res.json();
					graphNodes = stats.node_count ?? stats.nodes ?? 0;
					graphEdges = stats.edge_count ?? stats.edges ?? 0;
					graphLoaded = true;
				}
			} catch {
				// Graph API unavailable — leave hidden
			}

			loaded = true;
		})();

		return unsubscribe;
	});
</script>

<div class="mx-auto max-w-5xl space-y-6">
	<!-- Quick Actions (top row) -->
	<div class="flex flex-wrap gap-2">
		{#each topActions as action}
			<a
				href={action.href}
				class={`mv-btn rounded-full gap-2 px-4 py-2 text-sm font-medium transition-all duration-200 ${topColorMap[action.color]}`}
			>
				<svg class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
					<path stroke-linecap="round" stroke-linejoin="round" d={action.icon} />
				</svg>
				{action.label}
			</a>
		{/each}
	</div>

	<!-- AI Briefing -->
	<AiBriefingWidget
		summary={briefing?.summary ?? ''}
		date={briefing?.date ?? ''}
		loading={briefingLoading}
	/>

	<!-- Stats row -->
	<dl class="grid grid-cols-2 gap-3 sm:grid-cols-3 md:grid-cols-5">
		<div class="mv-section">
			<dd class="text-2xl font-bold text-[rgb(var(--mv-text))]">{allTasks.length}</dd>
			<dt class="text-xs text-[rgb(var(--mv-muted))]">Total Tasks</dt>
		</div>
		<div class="mv-section">
			<dd class="text-2xl font-bold text-sky-300">{inboxCount}</dd>
			<dt class="text-xs text-[rgb(var(--mv-muted))]">Inbox</dt>
		</div>
		<div class="mv-section">
			<dd class="text-2xl font-bold text-amber-300">{dueTodayCount}</dd>
			<dt class="text-xs text-[rgb(var(--mv-muted))]">Due Today</dt>
		</div>
		<div class="mv-section">
			<dd
				class="text-2xl font-bold {overdueCount > 0
					? 'text-red-400'
					: 'text-[rgb(var(--mv-text))]'}"
			>
				{overdueCount}
			</dd>
			<dt class="text-xs text-[rgb(var(--mv-muted))]">Overdue</dt>
		</div>
		<div class="mv-section">
			<dd class="text-2xl font-bold text-emerald-300">{doneThisWeek}</dd>
			<dt class="text-xs text-[rgb(var(--mv-muted))]">Done This Week</dt>
		</div>
	</dl>

	<!-- Main widget grid -->
	<div class="grid grid-cols-1 gap-6 md:grid-cols-2 xl:grid-cols-3">
		<!-- Due Today + Overdue + In Progress (span 2 on xl) -->
		<div class="space-y-4 md:col-span-2 xl:col-span-2">
			<div class="mv-section">
				<DueTasksWidget
					tasks={briefing?.due_today ?? []}
					title="Due Today"
					loading={briefingLoading}
				/>
			</div>

			{#if (briefing?.overdue ?? []).length > 0}
				<div class="mv-section">
					<DueTasksWidget tasks={briefing?.overdue ?? []} title="Overdue" loading={briefingLoading} />
				</div>
			{/if}

			<!-- In Progress -->
			{#if inProgressCount > 0}
				<div class="mv-section p-5">
					<h3 class="text-sm font-semibold text-[rgb(var(--mv-text))]">In Progress</h3>
					<div class="mt-3 space-y-2">
						{#each allTasks.filter((t) => t.status === 'in_progress').slice(0, 5) as task (task.id)}
							<div
								class="flex items-center gap-2.5 rounded-lg border border-[rgb(var(--mv-border))]/80 bg-[rgb(var(--mv-panel-strong))]/30 px-3 py-2"
							>
								<span class="h-2 w-2 rounded-full bg-blue-400"></span>
								<span class="truncate text-sm text-[rgb(var(--mv-text))]">{task.title}</span>
								{#if task.labels?.length}
									<div class="ml-auto flex gap-1">
										{#each task.labels.slice(0, 2) as label}
											<span
												class="rounded-full bg-[rgb(var(--mv-panel-strong))] px-2 py-0.5 text-[11px] text-[rgb(var(--mv-muted))]"
												>{label}</span
											>
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
			<!-- What's Next: AI-prioritized top task -->
			<div class="mv-section">
				<WhatsNextWidget />
			</div>

			<!-- Habits -->
			<div class="mv-section">
				<HabitsWidget habits={briefing?.habits_today ?? []} loading={briefingLoading} />
			</div>

			<!-- Quick actions -->
			<div class="mv-section p-5">
				<h3 class="text-sm font-semibold text-[rgb(var(--mv-text))]">Quick Actions</h3>
				<div class="mt-3 grid grid-cols-3 gap-2">
					{#each quickActions as action}
						<a
							href={action.href}
							class="flex flex-col items-center gap-1.5 rounded-lg border border-[rgb(var(--mv-border))]/80 px-2 py-3 transition hover:bg-[rgb(var(--mv-panel-strong))]/40"
						>
							<div
								class="flex h-8 w-8 items-center justify-center rounded-lg text-xs font-bold {colorMap[
									action.color
								]}"
							>
								{action.icon}
							</div>
							<span class="text-xs text-[rgb(var(--mv-muted))]">{action.label}</span>
						</a>
					{/each}
				</div>
			</div>

			<!-- Knowledge Graph Preview -->
			{#if graphLoaded}
				<a
					href="/notes?view=graph"
					class="mv-section flex items-center gap-3 transition hover:border-[rgb(var(--mv-accent))]/40"
				>
					<div class="flex h-10 w-10 items-center justify-center rounded-lg bg-violet-500/15">
						<svg class="h-5 w-5 text-violet-300" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
							<circle cx="12" cy="5" r="2" /><circle cx="5" cy="19" r="2" /><circle cx="19" cy="19" r="2" />
							<path d="M12 7v4m-5.2 4.8L11 13m2 0l4.2 2.8" />
						</svg>
					</div>
					<div>
						<h3 class="text-sm font-semibold text-[rgb(var(--mv-text))]">Knowledge Graph</h3>
						<p class="text-xs text-[rgb(var(--mv-muted))]">{graphNodes} nodes &middot; {graphEdges} edges</p>
					</div>
					<svg class="ml-auto h-4 w-4 text-[rgb(var(--mv-muted))]" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
						<path stroke-linecap="round" stroke-linejoin="round" d="M9 5l7 7-7 7" />
					</svg>
				</a>
			{/if}

			<!-- Agent Suggestions -->
			<IntentInbox />

			<!-- Proactive Intelligence -->
			<InsightsDashboard />

			<!-- Live Agent Intelligence -->
			<AgentStream />

			<!-- Recent Notes (from briefing API) -->
			<RecentNotesWidget notes={briefing?.recent_notes ?? []} loading={briefingLoading} />

			<!-- Keyboard hint -->
			<div
				class="rounded-xl border border-[rgb(var(--mv-border))]/70 bg-[rgb(var(--mv-panel))]/50 p-4"
			>
				<p class="text-xs text-[rgb(var(--mv-muted))]">
					<kbd
						class="rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))] px-1 py-0.5 text-xs text-[rgb(var(--mv-muted))]"
						>{isMac ? 'Cmd' : 'Ctrl'}+K</kbd
					>
					to open command palette
				</p>
			</div>
		</div>
	</div>

	<!-- Activity Feed -->
	{#if activityItems.length > 0}
		<div class="mv-section">
			<h3 class="text-sm font-semibold text-[rgb(var(--mv-text))]">Recent Activity</h3>
			<div class="mt-3 divide-y divide-[rgb(var(--mv-border))]/40">
				{#each activityItems as item (item.id)}
					<div class="flex items-center gap-3 py-2.5">
						{#if item.type === 'task'}
							<div class="flex h-7 w-7 shrink-0 items-center justify-center rounded-md bg-sky-500/15">
								<svg class="h-3.5 w-3.5 text-sky-300" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
									<path stroke-linecap="round" stroke-linejoin="round" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
								</svg>
							</div>
						{:else}
							<div class="flex h-7 w-7 shrink-0 items-center justify-center rounded-md bg-violet-500/15">
								<svg class="h-3.5 w-3.5 text-violet-300" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
									<path stroke-linecap="round" stroke-linejoin="round" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
								</svg>
							</div>
						{/if}
						<div class="min-w-0 flex-1">
							<p class="truncate text-sm text-[rgb(var(--mv-text))]">{item.title}</p>
							<p class="text-xs text-[rgb(var(--mv-muted))]">
								{item.type === 'task' ? 'Task' : 'Note'}
								{#if item.status}
									<span class="mx-1">&middot;</span>
									<span class="capitalize">{item.status.replace('_', ' ')}</span>
								{/if}
							</p>
						</div>
						<span class="shrink-0 text-xs text-[rgb(var(--mv-muted))]">{formatRelativeTime(item.updated_at)}</span>
					</div>
				{/each}
			</div>
		</div>
	{/if}
</div>
