<script lang="ts">
    import { intents, applyIntent, dismissIntent } from '$lib/api/agent';
    import { pushToast } from '$lib/stores/toast';
    import { fade, slide } from 'svelte/transition';
    import { flip } from 'svelte/animate';

    $: suggestionList = $intents;

    async function handleApply(id: string) {
        try {
            await applyIntent(id);
            pushToast('Action applied successfully!', 'success');
        } catch (e) {
            pushToast('Failed to apply action', 'danger');
        }
    }

    async function handleDismiss(id: string) {
        try {
            await dismissIntent(id);
        } catch (e) {
            pushToast('Failed to dismiss suggestion', 'danger');
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

    function getIntentLabel(type: string) {
        return type.replace(/_/g, ' ').toUpperCase();
    }
</script>

<div class="space-y-3">
    {#if suggestionList.length === 0}
        <div class="p-6 text-center border-2 border-dashed border-slate-800 rounded-2xl opacity-50">
            <p class="text-xs text-slate-500">No autonomous suggestions at the moment.</p>
        </div>
    {:else}
        {#each suggestionList as intent (intent.id)}
            <div 
                animate:flip={{ duration: 300 }}
                transition:slide
                class="group overflow-hidden rounded-2xl border border-violet-500/20 bg-violet-500/5 hover:bg-violet-500/10 transition-all duration-300"
            >
                <div class="p-4">
                    <div class="flex items-start justify-between gap-3 mb-2">
                        <div class="flex items-center gap-2">
                            <span class="text-lg">{getIntentIcon(intent.intent_type)}</span>
                            <div>
                                <h4 class="text-[10px] font-bold text-violet-400 tracking-widest">{getIntentLabel(intent.intent_type)}</h4>
                                <p class="text-xs text-slate-300 font-medium">
                                    High confidence suggestion detected.
                                </p>
                            </div>
                        </div>
                        <div class="flex items-center gap-1">
                            <div class="h-1 w-12 bg-slate-800 rounded-full overflow-hidden">
                                <div class="h-full bg-violet-500" style="width: {intent.confidence * 100}%"></div>
                            </div>
                            <span class="text-[9px] text-slate-500 font-mono">{Math.round(intent.confidence * 100)}%</span>
                        </div>
                    </div>

                    <div class="mt-4 flex items-center justify-end gap-2">
                        <button 
                            on:click={() => handleDismiss(intent.id)}
                            class="px-3 py-1.5 text-[10px] font-semibold text-slate-400 hover:text-white hover:bg-slate-800 rounded-lg transition-colors"
                        >
                            Dismiss
                        </button>
                        <button 
                            on:click={() => handleApply(intent.id)}
                            class="px-3 py-1.5 text-[10px] font-bold text-white bg-violet-600 hover:bg-violet-500 rounded-lg shadow-lg shadow-violet-900/20 transition-all active:scale-95"
                        >
                            Take Action
                        </button>
                    </div>
                </div>
            </div>
        {/each}
    {/if}
</div>
