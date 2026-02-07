<script lang="ts">
	import { get } from 'svelte/store';
	import { onMount } from 'svelte';
	import CalendarView from '$lib/components/CalendarView.svelte';
	import { formatCalendarDate, importIcal } from '$lib/api/calendar';
	import { pushToast } from '$lib/stores/toast';
	import type { CalendarItem } from '$lib/api/calendar';
	import { activeNamespace } from '$lib/stores/namespace';
	import {
		applyTimeBlocks,
		suggestTimeBlocks,
		type TimeBlockSuggestion
	} from '$lib/api/time-blocks';
	import SavedViewSelector from '$lib/components/SavedViewSelector.svelte';
	import { loadSavedViews, setActiveSavedView } from '$lib/stores/saved-views';
	import { goto } from '$app/navigation';
	import type { SavedView } from '$lib/api/saved-views';

	let icalInput: HTMLInputElement | null = null;
	let calendarViewRef: CalendarView | null = null;
	let plannerDate = new Date();
	let workdayStartHour = 9;
	let workdayEndHour = 18;
	let defaultBlockMinutes = 45;
	let timeBlockLimit = 6;
	let blockSuggestions: TimeBlockSuggestion[] = [];
	let suggestingBlocks = false;
	let applyingBlocks = false;

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

	function handleDateChange(event: CustomEvent<{ date: Date }>) {
		plannerDate = event.detail.date;
	}

	function formatSuggestionWindow(start: string, end: string): string {
		const startDate = new Date(start);
		const endDate = new Date(end);
		return `${startDate.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })} - ${endDate.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}`;
	}

	async function suggestFocusBlocks() {
		if (suggestingBlocks) return;
		suggestingBlocks = true;
		try {
			const namespace = get(activeNamespace);
			const suggestions = await suggestTimeBlocks({
				date: formatCalendarDate(plannerDate),
				namespace,
				limit: timeBlockLimit,
				workdayStartHour,
				workdayEndHour,
				defaultBlockMinutes
			});
			blockSuggestions = suggestions;
			if (suggestions.length === 0) {
				pushToast('No open slots found for suggested blocks.', 'info');
			} else {
				pushToast(`Prepared ${suggestions.length} focus block suggestions.`, 'success');
			}
		} catch {
			pushToast('Failed to generate focus block suggestions.', 'danger');
		} finally {
			suggestingBlocks = false;
		}
	}

	async function applySuggestedBlocks() {
		if (applyingBlocks || blockSuggestions.length === 0) return;
		applyingBlocks = true;
		try {
			const namespace = get(activeNamespace);
			const result = await applyTimeBlocks(blockSuggestions, { namespace });
			pushToast(`Created ${result.created} focus blocks on calendar.`, 'success');
			blockSuggestions = [];
			await calendarViewRef?.reload();
		} catch {
			pushToast('Failed to apply focus block suggestions.', 'danger');
		} finally {
			applyingBlocks = false;
		}
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

	<div class="rounded-xl border border-slate-800/70 bg-slate-900/40 p-4">
		<div class="flex flex-wrap items-center justify-between gap-3">
			<div>
				<h3 class="text-sm font-semibold text-white">AI Time-Blocking</h3>
				<p class="text-xs text-slate-400">Suggest focus blocks from prioritized tasks and available calendar space.</p>
			</div>
			<button
				class="rounded-lg border border-emerald-500/30 px-3 py-2 text-xs text-emerald-300 hover:bg-emerald-500/10 disabled:opacity-50"
				on:click={suggestFocusBlocks}
				disabled={suggestingBlocks || applyingBlocks}
			>
				{suggestingBlocks ? 'Suggesting...' : 'Suggest focus blocks'}
			</button>
		</div>

		<div class="mt-3 grid grid-cols-2 gap-2 md:grid-cols-4">
			<label class="text-[11px] text-slate-400">
				Start hour
				<input
					type="number"
					min="0"
					max="23"
					class="mt-1 w-full rounded-lg border border-slate-800 bg-slate-950 px-2 py-1.5 text-xs text-white"
					bind:value={workdayStartHour}
				/>
			</label>
			<label class="text-[11px] text-slate-400">
				End hour
				<input
					type="number"
					min="1"
					max="24"
					class="mt-1 w-full rounded-lg border border-slate-800 bg-slate-950 px-2 py-1.5 text-xs text-white"
					bind:value={workdayEndHour}
				/>
			</label>
			<label class="text-[11px] text-slate-400">
				Default block (min)
				<input
					type="number"
					min="15"
					max="180"
					step="5"
					class="mt-1 w-full rounded-lg border border-slate-800 bg-slate-950 px-2 py-1.5 text-xs text-white"
					bind:value={defaultBlockMinutes}
				/>
			</label>
			<label class="text-[11px] text-slate-400">
				Max suggestions
				<input
					type="number"
					min="1"
					max="12"
					class="mt-1 w-full rounded-lg border border-slate-800 bg-slate-950 px-2 py-1.5 text-xs text-white"
					bind:value={timeBlockLimit}
				/>
			</label>
		</div>

		{#if blockSuggestions.length > 0}
			<div class="mt-3 rounded-lg border border-slate-800 bg-slate-950/50 p-3">
				<div class="flex items-center justify-between gap-2">
					<p class="text-xs text-slate-300">Suggestions for {formatCalendarDate(plannerDate)}</p>
					<div class="flex items-center gap-2">
						<button
							class="rounded-lg border border-slate-700 px-2 py-1 text-[11px] text-slate-300 hover:bg-slate-800"
							on:click={() => {
								blockSuggestions = [];
							}}
						>
							Clear
						</button>
						<button
							class="rounded-lg bg-sky-500 px-2.5 py-1 text-[11px] font-semibold text-white hover:bg-sky-400 disabled:opacity-50"
							on:click={applySuggestedBlocks}
							disabled={applyingBlocks}
						>
							{applyingBlocks ? 'Applying...' : `Apply ${blockSuggestions.length} blocks`}
						</button>
					</div>
				</div>
				<ul class="mt-2 space-y-1.5">
					{#each blockSuggestions as block (block.taskId + block.start)}
						<li class="rounded-md border border-slate-800 px-2 py-1.5 text-[11px]">
							<p class="font-medium text-slate-200">{block.taskTitle}</p>
							<p class="text-slate-400">{formatSuggestionWindow(block.start, block.end)} • {block.durationMinutes}m</p>
							{#if block.reason}
								<p class="text-slate-500">{block.reason}</p>
							{/if}
						</li>
					{/each}
				</ul>
			</div>
		{/if}
	</div>

	<div class="calendar-wrapper rounded-xl border border-slate-800/60 overflow-hidden" style="min-height: 600px;">
		<CalendarView bind:this={calendarViewRef} on:itemClick={handleItemClick} on:dateChange={handleDateChange} />
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
