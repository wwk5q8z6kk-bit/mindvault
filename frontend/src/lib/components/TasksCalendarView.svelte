<script lang="ts">
	import { get } from 'svelte/store';
	import { onMount } from 'svelte';
	import CalendarView from '$lib/components/CalendarView.svelte';
	import { formatCalendarDate, importIcal, downloadIcal } from '$lib/api/calendar';
	import { pushToast } from '$lib/stores/toast';
	import type { CalendarItem } from '$lib/api/calendar';
	import { activeNamespace } from '$lib/stores/namespace';
	import {
		applyRescheduledTimeBlocks,
		applyTimeBlocks,
		suggestRescheduledTimeBlocks,
		suggestTimeBlocks,
		type TimeBlockRescheduleSuggestion,
		type TimeBlockSuggestion
	} from '$lib/api/time-blocks';
	import SavedViewSelector from '$lib/components/SavedViewSelector.svelte';
	import { loadSavedViews, setActiveSavedView } from '$lib/stores/saved-views';
	import { goto } from '$app/navigation';
	import type { SavedView } from '$lib/api/saved-views';

	let icalInput: HTMLInputElement | null = null;
	let calendarViewRef: CalendarView | null = null;
	let plannerDate = new Date();

	// iCal import options
	let showImportOptions = false;
	let importFile: File | null = null;
	let importDuplicateStrategy: 'skip' | 'update' | 'duplicate' = 'skip';
	let importTagsInput = '';
	let importing = false;
	let workdayStartHour = 9;
	let workdayEndHour = 18;
	let defaultBlockMinutes = 45;
	let timeBlockLimit = 6;
	let blockSuggestions: TimeBlockSuggestion[] = [];
	let reflowSuggestions: TimeBlockRescheduleSuggestion[] = [];
	let exporting = false;
	let suggestingBlocks = false;
	let suggestingReflow = false;
	let applyingBlocks = false;
	let applyingReflow = false;

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
			goto('/tasks?view=list');
			return;
		}
		if (view.view_type === 'kanban') {
			goto('/tasks?view=kanban');
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

	async function suggestReflowBlocks() {
		if (suggestingReflow) return;
		suggestingReflow = true;
		try {
			const namespace = get(activeNamespace);
			const suggestions = await suggestRescheduledTimeBlocks({
				date: formatCalendarDate(plannerDate),
				namespace,
				limit: timeBlockLimit,
				workdayStartHour,
				workdayEndHour,
				defaultBlockMinutes
			});
			reflowSuggestions = suggestions;
			if (suggestions.length === 0) {
				pushToast('No rescheduling opportunities detected.', 'info');
			} else {
				pushToast(`Prepared ${suggestions.length} reschedule suggestions.`, 'success');
			}
		} catch {
			pushToast('Failed to prepare reschedule suggestions.', 'danger');
		} finally {
			suggestingReflow = false;
		}
	}

	async function applyReflowSuggestions() {
		if (applyingReflow || reflowSuggestions.length === 0) return;
		applyingReflow = true;
		try {
			const result = await applyRescheduledTimeBlocks(reflowSuggestions);
			pushToast(`Rescheduled ${result.updated} focus blocks.`, 'success');
			reflowSuggestions = [];
			await calendarViewRef?.reload();
		} catch {
			pushToast('Failed to apply reschedule suggestions.', 'danger');
		} finally {
			applyingReflow = false;
		}
	}

	async function handleExportIcal() {
		if (exporting) return;
		exporting = true;
		try {
			const namespace = get(activeNamespace);
			await downloadIcal({ namespace: namespace || undefined });
			pushToast('Calendar exported as .ics file', 'success');
		} catch {
			pushToast('iCal export failed', 'danger');
		} finally {
			exporting = false;
		}
	}

	function handleIcalFileSelect(event: Event) {
		const input = event.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		if (!file) return;
		importFile = file;
		importDuplicateStrategy = 'skip';
		importTagsInput = '';
		showImportOptions = true;
	}

	function cancelImport() {
		showImportOptions = false;
		importFile = null;
		if (icalInput) icalInput.value = '';
	}

	async function confirmIcalImport() {
		if (!importFile || importing) return;
		importing = true;
		try {
			const tags = importTagsInput
				.split(',')
				.map((t) => t.trim())
				.filter((t) => t.length > 0);
			const namespace = get(activeNamespace);
			const result = await importIcal(importFile, {
				namespace: namespace || undefined,
				duplicate_strategy: importDuplicateStrategy,
				tags: tags.length > 0 ? tags : undefined
			});
			pushToast(`Imported ${result.imported_count} events (${result.skipped_count} skipped)`, 'success');
			showImportOptions = false;
			importFile = null;
			await calendarViewRef?.reload();
		} catch {
			pushToast('iCal import failed', 'danger');
		} finally {
			importing = false;
			if (icalInput) icalInput.value = '';
		}
	}
</script>

<div class="flex flex-col gap-4">
	<div class="flex items-center justify-between">
		<p class="text-xs text-[rgb(var(--mv-muted))]">View events and tasks from the server. Export/import iCal.</p>
		<div class="flex gap-2">
			<button
				class="rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))] px-3 py-2 text-xs text-[rgb(var(--mv-text))] transition hover:bg-[rgb(var(--mv-panel-strong))]/80 disabled:opacity-50"
				on:click={handleExportIcal}
				disabled={exporting}
			>
				{exporting ? 'Exporting...' : 'Export .ics'}
			</button>
			<label class="cursor-pointer rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))] px-3 py-2 text-xs text-[rgb(var(--mv-text))] transition hover:bg-[rgb(var(--mv-panel-strong))]/80">
				Import .ics
				<input type="file" accept=".ics,.ical" class="hidden" bind:this={icalInput} on:change={handleIcalFileSelect} />
			</label>
		</div>
	</div>

	<div>
		<SavedViewSelector currentView="calendar" on:apply={(event) => applySavedView(event.detail)} />
	</div>

	<div class="rounded-xl border border-[rgb(var(--mv-border))]/70 bg-[rgb(var(--mv-panel))]/40 p-4">
		<div class="flex flex-wrap items-center justify-between gap-3">
			<div>
				<h3 class="text-sm font-semibold text-[rgb(var(--mv-text))]">AI Time-Blocking</h3>
				<p class="text-xs text-[rgb(var(--mv-muted))]">Suggest focus blocks from prioritized tasks and available calendar space.</p>
			</div>
			<div class="flex flex-wrap items-center gap-2">
				<button
					class="rounded-lg border border-emerald-500/30 px-3 py-2 text-xs text-emerald-300 hover:bg-emerald-500/10 disabled:opacity-50"
					on:click={suggestFocusBlocks}
					disabled={suggestingBlocks || applyingBlocks}
				>
					{suggestingBlocks ? 'Suggesting...' : 'Suggest focus blocks'}
				</button>
				<button
					class="rounded-lg border border-amber-500/30 px-3 py-2 text-xs text-amber-300 hover:bg-amber-500/10 disabled:opacity-50"
					on:click={suggestReflowBlocks}
					disabled={suggestingReflow || applyingReflow}
				>
					{suggestingReflow ? 'Analyzing...' : 'Suggest reflow'}
				</button>
			</div>
		</div>

		<div class="mt-3 grid grid-cols-2 gap-2 md:grid-cols-4">
			<label class="text-[11px] text-[rgb(var(--mv-muted))]">
				Start hour
				<input
					type="number"
					min="0"
					max="23"
					class="mt-1 w-full rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))] px-2 py-1.5 text-xs text-[rgb(var(--mv-text))]"
					bind:value={workdayStartHour}
				/>
			</label>
			<label class="text-[11px] text-[rgb(var(--mv-muted))]">
				End hour
				<input
					type="number"
					min="1"
					max="24"
					class="mt-1 w-full rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))] px-2 py-1.5 text-xs text-[rgb(var(--mv-text))]"
					bind:value={workdayEndHour}
				/>
			</label>
			<label class="text-[11px] text-[rgb(var(--mv-muted))]">
				Default block (min)
				<input
					type="number"
					min="15"
					max="180"
					step="5"
					class="mt-1 w-full rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))] px-2 py-1.5 text-xs text-[rgb(var(--mv-text))]"
					bind:value={defaultBlockMinutes}
				/>
			</label>
			<label class="text-[11px] text-[rgb(var(--mv-muted))]">
				Max suggestions
				<input
					type="number"
					min="1"
					max="12"
					class="mt-1 w-full rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))] px-2 py-1.5 text-xs text-[rgb(var(--mv-text))]"
					bind:value={timeBlockLimit}
				/>
			</label>
		</div>

		{#if blockSuggestions.length > 0}
			<div class="mt-3 rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))]/50 p-3">
				<div class="flex items-center justify-between gap-2">
					<p class="text-xs text-[rgb(var(--mv-text))]">Suggestions for {formatCalendarDate(plannerDate)}</p>
					<div class="flex items-center gap-2">
						<button
							class="rounded-lg border border-[rgb(var(--mv-border))] px-2 py-1 text-[11px] text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-panel-strong))]"
							on:click={() => { blockSuggestions = []; }}
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
						<li class="rounded-md border border-[rgb(var(--mv-border))] px-2 py-1.5 text-[11px]">
							<p class="font-medium text-[rgb(var(--mv-text))]">{block.taskTitle}</p>
							<p class="text-[rgb(var(--mv-muted))]">{formatSuggestionWindow(block.start, block.end)} • {block.durationMinutes}m</p>
							{#if block.reason}
								<p class="text-[rgb(var(--mv-muted))]/60">{block.reason}</p>
							{/if}
						</li>
					{/each}
				</ul>
			</div>
		{/if}

		{#if reflowSuggestions.length > 0}
			<div class="mt-3 rounded-lg border border-amber-500/20 bg-amber-500/5 p-3">
				<div class="flex items-center justify-between gap-2">
					<p class="text-xs text-amber-200">Reflow suggestions for existing focus blocks</p>
					<div class="flex items-center gap-2">
						<button
							class="rounded-lg border border-[rgb(var(--mv-border))] px-2 py-1 text-[11px] text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-panel-strong))]"
							on:click={() => { reflowSuggestions = []; }}
						>
							Clear
						</button>
						<button
							class="rounded-lg bg-amber-500 px-2.5 py-1 text-[11px] font-semibold text-white hover:bg-amber-400 disabled:opacity-50"
							on:click={applyReflowSuggestions}
							disabled={applyingReflow}
						>
							{applyingReflow ? 'Applying...' : `Apply ${reflowSuggestions.length} reflows`}
						</button>
					</div>
				</div>
				<ul class="mt-2 space-y-1.5">
					{#each reflowSuggestions as block (block.eventId + block.start)}
						<li class="rounded-md border border-amber-500/20 px-2 py-1.5 text-[11px]">
							<p class="font-medium text-[rgb(var(--mv-text))]">{block.taskTitle}</p>
							<p class="text-[rgb(var(--mv-muted))]">
								{formatSuggestionWindow(block.previousStart, block.previousEnd)} -> {formatSuggestionWindow(block.start, block.end)}
							</p>
						</li>
					{/each}
				</ul>
			</div>
		{/if}
	</div>

	<div class="calendar-wrapper rounded-xl border border-[rgb(var(--mv-border))]/60 overflow-hidden" style="min-height: 600px;">
		<CalendarView bind:this={calendarViewRef} on:itemClick={handleItemClick} on:dateChange={handleDateChange} />
	</div>
</div>

<!-- iCal Import Options Modal -->
{#if showImportOptions}
	<div class="fixed inset-0 z-50 flex items-center justify-center" role="presentation">
		<div
			class="absolute inset-0 bg-black/50"
			on:click={cancelImport}
			on:keydown={(e) => e.key === 'Escape' && cancelImport()}
			role="button"
			tabindex="-1"
			aria-label="Close"
		></div>
		<div class="relative z-10 w-full max-w-md rounded-2xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))] p-5 shadow-2xl">
			<h3 class="text-sm font-semibold text-[rgb(var(--mv-text))]">Import iCal</h3>
			<p class="mt-1 text-[11px] text-[rgb(var(--mv-muted))]">
				{importFile?.name ?? 'Selected file'} ({Math.round((importFile?.size ?? 0) / 1024)}KB)
			</p>

			<div class="mt-4 space-y-3">
				<div>
					<label class="text-[10px] uppercase tracking-wide text-[rgb(var(--mv-muted))]/60" for="ical-dup-strategy">
						Duplicate handling
					</label>
					<select
						id="ical-dup-strategy"
						class="mt-1 w-full rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))] px-3 py-2 text-sm text-[rgb(var(--mv-text))]"
						bind:value={importDuplicateStrategy}
					>
						<option value="skip">Skip duplicates</option>
						<option value="update">Update existing</option>
						<option value="duplicate">Create duplicates</option>
					</select>
				</div>

				<div>
					<label class="text-[10px] uppercase tracking-wide text-[rgb(var(--mv-muted))]/60" for="ical-tags">
						Tags (comma-separated)
					</label>
					<input
						id="ical-tags"
						type="text"
						class="mt-1 w-full rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))] px-3 py-2 text-sm text-[rgb(var(--mv-text))] placeholder-[rgb(var(--mv-muted))]/40"
						placeholder="e.g. imported, work"
						bind:value={importTagsInput}
					/>
				</div>
			</div>

			<div class="mt-4 flex justify-end gap-2">
				<button
					class="rounded-lg border border-[rgb(var(--mv-border))] px-3 py-1.5 text-xs text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-panel-strong))]"
					on:click={cancelImport}
				>
					Cancel
				</button>
				<button
					class="rounded-lg bg-sky-500 px-3 py-1.5 text-xs font-semibold text-white hover:bg-sky-400 disabled:opacity-50"
					on:click={confirmIcalImport}
					disabled={importing}
				>
					{importing ? 'Importing...' : 'Import'}
				</button>
			</div>
		</div>
	</div>
{/if}

<style>
	.calendar-wrapper :global(.calendar-view) {
		--surface-color: rgb(var(--mv-panel) / 0.4);
		--border-color: rgb(var(--mv-border) / 0.6);
		--text-muted: rgb(var(--mv-muted));
		--primary-color: rgb(var(--mv-accent));
		--primary-light: rgb(var(--mv-accent) / 0.1);
		--primary-lighter: rgb(var(--mv-accent) / 0.2);
		--primary-bg: rgb(var(--mv-accent) / 0.05);
		--success-light: rgb(16 185 129 / 0.1);
		--success-lighter: rgb(16 185 129 / 0.2);
		--bg-muted: rgb(var(--mv-bg) / 0.3);
		background: transparent;
		color: rgb(var(--mv-text));
	}
	.calendar-wrapper :global(.calendar-title) { color: rgb(var(--mv-text)); }
	.calendar-wrapper :global(.btn-nav), .calendar-wrapper :global(.btn-today), .calendar-wrapper :global(.btn-export) {
		color: rgb(var(--mv-text));
		border-color: rgb(var(--mv-border));
		background: rgb(var(--mv-panel-strong) / 0.6);
	}
	.calendar-wrapper :global(.btn-nav:hover), .calendar-wrapper :global(.btn-today:hover), .calendar-wrapper :global(.btn-export:hover) {
		background: rgb(var(--mv-panel-strong));
	}
	.calendar-wrapper :global(.view-toggle) { border-color: rgb(var(--mv-border)); }
	.calendar-wrapper :global(.view-toggle button) { color: rgb(var(--mv-muted)); }
	.calendar-wrapper :global(.view-toggle button.active) { background: rgb(var(--mv-accent)); color: white; }
	.calendar-wrapper :global(.day-number) { color: rgb(var(--mv-text)); }
	.calendar-wrapper :global(.item-title) { color: rgb(var(--mv-text)); }
	.calendar-wrapper :global(.calendar-item) { color: rgb(var(--mv-text)); }
</style>
