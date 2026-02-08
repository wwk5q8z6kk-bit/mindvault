<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import RichNoteEditor from '$lib/components/RichNoteEditor.svelte';
	import AttachmentsPanel from '$lib/components/AttachmentsPanel.svelte';
	import { recentItems } from '$lib/stores/recent';
	import BacklinksPanel from '$lib/components/BacklinksPanel.svelte';
	import SuggestedConnections from '$lib/components/SuggestedConnections.svelte';
	import TemplatePicker from '$lib/components/TemplatePicker.svelte';
	import { applyTemplate } from '$lib/api/templates';
	import type { KnowledgeNode } from '$lib/api/types';
	import { buildAttachmentEmbedMarkdown } from '$lib/utils/attachments';
	import { createNote, listNotes, updateNote, deleteNote } from '$lib/api/notes';
	import { listNodes } from '$lib/api/nodes';
	import { kindLabel, kindBadgeClass } from '$lib/utils/kind-helpers';
	import type { NodeKind } from '$lib/api/types';
	import { assistAutoTag, assistTransform } from '$lib/api/assist';
	import { createTaskOptimistic } from '$lib/stores/tasks';
	import { buildTaskPayloadsFromActionItems, parseActionItems } from '$lib/tasks/action-items';
	import type { NodeAttachment } from '$lib/api/files';
	import type { Note } from '$lib/api/notes';

	// Extended Note with kind for filtering
	interface NoteWithKind extends Note {
		kind: NodeKind;
	}
	import { pushToast } from '$lib/stores/toast';
	import AiSuggestionsPanel from '$lib/components/AiSuggestionsPanel.svelte';
	import { fetchAgentContext, agentStore } from '$lib/api/agent';
	import VersionHistory from '$lib/components/VersionHistory.svelte';
	import { createVirtualizer } from '@tanstack/svelte-virtual';

	let notes: NoteWithKind[] = [];
	let selectedNote: NoteWithKind | null = null;
	let title = '';
	let markdown = '';
	let loading = false;
	let saving = false;
	let showVersionHistory = false;
	let searchQuery = '';
	let tagFilter = '';
	let kindFilter: NodeKind | 'all' = 'all';
	let autoTagging = false;

	// Note-like kinds that should appear in the notes view
	const NOTE_LIKE_KINDS: NodeKind[] = ['fact', 'decision', 'procedure', 'observation', 'preference', 'concept'];
	let extractingActionItems = false;
	let suggestedTags: string[] = [];
	let showAiSuggestions = true;
	let agentContextLoading = false;

	// Bulk selection state
	let selectedNoteIds: Set<string> = new Set();
	let bulkMode = false;
	let bulkTagInput = '';
	let bulkProcessing = false;

	// Keyboard navigation state
	let focusedIndex = 0;
	let listContainer: HTMLDivElement | null = null;

	let notesListParentRef: HTMLDivElement | null = null;

	const notesVirtualizer = createVirtualizer({
		get count() {
			return displayedNotes.length;
		},
		getScrollElement: () => notesListParentRef,
		estimateSize: () => 72,
		overscan: 5
	});

	$: selectedCount = selectedNoteIds.size;
	$: allDisplayedSelected = displayedNotes.length > 0 && displayedNotes.every((n) => selectedNoteIds.has(n.id));

	$: allTags = [...new Set(notes.flatMap((n) => n.tags ?? []))].sort();
	$: displayedNotes = notes.filter((n) => {
		// Kind filter
		if (kindFilter !== 'all' && n.kind !== kindFilter) return false;
		// Search filter
		if (searchQuery.trim()) {
			const q = searchQuery.toLowerCase();
			const matchTitle = (n.title ?? '').toLowerCase().includes(q);
			const matchContent = (n.markdown ?? '').toLowerCase().includes(q);
			if (!matchTitle && !matchContent) return false;
		}
		// Tag filter
		if (tagFilter && !(n.tags ?? []).includes(tagFilter)) return false;
		return true;
	});
	// Get unique kinds from loaded notes for the filter dropdown
	$: availableKinds = [...new Set(notes.map((n) => n.kind))].sort();

	// Reset focused index when displayed notes change
	$: if (displayedNotes.length > 0 && focusedIndex >= displayedNotes.length) {
		focusedIndex = Math.max(0, displayedNotes.length - 1);
	}

	function handleKeydown(event: KeyboardEvent) {
		// Skip if user is typing in an input
		const target = event.target as HTMLElement;
		if (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable) {
			return;
		}

		if (displayedNotes.length === 0) return;

		switch (event.key) {
			case 'j':
				event.preventDefault();
				focusedIndex = Math.min(focusedIndex + 1, displayedNotes.length - 1);
				scrollToFocused();
				break;
			case 'k':
				event.preventDefault();
				focusedIndex = Math.max(focusedIndex - 1, 0);
				scrollToFocused();
				break;
			case 'Enter':
				event.preventDefault();
				if (displayedNotes[focusedIndex]) {
					selectNote(displayedNotes[focusedIndex]);
				}
				break;
			case 'x':
				event.preventDefault();
				if (displayedNotes[focusedIndex]) {
					toggleNoteSelection(displayedNotes[focusedIndex].id);
					if (!bulkMode) {
						bulkMode = true;
					}
				}
				break;
		}
	}

	function scrollToFocused() {
		if (!listContainer) return;
		const items = listContainer.querySelectorAll('[data-note-item]');
		if (items[focusedIndex]) {
			items[focusedIndex].scrollIntoView({ block: 'nearest', behavior: 'smooth' });
		}
	}

	$: noteIdFromUrl = $page.url.searchParams.get('note');
	$: if (noteIdFromUrl && noteIdFromUrl !== selectedNote?.id) {
		const target = notes.find((item) => item.id === noteIdFromUrl);
		if (target) {
			selectNote(target);
		}
	}
	onMount(() => {
		void loadNotes();
	});

	async function loadNotes() {
		loading = true;
		try {
			// Load notes from all note-like kinds
			const allNotes: NoteWithKind[] = [];
			for (const kind of NOTE_LIKE_KINDS) {
				try {
					const nodes = await listNodes({ kind, limit: 100 });
					for (const node of nodes) {
						// Skip daily notes (tagged with day:)
						if (node.tags.some((t) => t.startsWith('day:'))) continue;
						allNotes.push({
							id: node.id,
							title: node.title,
							markdown: node.content ?? '',
							namespace: node.namespace,
							tags: node.tags,
							backlinks: [],
							pinned: Boolean(node.metadata?.pinned),
							created_at: node.temporal.created_at,
							updated_at: node.temporal.updated_at,
							metadata: node.metadata,
							kind: node.kind as NodeKind
						});
					}
				} catch {
					// Some kinds may not have any nodes, continue
				}
			}
			// Sort by updated_at descending
			allNotes.sort((a, b) => new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime());
			notes = allNotes;

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

	function selectNote(note: NoteWithKind) {
		selectedNote = note;
		title = note.title ?? '';
		markdown = note.markdown ?? '';
		showAiSuggestions = true;
		// Track in recent items
		recentItems.addNote(note.id, note.title ?? 'Untitled');
		// Load agent context for this note
		void loadAgentContext(note.id);
	}

	async function loadAgentContext(nodeId: string) {
		agentContextLoading = true;
		try {
			await fetchAgentContext(nodeId);
		} catch {
			// Agent context is supplementary — fail silently
		} finally {
			agentContextLoading = false;
		}
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

	// Bulk export functionality
	type ExportFormat = 'markdown' | 'json' | 'html';
	let showExportModal = false;
	let exportFormat: ExportFormat = 'markdown';
	let exportProgress = 0;
	let exporting = false;

	function getSelectedNotes(): NoteWithKind[] {
		return notes.filter((n) => selectedNoteIds.has(n.id));
	}

	function generateMarkdownExport(notesToExport: NoteWithKind[]): string {
		return notesToExport
			.map((note) => {
				const header = `# ${note.title ?? 'Untitled'}\n`;
				const meta = [
					`**Kind:** ${note.kind}`,
					note.tags?.length ? `**Tags:** ${note.tags.join(', ')}` : null,
					`**Created:** ${new Date(note.created_at).toLocaleString()}`,
					`**Updated:** ${new Date(note.updated_at).toLocaleString()}`
				]
					.filter(Boolean)
					.join('\n');
				const content = note.markdown ?? '';
				return `${header}\n${meta}\n\n---\n\n${content}`;
			})
			.join('\n\n---\n\n# \n\n');
	}

	function generateJsonExport(notesToExport: NoteWithKind[]): string {
		const exportData = notesToExport.map((note) => ({
			id: note.id,
			title: note.title,
			kind: note.kind,
			content: note.markdown,
			tags: note.tags,
			pinned: note.pinned,
			created_at: note.created_at,
			updated_at: note.updated_at,
			namespace: note.namespace
		}));
		return JSON.stringify(exportData, null, 2);
	}

	function generateHtmlExport(notesToExport: NoteWithKind[]): string {
		const notesHtml = notesToExport
			.map((note) => {
				const title = note.title ?? 'Untitled';
				const tags = note.tags?.map((t) => `<span class="tag">${t}</span>`).join('') ?? '';
				const content = (note.markdown ?? '')
					.replace(/&/g, '&amp;')
					.replace(/</g, '&lt;')
					.replace(/>/g, '&gt;')
					.replace(/\n/g, '<br>');
				return `
					<article class="note">
						<h2>${title}</h2>
						<div class="meta">
							<span class="kind">${note.kind}</span>
							${tags}
							<span class="date">${new Date(note.updated_at).toLocaleDateString()}</span>
						</div>
						<div class="content">${content}</div>
					</article>
				`;
			})
			.join('\n');

		return `<!DOCTYPE html>
<html lang="en">
<head>
	<meta charset="UTF-8">
	<meta name="viewport" content="width=device-width, initial-scale=1.0">
	<title>MindVault Notes Export</title>
	<style>
		body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 800px; margin: 0 auto; padding: 2rem; background: #0f172a; color: #e2e8f0; }
		h1 { color: #38bdf8; border-bottom: 1px solid #334155; padding-bottom: 1rem; }
		.note { background: #1e293b; border-radius: 12px; padding: 1.5rem; margin-bottom: 1.5rem; }
		.note h2 { margin: 0 0 0.5rem; color: #f1f5f9; }
		.meta { display: flex; gap: 0.5rem; flex-wrap: wrap; font-size: 0.75rem; color: #94a3b8; margin-bottom: 1rem; }
		.kind { background: #7c3aed33; color: #c4b5fd; padding: 0.25rem 0.5rem; border-radius: 4px; }
		.tag { background: #334155; padding: 0.25rem 0.5rem; border-radius: 4px; }
		.content { white-space: pre-wrap; line-height: 1.6; }
	</style>
</head>
<body>
	<h1>MindVault Notes Export</h1>
	<p style="color: #64748b; font-size: 0.875rem;">Exported ${notesToExport.length} note${notesToExport.length === 1 ? '' : 's'} on ${new Date().toLocaleString()}</p>
	${notesHtml}
</body>
</html>`;
	}

	async function bulkExportNotes() {
		if (selectedCount === 0) return;
		exporting = true;
		exportProgress = 0;

		const notesToExport = getSelectedNotes();
		let content: string;
		let filename: string;
		let mimeType: string;

		const timestamp = new Date().toISOString().slice(0, 10);

		switch (exportFormat) {
			case 'markdown':
				content = generateMarkdownExport(notesToExport);
				filename = `mindvault-notes-${timestamp}.md`;
				mimeType = 'text/markdown';
				break;
			case 'json':
				content = generateJsonExport(notesToExport);
				filename = `mindvault-notes-${timestamp}.json`;
				mimeType = 'application/json';
				break;
			case 'html':
				content = generateHtmlExport(notesToExport);
				filename = `mindvault-notes-${timestamp}.html`;
				mimeType = 'text/html';
				break;
		}

		for (let i = 0; i <= 100; i += 20) {
			exportProgress = i;
			await new Promise((r) => setTimeout(r, 50));
		}

		const blob = new Blob([content], { type: mimeType });
		const url = URL.createObjectURL(blob);
		const a = document.createElement('a');
		a.href = url;
		a.download = filename;
		document.body.appendChild(a);
		a.click();
		document.body.removeChild(a);
		URL.revokeObjectURL(url);

		exporting = false;
		showExportModal = false;
		pushToast(`Exported ${notesToExport.length} note${notesToExport.length === 1 ? '' : 's'}`, 'success');
	}

	function handleAttachmentEmbed(payload: { attachment: NodeAttachment; inlineUrl: string }) {
		if (!selectedNote) return;
		markdown = buildAttachmentEmbedMarkdown(markdown, payload.attachment, payload.inlineUrl);
		pushToast('Attachment embedded in note.', 'success');
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
			const fallback: NoteWithKind = {
				...saved,
				kind: selectedNote?.kind ?? 'fact'
			};
			const refreshed = notes.find((note) => note.id === saved.id) ?? fallback;
			selectNote(refreshed);
		} catch {
			pushToast('Unable to save note.', 'danger');
		} finally {
			saving = false;
		}
	}
</script>

<svelte:window on:keydown={handleKeydown} />

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
				<p class="mt-0.5 text-[10px] text-slate-600">
					<kbd class="rounded border border-slate-700 bg-slate-800 px-1">j</kbd>/<kbd class="rounded border border-slate-700 bg-slate-800 px-1">k</kbd> navigate,
					<kbd class="rounded border border-slate-700 bg-slate-800 px-1">Enter</kbd> open,
					<kbd class="rounded border border-slate-700 bg-slate-800 px-1">x</kbd> select
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

		<div class="mt-3 flex flex-wrap gap-2">
			<input
				class="min-w-0 flex-1 rounded-lg border border-slate-800 bg-slate-900 px-3 py-1.5 text-xs text-white placeholder-slate-500"
				placeholder="Search notes..."
				bind:value={searchQuery}
				aria-label="Search notes"
			/>
			<select
				class="rounded-lg border border-slate-800 bg-slate-900 px-2 py-1.5 text-xs text-white"
				bind:value={kindFilter}
				aria-label="Filter by kind"
			>
				<option value="all">All kinds</option>
				{#each availableKinds as kind}
					<option value={kind}>{kindLabel(kind)}</option>
				{/each}
			</select>
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
							class="rounded-lg border border-emerald-500/30 px-2 py-1 text-[11px] text-emerald-300 hover:bg-emerald-500/10"
							on:click={() => (showExportModal = true)}
							disabled={bulkProcessing}
						>
							Export ({selectedCount})
						</button>

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

		<div class="mt-3" bind:this={listContainer}>
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
				<div bind:this={notesListParentRef} style="max-height: 70vh; overflow-y: auto;">
					<div style="height: {$notesVirtualizer.getTotalSize()}px; width: 100%; position: relative;">
						{#each $notesVirtualizer.getVirtualItems() as row (row.key)}
							{@const note = displayedNotes[row.index]}
							{@const idx = row.index}
							<div
								style="position: absolute; top: 0; left: 0; width: 100%; transform: translateY({row.start}px);"
							>
								<div
									data-note-item
									class={`flex items-start gap-2 rounded-lg border px-3 py-2 mb-2 text-left text-xs transition ${
										idx === focusedIndex
											? 'ring-1 ring-sky-400/50'
											: ''
									} ${
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
										on:mouseenter={() => {
											focusedIndex = idx;
										}}
										on:click={() => bulkMode ? toggleNoteSelection(note.id) : selectNote(note)}
									>
										<div class="flex items-center gap-1.5">
											<span class={`rounded-full px-1.5 py-0.5 text-[9px] font-medium ${kindBadgeClass(note.kind)}`}>
												{kindLabel(note.kind)}
											</span>
											{#if note.pinned}<span class="text-amber-400" title="Pinned">*</span>{/if}
											<span class="font-semibold truncate">{note.title}</span>
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
							</div>
						{/each}
					</div>
				</div>
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

			{#if showAiSuggestions}
				<div class="mt-4">
					<AiSuggestionsPanel
						nodeId={selectedNote.id}
						on:applied={async () => {
							showAiSuggestions = false;
							await loadNotes();
							const refreshed = notes.find((n) => n.id === selectedNote?.id);
							if (refreshed) selectNote(refreshed);
						}}
						on:dismissed={() => { showAiSuggestions = false; }}
					/>
				</div>
			{/if}

			{#if $agentStore.relatedNodes.length > 0 || agentContextLoading}
				<div class="mt-4 rounded-xl border border-sky-500/20 bg-sky-500/5 p-4">
					<div class="flex items-center gap-2 mb-2">
						{#if agentContextLoading}
							<div class="h-2 w-2 rounded-full bg-sky-400 animate-pulse"></div>
						{/if}
						<h4 class="text-[10px] font-bold uppercase tracking-wider text-sky-400">Agent Context</h4>
					</div>
					<p class="text-xs text-slate-300 leading-relaxed">{$agentStore.summary}</p>
					{#if $agentStore.relatedNodes.length > 0}
						<div class="mt-2 space-y-1">
							{#each $agentStore.relatedNodes.slice(0, 5) as node (node.id)}
								<a
									href="/notes?note={node.id}"
									class="block rounded-lg border border-slate-800/60 bg-slate-900/40 px-2 py-1.5 text-xs text-slate-300 transition hover:border-sky-500/30 hover:text-sky-200"
								>
									{node.title || 'Untitled'}
								</a>
							{/each}
						</div>
					{/if}
				</div>
			{/if}

			<div class="mt-4">
				<AttachmentsPanel
					nodeId={selectedNote.id}
					allowEmbed
					onEmbed={handleAttachmentEmbed}
					title="Attachments"
					description="Files linked to this note."
				/>
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

<!-- Export Modal -->
{#if showExportModal}
	<div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm">
		<div class="w-full max-w-sm rounded-xl border border-slate-700 bg-slate-900 p-5 shadow-xl">
			<h3 class="text-sm font-semibold text-white">Export Notes</h3>
			<p class="mt-1 text-[11px] text-slate-400">
				Export {selectedCount} selected note{selectedCount === 1 ? '' : 's'}
			</p>

			<div class="mt-4">
				<span class="text-[10px] uppercase tracking-wide text-slate-500">Format</span>
				<div class="mt-2 grid grid-cols-3 gap-2">
					{#each [
						{ value: 'markdown', label: 'Markdown', desc: '.md file' },
						{ value: 'json', label: 'JSON', desc: 'Structured data' },
						{ value: 'html', label: 'HTML', desc: 'Styled page' }
					] as format}
						<button
							class="rounded-lg border px-3 py-2 text-left transition {exportFormat === format.value
								? 'border-sky-500 bg-sky-500/10'
								: 'border-slate-700 bg-slate-800/50 hover:border-slate-600'}"
							on:click={() => (exportFormat = format.value as ExportFormat)}
						>
							<div class="text-xs font-medium {exportFormat === format.value ? 'text-sky-300' : 'text-slate-200'}">
								{format.label}
							</div>
							<div class="text-[9px] text-slate-500">{format.desc}</div>
						</button>
					{/each}
				</div>
			</div>

			{#if exporting}
				<div class="mt-4">
					<div class="h-2 w-full rounded-full bg-slate-800">
						<div
							class="h-full rounded-full bg-sky-500 transition-all"
							style="width: {exportProgress}%"
						></div>
					</div>
					<p class="mt-1 text-center text-[10px] text-slate-400">Preparing export...</p>
				</div>
			{/if}

			<div class="mt-5 flex justify-end gap-2">
				<button
					class="rounded-lg border border-slate-700 px-3 py-1.5 text-xs text-slate-300 hover:bg-slate-800"
					on:click={() => (showExportModal = false)}
					disabled={exporting}
				>
					Cancel
				</button>
				<button
					class="rounded-lg bg-sky-500 px-4 py-1.5 text-xs font-semibold text-white hover:bg-sky-400 disabled:opacity-50"
					on:click={bulkExportNotes}
					disabled={exporting || selectedCount === 0}
				>
					{exporting ? 'Exporting...' : `Export ${selectedCount} Note${selectedCount === 1 ? '' : 's'}`}
				</button>
			</div>
		</div>
	</div>
{/if}
