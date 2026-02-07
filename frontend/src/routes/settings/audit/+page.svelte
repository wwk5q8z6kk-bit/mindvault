<script lang="ts">
	import { listAuditLogs, type AuditEntry } from '$lib/api/audit';
	import { pushToast } from '$lib/stores/toast';
	import { onMount } from 'svelte';

	let entries: AuditEntry[] = [];
	let loading = true;
	let subjectFilter = '';
	let successFilter: 'all' | 'success' | 'failure' = 'all';
	let limit = 50;
	let offset = 0;
	let hasMore = true;
	let expandedId: string | null = null;

	function statusColor(code: number): string {
		if (code >= 200 && code < 300) return 'text-emerald-400';
		if (code >= 400 && code < 500) return 'text-red-400';
		if (code >= 500) return 'text-red-500';
		return 'text-amber-400';
	}

	function statusBg(code: number): string {
		if (code >= 200 && code < 300) return 'bg-emerald-500/10';
		if (code >= 400) return 'bg-red-500/10';
		return 'bg-amber-500/10';
	}

	function methodColor(method: string): string {
		switch (method) {
			case 'GET': return 'text-sky-400';
			case 'POST': return 'text-emerald-400';
			case 'PUT': case 'PATCH': return 'text-amber-400';
			case 'DELETE': return 'text-red-400';
			default: return 'text-slate-400';
		}
	}

	function formatTimestamp(ts: string): string {
		try {
			const d = new Date(ts);
			return d.toLocaleString(undefined, {
				month: 'short',
				day: 'numeric',
				hour: '2-digit',
				minute: '2-digit',
				second: '2-digit'
			});
		} catch {
			return ts;
		}
	}

	function toggleRow(id: string) {
		expandedId = expandedId === id ? null : id;
	}

	async function fetchLogs(reset = false) {
		if (reset) {
			offset = 0;
			entries = [];
			hasMore = true;
		}
		loading = true;
		try {
			const result = await listAuditLogs({
				limit,
				offset,
				subject: subjectFilter.trim() || undefined
			});
			if (reset) {
				entries = result;
			} else {
				entries = [...entries, ...result];
			}
			hasMore = result.length === limit;
		} catch {
			pushToast('Failed to load audit logs', 'danger');
		} finally {
			loading = false;
		}
	}

	function loadMore() {
		offset += limit;
		fetchLogs(false);
	}

	function applyFilters() {
		fetchLogs(true);
	}

	$: filteredEntries = successFilter === 'all'
		? entries
		: successFilter === 'success'
			? entries.filter((e) => e.success)
			: entries.filter((e) => !e.success);

	onMount(() => {
		fetchLogs(true);
	});
</script>

<div class="mx-auto max-w-4xl">
	<div class="mb-4 flex items-center gap-3">
		<a
			href="/settings"
			class="rounded-lg border border-slate-700 px-2.5 py-1 text-xs text-slate-300 hover:bg-slate-800"
		>
			&larr; Settings
		</a>
		<div>
			<h2 class="text-lg font-semibold text-white">Audit Log</h2>
			<p class="text-xs text-slate-400">View API request history and server activity.</p>
		</div>
	</div>

	<!-- Filters -->
	<div class="mb-4 flex flex-wrap items-end gap-3 rounded-xl border border-slate-800 bg-slate-900/40 p-4">
		<div class="flex flex-col gap-1">
			<label class="text-[10px] uppercase tracking-wider text-slate-500" for="audit-subject">Subject</label>
			<input
				id="audit-subject"
				class="rounded-lg border border-slate-700 bg-slate-800 px-3 py-1.5 text-xs text-white outline-none focus:border-sky-500"
				placeholder="Filter by subject..."
				bind:value={subjectFilter}
			/>
		</div>
		<div class="flex flex-col gap-1">
			<span class="text-[10px] uppercase tracking-wider text-slate-500">Status</span>
			<div class="flex rounded-lg border border-slate-700 text-[10px]" role="group" aria-label="Status filter">
				<button
					class={`px-3 py-1.5 transition ${successFilter === 'all' ? 'bg-slate-700 text-white' : 'text-slate-400 hover:text-white'}`}
					on:click={() => { successFilter = 'all'; }}
				>
					All
				</button>
				<button
					class={`px-3 py-1.5 transition ${successFilter === 'success' ? 'bg-emerald-500/20 text-emerald-300' : 'text-slate-400 hover:text-white'}`}
					on:click={() => { successFilter = 'success'; }}
				>
					Success
				</button>
				<button
					class={`px-3 py-1.5 transition ${successFilter === 'failure' ? 'bg-red-500/20 text-red-300' : 'text-slate-400 hover:text-white'}`}
					on:click={() => { successFilter = 'failure'; }}
				>
					Failure
				</button>
			</div>
		</div>
		<button
			class="rounded-lg bg-sky-500 px-4 py-1.5 text-xs font-semibold text-white hover:bg-sky-400"
			on:click={applyFilters}
		>
			Apply
		</button>
		<span class="ml-auto text-[10px] text-slate-500">{filteredEntries.length} entries</span>
	</div>

	<!-- Table -->
	{#if loading && entries.length === 0}
		<div class="flex h-48 items-center justify-center rounded-xl border border-slate-800 bg-slate-900/40">
			<p class="text-xs text-slate-400">Loading audit logs...</p>
		</div>
	{:else if filteredEntries.length === 0}
		<div class="flex h-48 items-center justify-center rounded-xl border border-slate-800 bg-slate-900/40">
			<p class="text-xs text-slate-500">No audit entries found.</p>
		</div>
	{:else}
		<div class="overflow-hidden rounded-xl border border-slate-800">
			<table class="w-full text-left text-xs">
				<thead>
					<tr class="border-b border-slate-800 bg-slate-900/60 text-[10px] uppercase tracking-wider text-slate-500">
						<th class="px-3 py-2">Timestamp</th>
						<th class="px-3 py-2">Subject</th>
						<th class="px-3 py-2">Method</th>
						<th class="px-3 py-2">Path</th>
						<th class="px-3 py-2 text-right">Status</th>
						<th class="px-3 py-2 text-right">Latency</th>
					</tr>
				</thead>
				<tbody>
					{#each filteredEntries as entry (entry.request_id)}
						<tr
							class="border-b border-slate-800/40 transition hover:bg-slate-900/40 cursor-pointer {statusBg(entry.status_code)}"
							on:click={() => toggleRow(entry.request_id)}
							role="button"
							tabindex="0"
							on:keydown={(e) => { if (e.key === 'Enter') toggleRow(entry.request_id); }}
						>
							<td class="whitespace-nowrap px-3 py-2 text-slate-400">
								{formatTimestamp(entry.timestamp)}
							</td>
							<td class="px-3 py-2 text-slate-300">
								{entry.subject ?? '-'}
							</td>
							<td class="px-3 py-2 font-mono font-semibold {methodColor(entry.method)}">
								{entry.method}
							</td>
							<td class="max-w-[200px] truncate px-3 py-2 font-mono text-slate-300">
								{entry.path}
							</td>
							<td class="whitespace-nowrap px-3 py-2 text-right font-mono {statusColor(entry.status_code)}">
								{entry.status_code}
							</td>
							<td class="whitespace-nowrap px-3 py-2 text-right text-slate-500">
								{entry.latency_ms.toFixed(0)}ms
							</td>
						</tr>
						{#if expandedId === entry.request_id}
							<tr class="border-b border-slate-800/40 bg-slate-950/60">
								<td colspan="6" class="px-4 py-3">
									<div class="grid grid-cols-2 gap-x-6 gap-y-2 text-[11px]">
										<div>
											<span class="text-slate-500">Request ID:</span>
											<span class="ml-1 font-mono text-slate-300">{entry.request_id}</span>
										</div>
										<div>
											<span class="text-slate-500">Action:</span>
											<span class="ml-1 text-slate-300">{entry.action ?? '-'}</span>
										</div>
										<div>
											<span class="text-slate-500">Resource ID:</span>
											<span class="ml-1 font-mono text-slate-300">{entry.resource_id ?? '-'}</span>
										</div>
										<div>
											<span class="text-slate-500">Namespace:</span>
											<span class="ml-1 text-slate-300">{entry.namespace ?? '-'}</span>
										</div>
										<div>
											<span class="text-slate-500">Role:</span>
											<span class="ml-1 text-slate-300">{entry.role ?? '-'}</span>
										</div>
										<div>
											<span class="text-slate-500">Success:</span>
											<span class="ml-1 {entry.success ? 'text-emerald-400' : 'text-red-400'}">
												{entry.success ? 'Yes' : 'No'}
											</span>
										</div>
										{#if entry.error}
											<div class="col-span-2">
												<span class="text-slate-500">Error:</span>
												<span class="ml-1 text-red-300">{entry.error}</span>
											</div>
										{/if}
									</div>
								</td>
							</tr>
						{/if}
					{/each}
				</tbody>
			</table>
		</div>

		{#if hasMore}
			<div class="mt-3 flex justify-center">
				<button
					class="rounded-lg border border-slate-700 px-4 py-2 text-xs text-slate-300 transition hover:bg-slate-800 disabled:opacity-50"
					on:click={loadMore}
					disabled={loading}
				>
					{loading ? 'Loading...' : 'Load More'}
				</button>
			</div>
		{/if}
	{/if}
</div>
