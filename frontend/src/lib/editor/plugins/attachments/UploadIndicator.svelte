<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { onAttachmentStateChange, getUploadState } from './index';
	import { formatFileSize } from '$lib/api/files';

	interface Upload {
		file: File;
		progress: { loaded: number; total: number; percentage: number };
	}

	let uploads: Map<string, Upload> = new Map();
	let isUploading = false;

	let unsubscribe: (() => void) | null = null;

	onMount(() => {
		// Get initial state
		const state = getUploadState();
		isUploading = state.isUploading;
		uploads = state.uploads;

		// Subscribe to state changes
		unsubscribe = onAttachmentStateChange((state) => {
			isUploading = state.isUploading;
			uploads = state.uploads;
		});
	});

	onDestroy(() => {
		unsubscribe?.();
	});

	function getUploadList(): Array<{ id: string; file: File; progress: Upload['progress'] }> {
		return Array.from(uploads.entries()).map(([id, upload]) => ({
			id,
			file: upload.file,
			progress: upload.progress
		}));
	}

	$: uploadList = getUploadList();
</script>

{#if isUploading && uploadList.length > 0}
	<div class="fixed bottom-4 right-4 z-40 w-72 rounded-lg border border-slate-700 bg-slate-900 p-3 shadow-xl">
		<div class="mb-2 text-xs font-medium text-slate-400">
			Uploading {uploadList.length} file{uploadList.length > 1 ? 's' : ''}...
		</div>
		<div class="flex flex-col gap-2">
			{#each uploadList as upload (upload.id)}
				<div class="flex flex-col gap-1">
					<div class="flex items-center justify-between text-xs">
						<span class="truncate text-slate-300" title={upload.file.name}>
							{upload.file.name}
						</span>
						<span class="ml-2 shrink-0 text-slate-500">
							{upload.progress.percentage}%
						</span>
					</div>
					<div class="h-1.5 overflow-hidden rounded-full bg-slate-800">
						<div
							class="h-full bg-sky-500 transition-all duration-200"
							style="width: {upload.progress.percentage}%"
						></div>
					</div>
					<div class="text-[10px] text-slate-500">
						{formatFileSize(upload.progress.loaded)} / {formatFileSize(upload.progress.total)}
					</div>
				</div>
			{/each}
		</div>
	</div>
{/if}
