<script lang="ts">
	import { onMount } from 'svelte';
	import { pushToast } from '$lib/stores/toast';
	import {
		listPlugins,
		listHookPoints,
		installPlugin,
		uninstallPlugin,
		reloadPlugins,
		type PluginSummary,
		type HookPointInfo
	} from '$lib/api/plugins';

	let plugins: PluginSummary[] = [];
	let hookPoints: HookPointInfo[] = [];
	let loading = true;
	let installing = false;
	let showInstallForm = false;
	let manifestText = '';
	let wasmFileInput: HTMLInputElement;

	async function loadData() {
		loading = true;
		try {
			const [pluginRes, hookRes] = await Promise.all([listPlugins(), listHookPoints()]);
			plugins = pluginRes.plugins;
			hookPoints = hookRes.hooks;
		} catch {
			pushToast('Failed to load plugin data', 'danger');
		} finally {
			loading = false;
		}
	}

	onMount(loadData);

	async function handleInstall() {
		const files = wasmFileInput?.files;
		if (!manifestText.trim() || !files?.length) {
			pushToast('Provide both a manifest and a WASM file', 'warning');
			return;
		}
		installing = true;
		try {
			const result = await installPlugin(manifestText, files[0]);
			pushToast(`Plugin "${result.name}" installed`, 'success');
			showInstallForm = false;
			manifestText = '';
			await loadData();
		} catch (e: any) {
			pushToast(e.message ?? 'Install failed', 'danger');
		} finally {
			installing = false;
		}
	}

	async function handleUninstall(name: string) {
		try {
			await uninstallPlugin(name);
			pushToast(`Plugin "${name}" uninstalled`, 'success');
			await loadData();
		} catch (e: any) {
			pushToast(e.message ?? 'Uninstall failed', 'danger');
		}
	}

	async function handleReload() {
		try {
			const result = await reloadPlugins();
			pushToast(`Reloaded ${result.count} plugin(s)`, 'success');
			await loadData();
		} catch (e: any) {
			pushToast(e.message ?? 'Reload failed', 'danger');
		}
	}
</script>

<div class="mx-auto max-w-4xl space-y-6 p-6">
	<div class="flex items-center justify-between">
		<div>
			<h1 class="text-2xl font-bold text-[rgb(var(--mv-text))]">Plugins</h1>
			<p class="text-sm text-[rgb(var(--mv-muted))]">
				WASM-sandboxed extensions that hook into vault events.
			</p>
		</div>
		<div class="flex gap-2">
			<button
				class="rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))] px-3 py-1.5 text-sm text-[rgb(var(--mv-text))] hover:bg-[rgb(var(--mv-hover))]"
				onclick={() => handleReload()}
			>
				Reload
			</button>
			<button
				class="rounded bg-blue-600 px-3 py-1.5 text-sm text-white hover:bg-blue-700"
				onclick={() => (showInstallForm = !showInstallForm)}
			>
				{showInstallForm ? 'Cancel' : 'Install Plugin'}
			</button>
		</div>
	</div>

	<!-- Install form -->
	{#if showInstallForm}
		<div class="rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))] p-4 space-y-3">
			<h3 class="font-medium text-[rgb(var(--mv-text))]">Install Plugin</h3>
			<div>
				<label class="block text-xs text-[rgb(var(--mv-muted))] mb-1" for="manifest-input">Manifest JSON</label>
				<textarea
					id="manifest-input"
					bind:value={manifestText}
					rows={6}
					class="w-full rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-bg))] p-2 font-mono text-xs text-[rgb(var(--mv-text))]"
					placeholder={'{"id": "my-plugin", "name": "My Plugin", "version": "0.1.0", ...}'}
				></textarea>
			</div>
			<div>
				<label class="block text-xs text-[rgb(var(--mv-muted))] mb-1" for="wasm-input">WASM Module</label>
				<input
					id="wasm-input"
					type="file"
					accept=".wasm"
					bind:this={wasmFileInput}
					class="text-sm text-[rgb(var(--mv-text))]"
				/>
			</div>
			<button
				class="rounded bg-blue-600 px-3 py-1.5 text-sm text-white hover:bg-blue-700 disabled:opacity-50"
				disabled={installing}
				onclick={() => handleInstall()}
			>
				{installing ? 'Installing...' : 'Upload & Install'}
			</button>
		</div>
	{/if}

	{#if loading}
		<p class="text-[rgb(var(--mv-muted))]">Loading...</p>
	{:else}
		<!-- Installed plugins -->
		<div class="space-y-3">
			<h2 class="text-lg font-semibold text-[rgb(var(--mv-text))]">Installed ({plugins.length})</h2>

			{#if plugins.length === 0}
				<div class="rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))] p-6 text-center">
					<p class="text-[rgb(var(--mv-muted))]">No plugins installed yet.</p>
					<p class="mt-1 text-xs text-[rgb(var(--mv-muted))]">
						Use the Install button above to upload a WASM plugin, or load via the CLI.
					</p>
				</div>
			{:else}
				{#each plugins as plugin (plugin.id)}
					<div class="rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))] p-4">
						<div class="flex items-start justify-between">
							<div>
								<h3 class="font-medium text-[rgb(var(--mv-text))]">{plugin.name}</h3>
								<div class="mt-0.5 text-xs text-[rgb(var(--mv-muted))]">
									v{plugin.version}
									{#if plugin.author} &middot; {plugin.author}{/if}
								</div>
								{#if plugin.description}
									<p class="mt-1 text-sm text-[rgb(var(--mv-muted))]">{plugin.description}</p>
								{/if}
							</div>
							<div class="flex items-center gap-2">
								{#if (plugin.status ?? 'installed') === 'loaded'}
									<span class="rounded bg-green-500/20 px-2 py-0.5 text-xs text-green-400">Active</span>
								{:else}
									<span class="rounded bg-slate-600/30 px-2 py-0.5 text-xs text-slate-300">Installed</span>
								{/if}
								<button
									class="rounded border border-red-500/30 px-2 py-0.5 text-xs text-red-400 hover:bg-red-500/10"
									onclick={() => handleUninstall(plugin.id)}
								>
									Uninstall
								</button>
							</div>
						</div>
						{#if plugin.hooks.length > 0}
							<div class="mt-2 flex flex-wrap gap-1">
								{#each plugin.hooks as hook}
									<span class="rounded bg-[rgb(var(--mv-hover))] px-1.5 py-0.5 text-xs text-[rgb(var(--mv-muted))]">{hook}</span>
								{/each}
							</div>
						{/if}
					</div>
				{/each}
			{/if}
		</div>

		<!-- Hook points reference -->
		<div class="space-y-3">
			<h2 class="text-lg font-semibold text-[rgb(var(--mv-text))]">Available Hook Points</h2>
			<div class="grid grid-cols-1 gap-2 sm:grid-cols-2">
				{#each hookPoints as hook (hook.name)}
					<div class="rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))] p-3">
						<div class="font-mono text-sm text-[rgb(var(--mv-text))]">{hook.name}</div>
						<div class="mt-0.5 text-xs text-[rgb(var(--mv-muted))]">{hook.description}</div>
					</div>
				{/each}
			</div>
		</div>
	{/if}
</div>
