<script lang="ts">
    import { onMount } from 'svelte';
    import { slide } from 'svelte/transition';
    import { pushToast } from '$lib/stores/toast';
    import {
        type AccessPolicy,
        type PolicyMatrix,
        setPolicy,
        listPolicies,
        getPolicyMatrix,
        deletePolicy
    } from '$lib/api/policies';
    import { listConsumers, type ConsumerProfile } from '$lib/api/consumers';

    let policies: AccessPolicy[] = [];
    let matrix: PolicyMatrix | null = null;
    let consumers: ConsumerProfile[] = [];
    let loading = true;
    let viewMode: 'list' | 'matrix' = 'matrix';

    // New policy form
    let formSecretKey = '';
    let formConsumer = '';
    let formAllowed = true;
    let formTtl = '';
    let submitting = false;

    onMount(async () => {
        await refresh();
    });

    async function refresh() {
        loading = true;
        try {
            [policies, matrix, consumers] = await Promise.all([
                listPolicies(),
                getPolicyMatrix(),
                listConsumers()
            ]);
        } catch (e) {
            pushToast('Failed to load policies', 'danger');
        } finally {
            loading = false;
        }
    }

    async function handleSetPolicy() {
        if (!formSecretKey.trim() || !formConsumer.trim()) return;
        submitting = true;
        try {
            await setPolicy({
                secret_key: formSecretKey.trim(),
                consumer: formConsumer.trim(),
                allowed: formAllowed,
                max_ttl_seconds: formTtl ? parseInt(formTtl) : undefined
            });
            pushToast(
                `Policy ${formAllowed ? 'allowed' : 'denied'}: ${formConsumer} -> ${formSecretKey}`,
                'success'
            );
            formSecretKey = '';
            formConsumer = '';
            formTtl = '';
            await refresh();
        } catch (e) {
            pushToast('Failed to set policy', 'danger');
        } finally {
            submitting = false;
        }
    }

    async function handleDelete(id: string) {
        try {
            await deletePolicy(id);
            pushToast('Policy deleted', 'success');
            await refresh();
        } catch (e) {
            pushToast('Failed to delete policy', 'danger');
        }
    }

    async function handleToggle(secret: string, consumer: string, currentlyAllowed: boolean) {
        try {
            await setPolicy({
                secret_key: secret,
                consumer,
                allowed: !currentlyAllowed
            });
            await refresh();
        } catch (e) {
            pushToast('Failed to toggle policy', 'danger');
        }
    }

    function formatTtl(seconds: number | null): string {
        if (!seconds) return '-';
        if (seconds < 60) return `${seconds}s`;
        if (seconds < 3600) return `${Math.floor(seconds / 60)}m`;
        return `${Math.floor(seconds / 3600)}h`;
    }
</script>

<div class="rounded-2xl border border-slate-800 bg-slate-900/50 p-5 shadow-xl">
    <div class="flex items-center justify-between mb-5">
        <div>
            <h3 class="text-sm font-bold text-white mb-1">Access Policies</h3>
            <p class="text-[10px] text-slate-500">Per-secret, per-consumer ABAC with default deny.</p>
        </div>
        <div class="flex gap-2">
            <button
                on:click={() => viewMode = 'matrix'}
                class="px-3 py-1 rounded-lg text-[10px] font-bold transition-all {viewMode === 'matrix' ? 'bg-sky-600 text-white' : 'bg-slate-800 text-slate-400 hover:text-white'}"
            >
                Matrix
            </button>
            <button
                on:click={() => viewMode = 'list'}
                class="px-3 py-1 rounded-lg text-[10px] font-bold transition-all {viewMode === 'list' ? 'bg-sky-600 text-white' : 'bg-slate-800 text-slate-400 hover:text-white'}"
            >
                List
            </button>
            <button
                on:click={refresh}
                disabled={loading}
                aria-label="Refresh access policies"
                title="Refresh access policies"
                class="p-1.5 rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-400 hover:text-white transition-all disabled:opacity-50"
            >
                <svg class="w-3.5 h-3.5 {loading ? 'animate-spin' : ''}" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                </svg>
            </button>
        </div>
    </div>

    <!-- Set policy form -->
    <div class="mb-5 p-3 rounded-xl bg-slate-950/40 border border-slate-800">
        <div class="flex gap-2 items-end flex-wrap">
            <div class="flex-1 min-w-[120px]">
                <label class="block text-[9px] text-slate-500 uppercase tracking-widest mb-1" for="policy-secret">Secret Key</label>
                <input
                    id="policy-secret"
                    bind:value={formSecretKey}
                    placeholder="GITHUB_TOKEN"
                    class="w-full px-3 py-1.5 rounded-lg bg-slate-900 border border-slate-700 text-xs text-white placeholder:text-slate-600 focus:border-sky-500 focus:outline-none"
                />
            </div>
            <div class="flex-1 min-w-[120px]">
                <label class="block text-[9px] text-slate-500 uppercase tracking-widest mb-1" for="policy-consumer">Consumer</label>
                <select
                    id="policy-consumer"
                    bind:value={formConsumer}
                    class="w-full px-3 py-1.5 rounded-lg bg-slate-900 border border-slate-700 text-xs text-white focus:border-sky-500 focus:outline-none"
                >
                    <option value="">Select...</option>
                    {#each consumers.filter(c => !c.revoked_at) as c}
                        <option value={c.name}>{c.name}</option>
                    {/each}
                </select>
            </div>
            <div class="w-20">
                <label class="block text-[9px] text-slate-500 uppercase tracking-widest mb-1" for="policy-ttl">TTL (sec)</label>
                <input
                    id="policy-ttl"
                    bind:value={formTtl}
                    placeholder="300"
                    type="number"
                    class="w-full px-3 py-1.5 rounded-lg bg-slate-900 border border-slate-700 text-xs text-white placeholder:text-slate-600 focus:border-sky-500 focus:outline-none"
                />
            </div>
            <div class="flex items-center gap-2">
                <label class="flex items-center gap-1.5 cursor-pointer">
                    <input type="checkbox" bind:checked={formAllowed} class="rounded border-slate-600 bg-slate-900 text-sky-500" />
                    <span class="text-[10px] text-slate-400">{formAllowed ? 'Allow' : 'Deny'}</span>
                </label>
            </div>
            <button
                on:click={handleSetPolicy}
                disabled={submitting || !formSecretKey.trim() || !formConsumer}
                class="px-4 py-1.5 rounded-lg bg-sky-600 hover:bg-sky-500 text-white text-xs font-bold disabled:opacity-40 transition-all"
            >
                Set
            </button>
        </div>
    </div>

    <!-- Matrix view -->
    {#if viewMode === 'matrix' && matrix}
        <div class="overflow-x-auto rounded-xl border border-slate-800">
            <table class="w-full text-[10px]">
                <thead>
                    <tr class="bg-slate-950/50">
                        <th class="text-left px-3 py-2 text-slate-500 font-bold uppercase tracking-widest">Secret</th>
                        {#each matrix.consumers as consumer}
                            <th class="text-center px-3 py-2 text-slate-500 font-bold uppercase tracking-widest">{consumer}</th>
                        {/each}
                    </tr>
                </thead>
                <tbody>
                    {#each matrix.secrets as secret}
                        <tr class="border-t border-slate-800/50 hover:bg-slate-800/20">
                            <td class="px-3 py-2 text-white font-mono">{secret}</td>
                            {#each matrix.consumers as consumer}
                                <td class="text-center px-3 py-2">
                                    {#if matrix.matrix[secret]?.[consumer] !== undefined}
                                        <button
                                            on:click={() => handleToggle(secret, consumer, matrix?.matrix[secret]?.[consumer] ?? false)}
                                            class="w-6 h-6 rounded-lg transition-all {matrix.matrix[secret][consumer] ? 'bg-emerald-500/20 text-emerald-400 hover:bg-emerald-500/30' : 'bg-red-500/20 text-red-400 hover:bg-red-500/30'}"
                                        >
                                            {matrix.matrix[secret][consumer] ? '✓' : '✗'}
                                        </button>
                                    {:else}
                                        <button
                                            on:click={() => handleToggle(secret, consumer, false)}
                                            class="w-6 h-6 rounded-lg bg-slate-800/40 text-slate-600 hover:bg-slate-700/40 transition-all"
                                            title="No policy (default deny)"
                                        >
                                            -
                                        </button>
                                    {/if}
                                </td>
                            {/each}
                        </tr>
                    {/each}
                </tbody>
            </table>
            {#if matrix.secrets.length === 0 || matrix.consumers.length === 0}
                <div class="py-6 text-center">
                    <p class="text-[10px] text-slate-500">No secrets or consumers configured yet.</p>
                </div>
            {/if}
        </div>
    {/if}

    <!-- List view -->
    {#if viewMode === 'list'}
        <div class="space-y-2">
            {#if policies.length === 0}
                <div class="py-6 text-center bg-slate-950/30 rounded-xl border border-dashed border-slate-800">
                    <p class="text-[10px] text-slate-500 uppercase tracking-widest">No policies defined (default deny all)</p>
                </div>
            {:else}
                {#each policies as policy (policy.id)}
                    <div transition:slide class="p-3 rounded-xl border border-slate-800 bg-slate-950/30 flex items-center justify-between gap-3">
                        <div class="min-w-0 flex-1">
                            <div class="flex items-center gap-2">
                                <span class="font-mono text-xs text-white">{policy.secret_key}</span>
                                <span class="text-[9px] text-slate-600">→</span>
                                <span class="text-xs text-sky-400">{policy.consumer}</span>
                                {#if policy.allowed}
                                    <span class="text-[8px] px-1.5 py-0.5 rounded-full bg-emerald-500/10 text-emerald-400 font-bold uppercase">allow</span>
                                {:else}
                                    <span class="text-[8px] px-1.5 py-0.5 rounded-full bg-red-500/10 text-red-400 font-bold uppercase">deny</span>
                                {/if}
                                {#if policy.require_approval}
                                    <span class="text-[8px] px-1.5 py-0.5 rounded-full bg-amber-500/10 text-amber-400 font-bold uppercase">hitl</span>
                                {/if}
                            </div>
                            <div class="flex gap-3 mt-1">
                                {#if policy.max_ttl_seconds}
                                    <span class="text-[9px] text-slate-600">TTL: {formatTtl(policy.max_ttl_seconds)}</span>
                                {/if}
                                {#if policy.scopes.length > 0}
                                    <span class="text-[9px] text-slate-600">Scopes: {policy.scopes.join(', ')}</span>
                                {/if}
                                {#if policy.expires_at}
                                    <span class="text-[9px] text-slate-600">Expires: {new Date(policy.expires_at).toLocaleDateString()}</span>
                                {/if}
                            </div>
                        </div>
                        <button
                            on:click={() => handleDelete(policy.id)}
                            class="px-3 py-1 rounded-lg bg-red-500/10 hover:bg-red-500/20 text-red-400 text-[10px] font-bold transition-all"
                        >
                            Delete
                        </button>
                    </div>
                {/each}
            {/if}
        </div>
    {/if}
</div>
