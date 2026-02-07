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

	// Enhanced features
	let showMergeModal = false;
	let mergeTargetTag = '';
	let merging = false;
	let showAnalytics = false;
	let bulkAddTagInput = '';
	let bulkAddProcessing = false;

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

	// Merge tag into another
	async function mergeTag() {
		if (!selectedTag || !mergeTargetTag.trim() || selectedTag === mergeTargetTag.trim()) {
			pushToast('Select a valid target tag', 'warning');
			return;
		}

		const source = selectedTag;
		const target = mergeTargetTag.trim();
		merging = true;

		try {
			let count = 0;

			// Update tasks
			for (const task of $tasksStore) {
				const labels = task.labels ?? [];
				if (labels.includes(source)) {
					const newLabels = labels
						.filter((l) => l !== source)
						.concat(labels.includes(target) ? [] : [target]);
					await updateTaskOptimistic(task.id, { labels: [...new Set(newLabels)] });
					count++;
				}
			}

			// Update notes
			for (const note of $notesStore) {
				const noteTags = note.tags ?? [];
				if (noteTags.includes(source)) {
					const newTags = noteTags
						.filter((t) => t !== source)
						.concat(noteTags.includes(target) ? [] : [target]);
					await updateNote(note.id, { tags: [...new Set(newTags)] });
					count++;
				}
			}

			await Promise.all([loadTasks(), loadNotes()]);
			selectedTag = target;
			showMergeModal = false;
			mergeTargetTag = '';
			pushToast(`Merged "${source}" into "${target}" (${count} items)`, 'success');
		} catch {
			pushToast('Merge failed', 'danger');
		} finally {
			merging = false;
		}
	}

	// Bulk add tag to selected items
	async function bulkAddTag() {
		if (!selectedTag || !bulkAddTagInput.trim()) return;
		const newTag = bulkAddTagInput.trim().toLowerCase();
		bulkAddProcessing = true;

		try {
			let count = 0;

			for (const task of selectedItems.tasks) {
				const labels = task.labels ?? [];
				if (!labels.includes(newTag)) {
					await updateTaskOptimistic(task.id, { labels: [...labels, newTag] });
					count++;
				}
			}

			for (const note of selectedItems.notes) {
				const noteTags = note.tags ?? [];
				if (!noteTags.includes(newTag)) {
					await updateNote(note.id, { tags: [...noteTags, newTag] });
					count++;
				}
			}

			await Promise.all([loadTasks(), loadNotes()]);
			bulkAddTagInput = '';
			pushToast(`Added "${newTag}" to ${count} items`, 'success');
		} catch {
			pushToast('Bulk add failed', 'danger');
		} finally {
			bulkAddProcessing = false;
		}
	}

	// Analytics computations
	$: tagAnalytics = (() => {
		const orphanedTags = tags.filter((t) => t.total === 1);
		const taskOnlyTags = tags.filter((t) => t.noteCount === 0 && t.taskCount > 0);
		const noteOnlyTags = tags.filter((t) => t.taskCount === 0 && t.noteCount > 0);
		const topTags = [...tags].sort((a, b) => b.total - a.total).slice(0, 5);
		const avgItemsPerTag = tags.length > 0 ? tags.reduce((sum, t) => sum + t.total, 0) / tags.length : 0;

		return {
			totalTags: tags.length,
			orphanedTags,
			taskOnlyTags,
			noteOnlyTags,
			topTags,
			avgItemsPerTag: avgItemsPerTag.toFixed(1)
		};
	})();
</script>

<div class="grid gap-6 lg:grid-cols-12">
	<section class="lg:col-span-4">
		<div class="flex items-center justify-between">
			<div>
				<h2 class="text-lg font-semibold text-white">Tags</h2>
				<p class="text-xs text-slate-400">{tags.length} tags across all items</p>
			</div>
			<button
				class={`rounded-lg border px-3 py-1.5 text-xs transition ${
					showAnalytics
						? 'border-sky-500 bg-sky-500/20 text-sky-300'
						: 'border-slate-700 text-slate-300 hover:bg-slate-800'
				}`}
				on:click={() => (showAnalytics = !showAnalytics)}
			>
				Analytics
			</button>
		</div>

		{#if showAnalytics}
			<div class="mt-3 rounded-lg border border-slate-700 bg-slate-800/50 p-3">
				<h3 class="text-[10px] font-semibold uppercase tracking-wide text-slate-500">Tag Analytics</h3>
				<div class="mt-2 grid grid-cols-2 gap-2 text-xs">
					<div class="rounded-lg bg-slate-900/50 p-2">
						<div class="text-lg font-bold text-white">{tagAnalytics.totalTags}</div>
						<div class="text-[10px] text-slate-500">Total tags</div>
					</div>
					<div class="rounded-lg bg-slate-900/50 p-2">
						<div class="text-lg font-bold text-white">{tagAnalytics.avgItemsPerTag}</div>
						<div class="text-[10px] text-slate-500">Avg items/tag</div>
					</div>
				</div>

				{#if tagAnalytics.orphanedTags.length > 0}
					<div class="mt-2">
						<span class="text-[10px] text-amber-400">{tagAnalytics.orphanedTags.length} single-use tags:</span>
						<div class="mt-1 flex flex-wrap gap-1">
							{#each tagAnalytics.orphanedTags.slice(0, 5) as tag}
								<span class="rounded bg-amber-500/10 px-1.5 py-0.5 text-[9px] text-amber-300">{tag.name}</span>
							{/each}
							{#if tagAnalytics.orphanedTags.length > 5}
								<span class="text-[9px] text-slate-500">+{tagAnalytics.orphanedTags.length - 5} more</span>
							{/if}
						</div>
					</div>
				{/if}

				<div class="mt-2">
					<span class="text-[10px] text-slate-400">Top tags:</span>
					<div class="mt-1 flex flex-wrap gap-1">
						{#each tagAnalytics.topTags as tag}
							<button
								class="rounded bg-sky-500/10 px-1.5 py-0.5 text-[9px] text-sky-300 hover:bg-sky-500/20"
								on:click={() => { selectedTag = tag.name; summary = ''; }}
							>
								{tag.name} ({tag.total})
							</button>
						{/each}
					</div>
				</div>
			</div>
		{/if}

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
				<div class="flex flex-wrap gap-2">
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
						class="rounded-lg border border-purple-500/30 px-3 py-1.5 text-xs text-purple-300 hover:bg-purple-500/10"
						on:click={() => (showMergeModal = true)}
					>
						Merge Into
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

			<!-- Bulk add another tag -->
			<div class="mt-4 flex items-center gap-2">
				<span class="text-xs text-slate-500">Add tag to all items:</span>
				<input
					class="flex-1 rounded-lg border border-slate-700 bg-slate-800 px-2 py-1 text-xs text-white placeholder-slate-500"
					placeholder="New tag..."
					bind:value={bulkAddTagInput}
					on:keydown={(e) => e.key === 'Enter' && bulkAddTag()}
				/>
				<button
					class="rounded-lg bg-sky-500 px-3 py-1 text-xs font-semibold text-white hover:bg-sky-400 disabled:opacity-50"
					on:click={bulkAddTag}
					disabled={bulkAddProcessing || !bulkAddTagInput.trim()}
				>
					{bulkAddProcessing ? 'Adding...' : 'Add'}
				</button>
			</div>

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
					<p class="mt-1 text-xs text-slate-400">Click a tag to view its items, rename, merge, or generate an AI summary.</p>
				</div>
			</div>
		{/if}
	</section>
</div>

<!-- Merge Modal -->
{#if showMergeModal && selectedTag}
	<div class="fixed inset-0 z-50 flex items-center justify-center p-4">
		<div
			class="absolute inset-0 bg-black/60 backdrop-blur-sm"
			on:click={() => (showMergeModal = false)}
			on:keydown={(e) => e.key === 'Escape' && (showMergeModal = false)}
			role="button"
			tabindex="-1"
			aria-label="Close"
		></div>
		<div class="relative z-10 w-full max-w-md rounded-xl border border-slate-700 bg-slate-900 p-6 shadow-xl">
			<h3 class="text-sm font-semibold text-white">Merge Tag</h3>
			<p class="mt-1 text-xs text-slate-400">
				Merge <span class="font-medium text-sky-300">#{selectedTag}</span> into another tag.
				All items will be moved to the target tag.
			</p>

			<div class="mt-4">
				<label for="merge-target" class="text-xs text-slate-500">Target tag:</label>
				<div class="mt-1 flex gap-2">
					<input
						id="merge-target"
						class="flex-1 rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white placeholder-slate-500"
						placeholder="Enter or select target tag..."
						bind:value={mergeTargetTag}
						list="existing-tags"
					/>
					<datalist id="existing-tags">
						{#each tags.filter((t) => t.name !== selectedTag) as tag}
							<option value={tag.name}>{tag.name} ({tag.total})</option>
						{/each}
					</datalist>
				</div>
			</div>

			<div class="mt-2 flex flex-wrap gap-1">
				<span class="text-[10px] text-slate-500">Suggestions:</span>
				{#each tags.filter((t) => t.name !== selectedTag).slice(0, 6) as tag}
					<button
						class="rounded bg-slate-800 px-1.5 py-0.5 text-[10px] text-slate-300 hover:bg-slate-700"
						on:click={() => (mergeTargetTag = tag.name)}
					>
						{tag.name}
					</button>
				{/each}
			</div>

			<div class="mt-6 flex justify-end gap-2">
				<button
					class="rounded-lg border border-slate-700 px-4 py-2 text-xs text-slate-300 hover:bg-slate-800"
					on:click={() => (showMergeModal = false)}
				>
					Cancel
				</button>
				<button
					class="rounded-lg bg-purple-500 px-4 py-2 text-xs font-semibold text-white hover:bg-purple-400 disabled:opacity-50"
					on:click={mergeTag}
					disabled={merging || !mergeTargetTag.trim() || mergeTargetTag === selectedTag}
				>
					{merging ? 'Merging...' : 'Merge'}
				</button>
			</div>
		</div>
	</div>
{/if}
