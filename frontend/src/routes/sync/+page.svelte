<script lang="ts">
	import { onMount } from 'svelte';
	import { pushToast } from '$lib/stores/toast';
	import { syncStatus, syncExport, syncImport, type SyncStats, type SyncImportStats } from '$lib/api/sync';

	let stats: SyncStats | null = null;
	let lastImportStats: SyncImportStats | null = null;
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
			lastImportStats = res;
			pushToast(
				`Import complete: ${res.inserted} inserted, ${res.updated} updated, ${res.conflicts} conflicts`,
				'success'
			);
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
		<div class="grid grid-cols-4 gap-4">
			<div class="rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))] p-4 text-center">
				<div class="text-2xl font-bold text-[rgb(var(--mv-text))]">{stats.node_count}</div>
				<div class="text-xs text-[rgb(var(--mv-muted))]">Total Nodes</div>
			</div>
			<div class="rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))] p-4 text-center">
				<div class="text-sm font-medium text-[rgb(var(--mv-text))] break-all">
					{stats.device_id}
				</div>
				<div class="text-xs text-[rgb(var(--mv-muted))]">Device ID</div>
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
		{#if lastImportStats}
			<div class="mt-3 rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-bg))] p-3 text-sm text-[rgb(var(--mv-text))]">
				<div class="grid grid-cols-2 gap-2 text-xs text-[rgb(var(--mv-muted))]">
					<div>Scanned: <span class="text-[rgb(var(--mv-text))]">{lastImportStats.scanned}</span></div>
					<div>Inserted: <span class="text-[rgb(var(--mv-text))]">{lastImportStats.inserted}</span></div>
					<div>Updated: <span class="text-[rgb(var(--mv-text))]">{lastImportStats.updated}</span></div>
					<div>Skipped: <span class="text-[rgb(var(--mv-text))]">{lastImportStats.skipped}</span></div>
					<div>Conflicts: <span class="text-[rgb(var(--mv-text))]">{lastImportStats.conflicts}</span></div>
				</div>
				{#if lastImportStats.conflicts > 0}
					<div class="mt-3 space-y-2 text-xs">
						<p class="text-[rgb(var(--mv-muted))]">Review conflicts via Inbox proposals.</p>
						<a
							href="/inbox"
							class="inline-flex items-center rounded bg-blue-600 px-2.5 py-1 text-xs text-white hover:bg-blue-700"
						>Open Inbox</a>
						{#if lastImportStats.conflict_details?.length}
							<div class="mt-2 space-y-2">
								{#each lastImportStats.conflict_details as conflict (conflict.id)}
									<div class="rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))] p-2">
										<div class="text-[rgb(var(--mv-muted))]">
											{conflict.reason} • {new Date(conflict.detected_at).toLocaleString()}
										</div>
										<div class="mt-1 text-[rgb(var(--mv-text))]">
											Local: {conflict.local_content_preview}
										</div>
										<div class="mt-1 text-[rgb(var(--mv-text))]">
											Remote: {conflict.remote_content_preview}
										</div>
										{#if conflict.proposal_id}
											<div class="mt-1 text-[rgb(var(--mv-muted))]">
												Proposal: {conflict.proposal_id}
											</div>
										{/if}
									</div>
								{/each}
							</div>
						{/if}
					</div>
				{/if}
			</div>
		{/if}
	</div>
</div>
