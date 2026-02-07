<script lang="ts">
	import { page } from '$app/stores';

	const navItems = [
		{
			href: '/tasks',
			label: 'Tasks',
			icon: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2m-6 9l2 2 4-4" />`
		},
		{
			href: '/notes',
			label: 'Notes',
			icon: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />`
		},
		{
			href: '#capture',
			label: 'Capture',
			icon: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />`,
			action: 'capture'
		},
		{
			href: '#search',
			label: 'Search',
			icon: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />`,
			action: 'search'
		},
		{
			href: '/',
			label: 'Home',
			icon: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 12l2-2m0 0l7-7 7 7M5 10v10a1 1 0 001 1h3m10-11l2 2m-2-2v10a1 1 0 01-1 1h-3m-6 0a1 1 0 001-1v-4a1 1 0 011-1h2a1 1 0 011 1v4a1 1 0 001 1m-6 0h6" />`
		}
	];

	function handleAction(action: string | undefined) {
		if (action === 'capture') {
			window.dispatchEvent(
				new KeyboardEvent('keydown', { key: 'n', metaKey: true, shiftKey: true })
			);
		} else if (action === 'search') {
			window.dispatchEvent(
				new KeyboardEvent('keydown', { key: '/', metaKey: true })
			);
		}
	}

	function isActive(href: string): boolean {
		if (href === '/') return $page.url.pathname === '/';
		return $page.url.pathname.startsWith(href);
	}
</script>

<nav class="fixed bottom-0 left-0 right-0 z-40 border-t border-slate-800 bg-slate-900/95 backdrop-blur-sm md:hidden">
	<div class="flex items-center justify-around px-2 py-2 safe-area-bottom">
		{#each navItems as item}
			{#if item.action}
				<button
					class="flex flex-col items-center gap-1 rounded-lg px-3 py-2 transition {item.action === 'capture'
						? 'bg-sky-500/20 text-sky-300'
						: 'text-slate-400 hover:text-white'}"
					on:click={() => handleAction(item.action)}
				>
					<svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						{@html item.icon}
					</svg>
					<span class="text-[10px]">{item.label}</span>
				</button>
			{:else}
				<a
					href={item.href}
					class="flex flex-col items-center gap-1 rounded-lg px-3 py-2 transition {isActive(item.href)
						? 'text-sky-300'
						: 'text-slate-400 hover:text-white'}"
				>
					<svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						{@html item.icon}
					</svg>
					<span class="text-[10px]">{item.label}</span>
				</a>
			{/if}
		{/each}
	</div>
</nav>

<style>
	.safe-area-bottom {
		padding-bottom: max(0.5rem, env(safe-area-inset-bottom));
	}
</style>
