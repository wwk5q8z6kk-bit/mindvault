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
		class="fixed inset-0 z-50 flex items-start justify-center bg-black/60 pt-[15vh] backdrop-blur-sm transition-all duration-200"
		role="button"
		tabindex="0"
		transition:fade={{ duration: 150 }}
		on:click|self={closePalette}
		on:keydown={(event) => {
			if (event.key === 'Enter' || event.key === ' ') {
				event.preventDefault();
				closePalette();
			}
		}}
	>
		<div
			class="w-full max-w-2xl overflow-hidden rounded-2xl border border-[rgb(var(--mv-border))]/50 bg-[rgb(var(--mv-panel))]/80 p-0 shadow-2xl backdrop-blur-xl ring-1 ring-white/10"
			transition:scale={{ duration: 200, start: 0.95 }}
		>
			<div class="flex items-center gap-3 border-b border-[rgb(var(--mv-border))]/50 px-4 py-3">
				<span class="text-xs font-medium text-[rgb(var(--mv-muted))] opacity-70">⌘K</span>
				<input
					bind:this={inputRef}
					class="w-full bg-transparent text-lg text-[rgb(var(--mv-text))] placeholder:text-[rgb(var(--mv-muted))]/50 focus:outline-none"
					placeholder="What do you need?"
					role="combobox"
					aria-expanded="true"
					aria-controls="command-list"
					aria-autocomplete="list"
					bind:value={$paletteQuery}
				/>
			</div>

			<div class="relative">
				<div
					id="command-list"
					role="listbox"
					class="max-h-[320px] overflow-auto scroll-smooth py-2"
					style={`height: ${Math.min(LIST_HEIGHT, totalActions * ITEM_HEIGHT + 16)}px`}
					on:scroll={handleScroll}
					bind:this={listRef}
				>
					<div style={`padding-top: ${topPad}px; padding-bottom: ${bottomPad}px`}>
						{#if totalActions === 0}
							<div
								class="flex flex-col items-center justify-center p-8 text-center text-[rgb(var(--mv-muted))]"
							>
								{#if searchLoading}
									<div
										class="h-6 w-6 animate-spin rounded-full border-2 border-[rgb(var(--mv-border))] border-t-[rgb(var(--mv-accent))]"
									></div>
									<p class="mt-2 text-sm">Searching...</p>
								{:else}
									<p class="text-sm">No matching commands found.</p>
								{/if}
							</div>
						{:else}
							{#each visibleActions as item, index (item.action.id)}
								{@const actualIndex = startIndex + index}
								<div class="px-2">
									<button
										class={`group flex w-full items-center justify-between gap-3 rounded-lg px-3 py-3 text-left transition-all duration-150 ${
											actualIndex === selectedIndex
												? 'bg-[rgb(var(--mv-accent))]/10 text-[rgb(var(--mv-text))] ring-1 ring-[rgb(var(--mv-accent))]/20'
												: 'text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-panel-strong))]/40 hover:text-[rgb(var(--mv-text))]'
										}`}
										role="option"
										aria-selected={actualIndex === selectedIndex}
										on:click={() => executeAction(item.action)}
									>
										<div class="flex flex-col gap-0.5">
											<div
												class="text-sm font-medium ${actualIndex === selectedIndex
													? 'text-[rgb(var(--mv-accent-strong))]'
													: ''}"
											>
												{item.action.title}
											</div>
											{#if item.action.subtitle}
												<div class="text-xs opacity-70 truncate max-w-[400px]">
													{item.action.subtitle}
												</div>
											{/if}
										</div>
										{#if item.action.group}
											<span
												class={`rounded-md px-2 py-1 text-[10px] font-semibold uppercase tracking-wider ${
													actualIndex === selectedIndex
														? 'bg-[rgb(var(--mv-accent))]/20 text-[rgb(var(--mv-accent-strong))]'
														: 'bg-[rgb(var(--mv-panel-strong))] text-[rgb(var(--mv-muted))]'
												}`}
											>
												{item.action.group}
											</span>
										{/if}
									</button>
								</div>
							{/each}
						{/if}
					</div>
				</div>
			</div>

			<div
				class="border-t border-[rgb(var(--mv-border))]/50 bg-[rgb(var(--mv-panel-strong))]/30 px-4 py-2 text-[10px] text-[rgb(var(--mv-muted))] flex justify-between items-center"
			>
				<div class="flex gap-3">
					<span
						><kbd
							class="font-mono bg-[rgb(var(--mv-panel-strong))] px-1 rounded border border-[rgb(var(--mv-border))]"
							>↵</kbd
						> to select</span
					>
					<span
						><kbd
							class="font-mono bg-[rgb(var(--mv-panel-strong))] px-1 rounded border border-[rgb(var(--mv-border))]"
							>↓↑</kbd
						> to navigate</span
					>
					<span
						><kbd
							class="font-mono bg-[rgb(var(--mv-panel-strong))] px-1 rounded border border-[rgb(var(--mv-border))]"
							>esc</kbd
						> to close</span
					>
				</div>
				<div>MindVault Core</div>
			</div>
		</div>
	</div>
{/if}
