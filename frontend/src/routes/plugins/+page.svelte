<script lang="ts">
	import { onMount } from 'svelte';
	import { pushToast } from '$lib/stores/toast';
	import {
		listPlugins,
		listHookPoints,
		installPlugin,
		uninstallPlugin,
		reloadPlugins,
		listRuntimePlugins,
		reloadRuntimePlugin,
		unloadRuntimePlugin,
		type PluginSummary,
		type HookPointInfo,
		type RuntimePlugin
	} from '$lib/api/plugins';

	let plugins: PluginSummary[] = [];
	let runtimePlugins: RuntimePlugin[] = [];
	let hookPoints: HookPointInfo[] = [];
	let loading = true;
	let installing = false;
	let showInstallForm = false;
	let manifestText = '';
	let wasmFileInput: HTMLInputElement;
	let actionPluginId = '';

	$: runtimeMap = new Map(runtimePlugins.map((plugin) => [plugin.id, plugin]));
	$: hookUsage = hookPoints.map((hook) => ({
		...hook,
		count: runtimePlugins.filter((plugin) => plugin.hooks.includes(hook.name)).length
	}));

	async function loadData() {
		loading = true;
		try {
			const [pluginRes, runtimeRes, hookRes] = await Promise.all([
				listPlugins(),
				listRuntimePlugins(),
				listHookPoints()
			]);
			plugins = pluginRes.plugins;
			runtimePlugins = runtimeRes.plugins;
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
		actionPluginId = name;
		try {
			await uninstallPlugin(name);
			pushToast(`Plugin "${name}" uninstalled`, 'success');
			await loadData();
		} catch (e: any) {
			pushToast(e.message ?? 'Uninstall failed', 'danger');
		} finally {
			actionPluginId = '';
		}
	}

	async function handleReloadAll() {
		try {
			const result = await reloadPlugins();
			pushToast(`Reloaded ${result.count} plugin(s)`, 'success');
			await loadData();
		} catch (e: any) {
			pushToast(e.message ?? 'Reload failed', 'danger');
		}
	}

	async function handleRuntimeReload(pluginId: string) {
		actionPluginId = pluginId;
		try {
			await reloadRuntimePlugin(pluginId);
			pushToast(`Runtime plugin "${pluginId}" reloaded`, 'success');
			await loadData();
		} catch (e: any) {
			pushToast(e.message ?? 'Runtime reload failed', 'danger');
		} finally {
			actionPluginId = '';
		}
	}

	async function handleRuntimeUnload(pluginId: string) {
		actionPluginId = pluginId;
		try {
			await unloadRuntimePlugin(pluginId);
			pushToast(`Runtime plugin "${pluginId}" unloaded`, 'success');
			await loadData();
		} catch (e: any) {
			pushToast(e.message ?? 'Runtime unload failed', 'danger');
		} finally {
			actionPluginId = '';
		}
	}

	function pluginStatus(plugin: PluginSummary): 'loaded' | 'installed' {
		return runtimeMap.has(plugin.id) ? 'loaded' : 'installed';
	}

	function formatBytes(bytes: number): string {
		if (!bytes) return '0 B';
		if (bytes < 1024) return `${bytes} B`;
		if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
		return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
	}
</script>

<div class="mx-auto max-w-6xl space-y-6 p-6">
	<div class="flex flex-wrap items-start justify-between gap-3">
		<div>
			<h1 class="text-2xl font-bold text-[rgb(var(--mv-text))]">Plugin Manager</h1>
			<p class="text-sm text-[rgb(var(--mv-muted))]">Browse installed plugins, inspect runtime hooks, and control plugin lifecycle.</p>
		</div>
		<div class="flex gap-2">
			<button
				class="rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))] px-3 py-1.5 text-xs text-[rgb(var(--mv-text))] hover:bg-[rgb(var(--mv-hover))]"
				on:click={loadData}
			>
				Refresh
			</button>
			<button
				class="rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))] px-3 py-1.5 text-xs text-[rgb(var(--mv-text))] hover:bg-[rgb(var(--mv-hover))]"
				on:click={handleReloadAll}
			>
				Reload All
			</button>
			<button
				class="rounded-lg bg-sky-500 px-3 py-1.5 text-xs font-semibold text-white hover:bg-sky-400"
				on:click={() => (showInstallForm = !showInstallForm)}
			>
				{showInstallForm ? 'Cancel' : 'Install Plugin'}
			</button>
		</div>
	</div>

	<div class="grid grid-cols-2 gap-3 md:grid-cols-4">
		<div class="rounded-xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/40 p-3">
			<p class="text-[10px] uppercase tracking-wide text-[rgb(var(--mv-muted))]/70">Installed</p>
			<p class="mt-1 text-xl font-semibold text-[rgb(var(--mv-text))]">{plugins.length}</p>
		</div>
		<div class="rounded-xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/40 p-3">
			<p class="text-[10px] uppercase tracking-wide text-[rgb(var(--mv-muted))]/70">Loaded Runtime</p>
			<p class="mt-1 text-xl font-semibold text-green-300">{runtimePlugins.length}</p>
		</div>
		<div class="rounded-xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/40 p-3">
			<p class="text-[10px] uppercase tracking-wide text-[rgb(var(--mv-muted))]/70">Hook Points</p>
			<p class="mt-1 text-xl font-semibold text-[rgb(var(--mv-text))]">{hookPoints.length}</p>
		</div>
		<div class="rounded-xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/40 p-3">
			<p class="text-[10px] uppercase tracking-wide text-[rgb(var(--mv-muted))]/70">Active Hooks</p>
			<p class="mt-1 text-xl font-semibold text-violet-300">{runtimePlugins.reduce((sum, plugin) => sum + plugin.hooks.length, 0)}</p>
		</div>
	</div>

	{#if showInstallForm}
		<div class="rounded-xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/40 p-4 space-y-3">
			<h3 class="text-sm font-semibold text-[rgb(var(--mv-text))]">Install Plugin</h3>
			<div>
				<label class="mb-1 block text-xs text-[rgb(var(--mv-muted))]" for="manifest-input">Manifest JSON</label>
				<textarea
					id="manifest-input"
					bind:value={manifestText}
					rows={6}
					class="w-full rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-bg))] p-2 font-mono text-xs text-[rgb(var(--mv-text))]"
						placeholder="Paste plugin manifest JSON"
				></textarea>
			</div>
			<div>
				<label class="mb-1 block text-xs text-[rgb(var(--mv-muted))]" for="wasm-input">WASM Module</label>
				<input id="wasm-input" type="file" accept=".wasm" bind:this={wasmFileInput} class="text-sm text-[rgb(var(--mv-text))]" />
			</div>
			<button
				class="rounded bg-sky-500 px-3 py-1.5 text-xs font-semibold text-white hover:bg-sky-400 disabled:opacity-50"
				disabled={installing}
				on:click={handleInstall}
			>
				{installing ? 'Installing...' : 'Upload & Install'}
			</button>
		</div>
	{/if}

	{#if loading}
		<p class="text-[rgb(var(--mv-muted))]">Loading...</p>
	{:else}
		<div class="grid gap-4 lg:grid-cols-[2fr_1fr]">
			<div class="space-y-3">
				<h2 class="text-lg font-semibold text-[rgb(var(--mv-text))]">Plugin Browser</h2>
				{#if plugins.length === 0}
					<div class="rounded-xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/30 p-5 text-sm text-[rgb(var(--mv-muted))]">
						No plugins installed yet.
					</div>
				{:else}
					{#each plugins as plugin (plugin.id)}
						{@const runtime = runtimeMap.get(plugin.id)}
						<div class="rounded-xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/40 p-4">
							<div class="flex flex-wrap items-start justify-between gap-3">
								<div class="min-w-0 flex-1">
									<div class="flex items-center gap-2">
										<div class="flex h-8 w-8 items-center justify-center rounded-lg bg-sky-500/20 text-xs font-bold text-sky-200">
											{plugin.name.slice(0, 2).toUpperCase()}
										</div>
										<div>
											<h3 class="text-sm font-semibold text-[rgb(var(--mv-text))]">{plugin.name}</h3>
											<p class="text-xs text-[rgb(var(--mv-muted))]">v{plugin.version}{#if plugin.author} · {plugin.author}{/if}</p>
										</div>
										<span class="ml-auto rounded-full px-2 py-0.5 text-[10px] {pluginStatus(plugin) === 'loaded' ? 'bg-green-500/20 text-green-300' : 'bg-slate-500/20 text-slate-300'}">
											{pluginStatus(plugin) === 'loaded' ? 'Loaded' : 'Installed'}
										</span>
									</div>
									{#if plugin.description}
										<p class="mt-2 text-xs text-[rgb(var(--mv-muted))]">{plugin.description}</p>
									{/if}
									<div class="mt-2 flex flex-wrap gap-1">
										{#each plugin.hooks as hook}
											<span class="rounded border border-violet-500/30 bg-violet-500/10 px-1.5 py-0.5 text-[10px] text-violet-200">{hook}</span>
										{/each}
									</div>
									{#if runtime}
										<div class="mt-2 text-[10px] text-[rgb(var(--mv-muted))]/70">
											Runtime: {runtime.invocation_count} invocations · {formatBytes(runtime.wasm_size_bytes)}
										</div>
									{/if}
								</div>
								<div class="flex items-center gap-1.5">
									{#if runtime}
										<button
											class="rounded border border-sky-500/30 px-2 py-1 text-[10px] text-sky-200 hover:bg-sky-500/10 disabled:opacity-60"
											disabled={actionPluginId === plugin.id}
											on:click={() => handleRuntimeReload(plugin.id)}
										>
											Reload
										</button>
										<button
											class="rounded border border-amber-500/30 px-2 py-1 text-[10px] text-amber-200 hover:bg-amber-500/10 disabled:opacity-60"
											disabled={actionPluginId === plugin.id}
											on:click={() => handleRuntimeUnload(plugin.id)}
										>
											Unload
										</button>
									{/if}
									<button
										class="rounded border border-red-500/30 px-2 py-1 text-[10px] text-red-300 hover:bg-red-500/10 disabled:opacity-60"
										disabled={actionPluginId === plugin.id}
										on:click={() => handleUninstall(plugin.id)}
									>
										Uninstall
									</button>
								</div>
							</div>
						</div>
					{/each}
				{/if}
			</div>

			<div class="space-y-3">
				<h2 class="text-lg font-semibold text-[rgb(var(--mv-text))]">Hook Visualization</h2>
				<div class="rounded-xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/40 p-4 space-y-2">
					{#each hookUsage as hook (hook.name)}
						<div class="rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))]/30 px-3 py-2">
							<div class="flex items-center justify-between gap-2">
								<div>
									<p class="font-mono text-xs text-[rgb(var(--mv-text))]">{hook.name}</p>
									<p class="text-[10px] text-[rgb(var(--mv-muted))]">{hook.description}</p>
								</div>
								<span class="rounded-full bg-sky-500/20 px-2 py-0.5 text-[10px] text-sky-300">{hook.count}</span>
							</div>
						</div>
					{/each}
				</div>
			</div>
		</div>
	{/if}
</div>
