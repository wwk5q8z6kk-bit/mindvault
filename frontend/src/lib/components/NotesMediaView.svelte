<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import {
		attachmentDownloadUrl,
		attachmentInlineUrl,
		listAttachmentIndex,
		type AttachmentIndexItem
	} from '$lib/api/files';
	import {
		badgeClass,
		classifyAttachment,
		extractionBadge,
		formatExtractionStatus
	} from '$lib/utils/attachments';
	import { pushToast } from '$lib/stores/toast';

	const PAGE_SIZE = 24;

	let items: AttachmentIndexItem[] = [];
	let loading = true;
	let loadingMore = false;
	let query = '';
	let hasMore = false;
	let total = 0;
	let requestVersion = 0;
	let typeFilter: 'all' | 'image' | 'pdf' | 'audio' | 'video' | 'other' = 'all';

	async function loadMedia(reset = false) {
		const nextVersion = ++requestVersion;
		if (reset) {
			loading = true;
		} else {
			loadingMore = true;
		}
		try {
			const offset = reset ? 0 : items.length;
			const response = await listAttachmentIndex({
				q: query.trim() || undefined,
				limit: PAGE_SIZE,
				offset,
				sort: 'uploaded_desc'
			});

			// Ignore stale responses from older inflight requests.
			if (nextVersion !== requestVersion) return;

			items = reset ? response.items : [...items, ...response.items];
			hasMore = response.has_more;
			total = response.total;
		} catch {
			if (nextVersion !== requestVersion) return;
			if (reset) {
				items = [];
				hasMore = false;
				total = 0;
			}
			pushToast('Failed to load media library', 'danger');
		} finally {
			if (nextVersion !== requestVersion) return;
			loading = false;
			loadingMore = false;
		}
	}

	function matchesType(attachment: AttachmentIndexItem): boolean {
		if (typeFilter === 'all') return true;
		const kind = classifyAttachment(attachment);
		if (typeFilter === 'image') return kind.isImage;
		if (typeFilter === 'pdf') return kind.isPdf;
		if (typeFilter === 'audio') return kind.isAudio;
		if (typeFilter === 'video') return kind.isVideo;
		return !kind.isImage && !kind.isPdf && !kind.isAudio && !kind.isVideo;
	}

	$: filteredItems = items.filter((item) => {
		if (typeFilter !== 'all' && !matchesType(item)) return false;
		return true;
	});

	function openSource(item: AttachmentIndexItem) {
		const path = item.node_kind === 'task' ? `/tasks?task=${item.node_id}` : `/notes?note=${item.node_id}`;
		goto(path);
	}

	function onSearchInput() {
		void loadMedia(true);
	}

	onMount(() => {
		void loadMedia(true);
	});
</script>

<div class="mx-auto max-w-6xl">
	<div class="flex flex-wrap items-center justify-between gap-3">
		<div>
			<h2 class="text-lg font-semibold text-white">Media Library</h2>
			<p class="text-xs text-slate-400">All attachments across notes and tasks.</p>
		</div>
		<div class="flex flex-wrap items-center gap-2">
			<input
				class="w-56 rounded-lg border border-slate-800 bg-slate-900 px-3 py-2 text-xs text-slate-200"
				placeholder="Search attachments"
				bind:value={query}
				on:input={onSearchInput}
			/>
			<select
				class="rounded-lg border border-slate-800 bg-slate-900 px-2 py-2 text-xs text-slate-200"
				bind:value={typeFilter}
			>
				<option value="all">All types</option>
				<option value="image">Images</option>
				<option value="pdf">PDFs</option>
				<option value="audio">Audio</option>
				<option value="video">Video</option>
				<option value="other">Other</option>
			</select>
		</div>
	</div>

	<div class="mt-2 flex items-center justify-between text-[11px] text-slate-500">
		<span>Showing {filteredItems.length} of {total}</span>
		{#if query.trim()}
			<span>Server search: "{query.trim()}"</span>
		{/if}
	</div>

	<div class="mt-4">
		{#if loading}
			<p class="text-xs text-slate-500">Loading media...</p>
		{:else if filteredItems.length === 0}
			<p class="text-xs text-slate-500">No attachments found.</p>
		{:else}
			<div class="grid gap-4 md:grid-cols-2">
				{#each filteredItems as item (item.node_id + item.attachment_id)}
					{@const previewUrl = attachmentInlineUrl(item.node_id, item.attachment_id)}
					{@const kind = classifyAttachment(item)}
					{@const badge = extractionBadge(item.extraction_status)}
					<div class="rounded-2xl border border-slate-900 bg-slate-900/40 p-4">
						<div class="flex items-start justify-between gap-2">
							<div>
								<div class="text-sm font-semibold text-white">{item.file_name}</div>
								<div class="mt-1 text-[10px] text-slate-500">
									<span
										class="inline-flex items-center rounded-full border px-2 py-0.5 text-[10px] {badgeClass(
											badge.tone
										)}"
										title={formatExtractionStatus(item.extraction_status, item.extracted_chars)}
									>
										{badge.label}
									</span>
									<span class="ml-2 text-[10px] text-slate-400">
										{item.node_kind === 'task' ? 'Task' : 'Note'} · {item.node_title}
									</span>
								</div>
							</div>
							<div class="flex items-center gap-2">
								<a
									class="rounded-lg border border-slate-700 px-2 py-1 text-[10px] text-slate-200 hover:bg-slate-800"
									href={attachmentDownloadUrl(item.node_id, item.attachment_id)}
									target="_blank"
									rel="noreferrer"
								>
									Download
								</a>
								<button
									class="rounded-lg border border-slate-700 px-2 py-1 text-[10px] text-slate-200 hover:bg-slate-800"
									on:click={() => openSource(item)}
								>
									Open
								</button>
							</div>
						</div>

						<div class="mt-3">
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
								<audio class="w-full" controls src={previewUrl}></audio>
							{:else if kind.isVideo}
								<video
									class="h-44 w-full rounded-lg border border-slate-800 bg-black"
									controls
									src={previewUrl}
								>
									<track kind="captions" />
								</video>
							{:else}
								<div class="rounded-lg border border-dashed border-slate-700 px-3 py-4 text-xs text-slate-500">
									No preview available.
								</div>
							{/if}
						</div>
					</div>
				{/each}
			</div>

			{#if hasMore}
				<div class="mt-4 flex justify-center">
					<button
						class="rounded-lg border border-slate-700 px-4 py-2 text-xs text-slate-200 hover:bg-slate-800 disabled:opacity-50"
						on:click={() => void loadMedia(false)}
						disabled={loadingMore}
					>
						{loadingMore ? 'Loading...' : 'Load more'}
					</button>
				</div>
			{/if}
		{/if}
	</div>
</div>
