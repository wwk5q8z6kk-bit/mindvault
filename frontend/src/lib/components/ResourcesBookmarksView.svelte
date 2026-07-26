<script lang="ts">
	import { onMount } from 'svelte';
	import { get } from 'svelte/store';
	import { page } from '$app/stores';
	import {
		createBookmarkNote,
		createBookmark,
		deleteBookmark,
		listBookmarks,
		normalizeBookmarkUrl,
		setBookmarkRead,
		updateBookmarkTags,
		type Bookmark,
		type CreateBookmarkPayload
	} from '$lib/api/bookmarks';
	import { enrichClip } from '$lib/api/clips';
	import { assistAutoTag, assistTransform } from '$lib/api/assist';
	import { activeNamespace } from '$lib/stores/namespace';
	import { pushToast } from '$lib/stores/toast';
	import EmptyState from '$lib/components/EmptyState.svelte';

	// Folder/Collection system
	const FOLDER_PREFIX = 'folder:';
	const FOLDER_STORAGE_KEY = 'mindvault-bookmark-folders';
	const FOLDER_COLORS = ['slate', 'red', 'orange', 'amber', 'yellow', 'lime', 'green', 'emerald', 'teal', 'cyan', 'sky', 'blue', 'indigo', 'violet', 'purple', 'fuchsia', 'pink', 'rose'];

	interface BookmarkFolder {
		id: string;
		name: string;
		color: string;
		createdAt: string;
	}

	let bookmarks: Bookmark[] = [];
	let loading = true;
	let filter: 'all' | 'unread' | 'read' = 'all';
	let searchQuery = '';

	let clipUrl = '';
	let clipTitle = '';
	let clipExcerpt = '';
	let clipTags = '';
	let clipSource = 'manual';
	let alsoCreateNote = false;
	let savingClip = false;
	let enrichingClip = false;
	let aiTagging = false;
	let aiSummarizing = false;
	let hasPrefilledClip = false;
	let bookmarkletCopying = false;
	let lastPrefillSignature = '';
	let dedupeConflict: { bookmark: Bookmark; payload: CreateBookmarkPayload } | null = null;
	let resolvingConflict = false;

	// Folder state
	let folders: BookmarkFolder[] = [];
	let selectedFolder: string | null = null;
	let showFolderModal = false;
	let showMoveModal = false;
	let moveTargetBookmarkId: string | null = null;
	let newFolderName = '';
	let newFolderColor = 'sky';
	let editingFolder: BookmarkFolder | null = null;
	let foldersSidebarCollapsed = false;

	function loadFolders() {
		try {
			const stored = localStorage.getItem(FOLDER_STORAGE_KEY);
			folders = stored ? JSON.parse(stored) : [];
		} catch {
			folders = [];
		}
	}

	function saveFolders() {
		localStorage.setItem(FOLDER_STORAGE_KEY, JSON.stringify(folders));
	}

	function createFolder() {
		if (!newFolderName.trim()) return;
		const folder: BookmarkFolder = {
			id: crypto.randomUUID(),
			name: newFolderName.trim(),
			color: newFolderColor,
			createdAt: new Date().toISOString()
		};
		folders = [...folders, folder];
		saveFolders();
		newFolderName = '';
		newFolderColor = 'sky';
		showFolderModal = false;
		pushToast(`Folder "${folder.name}" created`, 'success');
	}

	function deleteFolder(folderId: string) {
		const folder = folders.find((f) => f.id === folderId);
		if (!folder) return;
		folders = folders.filter((f) => f.id !== folderId);
		saveFolders();
		if (selectedFolder === folderId) selectedFolder = null;
		pushToast(`Folder "${folder.name}" deleted`, 'success');
	}

	function getFolderTag(folderId: string): string {
		return `${FOLDER_PREFIX}${folderId}`;
	}

	function getBookmarkFolderId(bookmark: Bookmark): string | null {
		const folderTag = bookmark.tags.find((tag) => tag.startsWith(FOLDER_PREFIX));
		return folderTag ? folderTag.slice(FOLDER_PREFIX.length) : null;
	}

	async function moveBookmarkToFolder(bookmarkId: string, folderId: string | null) {
		const bookmark = bookmarks.find((b) => b.id === bookmarkId);
		if (!bookmark) return;
		const existingFolderTag = bookmark.tags.find((tag) => tag.startsWith(FOLDER_PREFIX));
		let newTags = bookmark.tags.filter((tag) => !tag.startsWith(FOLDER_PREFIX));
		if (folderId) {
			newTags = [...newTags, getFolderTag(folderId)];
		}
		try {
			const updated = await updateBookmarkTags(bookmarkId, newTags);
			bookmarks = bookmarks.map((b) => (b.id === bookmarkId ? updated : b));
			const folder = folders.find((f) => f.id === folderId);
			pushToast(folderId ? `Moved to "${folder?.name}"` : 'Removed from folder', 'success');
		} catch {
			pushToast('Failed to move bookmark', 'danger');
		}
		showMoveModal = false;
		moveTargetBookmarkId = null;
	}

	function openMoveModal(bookmarkId: string) {
		moveTargetBookmarkId = bookmarkId;
		showMoveModal = true;
	}

	function getBookmarksInFolder(folderId: string | null): Bookmark[] {
		if (folderId === null) {
			return bookmarks.filter((b) => !b.tags.some((tag) => tag.startsWith(FOLDER_PREFIX)));
		}
		const folderTag = getFolderTag(folderId);
		return bookmarks.filter((b) => b.tags.includes(folderTag));
	}

	function countBookmarksInFolder(folderId: string | null): number {
		return getBookmarksInFolder(folderId).length;
	}

	$: allTags = [...new Set(bookmarks.flatMap((item) => item.tags ?? []))].filter((tag) => !tag.startsWith(FOLDER_PREFIX)).sort();
	$: displayed = bookmarks
		.filter((bookmark) => {
			// Folder filter
			if (selectedFolder !== null) {
				const folderTag = getFolderTag(selectedFolder);
				if (!bookmark.tags.includes(folderTag)) return false;
			}
			if (filter === 'unread' && bookmark.read) return false;
			if (filter === 'read' && !bookmark.read) return false;
			if (searchQuery.trim()) {
				const q = searchQuery.toLowerCase();
				return (
					bookmark.title.toLowerCase().includes(q) ||
					bookmark.url.toLowerCase().includes(q) ||
					bookmark.excerpt.toLowerCase().includes(q)
				);
			}
			return true;
		})
		.sort((a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime());

	$: {
		const params = $page.url.searchParams;
		const hasClipParams = ['url', 'title', 'text', 'tags'].some((key) => Boolean(params.get(key)));
		if (!hasClipParams) {
			lastPrefillSignature = '';
		} else {
			const signature = [
				params.get('url') ?? '',
				params.get('title') ?? '',
				params.get('text') ?? '',
				params.get('tags') ?? ''
			].join('|');
			if (signature !== lastPrefillSignature) {
				prefillFromQuery(params);
				lastPrefillSignature = signature;
			}
		}
	}

	onMount(async () => {
		loadFolders();
		await refreshBookmarks();
		prefillFromQuery(get(page).url.searchParams);
	});

	async function refreshBookmarks() {
		loading = true;
		try {
			bookmarks = await listBookmarks(400, get(activeNamespace));
		} catch {
			pushToast('Failed to load bookmarks', 'danger');
		} finally {
			loading = false;
		}
	}

	function prefillFromQuery(params: URLSearchParams) {
		const paramUrl = (params.get('url') ?? '').trim();
		const paramTitle = (params.get('title') ?? '').trim();
		const paramText = (params.get('text') ?? '').trim();
		const paramTags = (params.get('tags') ?? '').trim();
		if (!paramUrl && !paramTitle && !paramText && !paramTags) return;

		if (paramUrl) clipUrl = paramUrl;
		if (paramTitle) clipTitle = paramTitle;
		if (paramText) clipExcerpt = paramText;
		if (paramTags) clipTags = paramTags;
		hasPrefilledClip = true;
		clipSource = 'browser-handoff';
	}

	function parseTags(text: string): string[] {
		return [...new Set(text.split(/[\s,]+/).map((item) => item.trim().toLowerCase()).filter(Boolean))];
	}

	function clearClipForm() {
		clipUrl = '';
		clipTitle = '';
		clipExcerpt = '';
		clipTags = '';
		clipSource = 'manual';
		hasPrefilledClip = false;
	}

	async function saveClip() {
		const normalizedUrl = normalizeBookmarkUrl(clipUrl);
		if (!normalizedUrl) {
			pushToast('A valid URL is required.', 'warning');
			return;
		}
		const payload: CreateBookmarkPayload = {
			url: normalizedUrl,
			title: clipTitle.trim() || undefined,
			excerpt: clipExcerpt.trim() || undefined,
			tags: parseTags(clipTags),
			namespace: get(activeNamespace),
			clip_source: clipSource,
			create_note: alsoCreateNote
		};
		savingClip = true;
		try {
			const created = await createBookmark(payload, { dedupe: true });

			await refreshBookmarks();
			if (created.created) {
				dedupeConflict = null;
				pushToast(alsoCreateNote ? 'Clip saved and note created.' : 'Clip saved.', 'success');
				clearClipForm();
			} else if (created.note) {
				dedupeConflict = null;
				pushToast('Clip already existed. Linked note created.', 'success');
				searchQuery = created.bookmark.url;
			} else {
				dedupeConflict = {
					bookmark: created.bookmark,
					payload: { ...payload, create_note: false }
				};
				pushToast('Clip already exists. Choose an action below.', 'info');
				searchQuery = created.bookmark.url;
			}
		} catch {
			pushToast('Failed to save clip.', 'danger');
		} finally {
			savingClip = false;
		}
	}

	async function suggestTags() {
		const text = [clipTitle.trim(), clipExcerpt.trim(), clipUrl.trim()].filter(Boolean).join('\n');
		if (text.length < 8 || aiTagging) return;
		aiTagging = true;
		try {
			const result = await assistAutoTag({ text, existing_tags: allTags, limit: 6 });
			const merged = [...new Set([...parseTags(clipTags), ...result.tags])];
			clipTags = merged.join(' ');
			if (result.tags.length === 0) {
				pushToast('No tags suggested.', 'info');
			}
		} catch {
			pushToast('Auto-tagging failed.', 'danger');
		} finally {
			aiTagging = false;
		}
	}

	async function fetchMetadata() {
		const normalizedUrl = normalizeBookmarkUrl(clipUrl);
		if (!normalizedUrl) {
			pushToast('A valid URL is required.', 'warning');
			return;
		}
		if (enrichingClip) return;
		enrichingClip = true;
		try {
			const enriched = await enrichClip({ url: normalizedUrl });
			clipUrl = enriched.normalized_url || normalizedUrl;
			if (!clipTitle.trim() && enriched.title) {
				clipTitle = enriched.title;
			}
			if (!clipExcerpt.trim()) {
				if (enriched.description) {
					clipExcerpt = enriched.description;
				} else if (enriched.content_preview) {
					clipExcerpt = enriched.content_preview;
				}
			}
			if (enriched.suggested_tags.length > 0) {
				const merged = [...new Set([...parseTags(clipTags), ...enriched.suggested_tags])];
				clipTags = merged.join(' ');
			}
			if (clipSource === 'manual') {
				clipSource = 'metadata-enriched';
			}
			pushToast('Clip metadata fetched.', 'success');
		} catch {
			pushToast('Failed to fetch clip metadata.', 'danger');
		} finally {
			enrichingClip = false;
		}
	}

	async function summarizeExcerpt() {
		if (aiSummarizing) return;
		const body = clipExcerpt.trim();
		if (body.length < 20) {
			pushToast('Add some clip text before summarizing.', 'warning');
			return;
		}
		aiSummarizing = true;
		try {
			const prompt = [
				'Summarize this web clip in 3 concise bullet points. Keep important facts, remove fluff.',
				'',
				`Title: ${clipTitle || 'Untitled clip'}`,
				`URL: ${clipUrl || 'unknown'}`,
				'',
				'Clip text:',
				body.slice(0, 4000)
			].join('\n');
			const result = await assistTransform({ text: prompt, mode: 'summarize' });
			clipExcerpt = result.transformed_text.trim();
			pushToast('Clip summarized.', 'success');
		} catch {
			pushToast('Summary failed.', 'danger');
		} finally {
			aiSummarizing = false;
		}
	}

	async function pasteFromClipboard() {
		try {
			const text = (await navigator.clipboard.readText()).trim();
			if (!text) {
				pushToast('Clipboard is empty.', 'info');
				return;
			}
			const urlMatch = text.match(/https?:\/\/\S+/i);
			if (urlMatch) {
				clipUrl = urlMatch[0];
				const remainder = text.replace(urlMatch[0], '').trim();
				if (!clipExcerpt.trim() && remainder) {
					clipExcerpt = remainder;
				}
			} else if (/^[a-z0-9.-]+\.[a-z]{2,}(?:\/\S*)?$/i.test(text)) {
				clipUrl = text;
			} else {
				clipExcerpt = text;
			}
			pushToast('Pasted from clipboard.', 'success');
		} catch {
			pushToast('Clipboard access denied.', 'warning');
		}
	}

	async function copyBookmarklet() {
		bookmarkletCopying = true;
		try {
			const origin = window.location.origin;
			const bookmarklet =
				"javascript:(()=>{const u=encodeURIComponent(location.href);const t=encodeURIComponent(document.title);const s=encodeURIComponent((window.getSelection&&window.getSelection().toString())||document.querySelector('meta[name=description]')?.content||'');window.open('" +
				origin +
				"/bookmarks?url='+u+'&title='+t+'&text='+s,'_blank');})();";
			await navigator.clipboard.writeText(bookmarklet);
			pushToast('Bookmarklet copied.', 'success');
		} catch {
			pushToast('Failed to copy bookmarklet.', 'danger');
		} finally {
			bookmarkletCopying = false;
		}
	}

	async function toggleRead(bookmarkId: string) {
		const current = bookmarks.find((item) => item.id === bookmarkId);
		if (!current) return;
		try {
			const updated = await setBookmarkRead(bookmarkId, !current.read);
			bookmarks = bookmarks.map((item) => (item.id === bookmarkId ? updated : item));
		} catch {
			pushToast('Failed to update bookmark state.', 'danger');
		}
	}

	async function removeBookmark(bookmarkId: string) {
		try {
			await deleteBookmark(bookmarkId);
			bookmarks = bookmarks.filter((item) => item.id !== bookmarkId);
			pushToast('Bookmark removed.', 'success');
		} catch {
			pushToast('Failed to delete bookmark.', 'danger');
		}
	}

	function relativeTime(dateStr: string): string {
		const diffMin = Math.floor((Date.now() - new Date(dateStr).getTime()) / 60000);
		if (diffMin < 60) return `${diffMin}m ago`;
		const diffHr = Math.floor(diffMin / 60);
		if (diffHr < 24) return `${diffHr}h ago`;
		return `${Math.floor(diffHr / 24)}d ago`;
	}

	function getDomain(url: string): string {
		try {
			return new URL(normalizeBookmarkUrl(url)).hostname.replace(/^www\./, '');
		} catch {
			return url.slice(0, 60);
		}
	}

	function focusExistingConflict() {
		if (!dedupeConflict) return;
		searchQuery = dedupeConflict.bookmark.url;
	}

	async function createNoteForExistingConflict() {
		if (!dedupeConflict || resolvingConflict) return;
		resolvingConflict = true;
		try {
			const result = await createBookmarkNote(
				dedupeConflict.bookmark.id,
				{
					title: dedupeConflict.payload.title,
					excerpt: dedupeConflict.payload.excerpt,
					tags: dedupeConflict.payload.tags,
					namespace: dedupeConflict.payload.namespace
				},
				{ dedupe: true }
			);
			await refreshBookmarks();
			if (result.created) {
				pushToast('Linked note created for existing clip.', 'success');
				dedupeConflict = null;
			} else {
				pushToast('Linked note already exists for this clip.', 'info');
				dedupeConflict = null;
			}
		} catch {
			pushToast('Failed to create linked note.', 'danger');
		} finally {
			resolvingConflict = false;
		}
	}
</script>

<div class="flex gap-6">
	<!-- Folder Sidebar -->
	<aside class="hidden w-56 flex-shrink-0 md:block">
		<div class="sticky top-6 rounded-xl border border-slate-800/60 bg-slate-900/40 p-3">
			<div class="flex items-center justify-between">
				<h3 class="text-xs font-semibold text-slate-300">Folders</h3>
				<button
					class="rounded p-1 text-slate-500 hover:bg-slate-800 hover:text-slate-300"
					on:click={() => (showFolderModal = true)}
					title="Create folder"
				>
					<svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
					</svg>
				</button>
			</div>
			<nav class="mt-3 space-y-1">
				<button
					class="flex w-full items-center justify-between rounded-lg px-2 py-1.5 text-left text-xs transition {selectedFolder === null ? 'bg-sky-500/20 text-sky-300' : 'text-slate-400 hover:bg-slate-800 hover:text-slate-200'}"
					on:click={() => (selectedFolder = null)}
				>
					<span>All Bookmarks</span>
					<span class="text-[10px] text-slate-500">{bookmarks.length}</span>
				</button>
				{#each folders as folder (folder.id)}
					<button
						class="group flex w-full items-center justify-between rounded-lg px-2 py-1.5 text-left text-xs transition {selectedFolder === folder.id ? `bg-${folder.color}-500/20 text-${folder.color}-300` : 'text-slate-400 hover:bg-slate-800 hover:text-slate-200'}"
						on:click={() => (selectedFolder = folder.id)}
					>
						<div class="flex items-center gap-2">
							<span class="h-2 w-2 rounded-full bg-{folder.color}-400"></span>
							<span class="truncate">{folder.name}</span>
						</div>
						<div class="flex items-center gap-1">
							<span class="text-[10px] text-slate-500">{countBookmarksInFolder(folder.id)}</span>
							<span
								role="button"
								tabindex="0"
								class="hidden rounded p-0.5 text-slate-600 hover:bg-red-500/20 hover:text-red-300 group-hover:block"
								on:click|stopPropagation={() => deleteFolder(folder.id)}
								on:keydown|stopPropagation={(e) => { if (e.key === 'Enter' || e.key === ' ') deleteFolder(folder.id); }}
								title="Delete folder"
							>
								<svg class="h-3 w-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
									<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
								</svg>
							</span>
						</div>
					</button>
				{/each}
				{#if folders.length === 0}
					<p class="px-2 py-1 text-[10px] text-slate-600">No folders yet</p>
				{/if}
			</nav>
		</div>
	</aside>

	<!-- Main Content -->
	<div class="min-w-0 flex-1">
		<div class="flex items-start justify-between gap-3">
			<div>
				<h2 class="text-lg font-semibold text-white">
					{#if selectedFolder}
						{folders.find(f => f.id === selectedFolder)?.name ?? 'Folder'}
					{:else}
						Reading List & Clipper
					{/if}
				</h2>
				<p class="text-xs text-slate-400">
					{#if selectedFolder}
						{countBookmarksInFolder(selectedFolder)} bookmark{countBookmarksInFolder(selectedFolder) === 1 ? '' : 's'} in this folder
					{:else}
						Capture web content into MindVault with local-first storage and AI assist.
					{/if}
				</p>
			</div>
			<div class="flex gap-2">
				<button
					class="rounded-lg border border-slate-700 px-3 py-2 text-xs text-slate-200 hover:bg-slate-800 md:hidden"
					on:click={() => (showFolderModal = true)}
				>
					Folders
				</button>
				<button
					class="rounded-lg border border-slate-700 px-3 py-2 text-xs text-slate-200 hover:bg-slate-800 disabled:opacity-50"
					on:click={copyBookmarklet}
					disabled={bookmarkletCopying}
				>
					{bookmarkletCopying ? 'Copying...' : 'Copy bookmarklet'}
				</button>
			</div>
		</div>

	<div class="mt-4 rounded-2xl border border-slate-800/70 bg-slate-900/40 p-4">
		<div class="flex items-center justify-between gap-2">
			<h3 class="text-sm font-semibold text-white">New Clip</h3>
			{#if hasPrefilledClip}
				<span class="rounded-full bg-emerald-500/15 px-2 py-0.5 text-[10px] text-emerald-300">
					Prefilled from browser handoff
				</span>
			{/if}
		</div>
		<div class="mt-3 grid gap-2 md:grid-cols-2">
			<input
				class="md:col-span-2 w-full rounded-lg border border-slate-800 bg-slate-950 px-3 py-2 text-xs text-white"
				placeholder="https://example.com/article"
				bind:value={clipUrl}
			/>
			<input
				class="w-full rounded-lg border border-slate-800 bg-slate-950 px-3 py-2 text-xs text-white"
				placeholder="Title (optional)"
				bind:value={clipTitle}
			/>
			<input
				class="w-full rounded-lg border border-slate-800 bg-slate-950 px-3 py-2 text-xs text-white"
				placeholder="tags e.g. research rust ai"
				bind:value={clipTags}
			/>
			<textarea
				class="md:col-span-2 h-28 w-full rounded-lg border border-slate-800 bg-slate-950 px-3 py-2 text-xs text-white"
				placeholder="Paste selected text or a short excerpt..."
				bind:value={clipExcerpt}
			></textarea>
		</div>
		<div class="mt-3 flex flex-wrap items-center gap-2">
			<button
				class="rounded-lg border border-slate-700 px-3 py-1.5 text-xs text-slate-200 hover:bg-slate-800"
				on:click={pasteFromClipboard}
			>
				Paste clipboard
			</button>
			<button
				class="rounded-lg border border-amber-500/30 px-3 py-1.5 text-xs text-amber-300 hover:bg-amber-500/10 disabled:opacity-50"
				on:click={fetchMetadata}
				disabled={enrichingClip}
			>
				{enrichingClip ? 'Fetching...' : 'Fetch metadata'}
			</button>
			<button
				class="rounded-lg border border-violet-500/30 px-3 py-1.5 text-xs text-violet-300 hover:bg-violet-500/10 disabled:opacity-50"
				on:click={suggestTags}
				disabled={aiTagging}
			>
				{aiTagging ? 'Tagging...' : 'AI tags'}
			</button>
			<button
				class="rounded-lg border border-sky-500/30 px-3 py-1.5 text-xs text-sky-300 hover:bg-sky-500/10 disabled:opacity-50"
				on:click={summarizeExcerpt}
				disabled={aiSummarizing}
			>
				{aiSummarizing ? 'Summarizing...' : 'AI summarize'}
			</button>
			<label class="ml-1 inline-flex items-center gap-2 text-[11px] text-slate-400">
				<input type="checkbox" bind:checked={alsoCreateNote} />
				Also create note
			</label>
			<button
				class="ml-auto rounded-lg bg-emerald-500 px-3 py-1.5 text-xs font-semibold text-white hover:bg-emerald-400 disabled:opacity-50"
				on:click={saveClip}
				disabled={savingClip}
			>
				{savingClip ? 'Saving...' : 'Save clip'}
			</button>
		</div>
	</div>

	{#if dedupeConflict}
		<div class="mt-3 rounded-xl border border-amber-500/30 bg-amber-500/10 p-3">
			<h4 class="text-xs font-semibold text-amber-200">Clip Already Exists</h4>
			<p class="mt-1 text-[11px] text-amber-100/90">
				{dedupeConflict.bookmark.title} is already in your vault. You can focus it or create a linked note without duplicating the bookmark.
			</p>
			<div class="mt-2 flex flex-wrap gap-2">
				<button
					class="rounded-lg border border-amber-300/40 px-3 py-1.5 text-[11px] text-amber-100 hover:bg-amber-500/20"
					on:click={focusExistingConflict}
				>
					Show existing clip
				</button>
				<button
					class="rounded-lg border border-emerald-400/40 px-3 py-1.5 text-[11px] text-emerald-200 hover:bg-emerald-500/15 disabled:opacity-50"
					on:click={createNoteForExistingConflict}
					disabled={resolvingConflict}
				>
					{resolvingConflict ? 'Creating note...' : 'Create linked note'}
				</button>
				<button
					class="rounded-lg border border-slate-600 px-3 py-1.5 text-[11px] text-slate-300 hover:bg-slate-800"
					on:click={() => {
						dedupeConflict = null;
					}}
				>
					Dismiss
				</button>
			</div>
		</div>
	{/if}

	<div class="mt-4 flex items-center gap-2">
		<input
			class="flex-1 rounded-lg border border-slate-800 bg-slate-900 px-3 py-1.5 text-xs text-white placeholder-slate-500"
			placeholder="Search bookmarks"
			bind:value={searchQuery}
			aria-label="Search bookmarks"
		/>
		<div class="flex rounded-lg border border-slate-700 bg-slate-800 p-0.5">
			{#each [
				{ key: 'all', label: 'All' },
				{ key: 'unread', label: 'Unread' },
				{ key: 'read', label: 'Read' }
			] as opt (opt.key)}
				<button
					class="rounded-md px-2.5 py-1 text-[10px] font-medium transition {filter === opt.key
						? 'bg-sky-500/20 text-sky-300'
						: 'text-slate-400 hover:text-white'}"
					on:click={() => {
						filter = opt.key as typeof filter;
					}}
				>
					{opt.label}
				</button>
			{/each}
		</div>
	</div>

	<div class="mt-4 flex flex-col gap-2">
		{#if loading}
			<div class="rounded-xl border border-slate-800 p-6 text-center text-xs text-slate-400">Loading bookmarks...</div>
		{:else if displayed.length === 0}
			<EmptyState
				icon="bookmarks"
				tone="amber"
				title={searchQuery || filter !== 'all' ? 'No matching bookmarks' : 'No bookmarks yet'}
				description={searchQuery || filter !== 'all'
					? 'Try clearing filters or search to see more clips.'
					: 'Save your first web clip above or use quick capture in link mode.'}
			/>
		{:else}
			{#each displayed as bookmark (bookmark.id)}
				<div class="rounded-xl border border-slate-800/60 bg-slate-900/40 p-4 transition hover:border-slate-700">
					<div class="flex items-start gap-3">
						<button
							class="mt-0.5 flex h-5 w-5 shrink-0 items-center justify-center rounded border transition {bookmark.read
								? 'border-emerald-500/40 bg-emerald-500/20 text-emerald-300'
								: 'border-slate-600 text-slate-600 hover:border-slate-400'}"
							on:click={() => toggleRead(bookmark.id)}
							title={bookmark.read ? 'Mark unread' : 'Mark read'}
						>
							{#if bookmark.read}
								<svg class="h-3 w-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
									<path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M5 13l4 4L19 7" />
								</svg>
							{/if}
						</button>
						<div class="min-w-0 flex-1">
							<h4 class="text-sm font-medium text-white {bookmark.read ? 'opacity-60' : ''}">{bookmark.title}</h4>
							<p class="mt-0.5 truncate text-[11px] text-slate-500">{getDomain(bookmark.url)}</p>
							{#if bookmark.excerpt}
								<p class="mt-1 line-clamp-2 text-[11px] text-slate-400">{bookmark.excerpt}</p>
							{/if}
							{#each folders.filter(f => f.id === getBookmarkFolderId(bookmark)) as bookmarkFolder (bookmarkFolder.id)}
								<div class="mt-1 flex items-center gap-1">
									<span class="h-1.5 w-1.5 rounded-full bg-{bookmarkFolder.color}-400"></span>
									<span class="text-[9px] text-{bookmarkFolder.color}-300">{bookmarkFolder.name}</span>
								</div>
							{/each}
							{#each [bookmark.tags.filter(t => !t.startsWith(FOLDER_PREFIX))] as visibleTags}
								{#if visibleTags.length > 0}
									<div class="mt-1.5 flex flex-wrap gap-1">
										{#each visibleTags as tag}
											<span class="rounded bg-slate-800 px-1.5 py-0.5 text-[9px] text-slate-400">{tag}</span>
										{/each}
									</div>
								{/if}
							{/each}
						</div>
						<div class="flex items-center gap-2">
							<span class="text-[9px] text-slate-600">{relativeTime(bookmark.created_at)}</span>
							<button
								class="rounded-lg border border-slate-700 px-2 py-1 text-[10px] text-slate-300 hover:bg-slate-800"
								on:click={() => openMoveModal(bookmark.id)}
								title="Move to folder"
							>
								Move
							</button>
							<a
								href={bookmark.url}
								target="_blank"
								rel="noreferrer"
								class="rounded-lg border border-slate-700 px-2 py-1 text-[10px] text-slate-300 hover:bg-slate-800"
							>
								Open
							</a>
							<button
								class="rounded-lg border border-red-500/30 px-2 py-1 text-[10px] text-red-300 hover:bg-red-500/10"
								on:click={() => removeBookmark(bookmark.id)}
							>
								Delete
							</button>
						</div>
					</div>
				</div>
			{/each}
		{/if}
	</div>
	</div>
</div>

<!-- Create Folder Modal -->
{#if showFolderModal}
	<div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm" role="presentation">
		<div class="w-full max-w-sm rounded-xl border border-slate-700 bg-slate-900 p-5 shadow-xl" role="dialog" aria-modal="true">
			<h3 class="text-sm font-semibold text-white">Create Folder</h3>
			<div class="mt-4 space-y-3">
				<div>
					<label class="text-[10px] uppercase tracking-wide text-slate-500" for="folder-name">Name</label>
					<input
						id="folder-name"
						class="mt-1 w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white"
						bind:value={newFolderName}
						placeholder="Reading Queue"
						on:keydown={(e) => e.key === 'Enter' && createFolder()}
					/>
				</div>
				<div>
					<span class="text-[10px] uppercase tracking-wide text-slate-500">Color</span>
					<div class="mt-1 flex flex-wrap gap-1.5">
						{#each FOLDER_COLORS as color}
							<button
								title="Color: {color}"
								class="h-5 w-5 rounded-full bg-{color}-400 ring-2 ring-offset-2 ring-offset-slate-900 transition {newFolderColor === color ? 'ring-white' : 'ring-transparent hover:ring-slate-600'}"
								on:click={() => (newFolderColor = color)}
							></button>
						{/each}
					</div>
				</div>
			</div>
			<div class="mt-5 flex justify-end gap-2">
				<button
					class="rounded-lg border border-slate-700 px-3 py-1.5 text-xs text-slate-300 hover:bg-slate-800"
					on:click={() => { showFolderModal = false; newFolderName = ''; }}
				>
					Cancel
				</button>
				<button
					class="rounded-lg bg-sky-500 px-3 py-1.5 text-xs font-semibold text-white hover:bg-sky-400 disabled:opacity-50"
					on:click={createFolder}
					disabled={!newFolderName.trim()}
				>
					Create
				</button>
			</div>
		</div>
	</div>
{/if}

<!-- Move to Folder Modal -->
{#if showMoveModal && moveTargetBookmarkId}
	{@const targetBookmark = bookmarks.find(b => b.id === moveTargetBookmarkId)}
	{@const currentFolderId = targetBookmark ? getBookmarkFolderId(targetBookmark) : null}
	<div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm" role="presentation">
		<div class="w-full max-w-sm rounded-xl border border-slate-700 bg-slate-900 p-5 shadow-xl" role="dialog" aria-modal="true">
			<h3 class="text-sm font-semibold text-white">Move to Folder</h3>
			<p class="mt-1 text-[11px] text-slate-400 truncate">{targetBookmark?.title}</p>
			<div class="mt-4 space-y-1">
				<button
					class="flex w-full items-center gap-2 rounded-lg px-3 py-2 text-left text-xs transition {currentFolderId === null ? 'bg-slate-700 text-white' : 'text-slate-400 hover:bg-slate-800 hover:text-slate-200'}"
					on:click={() => moveBookmarkToFolder(moveTargetBookmarkId!, null)}
				>
					<svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 8h14M5 8a2 2 0 110-4h14a2 2 0 110 4M5 8v10a2 2 0 002 2h10a2 2 0 002-2V8m-9 4h4" />
					</svg>
					No Folder
				</button>
				{#each folders as folder (folder.id)}
					<button
						class="flex w-full items-center gap-2 rounded-lg px-3 py-2 text-left text-xs transition {currentFolderId === folder.id ? `bg-${folder.color}-500/20 text-${folder.color}-300` : 'text-slate-400 hover:bg-slate-800 hover:text-slate-200'}"
						on:click={() => moveBookmarkToFolder(moveTargetBookmarkId!, folder.id)}
					>
						<span class="h-3 w-3 rounded-full bg-{folder.color}-400"></span>
						{folder.name}
					</button>
				{/each}
				{#if folders.length === 0}
					<p class="px-3 py-2 text-[10px] text-slate-600">No folders. Create one first.</p>
				{/if}
			</div>
			<div class="mt-4 flex justify-end">
				<button
					class="rounded-lg border border-slate-700 px-3 py-1.5 text-xs text-slate-300 hover:bg-slate-800"
					on:click={() => { showMoveModal = false; moveTargetBookmarkId = null; }}
				>
					Cancel
				</button>
			</div>
		</div>
	</div>
{/if}
