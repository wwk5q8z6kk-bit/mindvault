import { writable } from 'svelte/store';

export type TaskModalState = {
	open: boolean;
	mode: 'create' | 'edit';
	taskId: string | null;
};

export const taskModalState = writable<TaskModalState>({
	open: false,
	mode: 'create',
	taskId: null
});

export const selectedTaskId = writable<string | null>(null);
export const selectedNoteId = writable<string | null>(null);

export const quickAddFocus = writable<(() => void) | null>(null);

export type FocusPlannerItem = {
	task: {
		id: string;
		title: string;
		status: string;
		priority: number;
		due_at?: string | null;
	};
	score: number;
	rank: number;
	reason?: string | null;
};

export type FocusPlannerState = {
	open: boolean;
	generatedAt: string;
	items: FocusPlannerItem[];
};

export const focusPlannerState = writable<FocusPlannerState>({
	open: false,
	generatedAt: '',
	items: []
});
