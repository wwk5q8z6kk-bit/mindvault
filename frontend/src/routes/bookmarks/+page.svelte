<script lang="ts">
	import ViewToggle from '$lib/components/ViewToggle.svelte';
	import { createViewMode, setViewMode } from '$lib/utils/view-mode';
	import { setViewPreference } from '$lib/stores/view-preferences';
	import ResourcesBookmarksView from '$lib/components/ResourcesBookmarksView.svelte';
	import ResourcesTemplatesView from '$lib/components/ResourcesTemplatesView.svelte';
	import ResourcesFlashcardsView from '$lib/components/ResourcesFlashcardsView.svelte';

	const currentView = createViewMode('bookmarks', ['bookmarks', 'templates', 'flashcards']);

	const views = [
		{ key: 'bookmarks', label: 'Bookmarks' },
		{ key: 'templates', label: 'Templates' },
		{ key: 'flashcards', label: 'Flashcards' }
	];

	function handleViewChange(event: CustomEvent<string>) {
		const view = event.detail;
		setViewPreference('resources', view);
		setViewMode(view, 'bookmarks');
	}
</script>

<div class="flex flex-col gap-4">
	<ViewToggle {views} activeView={$currentView} on:change={handleViewChange} />

	{#if $currentView === 'bookmarks'}
		<ResourcesBookmarksView />
	{:else if $currentView === 'templates'}
		<ResourcesTemplatesView />
	{:else if $currentView === 'flashcards'}
		<ResourcesFlashcardsView />
	{/if}
</div>
