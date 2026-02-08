<script lang="ts">
	import { onMount } from 'svelte';
	import { pushToast } from '$lib/stores/toast';
	import { listPlugins, listHookPoints, type PluginSummary, type HookPointInfo } from '$lib/api/plugins';

	let plugins: PluginSummary[] = [];
	let hookPoints: HookPointInfo[] = [];
	let loading = true;

	onMount(async () => {
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
	});
</script>

<div class="mx-auto max-w-4xl space-y-6 p-6">
	<h1 class="text-2xl font-bold text-[rgb(var(--mv-text))]">Plugins</h1>
	<p class="text-sm text-[rgb(var(--mv-muted))]">
		WASM-sandboxed extensions that hook into vault events.
	</p>

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
						Plugins can be loaded via the CLI or configuration file.
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
							<span class="rounded bg-green-500/20 px-2 py-0.5 text-xs text-green-400">Active</span>
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
