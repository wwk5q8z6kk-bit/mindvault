<script lang="ts">
	// Preserves the existing graph, canvas, PDF, media, and power-user notes workspace.
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
	import EmptyState from '$lib/components/EmptyState.svelte';
	import { createTaskOptimistic } from '$lib/stores/tasks';
	import { buildTaskPayloadsFromActionItems, parseActionItems } from '$lib/tasks/action-items';
	import type { NodeAttachment } from '$lib/api/files';
	import type { Note } from '$lib/api/notes';
	import { markdownToHTML } from '$lib/editor';
	import ViewToggle from '$lib/components/ViewToggle.svelte';
	import DistillPanel from '$lib/components/DistillPanel.svelte';
	import ContextBoundary from '$lib/components/ContextBoundary.svelte';
	import { createViewMode, setViewMode } from '$lib/utils/view-mode';
	import {
		setViewPreference,
		setNotesListCollapsed,
		setNotesListTab,
		toggleNotesListCollapsed,
		viewPreferences
	} from '$lib/stores/view-preferences';
	import { filterNotesForSidebar, normalizeNotesListTab } from '$lib/notes/sidebar';
	import NotesGraphView from '$lib/components/NotesGraphView.svelte';
	import NotesCanvasView from '$lib/components/NotesCanvasView.svelte';
	import NotesPdfView from '$lib/components/NotesPdfView.svelte';
	import NotesMediaView from '$lib/components/NotesMediaView.svelte';
	import { fade } from 'svelte/transition';

	const currentView = createViewMode('list', ['list', 'graph', 'canvas', 'pdf', 'media']);

	const notesViews = [
		{ key: 'list', label: 'List' },
		{ key: 'graph', label: 'Graph' },
		{ key: 'canvas', label: 'Canvas' },
		{ key: 'pdf', label: 'PDF' },
		{ key: 'media', label: 'Media' }
	];

	function handleViewChange(event: CustomEvent<string>) {
		const view = event.detail;
		setViewPreference('notes', view);
		setViewMode(view, 'list');
	}

	// Extended Note with kind for filtering
	interface NoteWithKind extends Note {
		kind: NodeKind;
	}
	import { pushToast } from '$lib/stores/toast';
	import AiSuggestionsPanel from '$lib/components/AiSuggestionsPanel.svelte';
	import { fetchAgentContext, agentStore } from '$lib/api/agent';
	import VersionHistory from '$lib/components/VersionHistory.svelte';
	import PublicSharePanel from '$lib/components/PublicSharePanel.svelte';
	import CommentsPanel from '$lib/components/CommentsPanel.svelte';
	import { createVirtualizer } from '@tanstack/svelte-virtual';

	let notes: NoteWithKind[] = [];
	let selectedNote: NoteWithKind | null = null;
	let title = '';
	let markdown = '';
	let loading = false;
	let saving = false;
	let showDistillPanel = false;
	let showVersionHistory = false;
	let searchQuery = '';
	let tagFilter = '';
	let autoTagging = false;

	// Note-like kinds that should appear in the notes view
	const NOTE_LIKE_KINDS: NodeKind[] = [
		'fact',
		'decision',
		'procedure',
		'observation',
		'preference',
		'concept'
	];
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
	let displayedNotes: NoteWithKind[] = [];

	const notesVirtualizer = createVirtualizer({
		get count() {
			return displayedNotes.length;
		},
		getScrollElement: () => notesListParentRef,
		estimateSize: () => 72,
		overscan: 5
	});

	$: selectedCount = selectedNoteIds.size;
	$: allDisplayedSelected =
		displayedNotes.length > 0 && displayedNotes.every((n) => selectedNoteIds.has(n.id));

	$: allTags = [...new Set(notes.flatMap((n) => n.tags ?? []))].sort();
	// Get unique kinds from loaded notes for filter tabs
	$: availableKinds = [...new Set(notes.map((n) => n.kind))].sort();
	$: notesListCollapsed = $viewPreferences.notesListCollapsed;
	$: notesListTab = normalizeNotesListTab($viewPreferences.notesListTab, availableKinds);
	$: displayedNotes = filterNotesForSidebar(notes, {
		tab: notesListTab,
		searchQuery,
		tagFilter
	});
	$: pinnedCount = notes.filter((n) => n.pinned).length;

	function selectListTab(tab: string) {
		setNotesListTab(normalizeNotesListTab(tab, availableKinds));
	}

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

		if (event.key === '[') {
			event.preventDefault();
			toggleNotesListCollapsed();
			return;
		}

		if (displayedNotes.length === 0 || notesListCollapsed) return;

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
		if (
			!confirm(
				`Delete ${selectedCount} note${selectedCount > 1 ? 's' : ''}? This cannot be undone.`
			)
		)
			return;

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
			pushToast(
				`Added tag "${tag}" to ${successCount} note${successCount > 1 ? 's' : ''}`,
				'success'
			);
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
			pushToast(
				`Removed tag "${tag}" from ${successCount} note${successCount > 1 ? 's' : ''}`,
				'success'
			);
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

	function escapeHtml(value: string): string {
		return value
			.replace(/&/g, '&amp;')
			.replace(/</g, '&lt;')
			.replace(/>/g, '&gt;')
			.replace(/"/g, '&quot;')
			.replace(/'/g, '&#39;');
	}

	const htmlExportOptions = {
		renderers: {
			html: (node: { value?: string }) => {
				const raw = String(node?.value ?? '');
				if (!raw) {
					return '';
				}
				return `<pre class="html-block">${escapeHtml(raw)}</pre>\n`;
			}
		}
	};

	function generateHtmlExport(notesToExport: NoteWithKind[]): string {
		const notesHtml = notesToExport
			.map((note) => {
				const title = note.title ?? 'Untitled';
				const tags = note.tags?.map((t) => `<span class="tag">${t}</span>`).join('') ?? '';
				const content = markdownToHTML(note.markdown ?? '', undefined, htmlExportOptions);
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
		.content { line-height: 1.6; }
		.content pre { background: #0b1220; color: #e2e8f0; padding: 0.75rem; border-radius: 8px; overflow: auto; }
		.content code { font-family: 'SFMono-Regular', Consolas, 'Liberation Mono', Menlo, monospace; font-size: 0.85rem; }
		.content .mv-mermaid { background: #0b1220; padding: 0.75rem; border-radius: 8px; overflow: auto; }
		.content .html-block { background: #0b1220; color: #e2e8f0; padding: 0.75rem; border-radius: 8px; overflow: auto; }
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
		pushToast(
			`Exported ${notesToExport.length} note${notesToExport.length === 1 ? '' : 's'}`,
			'success'
		);
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

<div
	class="relative mb-6 flex flex-col gap-4 overflow-hidden rounded-[var(--mv-radius)] border border-white/5 bg-[rgb(var(--mv-panel))]/40 p-6 backdrop-blur-xl shadow-lg sm:flex-row sm:items-center sm:justify-between"
>
	<div
		class="absolute -left-20 -top-20 h-40 w-40 rounded-full bg-sky-500/10 blur-3xl pointer-events-none"
	></div>

	<div class="relative z-10">
		<h2
			class="text-3xl font-extrabold tracking-tight text-transparent bg-clip-text bg-gradient-to-r from-[rgb(var(--mv-text))] to-[rgb(var(--mv-muted))]"
		>
			Notes
		</h2>
		<p class="mt-1 text-sm font-medium text-[rgb(var(--mv-muted))]/80">
			Explore connections inside your private Personal Vault.
		</p>
		<div class="mt-3">
			<ContextBoundary compact detail="Local-first" />
		</div>
	</div>

	<div class="relative z-10 flex items-center gap-3">
		<button
			class="rounded-lg border border-sky-500/30 bg-sky-500/10 px-4 py-2 text-sm font-medium text-sky-200 transition-all hover:bg-sky-500/20 shadow-md hover:shadow-sky-500/20 hover:-translate-y-0.5"
			on:click={() => {
				showDistillPanel = true;
			}}
		>
			Summarize
		</button>
		{#if selectedNote}
			<a
				href={`/chat?node=${selectedNote.id}`}
				class="rounded-lg border border-violet-500/30 bg-violet-500/10 px-3 py-2 text-xs text-violet-200 hover:bg-violet-500/20"
			>
				Ask About Note
			</a>
		{/if}
		{#if $currentView === 'list'}
			<button
				class={`rounded-lg border px-3 py-2 text-xs transition ${
					bulkMode
						? 'border-sky-500 bg-sky-500/20 text-sky-300'
						: 'border-[rgb(var(--mv-border))] text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-panel-strong))]'
				}`}
				on:click={toggleBulkMode}
				title={bulkMode ? 'Exit bulk mode' : 'Select multiple notes'}
			>
				{bulkMode ? 'Done' : 'Select'}
			</button>
			<button
				class="rounded-lg bg-[rgb(var(--mv-panel-strong))] px-3 py-2 text-xs text-[rgb(var(--mv-text))]/90 hover:bg-[rgb(var(--mv-panel-strong))]/80"
				on:click={newNote}
			>
				New note
			</button>
		{/if}
	</div>
</div>

<div class="mt-4">
	<ViewToggle
		views={notesViews}
		activeView={$currentView}
		variant="tabs"
		size="sm"
		on:change={handleViewChange}
	/>
</div>

{#if $currentView === 'list'}
	<div class="mt-4 grid gap-4 lg:grid-cols-12 lg:gap-6">
		{#if notesListCollapsed}
			<aside
				class="order-2 flex items-center gap-2 lg:order-1 lg:col-span-1 lg:flex-col lg:items-stretch"
			>
				<button
					class="inline-flex items-center justify-center gap-2 rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))] px-3 py-2 text-xs text-[rgb(var(--mv-muted))] transition hover:bg-[rgb(var(--mv-panel-strong))] hover:text-[rgb(var(--mv-text))]"
					on:click={() => setNotesListCollapsed(false)}
					title="Show notes list ([)"
					aria-label="Show notes list"
					aria-expanded="false"
				>
					<svg
						class="h-4 w-4"
						fill="none"
						stroke="currentColor"
						viewBox="0 0 24 24"
						aria-hidden="true"
					>
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							stroke-width="2"
							d="M4 6h16M4 12h10M4 18h14"
						/>
					</svg>
					<span class="lg:hidden">Notes ({notes.length})</span>
				</button>
				<p class="hidden text-center text-[10px] text-[rgb(var(--mv-muted))]/50 lg:block">
					{notes.length}
				</p>
			</aside>
		{:else}
			<section class="order-1 lg:order-1 lg:col-span-4" aria-label="Notes list">
				<div class="mt-1 flex items-center justify-between gap-2">
					<p class="text-xs text-[rgb(var(--mv-muted))]/60">
						{#if bulkMode && selectedCount > 0}
							{selectedCount} selected
						{:else}
							{displayedNotes.length}{displayedNotes.length !== notes.length
								? ` / ${notes.length}`
								: ''} notes
						{/if}
					</p>
					<div class="flex items-center gap-2">
						<p class="hidden text-[10px] text-[rgb(var(--mv-muted))]/30 sm:block">
							<kbd
								class="rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))] px-1"
								>j</kbd
							>/<kbd
								class="rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))] px-1"
								>k</kbd
							>
							<span class="ml-1">navigate</span>
						</p>
						<button
							class="rounded-lg border border-[rgb(var(--mv-border))] px-2 py-1 text-[11px] text-[rgb(var(--mv-muted))] transition hover:bg-[rgb(var(--mv-panel-strong))] hover:text-[rgb(var(--mv-text))]"
							on:click={() => setNotesListCollapsed(true)}
							title="Collapse notes list ([)"
							aria-label="Collapse notes list"
							aria-expanded="true"
						>
							Hide
						</button>
					</div>
				</div>

				<div
					class="mt-3 flex gap-1 overflow-x-auto pb-1"
					role="tablist"
					aria-label="Notes list filters"
				>
					<button
						role="tab"
						aria-selected={notesListTab === 'all'}
						class={`shrink-0 rounded-lg px-2.5 py-1 text-[11px] font-medium transition ${
							notesListTab === 'all'
								? 'bg-sky-500/20 text-sky-200'
								: 'text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-panel-strong))]'
						}`}
						on:click={() => selectListTab('all')}
					>
						All
						<span class="ml-1 text-[10px] opacity-60">{notes.length}</span>
					</button>
					<button
						role="tab"
						aria-selected={notesListTab === 'pinned'}
						class={`shrink-0 rounded-lg px-2.5 py-1 text-[11px] font-medium transition ${
							notesListTab === 'pinned'
								? 'bg-amber-500/20 text-amber-200'
								: 'text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-panel-strong))]'
						}`}
						on:click={() => selectListTab('pinned')}
					>
						Pinned
						<span class="ml-1 text-[10px] opacity-60">{pinnedCount}</span>
					</button>
					{#each availableKinds as kind}
						<button
							role="tab"
							aria-selected={notesListTab === kind}
							class={`shrink-0 rounded-lg px-2.5 py-1 text-[11px] font-medium transition ${
								notesListTab === kind
									? 'bg-sky-500/20 text-sky-200'
									: 'text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-panel-strong))]'
							}`}
							on:click={() => selectListTab(kind)}
						>
							{kindLabel(kind)}
						</button>
					{/each}
				</div>

				<div class="mt-3 flex flex-wrap gap-2">
					<input
						class="min-w-0 flex-1 rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))] px-3 py-1.5 text-xs text-[rgb(var(--mv-text))] placeholder-[rgb(var(--mv-muted))]/40"
						placeholder="Search notes..."
						bind:value={searchQuery}
						aria-label="Search notes"
					/>
					{#if allTags.length > 0}
						<select
							class="rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))] px-2 py-1.5 text-xs text-[rgb(var(--mv-text))]"
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
					<div
						class="mt-3 rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))]/50 p-3"
					>
						<div class="flex flex-wrap items-center gap-2">
							<button
								class="rounded-lg border border-[rgb(var(--mv-border))] px-2 py-1 text-[11px] text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-panel-strong))]/80"
								on:click={toggleSelectAll}
							>
								{allDisplayedSelected ? 'Deselect all' : 'Select all'}
							</button>

							{#if selectedCount > 0}
								<span class="text-[11px] text-[rgb(var(--mv-muted))]/40">|</span>

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
										class="w-24 rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))] px-2 py-1 text-[11px] text-[rgb(var(--mv-text))] placeholder-[rgb(var(--mv-muted))]/40"
										placeholder="Add tag..."
										bind:value={bulkTagInput}
										on:keydown={(e) => e.key === 'Enter' && bulkAddTag()}
									/>
									<button
										class="rounded-lg border border-[rgb(var(--mv-border))] px-2 py-1 text-[11px] text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-panel-strong))]/80"
										on:click={bulkAddTag}
										disabled={bulkProcessing || !bulkTagInput.trim()}
									>
										Add
									</button>
								</div>

								{#if selectedNoteTags.length > 0}
									<div class="flex flex-wrap items-center gap-1">
										<span class="text-[11px] text-[rgb(var(--mv-muted))]/40">Remove:</span>
										{#each selectedNoteTags.slice(0, 5) as tag}
											<button
												class="rounded bg-[rgb(var(--mv-panel-strong))] px-1.5 py-0.5 text-[10px] text-[rgb(var(--mv-muted))] hover:bg-red-500/30 hover:text-red-200"
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
						<div
							class="rounded-lg border border-[rgb(var(--mv-border))] p-4 text-xs text-[rgb(var(--mv-muted))]/60"
						>
							Loading notes...
						</div>
					{:else if notes.length === 0}
						<EmptyState
							compact
							icon="generic"
							tone="sky"
							title="No notes yet"
							description="Start building your knowledge base."
							actionLabel="Create your first note"
							onAction={newNote}
						>
							<svelte:fragment slot="footer">
								<kbd
									class="rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))] px-1 py-0.5"
									>Cmd+Shift+N</kbd
								> for quick capture
							</svelte:fragment>
						</EmptyState>
					{:else if displayedNotes.length === 0}
						<EmptyState
							compact
							icon="search"
							tone="slate"
							title="No notes match your search"
							description="Try a different query or clear filters."
						/>
					{:else}
						<div bind:this={notesListParentRef} style="max-height: 70vh; overflow-y: auto;">
							<div
								style="height: {$notesVirtualizer.getTotalSize()}px; width: 100%; position: relative;"
							>
								{#each $notesVirtualizer.getVirtualItems() as row (row.key)}
									{@const note = displayedNotes[row.index]}
									{@const idx = row.index}
									<div
										style="position: absolute; top: 0; left: 0; width: 100%; transform: translateY({row.start}px);"
									>
										<div
											data-note-item
											class={`flex items-start gap-2 rounded-lg border px-3 py-2 mb-2 text-left text-xs transition ${
												idx === focusedIndex ? 'ring-1 ring-sky-400/50' : ''
											} ${
												selectedNoteIds.has(note.id)
													? 'border-sky-500 bg-sky-500/10'
													: note.id === selectedNote?.id
														? 'border-sky-500 bg-sky-500/10 text-sky-200'
														: 'border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/40 text-[rgb(var(--mv-text))]/90 hover:border-[rgb(var(--mv-border))]/80'
											}`}
										>
											{#if bulkMode}
												<label class="flex h-5 cursor-pointer items-center">
													<input
														type="checkbox"
														class="h-3.5 w-3.5 cursor-pointer rounded border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))] text-sky-500 focus:ring-sky-500 focus:ring-offset-0"
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
												on:click={() =>
													bulkMode ? toggleNoteSelection(note.id) : selectNote(note)}
											>
												<div class="flex items-center gap-1.5">
													<span
														class={`rounded-full px-1.5 py-0.5 text-[9px] font-medium ${kindBadgeClass(note.kind)}`}
													>
														{kindLabel(note.kind)}
													</span>
													{#if note.pinned}<span class="text-amber-400" title="Pinned">*</span>{/if}
													<span class="font-semibold truncate">{note.title}</span>
												</div>
												{#if note.tags && note.tags.length > 0}
													<div class="mt-1 flex flex-wrap gap-1">
														{#each note.tags.slice(0, 3) as tag}
															<span
																class="rounded bg-[rgb(var(--mv-panel-strong))] px-1.5 py-0.5 text-[9px] text-[rgb(var(--mv-muted))]/60"
																>{tag}</span
															>
														{/each}
													</div>
												{/if}
												<p class="mt-1 line-clamp-2 text-[11px] text-[rgb(var(--mv-muted))]/60">
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
		{/if}

		<section
			class={notesListCollapsed
				? 'order-1 lg:order-2 lg:col-span-11'
				: 'order-2 lg:order-2 lg:col-span-8'}
		>
			<div class="flex items-center justify-between">
				<div>
					<h2 class="text-lg font-semibold text-[rgb(var(--mv-text))]">
						{selectedNote ? 'Edit note' : 'New note'}
					</h2>
					<p class="text-xs text-[rgb(var(--mv-muted))]/60">Markdown remains canonical.</p>
				</div>
				<div class="flex items-center gap-2">
					{#if notesListCollapsed}
						<button
							class="rounded-lg border border-[rgb(var(--mv-border))] px-3 py-2 text-xs text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-panel-strong))]"
							on:click={() => setNotesListCollapsed(false)}
							title="Show notes list ([)"
						>
							Show list
						</button>
					{/if}
					<TemplatePicker
						kind="fact"
						namespace={selectedNote?.namespace ?? undefined}
						label="Use template"
						on:apply={(event) => applyTemplateToNote(event.detail)}
					/>
					<button
						class="rounded-lg bg-sky-500 px-3 py-2 text-xs font-semibold text-[rgb(var(--mv-text))] hover:bg-sky-400"
						on:click={saveNote}
						disabled={saving}
					>
						{saving ? 'Saving…' : 'Save note'}
					</button>
				</div>
			</div>

			<div class="mt-4">
				<label
					class="text-xs uppercase tracking-wide text-[rgb(var(--mv-muted))]/40"
					for="note-title"
				>
					Title
				</label>
				<input
					id="note-title"
					class="mt-2 w-full rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))] px-3 py-2 text-sm text-[rgb(var(--mv-text))]"
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
							: 'border-[rgb(var(--mv-border))] text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-panel-strong))]'}"
						on:click={togglePin}
					>
						{selectedNote.pinned ? 'Unpin' : 'Pin'}
					</button>
					<button
						class="rounded-lg border border-[rgb(var(--mv-border))] px-3 py-2 text-xs text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-panel-strong))]"
						on:click={() => {
							if (!selectedNote) return;
							const content = `# ${selectedNote.title ?? 'Untitled'}\n\n${selectedNote.markdown ?? ''}`;
							const filename =
								(selectedNote.title ?? 'untitled')
									.replace(/[^a-zA-Z0-9-_ ]/g, '')
									.trim()
									.replace(/\s+/g, '-') + '.md';
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
						class="rounded-lg border border-[rgb(var(--mv-border))] px-3 py-2 text-xs text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-panel-strong))]"
						on:click={() => {
							showVersionHistory = true;
						}}
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
						<span class="text-[10px] text-[rgb(var(--mv-muted))]/40">Suggested:</span>
						{#each suggestedTags as tag}
							<button
								class="rounded-lg border border-purple-500/30 bg-purple-500/10 px-2 py-1 text-[10px] text-purple-300 transition hover:bg-purple-500/20"
								on:click={() => applyTag(tag)}
							>
								+ {tag}
							</button>
						{/each}
						<button
							class="text-[10px] text-[rgb(var(--mv-muted))]/40 hover:text-[rgb(var(--mv-text))]"
							on:click={() => {
								suggestedTags = [];
							}}
						>
							dismiss
						</button>
					</div>
				{/if}

				<div class="mt-4">
					<BacklinksPanel nodeId={selectedNote.id} />
				</div>

				<div class="mt-4">
					<SuggestedConnections nodeId={selectedNote.id} content={selectedNote.markdown ?? ''} />
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
							on:dismissed={() => {
								showAiSuggestions = false;
							}}
						/>
					</div>
				{/if}

				{#if $agentStore.relatedNodes.length > 0 || agentContextLoading}
					<div class="mt-4 rounded-xl border border-sky-500/20 bg-sky-500/5 p-4">
						<div class="flex items-center gap-2 mb-2">
							{#if agentContextLoading}
								<div class="h-2 w-2 rounded-full bg-sky-400 animate-pulse"></div>
							{/if}
							<h4 class="text-[10px] font-bold uppercase tracking-wider text-sky-400">
								Agent Context
							</h4>
						</div>
						<p class="text-xs text-[rgb(var(--mv-muted))] leading-relaxed">{$agentStore.summary}</p>
						{#if $agentStore.relatedNodes.length > 0}
							<div class="mt-2 space-y-1">
								{#each $agentStore.relatedNodes.slice(0, 5) as node (node.id)}
									<a
										href="/notes?note={node.id}"
										class="block rounded-lg border border-[rgb(var(--mv-border))]/60 bg-[rgb(var(--mv-panel))]/40 px-2 py-1.5 text-xs text-[rgb(var(--mv-muted))] transition hover:border-sky-500/30 hover:text-sky-200"
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

				<div class="mt-4">
					<PublicSharePanel nodeId={selectedNote.id} nodeTitle={selectedNote.title ?? 'Untitled'} />
				</div>
				<div class="mt-4">
					<CommentsPanel nodeId={selectedNote.id} nodeTitle={selectedNote.title ?? 'Untitled'} />
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
			<div
				class="w-full max-w-sm rounded-xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))] p-5 shadow-xl"
			>
				<h3 class="text-sm font-semibold text-[rgb(var(--mv-text))]">Export Notes</h3>
				<p class="mt-1 text-[11px] text-[rgb(var(--mv-muted))]/60">
					Export {selectedCount} selected note{selectedCount === 1 ? '' : 's'}
				</p>

				<div class="mt-4">
					<span class="text-[10px] uppercase tracking-wide text-[rgb(var(--mv-muted))]/40"
						>Format</span
					>
					<div class="mt-2 grid grid-cols-3 gap-2">
						{#each [{ value: 'markdown', label: 'Markdown', desc: '.md file' }, { value: 'json', label: 'JSON', desc: 'Structured data' }, { value: 'html', label: 'HTML', desc: 'Styled page' }] as format}
							<button
								class="rounded-lg border px-3 py-2 text-left transition {exportFormat ===
								format.value
									? 'border-sky-500 bg-sky-500/10'
									: 'border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))]/50 hover:border-[rgb(var(--mv-muted))]/40'}"
								on:click={() => (exportFormat = format.value as ExportFormat)}
							>
								<div
									class="text-xs font-medium {exportFormat === format.value
										? 'text-sky-300'
										: 'text-[rgb(var(--mv-text))]/90'}"
								>
									{format.label}
								</div>
								<div class="text-[9px] text-[rgb(var(--mv-muted))]/40">{format.desc}</div>
							</button>
						{/each}
					</div>
				</div>

				{#if exporting}
					<div class="mt-4">
						<div class="h-2 w-full rounded-full bg-[rgb(var(--mv-panel-strong))]">
							<div
								class="h-full rounded-full bg-sky-500 transition-all"
								style="width: {exportProgress}%"
							></div>
						</div>
						<p class="mt-1 text-center text-[10px] text-[rgb(var(--mv-muted))]/60">
							Preparing export...
						</p>
					</div>
				{/if}

				<div class="mt-5 flex justify-end gap-2">
					<button
						class="rounded-lg border border-[rgb(var(--mv-border))] px-3 py-1.5 text-xs text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-panel-strong))]"
						on:click={() => (showExportModal = false)}
						disabled={exporting}
					>
						Cancel
					</button>
					<button
						class="rounded-lg bg-sky-500 px-4 py-1.5 text-xs font-semibold text-[rgb(var(--mv-text))] hover:bg-sky-400 disabled:opacity-50"
						on:click={bulkExportNotes}
						disabled={exporting || selectedCount === 0}
					>
						{exporting
							? 'Exporting...'
							: `Export ${selectedCount} Note${selectedCount === 1 ? '' : 's'}`}
					</button>
				</div>
			</div>
		</div>
	{/if}
{:else if $currentView === 'graph' || $currentView === 'canvas'}
	{#key $currentView}
		{#if $currentView === 'graph'}
			<div class="mt-4">
				<NotesGraphView />
			</div>
		{:else}
			<div class="mt-4">
				<NotesCanvasView />
			</div>
		{/if}
	{/key}
{:else if $currentView === 'pdf'}
	<div class="mt-4">
		<NotesPdfView />
	</div>
{:else if $currentView === 'media'}
	<div class="mt-4">
		<NotesMediaView />
	</div>
{/if}

<DistillPanel
	open={showDistillPanel}
	defaultNamespace={selectedNote?.namespace ?? 'default'}
	on:close={() => {
		showDistillPanel = false;
	}}
/>
