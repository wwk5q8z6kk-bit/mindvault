<script lang="ts">
	import { toastStore } from '$lib/stores/toast';
	import { popUndo } from '$lib/stores/undo';

	const variants: Record<string, string> = {
		info: 'bg-slate-800 text-slate-100 border-slate-700',
		success: 'bg-emerald-500/20 text-emerald-200 border-emerald-500/40',
		warning: 'bg-amber-500/20 text-amber-200 border-amber-500/40',
		danger: 'bg-rose-500/20 text-rose-200 border-rose-500/40'
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
		<div
			class={`flex items-center justify-between rounded-xl border px-4 py-3 text-sm shadow-lg backdrop-blur ${
				variants[toast.variant ?? 'info']
			}`}
		>
			<span>{toast.message}</span>
			{#if toast.undoId}
				<button
					class="ml-3 shrink-0 rounded-md px-2 py-0.5 text-xs font-semibold text-white hover:bg-white/10"
					on:click={() => handleUndo(toast.undoId ?? '', toast.id)}
				>
					Undo
				</button>
			{/if}
		</div>
	{/each}
</div>
