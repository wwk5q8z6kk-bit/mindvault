<script lang="ts">
	export let size: 'sm' | 'md' | 'lg' = 'md';
	export let label = 'Loading...';

	const sizes: Record<string, string> = {
		sm: 'h-4 w-4 border-2',
		md: 'h-6 w-6 border-2',
		lg: 'h-10 w-10 border-3'
	};
</script>

<div class="flex items-center justify-center gap-2" role="status" aria-label={label}>
	<div
		class="animate-spin rounded-full border-[rgb(var(--mv-accent))]/30 border-t-[rgb(var(--mv-accent))] {sizes[size]}"
	></div>
	{#if $$slots.default}
		<span class="text-sm text-[rgb(var(--mv-muted))]"><slot /></span>
	{/if}
</div>
