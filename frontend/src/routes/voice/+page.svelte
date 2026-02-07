<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import VoiceRecorder from '$lib/components/VoiceRecorder.svelte';
	import { listNodes } from '$lib/api/nodes';
	import { deleteNote } from '$lib/api/notes';
	import { formatDuration, type VoiceUploadResponse } from '$lib/api/voice';
	import { pushToast } from '$lib/stores/toast';
	import type { KnowledgeNode } from '$lib/api/types';

	let voiceNotes: KnowledgeNode[] = [];
	let loading = true;
	let selectedNote: KnowledgeNode | null = null;
	let searchQuery = '';
	let sortBy: 'newest' | 'oldest' | 'duration' = 'newest';

	onMount(async () => {
		await loadVoiceNotes();
	});

	async function loadVoiceNotes() {
		loading = true;
		try {
			// Voice notes are stored as facts with voice metadata
			const allNotes = await listNodes({ kind: 'fact', limit: 500 });
			voiceNotes = allNotes.filter((n) => n.metadata?.voice_duration || n.metadata?.transcript);
		} catch {
			pushToast('Failed to load voice notes', 'danger');
		} finally {
			loading = false;
		}
	}

	function handleRecordingSuccess(event: CustomEvent<VoiceUploadResponse>) {
		pushToast('Voice note created', 'success');
		void loadVoiceNotes();
	}

	async function handleDelete(note: KnowledgeNode) {
		if (!confirm(`Delete "${note.title || 'voice note'}"?`)) return;
		try {
			await deleteNote(note.id);
			pushToast('Deleted', 'success');
			if (selectedNote?.id === note.id) selectedNote = null;
			await loadVoiceNotes();
		} catch {
			pushToast('Failed to delete', 'danger');
		}
	}

	function openInNotes(note: KnowledgeNode) {
		goto(`/notes?note=${note.id}`);
	}

	function formatDate(isoDate: string): string {
		const d = new Date(isoDate);
		return d.toLocaleDateString('en-US', {
			month: 'short',
			day: 'numeric',
			year: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		});
	}

	function getDuration(note: KnowledgeNode): number {
		return (note.metadata?.voice_duration as number) ?? 0;
	}

	$: filteredNotes = (() => {
		let result = voiceNotes;

		if (searchQuery.trim()) {
			const q = searchQuery.toLowerCase();
			result = result.filter(
				(n) =>
					(n.title?.toLowerCase().includes(q)) ||
					(n.content?.toLowerCase().includes(q))
			);
		}

		// Sort
		if (sortBy === 'newest') {
			result = [...result].sort((a, b) =>
				new Date(b.temporal.created_at).getTime() - new Date(a.temporal.created_at).getTime()
			);
		} else if (sortBy === 'oldest') {
			result = [...result].sort((a, b) =>
				new Date(a.temporal.created_at).getTime() - new Date(b.temporal.created_at).getTime()
			);
		} else if (sortBy === 'duration') {
			result = [...result].sort((a, b) => getDuration(b) - getDuration(a));
		}

		return result;
	})();

	$: totalDuration = voiceNotes.reduce((sum, n) => sum + getDuration(n), 0);
</script>

<div class="grid gap-6 lg:grid-cols-12">
	<section class="lg:col-span-4">
		<div class="flex items-center justify-between">
			<div>
				<h2 class="text-lg font-semibold text-white">Voice Notes</h2>
				<p class="text-xs text-slate-400">
					{voiceNotes.length} recordings, {formatDuration(totalDuration)} total
				</p>
			</div>
		</div>

		<!-- Voice Recorder -->
		<div class="mt-4 rounded-xl border border-slate-800 bg-slate-900/60 p-4">
			<h3 class="mb-3 text-xs font-semibold uppercase tracking-wide text-slate-500">
				New Recording
			</h3>
			<VoiceRecorder on:success={handleRecordingSuccess} />
		</div>

		<!-- Search and Sort -->
		<div class="mt-4 flex gap-2">
			<input
				class="flex-1 rounded-lg border border-slate-700 bg-slate-900 px-3 py-1.5 text-xs text-white placeholder-slate-500"
				placeholder="Search transcripts..."
				bind:value={searchQuery}
			/>
			<select
				class="rounded-lg border border-slate-700 bg-slate-900 px-2 py-1.5 text-xs text-white"
				bind:value={sortBy}
			>
				<option value="newest">Newest</option>
				<option value="oldest">Oldest</option>
				<option value="duration">Duration</option>
			</select>
		</div>

		<!-- Notes List -->
		<div class="mt-4 flex flex-col gap-2">
			{#if loading}
				<div class="rounded-lg border border-slate-800 p-4 text-xs text-slate-400">
					Loading voice notes...
				</div>
			{:else if filteredNotes.length === 0}
				<div class="rounded-xl border border-dashed border-slate-800 bg-slate-900/20 p-6 text-center">
					<div class="mx-auto mb-3 flex h-10 w-10 items-center justify-center rounded-full bg-red-500/20 text-red-300">
						<svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11a7 7 0 01-7 7m0 0a7 7 0 01-7-7m7 7v4m0 0H8m4 0h4m-4-8a3 3 0 01-3-3V5a3 3 0 116 0v6a3 3 0 01-3 3z" />
						</svg>
					</div>
					<h3 class="text-sm font-medium text-white">
						{searchQuery ? 'No matches' : 'No voice notes yet'}
					</h3>
					<p class="mt-1 text-[11px] text-slate-500">
						{searchQuery ? 'Try a different search' : 'Record or upload audio to get started'}
					</p>
				</div>
			{:else}
				{#each filteredNotes as note (note.id)}
					<button
						class={`flex items-start gap-3 rounded-lg border p-3 text-left transition ${
							selectedNote?.id === note.id
								? 'border-sky-500 bg-sky-500/10'
								: 'border-slate-800 bg-slate-900/40 hover:border-slate-700'
						}`}
						on:click={() => (selectedNote = note)}
					>
						<div class="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-red-500/20 text-red-300">
							<svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
								<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11a7 7 0 01-7 7m0 0a7 7 0 01-7-7m7 7v4m0 0H8m4 0h4m-4-8a3 3 0 01-3-3V5a3 3 0 116 0v6a3 3 0 01-3 3z" />
							</svg>
						</div>
						<div class="min-w-0 flex-1">
							<div class="flex items-center gap-2">
								<span class="truncate text-sm font-medium text-white">
									{note.title || 'Untitled Recording'}
								</span>
								{#if getDuration(note) > 0}
									<span class="shrink-0 rounded bg-slate-800 px-1.5 py-0.5 text-[10px] text-slate-400">
										{formatDuration(getDuration(note))}
									</span>
								{/if}
							</div>
							<p class="mt-0.5 line-clamp-2 text-[11px] text-slate-400">
								{note.content?.slice(0, 100) || 'No transcript'}
							</p>
							<div class="mt-1 text-[10px] text-slate-500">
								{formatDate(note.temporal.created_at)}
							</div>
						</div>
					</button>
				{/each}
			{/if}
		</div>
	</section>

	<section class="lg:col-span-8">
		{#if selectedNote}
			<div class="flex items-center justify-between">
				<div>
					<h2 class="text-lg font-semibold text-white">
						{selectedNote.title || 'Untitled Recording'}
					</h2>
					<p class="text-xs text-slate-400">
						{formatDate(selectedNote.temporal.created_at)}
						{#if getDuration(selectedNote) > 0}
							• {formatDuration(getDuration(selectedNote))}
						{/if}
					</p>
				</div>
				<div class="flex gap-2">
					<button
						class="rounded-lg border border-slate-700 px-3 py-2 text-xs text-slate-300 hover:bg-slate-800"
						on:click={() => selectedNote && openInNotes(selectedNote)}
					>
						Edit in Notes
					</button>
					<button
						class="rounded-lg border border-red-500/30 px-3 py-2 text-xs text-red-300 hover:bg-red-500/10"
						on:click={() => selectedNote && handleDelete(selectedNote)}
					>
						Delete
					</button>
				</div>
			</div>

			{#if selectedNote.tags && selectedNote.tags.length > 0}
				<div class="mt-3 flex flex-wrap gap-1">
					{#each selectedNote.tags as tag}
						<span class="rounded bg-slate-800 px-2 py-0.5 text-[10px] text-slate-400">{tag}</span>
					{/each}
				</div>
			{/if}

			<div class="mt-4 rounded-xl border border-slate-800 bg-slate-900/60 p-4">
				<h3 class="text-xs font-semibold uppercase tracking-wide text-slate-500">Transcript</h3>
				<div class="mt-3 whitespace-pre-wrap text-sm leading-relaxed text-slate-200">
					{selectedNote.content || 'No transcript available'}
				</div>
			</div>

			{#if selectedNote.metadata?.voice_provider}
				<div class="mt-4 text-[10px] text-slate-600">
					Transcribed via {selectedNote.metadata.voice_provider}
					{#if selectedNote.metadata.voice_language}
						• Language: {selectedNote.metadata.voice_language}
					{/if}
				</div>
			{/if}
		{:else}
			<div class="flex h-full items-center justify-center rounded-xl border border-dashed border-slate-800 p-12">
				<div class="text-center">
					<div class="mx-auto mb-4 flex h-12 w-12 items-center justify-center rounded-full bg-red-500/20 text-red-300">
						<svg class="h-6 w-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11a7 7 0 01-7 7m0 0a7 7 0 01-7-7m7 7v4m0 0H8m4 0h4m-4-8a3 3 0 01-3-3V5a3 3 0 116 0v6a3 3 0 01-3 3z" />
						</svg>
					</div>
					<h3 class="text-sm font-semibold text-white">Select a voice note</h3>
					<p class="mt-1 text-xs text-slate-400">
						Click on a recording to view its transcript
					</p>
				</div>
			</div>
		{/if}
	</section>
</div>
