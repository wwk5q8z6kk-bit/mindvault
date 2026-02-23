<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { crossfade } from 'svelte/transition';
	import { quintOut } from 'svelte/easing';

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

	const [send, receive] = crossfade({
		duration: 350,
		easing: quintOut
	});

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
	class={`inline-flex relative ${variant === 'pills' ? 'gap-1 rounded-xl bg-[rgb(var(--mv-panel-strong))]/70 p-1 shadow-inner ring-1 ring-inset ring-black/10 backdrop-blur-md' : 'gap-0 border-b border-[rgb(var(--mv-border))]'}`}
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
				relative inline-flex items-center justify-center gap-2 font-medium transition-colors duration-300 select-none z-10
				${size === 'sm' ? 'px-3 py-1.5 text-xs' : 'px-4 py-2 text-sm'}
				${view.disabled ? 'opacity-40 cursor-not-allowed' : 'cursor-pointer'}
				${variant === 'pills'
					? isActive
						? 'text-[rgb(var(--mv-text))] drop-shadow-sm'
						: 'text-[rgb(var(--mv-muted))] hover:text-[rgb(var(--mv-text))] hover:bg-white/5 rounded-lg'
					: isActive
						? '-mb-px border-b-2 border-[rgb(var(--mv-accent))] pb-2 text-[rgb(var(--mv-accent))]'
						: 'pb-2 text-[rgb(var(--mv-muted))] hover:text-[rgb(var(--mv-text))]'
				}
			`}
			on:click={() => handleSelect(view.key, view.disabled)}
			on:keydown={(e) => handleKeydown(e, view.key, view.disabled)}
		>
			{#if variant === 'pills' && isActive}
				<div
					class="absolute inset-0 z-[-1] rounded-lg bg-[rgb(var(--mv-panel))]/90 shadow-[0_2px_8px_rgba(0,0,0,0.15)] border border-white/10 ring-1 ring-[rgb(var(--mv-accent))]/10"
					in:receive={{ key: 'active-pill' }}
					out:send={{ key: 'active-pill' }}
				></div>
			{/if}
			{#if view.icon}
				<svg class={`transition-transform duration-300 ${isActive ? 'scale-110 text-[rgb(var(--mv-accent))]' : 'opacity-80'} ${size === 'sm' ? 'h-3.5 w-3.5' : 'h-4 w-4'}`} fill="none" stroke="currentColor" viewBox="0 0 24 24">
					{@html view.icon}
				</svg>
			{/if}
			<span class="relative z-10 tracking-wide">{view.label}</span>
			{#if view.count != null}
				<span
					class={`relative z-10 rounded-full font-semibold transition-colors duration-300 ${
						isActive
							? 'bg-[rgb(var(--mv-accent))]/20 text-[rgb(var(--mv-accent))] ring-1 ring-[rgb(var(--mv-accent))]/30'
							: 'bg-[rgb(var(--mv-panel))]/50 text-[rgb(var(--mv-muted))] ring-1 ring-white/5'
					} ${size === 'sm' ? 'px-1.5 py-0.5 text-[10px]' : 'px-2 py-0.5 text-xs'}`}
				>
					{view.count}
				</span>
			{/if}
		</button>
	{/each}
</div>
