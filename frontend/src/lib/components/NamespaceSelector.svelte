<script lang="ts">
	import { ListFilter } from '@lucide/svelte';
	import { activeNamespace, availableNamespaces } from '$lib/stores/namespace';
	import { loadTasks } from '$lib/stores/tasks';
	import { loadNotes } from '$lib/stores/notes';

	export let id = 'namespace-filter';

	$: selectableNamespaces = Array.from(
		new Set([
			...$availableNamespaces,
			...($activeNamespace && !$availableNamespaces.includes($activeNamespace)
				? [$activeNamespace]
				: [])
		])
	);
	$: hasNamedNamespaces = selectableNamespaces.length > 0;
	$: helpId = `${id}-help`;

	async function handleChange(event: Event) {
		const value = (event.currentTarget as HTMLSelectElement).value;
		activeNamespace.set(value || null);
		await Promise.all([loadTasks(), loadNotes()]);
	}
</script>

<div
	class="namespace-filter"
	class:inactive={!hasNamedNamespaces}
	title={hasNamedNamespaces
		? 'Filter records within this Personal Vault'
		: 'All records are shown; no named namespaces are available'}
>
	<ListFilter size={14} strokeWidth={1.9} aria-hidden="true" />
	<label for={id}>Records</label>
	<select
		{id}
		value={$activeNamespace ?? ''}
		on:change={handleChange}
		aria-label="Filter records by namespace"
		aria-describedby={helpId}
		disabled={!hasNamedNamespaces}
	>
		<option value="">All records</option>
		{#each selectableNamespaces as namespace (namespace)}
			<option value={namespace}>{namespace}</option>
		{/each}
	</select>
	<span class="sr-only" id={helpId}>
		Filters visible records inside your Personal Vault. It does not change the current vault.
	</span>
</div>

<style>
	.namespace-filter {
		display: inline-flex;
		min-height: 36px;
		align-items: center;
		gap: 7px;
		padding: 3px 5px 3px 10px;
		border: 1px solid rgb(var(--mv-border) / 0.72);
		border-radius: 10px;
		background: rgb(var(--mv-panel-strong) / 0.52);
		color: rgb(var(--mv-muted));
		transition:
			border-color var(--mv-transition-fast),
			background var(--mv-transition-fast);
	}

	.namespace-filter:not(.inactive):hover {
		border-color: rgb(var(--mv-border));
		background: rgb(var(--mv-panel-strong) / 0.72);
	}

	.namespace-filter:focus-within {
		border-color: rgb(var(--mv-ring) / 0.78);
		box-shadow: 0 0 0 3px rgb(var(--mv-ring) / 0.18);
	}

	label {
		color: rgb(var(--mv-muted) / 0.78);
		font-size: 10px;
		font-weight: 700;
		letter-spacing: 0.07em;
		text-transform: uppercase;
	}

	select {
		min-width: 82px;
		max-width: 136px;
		border: 0;
		border-radius: 7px;
		background: transparent;
		padding: 5px 22px 5px 5px;
		color: rgb(var(--mv-text) / 0.82);
		font: inherit;
		font-size: 11px;
		font-weight: 650;
		outline: none;
		text-overflow: ellipsis;
	}

	select:disabled {
		cursor: default;
		opacity: 0.65;
	}

	.sr-only {
		position: absolute;
		width: 1px;
		height: 1px;
		overflow: hidden;
		margin: -1px;
		padding: 0;
		border: 0;
		clip: rect(0, 0, 0, 0);
		white-space: nowrap;
	}

	@media (max-width: 700px) {
		label {
			position: absolute;
			width: 1px;
			height: 1px;
			overflow: hidden;
			margin: -1px;
			padding: 0;
			border: 0;
			clip: rect(0, 0, 0, 0);
			white-space: nowrap;
		}
	}
</style>
