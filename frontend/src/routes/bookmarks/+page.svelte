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
		type Bookmark,
		type CreateBookmarkPayload
	} from '$lib/api/bookmarks';
	import { enrichClip } from '$lib/api/clips';
	import { assistAutoTag, assistTransform } from '$lib/api/assist';
	import { activeNamespace } from '$lib/stores/namespace';
	import { pushToast } from '$lib/stores/toast';

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

	$: allTags = [...new Set(bookmarks.flatMap((item) => item.tags ?? []))].sort();
	$: displayed = bookmarks
		.filter((bookmark) => {
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

<div class="mx-auto max-w-4xl">
	<div class="flex items-start justify-between gap-3">
		<div>
			<h2 class="text-lg font-semibold text-white">Reading List & Clipper</h2>
			<p class="text-xs text-slate-400">Capture web content into MindVault with local-first storage and AI assist.</p>
		</div>
		<button
			class="rounded-lg border border-slate-700 px-3 py-2 text-xs text-slate-200 hover:bg-slate-800 disabled:opacity-50"
			on:click={copyBookmarklet}
			disabled={bookmarkletCopying}
		>
			{bookmarkletCopying ? 'Copying...' : 'Copy bookmarklet'}
		</button>
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
			<div class="rounded-xl border border-dashed border-slate-800 p-8 text-center">
				<h3 class="text-sm font-semibold text-white">No bookmarks</h3>
				<p class="mt-1 text-xs text-slate-400">
					Save your first web clip above or use quick capture in link mode.
				</p>
			</div>
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
							{#if bookmark.tags.length > 0}
								<div class="mt-1.5 flex flex-wrap gap-1">
									{#each bookmark.tags as tag}
										<span class="rounded bg-slate-800 px-1.5 py-0.5 text-[9px] text-slate-400">{tag}</span>
									{/each}
								</div>
							{/if}
						</div>
						<div class="flex items-center gap-2">
							<span class="text-[9px] text-slate-600">{relativeTime(bookmark.created_at)}</span>
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
