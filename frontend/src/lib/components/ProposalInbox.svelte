<script lang="ts">
    import { intents, insights, applyIntent, dismissIntent, fetchIntents, fetchInsights } from '$lib/api/agent';
    import { approveProposal, listProposals, rejectProposal, type Proposal } from '$lib/api/exchange';
    import { pushToast } from '$lib/stores/toast';
    import { fade, slide, fly } from 'svelte/transition';
    import { flip } from 'svelte/animate';
    import { onMount } from 'svelte';

    let isOpen = false;
    let activeTab: 'intents' | 'insights' | 'proposals' = 'intents';
    let proposals: Proposal[] = [];
    let proposalsLoading = false;

    $: intentList = $intents.filter(i => i.status === 'suggested');
    $: insightList = $insights.filter(i => !i.dismissed_at);
    $: proposalList = proposals.filter(p => p.state === 'pending');
    $: totalCount = intentList.length + insightList.length + proposalList.length;

    onMount(() => {
        fetchIntents();
        fetchInsights();
        fetchProposalInbox();
    });

    async function handleApply(id: string) {
        try {
            await applyIntent(id);
            pushToast('Action applied!', 'success');
        } catch (e) {
            pushToast('Failed to apply action', 'danger');
        }
    }

    async function handleDismiss(id: string) {
        try {
            await dismissIntent(id);
        } catch (e) {
            pushToast('Failed to dismiss', 'danger');
        }
    }

    async function fetchProposalInbox() {
        proposalsLoading = true;
        try {
            proposals = await listProposals('pending');
        } catch (e) {
            pushToast('Failed to load proposals', 'danger');
        } finally {
            proposalsLoading = false;
        }
    }

    async function handleApprove(id: string) {
        try {
            await approveProposal(id);
            proposals = proposals.filter(p => p.id !== id);
            pushToast('Proposal approved', 'success');
        } catch (e) {
            pushToast('Failed to approve proposal', 'danger');
        }
    }

    async function handleReject(id: string) {
        try {
            await rejectProposal(id);
            proposals = proposals.filter(p => p.id !== id);
            pushToast('Proposal rejected', 'info');
        } catch (e) {
            pushToast('Failed to reject proposal', 'danger');
        }
    }

    function getIntentIcon(type: string) {
        switch (type) {
            case 'schedule_reminder': return '⏰';
            case 'extract_task': return '✅';
            case 'link_to_project': return '🔗';
            case 'suggest_link': return '🔗';
            case 'suggest_tag': return '🏷️';
            default: return '✨';
        }
    }

    function getInsightIcon(type: string) {
        switch (type) {
            case 'connection': return '🔗';
            case 'trend': return '📈';
            case 'gap': return '🕳️';
            case 'stale': return '⏳';
            case 'reminder': return '⏰';
            case 'cluster': return '📊';
            default: return '💡';
        }
    }

    function getIntentLabel(type: string) {
        return type.replace(/_/g, ' ');
    }

    function getProposalLabel(action: string) {
        return action.replace(/_/g, ' ');
    }

    function getProposalPreview(proposal: Proposal) {
        if (proposal.diff_preview) return proposal.diff_preview;
        const payload = proposal.payload ?? {};
        if (typeof payload.title === 'string') return payload.title;
        if (typeof payload.content === 'string') return payload.content;
        const raw = JSON.stringify(payload);
        return raw.length > 140 ? `${raw.slice(0, 140)}…` : raw;
    }

    function formatConfidence(confidence: number) {
        return Math.round(confidence * 100);
    }
</script>

<!-- Floating button -->
<div class="fixed bottom-6 right-6 z-50">
    {#if !isOpen}
        <button
            on:click={() => isOpen = true}
            class="relative flex items-center justify-center w-14 h-14 bg-violet-600 hover:bg-violet-500 rounded-full shadow-xl shadow-violet-900/30 transition-all hover:scale-105 active:scale-95"
            transition:fly={{ y: 20, duration: 200 }}
        >
            <svg class="w-6 h-6 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
            </svg>
            {#if totalCount > 0}
                <span class="absolute -top-1 -right-1 flex items-center justify-center w-5 h-5 bg-red-500 text-white text-[10px] font-bold rounded-full">
                    {totalCount > 9 ? '9+' : totalCount}
                </span>
            {/if}
        </button>
    {/if}
</div>

<!-- Inbox panel -->
{#if isOpen}
    <div
        class="fixed bottom-6 right-6 z-50 w-96 max-h-[70vh] bg-slate-950 border border-slate-800 rounded-2xl shadow-2xl overflow-hidden flex flex-col"
        transition:fly={{ y: 20, duration: 200 }}
    >
        <!-- Header -->
        <div class="flex items-center justify-between px-4 py-3 border-b border-slate-800 bg-slate-900/50">
            <div class="flex items-center gap-2">
                <div class="h-2 w-2 rounded-full bg-violet-500 animate-pulse"></div>
                <h3 class="text-sm font-bold text-white">Proposal Inbox</h3>
                {#if totalCount > 0}
                    <span class="px-2 py-0.5 bg-violet-500/20 text-violet-400 text-[10px] font-semibold rounded-full">
                        {totalCount}
                    </span>
                {/if}
            </div>
            <button
                on:click={() => isOpen = false}
                title="Close"
                class="p-1.5 text-slate-500 hover:text-white hover:bg-slate-800 rounded-lg transition-colors"
            >
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                </svg>
            </button>
        </div>

        <!-- Tabs -->
        <div class="flex border-b border-slate-800">
            <button
                on:click={() => activeTab = 'intents'}
                class="flex-1 px-4 py-2 text-xs font-semibold transition-colors {activeTab === 'intents' ? 'text-violet-400 border-b-2 border-violet-500' : 'text-slate-500 hover:text-slate-300'}"
            >
                Actions
                {#if intentList.length > 0}
                    <span class="ml-1 text-[10px]">({intentList.length})</span>
                {/if}
            </button>
            <button
                on:click={() => activeTab = 'insights'}
                class="flex-1 px-4 py-2 text-xs font-semibold transition-colors {activeTab === 'insights' ? 'text-amber-400 border-b-2 border-amber-500' : 'text-slate-500 hover:text-slate-300'}"
            >
                Insights
                {#if insightList.length > 0}
                    <span class="ml-1 text-[10px]">({insightList.length})</span>
                {/if}
            </button>
            <button
                on:click={() => activeTab = 'proposals'}
                class="flex-1 px-4 py-2 text-xs font-semibold transition-colors {activeTab === 'proposals' ? 'text-emerald-400 border-b-2 border-emerald-500' : 'text-slate-500 hover:text-slate-300'}"
            >
                Proposals
                {#if proposalList.length > 0}
                    <span class="ml-1 text-[10px]">({proposalList.length})</span>
                {/if}
            </button>
        </div>

        <!-- Content -->
        <div class="flex-1 overflow-y-auto p-3 space-y-2">
            {#if activeTab === 'intents'}
                {#if intentList.length === 0}
                    <div class="flex flex-col items-center justify-center py-8 text-center">
                        <div class="w-12 h-12 mb-3 border-2 border-dashed border-slate-700 rounded-full flex items-center justify-center">
                            <span class="text-xl opacity-50">✨</span>
                        </div>
                        <p class="text-xs text-slate-500">No pending actions</p>
                    </div>
                {:else}
                    {#each intentList as intent (intent.id)}
                        <div
                            animate:flip={{ duration: 200 }}
                            transition:slide={{ duration: 200 }}
                            class="p-3 rounded-xl border border-violet-500/20 bg-violet-500/5 hover:bg-violet-500/10 transition-all"
                        >
                            <div class="flex items-start gap-3">
                                <span class="text-lg">{getIntentIcon(intent.intent_type)}</span>
                                <div class="flex-1 min-w-0">
                                    <div class="flex items-center gap-2">
                                        <span class="text-[10px] font-bold text-violet-400 uppercase tracking-wider">
                                            {getIntentLabel(intent.intent_type)}
                                        </span>
                                        <span class="text-[9px] text-slate-500 font-mono">
                                            {formatConfidence(intent.confidence)}%
                                        </span>
                                    </div>
                                    <p class="text-xs text-slate-400 mt-0.5">
                                        Detected in note
                                    </p>
                                </div>
                            </div>
                            <div class="flex items-center justify-end gap-2 mt-3">
                                <button
                                    on:click={() => handleDismiss(intent.id)}
                                    class="px-2 py-1 text-[10px] text-slate-400 hover:text-white hover:bg-slate-800 rounded transition-colors"
                                >
                                    Dismiss
                                </button>
                                <button
                                    on:click={() => handleApply(intent.id)}
                                    class="px-3 py-1 text-[10px] font-bold text-white bg-violet-600 hover:bg-violet-500 rounded shadow-lg transition-all active:scale-95"
                                >
                                    Apply
                                </button>
                            </div>
                        </div>
                    {/each}
                {/if}
            {:else if activeTab === 'insights'}
                {#if insightList.length === 0}
                    <div class="flex flex-col items-center justify-center py-8 text-center">
                        <div class="w-12 h-12 mb-3 border-2 border-dashed border-slate-700 rounded-full flex items-center justify-center">
                            <span class="text-xl opacity-50">💡</span>
                        </div>
                        <p class="text-xs text-slate-500">No insights yet</p>
                    </div>
                {:else}
                    {#each insightList as insight (insight.id)}
                        <div
                            animate:flip={{ duration: 200 }}
                            transition:slide={{ duration: 200 }}
                            class="p-3 rounded-xl border border-amber-500/20 bg-amber-500/5 hover:bg-amber-500/10 transition-all"
                        >
                            <div class="flex items-start gap-3">
                                <span class="text-lg">{getInsightIcon(insight.insight_type)}</span>
                                <div class="flex-1 min-w-0">
                                    <h4 class="text-xs font-semibold text-amber-400">{insight.title}</h4>
                                    <p class="text-[11px] text-slate-400 mt-1 line-clamp-2">{insight.content}</p>
                                    {#if insight.related_node_ids.length > 0}
                                        <div class="flex items-center gap-1 mt-2">
                                            <span class="text-[9px] text-slate-500">Related:</span>
                                            <span class="text-[9px] text-amber-500/70">{insight.related_node_ids.length} notes</span>
                                        </div>
                                    {/if}
                                </div>
                            </div>
                        </div>
                    {/each}
                {/if}
            {:else}
                {#if proposalsLoading}
                    <div class="flex flex-col items-center justify-center py-8 text-center">
                        <div class="w-12 h-12 mb-3 border-2 border-dashed border-slate-700 rounded-full flex items-center justify-center">
                            <span class="text-xl opacity-50">⏳</span>
                        </div>
                        <p class="text-xs text-slate-500">Loading proposals...</p>
                    </div>
                {:else if proposalList.length === 0}
                    <div class="flex flex-col items-center justify-center py-8 text-center">
                        <div class="w-12 h-12 mb-3 border-2 border-dashed border-slate-700 rounded-full flex items-center justify-center">
                            <span class="text-xl opacity-50">📮</span>
                        </div>
                        <p class="text-xs text-slate-500">No pending proposals</p>
                    </div>
                {:else}
                    {#each proposalList as proposal (proposal.id)}
                        <div
                            animate:flip={{ duration: 200 }}
                            transition:slide={{ duration: 200 }}
                            class="p-3 rounded-xl border border-emerald-500/20 bg-emerald-500/5 hover:bg-emerald-500/10 transition-all"
                        >
                            <div class="flex items-start gap-3">
                                <span class="text-lg">📬</span>
                                <div class="flex-1 min-w-0">
                                    <div class="flex items-center gap-2">
                                        <span class="text-[10px] font-bold text-emerald-400 uppercase tracking-wider">
                                            {getProposalLabel(proposal.action)}
                                        </span>
                                        <span class="text-[9px] text-slate-500 font-mono">
                                            {formatConfidence(proposal.confidence)}%
                                        </span>
                                    </div>
                                    <p class="text-[11px] text-slate-400 mt-1 line-clamp-2">
                                        {getProposalPreview(proposal)}
                                    </p>
                                    <div class="flex items-center gap-2 mt-2">
                                        <span class="text-[9px] text-slate-500 uppercase tracking-wider">Sender</span>
                                        <span class="text-[9px] text-emerald-400">{proposal.sender}</span>
                                    </div>
                                </div>
                            </div>
                            <div class="flex items-center justify-end gap-2 mt-3">
                                <button
                                    on:click={() => handleReject(proposal.id)}
                                    class="px-2 py-1 text-[10px] text-slate-400 hover:text-white hover:bg-slate-800 rounded transition-colors"
                                >
                                    Reject
                                </button>
                                <button
                                    on:click={() => handleApprove(proposal.id)}
                                    class="px-3 py-1 text-[10px] font-bold text-white bg-emerald-600 hover:bg-emerald-500 rounded shadow-lg transition-all active:scale-95"
                                >
                                    Approve
                                </button>
                            </div>
                        </div>
                    {/each}
                {/if}
            {/if}
        </div>
    </div>
{/if}

<style>
    .line-clamp-2 {
        display: -webkit-box;
        -webkit-line-clamp: 2;
        line-clamp: 2;
        -webkit-box-orient: vertical;
        overflow: hidden;
    }
</style>
