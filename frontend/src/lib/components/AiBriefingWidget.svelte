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

<div
	class="relative overflow-hidden rounded-[var(--mv-radius)] border border-white/10 bg-[rgb(var(--mv-panel))]/40 p-6 backdrop-blur-2xl shadow-lg transition-all duration-500 hover:shadow-xl hover:bg-[rgb(var(--mv-panel))]/50"
>
	<div
		class="absolute -right-20 -top-20 h-40 w-40 rounded-full bg-[rgb(var(--mv-accent))]/10 blur-3xl pointer-events-none"
	></div>

	{#if loading}
		<div class="relative z-10 animate-pulse">
			<div class="h-5 w-48 bg-[rgb(var(--mv-muted))]/20 rounded mb-3"></div>
			<div class="h-3 w-full bg-[rgb(var(--mv-muted))]/10 rounded mb-2"></div>
			<div class="h-3 w-3/4 bg-[rgb(var(--mv-muted))]/10 rounded"></div>
		</div>
	{:else}
		<div class="relative z-10 flex items-center gap-3 mb-3">
			<div
				class="flex h-10 w-10 items-center justify-center rounded-xl bg-gradient-to-br from-[rgb(var(--mv-accent))] to-[rgb(var(--mv-accent-strong))] shadow-lg shadow-[rgb(var(--mv-accent))]/20"
			>
				<span class="text-xl">👋</span>
			</div>
			<h2
				class="text-lg font-bold tracking-tight text-transparent bg-clip-text bg-gradient-to-r from-[rgb(var(--mv-text))] to-[rgb(var(--mv-muted))]"
			>
				{formatDate(date)}
			</h2>
		</div>
		<p class="relative z-10 text-sm font-medium text-[rgb(var(--mv-muted))]/90 leading-relaxed">
			{summary}
		</p>
	{/if}
</div>
