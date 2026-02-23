<script lang="ts">
	import { createEventDispatcher, onMount } from 'svelte';
	import {
		getCalendarItems,
		getViewDateRange,
		formatCalendarDate,
		downloadIcal
	} from '$lib/api/calendar';
	import type { CalendarItem, CalendarView as ViewType } from '$lib/api/calendar';
	import { pushToast } from '$lib/stores/toast';
	import { goto } from '$app/navigation';

	export let initialDate: Date = new Date();
	export let initialView: ViewType = 'week';

	const dispatch = createEventDispatcher<{
		itemClick: CalendarItem;
		dateChange: { date: Date; view: ViewType };
	}>();

	let currentDate = initialDate;
	let view: ViewType = initialView;
	let items: CalendarItem[] = [];
	let loading = true;

	const DAYS = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
	const MONTHS = [
		'January',
		'February',
		'March',
		'April',
		'May',
		'June',
		'July',
		'August',
		'September',
		'October',
		'November',
		'December'
	];

	$: dateRange = getViewDateRange(currentDate, view);
	$: formattedTitle = getFormattedTitle(currentDate, view);
	$: weekDays = getWeekDays(dateRange.start);
	$: monthDays = getMonthDays(currentDate);

	onMount(() => {
		loadItems();
	});

	$: if (currentDate || view) {
		loadItems();
	}

	async function loadItems() {
		loading = true;
		try {
			const response = await getCalendarItems({
				date: formatCalendarDate(currentDate),
				view,
				include_tasks: true
			});
			items = response.items;
		} catch (err) {
			pushToast('Failed to load calendar items', 'danger');
		} finally {
			loading = false;
		}
	}

	export async function reload() {
		await loadItems();
	}

	function getFormattedTitle(date: Date, v: ViewType): string {
		if (v === 'day') {
			return date.toLocaleDateString(undefined, {
				weekday: 'long',
				month: 'long',
				day: 'numeric',
				year: 'numeric'
			});
		} else if (v === 'week') {
			const { start, end } = getViewDateRange(date, v);
			const sameMonth = start.getMonth() === end.getMonth();
			if (sameMonth) {
				return `${MONTHS[start.getMonth()]} ${start.getDate()} - ${end.getDate()}, ${start.getFullYear()}`;
			}
			return `${MONTHS[start.getMonth()]} ${start.getDate()} - ${MONTHS[end.getMonth()]} ${end.getDate()}, ${start.getFullYear()}`;
		} else {
			return `${MONTHS[date.getMonth()]} ${date.getFullYear()}`;
		}
	}

	function getWeekDays(start: Date): Date[] {
		const days: Date[] = [];
		for (let i = 0; i < 7; i++) {
			const d = new Date(start);
			d.setDate(start.getDate() + i);
			days.push(d);
		}
		return days;
	}

	function getMonthDays(date: Date): (Date | null)[][] {
		const weeks: (Date | null)[][] = [];
		const firstDay = new Date(date.getFullYear(), date.getMonth(), 1);
		const lastDay = new Date(date.getFullYear(), date.getMonth() + 1, 0);

		let week: (Date | null)[] = [];

		// Pad start of month
		for (let i = 0; i < firstDay.getDay(); i++) {
			week.push(null);
		}

		// Fill days
		for (let d = 1; d <= lastDay.getDate(); d++) {
			week.push(new Date(date.getFullYear(), date.getMonth(), d));
			if (week.length === 7) {
				weeks.push(week);
				week = [];
			}
		}

		// Pad end of month
		if (week.length > 0) {
			while (week.length < 7) week.push(null);
			weeks.push(week);
		}

		return weeks;
	}

	function getItemsForDate(date: Date): CalendarItem[] {
		const dateStr = formatCalendarDate(date);
		return items.filter((item) => item.start.startsWith(dateStr));
	}

	function navigate(direction: -1 | 1) {
		const newDate = new Date(currentDate);
		if (view === 'day') {
			newDate.setDate(newDate.getDate() + direction);
		} else if (view === 'week') {
			newDate.setDate(newDate.getDate() + direction * 7);
		} else {
			newDate.setMonth(newDate.getMonth() + direction);
		}
		currentDate = newDate;
		dispatch('dateChange', { date: currentDate, view });
	}

	function goToToday() {
		currentDate = new Date();
		dispatch('dateChange', { date: currentDate, view });
	}

	function setView(v: ViewType) {
		view = v;
		dispatch('dateChange', { date: currentDate, view });
	}

	function handleItemClick(item: CalendarItem) {
		dispatch('itemClick', item);
		// Navigate to the node
		if (item.kind === 'task') {
			goto(`/tasks?id=${item.id}`);
		} else {
			goto(`/notes/${item.id}`);
		}
	}

	async function exportCalendar() {
		try {
			const { start, end } = dateRange;
			await downloadIcal({
				start: formatCalendarDate(start),
				end: formatCalendarDate(end),
				filename: `mindvault-${view}-${formatCalendarDate(currentDate)}.ics`
			});
			pushToast('Calendar exported', 'success');
		} catch (err) {
			pushToast('Failed to export calendar', 'danger');
		}
	}

	function isToday(date: Date): boolean {
		const today = new Date();
		return (
			date.getDate() === today.getDate() &&
			date.getMonth() === today.getMonth() &&
			date.getFullYear() === today.getFullYear()
		);
	}
</script>

<div class="calendar-view">
	<header class="calendar-header">
		<div class="nav-controls">
			<button class="btn-nav" on:click={() => navigate(-1)} aria-label="Previous">
				<svg
					viewBox="0 0 24 24"
					width="20"
					height="20"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
				>
					<path d="M15 18l-6-6 6-6" />
				</svg>
			</button>
			<button class="btn-today" on:click={goToToday}>Today</button>
			<button class="btn-nav" on:click={() => navigate(1)} aria-label="Next">
				<svg
					viewBox="0 0 24 24"
					width="20"
					height="20"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
				>
					<path d="M9 18l6-6-6-6" />
				</svg>
			</button>
		</div>

		<h2 class="calendar-title">{formattedTitle}</h2>

		<div class="view-controls">
			<div class="view-toggle">
				<button class:active={view === 'day'} on:click={() => setView('day')}>Day</button>
				<button class:active={view === 'week'} on:click={() => setView('week')}>Week</button>
				<button class:active={view === 'month'} on:click={() => setView('month')}>Month</button>
			</div>
			<button class="btn-export" on:click={exportCalendar} title="Export to iCal">
				<svg
					viewBox="0 0 24 24"
					width="18"
					height="18"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
				>
					<path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4M7 10l5 5 5-5M12 15V3" />
				</svg>
			</button>
		</div>
	</header>

	<div class="calendar-body" class:loading>
		{#if view === 'day'}
			<div class="day-view">
				<div class="day-items">
					{#each getItemsForDate(currentDate) as item (item.id)}
						<button
							class="calendar-item"
							class:task={item.kind === 'task'}
							on:click={() => handleItemClick(item)}
						>
							<span class="item-time">
								{item.all_day
									? 'All day'
									: new Date(item.start).toLocaleTimeString(undefined, {
											hour: '2-digit',
											minute: '2-digit'
										})}
							</span>
							<span class="item-title">{item.title}</span>
						</button>
					{:else}
						<div class="no-items">No events for this day</div>
					{/each}
				</div>
			</div>
		{:else if view === 'week'}
			<div class="week-view">
				<div class="week-header">
					{#each weekDays as day}
						<div class="week-day-header" class:today={isToday(day)}>
							<span class="day-name">{DAYS[day.getDay()]}</span>
							<span class="day-number">{day.getDate()}</span>
						</div>
					{/each}
				</div>
				<div class="week-grid">
					{#each weekDays as day}
						<div class="week-day" class:today={isToday(day)}>
							{#each getItemsForDate(day) as item (item.id)}
								<button
									class="calendar-item compact"
									class:task={item.kind === 'task'}
									on:click={() => handleItemClick(item)}
								>
									{item.title}
								</button>
							{/each}
						</div>
					{/each}
				</div>
			</div>
		{:else}
			<div class="month-view">
				<div class="month-header">
					{#each DAYS as day}
						<div class="month-day-header">{day}</div>
					{/each}
				</div>
				<div class="month-grid">
					{#each monthDays as week}
						{#each week as day}
							<div class="month-day" class:today={day && isToday(day)} class:empty={!day}>
								{#if day}
									<span class="day-number">{day.getDate()}</span>
									<div class="day-items-compact">
										{#each getItemsForDate(day).slice(0, 3) as item (item.id)}
											<button
												class="calendar-item tiny"
												class:task={item.kind === 'task'}
												on:click={() => handleItemClick(item)}
											>
												{item.title}
											</button>
										{/each}
										{#if getItemsForDate(day).length > 3}
											<span class="more-items">+{getItemsForDate(day).length - 3} more</span>
										{/if}
									</div>
								{/if}
							</div>
						{/each}
					{/each}
				</div>
			</div>
		{/if}
	</div>
</div>

<style>
	.calendar-view {
		display: flex;
		flex-direction: column;
		height: 100%;
		background: var(--surface-color, #fff);
		border-radius: 12px;
		overflow: hidden;
	}

	.calendar-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 1rem 1.5rem;
		border-bottom: 1px solid var(--border-color, #e0e0e0);
	}

	.nav-controls {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.btn-nav,
	.btn-today,
	.btn-export {
		padding: 0.5rem;
		border: 1px solid var(--border-color, #e0e0e0);
		background: var(--surface-color, #fff);
		border-radius: 6px;
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.btn-today {
		padding: 0.5rem 1rem;
		font-size: 0.875rem;
	}

	.calendar-title {
		margin: 0;
		font-size: 1.25rem;
	}

	.view-controls {
		display: flex;
		align-items: center;
		gap: 0.75rem;
	}

	.view-toggle {
		display: flex;
		border: 1px solid var(--border-color, #e0e0e0);
		border-radius: 6px;
		overflow: hidden;
	}

	.view-toggle button {
		padding: 0.5rem 0.75rem;
		border: none;
		background: transparent;
		font-size: 0.875rem;
		cursor: pointer;
	}

	.view-toggle button.active {
		background: var(--primary-color, #3b82f6);
		color: white;
	}

	.calendar-body {
		flex: 1;
		overflow: auto;
		padding: 1rem;
	}

	.calendar-body.loading {
		opacity: 0.6;
	}

	/* Day View */
	.day-view {
		max-width: 600px;
		margin: 0 auto;
	}

	.day-items {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.calendar-item {
		display: flex;
		gap: 0.75rem;
		padding: 0.75rem 1rem;
		background: var(--primary-light, #dbeafe);
		border: 1px solid var(--primary-lighter);
		border-radius: 8px;
		text-align: left;
		cursor: pointer;
		transition: all 0.3s ease;
		width: 100%;
		box-shadow: 0 1px 3px 0 rgba(0, 0, 0, 0.1);
	}

	.calendar-item:hover {
		background: var(--primary-lighter, #bfdbfe);
		transform: translateY(-2px);
		box-shadow:
			0 4px 6px -1px rgba(0, 0, 0, 0.2),
			0 2px 4px -1px rgba(0, 0, 0, 0.1);
	}

	.calendar-item.task {
		background: var(--success-light, #dcfce7);
		border-color: var(--success-lighter);
	}

	.calendar-item.task:hover {
		background: var(--success-lighter, #bbf7d0);
	}

	.item-time {
		font-size: 0.875rem;
		color: var(--text-muted, #6b7280);
		min-width: 70px;
	}

	.item-title {
		font-weight: 500;
	}

	.no-items {
		text-align: center;
		color: var(--text-muted, #6b7280);
		padding: 2rem;
	}

	/* Week View */
	.week-view {
		display: flex;
		flex-direction: column;
		height: 100%;
	}

	.week-header {
		display: grid;
		grid-template-columns: repeat(7, 1fr);
		border-bottom: 1px solid var(--border-color, #e0e0e0);
	}

	.week-day-header {
		display: flex;
		flex-direction: column;
		align-items: center;
		padding: 0.5rem;
	}

	.week-day-header.today {
		background: var(--primary-light, #dbeafe);
	}

	.day-name {
		font-size: 0.75rem;
		color: var(--text-muted, #6b7280);
	}

	.day-number {
		font-weight: 600;
	}

	.week-grid {
		display: grid;
		grid-template-columns: repeat(7, 1fr);
		flex: 1;
	}

	.week-day {
		border-right: 1px solid var(--border-color, #e0e0e0);
		padding: 0.5rem;
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		min-height: 200px;
	}

	.week-day:last-child {
		border-right: none;
	}

	.week-day.today {
		background: var(--primary-bg, #eff6ff);
	}

	.calendar-item.compact {
		padding: 0.25rem 0.5rem;
		font-size: 0.75rem;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	/* Month View */
	.month-view {
		display: flex;
		flex-direction: column;
		height: 100%;
	}

	.month-header {
		display: grid;
		grid-template-columns: repeat(7, 1fr);
		border-bottom: 1px solid var(--border-color, #e0e0e0);
	}

	.month-day-header {
		text-align: center;
		padding: 0.5rem;
		font-size: 0.75rem;
		color: var(--text-muted, #6b7280);
	}

	.month-grid {
		display: grid;
		grid-template-columns: repeat(7, 1fr);
		flex: 1;
	}

	.month-day {
		border-right: 1px solid var(--border-color, #e0e0e0);
		border-bottom: 1px solid var(--border-color, #e0e0e0);
		padding: 0.25rem;
		min-height: 80px;
	}

	.month-day:nth-child(7n) {
		border-right: none;
	}

	.month-day.empty {
		background: var(--bg-muted, #f9fafb);
	}

	.month-day.today {
		background: var(--primary-bg, #eff6ff);
	}

	.month-day .day-number {
		font-size: 0.75rem;
		padding: 0.25rem;
	}

	.day-items-compact {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.calendar-item.tiny {
		padding: 2px 4px;
		font-size: 0.625rem;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.more-items {
		font-size: 0.625rem;
		color: var(--text-muted, #6b7280);
		padding: 2px 4px;
	}
</style>
