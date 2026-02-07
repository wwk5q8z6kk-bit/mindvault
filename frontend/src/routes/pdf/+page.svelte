<script lang="ts">
	import { pushToast } from '$lib/stores/toast';
	import { createNoteOptimistic } from '$lib/stores/notes';

	let pdfUrl = '';
	let fileInput: HTMLInputElement | null = null;
	let pdfObjectUrl: string | null = null;
	let pdfFileName = '';
	let annotations: Array<{
		id: string;
		page: number;
		text: string;
		color: string;
		created_at: string;
	}> = [];
	let newAnnotation = '';
	let annotationPage = 1;
	let annotationColor = '#fbbf24';
	let showAnnotations = true;

	const COLORS = [
		{ value: '#fbbf24', label: 'Yellow' },
		{ value: '#38bdf8', label: 'Blue' },
		{ value: '#34d399', label: 'Green' },
		{ value: '#f87171', label: 'Red' },
		{ value: '#c084fc', label: 'Purple' }
	];

	function handleFileSelect(event: Event) {
		const input = event.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		if (!file || file.type !== 'application/pdf') {
			pushToast('Please select a PDF file', 'warning');
			return;
		}
		if (pdfObjectUrl) URL.revokeObjectURL(pdfObjectUrl);
		pdfObjectUrl = URL.createObjectURL(file);
		pdfFileName = file.name;
		loadAnnotations(file.name);
	}

	function handleUrlLoad() {
		if (!pdfUrl.trim()) return;
		if (pdfObjectUrl) URL.revokeObjectURL(pdfObjectUrl);
		pdfObjectUrl = pdfUrl.trim();
		pdfFileName = new URL(pdfUrl).pathname.split('/').pop() ?? 'document.pdf';
		loadAnnotations(pdfFileName);
	}

	function loadAnnotations(fileName: string) {
		const saved = localStorage.getItem(`mv_pdf_annotations_${fileName}`);
		if (saved) {
			try { annotations = JSON.parse(saved); } catch { annotations = []; }
		} else {
			annotations = [];
		}
	}

	function saveAnnotations() {
		if (pdfFileName) {
			localStorage.setItem(`mv_pdf_annotations_${pdfFileName}`, JSON.stringify(annotations));
		}
	}

	function addAnnotation() {
		const text = newAnnotation.trim();
		if (!text) return;
		annotations = [...annotations, {
			id: Date.now().toString(36) + Math.random().toString(36).slice(2, 6),
			page: annotationPage,
			text,
			color: annotationColor,
			created_at: new Date().toISOString()
		}];
		newAnnotation = '';
		saveAnnotations();
	}

	function removeAnnotation(id: string) {
		annotations = annotations.filter((a) => a.id !== id);
		saveAnnotations();
	}

	async function exportAnnotationsAsNote() {
		if (annotations.length === 0) {
			pushToast('No annotations to export', 'warning');
			return;
		}
		const title = `Annotations: ${pdfFileName}`;
		const lines = annotations.map((a) =>
			`- **Page ${a.page}**: ${a.text}`
		);
		const markdown = `# ${title}\n\nSource: ${pdfFileName}\nExported: ${new Date().toLocaleString()}\n\n${lines.join('\n')}`;
		await createNoteOptimistic(markdown, title);
		pushToast('Annotations exported as note', 'success');
	}

	$: sortedAnnotations = [...annotations].sort((a, b) => a.page - b.page || a.created_at.localeCompare(b.created_at));
</script>

<div class="mx-auto max-w-6xl">
	<div class="flex items-center justify-between">
		<div>
			<h2 class="text-lg font-semibold text-white">PDF Viewer</h2>
			<p class="text-xs text-slate-400">
				{pdfFileName ? pdfFileName : 'Open a PDF to view and annotate'}
			</p>
		</div>
		<div class="flex items-center gap-2">
			<label class="rounded-lg border border-slate-700 bg-slate-800 px-3 py-1.5 text-xs text-slate-200 transition hover:bg-slate-700 cursor-pointer">
				Open file
				<input
					type="file"
					accept=".pdf"
					class="hidden"
					bind:this={fileInput}
					on:change={handleFileSelect}
				/>
			</label>
			<label class="flex items-center gap-1.5 text-[10px] text-slate-400">
				<input type="checkbox" bind:checked={showAnnotations} class="rounded border-slate-600" />
				Annotations
			</label>
		</div>
	</div>

	{#if !pdfObjectUrl}
		<div class="mt-6 rounded-2xl border border-dashed border-slate-800 p-12 text-center">
			<div class="flex justify-center">
				<div class="flex h-14 w-14 items-center justify-center rounded-2xl bg-red-500/10 text-red-300">
					<svg class="h-7 w-7" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
					</svg>
				</div>
			</div>
			<h3 class="mt-3 text-sm font-semibold text-white">No PDF loaded</h3>
			<p class="mt-1 text-xs text-slate-400">Select a local PDF file or paste a URL to get started.</p>
			<div class="mt-4 flex items-center justify-center gap-2">
				<input
					class="w-64 rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white placeholder-slate-500"
					placeholder="https://example.com/document.pdf"
					bind:value={pdfUrl}
					on:keydown={(e) => e.key === 'Enter' && handleUrlLoad()}
				/>
				<button
					class="rounded-lg bg-sky-500 px-3 py-2 text-xs font-semibold text-white hover:bg-sky-400 disabled:opacity-50"
					disabled={!pdfUrl.trim()}
					on:click={handleUrlLoad}
				>
					Load URL
				</button>
			</div>
		</div>
	{:else}
		<div class="mt-4 flex gap-4" style="height: calc(100vh - 200px);">
			<!-- PDF embed -->
			<div class="flex-1 overflow-hidden rounded-xl border border-slate-800/60 bg-slate-950">
				<object
					data={pdfObjectUrl}
					type="application/pdf"
					class="h-full w-full"
					title={pdfFileName}
				>
					<p class="p-8 text-center text-sm text-slate-400">
						PDF cannot be displayed. <a href={pdfObjectUrl} target="_blank" class="text-sky-400 underline">Download instead</a>
					</p>
				</object>
			</div>

			<!-- Annotations sidebar -->
			{#if showAnnotations}
				<div class="flex w-72 flex-col rounded-xl border border-slate-800/60 bg-slate-900/40">
					<div class="flex items-center justify-between border-b border-slate-800 px-4 py-3">
						<h3 class="text-xs font-semibold text-white">Annotations ({annotations.length})</h3>
						{#if annotations.length > 0}
							<button
								class="text-[10px] text-sky-400 hover:text-sky-300"
								on:click={exportAnnotationsAsNote}
							>
								Export as note
							</button>
						{/if}
					</div>

					<!-- Add annotation -->
					<div class="border-b border-slate-800 p-3">
						<div class="flex gap-2">
							<div>
								<label class="text-[9px] text-slate-500" for="ann-page">Page</label>
								<input
									id="ann-page"
									type="number"
									min="1"
									class="mt-0.5 w-14 rounded border border-slate-700 bg-slate-800 px-2 py-1 text-[10px] text-white"
									bind:value={annotationPage}
								/>
							</div>
							<div class="flex gap-1 self-end">
								{#each COLORS as c (c.value)}
									<button
										class="h-5 w-5 rounded-full border-2 {annotationColor === c.value ? 'border-white' : 'border-transparent'}"
										style="background-color: {c.value}"
										title={c.label}
										on:click={() => { annotationColor = c.value; }}
									></button>
								{/each}
							</div>
						</div>
						<div class="mt-2 flex gap-1.5">
							<input
								class="flex-1 rounded-lg border border-slate-700 bg-slate-800 px-2.5 py-1.5 text-[10px] text-white placeholder-slate-500"
								placeholder="Add annotation..."
								bind:value={newAnnotation}
								on:keydown={(e) => e.key === 'Enter' && addAnnotation()}
							/>
							<button
								class="rounded-lg bg-sky-500 px-2.5 py-1.5 text-[10px] font-semibold text-white hover:bg-sky-400 disabled:opacity-50"
								disabled={!newAnnotation.trim()}
								on:click={addAnnotation}
							>
								Add
							</button>
						</div>
					</div>

					<!-- Annotation list -->
					<div class="flex-1 overflow-y-auto p-2">
						{#if sortedAnnotations.length === 0}
							<p class="p-4 text-center text-[10px] text-slate-500">No annotations yet</p>
						{:else}
							{#each sortedAnnotations as ann (ann.id)}
								<div class="mb-1.5 rounded-lg border border-slate-800/60 p-2.5" style="border-left: 3px solid {ann.color}">
									<div class="flex items-start justify-between gap-1">
										<div>
											<span class="text-[9px] font-medium text-slate-500">p. {ann.page}</span>
											<p class="mt-0.5 text-[11px] text-white">{ann.text}</p>
										</div>
										<button
											class="text-[9px] text-red-400 hover:text-red-300 shrink-0"
											on:click={() => removeAnnotation(ann.id)}
										>
											x
										</button>
									</div>
								</div>
							{/each}
						{/if}
					</div>
				</div>
			{/if}
		</div>
	{/if}
</div>
