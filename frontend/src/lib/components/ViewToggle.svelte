<script lang="ts">
	import { createEventDispatcher } from 'svelte';

	export let views: {
		key: string;
		label: string;
		icon?: string;
		count?: number;
		disabled?: boolean;
	}[] = [];
	export let activeView = '';
	export let size: 'sm' | 'md' = 'md';
	export let variant: 'pills' | 'tabs' = 'pills';

	const dispatch = createEventDispatcher<{ change: string }>();

	function handleSelect(key: string, disabled?: boolean) {
		if (disabled) return;
		dispatch('change', key);
	}

	function handleKeydown(event: KeyboardEvent, key: string, disabled?: boolean) {
		if (event.key === 'Enter' || event.key === ' ') {
			event.preventDefault();
			handleSelect(key, disabled);
		}
	}
</script>

<div
	class={`inline-flex ${variant === 'pills' ? 'gap-1 rounded-xl bg-[rgb(var(--mv-panel-strong))]/60 p-1' : 'gap-0 border-b border-[rgb(var(--mv-border))]'}`}
	role="tablist"
	aria-label="View mode"
>
	{#each views as view (view.key)}
		{@const isActive = view.key === activeView}
		<button
			role="tab"
			aria-selected={isActive}
			aria-disabled={view.disabled ?? false}
			tabindex={isActive ? 0 : -1}
			class={`
				inline-flex items-center gap-1.5 font-medium transition-all duration-200 select-none
				${size === 'sm' ? 'px-2.5 py-1 text-xs' : 'px-3.5 py-1.5 text-sm'}
				${view.disabled ? 'opacity-40 cursor-not-allowed' : 'cursor-pointer'}
				${variant === 'pills'
					? isActive
						? 'rounded-lg bg-[rgb(var(--mv-accent))]/15 text-[rgb(var(--mv-accent))] shadow-sm'
						: 'rounded-lg text-[rgb(var(--mv-muted))] hover:text-[rgb(var(--mv-text))] hover:bg-[rgb(var(--mv-panel-strong))]/80'
					: isActive
						? '-mb-px border-b-2 border-[rgb(var(--mv-accent))] pb-2 text-[rgb(var(--mv-accent))]'
						: 'pb-2 text-[rgb(var(--mv-muted))] hover:text-[rgb(var(--mv-text))]'
				}
			`}
			on:click={() => handleSelect(view.key, view.disabled)}
			on:keydown={(e) => handleKeydown(e, view.key, view.disabled)}
		>
			{#if view.icon}
				<svg class={size === 'sm' ? 'h-3.5 w-3.5' : 'h-4 w-4'} fill="none" stroke="currentColor" viewBox="0 0 24 24">
					{@html view.icon}
				</svg>
			{/if}
			{view.label}
			{#if view.count != null}
				<span
					class={`rounded-full font-semibold ${
						isActive
							? 'bg-[rgb(var(--mv-accent))]/20 text-[rgb(var(--mv-accent))]'
							: 'bg-[rgb(var(--mv-panel-strong))] text-[rgb(var(--mv-muted))]'
					} ${size === 'sm' ? 'px-1.5 py-0.5 text-[10px]' : 'px-2 py-0.5 text-xs'}`}
				>
					{view.count}
				</span>
			{/if}
		</button>
	{/each}
</div>
