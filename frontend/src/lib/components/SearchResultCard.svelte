<script lang="ts">
	import { ArrowUpRight, Clock3, Database, Network } from '@lucide/svelte';
	import type { SearchResultDto } from '$lib/api/types';
	import { kindBadgeClass, kindLabel } from '$lib/utils/kind-helpers';
	import {
		matchSourceLabel,
		namespaceLabel,
		relativeUpdatedLabel,
		sourceLabel
	} from '$lib/search/results';
	import { createEventDispatcher } from 'svelte';

	export let result: SearchResultDto;
	export let index: number;
	export let selected = false;

	const dispatch = createEventDispatcher<{
		open: void;
		openGraph: void;
		select: void;
		filterTag: string;
		move: number;
	}>();

	function handleResultKeydown(event: KeyboardEvent) {
		if (event.key === 'ArrowDown') {
			event.preventDefault();
			dispatch('move', 1);
		} else if (event.key === 'ArrowUp') {
			event.preventDefault();
			dispatch('move', -1);
		}
	}

	$: source = sourceLabel(result.node.source);
	$: namespace = namespaceLabel(result.node.namespace);
	$: match = matchSourceLabel(result.match_source);
	$: updated = relativeUpdatedLabel(result.node.temporal.updated_at);
	$: typeLabel = kindLabel(result.node.kind);
</script>

<article
	class={`overflow-hidden rounded-xl border bg-[rgb(var(--mv-panel))]/60 transition ${
		selected
			? 'border-sky-500/70 ring-1 ring-sky-500/25'
			: 'border-[rgb(var(--mv-border))] hover:border-[rgb(var(--mv-muted))]/40'
	}`}
	role="listitem"
	aria-label={`${typeLabel}: ${result.node.title || 'Untitled'}`}
>
	<button
		data-result-item
		data-result-index={index}
		class="block w-full px-4 pb-3 pt-4 text-left outline-none transition hover:bg-[rgb(var(--mv-panel-strong))]/25 focus-visible:bg-sky-500/10"
		on:click={() => dispatch('open')}
		on:focus={() => dispatch('select')}
		on:keydown={handleResultKeydown}
		aria-label={`Open ${result.node.title || 'Untitled'} ${typeLabel.toLowerCase()}`}
	>
		<div class="flex min-w-0 items-start gap-3">
			<div class="min-w-0 flex-1">
				<div class="flex flex-wrap items-center gap-2">
					<span
						class={`rounded-full px-2 py-0.5 text-[10px] font-medium ${kindBadgeClass(result.node.kind)}`}
					>
						{typeLabel}
					</span>
					<h3 class="min-w-0 truncate text-sm font-semibold text-[rgb(var(--mv-text))]">
						{result.node.title || 'Untitled'}
					</h3>
				</div>

				{#if result.node.content}
					<p class="mt-2 line-clamp-2 max-w-3xl text-xs leading-5 text-[rgb(var(--mv-muted))]">
						{result.node.content.slice(0, 240)}
					</p>
				{/if}
			</div>

			<ArrowUpRight
				class="mt-0.5 shrink-0 text-[rgb(var(--mv-muted))]/60"
				size={16}
				strokeWidth={1.8}
				aria-hidden="true"
			/>
		</div>

		<div
			class="mt-3 flex flex-wrap items-center gap-x-3 gap-y-1 text-[10px] text-[rgb(var(--mv-muted))]/75"
		>
			<span class="inline-flex items-center gap-1.5" title="Search match method">
				<Database size={12} strokeWidth={1.8} aria-hidden="true" />
				{match}
			</span>
			<span aria-hidden="true">·</span>
			<span>{namespace}</span>
			{#if source}
				<span aria-hidden="true">·</span>
				<span title={result.node.source ?? undefined}>Source: {source}</span>
			{/if}
			<span aria-hidden="true">·</span>
			<time
				class="inline-flex items-center gap-1.5"
				datetime={result.node.temporal.updated_at}
				title={new Date(result.node.temporal.updated_at).toLocaleString()}
			>
				<Clock3 size={12} strokeWidth={1.8} aria-hidden="true" />
				{updated}
			</time>
		</div>
	</button>

	<div
		class="flex flex-wrap items-center justify-between gap-2 border-t border-[rgb(var(--mv-border))]/70 px-4 py-2.5"
	>
		<div class="flex min-w-0 flex-wrap gap-1.5" aria-label="Result tags">
			{#if result.node.tags.length > 0}
				{#each result.node.tags.slice(0, 4) as tag (tag)}
					<button
						class="rounded-md bg-[rgb(var(--mv-panel-strong))]/70 px-2 py-1 text-[10px] text-[rgb(var(--mv-muted))] outline-none transition hover:text-[rgb(var(--mv-text))] focus-visible:ring-2 focus-visible:ring-[rgb(var(--mv-ring))]/70"
						on:click={() => dispatch('filterTag', tag)}
						aria-label={`Filter results by ${tag}`}
					>
						{tag}
					</button>
				{/each}
			{:else}
				<span class="text-[10px] text-[rgb(var(--mv-muted))]/50">No tags</span>
			{/if}
		</div>

		<button
			class="inline-flex items-center gap-1.5 rounded-lg px-2 py-1 text-[10px] font-medium text-sky-300 outline-none transition hover:bg-sky-500/10 focus-visible:ring-2 focus-visible:ring-[rgb(var(--mv-ring))]/70"
			on:click={() => dispatch('openGraph')}
			aria-label={`View connections for ${result.node.title || 'Untitled'}`}
		>
			<Network size={13} strokeWidth={1.8} aria-hidden="true" />
			View connections
		</button>
	</div>
</article>
