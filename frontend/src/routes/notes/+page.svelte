<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import RichNoteEditor from '$lib/components/RichNoteEditor.svelte';
	import { recentItems } from '$lib/stores/recent';
	import BacklinksPanel from '$lib/components/BacklinksPanel.svelte';
	import SuggestedConnections from '$lib/components/SuggestedConnections.svelte';
	import TemplatePicker from '$lib/components/TemplatePicker.svelte';
	import { applyTemplate } from '$lib/api/templates';
	import type { KnowledgeNode } from '$lib/api/types';
	import {
		badgeClass,
		buildAttachmentEmbedMarkdown,
		classifyAttachment,
		extractionBadge,
		filterAttachmentChunks,
		formatExtractionStatus
	} from '$lib/utils/attachments';
	import { createNote, listNotes, updateNote, deleteNote } from '$lib/api/notes';
	import { assistAutoTag, assistTransform } from '$lib/api/assist';
	import { createTaskOptimistic } from '$lib/stores/tasks';
	import { buildTaskPayloadsFromActionItems, parseActionItems } from '$lib/tasks/action-items';
	import {
		attachmentDownloadUrl,
		attachmentInlineUrl,
		deleteNodeAttachment,
		formatFileSize,
		getAttachmentChunks,
		listNodeAttachments,
		uploadNodeAttachment,
		type AttachmentChunkListResponse,
		type NodeAttachment
	} from '$lib/api/files';
	import type { Note } from '$lib/api/notes';
	import { pushToast } from '$lib/stores/toast';
	import VersionHistory from '$lib/components/VersionHistory.svelte';

	let notes: Note[] = [];
	let selectedNote: Note | null = null;
	let title = '';
	let markdown = '';
	let loading = false;
	let saving = false;
	let showVersionHistory = false;
	let attachments: NodeAttachment[] = [];
	let attachmentsLoading = false;
	let attachmentUploadPending = false;
	let attachmentUploadProgress = 0;
	let attachmentFileInput: HTMLInputElement | null = null;
	let attachmentMarker: string | null = null;
	let searchQuery = '';
	let tagFilter = '';
	let autoTagging = false;
	let extractingActionItems = false;
	let suggestedTags: string[] = [];

	// Bulk selection state
	let selectedNoteIds: Set<string> = new Set();
	let bulkMode = false;
	let bulkTagInput = '';
	let bulkProcessing = false;

	$: selectedCount = selectedNoteIds.size;
	$: allDisplayedSelected = displayedNotes.length > 0 && displayedNotes.every((n) => selectedNoteIds.has(n.id));

	const ATTACHMENT_CHUNK_PAGE_SIZE = 6;
	const ATTACHMENT_CHUNK_MAX_PAGES = 12;

	type AttachmentChunkState = {
		open: boolean;
		loading: boolean;
		error?: string;
		data?: AttachmentChunkListResponse;
		page: number;
		query: string;
	};

	let attachmentChunkState: Record<string, AttachmentChunkState> = {};

	$: allTags = [...new Set(notes.flatMap((n) => n.tags ?? []))].sort();
	$: displayedNotes = notes.filter((n) => {
		if (searchQuery.trim()) {
			const q = searchQuery.toLowerCase();
			const matchTitle = (n.title ?? '').toLowerCase().includes(q);
			const matchContent = (n.markdown ?? '').toLowerCase().includes(q);
			if (!matchTitle && !matchContent) return false;
		}
		if (tagFilter && !(n.tags ?? []).includes(tagFilter)) return false;
		return true;
	});

	$: noteIdFromUrl = $page.url.searchParams.get('note');
	$: if (noteIdFromUrl && noteIdFromUrl !== selectedNote?.id) {
		const target = notes.find((item) => item.id === noteIdFromUrl);
		if (target) {
			selectNote(target);
		}
	}
	$: if (selectedNote?.id && selectedNote.id !== attachmentMarker) {
		attachmentMarker = selectedNote.id;
		void refreshAttachments(selectedNote.id);
	}
	$: if (!selectedNote) {
		attachmentMarker = null;
		attachments = [];
		attachmentChunkState = {};
	}

	onMount(() => {
		void loadNotes();
	});

	async function loadNotes() {
		loading = true;
		try {
			notes = await listNotes(100);
			const routeNoteId = noteIdFromUrl;
			if (routeNoteId) {
				const routeNote = notes.find((item) => item.id === routeNoteId);
				if (routeNote) {
					selectNote(routeNote);
					return;
				}
			}
			if (!selectedNote && notes.length > 0) {
				selectNote(notes[0]);
			}
		} catch {
			pushToast('Unable to load notes.', 'danger');
		} finally {
			loading = false;
		}
	}

	function selectNote(note: Note) {
		selectedNote = note;
		title = note.title ?? '';
		markdown = note.markdown ?? '';
		// Track in recent items
		recentItems.addNote(note.id, note.title ?? 'Untitled');
	}

	function newNote() {
		selectedNote = null;
		title = '';
		markdown = '';
	}

	async function deleteSelected() {
		if (!selectedNote) return;
		if (!confirm(`Delete note "${selectedNote.title || 'Untitled'}"?`)) return;
		try {
			await deleteNote(selectedNote.id);
			pushToast('Note deleted.', 'success');
			newNote();
			await loadNotes();
		} catch {
			pushToast('Unable to delete note.', 'danger');
		}
	}

	// Bulk operations
	function toggleBulkMode() {
		bulkMode = !bulkMode;
		if (!bulkMode) {
			selectedNoteIds = new Set();
		}
	}

	function toggleNoteSelection(noteId: string) {
		const newSet = new Set(selectedNoteIds);
		if (newSet.has(noteId)) {
			newSet.delete(noteId);
		} else {
			newSet.add(noteId);
		}
		selectedNoteIds = newSet;
	}

	function toggleSelectAll() {
		if (allDisplayedSelected) {
			// Deselect all displayed
			const newSet = new Set(selectedNoteIds);
			for (const note of displayedNotes) {
				newSet.delete(note.id);
			}
			selectedNoteIds = newSet;
		} else {
			// Select all displayed
			const newSet = new Set(selectedNoteIds);
			for (const note of displayedNotes) {
				newSet.add(note.id);
			}
			selectedNoteIds = newSet;
		}
	}

	function clearSelection() {
		selectedNoteIds = new Set();
	}

	async function bulkDeleteNotes() {
		if (selectedCount === 0) return;
		if (!confirm(`Delete ${selectedCount} note${selectedCount > 1 ? 's' : ''}? This cannot be undone.`)) return;

		bulkProcessing = true;
		let successCount = 0;
		let failCount = 0;

		for (const noteId of selectedNoteIds) {
			try {
				await deleteNote(noteId);
				successCount++;
			} catch {
				failCount++;
			}
		}

		bulkProcessing = false;
		selectedNoteIds = new Set();

		if (failCount === 0) {
			pushToast(`Deleted ${successCount} note${successCount > 1 ? 's' : ''}`, 'success');
		} else {
			pushToast(`Deleted ${successCount}, failed ${failCount}`, 'warning');
		}

		if (selectedNote && !notes.find((n) => n.id === selectedNote?.id)) {
			newNote();
		}
		await loadNotes();
	}

	async function bulkAddTag() {
		if (selectedCount === 0 || !bulkTagInput.trim()) return;
		const tag = bulkTagInput.trim().toLowerCase();

		bulkProcessing = true;
		let successCount = 0;

		for (const noteId of selectedNoteIds) {
			const note = notes.find((n) => n.id === noteId);
			if (!note) continue;
			const currentTags = note.tags ?? [];
			if (currentTags.includes(tag)) continue;

			try {
				await updateNote(noteId, { tags: [...currentTags, tag] });
				successCount++;
			} catch {
				// continue
			}
		}

		bulkProcessing = false;
		bulkTagInput = '';

		if (successCount > 0) {
			pushToast(`Added tag "${tag}" to ${successCount} note${successCount > 1 ? 's' : ''}`, 'success');
			await loadNotes();
		}
	}

	async function bulkRemoveTag(tag: string) {
		if (selectedCount === 0) return;

		bulkProcessing = true;
		let successCount = 0;

		for (const noteId of selectedNoteIds) {
			const note = notes.find((n) => n.id === noteId);
			if (!note) continue;
			const currentTags = note.tags ?? [];
			if (!currentTags.includes(tag)) continue;

			try {
				await updateNote(noteId, { tags: currentTags.filter((t) => t !== tag) });
				successCount++;
			} catch {
				// continue
			}
		}

		bulkProcessing = false;

		if (successCount > 0) {
			pushToast(`Removed tag "${tag}" from ${successCount} note${successCount > 1 ? 's' : ''}`, 'success');
			await loadNotes();
		}
	}

	// Get common tags from selected notes (for removal UI)
	$: selectedNoteTags = (() => {
		if (selectedCount === 0) return [];
		const tagCounts = new Map<string, number>();
		for (const noteId of selectedNoteIds) {
			const note = notes.find((n) => n.id === noteId);
			if (!note) continue;
			for (const tag of note.tags ?? []) {
				tagCounts.set(tag, (tagCounts.get(tag) ?? 0) + 1);
			}
		}
		return [...tagCounts.entries()]
			.filter(([_, count]) => count > 0)
			.sort((a, b) => b[1] - a[1])
			.map(([tag]) => tag);
	})();

	async function refreshAttachments(noteId: string) {
		attachmentsLoading = true;
		try {
			attachments = await listNodeAttachments(noteId);
		} catch {
			attachments = [];
			pushToast('Failed to load attachments', 'warning');
		} finally {
			attachmentsLoading = false;
		}
	}

	function ensureChunkState(attachmentId: string): AttachmentChunkState {
		const existing = attachmentChunkState[attachmentId];
		if (existing) return existing;
		const next: AttachmentChunkState = {
			open: false,
			loading: false,
			page: 0,
			query: ''
		};
		attachmentChunkState = { ...attachmentChunkState, [attachmentId]: next };
		return next;
	}

	function updateChunkState(attachmentId: string, updates: Partial<AttachmentChunkState>) {
		const current = ensureChunkState(attachmentId);
		attachmentChunkState = {
			...attachmentChunkState,
			[attachmentId]: { ...current, ...updates }
		};
	}

	async function loadAttachmentChunks(attachmentId: string, page = 0) {
		if (!selectedNote) return;
		updateChunkState(attachmentId, { loading: true, error: undefined, page });
		try {
			const offset = page * ATTACHMENT_CHUNK_PAGE_SIZE;
			const data = await getAttachmentChunks(selectedNote.id, attachmentId, {
				limit: ATTACHMENT_CHUNK_PAGE_SIZE,
				offset
			});
			updateChunkState(attachmentId, { loading: false, data });
		} catch (err) {
			console.error('Failed to load attachment chunks', err);
			updateChunkState(attachmentId, {
				loading: false,
				error: 'Unable to load attachment text.'
			});
		}
	}

	async function toggleAttachmentChunks(attachmentId: string) {
		const current = ensureChunkState(attachmentId);
		if (current.open) {
			updateChunkState(attachmentId, { open: false });
			return;
		}
		updateChunkState(attachmentId, { open: true });
		if (!current.data) {
			await loadAttachmentChunks(attachmentId, current.page);
		}
	}

	function insertAttachmentMarkdown(item: NodeAttachment) {
		if (!selectedNote) return;
		const inlineUrl = attachmentInlineUrl(selectedNote.id, item.attachment_id);
		markdown = buildAttachmentEmbedMarkdown(markdown, item, inlineUrl);
		pushToast('Attachment embedded in note.', 'success');
	}

	function updateChunkQuery(attachmentId: string, value: string) {
		updateChunkState(attachmentId, { query: value });
	}

	function copyChunk(text: string) {
		if (!navigator.clipboard) return;
		navigator.clipboard.writeText(text).catch(() => {
			pushToast('Unable to copy text.', 'warning');
		});
	}

	async function handleAttachmentUpload(event: Event) {
		const input = event.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		if (!file || !selectedNote) return;
		attachmentUploadPending = true;
		attachmentUploadProgress = 0;
		try {
			await uploadNodeAttachment(selectedNote.id, file, (progress) => {
				attachmentUploadProgress = progress.percentage;
			});
			pushToast('Attachment uploaded', 'success');
			await refreshAttachments(selectedNote.id);
		} catch {
			pushToast('Attachment upload failed', 'danger');
		} finally {
			attachmentUploadPending = false;
			attachmentUploadProgress = 0;
			if (attachmentFileInput) {
				attachmentFileInput.value = '';
			}
		}
	}

	async function removeAttachment(attachmentId: string) {
		if (!selectedNote) return;
		try {
			await deleteNodeAttachment(selectedNote.id, attachmentId);
			pushToast('Attachment removed', 'success');
			await refreshAttachments(selectedNote.id);
		} catch {
			pushToast('Failed to remove attachment', 'danger');
		}
	}

	async function togglePin() {
		if (!selectedNote) return;
		try {
			await updateNote(selectedNote.id, { pinned: !selectedNote.pinned });
			pushToast(selectedNote.pinned ? 'Unpinned' : 'Pinned', 'success');
			await loadNotes();
			const refreshed = notes.find((n) => n.id === selectedNote!.id);
			if (refreshed) selectNote(refreshed);
		} catch {
			pushToast('Failed to update pin state', 'danger');
		}
	}

	async function autoTag() {
		if (!selectedNote || autoTagging) return;
		autoTagging = true;
		suggestedTags = [];
		try {
			const content = `${selectedNote.title ?? ''}\n${selectedNote.markdown ?? ''}`;
			const result = await assistAutoTag({ text: content, existing_tags: allTags, limit: 4 });
			suggestedTags = result.tags.filter((t) => !(selectedNote!.tags ?? []).includes(t));
			if (suggestedTags.length === 0) {
				pushToast('No new tags suggested', 'info');
			}
		} catch {
			pushToast('Auto-tag failed. Check AI settings.', 'danger');
		} finally {
			autoTagging = false;
		}
	}

	async function applyTag(tag: string) {
		if (!selectedNote) return;
		try {
			const existingTags = selectedNote.tags ?? [];
			await updateNote(selectedNote.id, { tags: [...existingTags, tag] });
			suggestedTags = suggestedTags.filter((t) => t !== tag);
			await loadNotes();
			const refreshed = notes.find((n) => n.id === selectedNote!.id);
			if (refreshed) selectNote(refreshed);
			pushToast(`Tagged with "${tag}"`, 'success');
		} catch {
			pushToast('Failed to apply tag', 'danger');
		}
	}

	async function extractActionItems() {
		if (extractingActionItems) return;
		const currentTitle = title.trim() || selectedNote?.title || 'Untitled';
		const currentContent = markdown.trim();
		if (currentContent.length < 20) {
			pushToast('Add more note content before extracting action items.', 'warning');
			return;
		}

		extractingActionItems = true;
		try {
			const prompt = [
				'Extract concrete action items from this note.',
				'Return one task per line using markdown bullets.',
				'Each item should be concise and actionable.',
				'If possible include due:YYYY-MM-DD or due:today/tomorrow and p1..p5 priority markers.',
				'You may include #tags and est:NNm estimates when clearly supported by the note.',
				'',
				`Title: ${currentTitle}`,
				'',
				'Content:',
				currentContent.slice(0, 4000)
			].join('\n');

			const result = await assistTransform({
				text: prompt,
				mode: 'action_items',
				namespace: selectedNote?.namespace ?? undefined
			});

			const extracted = parseActionItems(result.transformed_text, { maxItems: 8 });
			if (extracted.length === 0) {
				pushToast('No actionable tasks found.', 'info');
				return;
			}

			const baseMetadata: Record<string, unknown> = {
				source_note_title: currentTitle,
				extracted_at: new Date().toISOString(),
				extraction_mode: 'assist_transform_action_items'
			};
			if (selectedNote?.id) {
				baseMetadata.source_note_id = selectedNote.id;
			}

			const payloads = buildTaskPayloadsFromActionItems(extracted, {
				defaultLabels: [...new Set([...(selectedNote?.tags ?? []), 'from-note'])],
				baseMetadata
			});

			for (const payload of payloads) {
				const sourceLine = selectedNote?.id
					? `Source note: ${currentTitle} (/notes?note=${selectedNote.id})`
					: `Source note draft: ${currentTitle}`;
				payload.description = payload.description
					? `${payload.description}\n\n${sourceLine}`
					: sourceLine;
				await createTaskOptimistic(payload);
			}

			pushToast(`Created ${payloads.length} tasks from this note.`, 'success');
		} catch {
			pushToast('Action item extraction failed. Check AI settings.', 'danger');
		} finally {
			extractingActionItems = false;
		}
	}

	async function applyTemplateToNote(template: KnowledgeNode) {
		if (selectedNote) {
			try {
				const response = await applyTemplate(template.id, {
					target_node_id: selectedNote.id,
					target_kind: 'fact',
					overwrite: false
				});
				const updated = response.node;
				title = updated.title ?? '';
				markdown = updated.content ?? '';
				pushToast('Template applied — empty fields filled.', 'success');
				await loadNotes();
				const refreshed = notes.find((note) => note.id === updated.id);
				if (refreshed) selectNote(refreshed);
				return;
			} catch {
				pushToast('Failed to apply template.', 'danger');
			}
		}

		const nextTitle = template.title ?? '';
		if (!title.trim() && nextTitle) title = nextTitle;
		if (!markdown.trim() && template.content) markdown = template.content;
		pushToast('Template applied to draft.', 'success');
	}

	async function saveNote() {
		if (!title.trim()) {
			pushToast('Title is required.', 'warning');
			return;
		}
		saving = true;
		try {
			let saved: Note | null;
			if (selectedNote) {
				saved = await updateNote(selectedNote.id, {
					title: title.trim(),
					markdown: markdown.trim()
				});
			} else {
				saved = await createNote(markdown.trim(), title.trim());
			}
			if (!saved) {
				throw new Error('Note save returned empty response');
			}
			pushToast('Note saved.', 'success');
			await loadNotes();
			const refreshed = notes.find((note) => note.id === saved.id) ?? saved;
			selectNote(refreshed);
		} catch {
			pushToast('Unable to save note.', 'danger');
		} finally {
			saving = false;
		}
	}
</script>

<div class="grid gap-6 lg:grid-cols-12">
	<section class="lg:col-span-4">
		<div class="flex items-center justify-between">
			<div>
				<h2 class="text-lg font-semibold text-white">Notes</h2>
				<p class="text-xs text-slate-400">
					{#if bulkMode && selectedCount > 0}
						{selectedCount} selected
					{:else}
						{displayedNotes.length}{displayedNotes.length !== notes.length ? ` / ${notes.length}` : ''} notes
					{/if}
				</p>
			</div>
			<div class="flex gap-2">
				<button
					class={`rounded-lg border px-3 py-2 text-xs transition ${
						bulkMode
							? 'border-sky-500 bg-sky-500/20 text-sky-300'
							: 'border-slate-700 text-slate-300 hover:bg-slate-800'
					}`}
					on:click={toggleBulkMode}
					title={bulkMode ? 'Exit bulk mode' : 'Select multiple notes'}
				>
					{bulkMode ? 'Done' : 'Select'}
				</button>
				<button
					class="rounded-lg bg-slate-800 px-3 py-2 text-xs text-slate-200 hover:bg-slate-700"
					on:click={newNote}
				>
					New note
				</button>
			</div>
		</div>

		<div class="mt-3 flex gap-2">
			<input
				class="flex-1 rounded-lg border border-slate-800 bg-slate-900 px-3 py-1.5 text-xs text-white placeholder-slate-500"
				placeholder="Search notes..."
				bind:value={searchQuery}
				aria-label="Search notes"
			/>
			{#if allTags.length > 0}
				<select
					class="rounded-lg border border-slate-800 bg-slate-900 px-2 py-1.5 text-xs text-white"
					bind:value={tagFilter}
					aria-label="Filter by tag"
				>
					<option value="">All tags</option>
					{#each allTags as tag}
						<option value={tag}>{tag}</option>
					{/each}
				</select>
			{/if}
		</div>

		{#if bulkMode && displayedNotes.length > 0}
			<div class="mt-3 rounded-lg border border-slate-700 bg-slate-800/50 p-3">
				<div class="flex flex-wrap items-center gap-2">
					<button
						class="rounded-lg border border-slate-600 px-2 py-1 text-[11px] text-slate-300 hover:bg-slate-700"
						on:click={toggleSelectAll}
					>
						{allDisplayedSelected ? 'Deselect all' : 'Select all'}
					</button>

					{#if selectedCount > 0}
						<span class="text-[11px] text-slate-500">|</span>

						<button
							class="rounded-lg border border-red-500/30 px-2 py-1 text-[11px] text-red-300 hover:bg-red-500/10"
							on:click={bulkDeleteNotes}
							disabled={bulkProcessing}
						>
							Delete ({selectedCount})
						</button>

						<div class="flex items-center gap-1">
							<input
								class="w-24 rounded-lg border border-slate-600 bg-slate-900 px-2 py-1 text-[11px] text-white placeholder-slate-500"
								placeholder="Add tag..."
								bind:value={bulkTagInput}
								on:keydown={(e) => e.key === 'Enter' && bulkAddTag()}
							/>
							<button
								class="rounded-lg border border-slate-600 px-2 py-1 text-[11px] text-slate-300 hover:bg-slate-700"
								on:click={bulkAddTag}
								disabled={bulkProcessing || !bulkTagInput.trim()}
							>
								Add
							</button>
						</div>

						{#if selectedNoteTags.length > 0}
							<div class="flex flex-wrap items-center gap-1">
								<span class="text-[11px] text-slate-500">Remove:</span>
								{#each selectedNoteTags.slice(0, 5) as tag}
									<button
										class="rounded bg-slate-700 px-1.5 py-0.5 text-[10px] text-slate-300 hover:bg-red-500/30 hover:text-red-200"
										on:click={() => bulkRemoveTag(tag)}
										disabled={bulkProcessing}
									>
										{tag} &times;
									</button>
								{/each}
							</div>
						{/if}
					{/if}
				</div>
			</div>
		{/if}

		<div class="mt-3 flex flex-col gap-2">
			{#if loading}
				<div class="rounded-lg border border-slate-800 p-4 text-xs text-slate-400">
					Loading notes...
				</div>
			{:else if notes.length === 0}
				<div class="rounded-xl border border-dashed border-slate-800 bg-slate-900/20 p-6 text-center">
					<div class="mx-auto mb-3 flex h-10 w-10 items-center justify-center rounded-full bg-sky-500/20 text-sm text-sky-300">
						N
					</div>
					<h3 class="text-sm font-medium text-white">No notes yet</h3>
					<p class="mt-1 text-[11px] text-slate-500">
						Start building your knowledge base.
					</p>
					<button
						class="mt-3 rounded-lg bg-sky-500 px-4 py-2 text-xs font-semibold text-white hover:bg-sky-400"
						on:click={newNote}
					>
						Create your first note
					</button>
					<p class="mt-2 text-[10px] text-slate-600">
						<kbd class="rounded border border-slate-700 bg-slate-800 px-1 py-0.5">Cmd+Shift+N</kbd> for quick capture
					</p>
				</div>
			{:else if displayedNotes.length === 0}
				<div class="rounded-lg border border-dashed border-slate-800 p-4 text-xs text-slate-400">
					No notes match your search.
				</div>
			{:else}
				{#each displayedNotes as note (note.id)}
					<div
						class={`flex items-start gap-2 rounded-lg border px-3 py-2 text-left text-xs transition ${
							selectedNoteIds.has(note.id)
								? 'border-sky-500 bg-sky-500/10'
								: note.id === selectedNote?.id
									? 'border-sky-500 bg-sky-500/10 text-sky-200'
									: 'border-slate-800 bg-slate-900/40 text-slate-200 hover:border-slate-700'
						}`}
					>
						{#if bulkMode}
							<label class="flex h-5 cursor-pointer items-center">
								<input
									type="checkbox"
									class="h-3.5 w-3.5 cursor-pointer rounded border-slate-600 bg-slate-800 text-sky-500 focus:ring-sky-500 focus:ring-offset-0"
									checked={selectedNoteIds.has(note.id)}
									on:change={() => toggleNoteSelection(note.id)}
								/>
							</label>
						{/if}
						<button
							class="min-w-0 flex-1 text-left"
							on:click={() => bulkMode ? toggleNoteSelection(note.id) : selectNote(note)}
						>
							<div class="flex items-center gap-1.5 font-semibold">
								{#if note.pinned}<span class="text-amber-400" title="Pinned">*</span>{/if}
								{note.title}
							</div>
							{#if note.tags && note.tags.length > 0}
								<div class="mt-1 flex flex-wrap gap-1">
									{#each note.tags.slice(0, 3) as tag}
										<span class="rounded bg-slate-800 px-1.5 py-0.5 text-[9px] text-slate-400">{tag}</span>
									{/each}
								</div>
							{/if}
							<p class="mt-1 line-clamp-2 text-[11px] text-slate-400">
								{note.markdown.slice(0, 120) || 'No content'}
							</p>
						</button>
					</div>
				{/each}
			{/if}
		</div>
	</section>

	<section class="lg:col-span-8">
		<div class="flex items-center justify-between">
			<div>
				<h2 class="text-lg font-semibold text-white">
					{selectedNote ? 'Edit note' : 'New note'}
				</h2>
				<p class="text-xs text-slate-400">Markdown remains canonical.</p>
			</div>
			<div class="flex items-center gap-2">
				<TemplatePicker
					kind="fact"
					namespace={selectedNote?.namespace ?? undefined}
					label="Use template"
					on:apply={(event) => applyTemplateToNote(event.detail)}
				/>
				<button
					class="rounded-lg bg-sky-500 px-3 py-2 text-xs font-semibold text-white hover:bg-sky-400"
					on:click={saveNote}
					disabled={saving}
				>
					{saving ? 'Saving…' : 'Save note'}
				</button>
			</div>
		</div>

		<div class="mt-4">
			<label class="text-xs uppercase tracking-wide text-slate-500" for="note-title">
				Title
			</label>
			<input
				id="note-title"
				class="mt-2 w-full rounded-lg border border-slate-800 bg-slate-900 px-3 py-2 text-sm text-white"
				placeholder="Note title"
				bind:value={title}
			/>
		</div>

		<div class="mt-4">
			<RichNoteEditor
				bind:markdown
				placeholder="Write a note…"
				noteTitle={title}
				namespace={selectedNote?.namespace ?? undefined}
				excludeNodeId={selectedNote?.id}
			/>
		</div>

		{#if selectedNote}
			<div class="mt-4 flex items-center gap-2">
				<button
					class="rounded-lg border px-3 py-2 text-xs transition {selectedNote.pinned
						? 'border-amber-500/30 bg-amber-500/10 text-amber-300'
						: 'border-slate-700 text-slate-300 hover:bg-slate-800'}"
					on:click={togglePin}
				>
					{selectedNote.pinned ? 'Unpin' : 'Pin'}
				</button>
				<button
					class="rounded-lg border border-slate-700 px-3 py-2 text-xs text-slate-300 hover:bg-slate-800"
					on:click={() => {
						if (!selectedNote) return;
						const content = `# ${selectedNote.title ?? 'Untitled'}\n\n${selectedNote.markdown ?? ''}`;
						const filename = (selectedNote.title ?? 'untitled').replace(/[^a-zA-Z0-9-_ ]/g, '').trim().replace(/\s+/g, '-') + '.md';
						const blob = new Blob([content], { type: 'text/markdown' });
						const url = URL.createObjectURL(blob);
						const a = document.createElement('a');
						a.href = url;
						a.download = filename;
						document.body.appendChild(a);
						a.click();
						document.body.removeChild(a);
						URL.revokeObjectURL(url);
						pushToast('Downloaded as Markdown', 'success');
					}}
				>
					Export .md
				</button>
				<button
					class="rounded-lg border border-slate-700 px-3 py-2 text-xs text-slate-300 hover:bg-slate-800"
					on:click={() => { showVersionHistory = true; }}
				>
					History
				</button>
				<button
					class="rounded-lg border border-red-500/30 px-3 py-2 text-xs text-red-300 hover:bg-red-500/10"
					on:click={deleteSelected}
				>
					Delete note
				</button>
				<button
					class="rounded-lg border border-purple-500/30 px-3 py-2 text-xs text-purple-300 hover:bg-purple-500/10 disabled:opacity-50"
					on:click={autoTag}
					disabled={autoTagging}
				>
					{autoTagging ? 'Analyzing...' : 'Auto-tag'}
				</button>
				<button
					class="rounded-lg border border-emerald-500/30 px-3 py-2 text-xs text-emerald-300 hover:bg-emerald-500/10 disabled:opacity-50"
					on:click={extractActionItems}
					disabled={extractingActionItems}
				>
					{extractingActionItems ? 'Extracting...' : 'Extract tasks'}
				</button>
			</div>

			{#if suggestedTags.length > 0}
				<div class="mt-3 flex items-center gap-2">
					<span class="text-[10px] text-slate-500">Suggested:</span>
					{#each suggestedTags as tag}
						<button
							class="rounded-lg border border-purple-500/30 bg-purple-500/10 px-2 py-1 text-[10px] text-purple-300 transition hover:bg-purple-500/20"
							on:click={() => applyTag(tag)}
						>
							+ {tag}
						</button>
					{/each}
					<button
						class="text-[10px] text-slate-500 hover:text-white"
						on:click={() => { suggestedTags = []; }}
					>
						dismiss
					</button>
				</div>
			{/if}

			<div class="mt-4">
				<BacklinksPanel nodeId={selectedNote.id} />
			</div>

			<div class="mt-4">
				<SuggestedConnections
					nodeId={selectedNote.id}
					content={selectedNote.markdown ?? ''}
				/>
			</div>

			<div class="mt-4 rounded-2xl border border-slate-900 bg-slate-900/40 p-4">
				<div class="flex items-center justify-between gap-2">
					<div>
						<h3 class="text-sm font-semibold text-white">Attachments</h3>
						<p class="text-[11px] text-slate-400">Files linked to this note.</p>
					</div>
					<label class="rounded-lg border border-slate-700 px-3 py-1.5 text-xs text-slate-200 hover:bg-slate-800">
						Upload
						<input
							type="file"
							class="hidden"
							bind:this={attachmentFileInput}
							on:change={handleAttachmentUpload}
						/>
					</label>
				</div>

				{#if attachmentUploadPending}
					<div class="mt-3 rounded border border-slate-800 px-3 py-2 text-xs text-slate-300">
						Uploading... {attachmentUploadProgress}%
					</div>
				{/if}

				<div class="mt-3 space-y-2">
					{#if attachmentsLoading}
						<p class="text-xs text-slate-500">Loading attachments...</p>
					{:else if attachments.length === 0}
						<p class="text-xs text-slate-500">No attachments yet.</p>
					{:else}
						{#each attachments as item (item.attachment_id)}
						{@const previewUrl = attachmentInlineUrl(selectedNote.id, item.attachment_id)}
						{@const kind = classifyAttachment(item)}
						{@const badge = extractionBadge(item.extraction_status)}
						{@const chunkState = attachmentChunkState[item.attachment_id] ?? { open: false, loading: false, page: 0, query: '' }}
						{@const maxPage = Math.min(
							ATTACHMENT_CHUNK_MAX_PAGES - 1,
							Math.max(0, Math.ceil((chunkState.data?.total_chunks ?? 0) / ATTACHMENT_CHUNK_PAGE_SIZE) - 1)
						)}
						<div class="rounded-lg border border-slate-800 bg-slate-900/40 p-3">
							<div class="flex flex-wrap items-center justify-between gap-2">
								<div class="min-w-0">
									<div class="truncate text-xs font-medium text-white">{item.file_name}</div>
									<div class="mt-1 text-[10px] text-slate-500">
										{formatFileSize(item.size_bytes)}
										<span
											class="ml-1 inline-flex items-center rounded-full border px-2 py-0.5 text-[10px] {badgeClass(badge.tone)}"
											title={formatExtractionStatus(item.extraction_status, item.extracted_chars)}
										>
											{badge.label}
										</span>
										{#if item.search_chunk_count}
											<span class="ml-1 text-[10px] text-slate-400">· {item.search_chunk_count} chunks</span>
										{/if}
									</div>
								</div>
								<div class="flex items-center gap-2">
									<button
										class="rounded-lg border border-slate-700 px-2 py-1 text-[10px] text-slate-200 hover:bg-slate-800"
										on:click={() => void toggleAttachmentChunks(item.attachment_id)}
									>
										{chunkState.open ? 'Hide text' : 'View text'}
									</button>
									<button
										class="rounded-lg border border-slate-700 px-2 py-1 text-[10px] text-slate-200 hover:bg-slate-800"
										on:click={() => insertAttachmentMarkdown(item)}
									>
										Embed
									</button>
									<a
										class="rounded-lg border border-slate-700 px-2 py-1 text-[10px] text-slate-200 hover:bg-slate-800"
										href={attachmentDownloadUrl(selectedNote.id, item.attachment_id)}
										target="_blank"
										rel="noreferrer"
									>
										Download
									</a>
									<button
										class="rounded-lg border border-red-500/30 px-2 py-1 text-[10px] text-red-300 hover:bg-red-500/10"
										on:click={() => void removeAttachment(item.attachment_id)}
									>
										Delete
									</button>
								</div>
							</div>

							{#if item.search_preview}
								<p class="mt-2 text-[11px] text-slate-400">{item.search_preview}</p>
							{/if}

							<div class="mt-3 space-y-3">
								{#if kind.isImage}
									<img
										class="h-40 w-full rounded-lg border border-slate-800 object-cover"
										src={previewUrl}
										alt={item.file_name}
										loading="lazy"
									/>
								{:else if kind.isPdf}
									<iframe
										class="h-44 w-full rounded-lg border border-slate-800 bg-slate-950"
										src={previewUrl}
										title={`Preview ${item.file_name}`}
										loading="lazy"
									></iframe>
								{:else if kind.isAudio}
									<audio
										class="w-full"
										controls
										src={previewUrl}
										aria-label={`Audio preview for ${item.file_name}`}
									></audio>
								{:else if kind.isVideo}
									<video
										class="h-44 w-full rounded-lg border border-slate-800 bg-black"
										controls
										src={previewUrl}
										aria-label={`Video preview for ${item.file_name}`}
									>
										<track kind="captions" />
									</video>
								{:else}
									<div class="rounded-lg border border-dashed border-slate-700 px-3 py-4 text-xs text-slate-500">
										No preview available.
									</div>
								{/if}
							</div>

							{#if chunkState.open}
								<div class="mt-3 rounded-lg border border-slate-800 bg-slate-950/70 p-3">
									<div class="flex flex-wrap items-center justify-between gap-2">
										<div class="text-[11px] text-slate-400">
											Indexed text chunks
											{#if chunkState.data}
												· {chunkState.data.returned_chunks}/{chunkState.data.total_chunks}
											{/if}
										</div>
										<input
											class="rounded border border-slate-800 bg-slate-900 px-2 py-1 text-[10px] text-slate-200"
											placeholder="Search chunks"
											value={chunkState.query}
											on:input={(event) => updateChunkQuery(item.attachment_id, (event.target as HTMLInputElement).value)}
											aria-label="Search attachment chunks"
										/>
										<div class="flex items-center gap-2">
											<button
												class="rounded border border-slate-700 px-2 py-1 text-[10px] text-slate-200 hover:bg-slate-800"
												disabled={chunkState.page === 0}
												on:click={() => loadAttachmentChunks(item.attachment_id, Math.max(0, chunkState.page - 1))}
											>
												Prev
											</button>
											<button
												class="rounded border border-slate-700 px-2 py-1 text-[10px] text-slate-200 hover:bg-slate-800"
												disabled={chunkState.page >= maxPage}
												on:click={() => loadAttachmentChunks(item.attachment_id, Math.min(maxPage, chunkState.page + 1))}
											>
												Next
											</button>
										</div>
									</div>

									{#if chunkState.loading}
										<p class="mt-2 text-[11px] text-slate-400">Loading chunks…</p>
									{:else if chunkState.error}
										<p class="mt-2 text-[11px] text-red-300">{chunkState.error}</p>
									{:else if (chunkState.data?.chunks ?? []).length === 0}
										<p class="mt-2 text-[11px] text-slate-400">No extracted text yet.</p>
									{:else}
										<div class="mt-2 space-y-2">
											{#each filterAttachmentChunks(chunkState.data?.chunks ?? [], chunkState.query) as chunk}
												<div class="rounded border border-slate-800 bg-slate-900/60 p-2">
													<div class="flex items-center justify-between text-[10px] text-slate-400">
														<span>Chunk {chunk.index + 1} · {chunk.char_count} chars</span>
														<button
															class="text-slate-300 hover:text-white"
															on:click={() => copyChunk(chunk.text)}
														>
															Copy
														</button>
													</div>
													<p class="mt-1 whitespace-pre-line text-[11px] text-slate-200">{chunk.text}</p>
												</div>
											{/each}
										</div>
									{/if}
								</div>
							{/if}
						</div>
					{/each}
					{/if}
				</div>
			</div>
		{/if}
	</section>
</div>

{#if selectedNote}
	<VersionHistory
		nodeId={selectedNote.id}
		bind:open={showVersionHistory}
		on:restored={async () => {
			showVersionHistory = false;
			await loadNotes();
			if (selectedNote) {
				const refreshed = notes.find((n) => n.id === selectedNote?.id);
				if (refreshed) selectNote(refreshed);
			}
		}}
	/>
{/if}
