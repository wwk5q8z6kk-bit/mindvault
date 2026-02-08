<script lang="ts">
	import '../app.css';
	import { page } from '$app/stores';
	import ToastStack from '$lib/components/ToastStack.svelte';
	import CommandPalette from '$lib/components/CommandPalette.svelte';
	import FocusPlannerModal from '$lib/components/FocusPlannerModal.svelte';
	import QuickCaptureModal from '$lib/components/QuickCaptureModal.svelte';
	import QuickSearch from '$lib/components/QuickSearch.svelte';
	import KeyboardShortcutsModal from '$lib/components/KeyboardShortcutsModal.svelte';
	import MobileNav from '$lib/components/MobileNav.svelte';
	import UndoIndicator from '$lib/components/UndoIndicator.svelte';
	import LinkPreview from '$lib/components/LinkPreview.svelte';
	import FavoritesBar from '$lib/components/FavoritesBar.svelte';
	import ProposalInbox from '$lib/components/ProposalInbox.svelte';
	import { startSyncLoop, pendingSyncCount } from '$lib/stores/tasks';
	import { loadNotes } from '$lib/stores/notes';
	import { startNotifications, stopNotifications } from '$lib/stores/notifications';
	import { startWebSocket, stopWebSocket, wsStatus } from '$lib/stores/websocket';
	import { handleUndoKeyboard } from '$lib/stores/undo';
	import NamespaceSelector from '$lib/components/NamespaceSelector.svelte';
	import { loadAvailableNamespaces } from '$lib/stores/namespace';
	import { connectAgentStream, disconnectAgentStream } from '$lib/api/agent';
	import { onMount } from 'svelte';

	let online = true;
	let mobileMenuOpen = false;

	function closeMobileMenu() {
		mobileMenuOpen = false;
	}

	const navItems = [
		{ label: 'Plan', href: '/plan' },
		{ label: 'Tasks', href: '/tasks' },
		{ label: 'Focus', href: '/focus' },
		{ label: 'Goals', href: '/goals' },
		{ label: 'Notes', href: '/notes' },
		{ label: 'Voice', href: '/voice' },
		{ label: 'Chat', href: '/chat' },
		{ label: 'Relay', href: '/relay' },
		{ label: 'Inbox', href: '/inbox' },
		{ label: 'Templates', href: '/templates' },
		{ label: 'Daily', href: '/daily' },
		{ label: 'Search', href: '/search' },
		{ label: 'Kanban', href: '/kanban' },
		{ label: 'Calendar', href: '/calendar' },
		{ label: 'Timeline', href: '/timeline' },
		{ label: 'Review', href: '/review' },
		{ label: 'Tags', href: '/tags' },
		{ label: 'Bookmarks', href: '/bookmarks' },
		{ label: 'Flashcards', href: '/flashcards' },
		{ label: 'Graph', href: '/graph' },
		{ label: 'Canvas', href: '/canvas' },
		{ label: 'PDF', href: '/pdf' },
		{ label: 'Trash', href: '/trash' },
		{ label: 'Autonomy', href: '/autonomy' },
		{ label: 'Federation', href: '/federation' },
		{ label: 'Sync', href: '/sync' },
		{ label: 'Plugins', href: '/plugins' },
		{ label: 'Adapters', href: '/adapters' },
		{ label: 'Provenance', href: '/provenance' },
		{ label: 'Profiles', href: '/settings/profiles' },
		{ label: 'Settings', href: '/settings' }
	];

	const routeMeta: Array<{ href: string; title: string; subtitle: string }> = [
		{
			href: '/plan',
			title: 'Smart Daily Planner',
			subtitle: 'AI-powered daily planning with priorities and focus blocks.'
		},
		{
			href: '/tasks',
			title: 'Task Management Center',
			subtitle: 'Command the day. Syncs locally by default.'
		},
		{
			href: '/focus',
			title: 'Focus Mode',
			subtitle: 'Deep work with AI-prioritized tasks and Pomodoro timer.'
		},
		{
			href: '/voice',
			title: 'Voice Notes',
			subtitle: 'Record audio and get AI-powered transcriptions.'
		},
		{
			href: '/goals',
			title: 'Goals & Habits',
			subtitle: 'Track long-term outcomes with streaks, milestones, and reviews.'
		},
		{
			href: '/notes',
			title: 'Notes Workspace',
			subtitle: 'Capture and refine ideas with backlinks and rich editing.'
		},
		{
			href: '/chat',
			title: 'AI Chat',
			subtitle: 'Ask questions and get answers grounded in your knowledge base.'
		},
		{
			href: '/relay',
			title: 'Communication Relay',
			subtitle: 'Sovereign messaging with contacts — vault-registered, context-aware.'
		},
		{
			href: '/inbox',
			title: 'Smart Inbox',
			subtitle: 'Triage new captures — tag, categorize, and route items.'
		},
		{
			href: '/templates',
			title: 'Template Studio',
			subtitle: 'Create reusable note and task blueprints with variable placeholders.'
		},
		{
			href: '/daily',
			title: 'Daily Notes',
			subtitle: 'Plan, reflect, and keep momentum with date-based notes.'
		},
		{
			href: '/search',
			title: 'Global Search',
			subtitle: 'Find tasks and notes instantly across your vault.'
		},
		{
			href: '/kanban',
			title: 'Kanban Board',
			subtitle: 'Move work forward across inbox, planned, in-progress, and done.'
		},
		{
			href: '/calendar',
			title: 'Calendar Planner',
			subtitle: 'Visualize due dates and weekly workload at a glance.'
		},
		{
			href: '/timeline',
			title: 'Activity Timeline',
			subtitle: 'Review edits and completions in chronological order.'
		},
		{
			href: '/review',
			title: 'Proactive Review',
			subtitle: 'AI-powered weekly digest and spaced review prompts.'
		},
		{
			href: '/tags',
			title: 'Tag Manager',
			subtitle: 'Browse, rename, merge, and AI-summarize tags across your vault.'
		},
		{
			href: '/bookmarks',
			title: 'Reading List',
			subtitle: 'Save and organize web clips, references, and reading material.'
		},
		{
			href: '/flashcards',
			title: 'Flashcards',
			subtitle: 'AI-generated spaced repetition cards from your notes.'
		},
		{
			href: '/graph',
			title: 'Knowledge Graph',
			subtitle: 'Visualize connections between nodes in your vault.'
		},
		{
			href: '/canvas',
			title: 'Canvas',
			subtitle: 'Freeform spatial workspace — drag, arrange, and mind-map your ideas.'
		},
		{
			href: '/pdf',
			title: 'PDF Viewer',
			subtitle: 'Read, annotate, and export highlights from PDF documents.'
		},
		{
			href: '/trash',
			title: 'Trash',
			subtitle: 'Recover deleted tasks and notes or remove them permanently.'
		},
		{
			href: '/autonomy',
			title: 'Autonomy Controls',
			subtitle: 'Configure when agents act autonomously vs. defer to your approval.'
		},
		{
			href: '/federation',
			title: 'Federation',
			subtitle: 'Connect with trusted peer vaults for read-only knowledge queries.'
		},
		{
			href: '/sync',
			title: 'Device Sync',
			subtitle: 'Export and import vault snapshots for offline synchronization.'
		},
		{
			href: '/plugins',
			title: 'Plugins',
			subtitle: 'Manage WASM-sandboxed extensions that hook into vault events.'
		},
		{
			href: '/adapters',
			title: 'Adapters',
			subtitle: 'Bridge external messaging platforms into the relay engine.'
		},
		{
			href: '/provenance',
			title: 'Provenance & Observability',
			subtitle: 'Audit trail, agent metrics, and transparency logs.'
		},
		{
			href: '/settings/profiles',
			title: 'Profiles & Access Keys',
			subtitle: 'Manage API access keys and permission templates.'
		},
		{
			href: '/settings/keychain',
			title: 'Sovereign Keychain',
			subtitle: 'Vault & credential management'
		},
		{
			href: '/settings',
			title: 'Settings',
			subtitle: 'Manage preferences, connection settings, and local cache.'
		},
		{
			href: '/',
			title: 'MindVault',
			subtitle: 'Local-first knowledge and execution workspace.'
		}
	];

	function resolveRouteMeta(pathname: string) {
		if (pathname === '/') {
			return routeMeta.find((item) => item.href === '/') ?? routeMeta[0];
		}
		return routeMeta.find((item) => item.href !== '/' && pathname.startsWith(item.href)) ?? routeMeta[0];
	}

	$: currentRoute = resolveRouteMeta($page.url.pathname);

	onMount(() => {
		const onOnline = () => (online = true);
		const onOffline = () => (online = false);
		online = navigator.onLine;
		startSyncLoop();
		startNotifications();
		startWebSocket();
		connectAgentStream();
		loadAvailableNamespaces();
		loadNotes();
		window.addEventListener('online', onOnline);
		window.addEventListener('offline', onOffline);
		return () => {
			window.removeEventListener('online', onOnline);
			window.removeEventListener('offline', onOffline);
			stopWebSocket();
			disconnectAgentStream();
		};
	});

	async function handleGlobalKeydown(event: KeyboardEvent) {
		// Handle undo/redo shortcuts (Cmd+Z, Cmd+Shift+Z, Ctrl+Y)
		await handleUndoKeyboard(event);
	}
</script>

<svelte:window on:keydown={handleGlobalKeydown} />

<svelte:head>
	<title>MindVault</title>
</svelte:head>

<div class="flex min-h-screen bg-[rgb(var(--mv-bg))] text-[rgb(var(--mv-text))]">
	<aside
		class="hidden w-64 flex-col border-r border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/80 p-6 md:flex"
	>
		<div class="flex items-center gap-3 text-lg font-semibold">
			<div class="flex h-10 w-10 items-center justify-center rounded-xl bg-sky-500/20 text-sky-200">
				MV
			</div>
			<span>MindVault</span>
		</div>

		<nav class="mt-10 flex flex-1 flex-col gap-2 text-sm">
			{#each navItems as item (item.href)}
				<a
					href={item.href}
					class={`rounded-lg px-3 py-2 transition ${
						$page.url.pathname.startsWith(item.href)
							? 'bg-slate-800 text-white'
							: 'text-slate-300 hover:bg-slate-900/60'
					}`}
				>
					{item.label}
				</a>
			{/each}
		</nav>

		<div class="text-xs text-slate-500">Local-first · Private · Offline-ready</div>
	</aside>

	<div class="flex flex-1 flex-col">
		<header
			class="flex items-center justify-between border-b border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/80 px-6 py-4"
		>
			<div class="flex items-center gap-3">
				<button
					class="flex h-8 w-8 items-center justify-center rounded-lg text-slate-400 hover:bg-slate-800 hover:text-white md:hidden"
					on:click={() => (mobileMenuOpen = !mobileMenuOpen)}
					aria-label="Toggle menu"
				>
					<svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						{#if mobileMenuOpen}
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
						{:else}
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h16M4 18h16" />
						{/if}
					</svg>
				</button>
				<div>
					<h1 class="text-lg font-semibold text-white">{currentRoute.title}</h1>
					<p class="hidden text-xs text-slate-400 sm:block">{currentRoute.subtitle}</p>
				</div>
			</div>
			<div class="flex items-center gap-2 text-xs text-slate-400">
				<NamespaceSelector />
				<span
					class={`rounded-full px-3 py-1 ${
						$wsStatus === 'connected'
							? 'bg-emerald-500/20 text-emerald-200'
							: online
								? 'bg-sky-500/20 text-sky-200'
								: 'bg-amber-500/20 text-amber-200'
					}`}
				>
					{$wsStatus === 'connected' ? 'Live' : online ? 'Online' : 'Offline'}
				</span>
				{#if $pendingSyncCount > 0}
					<span class="rounded-full bg-amber-500/20 px-2 py-1 text-[10px] font-medium text-amber-200">
						{$pendingSyncCount} pending
					</span>
				{/if}
			</div>
		</header>
		<FavoritesBar />
		<!-- Mobile slide-out menu -->
		{#if mobileMenuOpen}
			<div
				class="fixed inset-0 z-30 md:hidden"
				role="presentation"
			>
				<div
					class="absolute inset-0 bg-black/50"
					on:click={closeMobileMenu}
					on:keydown={(e) => e.key === 'Escape' && closeMobileMenu()}
					role="button"
					tabindex="-1"
					aria-label="Close menu"
				></div>
				<nav class="absolute left-0 top-0 flex h-full w-64 flex-col border-r border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))] p-6">
					<div class="flex items-center gap-3 text-lg font-semibold">
						<div class="flex h-10 w-10 items-center justify-center rounded-xl bg-sky-500/20 text-sky-200">
							MV
						</div>
						<span>MindVault</span>
					</div>
					<div class="mt-8 flex flex-1 flex-col gap-2 text-sm">
						{#each navItems as item (item.href)}
							<a
								href={item.href}
								class={`rounded-lg px-3 py-2 transition ${
									$page.url.pathname.startsWith(item.href)
										? 'bg-slate-800 text-white'
										: 'text-slate-300 hover:bg-slate-900/60'
								}`}
								on:click={closeMobileMenu}
							>
								{item.label}
							</a>
						{/each}
					</div>
					<div class="text-xs text-slate-500">Local-first · Private · Offline-ready</div>
				</nav>
			</div>
		{/if}

		<main class="flex-1 bg-[rgb(var(--mv-bg))]/70 p-6 pb-20 md:pb-6">
			<slot />
		</main>
	</div>
</div>

<ToastStack />
<CommandPalette />
<FocusPlannerModal />
<QuickCaptureModal />
<QuickSearch />
<KeyboardShortcutsModal />
<MobileNav />
<UndoIndicator />
<LinkPreview />
<ProposalInbox />
