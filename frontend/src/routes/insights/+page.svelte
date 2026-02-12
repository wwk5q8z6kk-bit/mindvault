<script lang="ts">
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	onMount(() => { goto('/review?view=insights', { replaceState: true }); });
</script>
