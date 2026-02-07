<script lang="ts">
	import { pushToast } from '$lib/stores/toast';
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
	import {
		badgeClass,
		classifyAttachment,
		extractionBadge,
		filterAttachmentChunks,
		formatExtractionStatus
	} from '$lib/utils/attachments';

	export let nodeId: string | null = null;
	export let title = 'Attachments';
	export let description = 'Files linked to this item.';
	export let allowEmbed = false;
	export let onEmbed:
		| ((payload: { attachment: NodeAttachment; inlineUrl: string }) => void)
		| null = null;

	let attachments: NodeAttachment[] = [];
	let attachmentsLoading = false;
	let attachmentUploadPending = false;
	let attachmentUploadProgress = 0;
	let attachmentFileInput: HTMLInputElement | null = null;
	let attachmentMarker: string | null = null;

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

	$: if (nodeId && nodeId !== attachmentMarker) {
		attachmentMarker = nodeId;
		void refreshAttachments(nodeId);
	}
	$: if (!nodeId) {
		attachmentMarker = null;
		attachments = [];
		attachmentChunkState = {};
	}

	async function refreshAttachments(activeNodeId: string) {
		attachmentsLoading = true;
		try {
			attachments = await listNodeAttachments(activeNodeId);
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
		const next = { open: false, loading: false, page: 0, query: '' };
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
		if (!nodeId) return;
		updateChunkState(attachmentId, { loading: true, error: undefined, page });
		try {
			const offset = page * ATTACHMENT_CHUNK_PAGE_SIZE;
			const data = await getAttachmentChunks(nodeId, attachmentId, {
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
		if (!file || !nodeId) return;
		attachmentUploadPending = true;
		attachmentUploadProgress = 0;
		try {
			await uploadNodeAttachment(nodeId, file, (progress) => {
				attachmentUploadProgress = progress.percentage;
			});
			pushToast('Attachment uploaded', 'success');
			await refreshAttachments(nodeId);
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
		if (!nodeId) return;
		try {
			await deleteNodeAttachment(nodeId, attachmentId);
			pushToast('Attachment removed', 'success');
			await refreshAttachments(nodeId);
		} catch {
			pushToast('Failed to remove attachment', 'danger');
		}
	}

	function handleEmbed(item: NodeAttachment) {
		if (!allowEmbed || !nodeId || !onEmbed) return;
		const inlineUrl = attachmentInlineUrl(nodeId, item.attachment_id);
		onEmbed({ attachment: item, inlineUrl });
	}
</script>

<div class="rounded-2xl border border-slate-900 bg-slate-900/40 p-4">
	<div class="flex items-center justify-between gap-2">
		<div>
			<h3 class="text-sm font-semibold text-white">{title}</h3>
			<p class="text-[11px] text-slate-400">{description}</p>
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
				{@const previewUrl = nodeId ? attachmentInlineUrl(nodeId, item.attachment_id) : ''}
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
							{#if allowEmbed && onEmbed}
								<button
									class="rounded-lg border border-slate-700 px-2 py-1 text-[10px] text-slate-200 hover:bg-slate-800"
									on:click={() => handleEmbed(item)}
								>
									Embed
								</button>
							{/if}
							<a
								class="rounded-lg border border-slate-700 px-2 py-1 text-[10px] text-slate-200 hover:bg-slate-800"
								href={nodeId ? attachmentDownloadUrl(nodeId, item.attachment_id) : '#'}
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
