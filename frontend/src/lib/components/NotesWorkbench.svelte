<!-- Query-string variants are composed after SvelteKit resolves the application base path. -->
<script lang="ts">
	/* eslint-disable svelte/no-navigation-without-resolve */
	import { onDestroy, onMount, tick } from 'svelte';
	import { resolve } from '$app/paths';
	import {
		ArrowLeft,
		Bookmark,
		CalendarDays,
		Check,
		CheckCircle2,
		ChevronDown,
		Command,
		Expand,
		FileText,
		History,
		Inbox,
		LayoutDashboard,
		Link2,
		Minimize2,
		Network,
		Pin,
		Plus,
		Search,
		Settings2,
		SlidersHorizontal,
		SquareCheckBig,
		Target,
		Trash2,
		X
	} from '@lucide/svelte';
	import FocusedNoteEditor from '$lib/components/FocusedNoteEditor.svelte';
	import { createNote, deleteNote, updateNote, type Note } from '$lib/api/notes';
	import { listNodes } from '$lib/api/nodes';
	import type { NodeKind } from '$lib/api/types';
	import { pushToast } from '$lib/stores/toast';

	interface WorkbenchNote extends Note {
		kind: NodeKind;
	}

	type ListTab = 'all' | 'pinned' | 'recent';
	type SaveStatus = 'saved' | 'saving' | 'error';

	const noteKinds: NodeKind[] = [
		'fact',
		'decision',
		'procedure',
		'observation',
		'preference',
		'concept'
	];

	const navGroups = [
		{
			label: '',
			items: [
				{ label: 'Dashboard', href: '/', icon: LayoutDashboard },
				{ label: 'Inbox', href: '/inbox', icon: Inbox },
				{ label: 'Daily', href: '/focus', icon: CalendarDays },
				{ label: 'Search', href: '/search', icon: Search }
			]
		},
		{
			label: 'Knowledge Base',
			items: [
				{ label: 'Notes', href: '/notes', icon: FileText },
				{ label: 'Resources', href: '/bookmarks', icon: Bookmark }
			]
		},
		{
			label: 'Productivity',
			items: [
				{ label: 'Tasks', href: '/tasks', icon: SquareCheckBig },
				{ label: 'Goals', href: '/goals', icon: Target }
			]
		},
		{
			label: 'Connections',
			items: [
				{ label: 'Backlinks', href: '/graph', icon: Link2 },
				{ label: 'Graph', href: '/notes', query: '?view=graph', icon: Network }
			]
		}
	] as const;

	let notes: WorkbenchNote[] = [];
	let selectedNote: WorkbenchNote | null = null;
	let title = '';
	let markdown = '';
	let activeTags: string[] = [];
	let searchQuery = '';
	let activeTab: ListTab = 'all';
	let activeTagFilter: string | null = null;
	let loading = true;
	let demoMode = false;
	let toolsOpen = false;
	let tagEditorOpen = false;
	let tagDraft = '';
	let filtersExpanded = false;
	let mobilePane: 'list' | 'editor' = 'editor';
	let focusMode = false;
	let savedStatus: SaveStatus = 'saved';
	let savedAt = new Date();
	let searchInput: HTMLInputElement;
	let titleInput: HTMLTextAreaElement;
	let tagInput: HTMLInputElement;
	let saveTimer: ReturnType<typeof setTimeout> | null = null;
	let saveInFlight = false;

	function normalizeTag(tag: string) {
		return tag.trim().toLowerCase();
	}

	$: tagCounts = notes.reduce((counts, note) => {
		for (const tag of new Set(note.tags.map(normalizeTag).filter(Boolean))) {
			counts.set(tag, (counts.get(tag) ?? 0) + 1);
		}
		return counts;
	}, new Map<string, number>());
	$: availableTags = Array.from(tagCounts.keys()).sort(
		(a, b) => (tagCounts.get(b) ?? 0) - (tagCounts.get(a) ?? 0) || a.localeCompare(b)
	);
	$: hasActiveFilters =
		Boolean(searchQuery.trim()) || activeTab !== 'all' || activeTagFilter !== null;

	function createDemoNotes(): WorkbenchNote[] {
		const now = Date.now();
		const minute = 60_000;
		const day = 86_400_000;
		const defaults: Array<{
			title: string;
			markdown: string;
			tags: string[];
			age: number;
			pinned?: boolean;
			kind?: NodeKind;
		}> = [
			{
				title: 'Building a local-first memory system',
				markdown:
					"The goal is simple: capture ideas and knowledge in a way that's fast, private, and always available. Everything should work offline first, sync when possible, and stay under my control.\n\nCore principles:\n\n- Local-first by default\n- Plain text over proprietary formats\n- Incremental sync with conflict safety\n- Powerful search and linking\n- Calm, focused writing experience\n\nIf we get these right, the system fades into the background and thinking stays in the foreground.",
				tags: ['systems', 'architecture'],
				age: 10 * minute,
				pinned: true,
				kind: 'concept'
			},
			{
				title: 'Daily plan – May 14, 2025',
				markdown:
					'Protect the morning for focused work, review open decisions after lunch, and close the day by clearing the inbox.',
				tags: ['daily', 'planning'],
				age: 65 * minute
			},
			{
				title: 'How I capture ideas',
				markdown:
					'Capture first, organize later. The fastest trustworthy path wins: one place, lightweight context, and a clear next review.',
				tags: ['writing'],
				age: day
			},
			{
				title: 'Books worth revisiting',
				markdown:
					'A short shelf of books whose ideas keep compounding: systems thinking, humane technology, and the craft of clear explanation.',
				tags: ['reading'],
				age: day + 3 * 60 * minute
			},
			{
				title: 'Offline-first sync patterns',
				markdown:
					'Optimistic local writes, durable operation logs, explicit conflict handling, and quiet background reconciliation.',
				tags: ['systems'],
				age: 2 * day,
				kind: 'procedure'
			},
			{
				title: 'Weekly review – Week 19',
				markdown:
					'Review what moved, what stalled, and what needs a deliberate decision before the next planning cycle.',
				tags: ['review', 'planning'],
				age: 3 * day
			},
			{
				title: 'Design principles for calm software',
				markdown:
					'Clarity over spectacle. Progressive disclosure over crowded surfaces. Reliable states over clever animation.',
				tags: ['design'],
				age: 4 * day,
				kind: 'preference'
			},
			{
				title: 'Ideas backlog',
				markdown:
					'A parking place for useful possibilities that should not interrupt the work already in motion.',
				tags: ['inbox'],
				age: 5 * day
			}
		];

		return defaults.map((item, index) => ({
			id: `local-preview-${index + 1}`,
			title: item.title,
			markdown: item.markdown,
			namespace: 'default',
			tags: item.tags,
			backlinks: [],
			pinned: item.pinned ?? false,
			created_at: new Date(now - item.age - day).toISOString(),
			updated_at: new Date(now - item.age).toISOString(),
			metadata: {},
			kind: item.kind ?? 'fact'
		}));
	}

	$: filteredNotes = notes
		.filter((note) => {
			if (activeTab === 'pinned' && !note.pinned) return false;
			if (
				activeTab === 'recent' &&
				Date.now() - new Date(note.updated_at).getTime() > 7 * 86_400_000
			)
				return false;
			if (activeTagFilter && !note.tags.some((tag) => normalizeTag(tag) === activeTagFilter))
				return false;
			const query = searchQuery.trim().toLowerCase();
			if (!query) return true;
			return (
				(note.title ?? '').toLowerCase().includes(query) ||
				note.markdown.toLowerCase().includes(query) ||
				note.tags.some((tag) => tag.toLowerCase().includes(query))
			);
		})
		.sort((a, b) => new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime());

	$: wordCount = markdown.trim() ? markdown.trim().split(/\s+/).length : 0;
	$: characterCount = markdown.length;
	$: saveLabel =
		savedStatus === 'saving'
			? 'Saving locally…'
			: savedStatus === 'error'
				? 'Saved in this session'
				: 'Saved locally';

	onMount(() => {
		void loadWorkbenchNotes();
	});

	onDestroy(() => {
		if (saveTimer) clearTimeout(saveTimer);
	});

	async function loadWorkbenchNotes() {
		loading = true;
		const loaded: WorkbenchNote[] = [];
		let successfulRequests = 0;

		for (const kind of noteKinds) {
			try {
				const nodes = await listNodes({ kind, limit: 100 });
				successfulRequests += 1;
				for (const node of nodes) {
					if (node.tags.some((tag) => tag.startsWith('day:'))) continue;
					loaded.push({
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
						kind: node.kind
					});
				}
			} catch {
				// A local preview remains useful while the API is unavailable.
			}
		}

		if (successfulRequests === 0) {
			demoMode = true;
			notes = createDemoNotes();
		} else {
			notes = loaded;
		}

		loading = false;
		const first = notes[0];
		if (first) {
			selectNote(first, false);
		} else {
			void newNote(false);
			mobilePane = 'list';
		}
	}

	function selectNote(note: WorkbenchNote, moveToEditor = true) {
		if (saveTimer) clearTimeout(saveTimer);
		selectedNote = note;
		title = note.title ?? '';
		markdown = note.markdown;
		activeTags = [...note.tags];
		savedStatus = 'saved';
		savedAt = new Date(note.updated_at);
		toolsOpen = false;
		tagEditorOpen = false;
		if (moveToEditor) mobilePane = 'editor';
	}

	async function newNote(focusTitle = true) {
		if (saveTimer) clearTimeout(saveTimer);
		selectedNote = null;
		title = '';
		markdown = '';
		activeTags = [];
		savedStatus = 'saved';
		tagEditorOpen = false;
		toolsOpen = false;
		mobilePane = 'editor';
		if (focusTitle) {
			await tick();
			titleInput?.focus();
		}
	}

	function scheduleSave() {
		savedStatus = 'saving';
		if (saveTimer) clearTimeout(saveTimer);
		saveTimer = setTimeout(() => {
			void saveCurrentNote();
		}, 700);
	}

	function updateNoteInMemory(note: WorkbenchNote) {
		const existingIndex = notes.findIndex((item) => item.id === note.id);
		if (existingIndex === -1) {
			notes = [note, ...notes];
		} else {
			notes = notes.map((item) => (item.id === note.id ? note : item));
		}
		selectedNote = note;
		savedAt = new Date(note.updated_at);
	}

	async function saveCurrentNote() {
		if (saveInFlight) {
			scheduleSave();
			return;
		}
		if (!title.trim() && !markdown.trim()) {
			savedStatus = 'saved';
			return;
		}

		saveInFlight = true;
		savedStatus = 'saving';
		const timestamp = new Date().toISOString();

		try {
			if (
				demoMode ||
				selectedNote?.id.startsWith('local-preview-') ||
				selectedNote?.id.startsWith('local-draft-')
			) {
				const localNote: WorkbenchNote = {
					id: selectedNote?.id ?? `local-draft-${crypto.randomUUID()}`,
					title: title.trim() || 'Untitled',
					markdown,
					namespace: selectedNote?.namespace ?? 'default',
					tags: activeTags,
					backlinks: selectedNote?.backlinks ?? [],
					pinned: selectedNote?.pinned ?? false,
					created_at: selectedNote?.created_at ?? timestamp,
					updated_at: timestamp,
					metadata: selectedNote?.metadata ?? {},
					kind: selectedNote?.kind ?? 'fact'
				};
				updateNoteInMemory(localNote);
			} else if (selectedNote) {
				const saved = await updateNote(selectedNote.id, {
					title: title.trim() || 'Untitled',
					markdown,
					tags: activeTags
				});
				updateNoteInMemory({ ...saved, kind: selectedNote.kind });
			} else {
				const saved = await createNote({
					title: title.trim() || 'Untitled',
					markdown,
					tags: activeTags
				});
				updateNoteInMemory({ ...saved, kind: 'fact' });
			}
			savedStatus = 'saved';
		} catch {
			demoMode = true;
			const localNote: WorkbenchNote = {
				id: selectedNote?.id ?? `local-draft-${crypto.randomUUID()}`,
				title: title.trim() || 'Untitled',
				markdown,
				namespace: selectedNote?.namespace ?? 'default',
				tags: activeTags,
				backlinks: selectedNote?.backlinks ?? [],
				pinned: selectedNote?.pinned ?? false,
				created_at: selectedNote?.created_at ?? timestamp,
				updated_at: timestamp,
				metadata: selectedNote?.metadata ?? {},
				kind: selectedNote?.kind ?? 'fact'
			};
			updateNoteInMemory(localNote);
			savedStatus = 'error';
		} finally {
			saveInFlight = false;
		}
	}

	async function addTag() {
		const nextTag = tagDraft.trim().toLowerCase().replace(/\s+/g, '-');
		if (!nextTag || activeTags.includes(nextTag)) {
			tagDraft = '';
			tagEditorOpen = false;
			return;
		}
		activeTags = [...activeTags, nextTag];
		tagDraft = '';
		tagEditorOpen = false;
		scheduleSave();
	}

	async function openTagEditor() {
		tagEditorOpen = true;
		await tick();
		tagInput?.focus();
	}

	function removeTag(tag: string) {
		activeTags = activeTags.filter((item) => item !== tag);
		scheduleSave();
	}

	function toggleTagFilter(tag: string, showList = false) {
		const normalizedTag = normalizeTag(tag);
		activeTagFilter = activeTagFilter === normalizedTag ? null : normalizedTag;
		filtersExpanded = true;
		if (showList) mobilePane = 'list';
	}

	function clearFilters() {
		searchQuery = '';
		activeTab = 'all';
		activeTagFilter = null;
	}

	async function togglePin() {
		if (!selectedNote) return;
		const next = !selectedNote.pinned;
		const updated = { ...selectedNote, pinned: next, updated_at: new Date().toISOString() };
		updateNoteInMemory(updated);
		toolsOpen = false;

		if (!demoMode && !selectedNote.id.startsWith('local-')) {
			try {
				await updateNote(selectedNote.id, { pinned: next });
			} catch {
				demoMode = true;
			}
		}
		pushToast(next ? 'Note pinned.' : 'Note unpinned.', 'success');
	}

	async function removeCurrentNote() {
		if (!selectedNote) return;
		if (!confirm(`Delete "${selectedNote.title || 'Untitled'}"?`)) return;
		const deleting = selectedNote;
		notes = notes.filter((note) => note.id !== deleting.id);
		toolsOpen = false;
		const next = notes[0];
		if (next) selectNote(next, false);
		else void newNote(false);

		if (!demoMode && !deleting.id.startsWith('local-')) {
			try {
				await deleteNote(deleting.id);
			} catch {
				demoMode = true;
			}
		}
		pushToast('Note moved to trash.', 'success');
	}

	function formatListTime(value: string) {
		const date = new Date(value);
		const now = new Date();
		const today = date.toDateString() === now.toDateString();
		const yesterday = new Date(now.getTime() - 86_400_000).toDateString() === date.toDateString();
		if (today) return date.toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' });
		if (yesterday) return 'Yesterday';
		return date.toLocaleDateString([], { month: 'short', day: 'numeric' });
	}

	async function focusSearch() {
		mobilePane = 'list';
		await tick();
		searchInput?.focus();
		searchInput?.select();
	}

	function handleKeyboard(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			if (focusMode) {
				focusMode = false;
				return;
			}
			if (document.activeElement === searchInput) {
				if (searchQuery) searchQuery = '';
				else searchInput.blur();
			}
			toolsOpen = false;
			tagEditorOpen = false;
		}
		if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'k') {
			event.preventDefault();
			void focusSearch();
		}
		if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 's') {
			event.preventDefault();
			if (saveTimer) clearTimeout(saveTimer);
			void saveCurrentNote();
		}
		if ((event.metaKey || event.ctrlKey) && event.shiftKey && event.key.toLowerCase() === 'n') {
			event.preventDefault();
			void newNote();
		}
		if ((event.metaKey || event.ctrlKey) && event.shiftKey && event.key.toLowerCase() === 'f') {
			event.preventDefault();
			focusMode = !focusMode;
		}
	}
</script>

<svelte:window on:keydown={handleKeyboard} />

<svelte:head>
	<title>Notes · MindVault</title>
	<meta
		name="description"
		content="A calm, local-first notes workspace for capturing and connecting knowledge."
	/>
</svelte:head>

<div class="workbench" class:focus-mode={focusMode}>
	<aside class="primary-sidebar" aria-label="Primary navigation">
		<div class="brand">
			<div class="brand-mark" aria-hidden="true">MV</div>
			<div class="brand-copy">
				<strong>MindVault</strong>
				<span>Workspace</span>
			</div>
		</div>

		<nav class="primary-nav">
			{#each navGroups as group (group.label ?? 'primary')}
				<div class="nav-group">
					{#if group.label}<p>{group.label}</p>{/if}
					{#each group.items as item (item.label)}
						<a
							href={`${resolve(item.href)}${'query' in item ? item.query : ''}`}
							class:active={item.label === 'Notes'}
							aria-current={item.label === 'Notes' ? 'page' : undefined}
							title={item.label}
						>
							<svelte:component this={item.icon} size={19} strokeWidth={1.8} aria-hidden="true" />
							<span>{item.label}</span>
							{#if item.label === 'Notes'}<i aria-hidden="true"></i>{/if}
						</a>
					{/each}
				</div>
			{/each}
		</nav>

		<div class="sync-footer">
			<div class="sync-state">
				<span class="sync-dot" aria-hidden="true"></span>
				<div>
					<strong>{demoMode ? 'Local workspace' : 'All synced'}</strong>
					<span>{demoMode ? 'Private preview' : 'Local-first · just now'}</span>
				</div>
			</div>
			<a href={resolve('/settings')} aria-label="Settings" title="Settings">
				<Settings2 size={19} strokeWidth={1.8} />
			</a>
		</div>
	</aside>

	<section class="notes-panel" class:mobile-hidden={mobilePane !== 'list'} aria-label="Notes list">
		<div class="notes-panel-header">
			<div class="panel-title">
				<h1>Notes</h1>
				<button
					aria-label="Filter notes"
					title="Filter notes"
					class:active={filtersExpanded || activeTagFilter !== null}
					on:click={() => (filtersExpanded = !filtersExpanded)}
					aria-controls="note-filters"
					aria-expanded={filtersExpanded}
				>
					<SlidersHorizontal size={18} strokeWidth={1.8} />
				</button>
			</div>

			<div class="new-note-row">
				<button
					class="new-note"
					on:click={() => void newNote()}
					aria-keyshortcuts="Meta+Shift+N Control+Shift+N"
				>
					<Plus size={19} strokeWidth={2} />
					<span>New note</span>
				</button>
			</div>

			<div class="note-search" role="search">
				<Search size={17} strokeWidth={1.8} aria-hidden="true" />
				<input
					bind:this={searchInput}
					bind:value={searchQuery}
					type="search"
					placeholder="Search notes…"
					aria-label="Search notes"
					aria-keyshortcuts="Meta+K Control+K"
					autocomplete="off"
				/>
				{#if searchQuery}
					<button
						class="clear-search"
						aria-label="Clear note search"
						title="Clear search"
						on:click={() => {
							searchQuery = '';
							searchInput?.focus();
						}}
					>
						<X size={15} strokeWidth={2} />
					</button>
				{:else}
					<span class="search-shortcut" aria-hidden="true"
						><Command size={12} strokeWidth={2} />K</span
					>
				{/if}
			</div>

			<div class="list-tabs" role="tablist" aria-label="Note filters">
				<button
					class:active={activeTab === 'all'}
					on:click={() => (activeTab = 'all')}
					role="tab"
					aria-selected={activeTab === 'all'}>All</button
				>
				<button
					class:active={activeTab === 'pinned'}
					on:click={() => (activeTab = 'pinned')}
					role="tab"
					aria-selected={activeTab === 'pinned'}>Pinned</button
				>
				<button
					class:active={activeTab === 'recent'}
					on:click={() => (activeTab = 'recent')}
					role="tab"
					aria-selected={activeTab === 'recent'}>Recent</button
				>
			</div>

			{#if filtersExpanded}
				<div class="filter-panel" id="note-filters">
					<div class="filter-panel-heading">
						<div>
							<strong>Filter by tag</strong>
							<span
								>{filteredNotes.length} {filteredNotes.length === 1 ? 'note' : 'notes'} shown</span
							>
						</div>
						{#if hasActiveFilters}
							<button on:click={clearFilters}>Clear all</button>
						{/if}
					</div>
					{#if availableTags.length}
						<div class="tag-filter-list" aria-label="Available tag filters">
							{#each availableTags as tag (tag)}
								<button
									class:active={activeTagFilter === normalizeTag(tag)}
									aria-pressed={activeTagFilter === normalizeTag(tag)}
									on:click={() => toggleTagFilter(tag)}
								>
									<span>{tag}</span>
									<small>{tagCounts.get(tag) ?? 0}</small>
								</button>
							{/each}
						</div>
					{:else}
						<p class="no-tags">Add a tag to a note to filter your workspace.</p>
					{/if}
				</div>
			{/if}
		</div>

		<div class="note-list" aria-live="polite">
			{#if loading}
				<div class="list-state">
					<span class="loading-ring"></span>
					<p>Opening your vault…</p>
				</div>
			{:else if filteredNotes.length === 0}
				<div class="list-state">
					<FileText size={24} strokeWidth={1.5} />
					<p>No notes match this view.</p>
					<button on:click={clearFilters}>Show all notes</button>
				</div>
			{:else}
				{#each filteredNotes as note (note.id)}
					<button
						class="note-item"
						class:active={note.id === selectedNote?.id}
						on:click={() => selectNote(note)}
						aria-pressed={note.id === selectedNote?.id}
					>
						<div class="note-item-title-row">
							<strong>{note.title || 'Untitled'}</strong>
							{#if note.pinned}<Pin size={12} strokeWidth={2} aria-label="Pinned" />{/if}
						</div>
						<div class="note-item-meta">
							<div>
								{#each note.tags.slice(0, 2) as tag (tag)}
									<span>{tag}</span>
								{/each}
							</div>
							<time datetime={note.updated_at}>{formatListTime(note.updated_at)}</time>
						</div>
					</button>
				{/each}
			{/if}
		</div>
	</section>

	<main class="editor-panel" class:mobile-hidden={mobilePane !== 'editor'}>
		<header class="editor-header">
			<div class="editor-context">
				<button
					class="mobile-back"
					on:click={() => (mobilePane = 'list')}
					aria-label="Back to notes"
				>
					<ArrowLeft size={19} strokeWidth={1.8} />
				</button>
				<FileText size={19} strokeWidth={1.7} aria-hidden="true" />
				<span>{title.trim() || 'Untitled note'}</span>
				<div class="save-state" class:saving={savedStatus === 'saving'}>
					{#if savedStatus === 'saving'}
						<span class="saving-pulse" aria-hidden="true"></span>
					{:else}
						<CheckCircle2 size={14} strokeWidth={1.8} aria-hidden="true" />
					{/if}
					<span>{saveLabel}</span>
				</div>
			</div>

			<div class="tools-wrap">
				<button
					class="tools-button"
					on:click={() => (toolsOpen = !toolsOpen)}
					aria-haspopup="menu"
					aria-expanded={toolsOpen}
				>
					<SlidersHorizontal size={18} strokeWidth={1.8} />
					<span>Tools</span>
					<ChevronDown size={15} strokeWidth={1.8} />
				</button>
				{#if toolsOpen}
					<div class="tools-menu" role="menu">
						{#if selectedNote}
							<button role="menuitem" on:click={togglePin}>
								<Pin size={16} strokeWidth={1.8} />
								{selectedNote.pinned ? 'Unpin note' : 'Pin note'}
							</button>
						{/if}
						<a href={`${resolve('/notes')}?view=graph`} role="menuitem">
							<Network size={16} strokeWidth={1.8} />
							Open knowledge graph
						</a>
						<a href={`${resolve('/notes')}?view=canvas`} role="menuitem">
							<Expand size={16} strokeWidth={1.8} />
							Open canvas
						</a>
						<a href={resolve('/review')} role="menuitem">
							<History size={16} strokeWidth={1.8} />
							Review recent changes
						</a>
						{#if selectedNote}
							<div class="menu-separator"></div>
							<button class="danger" role="menuitem" on:click={removeCurrentNote}>
								<Trash2 size={16} strokeWidth={1.8} />
								Move to trash
							</button>
						{/if}
					</div>
				{/if}
			</div>
		</header>

		<div class="editor-scroll">
			<article class="document">
				<textarea
					class="document-title"
					bind:this={titleInput}
					bind:value={title}
					on:input={scheduleSave}
					placeholder="Untitled note"
					aria-label="Note title"
					rows="2"
				></textarea>

				<div class="tag-row">
					{#each activeTags as tag (tag)}
						<div class="editor-tag">
							<button
								class="tag"
								class:active={activeTagFilter === normalizeTag(tag)}
								on:click={() => toggleTagFilter(tag, true)}
								aria-pressed={activeTagFilter === normalizeTag(tag)}
								title={`Filter notes by ${tag}`}
							>
								{tag}
							</button>
							<button
								class="remove-tag"
								on:click={() => removeTag(tag)}
								aria-label={`Remove ${tag} tag from this note`}
								title={`Remove ${tag} tag`}
							>
								<X size={13} strokeWidth={2} />
							</button>
						</div>
					{/each}
					{#if tagEditorOpen}
						<form on:submit|preventDefault={addTag}>
							<input
								bind:this={tagInput}
								bind:value={tagDraft}
								placeholder="tag"
								aria-label="New tag"
								on:blur={() => !tagDraft.trim() && (tagEditorOpen = false)}
							/>
							<button type="submit" aria-label="Add tag"><Check size={14} strokeWidth={2} /></button
							>
						</form>
					{:else}
						<button class="add-tag" on:click={openTagEditor}>
							<Plus size={15} strokeWidth={1.8} />
							Add tag
						</button>
					{/if}
				</div>

				<div class="document-rule"></div>

				{#key selectedNote?.id ?? 'new-note'}
					<FocusedNoteEditor bind:markdown on:change={scheduleSave} />
				{/key}

				<p class="command-hint">Type ‘/’ for commands</p>
			</article>
		</div>

		<footer class="editor-footer">
			<div class="footer-local">
				<CheckCircle2 size={14} strokeWidth={1.8} />
				<span
					>{demoMode
						? 'Stored in this private session'
						: `Saved ${savedAt.toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' })}`}</span
				>
			</div>
			<div class="editor-stats">
				<span>{wordCount} words</span>
				<span>{characterCount.toLocaleString()} characters</span>
				<i></i>
				<button
					class="focus-toggle"
					class:active={focusMode}
					aria-label={focusMode ? 'Exit focus mode' : 'Enter focus mode'}
					title={focusMode ? 'Exit focus mode (Esc)' : 'Enter focus mode (⌘⇧F)'}
					aria-pressed={focusMode}
					aria-keyshortcuts="Meta+Shift+F Control+Shift+F"
					on:click={() => (focusMode = !focusMode)}
				>
					{#if focusMode}
						<Minimize2 size={17} strokeWidth={1.8} />
					{:else}
						<Expand size={17} strokeWidth={1.8} />
					{/if}
				</button>
			</div>
		</footer>
	</main>
</div>

<style>
	:global(html) {
		background: #0c0c0f;
	}

	:global(body) {
		margin: 0;
		overflow: hidden;
	}

	:global(*) {
		box-sizing: border-box;
	}

	.workbench {
		display: grid;
		grid-template-columns: 272px 345px minmax(0, 1fr);
		width: 100%;
		height: 100dvh;
		min-height: 640px;
		overflow: hidden;
		background: radial-gradient(circle at 85% 12%, rgb(38 35 58 / 10%), transparent 35%), #0c0c0f;
		color: #f4f4f6;
		font-family: 'Plus Jakarta Sans', system-ui, sans-serif;
	}

	.workbench.focus-mode {
		grid-template-columns: minmax(0, 1fr);
	}

	.workbench.focus-mode .primary-sidebar,
	.workbench.focus-mode .notes-panel {
		display: none;
	}

	.workbench.focus-mode .editor-panel {
		grid-column: 1;
	}

	button,
	input,
	textarea {
		font: inherit;
	}

	button,
	a {
		-webkit-tap-highlight-color: transparent;
	}

	button:focus-visible,
	a:focus-visible,
	input:focus-visible,
	textarea:focus-visible {
		outline: 2px solid #8066ff;
		outline-offset: 2px;
	}

	.primary-sidebar,
	.notes-panel {
		border-right: 1px solid #25252b;
	}

	.primary-sidebar {
		display: flex;
		min-width: 0;
		flex-direction: column;
		padding: 30px 26px 25px;
		background: linear-gradient(145deg, #101014 0%, #0d0d11 60%, #0b0b0e 100%);
	}

	.brand {
		display: flex;
		align-items: center;
		gap: 14px;
		margin-bottom: 34px;
	}

	.brand-mark {
		display: grid;
		width: 46px;
		height: 46px;
		flex: 0 0 46px;
		place-items: center;
		border: 1px solid rgb(255 255 255 / 10%);
		border-radius: 12px;
		background: linear-gradient(145deg, #8877ff 0%, #6544fa 52%, #4631d7 100%);
		box-shadow:
			0 10px 28px rgb(70 49 215 / 28%),
			inset 0 1px 0 rgb(255 255 255 / 24%);
		color: white;
		font-size: 20px;
		font-weight: 700;
		letter-spacing: -0.7px;
	}

	.brand-copy {
		display: flex;
		min-width: 0;
		flex-direction: column;
	}

	.brand-copy strong {
		font-size: 20px;
		font-weight: 700;
		letter-spacing: -0.6px;
	}

	.brand-copy span {
		margin-top: 2px;
		color: #777783;
		font-size: 10px;
		font-weight: 600;
		letter-spacing: 1.35px;
		text-transform: uppercase;
	}

	.primary-nav {
		flex: 1;
		overflow-y: auto;
		scrollbar-width: none;
	}

	.primary-nav::-webkit-scrollbar {
		display: none;
	}

	.nav-group {
		margin-bottom: 27px;
		padding-bottom: 21px;
		border-bottom: 1px solid #24242a;
	}

	.nav-group:last-child {
		border-bottom: 0;
	}

	.nav-group p {
		margin: 0 0 10px 2px;
		color: #696975;
		font-size: 11px;
		font-weight: 600;
		letter-spacing: 1.25px;
		text-transform: uppercase;
	}

	.nav-group a {
		position: relative;
		display: flex;
		height: 47px;
		align-items: center;
		gap: 14px;
		margin: 1px -13px;
		padding: 0 22px;
		border-radius: 8px;
		color: #9c9ca9;
		font-size: 15px;
		font-weight: 500;
		text-decoration: none;
		transition:
			background 160ms ease,
			color 160ms ease;
	}

	.nav-group a:hover {
		background: #17171d;
		color: #eeeef2;
	}

	.nav-group a.active {
		background: linear-gradient(90deg, rgb(105 76 255 / 18%), rgb(105 76 255 / 4%));
		color: #eceaf9;
		box-shadow: inset 3px 0 #7554ff;
	}

	.nav-group a.active i {
		position: absolute;
		right: 22px;
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background: #7657ff;
		box-shadow: 0 0 9px #7251ff;
	}

	.sync-footer {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding-top: 22px;
		border-top: 1px solid #24242a;
	}

	.sync-state {
		display: flex;
		align-items: flex-start;
		gap: 10px;
	}

	.sync-dot {
		width: 8px;
		height: 8px;
		margin-top: 5px;
		border-radius: 50%;
		background: #43d393;
		box-shadow: 0 0 10px rgb(67 211 147 / 30%);
	}

	.sync-state div {
		display: flex;
		flex-direction: column;
	}

	.sync-state strong {
		color: #bebec7;
		font-size: 12px;
		font-weight: 600;
	}

	.sync-state span {
		margin-top: 3px;
		color: #656570;
		font-size: 11px;
	}

	.sync-footer > a {
		display: grid;
		width: 34px;
		height: 34px;
		place-items: center;
		border-radius: 8px;
		color: #90909c;
		transition: 160ms ease;
	}

	.sync-footer > a:hover {
		background: #1b1b21;
		color: white;
	}

	.notes-panel {
		display: flex;
		min-width: 0;
		flex-direction: column;
		overflow: hidden;
		background: linear-gradient(165deg, #111116 0%, #0d0d11 66%);
	}

	.notes-panel-header {
		flex: 0 0 auto;
		padding: 35px 28px 0;
	}

	.panel-title {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 20px;
	}

	.panel-title h1 {
		margin: 0;
		color: #f4f4f6;
		font-size: 23px;
		font-weight: 650;
		letter-spacing: -0.55px;
	}

	.panel-title button {
		display: grid;
		width: 42px;
		height: 42px;
		place-items: center;
		border: 1px solid transparent;
		border-radius: 9px;
		background: transparent;
		color: #92929d;
		cursor: pointer;
	}

	.panel-title button:hover {
		border-color: #32323a;
		background: #18181d;
		color: white;
	}

	.panel-title button.active {
		border-color: #49405f;
		background: #211c30;
		color: #b5a8ff;
	}

	.new-note-row {
		display: block;
	}

	.new-note {
		display: flex;
		width: 100%;
		height: 43px;
		align-items: center;
		justify-content: center;
		gap: 9px;
		border: 0;
		border-radius: 8px;
		background: linear-gradient(110deg, #6948ff, #7857ff);
		box-shadow:
			0 8px 22px rgb(91 62 229 / 20%),
			inset 0 1px 0 rgb(255 255 255 / 18%);
		color: white;
		font-size: 15px;
		font-weight: 600;
		cursor: pointer;
		transition:
			transform 150ms ease,
			filter 150ms ease;
	}

	.new-note:hover {
		filter: brightness(1.08);
		transform: translateY(-1px);
	}

	.note-search {
		display: grid;
		grid-template-columns: 18px 1fr auto;
		height: 41px;
		align-items: center;
		gap: 9px;
		margin-top: 13px;
		padding: 0 12px;
		border: 1px solid #303037;
		border-radius: 8px;
		background: #0e0e12;
		color: #777782;
	}

	.note-search:focus-within {
		border-color: #6651c7;
		box-shadow: 0 0 0 3px rgb(110 84 230 / 12%);
	}

	.note-search input {
		width: 100%;
		min-width: 0;
		border: 0;
		outline: 0;
		background: transparent;
		color: #e1e1e6;
		font-size: 13px;
	}

	.note-search input::-webkit-search-cancel-button {
		display: none;
	}

	.note-search input::placeholder {
		color: #6e6e79;
	}

	.search-shortcut {
		display: flex;
		align-items: center;
		gap: 2px;
		color: #676772;
		font-size: 11px;
	}

	.clear-search {
		display: grid;
		width: 28px;
		height: 28px;
		place-items: center;
		border: 0;
		border-radius: 6px;
		background: transparent;
		color: #777782;
		cursor: pointer;
	}

	.clear-search:hover {
		background: #1c1c22;
		color: #f4f4f6;
	}

	.list-tabs {
		display: flex;
		gap: 25px;
		height: 47px;
		align-items: end;
		border-bottom: 1px solid #24242a;
	}

	.list-tabs button {
		position: relative;
		height: 40px;
		border: 0;
		background: transparent;
		color: #777783;
		font-size: 13px;
		font-weight: 500;
		cursor: pointer;
	}

	.list-tabs button::after {
		position: absolute;
		right: 0;
		bottom: -1px;
		left: 0;
		height: 2px;
		border-radius: 3px;
		background: transparent;
		content: '';
	}

	.list-tabs button.active {
		color: #e9e9ee;
	}

	.list-tabs button.active::after {
		background: #7354ff;
		box-shadow: 0 0 8px rgb(115 84 255 / 45%);
	}

	.filter-panel {
		padding: 13px 0 14px;
		border-bottom: 1px solid #24242a;
	}

	.filter-panel-heading {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: 12px;
		margin-bottom: 11px;
	}

	.filter-panel-heading > div {
		display: flex;
		min-width: 0;
		flex-direction: column;
		gap: 3px;
	}

	.filter-panel-heading strong {
		color: #c8c8d0;
		font-size: 11px;
		font-weight: 600;
	}

	.filter-panel-heading span,
	.no-tags {
		color: #6f6f7a;
		font-size: 10px;
	}

	.filter-panel-heading button,
	.list-state button {
		border: 0;
		background: transparent;
		color: #a99cff;
		font-size: 11px;
		cursor: pointer;
	}

	.tag-filter-list {
		display: flex;
		max-height: 104px;
		flex-wrap: wrap;
		gap: 6px;
		overflow-y: auto;
		padding: 1px 2px 1px 0;
		scrollbar-width: thin;
		scrollbar-color: #313138 transparent;
	}

	.tag-filter-list button {
		display: inline-flex;
		height: 27px;
		align-items: center;
		gap: 7px;
		border: 1px solid #30303a;
		border-radius: 6px;
		background: #1b1b22;
		padding: 0 7px 0 9px;
		color: #9d9da9;
		font-size: 10px;
		cursor: pointer;
		transition:
			border-color 140ms ease,
			background 140ms ease,
			color 140ms ease;
	}

	.tag-filter-list button:hover {
		border-color: #494257;
		background: #24202f;
		color: #d5d0ef;
	}

	.tag-filter-list button.active {
		border-color: #6652bd;
		background: #2a2242;
		color: #c4b8ff;
	}

	.tag-filter-list small {
		display: grid;
		min-width: 16px;
		height: 16px;
		place-items: center;
		border-radius: 5px;
		background: rgb(255 255 255 / 6%);
		color: #777782;
		font-size: 9px;
	}

	.tag-filter-list button.active small {
		background: rgb(140 113 255 / 18%);
		color: #b8aaff;
	}

	.no-tags {
		margin: 0;
		line-height: 1.45;
	}

	.note-list {
		min-height: 0;
		flex: 1;
		overflow-y: auto;
		padding: 10px 19px 28px;
		scrollbar-width: thin;
		scrollbar-color: #313138 transparent;
	}

	.note-item {
		position: relative;
		display: flex;
		width: 100%;
		min-height: 88px;
		flex-direction: column;
		justify-content: center;
		margin: 0;
		padding: 13px 13px 12px 22px;
		border: 1px solid transparent;
		border-bottom-color: #25252b;
		border-radius: 0;
		background: transparent;
		color: #d8d8de;
		text-align: left;
		cursor: pointer;
		transition:
			border-color 140ms ease,
			background 140ms ease;
	}

	.note-item::before {
		position: absolute;
		top: 9px;
		bottom: 9px;
		left: 0;
		width: 3px;
		border-radius: 3px;
		background: transparent;
		content: '';
	}

	.note-item:hover {
		background: #15151a;
	}

	.note-item.active {
		min-height: 107px;
		margin: 1px 0 4px;
		border-color: #303038;
		border-radius: 8px;
		background: linear-gradient(115deg, #1c1c22, #19191f);
		box-shadow: inset 0 1px 0 rgb(255 255 255 / 2%);
	}

	.note-item.active::before {
		background: #7655ff;
		box-shadow: 0 0 9px rgb(118 85 255 / 35%);
	}

	.note-item-title-row {
		display: flex;
		align-items: flex-start;
		gap: 7px;
	}

	.note-item-title-row strong {
		display: -webkit-box;
		overflow: hidden;
		color: #e1e1e6;
		font-size: 15px;
		font-weight: 500;
		line-height: 1.45;
		-webkit-box-orient: vertical;
		-webkit-line-clamp: 2;
		line-clamp: 2;
	}

	.note-item-title-row :global(svg) {
		flex: 0 0 auto;
		margin-top: 4px;
		color: #9a86ff;
	}

	.note-item-meta {
		display: flex;
		align-items: end;
		justify-content: space-between;
		gap: 10px;
		margin-top: 8px;
	}

	.note-item-meta > div {
		display: flex;
		min-width: 0;
		flex-wrap: wrap;
		gap: 5px;
	}

	.note-item-meta span {
		border: 1px solid #30303a;
		border-radius: 5px;
		background: #24232d;
		padding: 2px 7px;
		color: #a89bd8;
		font-size: 10px;
		line-height: 1.25;
	}

	.note-item-meta time {
		flex: 0 0 auto;
		color: #73737f;
		font-size: 10px;
	}

	.list-state {
		display: flex;
		height: 240px;
		align-items: center;
		justify-content: center;
		flex-direction: column;
		gap: 12px;
		color: #747480;
		text-align: center;
	}

	.list-state p {
		margin: 0;
		font-size: 13px;
	}

	.loading-ring {
		width: 22px;
		height: 22px;
		border: 2px solid #32323a;
		border-top-color: #7456ff;
		border-radius: 50%;
		animation: spin 800ms linear infinite;
	}

	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}

	.editor-panel {
		position: relative;
		display: grid;
		min-width: 0;
		grid-template-rows: 90px minmax(0, 1fr) 48px;
		overflow: hidden;
		background: radial-gradient(circle at 64% 5%, rgb(48 42 74 / 8%), transparent 32%), #0c0c0f;
	}

	.editor-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 20px;
		margin: 0 31px;
		border-bottom: 1px solid #232329;
	}

	.editor-context {
		display: flex;
		min-width: 0;
		align-items: center;
		gap: 12px;
		color: #9a9aa5;
	}

	.editor-context > span:first-of-type {
		overflow: hidden;
		max-width: 390px;
		color: #c7c7cf;
		font-size: 13px;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.save-state {
		display: flex;
		align-items: center;
		gap: 5px;
		color: #666671;
		font-size: 11px;
	}

	.save-state.saving {
		color: #8a839e;
	}

	.saving-pulse {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: #8b73ff;
		animation: pulse 900ms ease-in-out infinite;
	}

	@keyframes pulse {
		50% {
			opacity: 0.35;
			transform: scale(0.8);
		}
	}

	.tools-wrap {
		position: relative;
	}

	.tools-button {
		display: flex;
		height: 40px;
		align-items: center;
		gap: 10px;
		padding: 0 14px;
		border: 1px solid #313138;
		border-radius: 9px;
		background: #101014;
		color: #ccccd3;
		font-size: 13px;
		cursor: pointer;
	}

	.tools-button:hover {
		border-color: #484851;
		background: #17171c;
	}

	.tools-menu {
		position: absolute;
		z-index: 20;
		top: 48px;
		right: 0;
		width: 220px;
		padding: 7px;
		border: 1px solid #32323a;
		border-radius: 11px;
		background: #17171c;
		box-shadow: 0 18px 48px rgb(0 0 0 / 45%);
	}

	.tools-menu button,
	.tools-menu a {
		display: flex;
		width: 100%;
		height: 38px;
		align-items: center;
		gap: 10px;
		padding: 0 10px;
		border: 0;
		border-radius: 7px;
		background: transparent;
		color: #bcbcc5;
		font-size: 12px;
		text-align: left;
		text-decoration: none;
		cursor: pointer;
	}

	.tools-menu button:hover,
	.tools-menu a:hover {
		background: #23232a;
		color: white;
	}

	.tools-menu .danger {
		color: #f28f96;
	}

	.menu-separator {
		height: 1px;
		margin: 6px 4px;
		background: #303037;
	}

	.mobile-back {
		display: none;
		border: 0;
		background: transparent;
		color: #9a9aa5;
		cursor: pointer;
	}

	.editor-scroll {
		overflow-y: auto;
		scrollbar-width: thin;
		scrollbar-color: #303038 transparent;
	}

	.document {
		width: min(100%, 820px);
		margin: 0 auto;
		padding: 76px 45px 100px;
	}

	.document-title {
		display: block;
		width: 100%;
		max-width: 620px;
		height: 114px;
		overflow: hidden;
		border: 0;
		outline: 0;
		background: transparent;
		color: #f2f2f4;
		font-family: 'Iowan Old Style', 'Palatino Linotype', 'Book Antiqua', Georgia, serif;
		font-size: clamp(37px, 3.25vw, 48px);
		font-weight: 700;
		line-height: 1.18;
		letter-spacing: -1.2px;
		resize: none;
	}

	.document-title::placeholder {
		color: #4e4e57;
	}

	.tag-row {
		display: flex;
		min-height: 31px;
		align-items: center;
		flex-wrap: wrap;
		gap: 8px;
		margin-top: 27px;
	}

	.editor-tag {
		display: inline-flex;
		height: 29px;
		align-items: stretch;
		overflow: hidden;
		border: 1px solid #302d44;
		border-radius: 6px;
		background: #201d31;
		color: #a995ff;
	}

	.editor-tag:nth-child(even) {
		border-color: #293143;
		background: #1c2532;
		color: #9bb7ef;
	}

	.tag,
	.remove-tag {
		border: 0;
		background: transparent;
		color: inherit;
		cursor: pointer;
	}

	.tag {
		padding: 0 8px 0 10px;
		font-size: 11px;
	}

	.tag:hover,
	.tag.active {
		background: rgb(126 97 255 / 14%);
		color: #c2b6ff;
	}

	.remove-tag {
		display: grid;
		width: 27px;
		place-items: center;
		border-left: 1px solid rgb(255 255 255 / 7%);
		color: #736c8e;
	}

	.remove-tag:hover {
		background: rgb(255 255 255 / 7%);
		color: #d1c9ef;
	}

	.add-tag {
		display: flex;
		align-items: center;
		gap: 6px;
		border: 0;
		background: transparent;
		padding: 5px 7px;
		color: #8a8a96;
		font-size: 12px;
		cursor: pointer;
	}

	.add-tag:hover {
		color: #d8d8df;
	}

	.tag-row form {
		display: flex;
		height: 30px;
		align-items: center;
		border: 1px solid #56469a;
		border-radius: 6px;
		background: #16141f;
	}

	.tag-row form input {
		width: 88px;
		border: 0;
		outline: 0;
		background: transparent;
		padding: 0 7px;
		color: #d8d2f5;
		font-size: 11px;
	}

	.tag-row form button {
		display: grid;
		width: 28px;
		height: 28px;
		place-items: center;
		border: 0;
		background: transparent;
		color: #a995ff;
		cursor: pointer;
	}

	.document-rule {
		height: 1px;
		margin: 19px 0 33px;
		background: #232329;
	}

	.command-hint {
		margin: 74px 0 0;
		color: #656570;
		font-size: 12px;
	}

	.editor-footer {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0 36px;
		border-top: 1px solid #232329;
		background: rgb(12 12 15 / 92%);
		color: #777782;
		font-size: 11px;
		backdrop-filter: blur(12px);
	}

	.footer-local,
	.editor-stats {
		display: flex;
		align-items: center;
		gap: 20px;
	}

	.footer-local {
		gap: 7px;
		color: #686874;
	}

	.editor-stats i {
		width: 1px;
		height: 18px;
		background: #35353c;
	}

	.editor-stats button {
		display: flex;
		align-items: center;
		gap: 5px;
		border: 0;
		background: transparent;
		color: #9a9aa5;
		font-size: 11px;
		cursor: pointer;
	}

	.focus-toggle {
		width: 32px;
		height: 32px;
		justify-content: center;
		border-radius: 7px !important;
	}

	.focus-toggle:hover,
	.focus-toggle.active {
		background: #202028;
		color: #f4f4f6;
	}

	@media (max-width: 1180px) {
		.workbench {
			grid-template-columns: 78px 320px minmax(0, 1fr);
		}

		.primary-sidebar {
			align-items: center;
			padding: 25px 12px;
		}

		.brand {
			margin-bottom: 28px;
		}

		.brand-copy,
		.nav-group p,
		.nav-group a span,
		.nav-group a.active i,
		.sync-state,
		.sync-footer > a {
			display: none;
		}

		.nav-group {
			width: 54px;
			padding-bottom: 16px;
		}

		.nav-group a {
			width: 46px;
			height: 44px;
			justify-content: center;
			margin: 2px auto;
			padding: 0;
		}

		.document {
			padding-right: 36px;
			padding-left: 36px;
		}

		.editor-context > span:first-of-type {
			max-width: 240px;
		}
	}

	@media (max-width: 820px) {
		.workbench {
			grid-template-columns: 1fr;
		}

		.primary-sidebar {
			display: none;
		}

		.notes-panel,
		.editor-panel {
			grid-column: 1;
			grid-row: 1;
		}

		.mobile-hidden {
			display: none;
		}

		.notes-panel {
			border-right: 0;
		}

		.notes-panel-header {
			padding-top: 25px;
		}

		.editor-panel {
			grid-template-rows: 70px minmax(0, 1fr) 44px;
		}

		.editor-header {
			margin: 0 18px;
		}

		.mobile-back {
			display: grid;
			width: 34px;
			height: 34px;
			place-items: center;
		}

		.editor-context > :global(svg):not(.mobile-back :global(svg)),
		.editor-context > span:first-of-type,
		.footer-local {
			display: none;
		}

		.tools-button span {
			display: none;
		}

		.tools-button {
			width: 43px;
			justify-content: center;
			padding: 0;
		}

		.tools-button :global(svg:last-child) {
			display: none;
		}

		.document {
			padding: 46px 24px 86px;
		}

		.document-title {
			font-size: 36px;
		}

		.editor-footer {
			justify-content: flex-end;
			padding: 0 18px;
		}

		.editor-stats {
			gap: 12px;
		}
	}

	@media (prefers-reduced-motion: reduce) {
		* {
			scroll-behavior: auto !important;
			animation-duration: 0.01ms !important;
			animation-iteration-count: 1 !important;
			transition-duration: 0.01ms !important;
		}
	}
</style>
