<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import type { Note } from '$lib/api/notes';

	/** The current note (to show its backlinks) */
	export let note: Note | null = null;
	/** Map of note IDs to Note objects for display */
	export let notesMap: Map<string, Note> = new Map();
	/** Whether the panel is expanded */
	export let expanded = true;

	const dispatch = createEventDispatcher<{
		navigate: { noteId: string };
		toggle: { expanded: boolean };
	}>();

	$: backlinks = note?.backlinks ?? [];
	$: backlinkNotes = backlinks
		.map((id) => notesMap.get(id))
		.filter((n): n is Note => n !== undefined);

	function toggleExpanded() {
		expanded = !expanded;
		dispatch('toggle', { expanded });
	}

	function handleNavigate(noteId: string) {
		dispatch('navigate', { noteId });
	}

	function getPreview(note: Note): string {
		const content = note.markdown || '';
		// Remove title (first heading) and get first 140 chars
		const withoutTitle = content.replace(/^#\s+[^\n]+\n*/, '');
		const preview = withoutTitle.slice(0, 140).trim();
		return preview.length < withoutTitle.length ? preview + '...' : preview;
	}

	function getTitle(note: Note): string {
		if (note.title) return note.title;
		// Extract from first heading
		const match = note.markdown?.match(/^#\s+(.+)$/m);
		return match ? match[1] : 'Untitled';
	}
</script>

{#if backlinks.length > 0}
	<div class="backlinks-panel border-t border-slate-800/60">
		<button
			class="flex w-full items-center justify-between px-4 py-2 text-left text-xs text-slate-400 hover:bg-slate-800/30"
			on:click={toggleExpanded}
		>
			<span class="flex items-center gap-2">
				<svg
					class="h-3.5 w-3.5 transition-transform"
					class:rotate-90={expanded}
					fill="none"
					viewBox="0 0 24 24"
					stroke="currentColor"
				>
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
				</svg>
				<span class="font-medium">Backlinks</span>
				<span class="rounded-full bg-slate-700 px-1.5 py-0.5 text-[10px]">{backlinks.length}</span>
			</span>
		</button>

		{#if expanded}
			<div class="max-h-48 overflow-y-auto px-4 pb-3">
				{#if backlinkNotes.length > 0}
					<div class="space-y-2">
						{#each backlinkNotes as linkedNote (linkedNote.id)}
							<button
								class="group flex w-full flex-col gap-0.5 rounded-lg border border-slate-800 bg-slate-900/50 p-2 text-left transition hover:border-slate-700 hover:bg-slate-800/50"
								on:click={() => handleNavigate(linkedNote.id)}
							>
								<span class="text-xs font-medium text-slate-200 group-hover:text-white">
									{getTitle(linkedNote)}
								</span>
								<span class="text-[10px] leading-relaxed text-slate-500">
									{getPreview(linkedNote)}
								</span>
							</button>
						{/each}
					</div>
				{:else}
					<div class="text-[10px] text-slate-500">
						{backlinks.length} backlink{backlinks.length === 1 ? '' : 's'} (notes not loaded)
					</div>
				{/if}
			</div>
		{/if}
	</div>
{/if}

<style>
	.backlinks-panel {
		background: rgba(15, 23, 42, 0.3);
	}

	.rotate-90 {
		transform: rotate(90deg);
	}
</style>
