<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { pushToast } from '$lib/stores/toast';
	import {
		exportVaultBundle,
		importVaultBundle,
		type VaultTransferBundle,
		type ImportBundleResponse
	} from '$lib/api/vault-transfer';
	import { getNode } from '$lib/api/nodes';
	import type { KnowledgeNode } from '$lib/api/types';
	import { downloadIcal, importIcal, type IcalImportResponse } from '$lib/api/calendar';

	export let open = false;

	const dispatch = createEventDispatcher<{
		close: void;
		exportComplete: { exportId: string };
		importComplete: { importId: string; nodesImported: number };
	}>();

	type ConflictPreview = {
		id: string;
		existing: KnowledgeNode;
		incoming: KnowledgeNode;
	};

	let activeTab: 'export' | 'import' | 'ical' = 'export';

	// Export state
	let exportNamespace = '';
	let includeRelationships = true;
	let exporting = false;

	// Import state
	let importFile: File | null = null;
	let importBundle: VaultTransferBundle | null = null;
	let importNamespaceOverride = '';
	let overwriteExisting = false;
	let importIncludeRelationships = true;
	let importing = false;
	let importResult: ImportBundleResponse | null = null;
	let conflictPreviewLoading = false;
	let conflictPreview: ConflictPreview[] = [];

	// iCal state
	let exportingIcal = false;
	let importingIcal = false;
	let icalImportFile: File | null = null;
	let icalDuplicateStrategy: 'skip' | 'update' | 'duplicate' = 'skip';
	let icalResult: IcalImportResponse | null = null;

	function close() {
		open = false;
		dispatch('close');
	}

	function downloadJson(data: unknown, filename: string) {
		const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' });
		const url = URL.createObjectURL(blob);
		const a = document.createElement('a');
		a.href = url;
		a.download = filename;
		a.click();
		URL.revokeObjectURL(url);
	}

	async function handleExportBundle() {
		exporting = true;
		try {
			const bundle = await exportVaultBundle({
				namespace: exportNamespace.trim() || undefined,
				includeRelationships
			});
			const date = new Date().toISOString().slice(0, 10);
			downloadJson(bundle, `mindvault-export-${date}.json`);
			pushToast('Export bundle downloaded', 'success');
			dispatch('exportComplete', { exportId: bundle.exported_at });
		} catch {
			pushToast('Export failed', 'danger');
		} finally {
			exporting = false;
		}
	}

	async function analyzePotentialConflicts(bundle: VaultTransferBundle) {
		conflictPreviewLoading = true;
		conflictPreview = [];
		try {
			const candidates = bundle.nodes.slice(0, 12);
			const checks = await Promise.all(
				candidates.map(async (incoming) => {
					try {
						const existing = await getNode(incoming.id);
						return { id: incoming.id, existing, incoming } as ConflictPreview;
					} catch {
						return null;
					}
				})
			);
			conflictPreview = checks.filter((item): item is ConflictPreview => item !== null);
		} finally {
			conflictPreviewLoading = false;
		}
	}

	async function handleImportFile(event: Event) {
		const target = event.currentTarget as HTMLInputElement;
		const file = target.files?.[0];
		if (!file) return;
		try {
			const raw = await file.text();
			const parsed = JSON.parse(raw) as VaultTransferBundle;
			if (!Array.isArray(parsed.nodes)) {
				throw new Error('Invalid bundle format: missing nodes');
			}
			if (!Array.isArray(parsed.relationships)) {
				parsed.relationships = [];
			}
			importFile = file;
			importBundle = parsed;
			importResult = null;
			await analyzePotentialConflicts(parsed);
			pushToast('Bundle loaded for import', 'success');
		} catch {
			importFile = null;
			importBundle = null;
			conflictPreview = [];
			pushToast('Unable to parse import bundle', 'danger');
		}
	}

	async function handleImportBundle() {
		if (!importBundle) {
			pushToast('Select a JSON bundle first', 'warning');
			return;
		}
		importing = true;
		try {
			const result = await importVaultBundle({
				nodes: importBundle.nodes,
				relationships: importBundle.relationships,
				overwrite_existing: overwriteExisting,
				include_relationships: importIncludeRelationships,
				namespace_override: importNamespaceOverride.trim() || undefined
			});
			importResult = result;
			pushToast(
				`Imported ${result.imported_nodes} nodes (${result.updated_nodes} updated, ${result.skipped_nodes} skipped)`,
				'success'
			);
			dispatch('importComplete', {
				importId: new Date().toISOString(),
				nodesImported: result.imported_nodes + result.updated_nodes
			});
		} catch {
			pushToast('Import failed', 'danger');
		} finally {
			importing = false;
		}
	}

	async function handleExportIcal() {
		exportingIcal = true;
		try {
			await downloadIcal({ filename: `mindvault-calendar-${new Date().toISOString().slice(0, 10)}.ics` });
			pushToast('iCalendar export downloaded', 'success');
		} catch {
			pushToast('iCalendar export failed', 'danger');
		} finally {
			exportingIcal = false;
		}
	}

	async function handleIcalImport(event: Event) {
		const target = event.currentTarget as HTMLInputElement;
		const file = target.files?.[0];
		if (!file) return;
		icalImportFile = file;
		importingIcal = true;
		try {
			icalResult = await importIcal(file, {
				duplicate_strategy: icalDuplicateStrategy
			});
			pushToast(`Imported ${icalResult.imported_count} calendar items`, 'success');
		} catch {
			pushToast('iCalendar import failed', 'danger');
		} finally {
			importingIcal = false;
		}
	}
</script>

{#if open}
	<div class="fixed inset-0 z-50 flex items-center justify-center p-4" role="presentation">
		<div
			class="absolute inset-0 bg-black/60"
			role="button"
			tabindex="0"
			aria-label="Close"
			on:click={close}
			on:keydown={(e) => e.key === 'Escape' && close()}
		></div>
		<div class="relative z-10 w-full max-w-4xl rounded-2xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))] p-5 shadow-2xl">
			<div class="flex items-start justify-between gap-3">
				<div>
					<h2 class="text-lg font-semibold text-[rgb(var(--mv-text))]">Import / Export Hub</h2>
					<p class="text-xs text-[rgb(var(--mv-muted))]">Bundle export/import, iCal portability, and conflict previews.</p>
				</div>
				<button
					class="rounded-lg border border-[rgb(var(--mv-border))] px-2 py-1 text-xs text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-panel-strong))]"
					on:click={close}
				>
					Close
				</button>
			</div>

			<div class="mt-4 flex gap-2 border-b border-[rgb(var(--mv-border))] pb-2">
				<button
					class="rounded-lg px-3 py-1.5 text-xs {activeTab === 'export' ? 'bg-sky-500/20 text-sky-200' : 'text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-panel-strong))]'}"
					on:click={() => (activeTab = 'export')}
				>
					Export
				</button>
				<button
					class="rounded-lg px-3 py-1.5 text-xs {activeTab === 'import' ? 'bg-sky-500/20 text-sky-200' : 'text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-panel-strong))]'}"
					on:click={() => (activeTab = 'import')}
				>
					Import
				</button>
				<button
					class="rounded-lg px-3 py-1.5 text-xs {activeTab === 'ical' ? 'bg-sky-500/20 text-sky-200' : 'text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-panel-strong))]'}"
					on:click={() => (activeTab = 'ical')}
				>
					iCalendar
				</button>
			</div>

			{#if activeTab === 'export'}
				<div class="mt-4 space-y-4">
					<div class="grid gap-3 md:grid-cols-2">
						<div>
							<label for="export-namespace" class="mb-1 block text-xs text-[rgb(var(--mv-muted))]">Namespace (optional)</label>
							<input
								id="export-namespace"
								bind:value={exportNamespace}
								class="w-full rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))] px-3 py-2 text-sm text-[rgb(var(--mv-text))]"
								placeholder="default"
							/>
						</div>
						<label class="flex items-center gap-2 rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))]/30 px-3 py-2 text-xs text-[rgb(var(--mv-muted))]">
							<input type="checkbox" bind:checked={includeRelationships} />
							Include relationships
						</label>
					</div>
					<button
						class="rounded-lg bg-sky-500 px-4 py-2 text-xs font-semibold text-white hover:bg-sky-400 disabled:opacity-60"
						on:click={handleExportBundle}
						disabled={exporting}
					>
						{exporting ? 'Exporting...' : 'Download JSON Bundle'}
					</button>
				</div>
			{:else if activeTab === 'import'}
				<div class="mt-4 space-y-4">
					<div>
						<label class="mb-1 block text-xs text-[rgb(var(--mv-muted))]" for="bundle-import">Import JSON Bundle</label>
						<input id="bundle-import" type="file" accept=".json" on:change={handleImportFile} class="text-sm text-[rgb(var(--mv-text))]" />
						{#if importFile}
							<p class="mt-1 text-[11px] text-[rgb(var(--mv-muted))]">Loaded: {importFile.name}</p>
						{/if}
					</div>

					{#if importBundle}
						<div class="grid gap-3 md:grid-cols-3">
							<div class="rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))]/30 p-3">
								<p class="text-[10px] uppercase tracking-wide text-[rgb(var(--mv-muted))]/70">Nodes</p>
								<p class="mt-1 text-lg font-semibold text-[rgb(var(--mv-text))]">{importBundle.nodes.length}</p>
							</div>
							<div class="rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))]/30 p-3">
								<p class="text-[10px] uppercase tracking-wide text-[rgb(var(--mv-muted))]/70">Relationships</p>
								<p class="mt-1 text-lg font-semibold text-[rgb(var(--mv-text))]">{importBundle.relationships.length}</p>
							</div>
							<div class="rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))]/30 p-3">
								<p class="text-[10px] uppercase tracking-wide text-[rgb(var(--mv-muted))]/70">Potential Conflicts</p>
								<p class="mt-1 text-lg font-semibold text-amber-300">{conflictPreview.length}</p>
							</div>
						</div>

						<div class="grid gap-3 md:grid-cols-2">
							<label class="flex items-center gap-2 rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))]/30 px-3 py-2 text-xs text-[rgb(var(--mv-muted))]">
								<input type="checkbox" bind:checked={overwriteExisting} />
								Overwrite existing nodes on ID conflict
							</label>
							<label class="flex items-center gap-2 rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))]/30 px-3 py-2 text-xs text-[rgb(var(--mv-muted))]">
								<input type="checkbox" bind:checked={importIncludeRelationships} />
								Import relationships
							</label>
						</div>

						<div>
							<label for="import-namespace-override" class="mb-1 block text-xs text-[rgb(var(--mv-muted))]">Namespace Override (optional)</label>
							<input
								id="import-namespace-override"
								bind:value={importNamespaceOverride}
								class="w-full rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))] px-3 py-2 text-sm text-[rgb(var(--mv-text))]"
								placeholder="target namespace"
							/>
						</div>

						<div class="rounded-xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))]/20 p-3">
							<div class="mb-2 flex items-center justify-between">
								<h3 class="text-sm font-semibold text-[rgb(var(--mv-text))]">Conflict Resolution Preview</h3>
								{#if conflictPreviewLoading}
									<span class="text-[10px] text-[rgb(var(--mv-muted))]/70">Analyzing...</span>
								{/if}
							</div>
							{#if !conflictPreviewLoading && conflictPreview.length === 0}
								<p class="text-xs text-[rgb(var(--mv-muted))]/70">No ID conflicts detected in the preview sample.</p>
							{:else}
								<div class="space-y-2 max-h-44 overflow-y-auto">
									{#each conflictPreview as conflict (conflict.id)}
										<div class="rounded-lg border border-amber-500/20 bg-amber-500/5 p-2">
											<p class="text-[10px] font-semibold text-amber-200">Node {conflict.id.slice(0, 8)}</p>
											<div class="mt-1 grid gap-2 md:grid-cols-2">
												<div>
													<p class="text-[10px] text-[rgb(var(--mv-muted))]/70">Existing</p>
													<p class="text-xs text-[rgb(var(--mv-text))]">{conflict.existing.title || 'Untitled'}</p>
													<p class="line-clamp-2 text-[10px] text-[rgb(var(--mv-muted))]/70">{conflict.existing.content}</p>
												</div>
												<div>
													<p class="text-[10px] text-[rgb(var(--mv-muted))]/70">Incoming</p>
													<p class="text-xs text-[rgb(var(--mv-text))]">{conflict.incoming.title || 'Untitled'}</p>
													<p class="line-clamp-2 text-[10px] text-[rgb(var(--mv-muted))]/70">{conflict.incoming.content}</p>
												</div>
											</div>
										</div>
									{/each}
								</div>
							{/if}
						</div>

						<button
							class="rounded-lg bg-sky-500 px-4 py-2 text-xs font-semibold text-white hover:bg-sky-400 disabled:opacity-60"
							on:click={handleImportBundle}
							disabled={importing}
						>
							{importing ? 'Importing...' : 'Import Bundle'}
						</button>
					{/if}

					{#if importResult}
						<div class="rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))]/20 p-3 text-xs text-[rgb(var(--mv-muted))]">
							Imported {importResult.imported_nodes}, updated {importResult.updated_nodes}, skipped {importResult.skipped_nodes}; relationships imported {importResult.imported_relationships}.
						</div>
					{/if}
				</div>
			{:else}
				<div class="mt-4 space-y-4">
					<button
						class="rounded-lg bg-sky-500 px-4 py-2 text-xs font-semibold text-white hover:bg-sky-400 disabled:opacity-60"
						on:click={handleExportIcal}
						disabled={exportingIcal}
					>
						{exportingIcal ? 'Exporting...' : 'Download .ics'}
					</button>

					<div class="grid gap-3 md:grid-cols-[220px_1fr]">
						<div>
							<label for="ical-duplicate" class="mb-1 block text-xs text-[rgb(var(--mv-muted))]">Duplicate Strategy</label>
							<select
								id="ical-duplicate"
								bind:value={icalDuplicateStrategy}
								class="w-full rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))] px-3 py-2 text-sm text-[rgb(var(--mv-text))]"
							>
								<option value="skip">Skip duplicates</option>
								<option value="update">Update duplicates</option>
								<option value="duplicate">Duplicate entries</option>
							</select>
						</div>
						<div>
							<label for="ical-import" class="mb-1 block text-xs text-[rgb(var(--mv-muted))]">Import .ics</label>
							<input id="ical-import" type="file" accept=".ics" on:change={handleIcalImport} class="text-sm text-[rgb(var(--mv-text))]" />
							{#if importingIcal}
								<p class="mt-1 text-[11px] text-[rgb(var(--mv-muted))]/70">Importing...</p>
							{/if}
						</div>
					</div>

					{#if icalResult}
						<div class="rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))]/20 p-3 text-xs text-[rgb(var(--mv-muted))]">
							Imported {icalResult.imported_count}, skipped {icalResult.skipped_count}, errors {icalResult.errors.length}.
						</div>
					{/if}
				</div>
			{/if}
		</div>
	</div>
{/if}
