<script lang="ts">
	import { toastStore } from '$lib/stores/toast';
	import { popUndo } from '$lib/stores/undo';

	const variants: Record<string, string> = {
		info: 'bg-slate-900 text-slate-100 border-slate-700',
		success: 'bg-emerald-50 text-emerald-900 border-emerald-300',
		warning: 'bg-amber-50 text-amber-900 border-amber-300',
		danger: 'bg-rose-50 text-rose-900 border-rose-300'
	};

	async function handleUndo(undoId: string, toastId: string) {
		const ok = await popUndo(undoId);
		if (ok) {
			toastStore.update((items) => items.filter((t) => t.id !== toastId));
		}
	}
</script>

<div class="fixed left-3 right-3 top-[calc(4.5rem+env(safe-area-inset-top))] z-50 flex flex-col gap-2 md:left-auto md:right-6 md:top-6 md:w-80">
	{#each $toastStore as toast (toast.id)}
		<div class={`flex items-center justify-between rounded-xl border px-4 py-3 text-sm shadow-lg ${variants[toast.variant ?? 'info']}`}>
			<span>{toast.message}</span>
			{#if toast.undoId}
				<button
					class="ml-3 shrink-0 rounded-md border border-current/25 px-2 py-0.5 text-xs font-semibold text-current hover:bg-black/5"
					on:click={() => handleUndo(toast.undoId ?? '', toast.id)}
				>
					Undo
				</button>
			{/if}
		</div>
	{/each}
</div>
