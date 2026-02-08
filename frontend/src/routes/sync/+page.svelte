<script lang="ts">
	import { onMount } from 'svelte';
	import { pushToast } from '$lib/stores/toast';
	import { syncStatus, syncExport, syncImport, type SyncStats } from '$lib/api/sync';

	let stats: SyncStats | null = null;
	let loading = true;
	let exporting = false;
	let importing = false;
	let importFileInput: HTMLInputElement | null = null;
	let exportNamespace = '';

	onMount(async () => {
		await loadStatus();
	});

	async function loadStatus() {
		loading = true;
		try {
			stats = await syncStatus();
		} catch {
			pushToast('Failed to load sync status', 'danger');
		} finally {
			loading = false;
		}
	}

	async function handleExport() {
		exporting = true;
		try {
			const snapshot = await syncExport(
				exportNamespace ? { namespace: exportNamespace } : undefined
			);
			const blob = new Blob([JSON.stringify(snapshot, null, 2)], { type: 'application/json' });
			const url = URL.createObjectURL(blob);
			const a = document.createElement('a');
			a.href = url;
			a.download = `mindvault-export-${new Date().toISOString().slice(0, 10)}.json`;
			a.click();
			URL.revokeObjectURL(url);
			pushToast('Export downloaded', 'success');
			await loadStatus();
		} catch {
			pushToast('Export failed', 'danger');
		} finally {
			exporting = false;
		}
	}

	async function handleImportFile(e: Event) {
		const target = e.target as HTMLInputElement;
		const file = target.files?.[0];
		if (!file) return;

		importing = true;
		try {
			const text = await file.text();
			const snapshot = JSON.parse(text);
			const res = await syncImport(snapshot);
			pushToast(`Imported ${res.imported} nodes`, 'success');
			await loadStatus();
		} catch {
			pushToast('Import failed', 'danger');
		} finally {
			importing = false;
			if (importFileInput) importFileInput.value = '';
		}
	}
</script>

<div class="mx-auto max-w-3xl space-y-6 p-6">
	<h1 class="text-2xl font-bold text-[rgb(var(--mv-text))]">Device Sync</h1>
	<p class="text-sm text-[rgb(var(--mv-muted))]">
		Export and import vault snapshots for offline device synchronization.
	</p>

	{#if loading}
		<p class="text-[rgb(var(--mv-muted))]">Loading...</p>
	{:else if stats}
		<!-- Status -->
		<div class="grid grid-cols-3 gap-4">
			<div class="rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))] p-4 text-center">
				<div class="text-2xl font-bold text-[rgb(var(--mv-text))]">{stats.node_count}</div>
				<div class="text-xs text-[rgb(var(--mv-muted))]">Total Nodes</div>
			</div>
			<div class="rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))] p-4 text-center">
				<div class="text-sm font-medium text-[rgb(var(--mv-text))]">
					{stats.last_export ? new Date(stats.last_export).toLocaleDateString() : 'Never'}
				</div>
				<div class="text-xs text-[rgb(var(--mv-muted))]">Last Export</div>
			</div>
			<div class="rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))] p-4 text-center">
				<div class="text-sm font-medium text-[rgb(var(--mv-text))]">
					{stats.last_import ? new Date(stats.last_import).toLocaleDateString() : 'Never'}
				</div>
				<div class="text-xs text-[rgb(var(--mv-muted))]">Last Import</div>
			</div>
		</div>
	{/if}

	<!-- Export -->
	<div class="space-y-3 rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))] p-4">
		<h2 class="text-lg font-semibold text-[rgb(var(--mv-text))]">Export</h2>
		<div class="flex gap-2">
			<input
				bind:value={exportNamespace}
				class="flex-1 rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-bg))] px-3 py-2 text-sm text-[rgb(var(--mv-text))] placeholder-[rgb(var(--mv-muted))]"
				placeholder="Namespace (leave empty for all)"
			/>
			<button
				class="rounded bg-blue-600 px-4 py-2 text-sm text-white hover:bg-blue-700 disabled:opacity-50"
				onclick={handleExport}
				disabled={exporting}
			>{exporting ? 'Exporting...' : 'Export Snapshot'}</button>
		</div>
	</div>

	<!-- Import -->
	<div class="space-y-3 rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))] p-4">
		<h2 class="text-lg font-semibold text-[rgb(var(--mv-text))]">Import</h2>
		<p class="text-xs text-[rgb(var(--mv-muted))]">
			Import a JSON snapshot exported from another MindVault instance. Conflicts arrive as proposals in your Inbox.
		</p>
		<div>
			<input
				bind:this={importFileInput}
				type="file"
				accept=".json"
				onchange={handleImportFile}
				class="text-sm text-[rgb(var(--mv-muted))]"
			/>
			{#if importing}
				<p class="mt-2 text-sm text-[rgb(var(--mv-muted))]">Importing...</p>
			{/if}
		</div>
	</div>
</div>
