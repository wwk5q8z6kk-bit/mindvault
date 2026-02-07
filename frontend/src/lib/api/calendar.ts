/**
 * Calendar API - event viewing and iCal import/export.
 */

import { fetchJson, API_BASE_URL, ApiError } from './client';
import type { KnowledgeNode } from './types';

export type CalendarView = 'day' | 'week' | 'month';

export interface CalendarItem {
	id: string;
	title: string;
	kind: 'event' | 'task';
	start: string; // ISO datetime
	end?: string | null;
	all_day: boolean;
	location?: string | null;
	description?: string | null;
	recurrence?: string | null;
	node: KnowledgeNode;
}

export interface CalendarItemsResponse {
	items: CalendarItem[];
	view: CalendarView;
	range_start: string;
	range_end: string;
}

export interface CalendarItemsParams {
	/** The date to center the view on (YYYY-MM-DD) */
	date?: string;
	/** View type: day, week, or month */
	view?: CalendarView;
	/** Include tasks with due dates */
	include_tasks?: boolean;
	/** Namespace filter */
	namespace?: string;
}

/**
 * Get calendar items for a date range.
 */
export async function getCalendarItems(
	params: CalendarItemsParams = {}
): Promise<CalendarItemsResponse> {
	const searchParams = new URLSearchParams();

	if (params.date) searchParams.set('date', params.date);
	if (params.view) searchParams.set('view', params.view);
	if (params.include_tasks !== undefined)
		searchParams.set('include_tasks', String(params.include_tasks));
	if (params.namespace) searchParams.set('namespace', params.namespace);

	const query = searchParams.toString();
	const path = `/api/v1/calendar/items${query ? `?${query}` : ''}`;

	return await fetchJson<CalendarItemsResponse>(path);
}

/**
 * Export calendar as iCal format.
 * Returns the raw iCal string.
 */
export async function exportIcal(params: {
	start?: string;
	end?: string;
	namespace?: string;
}): Promise<string> {
	const searchParams = new URLSearchParams();
	if (params.start) searchParams.set('start', params.start);
	if (params.end) searchParams.set('end', params.end);
	if (params.namespace) searchParams.set('namespace', params.namespace);

	const query = searchParams.toString();
	const response = await fetch(
		`${API_BASE_URL}/api/v1/calendar/ical${query ? `?${query}` : ''}`
	);

	if (!response.ok) {
		throw new ApiError(`iCal export failed (${response.status})`, response.status);
	}

	return await response.text();
}

/**
 * Download calendar as .ics file.
 */
export async function downloadIcal(params: {
	start?: string;
	end?: string;
	namespace?: string;
	filename?: string;
}): Promise<void> {
	const icalContent = await exportIcal(params);
	const blob = new Blob([icalContent], { type: 'text/calendar;charset=utf-8' });
	const url = URL.createObjectURL(blob);

	const a = document.createElement('a');
	a.href = url;
	a.download = params.filename || 'mindvault-calendar.ics';
	document.body.appendChild(a);
	a.click();
	document.body.removeChild(a);
	URL.revokeObjectURL(url);
}

export interface IcalImportResponse {
	imported_count: number;
	skipped_count: number;
	errors: string[];
	created_ids: string[];
}

export interface IcalImportOptions {
	/** Namespace to import events into */
	namespace?: string;
	/** How to handle duplicates: 'skip' | 'update' | 'duplicate' */
	duplicate_strategy?: 'skip' | 'update' | 'duplicate';
	/** Tags to apply to all imported events */
	tags?: string[];
}

/**
 * Import events from an iCal file.
 */
export async function importIcal(
	file: File,
	options: IcalImportOptions = {}
): Promise<IcalImportResponse> {
	const formData = new FormData();
	formData.append('file', file);
	if (options.namespace) formData.append('namespace', options.namespace);
	if (options.duplicate_strategy) formData.append('duplicate_strategy', options.duplicate_strategy);
	if (options.tags?.length) formData.append('tags', JSON.stringify(options.tags));

	const response = await fetch(`${API_BASE_URL}/api/v1/calendar/ical/import`, {
		method: 'POST',
		body: formData
	});

	if (!response.ok) {
		let body: unknown;
		try {
			body = await response.json();
		} catch {
			body = await response.text();
		}
		throw new ApiError(`iCal import failed (${response.status})`, response.status, body);
	}

	return (await response.json()) as IcalImportResponse;
}

/**
 * Get the date range for a view centered on a date.
 */
export function getViewDateRange(
	centerDate: Date,
	view: CalendarView
): { start: Date; end: Date } {
	const start = new Date(centerDate);
	const end = new Date(centerDate);

	switch (view) {
		case 'day':
			start.setHours(0, 0, 0, 0);
			end.setHours(23, 59, 59, 999);
			break;
		case 'week':
			// Start on Sunday
			const dayOfWeek = start.getDay();
			start.setDate(start.getDate() - dayOfWeek);
			start.setHours(0, 0, 0, 0);
			end.setDate(start.getDate() + 6);
			end.setHours(23, 59, 59, 999);
			break;
		case 'month':
			start.setDate(1);
			start.setHours(0, 0, 0, 0);
			end.setMonth(end.getMonth() + 1, 0);
			end.setHours(23, 59, 59, 999);
			break;
	}

	return { start, end };
}

/**
 * Format a date for the calendar API (YYYY-MM-DD).
 */
export function formatCalendarDate(date: Date): string {
	return date.toISOString().slice(0, 10);
}
