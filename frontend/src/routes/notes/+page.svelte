<script lang="ts">
	import { page } from '$app/stores';
	import NotesAdvancedWorkspace from '$lib/components/NotesAdvancedWorkspace.svelte';
	import NotesWorkbench from '$lib/components/NotesWorkbench.svelte';

	$: activeView = $page.url.searchParams.get('view') ?? 'list';
</script>

{#if activeView === 'list'}
	<NotesWorkbench />
{:else}
	<NotesAdvancedWorkspace />
{/if}
