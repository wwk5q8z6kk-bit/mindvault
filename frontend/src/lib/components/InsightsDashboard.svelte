<script lang="ts">
    import { insights, fetchInsights, generateInsights } from '$lib/api/agent';
    import { onMount } from 'svelte';
    import { fade, slide } from 'svelte/transition';
    import { pushToast } from '$lib/stores/toast';

    let loading = false;

    onMount(async () => {
        try {
            await fetchInsights();
        } catch {
            // Non-blocking widget; banner/toasts elsewhere handle backend visibility.
        }
    });

    async function handleRefresh() {
        loading = true;
        try {
            await generateInsights();
            pushToast('Deep analysis complete!', 'success');
        } catch (e) {
            pushToast('Analysis failed', 'danger');
        } finally {
            loading = false;
        }
    }

    function getInsightColor(type: string) {
        switch (type) {
            case 'connection': return 'text-cyan-400 bg-cyan-500/10 border-cyan-500/20';
            case 'trend': return 'text-emerald-400 bg-emerald-500/10 border-emerald-500/20';
            case 'gap': return 'text-amber-400 bg-amber-500/10 border-amber-500/20';
            default: return 'text-indigo-400 bg-indigo-500/10 border-indigo-500/20';
        }
    }
</script>

<div class="rounded-2xl border border-slate-800 bg-slate-900/50 p-5 shadow-xl transition-all hover:border-slate-700">
    <div class="flex items-center justify-between mb-6">
        <div>
            <h3 class="text-sm font-bold text-white mb-1">Proactive Intelligence</h3>
            <p class="text-[10px] text-slate-500">Autonomous discovery of patterns and gaps.</p>
        </div>
        <button
            on:click={handleRefresh}
            disabled={loading}
            title="Refresh insights"
            class="p-2 rounded-xl bg-slate-800 hover:bg-slate-700 text-slate-400 hover:text-white transition-all disabled:opacity-50"
        >
            <svg class="w-4 h-4 {loading ? 'animate-spin' : ''}" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
            </svg>
        </button>
    </div>

    <div class="space-y-3">
        {#if $insights.length === 0}
            <div class="py-8 text-center bg-slate-950/30 rounded-xl border border-dashed border-slate-800">
                <p class="text-[10px] text-slate-500 uppercase tracking-widest">No insights discovered yet</p>
                <button 
                    on:click={handleRefresh}
                    class="mt-3 text-[10px] font-bold text-sky-400 hover:text-sky-300 underline underline-offset-4"
                >
                    Run deep analysis
                </button>
            </div>
        {:else}
            {#each $insights.slice(0, 3) as insight (insight.id)}
                <div transition:slide class="p-3 rounded-xl border {getInsightColor(insight.insight_type)}">
                    <div class="flex items-center justify-between mb-1.6">
                        <span class="text-[9px] font-black uppercase tracking-widest">{insight.insight_type}</span>
                        <div class="flex gap-0.5">
                            {#each Array(Math.round(insight.importance * 3)) as _}
                                <div class="w-1 h-1 rounded-full bg-current opacity-60"></div>
                            {/each}
                        </div>
                    </div>
                    <h4 class="text-xs font-bold text-white mb-1">{insight.title}</h4>
                    <p class="text-[10px] text-slate-400 leading-relaxed line-clamp-2">
                        {insight.content}
                    </p>
                </div>
            {/each}
        {/if}
    </div>

    {#if $insights.length > 3}
        <button class="w-full mt-4 py-2 text-[10px] font-bold text-slate-500 hover:text-white transition-colors uppercase tracking-widest">
            View All Insights ({$insights.length})
        </button>
    {/if}
</div>
