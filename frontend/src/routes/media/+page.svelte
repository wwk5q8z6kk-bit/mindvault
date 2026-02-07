<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { listNotes } from '$lib/api/notes';
	import { listTasks } from '$lib/api/tasks';
	import {
		attachmentDownloadUrl,
		attachmentInlineUrl,
		listNodeAttachments,
		type NodeAttachment
	} from '$lib/api/files';
	import {
		badgeClass,
		classifyAttachment,
		extractionBadge,
		formatExtractionStatus
	} from '$lib/utils/attachments';
	import { pushToast } from '$lib/stores/toast';
	import type { Note } from '$lib/api/notes';
	import type { Task } from '$lib/api/tasks';

	type MediaItem = {
		attachment: NodeAttachment;
		nodeId: string;
		nodeTitle: string;
		nodeKind: 'note' | 'task';
		uploadedAt?: string | null;
	};

	let items: MediaItem[] = [];
	let loading = true;
	let query = '';
	let typeFilter: 'all' | 'image' | 'pdf' | 'audio' | 'video' | 'other' = 'all';

	function hasAttachments(meta: Record<string, unknown>): boolean {
		const raw = meta?.attachments as unknown;
		return Array.isArray(raw) && raw.length > 0;
	}

	async function loadMedia() {
		loading = true;
		try {
			const [notes, tasks] = await Promise.all([listNotes(100), listTasks()]);
			const targets: Array<{ id: string; title: string; kind: 'note' | 'task'; meta: Record<string, unknown> }> = [
				...notes.map((note: Note) => ({
					id: note.id,
					title: note.title ?? 'Untitled note',
					kind: 'note' as const,
					meta: note.metadata ?? {}
				})),
				...tasks.map((task: Task) => ({
					id: task.id,
					title: task.title ?? 'Untitled task',
					kind: 'task' as const,
					meta: task.metadata ?? {}
				}))
			];

			const withAttachments = targets.filter((t) => hasAttachments(t.meta));
			const batches = await Promise.all(
				withAttachments.map(async (target) => {
					try {
						const attachments = await listNodeAttachments(target.id);
						return attachments.map((attachment) => ({
							attachment,
							nodeId: target.id,
							nodeTitle: target.title,
							nodeKind: target.kind,
							uploadedAt: attachment.uploaded_at
						}));
					} catch {
						return [] as MediaItem[];
					}
				})
			);
			items = batches.flat().sort((a, b) => {
				const aTime = a.uploadedAt ? new Date(a.uploadedAt).getTime() : 0;
				const bTime = b.uploadedAt ? new Date(b.uploadedAt).getTime() : 0;
				return bTime - aTime;
			});
		} catch {
			pushToast('Failed to load media library', 'danger');
		} finally {
			loading = false;
		}
	}

	function matchesType(attachment: NodeAttachment): boolean {
		if (typeFilter === 'all') return true;
		const kind = classifyAttachment(attachment);
		if (typeFilter === 'image') return kind.isImage;
		if (typeFilter === 'pdf') return kind.isPdf;
		if (typeFilter === 'audio') return kind.isAudio;
		if (typeFilter === 'video') return kind.isVideo;
		return !kind.isImage && !kind.isPdf && !kind.isAudio && !kind.isVideo;
	}

	$: filteredItems = items.filter((item) => {
		if (typeFilter !== 'all' && !matchesType(item.attachment)) return false;
		if (!query.trim()) return true;
		const q = query.toLowerCase();
		return (
			item.attachment.file_name.toLowerCase().includes(q) ||
			item.nodeTitle.toLowerCase().includes(q)
		);
	});

	function openSource(item: MediaItem) {
		const path = item.nodeKind === 'note' ? `/notes?note=${item.nodeId}` : `/tasks?task=${item.nodeId}`;
		goto(path);
	}

	onMount(() => {
		void loadMedia();
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

	<div class="mt-4">
		{#if loading}
			<p class="text-xs text-slate-500">Loading media…</p>
		{:else if filteredItems.length === 0}
			<p class="text-xs text-slate-500">No attachments found.</p>
		{:else}
			<div class="grid gap-4 md:grid-cols-2">
				{#each filteredItems as item (item.nodeId + item.attachment.attachment_id)}
					{@const previewUrl = attachmentInlineUrl(item.nodeId, item.attachment.attachment_id)}
					{@const kind = classifyAttachment(item.attachment)}
					{@const badge = extractionBadge(item.attachment.extraction_status)}
					<div class="rounded-2xl border border-slate-900 bg-slate-900/40 p-4">
						<div class="flex items-start justify-between gap-2">
							<div>
								<div class="text-sm font-semibold text-white">{item.attachment.file_name}</div>
								<div class="mt-1 text-[10px] text-slate-500">
									<span
										class="inline-flex items-center rounded-full border px-2 py-0.5 text-[10px] {badgeClass(badge.tone)}"
										title={formatExtractionStatus(item.attachment.extraction_status, item.attachment.extracted_chars)}
									>
										{badge.label}
									</span>
									<span class="ml-2 text-[10px] text-slate-400">
										{item.nodeKind === 'note' ? 'Note' : 'Task'} · {item.nodeTitle}
									</span>
								</div>
							</div>
							<div class="flex items-center gap-2">
								<a
									class="rounded-lg border border-slate-700 px-2 py-1 text-[10px] text-slate-200 hover:bg-slate-800"
									href={attachmentDownloadUrl(item.nodeId, item.attachment.attachment_id)}
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
									alt={item.attachment.file_name}
									loading="lazy"
								/>
							{:else if kind.isPdf}
								<iframe
									class="h-44 w-full rounded-lg border border-slate-800 bg-slate-950"
									src={previewUrl}
									title={`Preview ${item.attachment.file_name}`}
									loading="lazy"
								></iframe>
							{:else if kind.isAudio}
								<audio class="w-full" controls src={previewUrl}></audio>
							{:else if kind.isVideo}
								<video class="h-44 w-full rounded-lg border border-slate-800 bg-black" controls src={previewUrl}>
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
		{/if}
	</div>
</div>
