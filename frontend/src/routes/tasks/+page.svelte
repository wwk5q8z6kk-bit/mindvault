<script lang="ts">
	import ViewToggle from '$lib/components/ViewToggle.svelte';
	import { createViewMode, setViewMode } from '$lib/utils/view-mode';
	import { setViewPreference } from '$lib/stores/view-preferences';
	import TasksListView from '$lib/components/TasksListView.svelte';
	import TasksKanbanView from '$lib/components/TasksKanbanView.svelte';
	import TasksCalendarView from '$lib/components/TasksCalendarView.svelte';
	import TasksTimelineView from '$lib/components/TasksTimelineView.svelte';
	import { fade } from 'svelte/transition';

	const currentView = createViewMode('list', ['list', 'kanban', 'calendar', 'timeline']);

	const views = [
		{
			key: 'list',
			label: 'List',
			icon: '<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 10h16M4 14h16M4 18h16" />'
		},
		{
			key: 'kanban',
			label: 'Kanban',
			icon: '<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 17V7m0 10a2 2 0 01-2 2H5a2 2 0 01-2-2V7a2 2 0 012-2h2a2 2 0 012 2m0 10a2 2 0 002 2h2a2 2 0 002-2M9 7a2 2 0 012-2h2a2 2 0 012 2m0 10V7m0 10a2 2 0 002 2h2a2 2 0 002-2V7a2 2 0 00-2-2h-2a2 2 0 00-2 2" />'
		},
		{
			key: 'calendar',
			label: 'Calendar',
			icon: '<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7V3m8 4V3m-9 8h10M5 21h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z" />'
		},
		{
			key: 'timeline',
			label: 'Timeline',
			icon: '<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />'
		}
	];

	function handleViewChange(event: CustomEvent<string>) {
		const view = event.detail;
		setViewPreference('tasks', view);
		setViewMode(view, 'list');
	}
</script>

<div class="flex flex-col gap-6">
	<div
		class="relative flex flex-col gap-4 overflow-hidden rounded-[var(--mv-radius)] border border-white/5 bg-[rgb(var(--mv-panel))]/40 p-6 backdrop-blur-xl shadow-lg sm:flex-row sm:items-center sm:justify-between"
	>
		<div
			class="absolute -left-20 -top-20 h-40 w-40 rounded-full bg-[rgb(var(--mv-accent))]/10 blur-3xl pointer-events-none"
		></div>

		<div class="relative z-10">
			<h2
				class="text-3xl font-extrabold tracking-tight text-transparent bg-clip-text bg-gradient-to-r from-[rgb(var(--mv-text))] to-[rgb(var(--mv-muted))]"
			>
				Tasks
			</h2>
			<p class="mt-1 text-sm font-medium text-[rgb(var(--mv-muted))]/80">
				Manage, prioritize, and conquer your day.
			</p>
		</div>
		<div class="relative z-10">
			<ViewToggle {views} activeView={$currentView} on:change={handleViewChange} />
		</div>
	</div>

	<div class="grid w-full items-start mt-2">
		{#if $currentView === 'list'}
			<div
				in:fade={{ duration: 300, delay: 150 }}
				out:fade={{ duration: 150 }}
				class="col-start-1 row-start-1 w-full min-w-0"
			>
				<TasksListView />
			</div>
		{:else if $currentView === 'kanban'}
			<div
				in:fade={{ duration: 300, delay: 150 }}
				out:fade={{ duration: 150 }}
				class="col-start-1 row-start-1 w-full min-w-0"
			>
				<TasksKanbanView />
			</div>
		{:else if $currentView === 'calendar'}
			<div
				in:fade={{ duration: 300, delay: 150 }}
				out:fade={{ duration: 150 }}
				class="col-start-1 row-start-1 w-full min-w-0"
			>
				<TasksCalendarView />
			</div>
		{:else if $currentView === 'timeline'}
			<div
				in:fade={{ duration: 300, delay: 150 }}
				out:fade={{ duration: 150 }}
				class="col-start-1 row-start-1 w-full min-w-0"
			>
				<TasksTimelineView />
			</div>
		{/if}
	</div>
</div>
