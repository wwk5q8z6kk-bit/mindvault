<script lang="ts">
	import { goto } from '$app/navigation';
	import { tasksStore, loadTasks } from '$lib/stores/tasks';
	import { loadNotes } from '$lib/stores/notes';
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

	$: allTasks = $tasksStore;
	$: inboxCount = allTasks.filter((t) => t.status === 'inbox').length;
	$: inProgressCount = allTasks.filter((t) => t.status === 'in_progress').length;

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

	onMount(() => {
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

			loaded = true;
		})();

		return unsubscribe;
	});
</script>

<div class="mx-auto max-w-5xl space-y-6">
	<!-- AI Briefing -->
	<AiBriefingWidget
		summary={briefing?.summary ?? ''}
		date={briefing?.date ?? ''}
		loading={briefingLoading}
	/>

	<!-- Stats row -->
	<div class="grid grid-cols-2 gap-3 md:grid-cols-5">
		<div class="rounded-xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/80 p-4">
			<div class="text-2xl font-bold text-[rgb(var(--mv-text))]">{allTasks.length}</div>
			<div class="text-xs text-[rgb(var(--mv-muted))]">Total Tasks</div>
		</div>
		<div class="rounded-xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/80 p-4">
			<div class="text-2xl font-bold text-sky-300">{inboxCount}</div>
			<div class="text-xs text-[rgb(var(--mv-muted))]">Inbox</div>
		</div>
		<div class="rounded-xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/80 p-4">
			<div class="text-2xl font-bold text-amber-300">{dueTodayCount}</div>
			<div class="text-xs text-[rgb(var(--mv-muted))]">Due Today</div>
		</div>
		<div class="rounded-xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/80 p-4">
			<div
				class="text-2xl font-bold {overdueCount > 0
					? 'text-red-400'
					: 'text-[rgb(var(--mv-text))]'}"
			>
				{overdueCount}
			</div>
			<div class="text-xs text-[rgb(var(--mv-muted))]">Overdue</div>
		</div>
		<div class="rounded-xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/80 p-4">
			<div class="text-2xl font-bold text-emerald-300">{doneThisWeek}</div>
			<div class="text-xs text-[rgb(var(--mv-muted))]">Done This Week</div>
		</div>
	</div>

	<div class="grid gap-6 lg:grid-cols-3">
		<!-- Due Today + Overdue + In Progress -->
		<div class="lg:col-span-2 space-y-4">
			<DueTasksWidget
				tasks={briefing?.due_today ?? []}
				title="Due Today"
				loading={briefingLoading}
			/>

			{#if (briefing?.overdue ?? []).length > 0}
				<DueTasksWidget tasks={briefing?.overdue ?? []} title="Overdue" loading={briefingLoading} />
			{/if}

			<!-- In Progress -->
			{#if inProgressCount > 0}
				<div
					class="rounded-xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/80 p-5"
				>
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
			<WhatsNextWidget />

			<!-- Habits -->
			<HabitsWidget habits={briefing?.habits_today ?? []} loading={briefingLoading} />

			<!-- Quick actions -->
			<div
				class="rounded-xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/80 p-5"
			>
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
						>Cmd+K</kbd
					>
					to open command palette
				</p>
			</div>
		</div>
	</div>
</div>
