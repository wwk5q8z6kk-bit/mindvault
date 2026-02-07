<script lang="ts">
	import { onMount, tick } from 'svelte';
	import { fade, scale } from 'svelte/transition';
	import { actionsStore, filterAvailable } from '$lib/command-palette/registry';
	import { rankActions } from '$lib/command-palette/search';
	import { recordUsage } from '$lib/command-palette/usage';
	import {
		buildCommandContext,
		closePalette,
		openPalette,
		paletteOpen,
		paletteQuery
	} from '$lib/command-palette/store';
	import { buildDynamicActions, registerBuiltInActions } from '$lib/command-palette/actions';
	import type { CommandAction } from '$lib/command-palette/types';

	const ITEM_HEIGHT = 52;
	const LIST_HEIGHT = 320;
	const VISIBLE_COUNT = Math.ceil(LIST_HEIGHT / ITEM_HEIGHT) + 6;

	let inputRef: HTMLInputElement | null = null;
	let listRef: HTMLDivElement | null = null;
	let selectedIndex = 0;
	let scrollTop = 0;
	let searchActions: CommandAction[] = [];
	let searchTimeout: ReturnType<typeof setTimeout> | null = null;
	let searchLoading = false;

	onMount(() => {
		registerBuiltInActions();
		const unsubscribeOpen = paletteOpen.subscribe((open) => {
			if (open) {
				selectedIndex = 0;
				tick().then(() => inputRef?.focus());
			}
		});
		const unsubscribeQuery = paletteQuery.subscribe((value) => {
			const trimmed = value.trim().toLowerCase();
			const isSearch = trimmed.startsWith('search ') || trimmed.startsWith('find ');
			if (!isSearch) {
				searchActions = [];
				searchLoading = false;
				return;
			}
			const q = trimmed.replace(/^search\s+|^find\s+/, '').trim();
			if (searchTimeout) clearTimeout(searchTimeout);
			if (!q) {
				searchActions = [];
				searchLoading = false;
				return;
			}
			searchLoading = true;
			searchTimeout = setTimeout(async () => {
				const localCtx = buildCommandContext(value);
				const results = await localCtx.searchFts(q);
				searchActions = results.slice(0, 10).map((result) => {
					const title =
						result.content.length > 80 ? `${result.content.slice(0, 80)}…` : result.content;
					return {
						id: `search-${result.object_type}-${result.object_id}`,
						title: `Open ${result.object_type}: ${title}`,
						subtitle: result.object_id,
						group: 'Search results',
						handler: async (innerCtx) => {
							if (result.object_type === 'task') {
								await innerCtx.navigate('/tasks');
								innerCtx.selectTask(result.object_id);
								innerCtx.toast('Task selected', 'success');
							} else if (result.object_type === 'note') {
								await innerCtx.navigate(`/notes?note=${result.object_id}`);
								innerCtx.toast('Note opened', 'success');
							}
						}
					};
				});
				searchLoading = false;
			}, 150);
		});
		return () => {
			if (searchTimeout) clearTimeout(searchTimeout);
			unsubscribeOpen();
			unsubscribeQuery();
		};
	});

	$: ctx = buildCommandContext($paletteQuery);
	$: availableActions = filterAvailable($actionsStore, ctx);
	$: dynamicActions = buildDynamicActions($paletteQuery, ctx);
	$: isSearchMode =
		$paletteQuery.trim().toLowerCase().startsWith('search ') ||
		$paletteQuery.trim().toLowerCase().startsWith('find ');
	$: searchTerm = $paletteQuery
		.trim()
		.replace(/^search\s+|^find\s+/, '')
		.trim();
	$: rankQuery = isSearchMode ? searchTerm : $paletteQuery;
	$: rankedActions = rankActions(
		[...dynamicActions, ...searchActions, ...availableActions],
		rankQuery
	);

	// paletteOpen handled via subscription

	$: totalActions = rankedActions.length;
	$: startIndex = Math.max(0, Math.floor(scrollTop / ITEM_HEIGHT) - 3);
	$: endIndex = Math.min(totalActions, startIndex + VISIBLE_COUNT);
	$: visibleActions = rankedActions.slice(startIndex, endIndex);
	$: topPad = startIndex * ITEM_HEIGHT;
	$: bottomPad = (totalActions - endIndex) * ITEM_HEIGHT;

	function handleScroll(event: Event) {
		const target = event.currentTarget as HTMLDivElement;
		scrollTop = target.scrollTop;
	}

	function ensureVisible(index: number) {
		if (!listRef) return;
		const top = index * ITEM_HEIGHT;
		const bottom = top + ITEM_HEIGHT;
		if (top < listRef.scrollTop) {
			listRef.scrollTop = top;
		} else if (bottom > listRef.scrollTop + LIST_HEIGHT) {
			listRef.scrollTop = bottom - LIST_HEIGHT;
		}
	}

	function selectIndex(nextIndex: number) {
		if (!totalActions) return;
		selectedIndex = Math.max(0, Math.min(nextIndex, totalActions - 1));
		ensureVisible(selectedIndex);
	}

	async function executeAction(action: CommandAction) {
		try {
			await action.handler(ctx);
			recordUsage(action.id);
			if (action.closeOnRun !== false) {
				closePalette();
			} else {
				await tick();
				inputRef?.focus();
			}
		} catch {
			ctx.toast('Command failed', 'danger');
		}
	}

	function onKeydown(event: KeyboardEvent) {
		if (!$paletteOpen) {
			if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'k') {
				event.preventDefault();
				openPalette('');
			}
			return;
		}

		if (event.key === 'Escape') {
			event.preventDefault();
			closePalette();
			return;
		}

		switch (event.key) {
			case 'ArrowDown':
				event.preventDefault();
				selectIndex(selectedIndex + 1);
				break;
			case 'ArrowUp':
				event.preventDefault();
				selectIndex(selectedIndex - 1);
				break;
			case 'Home':
				event.preventDefault();
				selectIndex(0);
				break;
			case 'End':
				event.preventDefault();
				selectIndex(totalActions - 1);
				break;
			case 'Tab':
				event.preventDefault();
				selectIndex(selectedIndex + (event.shiftKey ? -1 : 1));
				break;
			case 'Enter': {
				event.preventDefault();
				const action = rankedActions[selectedIndex]?.action;
				if (action) void executeAction(action);
				break;
			}
		}
	}
</script>

<svelte:window on:keydown={onKeydown} />

{#if $paletteOpen}
	<div
		class="fixed inset-0 z-50 flex items-start justify-center bg-black/40 p-6 backdrop-blur-sm"
		role="button"
		tabindex="0"
		transition:fade={{ duration: 120 }}
		on:click|self={closePalette}
		on:keydown={(event) => {
			if (event.key === 'Enter' || event.key === ' ') {
				event.preventDefault();
				closePalette();
			}
		}}
	>
		<div
			class="w-full max-w-2xl rounded-2xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/95 p-4 shadow-2xl"
			transition:scale={{ duration: 140, start: 0.97 }}
		>
			<div
				class="flex items-center gap-2 rounded-xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))] px-3 py-2"
			>
				<span class="text-xs text-[rgb(var(--mv-muted))]">⌘K</span>
				<input
					bind:this={inputRef}
					class="w-full bg-transparent text-sm text-[rgb(var(--mv-text))] placeholder:text-[rgb(var(--mv-muted))] focus:outline-none"
					placeholder="Search commands, tasks, notes…"
					role="combobox"
					aria-expanded="true"
					aria-controls="command-list"
					aria-autocomplete="list"
					bind:value={$paletteQuery}
				/>
			</div>

			<div class="mt-3">
				<div
					id="command-list"
					role="listbox"
					class="max-h-[320px] overflow-auto"
					style={`height: ${LIST_HEIGHT}px`}
					on:scroll={handleScroll}
					bind:this={listRef}
				>
					<div style={`padding-top: ${topPad}px; padding-bottom: ${bottomPad}px`}>
						{#if totalActions === 0}
							<div
								class="rounded-xl border border-dashed border-[rgb(var(--mv-border))] p-6 text-center text-sm text-[rgb(var(--mv-muted))]"
							>
								{searchLoading ? 'Searching…' : 'No matching commands'}
							</div>
						{:else}
							{#each visibleActions as item, index (item.action.id)}
								{@const actualIndex = startIndex + index}
								<button
									class={`flex w-full items-start justify-between gap-3 rounded-xl px-3 py-2 text-left transition ${
										actualIndex === selectedIndex
											? 'bg-[rgb(var(--mv-accent))]/10 text-[rgb(var(--mv-text))]'
											: 'text-[rgb(var(--mv-text))] hover:bg-[rgb(var(--mv-panel-strong))]/60'
									}`}
									role="option"
									aria-selected={actualIndex === selectedIndex}
									on:click={() => executeAction(item.action)}
								>
									<div>
										<div class="text-sm font-medium">{item.action.title}</div>
										{#if item.action.subtitle}
											<div class="text-xs text-[rgb(var(--mv-muted))]">
												{item.action.subtitle}
											</div>
										{/if}
									</div>
									{#if item.action.group}
										<span
											class="rounded-full bg-[rgb(var(--mv-panel-strong))] px-2 py-1 text-[10px] uppercase text-[rgb(var(--mv-muted))]"
										>
											{item.action.group}
										</span>
									{/if}
								</button>
							{/each}
						{/if}
					</div>
				</div>
			</div>
		</div>
	</div>
{/if}
