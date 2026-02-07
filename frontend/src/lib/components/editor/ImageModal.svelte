<script lang="ts">
	import { createEventDispatcher } from 'svelte';

	/** Pre-filled URL */
	export let url = '';
	/** Pre-filled alt text */
	export let alt = '';

	const dispatch = createEventDispatcher<{
		submit: { url: string; alt: string };
		cancel: void;
	}>();

	function handleSubmit() {
		if (url) {
			dispatch('submit', { url, alt });
		}
	}

	function handleCancel() {
		dispatch('cancel');
	}

	function handleKeyDown(event: KeyboardEvent) {
		if (event.key === 'Enter') {
			handleSubmit();
		} else if (event.key === 'Escape') {
			handleCancel();
		}
	}
</script>

<div
	class="fixed inset-0 z-50 flex items-center justify-center bg-black/60"
	on:click|self={handleCancel}
	on:keydown={handleKeyDown}
	role="dialog"
	aria-modal="true"
	aria-labelledby="image-modal-title" tabindex="0"
>
	<div class="w-96 rounded-xl border border-slate-700 bg-slate-900 p-5 shadow-2xl">
		<h3 id="image-modal-title" class="mb-3 text-sm font-semibold text-white">Insert Image</h3>
		<div class="flex flex-col gap-3">
			<input
				class="rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white outline-none focus:border-sky-500"
				placeholder="Image URL"
				bind:value={url}
								on:keydown={handleKeyDown}
			/>
			<input
				class="rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white outline-none focus:border-sky-500"
				placeholder="Alt text (optional)"
				bind:value={alt}
				on:keydown={handleKeyDown}
			/>
			{#if url}
				<div class="rounded-lg border border-slate-700 bg-slate-800 p-2">
					<img
						src={url}
						alt={alt || 'Preview'}
						class="max-h-32 w-full rounded object-contain"
						on:error={(e) => {
							const img = e.currentTarget as HTMLImageElement;
							img.style.display = 'none';
						}}
					/>
				</div>
			{/if}
			<div class="flex justify-end gap-2">
				<button
					class="rounded-lg px-3 py-2 text-xs text-slate-400 hover:bg-slate-800 hover:text-white"
					on:click={handleCancel}
				>
					Cancel
				</button>
				<button
					class="rounded-lg bg-sky-500 px-3 py-2 text-xs font-semibold text-white hover:bg-sky-400 disabled:opacity-40"
					disabled={!url}
					on:click={handleSubmit}
				>
					Insert
				</button>
			</div>
		</div>
	</div>
</div>
