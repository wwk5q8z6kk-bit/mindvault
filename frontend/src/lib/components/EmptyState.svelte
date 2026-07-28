<script lang="ts">
	import Button from '$lib/components/Button.svelte';

	export let title: string;
	export let description: string = '';
	export let icon: 'inbox' | 'tasks' | 'tags' | 'bookmarks' | 'search' | 'generic' = 'generic';
	export let tone: 'sky' | 'violet' | 'emerald' | 'amber' | 'slate' = 'sky';
	export let actionLabel: string | undefined = undefined;
	export let onAction: (() => void) | undefined = undefined;
	export let compact = false;

	const toneClasses: Record<string, string> = {
		sky: 'bg-sky-500/15 text-sky-300',
		violet: 'bg-violet-500/15 text-violet-300',
		emerald: 'bg-emerald-500/15 text-emerald-300',
		amber: 'bg-amber-500/15 text-amber-300',
		slate: 'bg-[rgb(var(--mv-panel-strong))] text-[rgb(var(--mv-muted))]'
	};
</script>

<div
	class={`rounded-2xl border border-dashed border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/20 text-center ${
		compact ? 'p-6' : 'p-10'
	}`}
	role="status"
>
	<div class={`mx-auto mb-3 flex items-center justify-center rounded-full ${toneClasses[tone]} ${compact ? 'h-10 w-10' : 'h-14 w-14'}`}>
		{#if icon === 'inbox'}
			<svg class={compact ? 'h-5 w-5' : 'h-6 w-6'} fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
				<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M5 13l4 4L19 7" />
			</svg>
		{:else if icon === 'tasks'}
			<svg class={compact ? 'h-5 w-5' : 'h-6 w-6'} fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
				<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2" />
			</svg>
		{:else if icon === 'tags'}
			<svg class={compact ? 'h-5 w-5' : 'h-6 w-6'} fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
				<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M7 7h.01M3 11l8.5-8.5a2 2 0 012.828 0L21 9.172a2 2 0 010 2.828L12.5 20.5a2 2 0 01-2.828 0L3 14V11z" />
			</svg>
		{:else if icon === 'bookmarks'}
			<svg class={compact ? 'h-5 w-5' : 'h-6 w-6'} fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
				<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M5 5a2 2 0 012-2h10a2 2 0 012 2v16l-7-3.5L5 21V5z" />
			</svg>
		{:else if icon === 'search'}
			<svg class={compact ? 'h-5 w-5' : 'h-6 w-6'} fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
				<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
			</svg>
		{:else}
			<svg class={compact ? 'h-5 w-5' : 'h-6 w-6'} fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
				<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M12 6v12m6-6H6" />
			</svg>
		{/if}
	</div>
	<h3 class="text-sm font-medium text-[rgb(var(--mv-text))]">{title}</h3>
	{#if description}
		<p class="mx-auto mt-1 max-w-sm text-xs text-[rgb(var(--mv-muted))]/70">{description}</p>
	{/if}
	{#if $$slots.default}
		<div class="mt-4 flex flex-wrap items-center justify-center gap-2">
			<slot />
		</div>
	{:else if actionLabel && onAction}
		<div class="mt-4 flex justify-center">
			<Button size="sm" on:click={() => onAction?.()}>{actionLabel}</Button>
		</div>
	{/if}
	{#if $$slots.footer}
		<div class="mt-3 text-xs text-[rgb(var(--mv-muted))]">
			<slot name="footer" />
		</div>
	{/if}
</div>
