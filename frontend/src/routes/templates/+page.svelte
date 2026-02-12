<script lang="ts">
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	onMount(() => { goto('/bookmarks?view=templates', { replaceState: true }); });
</script>
