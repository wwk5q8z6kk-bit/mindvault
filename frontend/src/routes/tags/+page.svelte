<script lang="ts">
	import { onMount } from 'svelte';
	import { tasksStore, loadTasks, updateTaskOptimistic } from '$lib/stores/tasks';
	import { notesStore, loadNotes } from '$lib/stores/notes';
	import { updateNote } from '$lib/api/notes';
	import { assistTransform } from '$lib/api/assist';
	import { pushToast } from '$lib/stores/toast';

	type TagInfo = {
		name: string;
		taskCount: number;
		noteCount: number;
		total: number;
	};

	let tags: TagInfo[] = [];
	let loading = true;
	let searchQuery = '';
	let selectedTag: string | null = null;
	let summarizing = false;
	let summary = '';

	$: {
		const tagMap = new Map<string, TagInfo>();
		for (const task of $tasksStore) {
			for (const label of task.labels ?? []) {
				const existing = tagMap.get(label) ?? { name: label, taskCount: 0, noteCount: 0, total: 0 };
				existing.taskCount++;
				existing.total++;
				tagMap.set(label, existing);
			}
		}
		for (const note of $notesStore) {
			for (const tag of note.tags ?? []) {
				const existing = tagMap.get(tag) ?? { name: tag, taskCount: 0, noteCount: 0, total: 0 };
				existing.noteCount++;
				existing.total++;
				tagMap.set(tag, existing);
			}
		}
		tags = [...tagMap.values()].sort((a, b) => b.total - a.total);
	}

	$: filteredTags = searchQuery.trim()
		? tags.filter((t) => t.name.toLowerCase().includes(searchQuery.toLowerCase()))
		: tags;

	$: selectedItems = selectedTag
		? {
				tasks: $tasksStore.filter((t) => (t.labels ?? []).includes(selectedTag!)),
				notes: $notesStore.filter((n) => (n.tags ?? []).includes(selectedTag!))
			}
		: { tasks: [], notes: [] };

	onMount(async () => {
		await Promise.all([loadTasks(), loadNotes()]);
		loading = false;
	});

	async function renameTag(oldName: string) {
		const newName = prompt(`Rename tag "${oldName}" to:`, oldName);
		if (!newName?.trim() || newName === oldName) return;
		const trimmed = newName.trim();

		try {
			let count = 0;
			for (const task of $tasksStore) {
				const labels = task.labels ?? [];
				if (labels.includes(oldName)) {
					const newLabels = labels.map((l) => (l === oldName ? trimmed : l));
					await updateTaskOptimistic(task.id, { labels: newLabels });
					count++;
				}
			}
			for (const note of $notesStore) {
				const noteTags = note.tags ?? [];
				if (noteTags.includes(oldName)) {
					const newTags = noteTags.map((t) => (t === oldName ? trimmed : t));
					await updateNote(note.id, { tags: newTags });
					count++;
				}
			}
			await Promise.all([loadTasks(), loadNotes()]);
			if (selectedTag === oldName) selectedTag = trimmed;
			pushToast(`Renamed "${oldName}" to "${trimmed}" on ${count} items`, 'success');
		} catch {
			pushToast('Rename failed', 'danger');
		}
	}

	async function deleteTag(tagName: string) {
		if (!confirm(`Remove tag "${tagName}" from all items?`)) return;

		try {
			let count = 0;
			for (const task of $tasksStore) {
				const labels = task.labels ?? [];
				if (labels.includes(tagName)) {
					await updateTaskOptimistic(task.id, { labels: labels.filter((l) => l !== tagName) });
					count++;
				}
			}
			for (const note of $notesStore) {
				const noteTags = note.tags ?? [];
				if (noteTags.includes(tagName)) {
					await updateNote(note.id, { tags: noteTags.filter((t) => t !== tagName) });
					count++;
				}
			}
			await Promise.all([loadTasks(), loadNotes()]);
			if (selectedTag === tagName) selectedTag = null;
			pushToast(`Removed "${tagName}" from ${count} items`, 'success');
		} catch {
			pushToast('Delete failed', 'danger');
		}
	}

	async function summarizeTag(tagName: string) {
		summarizing = true;
		summary = '';
		try {
			const items = [
				...$tasksStore
					.filter((t) => (t.labels ?? []).includes(tagName))
					.map((t) => `[Task] ${t.title}: ${t.description ?? ''}`),
				...$notesStore
					.filter((n) => (n.tags ?? []).includes(tagName))
					.map((n) => `[Note] ${n.title ?? 'Untitled'}: ${(n.markdown ?? '').slice(0, 300)}`)
			];

			if (items.length === 0) {
				summary = 'No items with this tag.';
				return;
			}

			const context = items.join('\n\n');
			const prompt = `Summarize the following collection of items tagged "${tagName}". Identify key themes, patterns, and actionable insights. Be concise (3-5 bullet points).\n\n${context}`;

			const result = await assistTransform({ text: prompt, mode: 'summarize' });
			summary = result.transformed_text;
			pushToast('Summary generated', 'success');
		} catch {
			pushToast('Summary failed. Check AI settings.', 'danger');
		} finally {
			summarizing = false;
		}
	}
</script>

<div class="grid gap-6 lg:grid-cols-12">
	<section class="lg:col-span-4">
		<div class="flex items-center justify-between">
			<div>
				<h2 class="text-lg font-semibold text-white">Tags</h2>
				<p class="text-xs text-slate-400">{tags.length} tags across all items</p>
			</div>
		</div>

		<input
			class="mt-3 w-full rounded-lg border border-slate-800 bg-slate-900 px-3 py-1.5 text-xs text-white placeholder-slate-500"
			placeholder="Search tags..."
			bind:value={searchQuery}
			aria-label="Search tags"
		/>

		<div class="mt-3 flex flex-col gap-1.5">
			{#if loading}
				<div class="rounded-lg border border-slate-800 p-4 text-xs text-slate-400">Loading...</div>
			{:else if filteredTags.length === 0}
				<div class="rounded-lg border border-dashed border-slate-800 p-4 text-xs text-slate-400">
					{searchQuery ? 'No matching tags.' : 'No tags yet.'}
				</div>
			{:else}
				{#each filteredTags as tag (tag.name)}
					<button
						class="flex items-center justify-between rounded-lg border px-3 py-2 text-left text-xs transition {selectedTag === tag.name
							? 'border-sky-500 bg-sky-500/10 text-sky-200'
							: 'border-slate-800 bg-slate-900/40 text-slate-200 hover:border-slate-700'}"
						on:click={() => { selectedTag = selectedTag === tag.name ? null : tag.name; summary = ''; }}
					>
						<span class="font-medium">{tag.name}</span>
						<div class="flex items-center gap-2">
							{#if tag.taskCount > 0}
								<span class="rounded bg-violet-500/20 px-1.5 py-0.5 text-[9px] text-violet-300">{tag.taskCount} tasks</span>
							{/if}
							{#if tag.noteCount > 0}
								<span class="rounded bg-sky-500/20 px-1.5 py-0.5 text-[9px] text-sky-300">{tag.noteCount} notes</span>
							{/if}
						</div>
					</button>
				{/each}
			{/if}
		</div>
	</section>

	<section class="lg:col-span-8">
		{#if selectedTag}
			<div class="flex items-center justify-between">
				<div>
					<h2 class="text-lg font-semibold text-white">#{selectedTag}</h2>
					<p class="text-xs text-slate-400">
						{selectedItems.tasks.length} tasks, {selectedItems.notes.length} notes
					</p>
				</div>
				<div class="flex gap-2">
					<button
						class="rounded-lg bg-gradient-to-r from-sky-500 to-violet-500 px-3 py-1.5 text-xs font-semibold text-white hover:from-sky-400 hover:to-violet-400 disabled:opacity-50"
						on:click={() => selectedTag && summarizeTag(selectedTag)}
						disabled={summarizing}
					>
						{summarizing ? 'Summarizing...' : 'AI Summary'}
					</button>
					<button
						class="rounded-lg border border-slate-700 px-3 py-1.5 text-xs text-slate-300 hover:bg-slate-800"
						on:click={() => selectedTag && renameTag(selectedTag)}
					>
						Rename
					</button>
					<button
						class="rounded-lg border border-red-500/30 px-3 py-1.5 text-xs text-red-300 hover:bg-red-500/10"
						on:click={() => selectedTag && deleteTag(selectedTag)}
					>
						Remove
					</button>
				</div>
			</div>

			{#if summary}
				<div class="mt-4 rounded-xl border border-sky-500/20 bg-sky-500/5 p-4">
					<h3 class="text-xs font-semibold text-sky-200">AI Summary</h3>
					<div class="mt-2 whitespace-pre-wrap text-xs leading-relaxed text-sky-100/80">{summary}</div>
				</div>
			{/if}

			<div class="mt-4 flex flex-col gap-2">
				{#each selectedItems.tasks as task (task.id)}
					<a
						href="/tasks?task={task.id}"
						class="rounded-lg border border-slate-800/60 bg-slate-900/40 px-3 py-2 text-xs transition hover:border-slate-700"
					>
						<div class="flex items-center gap-2">
							<span class="rounded bg-violet-500/20 px-1.5 py-0.5 text-[9px] text-violet-300">task</span>
							<span class="text-white">{task.title}</span>
							<span class="ml-auto text-[9px] text-slate-500">{task.status}</span>
						</div>
					</a>
				{/each}
				{#each selectedItems.notes as note (note.id)}
					<a
						href="/notes?note={note.id}"
						class="rounded-lg border border-slate-800/60 bg-slate-900/40 px-3 py-2 text-xs transition hover:border-slate-700"
					>
						<div class="flex items-center gap-2">
							<span class="rounded bg-sky-500/20 px-1.5 py-0.5 text-[9px] text-sky-300">note</span>
							<span class="text-white">{note.title ?? 'Untitled'}</span>
						</div>
						{#if note.markdown}
							<p class="mt-1 line-clamp-1 text-[10px] text-slate-500">{note.markdown.slice(0, 100)}</p>
						{/if}
					</a>
				{/each}
			</div>
		{:else}
			<div class="flex h-full items-center justify-center rounded-xl border border-dashed border-slate-800 p-12">
				<div class="text-center">
					<h3 class="text-sm font-semibold text-white">Select a tag</h3>
					<p class="mt-1 text-xs text-slate-400">Click a tag to view its items, rename, remove, or generate an AI summary.</p>
				</div>
			</div>
		{/if}
	</section>
</div>
