import { fetchJson } from './client';
import type { Task } from './tasks';

export interface BriefingTask {
	id: string;
	title: string;
	due_at: string | null;
	priority: number;
	status: string;
}

export interface BriefingHabit {
	id: string;
	name: string;
	completed_today: boolean;
	current_streak: number;
}

export interface BriefingNote {
	id: string;
	title: string;
	updated_at: string;
}

export interface BriefingResponse {
	date: string;
	due_today: BriefingTask[];
	overdue: BriefingTask[];
	in_progress: BriefingTask[];
	habits_today: BriefingHabit[];
	recent_notes: BriefingNote[];
	summary: string;
}

export async function fetchBriefing(namespace?: string): Promise<BriefingResponse> {
	const params = new URLSearchParams();
	if (namespace) {
		params.set('namespace', namespace);
	}
	const query = params.toString();
	const url = query ? `/api/briefing?${query}` : '/api/briefing';
	return fetchJson<BriefingResponse>(url);
}
