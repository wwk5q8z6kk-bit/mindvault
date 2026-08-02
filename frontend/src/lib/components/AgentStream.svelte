<script lang="ts">
	import { agentStore } from '$lib/api/agent';
	import { slide } from 'svelte/transition';

	// Connection lifecycle is owned by `+layout.svelte`. This component must
	// not disconnect on unmount — leaving the home page used to tear down the
	// global `/ws/agent` socket and starve work-order live updates.

	$: ({ summary, relatedNodes } = $agentStore);
</script>

<div
	class="relative overflow-hidden rounded-[var(--mv-radius)] border border-sky-500/20 bg-[rgb(var(--mv-panel))]/40 p-5 backdrop-blur-2xl shadow-lg transition-all duration-300 hover:border-sky-500/40 hover:bg-[rgb(var(--mv-panel))]/60"
>
	<div
		class="absolute -right-10 -top-10 h-32 w-32 rounded-full bg-sky-500/10 blur-2xl pointer-events-none"
	></div>

	<div class="relative z-10 flex items-center gap-3 mb-4">
		<div class="relative flex h-3 w-3">
			<span
				class="absolute inline-flex h-full w-full animate-ping rounded-full bg-sky-400 opacity-75"
			></span>
			<span class="relative inline-flex h-3 w-3 rounded-full bg-sky-500"></span>
		</div>
		<h3
			class="text-xs font-bold uppercase tracking-widest text-transparent bg-clip-text bg-gradient-to-r from-sky-400 to-indigo-400"
		>
			Agent Intelligence
		</h3>
	</div>

	<p class="relative z-10 text-sm font-medium text-[rgb(var(--mv-muted))] leading-relaxed mb-5">
		{summary}
	</p>

	{#if relatedNodes.length > 0}
		<div class="relative z-10 space-y-3">
			<h4 class="text-[10px] font-bold text-[rgb(var(--mv-muted))]/60 uppercase tracking-widest">
				Related Context
			</h4>
			{#each relatedNodes as node (node.id)}
				<div
					transition:slide
					class="group relative rounded-xl border border-white/5 bg-[rgb(var(--mv-panel-strong))]/40 p-3 hover:border-sky-500/30 hover:bg-[rgb(var(--mv-panel-strong))]/60 transition-all duration-300"
				>
					<a href={`/notes?note=${node.id}`} class="block">
						<div
							class="text-sm font-semibold text-[rgb(var(--mv-text))] group-hover:text-sky-300 transition-colors truncate"
						>
							{node.title || 'Untitled'}
						</div>
						<div class="mt-1 text-[10px] font-medium text-[rgb(var(--mv-muted))]/80">
							Updated {new Date(node.updated_at).toLocaleDateString()}
						</div>
					</a>
				</div>
			{/each}
		</div>
	{/if}
</div>
