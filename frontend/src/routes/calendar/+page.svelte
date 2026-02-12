<script lang="ts">
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	onMount(() => {
		goto('/tasks?view=calendar', { replaceState: true });
	});
</script>
