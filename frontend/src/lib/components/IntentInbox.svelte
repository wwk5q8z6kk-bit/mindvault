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
			case 'schedule_reminder':
				return '⏰';
			case 'extract_task':
				return '✅';
			case 'link_to_project':
				return '🔗';
			case 'suggest_link':
				return '🔗';
			case 'suggest_tag':
				return '🏷️';
			default:
				return '✨';
		}
	}

	function getIntentLabel(type: string) {
		return type.replace(/_/g, ' ').toUpperCase();
	}
</script>

<div class="space-y-4">
	{#if suggestionList.length === 0}
		<div
			class="p-8 text-center border-2 border-dashed border-white/10 rounded-[var(--mv-radius)] bg-[rgb(var(--mv-panel))]/20 backdrop-blur-xl"
		>
			<p class="text-sm font-medium text-[rgb(var(--mv-muted))]/70">
				No autonomous suggestions at the moment.
			</p>
		</div>
	{:else}
		{#each suggestionList as intent (intent.id)}
			<div
				animate:flip={{ duration: 400 }}
				transition:slide
				class="group relative overflow-hidden rounded-[var(--mv-radius)] border border-violet-500/20 bg-[rgb(var(--mv-panel))]/40 backdrop-blur-2xl hover:bg-[rgb(var(--mv-panel))]/60 hover:border-violet-500/40 transition-all duration-300 shadow-lg hover:shadow-xl hover:-translate-y-0.5"
			>
				<div
					class="absolute -left-10 -bottom-10 h-24 w-24 rounded-full bg-violet-500/10 blur-2xl pointer-events-none transition-opacity group-hover:bg-violet-500/20"
				></div>

				<div class="p-5 relative z-10">
					<div class="flex items-start justify-between gap-4 mb-3">
						<div class="flex items-center gap-3">
							<div
								class="flex h-10 w-10 items-center justify-center rounded-xl bg-violet-500/10 border border-violet-500/20 shadow-inner"
							>
								<span class="text-xl drop-shadow-md">{getIntentIcon(intent.intent_type)}</span>
							</div>
							<div>
								<h4
									class="text-xs font-extrabold text-transparent bg-clip-text bg-gradient-to-r from-violet-400 to-fuchsia-400 tracking-widest"
								>
									{getIntentLabel(intent.intent_type)}
								</h4>
								<p class="text-sm font-medium text-[rgb(var(--mv-muted))] mt-0.5 leading-tight">
									High confidence suggestion detected.
								</p>
							</div>
						</div>
						<div class="flex flex-col items-end gap-1.5">
							<span
								class="text-[10px] uppercase font-bold text-[rgb(var(--mv-muted))] tracking-wider"
								>Confidence</span
							>
							<div class="flex items-center gap-2">
								<div class="h-1.5 w-16 bg-white/5 rounded-full overflow-hidden shadow-inner">
									<div
										class="h-full bg-gradient-to-r from-violet-500 to-fuchsia-500"
										style="width: {intent.confidence * 100}%"
									></div>
								</div>
								<span class="text-xs font-mono font-bold text-violet-300"
									>{Math.round(intent.confidence * 100)}%</span
								>
							</div>
						</div>
					</div>

					<div class="mt-5 flex items-center justify-end gap-3 border-t border-white/5 pt-4">
						<button
							on:click={() => handleDismiss(intent.id)}
							class="px-4 py-2 text-xs font-bold text-[rgb(var(--mv-muted))] hover:text-[rgb(var(--mv-text))] hover:bg-white/5 rounded-lg transition-colors duration-200"
						>
							Dismiss
						</button>
						<button
							on:click={() => handleApply(intent.id)}
							class="px-5 py-2 text-xs font-bold text-white bg-gradient-to-r from-violet-600 to-fuchsia-600 hover:from-violet-500 hover:to-fuchsia-500 rounded-lg shadow-lg shadow-violet-900/40 transition-all duration-200 active:scale-95 border border-white/10 focus:ring-2 focus:ring-violet-500/50"
						>
							Take Action
						</button>
					</div>
				</div>
			</div>
		{/each}
	{/if}
</div>
