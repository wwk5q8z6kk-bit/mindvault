<script lang="ts">
	import { onMount } from 'svelte';
	import { pushToast } from '$lib/stores/toast';
	import {
		listMcpConnectors,
		createMcpConnector,
		updateMcpConnector,
		deleteMcpConnector,
		type McpConnector
	} from '$lib/api/mcp';

	type EditDraft = {
		id: string;
		name: string;
		version: string;
		description: string;
		publisher: string;
		homepage_url: string;
		repository_url: string;
		capabilities: string;
		verified: boolean;
		config_schema: string;
	};

	let connectors: McpConnector[] = [];
	let error = '';
	let refreshing = false;
	let creating = false;
	let deletingId = '';
	let editing: EditDraft | null = null;

	let newName = '';
	let newVersion = '';
	let newDescription = '';
	let newPublisher = '';
	let newHomepageUrl = '';
	let newRepositoryUrl = '';
	let newCapabilities = '';
	let newVerified = false;
	let newConfigSchema = '{}';

	let searchQuery = '';
	let publisherFilter = '';
	let verifiedOnly = false;

	let bulkImportJson = '';
	let importing = false;
	let importSummary = '';
	let importErrors: string[] = [];

	onMount(() => {
		void refreshConnectors();
	});

	function formatDate(iso: string): string {
		return new Date(iso).toLocaleString();
	}

	function formatCapabilities(list: string[]): string {
		return list.length > 0 ? list.join(', ') : 'none';
	}

	function parseCapabilities(input: string): string[] {
		return input
			.split(',')
			.map((item) => item.trim())
			.filter((item) => item.length > 0);
	}

	function parseSchema(input: string): Record<string, unknown> | null {
		const trimmed = input.trim();
		if (!trimmed) return {};
		try {
			return JSON.parse(trimmed) as Record<string, unknown>;
		} catch {
			return null;
		}
	}

	function parseDate(iso: string): number {
		const parsed = Date.parse(iso);
		return Number.isNaN(parsed) ? 0 : parsed;
	}

	function matchesSearch(connector: McpConnector, query: string): boolean {
		const normalized = query.trim().toLowerCase();
		if (!normalized) return true;
		const haystack = [
			connector.name,
			connector.version,
			connector.description ?? '',
			connector.publisher ?? '',
			connector.homepage_url ?? '',
			connector.repository_url ?? '',
			connector.capabilities.join(', ')
		]
			.join(' ')
			.toLowerCase();
		return haystack.includes(normalized);
	}

	function matchesPublisher(connector: McpConnector, filterValue: string): boolean {
		const normalized = filterValue.trim().toLowerCase();
		if (!normalized) return true;
		return (connector.publisher ?? '').toLowerCase().includes(normalized);
	}

	function startEdit(connector: McpConnector) {
		editing = {
			id: connector.id,
			name: connector.name,
			version: connector.version,
			description: connector.description ?? '',
			publisher: connector.publisher ?? '',
			homepage_url: connector.homepage_url ?? '',
			repository_url: connector.repository_url ?? '',
			capabilities: connector.capabilities.join(', '),
			verified: connector.verified,
			config_schema: JSON.stringify(connector.config_schema ?? {}, null, 2)
		};
	}

	function resetNewForm() {
		newName = '';
		newVersion = '';
		newDescription = '';
		newPublisher = '';
		newHomepageUrl = '';
		newRepositoryUrl = '';
		newCapabilities = '';
		newVerified = false;
		newConfigSchema = '{}';
	}

	async function refreshConnectors() {
		refreshing = true;
		error = '';
		try {
			connectors = await listMcpConnectors({ limit: 200 });
		} catch (e: any) {
			error = e?.message ?? 'Failed to load MCP connectors';
		} finally {
			refreshing = false;
		}
	}

	async function handleCreate() {
		if (!newName.trim() || !newVersion.trim()) {
			pushToast('Name and version are required', 'warning');
			return;
		}
		const schema = parseSchema(newConfigSchema);
		if (schema === null) {
			pushToast('Config schema must be valid JSON', 'warning');
			return;
		}
		creating = true;
		try {
			await createMcpConnector({
				name: newName.trim(),
				version: newVersion.trim(),
				description: newDescription.trim() || undefined,
				publisher: newPublisher.trim() || undefined,
				homepage_url: newHomepageUrl.trim() || undefined,
				repository_url: newRepositoryUrl.trim() || undefined,
				capabilities: parseCapabilities(newCapabilities),
				verified: newVerified,
				config_schema: schema
			});
			pushToast('Connector added', 'success');
			resetNewForm();
			await refreshConnectors();
		} catch (e: any) {
			pushToast(e?.message ?? 'Failed to create connector', 'danger');
		} finally {
			creating = false;
		}
	}

	async function handleUpdate() {
		if (!editing) return;
		if (!editing.name.trim() || !editing.version.trim()) {
			pushToast('Name and version are required', 'warning');
			return;
		}
		const schema = parseSchema(editing.config_schema);
		if (schema === null) {
			pushToast('Config schema must be valid JSON', 'warning');
			return;
		}
		try {
			await updateMcpConnector(editing.id, {
				name: editing.name.trim(),
				version: editing.version.trim(),
				description: editing.description.trim() || undefined,
				publisher: editing.publisher.trim() || undefined,
				homepage_url: editing.homepage_url.trim() || undefined,
				repository_url: editing.repository_url.trim() || undefined,
				capabilities: parseCapabilities(editing.capabilities),
				verified: editing.verified,
				config_schema: schema
			});
			pushToast('Connector updated', 'success');
			editing = null;
			await refreshConnectors();
		} catch (e: any) {
			pushToast(e?.message ?? 'Failed to update connector', 'danger');
		}
	}

	async function handleDelete(connector: McpConnector) {
		if (!confirm(`Delete connector "${connector.name}"?`)) return;
		deletingId = connector.id;
		try {
			await deleteMcpConnector(connector.id);
			pushToast('Connector deleted', 'success');
			await refreshConnectors();
		} catch (e: any) {
			pushToast(e?.message ?? 'Failed to delete connector', 'danger');
		} finally {
			deletingId = '';
		}
	}

	async function handleBulkImport() {
		const trimmed = bulkImportJson.trim();
		if (!trimmed) {
			pushToast('Paste JSON first', 'warning');
			return;
		}
		let payloads: any;
		try {
			payloads = JSON.parse(trimmed);
		} catch {
			pushToast('Bulk import JSON is invalid', 'danger');
			return;
		}

		const items = Array.isArray(payloads) ? payloads : [payloads];
		if (items.length === 0) {
			pushToast('No connectors found in JSON', 'warning');
			return;
		}

		importing = true;
		importSummary = '';
		importErrors = [];
		let successCount = 0;
		let failedCount = 0;

		for (let index = 0; index < items.length; index += 1) {
			const entry = items[index] ?? {};
			const name = String(entry.name ?? '').trim();
			const version = String(entry.version ?? '').trim();
			if (!name || !version) {
				failedCount += 1;
				importErrors = [
					...importErrors,
					`[${index + 1}] Missing required name/version`
				];
				continue;
			}
			try {
				await createMcpConnector({
					name,
					version,
					description: entry.description ? String(entry.description) : undefined,
					publisher: entry.publisher ? String(entry.publisher) : undefined,
					homepage_url: entry.homepage_url ? String(entry.homepage_url) : undefined,
					repository_url: entry.repository_url ? String(entry.repository_url) : undefined,
					capabilities: Array.isArray(entry.capabilities)
						? entry.capabilities.map((cap: any) => String(cap))
						: undefined,
					verified: typeof entry.verified === 'boolean' ? entry.verified : undefined,
					config_schema:
						entry.config_schema && typeof entry.config_schema === 'object'
							? entry.config_schema
							: undefined
				});
				successCount += 1;
			} catch (e: any) {
				failedCount += 1;
				importErrors = [
					...importErrors,
					`[${index + 1}] ${e?.message ?? 'Failed to import'}`
				];
			}
		}

		importSummary = `Imported ${successCount} connector${successCount === 1 ? '' : 's'}.`;
		if (failedCount > 0) {
			importSummary += ` ${failedCount} failed.`;
		}
		pushToast(importSummary, failedCount > 0 ? 'warning' : 'success');
		importing = false;

		if (successCount > 0) {
			await refreshConnectors();
		}
	}

	$: filteredConnectors = connectors
		.filter((connector) => matchesSearch(connector, searchQuery))
		.filter((connector) => matchesPublisher(connector, publisherFilter))
		.filter((connector) => (verifiedOnly ? connector.verified : true))
		.slice()
		.sort((a, b) => parseDate(b.updated_at) - parseDate(a.updated_at));
</script>

<section class="rounded-xl border border-slate-800 bg-slate-900/40 p-5">
	<div class="flex items-center justify-between gap-3">
		<div>
			<h3 class="text-sm font-semibold text-white">MCP Connector Registry</h3>
			<p class="mt-1 text-[11px] text-slate-400">
				Manage MCP connectors exposed to agents. Create entries for custom MCP servers or marketplaces.
			</p>
		</div>
		<button
			class="rounded-lg border border-slate-700 px-2.5 py-1.5 text-[10px] text-slate-300 hover:bg-slate-800 disabled:opacity-50"
			on:click={refreshConnectors}
			disabled={refreshing}
		>
			{refreshing ? 'Refreshing...' : 'Refresh'}
		</button>
	</div>

	{#if error}
		<div class="mt-3 rounded-lg border border-red-500/30 bg-red-500/10 px-3 py-2 text-xs text-red-300">
			{error}
		</div>
	{/if}

	<div class="mt-4 grid grid-cols-1 gap-3 md:grid-cols-4">
		<div class="md:col-span-2">
			<label class="text-[10px] uppercase tracking-wider text-slate-500" for="mcp-search">
				Search
			</label>
			<input
				id="mcp-search"
				class="mt-1 w-full rounded border border-slate-700 bg-slate-900 px-2 py-1 text-xs text-slate-200"
				placeholder="Name, publisher, capability..."
				bind:value={searchQuery}
			/>
		</div>
		<div>
			<label class="text-[10px] uppercase tracking-wider text-slate-500" for="mcp-publisher-filter">
				Publisher
			</label>
			<input
				id="mcp-publisher-filter"
				class="mt-1 w-full rounded border border-slate-700 bg-slate-900 px-2 py-1 text-xs text-slate-200"
				placeholder="Filter by publisher"
				bind:value={publisherFilter}
			/>
		</div>
		<div class="flex items-center gap-3 md:pt-5">
			<label class="flex items-center gap-2 text-[11px] text-slate-400">
				<input type="checkbox" class="h-3 w-3" bind:checked={verifiedOnly} />
				Verified only
			</label>
			<button
				class="rounded-md border border-slate-700 px-2 py-1 text-[10px] text-slate-300 hover:bg-slate-800"
				on:click={() => {
					searchQuery = '';
					publisherFilter = '';
					verifiedOnly = false;
				}}
			>
				Clear
			</button>
		</div>
	</div>

	<p class="mt-3 text-[10px] text-slate-500">
		Showing {filteredConnectors.length} of {connectors.length} connectors.
	</p>

	<div class="mt-4 space-y-2">
		{#if refreshing && connectors.length === 0}
			<p class="text-xs text-slate-500">Loading connectors...</p>
		{:else if connectors.length === 0}
			<div class="rounded-lg border border-dashed border-slate-800 px-3 py-3 text-center text-[11px] text-slate-500">
				No MCP connectors registered yet.
			</div>
		{:else if filteredConnectors.length === 0}
			<div class="rounded-lg border border-dashed border-slate-800 px-3 py-3 text-center text-[11px] text-slate-500">
				No connectors match your filters.
			</div>
		{:else}
			{#each filteredConnectors as connector (connector.id)}
				<div class="rounded-lg border border-slate-800/60 bg-slate-950/40 px-3 py-3">
					<div class="flex flex-wrap items-center justify-between gap-3">
						<div class="min-w-0">
							<div class="flex flex-wrap items-center gap-2">
								<span class="truncate text-xs font-semibold text-white">{connector.name}</span>
								<span class="rounded bg-slate-800 px-1.5 py-0.5 text-[9px] text-slate-400">
									v{connector.version}
								</span>
								<span
									class="rounded px-1.5 py-0.5 text-[9px] {connector.verified
										? 'bg-emerald-500/15 text-emerald-300'
										: 'bg-slate-700 text-slate-400'}"
								>
									{connector.verified ? 'verified' : 'unverified'}
								</span>
							</div>
							<p class="mt-1 text-[10px] text-slate-500">
								{connector.publisher ?? 'Unknown publisher'} · Capabilities: {formatCapabilities(connector.capabilities)}
							</p>
							{#if connector.description}
								<p class="mt-1 text-[11px] text-slate-300">{connector.description}</p>
							{/if}
							<p class="mt-2 text-[10px] text-slate-500">
								Updated {formatDate(connector.updated_at)}
							</p>
						</div>
						<div class="flex items-center gap-2">
							<button
								class="rounded-md border border-slate-700 px-2 py-1 text-[10px] text-slate-300 hover:bg-slate-800"
								on:click={() => startEdit(connector)}
							>
								Edit
							</button>
							<button
								class="rounded-md border border-red-500/40 px-2 py-1 text-[10px] text-red-300 hover:bg-red-500/10 disabled:opacity-50"
								on:click={() => handleDelete(connector)}
								disabled={deletingId === connector.id}
							>
								{deletingId === connector.id ? 'Deleting...' : 'Delete'}
							</button>
						</div>
					</div>

					{#if editing && editing.id === connector.id}
						<div class="mt-3 grid grid-cols-1 gap-3 md:grid-cols-2">
							<div>
								<label class="text-[10px] uppercase tracking-wider text-slate-500" for={`edit-name-${connector.id}`}>
									Name
								</label>
								<input
									id={`edit-name-${connector.id}`}
									class="mt-1 w-full rounded border border-slate-700 bg-slate-900 px-2 py-1 text-xs text-slate-200"
									bind:value={editing.name}
								/>
							</div>
							<div>
								<label class="text-[10px] uppercase tracking-wider text-slate-500" for={`edit-version-${connector.id}`}>
									Version
								</label>
								<input
									id={`edit-version-${connector.id}`}
									class="mt-1 w-full rounded border border-slate-700 bg-slate-900 px-2 py-1 text-xs text-slate-200"
									bind:value={editing.version}
								/>
							</div>
							<div>
								<label class="text-[10px] uppercase tracking-wider text-slate-500" for={`edit-publisher-${connector.id}`}>
									Publisher
								</label>
								<input
									id={`edit-publisher-${connector.id}`}
									class="mt-1 w-full rounded border border-slate-700 bg-slate-900 px-2 py-1 text-xs text-slate-200"
									bind:value={editing.publisher}
								/>
							</div>
							<div>
								<label class="text-[10px] uppercase tracking-wider text-slate-500" for={`edit-capabilities-${connector.id}`}>
									Capabilities (comma separated)
								</label>
								<input
									id={`edit-capabilities-${connector.id}`}
									class="mt-1 w-full rounded border border-slate-700 bg-slate-900 px-2 py-1 text-xs text-slate-200"
									bind:value={editing.capabilities}
								/>
							</div>
							<div class="md:col-span-2">
								<label class="text-[10px] uppercase tracking-wider text-slate-500" for={`edit-description-${connector.id}`}>
									Description
								</label>
								<input
									id={`edit-description-${connector.id}`}
									class="mt-1 w-full rounded border border-slate-700 bg-slate-900 px-2 py-1 text-xs text-slate-200"
									bind:value={editing.description}
								/>
							</div>
							<div>
								<label class="text-[10px] uppercase tracking-wider text-slate-500" for={`edit-homepage-${connector.id}`}>
									Homepage URL
								</label>
								<input
									id={`edit-homepage-${connector.id}`}
									class="mt-1 w-full rounded border border-slate-700 bg-slate-900 px-2 py-1 text-xs text-slate-200"
									bind:value={editing.homepage_url}
								/>
							</div>
							<div>
								<label class="text-[10px] uppercase tracking-wider text-slate-500" for={`edit-repo-${connector.id}`}>
									Repository URL
								</label>
								<input
									id={`edit-repo-${connector.id}`}
									class="mt-1 w-full rounded border border-slate-700 bg-slate-900 px-2 py-1 text-xs text-slate-200"
									bind:value={editing.repository_url}
								/>
							</div>
							<div class="md:col-span-2">
								<label class="text-[10px] uppercase tracking-wider text-slate-500" for={`edit-schema-${connector.id}`}>
									Config schema (JSON)
								</label>
								<textarea
									id={`edit-schema-${connector.id}`}
									rows={4}
									class="mt-1 w-full rounded border border-slate-700 bg-slate-900 px-2 py-1 font-mono text-[11px] text-slate-200"
									bind:value={editing.config_schema}
								></textarea>
							</div>
							<div class="flex items-center gap-2 md:col-span-2">
								<input
									id={`edit-verified-${connector.id}`}
									type="checkbox"
									class="h-3 w-3"
									bind:checked={editing.verified}
								/>
								<label class="text-[10px] text-slate-400" for={`edit-verified-${connector.id}`}>
									Verified
								</label>
							</div>
						</div>
						<div class="mt-3 flex flex-wrap gap-2">
							<button
								class="rounded-lg bg-sky-500 px-3 py-2 text-xs font-semibold text-white hover:bg-sky-400"
								on:click={handleUpdate}
							>
								Save changes
							</button>
							<button
								class="rounded-lg border border-slate-700 px-3 py-2 text-xs text-slate-300 hover:bg-slate-800"
								on:click={() => {
									editing = null;
								}}
							>
								Cancel
							</button>
						</div>
					{/if}
				</div>
			{/each}
		{/if}
	</div>

	<div class="mt-4 border-t border-slate-800/60 pt-4">
		<h4 class="text-[10px] uppercase tracking-wider text-slate-500">Add Connector</h4>
		<div class="mt-2 grid grid-cols-1 gap-3 md:grid-cols-2">
			<div>
				<label class="text-[10px] uppercase tracking-wider text-slate-500" for="mcp-name">Name</label>
				<input
					id="mcp-name"
					class="mt-1 w-full rounded border border-slate-700 bg-slate-900 px-2 py-1 text-xs text-slate-200"
					placeholder="Notion MCP"
					bind:value={newName}
				/>
			</div>
			<div>
				<label class="text-[10px] uppercase tracking-wider text-slate-500" for="mcp-version">Version</label>
				<input
					id="mcp-version"
					class="mt-1 w-full rounded border border-slate-700 bg-slate-900 px-2 py-1 text-xs text-slate-200"
					placeholder="1.0.0"
					bind:value={newVersion}
				/>
			</div>
			<div>
				<label class="text-[10px] uppercase tracking-wider text-slate-500" for="mcp-publisher">
					Publisher
				</label>
				<input
					id="mcp-publisher"
					class="mt-1 w-full rounded border border-slate-700 bg-slate-900 px-2 py-1 text-xs text-slate-200"
					placeholder="MindVault Labs"
					bind:value={newPublisher}
				/>
			</div>
			<div>
				<label class="text-[10px] uppercase tracking-wider text-slate-500" for="mcp-capabilities">
					Capabilities (comma separated)
				</label>
				<input
					id="mcp-capabilities"
					class="mt-1 w-full rounded border border-slate-700 bg-slate-900 px-2 py-1 text-xs text-slate-200"
					placeholder="read, write, search"
					bind:value={newCapabilities}
				/>
			</div>
			<div class="md:col-span-2">
				<label class="text-[10px] uppercase tracking-wider text-slate-500" for="mcp-description">
					Description
				</label>
				<input
					id="mcp-description"
					class="mt-1 w-full rounded border border-slate-700 bg-slate-900 px-2 py-1 text-xs text-slate-200"
					placeholder="Connector for team knowledge bases"
					bind:value={newDescription}
				/>
			</div>
			<div>
				<label class="text-[10px] uppercase tracking-wider text-slate-500" for="mcp-homepage">
					Homepage URL
				</label>
				<input
					id="mcp-homepage"
					class="mt-1 w-full rounded border border-slate-700 bg-slate-900 px-2 py-1 text-xs text-slate-200"
					placeholder="https://example.com"
					bind:value={newHomepageUrl}
				/>
			</div>
			<div>
				<label class="text-[10px] uppercase tracking-wider text-slate-500" for="mcp-repo">
					Repository URL
				</label>
				<input
					id="mcp-repo"
					class="mt-1 w-full rounded border border-slate-700 bg-slate-900 px-2 py-1 text-xs text-slate-200"
					placeholder="https://github.com/org/repo"
					bind:value={newRepositoryUrl}
				/>
			</div>
			<div class="md:col-span-2">
				<label class="text-[10px] uppercase tracking-wider text-slate-500" for="mcp-schema">
					Config schema (JSON)
				</label>
				<textarea
					id="mcp-schema"
					rows={4}
					class="mt-1 w-full rounded border border-slate-700 bg-slate-900 px-2 py-1 font-mono text-[11px] text-slate-200"
					bind:value={newConfigSchema}
				></textarea>
			</div>
			<div class="flex items-center gap-2 md:col-span-2">
				<input id="mcp-verified" type="checkbox" class="h-3 w-3" bind:checked={newVerified} />
				<label class="text-[10px] text-slate-400" for="mcp-verified">Verified</label>
			</div>
		</div>
		<button
			class="mt-3 rounded-lg bg-sky-500 px-3 py-2 text-xs font-semibold text-white hover:bg-sky-400 disabled:opacity-50"
			on:click={handleCreate}
			disabled={creating || !newName.trim() || !newVersion.trim()}
		>
			{creating ? 'Creating...' : 'Add connector'}
		</button>
	</div>

	<div class="mt-4 border-t border-slate-800/60 pt-4">
		<h4 class="text-[10px] uppercase tracking-wider text-slate-500">Bulk Import</h4>
		<p class="mt-1 text-[10px] text-slate-500">
			Paste a JSON array (or single object) with connector fields. Required: name, version.
		</p>
		<textarea
			class="mt-2 w-full rounded border border-slate-700 bg-slate-900 px-2 py-2 font-mono text-[11px] text-slate-200"
			rows={5}
			placeholder={'[{"name":"Notion MCP","version":"1.0.0","publisher":"MindVault","capabilities":["read","write"]}]'}
			bind:value={bulkImportJson}
		></textarea>
		<div class="mt-2 flex flex-wrap items-center gap-2">
			<button
				class="rounded-lg bg-sky-500 px-3 py-2 text-xs font-semibold text-white hover:bg-sky-400 disabled:opacity-50"
				on:click={handleBulkImport}
				disabled={importing}
			>
				{importing ? 'Importing...' : 'Import connectors'}
			</button>
			{#if importSummary}
				<span class="text-[11px] text-slate-400">{importSummary}</span>
			{/if}
		</div>
		{#if importErrors.length > 0}
			<div class="mt-2 rounded-lg border border-amber-500/20 bg-amber-500/10 px-3 py-2 text-[10px] text-amber-200">
				<p class="font-semibold">Import issues</p>
				<ul class="mt-1 list-disc pl-4">
					{#each importErrors as err}
						<li>{err}</li>
					{/each}
				</ul>
			</div>
		{/if}
	</div>
</section>
