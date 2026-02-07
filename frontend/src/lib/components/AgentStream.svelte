<script lang="ts">
    import { agentStore, connectAgentStream, disconnectAgentStream } from '$lib/api/agent';
    import { onMount } from 'svelte';
    import { slide } from 'svelte/transition';

    onMount(() => {
        connectAgentStream();
        return () => disconnectAgentStream();
    });

    $: ({ summary, relatedNodes } = $agentStore);
</script>

<div class="rounded-xl border border-sky-500/20 bg-sky-500/5 p-4 backdrop-blur-sm">
    <div class="flex items-center gap-2 mb-3">
        <div class="h-2 w-2 rounded-full bg-sky-400 animate-pulse"></div>
        <h3 class="text-xs font-bold uppercase tracking-wider text-sky-400">Agent Intelligence</h3>
    </div>
    
    <p class="text-xs text-slate-300 leading-relaxed mb-4">
        {summary}
    </p>

    {#if relatedNodes.length > 0}
        <div class="space-y-2">
            <h4 class="text-[10px] font-semibold text-slate-500 uppercase">Related Context</h4>
            {#each relatedNodes as node (node.id)}
                <div transition:slide class="group relative rounded-lg border border-slate-800/60 bg-slate-900/40 p-2 hover:border-sky-500/40 transition-colors">
                    <a href={`/notes?note=${node.id}`} class="block">
                        <div class="text-xs font-medium text-white group-hover:text-sky-300 truncate">
                            {node.title || 'Untitled'}
                        </div>
                        <div class="mt-1 text-[9px] text-slate-500">
                            Updated {new Date(node.updated_at).toLocaleDateString()}
                        </div>
                    </a>
                </div>
            {/each}
        </div>
    {/if}
</div>
