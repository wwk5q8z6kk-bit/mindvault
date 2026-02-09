<script lang="ts">
    import { onMount } from 'svelte';
    import { slide } from 'svelte/transition';
    import { pushToast } from '$lib/stores/toast';
    import {
        type ConsumerProfile,
        type CreateConsumerResponse,
        createConsumer,
        listConsumers,
        revokeConsumer
    } from '$lib/api/consumers';

    let profiles: ConsumerProfile[] = [];
    let loading = true;
    let newName = '';
    let newDescription = '';
    let createdToken: CreateConsumerResponse | null = null;
    let creating = false;

    onMount(async () => {
        await refresh();
    });

    async function refresh() {
        loading = true;
        try {
            profiles = await listConsumers();
        } catch (e) {
            pushToast('Failed to load consumer profiles', 'danger');
        } finally {
            loading = false;
        }
    }

    async function handleCreate() {
        if (!newName.trim()) return;
        creating = true;
        try {
            createdToken = await createConsumer(
                newName.trim(),
                newDescription.trim() || undefined
            );
            newName = '';
            newDescription = '';
            pushToast(`Profile "${createdToken.name}" created`, 'success');
            await refresh();
        } catch (e) {
            pushToast('Failed to create profile', 'danger');
        } finally {
            creating = false;
        }
    }

    async function handleRevoke(id: string, name: string) {
        try {
            await revokeConsumer(id);
            pushToast(`Profile "${name}" revoked`, 'success');
            await refresh();
        } catch (e) {
            pushToast('Failed to revoke profile', 'danger');
        }
    }

    function dismissToken() {
        createdToken = null;
    }

    function formatDate(iso: string | null | undefined): string {
        if (!iso) return 'never';
        return new Date(iso).toLocaleDateString('en-US', {
            month: 'short',
            day: 'numeric',
            hour: '2-digit',
            minute: '2-digit'
        });
    }
</script>

<div class="rounded-2xl border border-slate-800 bg-slate-900/50 p-5 shadow-xl">
    <div class="flex items-center justify-between mb-5">
        <div>
            <h3 class="text-sm font-bold text-white mb-1">Consumer Profiles</h3>
            <p class="text-[10px] text-slate-500">Named identities for AI agents and services.</p>
        </div>
        <button
            on:click={refresh}
            disabled={loading}
            title="Refresh"
            class="p-2 rounded-xl bg-slate-800 hover:bg-slate-700 text-slate-400 hover:text-white transition-all disabled:opacity-50"
        >
            <svg class="w-4 h-4 {loading ? 'animate-spin' : ''}" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
            </svg>
        </button>
    </div>

    <!-- Token display (one-time) -->
    {#if createdToken}
        <div transition:slide class="mb-4 p-4 rounded-xl border border-amber-500/30 bg-amber-500/5">
            <div class="flex items-center justify-between mb-2">
                <span class="text-[9px] font-black uppercase tracking-widest text-amber-400">One-time token — copy now</span>
                <button on:click={dismissToken} class="text-slate-500 hover:text-white text-xs">dismiss</button>
            </div>
            <p class="text-[10px] text-slate-400 mb-2">Profile: <strong class="text-white">{createdToken.name}</strong></p>
            <code class="block p-2 rounded bg-slate-950 text-[10px] text-amber-300 break-all select-all font-mono">
                {createdToken.token}
            </code>
            <p class="text-[9px] text-slate-500 mt-2">This token will not be shown again.</p>
        </div>
    {/if}

    <!-- Create form -->
    <div class="mb-5 p-3 rounded-xl bg-slate-950/40 border border-slate-800">
        <div class="flex gap-2 items-end">
            <div class="flex-1">
                <label class="block text-[9px] text-slate-500 uppercase tracking-widest mb-1" for="consumer-name">Name</label>
                <input
                    id="consumer-name"
                    bind:value={newName}
                    placeholder="e.g. openclaw"
                    class="w-full px-3 py-1.5 rounded-lg bg-slate-900 border border-slate-700 text-xs text-white placeholder:text-slate-600 focus:border-sky-500 focus:outline-none"
                />
            </div>
            <div class="flex-1">
                <label class="block text-[9px] text-slate-500 uppercase tracking-widest mb-1" for="consumer-desc">Description</label>
                <input
                    id="consumer-desc"
                    bind:value={newDescription}
                    placeholder="Optional description"
                    class="w-full px-3 py-1.5 rounded-lg bg-slate-900 border border-slate-700 text-xs text-white placeholder:text-slate-600 focus:border-sky-500 focus:outline-none"
                />
            </div>
            <button
                on:click={handleCreate}
                disabled={creating || !newName.trim()}
                class="px-4 py-1.5 rounded-lg bg-sky-600 hover:bg-sky-500 text-white text-xs font-bold disabled:opacity-40 transition-all whitespace-nowrap"
            >
                {creating ? 'Creating...' : 'Create'}
            </button>
        </div>
    </div>

    <!-- Profiles list -->
    <div class="space-y-2">
        {#if loading && profiles.length === 0}
            <div class="py-6 text-center">
                <p class="text-[10px] text-slate-500 animate-pulse">Loading profiles...</p>
            </div>
        {:else if profiles.length === 0}
            <div class="py-6 text-center bg-slate-950/30 rounded-xl border border-dashed border-slate-800">
                <p class="text-[10px] text-slate-500 uppercase tracking-widest">No consumer profiles yet</p>
            </div>
        {:else}
            {#each profiles as profile (profile.id)}
                <div transition:slide class="p-3 rounded-xl border border-slate-800 bg-slate-950/30 flex items-center justify-between gap-3">
                    <div class="min-w-0 flex-1">
                        <div class="flex items-center gap-2">
                            <span class="text-xs font-bold text-white truncate">{profile.name}</span>
                            {#if profile.revoked_at}
                                <span class="text-[8px] px-1.5 py-0.5 rounded-full bg-red-500/10 text-red-400 font-bold uppercase">revoked</span>
                            {:else}
                                <span class="text-[8px] px-1.5 py-0.5 rounded-full bg-emerald-500/10 text-emerald-400 font-bold uppercase">active</span>
                            {/if}
                        </div>
                        {#if profile.description}
                            <p class="text-[10px] text-slate-500 truncate mt-0.5">{profile.description}</p>
                        {/if}
                        <div class="flex gap-3 mt-1">
                            <span class="text-[9px] text-slate-600">Created {formatDate(profile.created_at)}</span>
                            <span class="text-[9px] text-slate-600">Last used {formatDate(profile.last_used_at)}</span>
                        </div>
                    </div>
                    {#if !profile.revoked_at}
                        <button
                            on:click={() => handleRevoke(profile.id, profile.name)}
                            class="px-3 py-1 rounded-lg bg-red-500/10 hover:bg-red-500/20 text-red-400 text-[10px] font-bold transition-all"
                        >
                            Revoke
                        </button>
                    {/if}
                </div>
            {/each}
        {/if}
    </div>
</div>
