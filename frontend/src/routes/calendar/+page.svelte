<script lang="ts">
	import { onMount } from 'svelte';
	import CalendarView from '$lib/components/CalendarView.svelte';
	import { importIcal } from '$lib/api/calendar';
	import { pushToast } from '$lib/stores/toast';
	import type { CalendarItem } from '$lib/api/calendar';
	import SavedViewSelector from '$lib/components/SavedViewSelector.svelte';
	import { loadSavedViews, setActiveSavedView } from '$lib/stores/saved-views';
	import { goto } from '$app/navigation';
	import type { SavedView } from '$lib/api/saved-views';

	let icalInput: HTMLInputElement | null = null;

	onMount(() => {
		loadSavedViews();
	});

	function applySavedView(view: SavedView | null) {
		if (!view) {
			setActiveSavedView(null);
			return;
		}
		setActiveSavedView(view);
		if (view.view_type === 'list') {
			goto('/tasks');
			return;
		}
		if (view.view_type === 'kanban') {
			goto('/kanban');
		}
	}


	function handleItemClick(e: CustomEvent<CalendarItem>) {
		// CalendarView already handles navigation internally
	}

	async function handleIcalImport(event: Event) {
		const input = event.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		if (!file) return;
		try {
			const result = await importIcal(file);
			pushToast(`Imported ${result.imported_count} events`, 'success');
		} catch {
			pushToast('iCal import failed', 'danger');
		} finally {
			if (icalInput) icalInput.value = '';
		}
	}
</script>

<div class="flex flex-col gap-4">
	<div class="flex items-center justify-between">
		<div>
			<h2 class="text-lg font-semibold text-white">Calendar</h2>
			<p class="text-xs text-slate-400">View events and tasks from the server. Export/import iCal.</p>
		</div>
		<label class="cursor-pointer rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-slate-200 transition hover:bg-slate-700">
			Import .ics
			<input type="file" accept=".ics,.ical" class="hidden" bind:this={icalInput} on:change={handleIcalImport} />
		</label>
	</div>

	<div class="mt-3">
		<SavedViewSelector currentView="calendar" on:apply={(event) => applySavedView(event.detail)} />
	</div>

	<div class="calendar-wrapper rounded-xl border border-slate-800/60 overflow-hidden" style="min-height: 600px;">
		<CalendarView on:itemClick={handleItemClick} />
	</div>
</div>

<style>
	.calendar-wrapper :global(.calendar-view) {
		--surface-color: rgb(15 23 42 / 0.4);
		--border-color: rgb(30 41 59 / 0.6);
		--text-muted: rgb(148 163 184);
		--primary-color: rgb(14 165 233);
		--primary-light: rgb(14 165 233 / 0.1);
		--primary-lighter: rgb(14 165 233 / 0.2);
		--primary-bg: rgb(14 165 233 / 0.05);
		--success-light: rgb(16 185 129 / 0.1);
		--success-lighter: rgb(16 185 129 / 0.2);
		--bg-muted: rgb(15 23 42 / 0.3);
		background: transparent;
		color: rgb(226 232 240);
	}
	.calendar-wrapper :global(.calendar-title) { color: white; }
	.calendar-wrapper :global(.btn-nav), .calendar-wrapper :global(.btn-today), .calendar-wrapper :global(.btn-export) {
		color: rgb(203 213 225);
		border-color: rgb(51 65 85);
		background: rgb(30 41 59 / 0.6);
	}
	.calendar-wrapper :global(.btn-nav:hover), .calendar-wrapper :global(.btn-today:hover), .calendar-wrapper :global(.btn-export:hover) {
		background: rgb(51 65 85);
	}
	.calendar-wrapper :global(.view-toggle) { border-color: rgb(51 65 85); }
	.calendar-wrapper :global(.view-toggle button) { color: rgb(148 163 184); }
	.calendar-wrapper :global(.view-toggle button.active) { background: rgb(14 165 233); color: white; }
	.calendar-wrapper :global(.day-number) { color: rgb(203 213 225); }
	.calendar-wrapper :global(.item-title) { color: rgb(226 232 240); }
	.calendar-wrapper :global(.calendar-item) { color: rgb(226 232 240); }
</style>
