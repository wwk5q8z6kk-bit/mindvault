<script lang="ts">
    import { chronicles } from '$lib/api/agent';
    import { fade, slide } from 'svelte/transition';
    import { flip } from 'svelte/animate';

    $: steps = $chronicles;

    function formatTime(ts: string) {
        return new Date(ts).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
    }
</script>

<div class="flex flex-col h-full bg-slate-950/50 backdrop-blur-md border-l border-slate-800">
    <div class="flex items-center justify-between p-4 border-b border-slate-800 bg-slate-900/40">
        <div class="flex items-center gap-2">
            <div class="h-2 w-2 rounded-full bg-amber-400 animate-pulse"></div>
            <h3 class="text-xs font-bold uppercase tracking-tighter text-amber-500">The Chronicle</h3>
        </div>
        <span class="text-[10px] text-slate-500 font-mono">Live Reasoning logs</span>
    </div>

    <div class="flex-1 overflow-y-auto p-4 space-y-4 custom-scrollbar">
        {#if steps.length === 0}
            <div class="flex flex-col items-center justify-center h-full text-center p-8 opacity-40">
                <div class="w-12 h-12 mb-4 border-2 border-dashed border-slate-700 rounded-full"></div>
                <p class="text-xs text-slate-400">Waiting for agent activity...</p>
            </div>
        {:else}
            {#each steps as step (step.id)}
                <div 
                    animate:flip={{ duration: 300 }}
                    transition:slide
                    class="group relative pl-4 border-l-2 border-slate-800 hover:border-amber-500/50 transition-colors py-1"
                >
                    <div class="flex items-center justify-between mb-1">
                        <span class="text-[10px] font-bold text-amber-400/80 uppercase tracking-widest">{step.step_name}</span>
                        <span class="text-[9px] font-mono text-slate-600 group-hover:text-slate-400 transition-colors">{formatTime(step.timestamp)}</span>
                    </div>
                    
                    <p class="text-xs text-slate-300 leading-relaxed">
                        {step.logic}
                    </p>

                    {#if step.node_id}
                        <div class="mt-2 flex flex-wrap gap-2">
                            <a
                                href={`/notes?note=${step.node_id}`}
                                class="text-[9px] bg-slate-900 border border-slate-800 px-1.5 py-0.5 rounded text-amber-200/60 hover:text-amber-300 hover:border-amber-500/30 transition-all"
                            >
                                Ref: {step.node_id.slice(0, 8)}
                            </a>
                        </div>
                    {/if}
                </div>
            {/each}
        {/if}
    </div>
</div>

<style>
    .custom-scrollbar::-webkit-scrollbar {
        width: 4px;
    }
    .custom-scrollbar::-webkit-scrollbar-track {
        background: transparent;
    }
    .custom-scrollbar::-webkit-scrollbar-thumb {
        background: #1e293b;
        border-radius: 2px;
    }
    .custom-scrollbar::-webkit-scrollbar-thumb:hover {
        background: #334155;
    }
</style>
