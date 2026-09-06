<script lang="ts">
	import '../app.css';
	import { page } from '$app/stores';
	import ToastStack from '$lib/components/ToastStack.svelte';
	import { focusPlannerState } from '$lib/stores/ui';
	import { paletteOpen, openPalette } from '$lib/command-palette/store';
	import QuickCaptureModal from '$lib/components/QuickCaptureModal.svelte';
	import QuickSearch from '$lib/components/QuickSearch.svelte';
	import KeyboardShortcutsModal from '$lib/components/KeyboardShortcutsModal.svelte';
	import MobileNav from '$lib/components/MobileNav.svelte';
	import UndoIndicator from '$lib/components/UndoIndicator.svelte';
	import LinkPreview from '$lib/components/LinkPreview.svelte';
	import FavoritesBar from '$lib/components/FavoritesBar.svelte';
	import ProposalInbox from '$lib/components/ProposalInbox.svelte';
	import ApiHealthBanner from '$lib/components/ApiHealthBanner.svelte';
	import { apiHealth } from '$lib/api/client';
	import { connectionStatusLabel, deriveConnectionStatus } from '$lib/connection/status';
	import { startSyncLoop, stopSyncLoop, pendingSyncCount } from '$lib/stores/tasks';
	import { loadNotes } from '$lib/stores/notes';
	import { startNotifications, stopNotifications } from '$lib/stores/notifications';
	import { startWebSocket, stopWebSocket, wsStatus } from '$lib/stores/websocket';
	import { handleUndoKeyboard } from '$lib/stores/undo';
	import NamespaceSelector from '$lib/components/NamespaceSelector.svelte';
	import ContextBoundary from '$lib/components/ContextBoundary.svelte';
	import { loadAvailableNamespaces } from '$lib/stores/namespace';
	import { connectAgentStream, disconnectAgentStream } from '$lib/api/agent';
	import { keychainStore, pollVaultStatus } from '$lib/stores/keychain';
	import { pushToast } from '$lib/stores/toast';
	import { onMount } from 'svelte';
	import { fly, slide } from 'svelte/transition';
	import {
		viewPreferences,
		toggleSidebarGroup,
		isSidebarGroupCollapsed
	} from '$lib/stores/view-preferences';
	import {
		isSidebarNavigationItemActive,
		sidebarNavigationGroups,
		sidebarNavigationHref
	} from '$lib/navigation/sidebar';

	let online = true;
	$: connectionStatus = deriveConnectionStatus({
		browserOnline: online,
		apiStatus: $apiHealth.status,
		wsStatus: $wsStatus
	});
	let mobileMenuOpen = false;
	let keychainStatusPoll: ReturnType<typeof setInterval> | null = null;

	function closeMobileMenu() {
		mobileMenuOpen = false;
	}

	const routeMeta: Array<{ href: string; title: string; subtitle: string }> = [
		{
			href: '/onboarding',
			title: 'Welcome to MindVault',
			subtitle: 'Set up your private, local-first Personal Vault.'
		},
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
			title: 'Notes',
			subtitle: 'Capture and refine ideas in your private, local-first vault.'
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
			href: '/stats',
			title: 'Stats',
			subtitle: 'Productivity statistics and trends.'
		},
		{
			href: '/insights',
			title: 'Insights',
			subtitle: 'AI-powered knowledge analysis.'
		},
		{
			href: '/tags',
			title: 'Tag Manager',
			subtitle: 'Browse, rename, merge, and AI-summarize tags across your vault.'
		},
		{
			href: '/bookmarks',
			title: 'Reading List',
			subtitle: 'Bookmarks, templates, and flashcards.'
		},
		{
			href: '/media',
			title: 'Media Library',
			subtitle: 'Browse and manage all attachments across notes and tasks.'
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
			href: '/work-orders',
			title: 'Work Orders',
			subtitle: 'Governed agent runs, gate evidence, and verified artifacts.'
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
		const defaultMeta = routeMeta.find((item) => item.href === '/') ?? routeMeta[0];
		if (pathname === '/') {
			return defaultMeta;
		}
		return (
			routeMeta.find((item) => item.href !== '/' && pathname.startsWith(item.href)) ?? defaultMeta
		);
	}

	$: currentRoute = resolveRouteMeta($page.url.pathname);
	$: isNotesRoute = $page.url.pathname === '/notes';

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
		void pollVaultStatus();
		keychainStatusPoll = setInterval(() => {
			void pollVaultStatus();
		}, 30_000);
		const onUnhandledRejection = (e: PromiseRejectionEvent) => {
			console.error('Unhandled promise rejection:', e.reason);
			pushToast('An unexpected error occurred', 'danger');
		};
		window.addEventListener('online', onOnline);
		window.addEventListener('offline', onOffline);
		window.addEventListener('unhandledrejection', onUnhandledRejection);
		return () => {
			window.removeEventListener('online', onOnline);
			window.removeEventListener('offline', onOffline);
			window.removeEventListener('unhandledrejection', onUnhandledRejection);
			if (keychainStatusPoll) {
				clearInterval(keychainStatusPoll);
				keychainStatusPoll = null;
			}
			stopSyncLoop();
			stopNotifications();
			stopWebSocket();
			disconnectAgentStream();
		};
	});

	async function handleGlobalKeydown(event: KeyboardEvent) {
		if (mobileMenuOpen && event.key === 'Escape') {
			closeMobileMenu();
			return;
		}
		// Open command palette on Cmd+K / Ctrl+K
		if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'k') {
			event.preventDefault();
			openPalette('');
			return;
		}
		// Handle undo/redo shortcuts (Cmd+Z, Cmd+Shift+Z, Ctrl+Y)
		await handleUndoKeyboard(event);
	}
</script>

<svelte:window on:keydown={handleGlobalKeydown} />

<svelte:head>
	<title>MindVault</title>
</svelte:head>

<div
	class={`${isNotesRoute ? 'block h-screen overflow-hidden' : 'flex min-h-screen'} bg-[rgb(var(--mv-bg))] text-[rgb(var(--mv-text))] font-sans antialiased selection:bg-[rgb(var(--mv-accent))]/20 selection:text-[rgb(var(--mv-accent-strong))]`}
>
	{#if !isNotesRoute}
		<aside
			class="hidden w-[280px] flex-col rounded-3xl border border-white/5 bg-[rgb(var(--mv-panel))]/60 backdrop-blur-3xl p-5 md:flex transition-all duration-500 ease-out m-4 h-[calc(100vh-2rem)] shadow-2xl"
		>
			<div class="flex items-center gap-3 px-2 py-3 mb-6">
				<div
					class="relative flex h-10 w-10 items-center justify-center rounded-xl bg-gradient-to-br from-[rgb(var(--mv-accent))] to-[rgb(var(--mv-accent-strong))] text-white shadow-lg shadow-[rgb(var(--mv-accent))]/20"
				>
					<span class="font-bold text-lg tracking-tight">MV</span>
					<div class="absolute inset-0 rounded-xl ring-1 ring-inset ring-white/20"></div>
				</div>
				<div class="flex flex-col">
					<span class="font-bold text-lg leading-tight tracking-tight">MindVault</span>
					<span
						class="text-[10px] uppercase tracking-wider font-semibold text-[rgb(var(--mv-muted))] opacity-80"
						>Personal Vault</span
					>
				</div>
			</div>

			<nav class="flex-1 overflow-y-auto pr-2 space-y-6">
				{#each sidebarNavigationGroups as group}
					{@const preferenceKey = group.preferenceKey ?? group.label}
					{@const collapsed =
						group.collapsible && preferenceKey
							? isSidebarGroupCollapsed($viewPreferences.sidebarCollapsed, preferenceKey)
							: false}
					<div>
						{#if group.label}
							{#if group.collapsible}
								<button
									class="flex w-full items-center justify-between mb-2 px-3 text-[11px] font-bold uppercase tracking-widest text-[rgb(var(--mv-muted))]/70 hover:text-[rgb(var(--mv-muted))] transition-colors duration-150"
									on:click={() => preferenceKey && toggleSidebarGroup(preferenceKey)}
									aria-expanded={!collapsed}
								>
									{group.label}
									<svg
										class="h-3 w-3 transition-transform duration-200"
										style:transform={collapsed ? 'rotate(-90deg)' : 'rotate(0deg)'}
										fill="none"
										stroke="currentColor"
										viewBox="0 0 24 24"
									>
										<path
											stroke-linecap="round"
											stroke-linejoin="round"
											stroke-width="2.5"
											d="M19 9l-7 7-7-7"
										/>
									</svg>
								</button>
							{:else}
								<div
									class="mb-2 px-3 text-[11px] font-bold uppercase tracking-widest text-[rgb(var(--mv-muted))]/70"
								>
									{group.label}
								</div>
							{/if}
						{/if}
						{#if !collapsed}
							<div class="space-y-0.5" transition:slide={{ duration: 200 }}>
								{#each group.items as item (item.href)}
									{@const isActive = isSidebarNavigationItemActive(item, $page.url)}
									<a
										href={sidebarNavigationHref(item, $viewPreferences)}
										class={`group relative flex items-center gap-3 rounded-xl px-4 py-2.5 text-sm font-medium transition-all duration-300 ${
											isActive
												? 'bg-gradient-to-r from-[rgb(var(--mv-accent))]/10 to-transparent text-[rgb(var(--mv-text))] shadow-[inset_3px_0_0_0_rgb(var(--mv-accent-strong))]'
												: 'text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-panel-strong))]/50 hover:text-[rgb(var(--mv-text))] hover:translate-x-1'
										}`}
										aria-current={isActive ? 'page' : undefined}
									>
										{#if isActive}
											<div
												class="absolute right-4 top-1/2 h-1.5 w-1.5 -translate-y-1/2 rounded-full bg-[rgb(var(--mv-accent-strong))] shadow-[0_0_8px_rgb(var(--mv-accent))]"
											></div>
										{/if}
										{item.label}
									</a>
								{/each}
							</div>
						{/if}
					</div>
				{/each}
			</nav>

			<div class="mt-4 pt-4 border-t border-[rgb(var(--mv-border))]">
				<ContextBoundary detail="Local-first" />
			</div>
		</aside>
	{/if}

	<div class={`flex min-w-0 flex-1 flex-col ${isNotesRoute ? 'h-full' : ''}`}>
		{#if !isNotesRoute}
			<header
				class="sticky top-4 z-20 flex items-center justify-between border border-white/5 bg-[rgb(var(--mv-panel))]/60 backdrop-blur-2xl px-6 py-4 transition-all duration-300 md:mr-4 md:rounded-2xl shadow-lg mx-4 md:mx-0 mt-4 md:mt-4 mb-2"
			>
				<div class="flex items-center gap-4">
					<button
						class="flex h-9 w-9 items-center justify-center rounded-lg text-[rgb(var(--mv-muted))] transition hover:bg-[rgb(var(--mv-panel-strong))] hover:text-[rgb(var(--mv-text))] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[rgb(var(--mv-ring))]/70 md:hidden"
						on:click={() => (mobileMenuOpen = !mobileMenuOpen)}
						aria-label="Toggle menu"
						aria-expanded={mobileMenuOpen}
					>
						<svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
							{#if mobileMenuOpen}
								<path
									stroke-linecap="round"
									stroke-linejoin="round"
									stroke-width="2"
									d="M6 18L18 6M6 6l12 12"
								/>
							{:else}
								<path
									stroke-linecap="round"
									stroke-linejoin="round"
									stroke-width="2"
									d="M4 6h16M4 12h16M4 18h16"
								/>
							{/if}
						</svg>
					</button>
					<div>
						<h1 class="text-xl font-bold tracking-tight text-[rgb(var(--mv-text))]">
							{currentRoute.title}
						</h1>
						<p class="hidden text-sm font-medium text-[rgb(var(--mv-muted))] sm:block">
							{currentRoute.subtitle}
						</p>
					</div>
				</div>
				<div class="flex items-center gap-3 text-sm text-[rgb(var(--mv-muted))]">
					<NamespaceSelector />
					<div class="h-4 w-px bg-[rgb(var(--mv-border))] mx-1"></div>
					<span
						class={`flex items-center gap-1.5 rounded-full px-2.5 py-0.5 text-xs font-semibold ring-1 ring-inset ${
							connectionStatus === 'live'
								? 'bg-emerald-500/10 text-emerald-400 ring-emerald-500/20'
								: connectionStatus === 'online' || connectionStatus === 'connecting'
									? 'bg-sky-500/10 text-sky-400 ring-sky-500/20'
									: connectionStatus === 'degraded'
										? 'bg-amber-500/10 text-amber-400 ring-amber-500/20'
										: 'bg-rose-500/10 text-rose-400 ring-rose-500/20'
						}`}
						title={connectionStatus === 'live'
							? 'API and realtime sync connected'
							: connectionStatus === 'online'
								? 'API reachable; realtime reconnecting'
								: connectionStatus === 'connecting'
									? 'Checking backend connectivity'
									: connectionStatus === 'degraded'
										? 'API returning errors'
										: 'Backend unavailable'}
						aria-label={`Connection status: ${connectionStatusLabel(connectionStatus)}`}
					>
						<span
							class={`h-1.5 w-1.5 rounded-full ${
								connectionStatus === 'live'
									? 'bg-emerald-400'
									: connectionStatus === 'online' || connectionStatus === 'connecting'
										? 'bg-sky-400'
										: connectionStatus === 'degraded'
											? 'bg-amber-400'
											: 'bg-rose-400'
							}`}
						></span>
						{connectionStatusLabel(connectionStatus)}
					</span>
					{#if $pendingSyncCount > 0}
						<span
							class="flex items-center gap-1.5 rounded-full bg-amber-500/10 px-2.5 py-0.5 text-xs font-semibold text-amber-400 ring-1 ring-inset ring-amber-500/20"
						>
							<span class="animate-pulse">●</span>
							{$pendingSyncCount} pending
						</span>
					{/if}
				</div>
			</header>

			<ApiHealthBanner {online} />
			{#if $keychainStore.state === 'sealed'}
				<div
					class="mx-4 mt-3 rounded-xl border border-rose-500/35 bg-rose-500/10 px-4 py-3 text-sm text-rose-100"
				>
					Vault sealed. API operations are paused until you unseal from
					<a href="/settings/keychain" class="font-semibold underline underline-offset-2"
						>Settings -> Keychain</a
					>.
				</div>
			{:else if $keychainStore.degradedSecurity}
				<div
					class="mx-4 mt-3 rounded-xl border border-amber-500/35 bg-amber-500/10 px-4 py-3 text-sm text-amber-100"
				>
					Degraded security mode active: passphrase fallback is in use because hardware-backed key
					storage is unavailable.
				</div>
			{/if}
			<FavoritesBar />
		{/if}

		<!-- Mobile slide-out menu -->
		{#if mobileMenuOpen && !isNotesRoute}
			<div class="fixed inset-0 z-30 md:hidden" role="presentation">
				<div
					class="absolute inset-0 bg-black/60 backdrop-blur-sm transition-opacity"
					on:click={closeMobileMenu}
					role="button"
					tabindex="0"
					aria-label="Close menu"
					on:keydown={(e) => e.key === 'Escape' && closeMobileMenu()}
				></div>
				<nav
					class="absolute left-0 top-0 flex h-full w-[280px] flex-col border-r border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-bg))] p-4 shadow-2xl transition-transform"
				>
					<div class="flex items-center gap-3 px-2 py-3 mb-6">
						<div
							class="flex h-10 w-10 items-center justify-center rounded-xl bg-gradient-to-br from-[rgb(var(--mv-accent))] to-[rgb(var(--mv-accent-strong))] text-white shadow-lg"
						>
							<span class="font-bold">MV</span>
						</div>
						<div class="flex flex-col">
							<span class="font-bold text-lg leading-tight">MindVault</span>
							<span
								class="text-[10px] uppercase tracking-wider font-semibold text-[rgb(var(--mv-muted))]"
								>Personal Vault</span
							>
						</div>
					</div>
					<div class="flex-1 overflow-y-auto space-y-6 pr-2">
						{#each sidebarNavigationGroups as group}
							{@const mobilePreferenceKey = group.preferenceKey ?? group.label}
							{@const mobileCollapsed =
								group.collapsible && mobilePreferenceKey
									? isSidebarGroupCollapsed($viewPreferences.sidebarCollapsed, mobilePreferenceKey)
									: false}
							<div>
								{#if group.label}
									{#if group.collapsible}
										<button
											class="flex w-full items-center justify-between mb-2 px-3 text-[11px] font-bold uppercase tracking-widest text-[rgb(var(--mv-muted))]/70 hover:text-[rgb(var(--mv-muted))] transition-colors duration-150"
											on:click={() =>
												mobilePreferenceKey && toggleSidebarGroup(mobilePreferenceKey)}
											aria-expanded={!mobileCollapsed}
										>
											{group.label}
											<svg
												class="h-3 w-3 transition-transform duration-200"
												style:transform={mobileCollapsed ? 'rotate(-90deg)' : 'rotate(0deg)'}
												fill="none"
												stroke="currentColor"
												viewBox="0 0 24 24"
											>
												<path
													stroke-linecap="round"
													stroke-linejoin="round"
													stroke-width="2.5"
													d="M19 9l-7 7-7-7"
												/>
											</svg>
										</button>
									{:else}
										<div
											class="mb-2 px-3 text-[11px] font-bold uppercase tracking-widest text-[rgb(var(--mv-muted))]/70"
										>
											{group.label}
										</div>
									{/if}
								{/if}
								{#if !mobileCollapsed}
									<div class="space-y-0.5" transition:slide={{ duration: 200 }}>
										{#each group.items as item (item.href)}
											{@const mobileActive = isSidebarNavigationItemActive(item, $page.url)}
											<a
												href={sidebarNavigationHref(item, $viewPreferences)}
												class={`block rounded-lg px-3 py-2 text-sm font-medium transition-colors ${
													mobileActive
														? 'bg-[rgb(var(--mv-accent))]/10 text-[rgb(var(--mv-accent-strong))]'
														: 'text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-panel-strong))]/50 hover:text-[rgb(var(--mv-text))]'
												}`}
												aria-current={mobileActive ? 'page' : undefined}
												on:click={closeMobileMenu}
											>
												{item.label}
											</a>
										{/each}
									</div>
								{/if}
							</div>
						{/each}
					</div>
				</nav>
			</div>
		{/if}

		<main
			class={isNotesRoute
				? 'h-full flex-1 overflow-hidden p-0'
				: 'flex-1 overflow-x-hidden p-6 pb-20 pt-4 md:pb-8'}
		>
			{#key $page.url.pathname}
				<div
					class={isNotesRoute ? 'h-full' : 'mx-auto max-w-7xl'}
					in:fly={{
						y: isNotesRoute ? 0 : 10,
						duration: isNotesRoute ? 0 : 300,
						delay: isNotesRoute ? 0 : 100
					}}
					out:fly={{ y: isNotesRoute ? 0 : -10, duration: isNotesRoute ? 0 : 200 }}
				>
					<slot />
				</div>
			{/key}
		</main>
	</div>
</div>

<ToastStack />
{#if $paletteOpen}
	{#await import('$lib/components/CommandPalette.svelte') then { default: CommandPalette }}
		<svelte:component this={CommandPalette} />
	{/await}
{/if}
{#if $focusPlannerState.open}
	{#await import('$lib/components/FocusPlannerModal.svelte') then { default: FocusPlannerModal }}
		<svelte:component this={FocusPlannerModal} />
	{/await}
{/if}
{#if !isNotesRoute}
	<QuickCaptureModal />
{/if}
<QuickSearch />
<KeyboardShortcutsModal />
{#if !isNotesRoute}
	<MobileNav />
{/if}
<UndoIndicator />
<LinkPreview />
{#if !isNotesRoute}
	<ProposalInbox />
{/if}
