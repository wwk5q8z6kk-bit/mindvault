<script lang="ts">
	import ViewToggle from '$lib/components/ViewToggle.svelte';
	import { createViewMode, setViewMode } from '$lib/utils/view-mode';
	import { setViewPreference } from '$lib/stores/view-preferences';
	import TasksListView from '$lib/components/TasksListView.svelte';
	import TasksKanbanView from '$lib/components/TasksKanbanView.svelte';
	import TasksCalendarView from '$lib/components/TasksCalendarView.svelte';
	import TasksTimelineView from '$lib/components/TasksTimelineView.svelte';

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

<div class="flex flex-col gap-4">
	<div class="flex items-center justify-between">
		<h2 class="text-lg font-semibold text-[rgb(var(--mv-text))]">Tasks</h2>
		<ViewToggle {views} activeView={$currentView} on:change={handleViewChange} />
	</div>

	{#if $currentView === 'list'}
		<TasksListView />
	{:else if $currentView === 'kanban'}
		<TasksKanbanView />
	{:else if $currentView === 'calendar'}
		<TasksCalendarView />
	{:else if $currentView === 'timeline'}
		<TasksTimelineView />
	{/if}
</div>
