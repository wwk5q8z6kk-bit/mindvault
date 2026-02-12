<script lang="ts">
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	onMount(() => { goto('/bookmarks?view=flashcards', { replaceState: true }); });
</script>
