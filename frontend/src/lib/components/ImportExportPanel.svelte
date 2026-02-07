<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import {
		startExport,
		downloadExport,
		importData,
		pollJobCompletion,
		getExportStatus,
		getImportStatus
	} from '$lib/api/portability';
	import type { ExportOptions, ImportOptions, ImportProgress } from '$lib/api/portability';
	import { pushToast } from '$lib/stores/toast';

	export let open = false;

	const dispatch = createEventDispatcher<{
		close: void;
		exportComplete: { exportId: string };
		importComplete: { importId: string; nodesImported: number };
	}>();

	let activeTab: 'export' | 'import' = 'export';

	// Export state
	let exportFormat: 'json' | 'markdown' | 'archive' = 'archive';
	let includeRelationships = true;
	let includeAttachments = true;
	let includeVersions = false;
	let exporting = false;
	let exportProgress = '';

	// Import state
	let importFile: File | null = null;
	let conflictStrategy: 'skip' | 'replace' | 'merge' | 'duplicate' = 'skip';
	let importRelationships = true;
	let importAttachments = true;
	let addTags: string[] = [];
	let tagInput = '';
	let importing = false;
	let importProgress: ImportProgress | null = null;

	async function handleExport() {
		exporting = true;
		exportProgress = 'Starting export...';

		try {
			const options: ExportOptions = {
				format: exportFormat,
				include_relationships: includeRelationships,
				include_attachments: includeAttachments && exportFormat === 'archive',
				include_versions: includeVersions
			};

			const exportJob = await startExport(options);
			exportProgress = 'Processing...';

			// Poll for completion
			const completed = await pollJobCompletion(
				() => getExportStatus(exportJob.export_id),
				1000,
				300000
			);

			if (completed.status === 'completed') {
				exportProgress = 'Downloading...';
				await downloadExport(exportJob.export_id);
				pushToast('Export completed successfully', 'success');
				dispatch('exportComplete', { exportId: exportJob.export_id });
			} else {
				throw new Error(completed.error || 'Export failed');
			}
		} catch (err) {
			pushToast(`Export failed: ${err instanceof Error ? err.message : 'Unknown error'}`, 'danger');
		} finally {
			exporting = false;
			exportProgress = '';
		}
	}

	async function handleImport() {
		if (!importFile) {
			pushToast('Please select a file to import', 'warning');
			return;
		}

		importing = true;

		try {
			const options: ImportOptions = {
				conflict_strategy: conflictStrategy,
				import_relationships: importRelationships,
				import_attachments: importAttachments,
				add_tags: addTags.length > 0 ? addTags : undefined
			};

			const importJob = await importData(importFile, options, (progress) => {
				importProgress = progress;
			});

			// Poll for completion if needed
			let result = importJob;
			if (result.status === 'processing' || result.status === 'pending') {
				result = await pollJobCompletion(
					() => getImportStatus(importJob.import_id),
					1000,
					300000
				);
			}

			if (result.status === 'completed') {
				pushToast(`Imported ${result.nodes_imported} nodes successfully`, 'success');
				dispatch('importComplete', {
					importId: result.import_id,
					nodesImported: result.nodes_imported
				});
				// Reset form
				importFile = null;
				addTags = [];
				tagInput = '';
			} else {
				const errorMsg = result.errors.length > 0
					? result.errors[0].error
					: 'Import failed';
				throw new Error(errorMsg);
			}
		} catch (err) {
			pushToast(`Import failed: ${err instanceof Error ? err.message : 'Unknown error'}`, 'danger');
		} finally {
			importing = false;
			importProgress = null;
		}
	}

	function handleFileSelect(event: Event) {
		const input = event.target as HTMLInputElement;
		if (input.files?.length) {
			importFile = input.files[0];
		}
	}

	function handleFileDrop(event: DragEvent) {
		event.preventDefault();
		const files = event.dataTransfer?.files;
		if (files?.length) {
			importFile = files[0];
		}
	}

	function addTag() {
		const tag = tagInput.trim();
		if (tag && !addTags.includes(tag)) {
			addTags = [...addTags, tag];
			tagInput = '';
		}
	}

	function removeTag(tag: string) {
		addTags = addTags.filter((t) => t !== tag);
	}

	function close() {
		open = false;
		dispatch('close');
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') close();
	}
</script>

<svelte:window on:keydown={handleKeydown} />

{#if open}
	<div class="modal-overlay" on:click={close} on:keydown={handleKeydown} role="button" tabindex="0">
		<!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
		<div class="modal-content" on:click|stopPropagation on:keydown|stopPropagation role="dialog" aria-modal="true" tabindex="-1">
			<header class="modal-header">
				<h2>Data Portability</h2>
				<button class="close-btn" on:click={close} aria-label="Close">
					<svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="2">
						<path d="M18 6L6 18M6 6l12 12"/>
					</svg>
				</button>
			</header>

			<div class="tabs">
				<button class:active={activeTab === 'export'} on:click={() => (activeTab = 'export')}>
					Export
				</button>
				<button class:active={activeTab === 'import'} on:click={() => (activeTab = 'import')}>
					Import
				</button>
			</div>

			<div class="modal-body">
				{#if activeTab === 'export'}
					<div class="export-section">
						<div class="form-group">
							<span class="form-label">Export Format</span>
							<div class="radio-group">
								<label class="radio-option">
									<input type="radio" bind:group={exportFormat} value="archive" />
									<span class="radio-label">
										<strong>Archive (ZIP)</strong>
										<small>Full backup with all content and attachments</small>
									</span>
								</label>
								<label class="radio-option">
									<input type="radio" bind:group={exportFormat} value="json" />
									<span class="radio-label">
										<strong>JSON</strong>
										<small>Structured data for programmatic access</small>
									</span>
								</label>
								<label class="radio-option">
									<input type="radio" bind:group={exportFormat} value="markdown" />
									<span class="radio-label">
										<strong>Markdown</strong>
										<small>Human-readable text files</small>
									</span>
								</label>
							</div>
						</div>

						<div class="form-group">
							<span class="form-label">Include</span>
							<div class="checkbox-group">
								<label class="checkbox-option">
									<input type="checkbox" bind:checked={includeRelationships} />
									<span>Relationships between notes</span>
								</label>
								{#if exportFormat === 'archive'}
									<label class="checkbox-option">
										<input type="checkbox" bind:checked={includeAttachments} />
										<span>File attachments</span>
									</label>
								{/if}
								<label class="checkbox-option">
									<input type="checkbox" bind:checked={includeVersions} />
									<span>Version history</span>
								</label>
							</div>
						</div>

						{#if exportProgress}
							<div class="progress-indicator">
								<div class="spinner"></div>
								<span>{exportProgress}</span>
							</div>
						{/if}

						<button class="btn-primary" on:click={handleExport} disabled={exporting}>
							{exporting ? 'Exporting...' : 'Export Data'}
						</button>
					</div>
				{:else}
					<div class="import-section">
						<div
							class="drop-zone"
							class:has-file={importFile}
							on:dragover|preventDefault
							on:drop={handleFileDrop}
							role="button"
							tabindex="0"
						>
							{#if importFile}
								<div class="file-info">
									<svg viewBox="0 0 24 24" width="24" height="24" fill="none" stroke="currentColor" stroke-width="2">
										<path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z"/>
										<path d="M14 2v6h6M12 18v-6M9 15l3 3 3-3"/>
									</svg>
									<span class="file-name">{importFile.name}</span>
									<button class="btn-remove" on:click={() => (importFile = null)} aria-label="Remove file">
										<svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2">
											<path d="M18 6L6 18M6 6l12 12"/>
										</svg>
									</button>
								</div>
							{:else}
								<svg viewBox="0 0 24 24" width="32" height="32" fill="none" stroke="currentColor" stroke-width="2">
									<path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4M17 8l-5-5-5 5M12 3v12"/>
								</svg>
								<p>Drop a file here or <label class="browse-link">browse<input type="file" accept=".zip,.json,.md" on:change={handleFileSelect} /></label></p>
								<small>Supports ZIP, JSON, or Markdown files</small>
							{/if}
						</div>

						<div class="form-group">
							<label for="import-conflict">Conflict Handling</label>
							<select id="import-conflict" bind:value={conflictStrategy}>
								<option value="skip">Skip duplicates</option>
								<option value="replace">Replace existing</option>
								<option value="merge">Merge content</option>
								<option value="duplicate">Create duplicates</option>
							</select>
						</div>

						<div class="form-group">
							<span class="form-label">Options</span>
							<div class="checkbox-group">
								<label class="checkbox-option">
									<input type="checkbox" bind:checked={importRelationships} />
									<span>Import relationships</span>
								</label>
								<label class="checkbox-option">
									<input type="checkbox" bind:checked={importAttachments} />
									<span>Import attachments</span>
								</label>
							</div>
						</div>

						<div class="form-group">
							<label for="import-tags">Add Tags to Imported Notes</label>
							<div class="tag-input-container">
								<input
									id="import-tags"
									type="text"
									bind:value={tagInput}
									placeholder="Enter tag and press Enter"
									on:keydown={(e) => e.key === 'Enter' && (e.preventDefault(), addTag())}
								/>
								<button class="btn-add-tag" on:click={addTag}>Add</button>
							</div>
							{#if addTags.length > 0}
								<div class="tags-list">
									{#each addTags as tag}
										<span class="tag">
											{tag}
											<button on:click={() => removeTag(tag)}>&times;</button>
										</span>
									{/each}
								</div>
							{/if}
						</div>

						{#if importProgress}
							<div class="progress-indicator">
								<div class="spinner"></div>
								<span>
									{importProgress.phase === 'uploading' ? 'Uploading' : 'Processing'}...
									{#if importProgress.percentage !== undefined}
										{importProgress.percentage}%
									{/if}
								</span>
							</div>
						{/if}

						<button class="btn-primary" on:click={handleImport} disabled={importing || !importFile}>
							{importing ? 'Importing...' : 'Import Data'}
						</button>
					</div>
				{/if}
			</div>
		</div>
	</div>
{/if}

<style>
	.modal-overlay {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.5);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 1000;
	}

	.modal-content {
		--surface-color: rgb(15 23 42);
		--border-color: rgb(51 65 85);
		--text-color: rgb(226 232 240);
		--text-muted: rgb(148 163 184);
		--primary-color: rgb(14 165 233);
		--primary-light: rgb(14 165 233 / 0.15);
		--primary-lighter: rgb(14 165 233 / 0.25);
		--primary-bg: rgb(14 165 233 / 0.05);
		--bg-muted: rgb(30 41 59);
		background: var(--surface-color);
		color: var(--text-color);
		border: 1px solid var(--border-color);
		border-radius: 12px;
		width: 90vw;
		max-width: 500px;
		max-height: 80vh;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.modal-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 1rem 1.5rem;
		border-bottom: 1px solid var(--border-color, #e0e0e0);
	}

	.modal-header h2 {
		margin: 0;
		font-size: 1.25rem;
	}

	.close-btn {
		background: none;
		border: none;
		cursor: pointer;
		padding: 0.25rem;
		color: var(--text-muted, #6b7280);
	}

	.tabs {
		display: flex;
		border-bottom: 1px solid var(--border-color, #e0e0e0);
	}

	.tabs button {
		flex: 1;
		padding: 0.75rem;
		border: none;
		background: transparent;
		font-size: 0.875rem;
		font-weight: 500;
		cursor: pointer;
		color: var(--text-muted, #6b7280);
		border-bottom: 2px solid transparent;
		margin-bottom: -1px;
	}

	.tabs button.active {
		color: var(--primary-color, #3b82f6);
		border-bottom-color: var(--primary-color, #3b82f6);
	}

	.modal-body {
		flex: 1;
		overflow-y: auto;
		padding: 1.5rem;
	}

	.form-group {
		margin-bottom: 1.25rem;
	}

	.form-group > label,
	.form-group > .form-label {
		display: block;
		font-size: 0.875rem;
		font-weight: 500;
		margin-bottom: 0.5rem;
	}

	.radio-group {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.radio-option {
		display: flex;
		align-items: flex-start;
		gap: 0.5rem;
		padding: 0.75rem;
		border: 1px solid var(--border-color, #e0e0e0);
		border-radius: 8px;
		cursor: pointer;
	}

	.radio-option:has(input:checked) {
		border-color: var(--primary-color, #3b82f6);
		background: var(--primary-bg, #eff6ff);
	}

	.radio-label {
		display: flex;
		flex-direction: column;
	}

	.radio-label strong {
		font-size: 0.875rem;
	}

	.radio-label small {
		color: var(--text-muted, #6b7280);
		font-size: 0.75rem;
	}

	.checkbox-group {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.checkbox-option {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 0.875rem;
		cursor: pointer;
	}

	.drop-zone {
		border: 2px dashed var(--border-color, #e0e0e0);
		border-radius: 8px;
		padding: 2rem;
		text-align: center;
		margin-bottom: 1.25rem;
		transition: border-color 0.15s;
	}

	.drop-zone:hover, .drop-zone.has-file {
		border-color: var(--primary-color, #3b82f6);
	}

	.drop-zone p {
		margin: 0.5rem 0;
		color: var(--text-color, #374151);
	}

	.drop-zone small {
		color: var(--text-muted, #6b7280);
	}

	.browse-link {
		color: var(--primary-color, #3b82f6);
		cursor: pointer;
	}

	.browse-link input {
		display: none;
	}

	.file-info {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.file-name {
		flex: 1;
		font-weight: 500;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.btn-remove {
		background: none;
		border: none;
		padding: 0.25rem;
		cursor: pointer;
		color: var(--text-muted, #6b7280);
	}

	select {
		width: 100%;
		padding: 0.5rem;
		border: 1px solid var(--border-color, #e0e0e0);
		border-radius: 6px;
		font-size: 0.875rem;
		background: var(--bg-muted, #f9fafb);
		color: var(--text-color, #374151);
	}

	.tag-input-container {
		display: flex;
		gap: 0.5rem;
	}

	.tag-input-container input {
		flex: 1;
		padding: 0.5rem;
		border: 1px solid var(--border-color, #e0e0e0);
		border-radius: 6px;
		font-size: 0.875rem;
		background: var(--bg-muted, #f9fafb);
		color: var(--text-color, #374151);
	}

	.btn-add-tag {
		padding: 0.5rem 0.75rem;
		border: 1px solid var(--border-color, #e0e0e0);
		background: var(--surface-color, #fff);
		border-radius: 6px;
		font-size: 0.875rem;
		cursor: pointer;
	}

	.tags-list {
		display: flex;
		flex-wrap: wrap;
		gap: 0.25rem;
		margin-top: 0.5rem;
	}

	.tag {
		display: inline-flex;
		align-items: center;
		gap: 0.25rem;
		padding: 0.25rem 0.5rem;
		background: var(--primary-light, #dbeafe);
		border-radius: 4px;
		font-size: 0.75rem;
	}

	.tag button {
		background: none;
		border: none;
		padding: 0;
		cursor: pointer;
		font-size: 1rem;
		line-height: 1;
	}

	.progress-indicator {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.75rem;
		background: var(--bg-muted, #f9fafb);
		border-radius: 6px;
		margin-bottom: 1rem;
	}

	.spinner {
		width: 16px;
		height: 16px;
		border: 2px solid var(--border-color, #e0e0e0);
		border-top-color: var(--primary-color, #3b82f6);
		border-radius: 50%;
		animation: spin 0.8s linear infinite;
	}

	@keyframes spin {
		to { transform: rotate(360deg); }
	}

	.btn-primary {
		width: 100%;
		padding: 0.75rem;
		background: var(--primary-color, #3b82f6);
		color: white;
		border: none;
		border-radius: 8px;
		font-size: 0.875rem;
		font-weight: 500;
		cursor: pointer;
	}

	.btn-primary:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
</style>
