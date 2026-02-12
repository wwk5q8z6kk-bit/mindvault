<script lang="ts">
	import { createEventDispatcher, onMount } from 'svelte';
	import { fade, fly } from 'svelte/transition';

	export let open = false;
	export let title = '';
	export let size: 'sm' | 'md' | 'lg' = 'md';

	const dispatch = createEventDispatcher<{ close: void }>();

	let dialog: HTMLDivElement;

	const sizes: Record<string, string> = {
		sm: 'max-w-sm',
		md: 'max-w-lg',
		lg: 'max-w-2xl'
	};

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape' && open) {
			dispatch('close');
		}
	}

	function handleBackdropClick(event: MouseEvent) {
		if (event.target === event.currentTarget) {
			dispatch('close');
		}
	}

	$: if (open && dialog) {
		const focusable = dialog.querySelector<HTMLElement>(
			'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
		);
		focusable?.focus();
	}
</script>

<svelte:window on:keydown={handleKeydown} />

{#if open}
	<div
		class="fixed inset-0 z-50 flex items-center justify-center p-4"
		role="dialog"
		aria-modal="true"
		aria-label={title}
		on:click={handleBackdropClick}
		transition:fade={{ duration: 150 }}
	>
		<div class="absolute inset-0 bg-black/60 backdrop-blur-sm" aria-hidden="true"></div>
		<div
			bind:this={dialog}
			class="relative w-full {sizes[size]} rounded-2xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))] p-6 shadow-2xl"
			in:fly={{ y: 16, duration: 200 }}
			out:fade={{ duration: 100 }}
		>
			{#if title}
				<h2 class="mb-4 text-lg font-semibold text-[rgb(var(--mv-text))]">{title}</h2>
			{/if}
			<slot />
		</div>
	</div>
{/if}
