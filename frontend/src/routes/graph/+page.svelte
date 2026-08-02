<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';
	import { onMount } from 'svelte';
	onMount(() => {
		const nodeId = $page.url.searchParams.get('node');
		const destination = nodeId
			? `/notes?view=graph&node=${encodeURIComponent(nodeId)}`
			: '/notes?view=graph';
		goto(destination, { replaceState: true });
	});
</script>
