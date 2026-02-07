<script lang="ts">
	import { fade, scale } from 'svelte/transition';

	let open = false;

	type Shortcut = {
		keys: string[];
		description: string;
	};

	type ShortcutGroup = {
		name: string;
		shortcuts: Shortcut[];
	};

	const shortcutGroups: ShortcutGroup[] = [
		{
			name: 'Navigation',
			shortcuts: [
				{ keys: ['Cmd', 'K'], description: 'Open command palette' },
				{ keys: ['Cmd', '/'], description: 'Quick search' },
				{ keys: ['Cmd', 'Shift', 'N'], description: 'Quick capture' },
				{ keys: ['Esc'], description: 'Close modal / Cancel' }
			]
		},
		{
			name: 'Tasks',
			shortcuts: [
				{ keys: ['N'], description: 'New task (on tasks page)' },
				{ keys: ['Enter'], description: 'Open selected task' },
				{ keys: ['Cmd', 'Enter'], description: 'Save task' }
			]
		},
		{
			name: 'Notes Editor',
			shortcuts: [
				{ keys: ['Cmd', 'S'], description: 'Save note' },
				{ keys: ['Cmd', 'B'], description: 'Bold text' },
				{ keys: ['Cmd', 'I'], description: 'Italic text' },
				{ keys: ['Cmd', 'U'], description: 'Underline text' },
				{ keys: ['Cmd', '.'], description: 'AI transform (select text first)' },
				{ keys: ['Tab'], description: 'Accept AI completion' },
				{ keys: ['[['], description: 'Link to note (wiki-link)' },
				{ keys: ['@'], description: 'Mention/link to note' }
			]
		},
		{
			name: 'Command Palette',
			shortcuts: [
				{ keys: ['Arrow', 'Up/Down'], description: 'Navigate options' },
				{ keys: ['Enter'], description: 'Execute command' },
				{ keys: ['Tab'], description: 'Next option' },
				{ keys: ['Shift', 'Tab'], description: 'Previous option' }
			]
		},
		{
			name: 'Search',
			shortcuts: [
				{ keys: ['Cmd', '/'], description: 'Open quick search' },
				{ keys: ['Arrow', 'Up/Down'], description: 'Navigate results' },
				{ keys: ['Enter'], description: 'Open selected result' }
			]
		}
	];

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === '?' && !event.ctrlKey && !event.metaKey && !event.altKey) {
			// Check if we're not in an input field
			const target = event.target as HTMLElement;
			if (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable) {
				return;
			}
			event.preventDefault();
			open = true;
		}
		if (event.key === 'Escape' && open) {
			event.preventDefault();
			open = false;
		}
	}

	function close() {
		open = false;
	}
</script>

<svelte:window on:keydown={handleKeydown} />

{#if open}
	<div
		class="fixed inset-0 z-50 flex items-center justify-center p-4"
		role="presentation"
		transition:fade={{ duration: 100 }}
	>
		<div
			class="absolute inset-0 bg-black/60 backdrop-blur-sm"
			on:click={close}
			on:keydown={(e) => e.key === 'Escape' && close()}
			role="button"
			tabindex="-1"
			aria-label="Close shortcuts"
		></div>

		<div
			class="relative z-10 max-h-[85vh] w-full max-w-2xl overflow-auto rounded-2xl border border-slate-700 bg-slate-900/95 shadow-2xl"
			role="dialog"
			aria-modal="true"
			aria-label="Keyboard shortcuts"
			transition:scale={{ duration: 120, start: 0.97 }}
		>
			<div class="sticky top-0 flex items-center justify-between border-b border-slate-800 bg-slate-900/95 px-6 py-4 backdrop-blur-sm">
				<div>
					<h2 class="text-lg font-semibold text-white">Keyboard Shortcuts</h2>
					<p class="text-xs text-slate-500">Press <kbd class="rounded border border-slate-700 bg-slate-800 px-1 py-0.5 text-[10px]">?</kbd> anytime to see this</p>
				</div>
				<button
					class="rounded-lg p-2 text-slate-400 hover:bg-slate-800 hover:text-white"
					on:click={close}
					aria-label="Close"
				>
					<svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
					</svg>
				</button>
			</div>

			<div class="grid gap-6 p-6 md:grid-cols-2">
				{#each shortcutGroups as group}
					<div>
						<h3 class="mb-3 text-xs font-semibold uppercase tracking-wider text-slate-500">
							{group.name}
						</h3>
						<div class="space-y-2">
							{#each group.shortcuts as shortcut}
								<div class="flex items-center justify-between rounded-lg border border-slate-800/60 bg-slate-800/30 px-3 py-2">
									<span class="text-xs text-slate-300">{shortcut.description}</span>
									<div class="flex items-center gap-1">
										{#each shortcut.keys as key, i}
											{#if i > 0}
												<span class="text-[10px] text-slate-600">+</span>
											{/if}
											<kbd class="rounded border border-slate-700 bg-slate-800 px-1.5 py-0.5 text-[10px] text-slate-400">
												{key}
											</kbd>
										{/each}
									</div>
								</div>
							{/each}
						</div>
					</div>
				{/each}
			</div>

			<div class="border-t border-slate-800 px-6 py-4">
				<p class="text-center text-[11px] text-slate-600">
					Tip: Most shortcuts work from anywhere in the app
				</p>
			</div>
		</div>
	</div>
{/if}
