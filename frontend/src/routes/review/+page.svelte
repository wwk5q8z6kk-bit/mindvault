<script lang="ts">
	import ViewToggle from '$lib/components/ViewToggle.svelte';
	import { createViewMode, setViewMode } from '$lib/utils/view-mode';
	import { setViewPreference } from '$lib/stores/view-preferences';
	import ReviewDigestView from '$lib/components/ReviewDigestView.svelte';
	import ReviewStatsView from '$lib/components/ReviewStatsView.svelte';
	import ReviewTagsView from '$lib/components/ReviewTagsView.svelte';
	import ReviewInsightsView from '$lib/components/ReviewInsightsView.svelte';

	const currentView = createViewMode('digest', ['digest', 'stats', 'tags', 'insights']);

	const reviewViews = [
		{ key: 'digest', label: 'Digest' },
		{ key: 'stats', label: 'Stats' },
		{ key: 'tags', label: 'Tags' },
		{ key: 'insights', label: 'Insights' }
	];

	function handleViewChange(event: CustomEvent<string>) {
		const view = event.detail;
		setViewPreference('review', view);
		setViewMode(view, 'digest');
	}
</script>

<div class="flex items-center justify-between">
	<h2 class="text-lg font-semibold text-[rgb(var(--mv-text))]">Review</h2>
</div>

<div class="mt-4">
	<ViewToggle
		views={reviewViews}
		activeView={$currentView}
		variant="tabs"
		size="sm"
		on:change={handleViewChange}
	/>
</div>

{#if $currentView === 'digest'}
	<div class="mt-4">
		<ReviewDigestView />
	</div>
{:else if $currentView === 'stats'}
	<div class="mt-4">
		<ReviewStatsView />
	</div>
{:else if $currentView === 'tags'}
	<div class="mt-4">
		<ReviewTagsView />
	</div>
{:else if $currentView === 'insights'}
	<div class="mt-4">
		<ReviewInsightsView />
	</div>
{/if}
