<script lang="ts">
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	onMount(() => { goto('/notes?view=pdf', { replaceState: true }); });
</script>
