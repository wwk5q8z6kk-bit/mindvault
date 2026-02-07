<script lang="ts">
	export let summary: string = '';
	export let date: string = '';
	export let loading: boolean = false;

	function formatDate(dateStr: string): string {
		if (!dateStr) return '';
		const d = new Date(dateStr + 'T00:00:00');
		return d.toLocaleDateString(undefined, {
			weekday: 'long',
			month: 'long',
			day: 'numeric'
		});
	}
</script>

<div class="rounded-xl border border-slate-800 bg-gradient-to-r from-sky-900/30 to-violet-900/30 p-5">
	{#if loading}
		<div class="animate-pulse">
			<div class="h-4 w-32 bg-slate-700 rounded mb-2"></div>
			<div class="h-3 w-full bg-slate-700 rounded"></div>
		</div>
	{:else}
		<div class="flex items-center gap-2 mb-2">
			<span class="text-lg">👋</span>
			<h2 class="text-sm font-semibold text-white">{formatDate(date)}</h2>
		</div>
		<p class="text-xs text-slate-300 leading-relaxed">{summary}</p>
	{/if}
</div>
