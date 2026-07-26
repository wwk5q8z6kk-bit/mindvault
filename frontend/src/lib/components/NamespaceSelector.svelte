<script lang="ts">
	import { activeNamespace, availableNamespaces } from '$lib/stores/namespace';
	import { loadTasks } from '$lib/stores/tasks';
	import { loadNotes } from '$lib/stores/notes';

	async function handleChange(event: Event) {
		const value = (event.currentTarget as HTMLSelectElement).value;
		activeNamespace.set(value || null);
		await Promise.all([loadTasks(), loadNotes()]);
	}
</script>

<select
	class="rounded-lg border border-slate-700 bg-slate-800/60 px-2 py-1 text-[10px] text-slate-300 outline-none"
	value={$activeNamespace ?? ''}
	on:change={handleChange}
	aria-label="Namespace filter"
	title="Filter records by namespace"
>
	<option value="">All namespaces</option>
	{#each $availableNamespaces as ns (ns)}
		<option value={ns}>{ns}</option>
	{/each}
</select>
