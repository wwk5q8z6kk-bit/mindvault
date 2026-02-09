<script lang="ts">
    import { onMount } from 'svelte';
    import { slide } from 'svelte/transition';
    import { pushToast } from '$lib/stores/toast';
    import { listProxyAudit, type ProxyAuditEntry } from '$lib/api/proxy-audit';

    let entries: ProxyAuditEntry[] = [];
    let loading = true;
    let filterConsumer = '';
    let pageSize = 25;
    let offset = 0;

    onMount(async () => {
        await refresh();
    });

    async function refresh() {
        loading = true;
        try {
            entries = await listProxyAudit(
                filterConsumer || undefined,
                pageSize,
                offset
            );
        } catch (e) {
            pushToast('Failed to load audit log', 'danger');
        } finally {
            loading = false;
        }
    }

    async function applyFilter() {
        offset = 0;
        await refresh();
    }

    async function nextPage() {
        offset += pageSize;
        await refresh();
    }

    async function prevPage() {
        offset = Math.max(0, offset - pageSize);
        await refresh();
    }

    function formatTimestamp(iso: string): string {
        const d = new Date(iso);
        return d.toLocaleDateString('en-US', {
            month: 'short',
            day: 'numeric',
            hour: '2-digit',
            minute: '2-digit',
            second: '2-digit'
        });
    }

    function getStatusColor(entry: ProxyAuditEntry): string {
        if (entry.success === null) return 'text-slate-500 bg-slate-500/10';
        return entry.success
            ? 'text-emerald-400 bg-emerald-500/10'
            : 'text-red-400 bg-red-500/10';
    }

    function getStatusLabel(entry: ProxyAuditEntry): string {
        if (entry.success === null) return 'pending';
        return entry.success ? 'ok' : 'fail';
    }
</script>

<div class="rounded-2xl border border-slate-800 bg-slate-900/50 p-5 shadow-xl">
    <div class="flex items-center justify-between mb-5">
        <div>
            <h3 class="text-sm font-bold text-white mb-1">Proxy Audit Log</h3>
            <p class="text-[10px] text-slate-500">Every credential-proxied request, logged and sanitized.</p>
        </div>
        <button
            on:click={refresh}
            disabled={loading}
            class="p-2 rounded-xl bg-slate-800 hover:bg-slate-700 text-slate-400 hover:text-white transition-all disabled:opacity-50"
        >
            <svg class="w-4 h-4 {loading ? 'animate-spin' : ''}" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
            </svg>
        </button>
    </div>

    <!-- Filter bar -->
    <div class="mb-4 flex gap-2 items-end">
        <div class="flex-1">
            <label class="block text-[9px] text-slate-500 uppercase tracking-widest mb-1" for="audit-filter">Filter by consumer</label>
            <input
                id="audit-filter"
                bind:value={filterConsumer}
                placeholder="All consumers"
                on:keydown={(e) => e.key === 'Enter' && applyFilter()}
                class="w-full px-3 py-1.5 rounded-lg bg-slate-900 border border-slate-700 text-xs text-white placeholder:text-slate-600 focus:border-sky-500 focus:outline-none"
            />
        </div>
        <button
            on:click={applyFilter}
            class="px-4 py-1.5 rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-400 hover:text-white text-xs font-bold transition-all"
        >
            Filter
        </button>
    </div>

    <!-- Audit entries -->
    <div class="space-y-1.5">
        {#if loading && entries.length === 0}
            <div class="py-8 text-center">
                <p class="text-[10px] text-slate-500 animate-pulse">Loading audit log...</p>
            </div>
        {:else if entries.length === 0}
            <div class="py-8 text-center bg-slate-950/30 rounded-xl border border-dashed border-slate-800">
                <p class="text-[10px] text-slate-500 uppercase tracking-widest">No audit entries</p>
            </div>
        {:else}
            {#each entries as entry (entry.id)}
                <div transition:slide class="p-3 rounded-xl border border-slate-800/60 bg-slate-950/30 hover:bg-slate-950/50 transition-colors">
                    <div class="flex items-start justify-between gap-2">
                        <div class="min-w-0 flex-1">
                            <div class="flex items-center gap-2 flex-wrap">
                                <span class="text-[8px] px-1.5 py-0.5 rounded-full font-bold uppercase {getStatusColor(entry)}">
                                    {getStatusLabel(entry)}
                                </span>
                                <span class="text-[9px] px-1.5 py-0.5 rounded bg-slate-800 text-slate-400 font-mono uppercase">
                                    {entry.action}
                                </span>
                                <span class="text-xs text-white font-medium truncate">{entry.target}</span>
                                {#if entry.sanitized}
                                    <span class="text-[8px] px-1.5 py-0.5 rounded-full bg-amber-500/10 text-amber-400 font-bold uppercase">sanitized</span>
                                {/if}
                            </div>
                            <div class="flex gap-3 mt-1.5 flex-wrap">
                                <span class="text-[9px] text-sky-400 font-medium">{entry.consumer}</span>
                                <span class="text-[9px] text-slate-600 font-mono">{entry.secret_ref}</span>
                                {#if entry.response_status}
                                    <span class="text-[9px] text-slate-500">HTTP {entry.response_status}</span>
                                {/if}
                                <span class="text-[9px] text-slate-600">{formatTimestamp(entry.timestamp)}</span>
                            </div>
                            {#if entry.intent}
                                <p class="text-[9px] text-slate-500 mt-1 italic truncate">"{entry.intent}"</p>
                            {/if}
                            {#if entry.error}
                                <p class="text-[9px] text-red-400 mt-1 truncate">{entry.error}</p>
                            {/if}
                        </div>
                    </div>
                </div>
            {/each}
        {/if}
    </div>

    <!-- Pagination -->
    {#if entries.length > 0}
        <div class="flex items-center justify-between mt-4 pt-3 border-t border-slate-800/50">
            <button
                on:click={prevPage}
                disabled={offset === 0}
                class="px-3 py-1 rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-400 hover:text-white text-[10px] font-bold disabled:opacity-30 transition-all"
            >
                Previous
            </button>
            <span class="text-[9px] text-slate-600">
                {offset + 1}–{offset + entries.length}
            </span>
            <button
                on:click={nextPage}
                disabled={entries.length < pageSize}
                class="px-3 py-1 rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-400 hover:text-white text-[10px] font-bold disabled:opacity-30 transition-all"
            >
                Next
            </button>
        </div>
    {/if}
</div>
