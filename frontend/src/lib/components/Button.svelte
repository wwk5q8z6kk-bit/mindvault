<script lang="ts">
	import { createEventDispatcher } from 'svelte';

	export let variant: 'primary' | 'secondary' | 'danger' | 'ghost' = 'primary';
	export let size: 'sm' | 'md' | 'lg' = 'md';
	export let disabled = false;
	export let type: 'button' | 'submit' | 'reset' = 'button';

	const dispatch = createEventDispatcher<{ click: MouseEvent }>();

	const base =
		'inline-flex items-center justify-center font-medium transition rounded-lg focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[rgb(var(--mv-ring))]/70 disabled:opacity-50 disabled:pointer-events-none';

	const variants: Record<string, string> = {
		primary:
			'bg-[rgb(var(--mv-accent))] text-white hover:bg-[rgb(var(--mv-accent))]/80',
		secondary:
			'border border-[rgb(var(--mv-border))] text-[rgb(var(--mv-muted))] hover:border-[rgb(var(--mv-muted))]/40 hover:text-[rgb(var(--mv-text))]',
		danger:
			'bg-[rgb(var(--mv-danger))]/20 text-red-200 hover:bg-[rgb(var(--mv-danger))]/30',
		ghost:
			'text-[rgb(var(--mv-muted))] hover:text-[rgb(var(--mv-text))] hover:bg-[rgb(var(--mv-panel-strong))]'
	};

	const sizes: Record<string, string> = {
		sm: 'px-2.5 py-1 text-xs gap-1.5',
		md: 'px-4 py-2 text-sm gap-2',
		lg: 'px-6 py-2.5 text-base gap-2'
	};
</script>

<button
	{type}
	{disabled}
	class="{base} {variants[variant]} {sizes[size]}"
	on:click={(e) => dispatch('click', e)}
>
	<slot />
</button>
