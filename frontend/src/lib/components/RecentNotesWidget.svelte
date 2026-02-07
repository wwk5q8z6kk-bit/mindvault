<script lang="ts">
	import { resolve } from '$app/paths';
	import type { BriefingNote } from '$lib/api/briefing';

	export let notes: BriefingNote[] = [];
	export let loading: boolean = false;

	function timeAgo(dateStr: string): string {
		const d = new Date(dateStr);
		const now = new Date();
		const diff = now.getTime() - d.getTime();
		const mins = Math.floor(diff / 60000);
		if (mins < 60) return `${mins}m ago`;
		const hours = Math.floor(mins / 60);
		if (hours < 24) return `${hours}h ago`;
		const days = Math.floor(hours / 24);
		return `${days}d ago`;
	}
</script>

<div class="rounded-xl border border-slate-800 bg-slate-900/40 p-5">
	<div class="flex items-center justify-between">
		<h3 class="text-sm font-semibold text-white">Recent Notes</h3>
		<a href={resolve('/notes')} class="text-[11px] text-sky-400 hover:text-sky-300">All notes</a>
	</div>
	<div class="mt-3 space-y-1.5">
		{#if loading}
			<p class="text-xs text-slate-500">Loading...</p>
		{:else if notes.length === 0}
			<p class="text-[11px] text-slate-500">No notes yet.</p>
		{:else}
			{#each notes as note (note.id)}
				<a
					href={`/notes?note=${note.id}`}
					class="block rounded-lg px-2.5 py-2 transition hover:bg-slate-800/60"
				>
					<div class="flex items-center justify-between">
						<span class="truncate text-xs font-medium text-slate-200">{note.title}</span>
						<span class="text-[10px] text-slate-500">{timeAgo(note.updated_at)}</span>
					</div>
				</a>
			{/each}
		{/if}
	</div>
</div>
