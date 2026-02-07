import Dexie, { type Table } from 'dexie';
import type { Task } from '$lib/api/tasks';
import type { Note } from '$lib/api/notes';
import type { Goal, Habit } from '$lib/api/goals';

export interface TaskRecord extends Task {
	pending?: boolean;
	localOnly?: boolean;
}

export interface NoteRecord extends Note {
	pending?: boolean;
}

export interface GoalRecord extends Goal {
	pending?: boolean;
}

export interface HabitRecord extends Habit {
	pending?: boolean;
}

export interface Flashcard {
	id: string;
	front: string;
	back: string;
	sourceNodeId?: string;
	sourceTitle?: string;
	// SM-2 fields
	interval: number; // days until next review
	repetitions: number;
	easeFactor: number;
	nextReviewAt: string; // ISO date string
	lastReviewedAt?: string;
	created_at: string;
}

export interface OfflineOp {
	id: string;
	op: 'create' | 'update' | 'delete' | 'complete' | 'reopen' | 'snooze';
	taskId?: string;
	localId?: string;
	payload?: Partial<Task> | Record<string, unknown>;
	createdAt: string;
}

class MindVaultDB extends Dexie {
	tasks!: Table<TaskRecord, string>;
	notes!: Table<NoteRecord, string>;
	goals!: Table<GoalRecord, string>;
	habits!: Table<HabitRecord, string>;
	flashcards!: Table<Flashcard, string>;
	queue!: Table<OfflineOp, string>;

	constructor() {
		super('MindVaultDB');
		this.version(1).stores({
			tasks: 'id, status, due_at, priority',
			queue: 'id, createdAt, op'
		});
		this.version(2).stores({
			tasks: 'id, status, due_at, priority',
			notes: 'id, title, pinned',
			queue: 'id, createdAt, op'
		});
		this.version(3).stores({
			tasks: 'id, status, due_at, priority',
			notes: 'id, title, pinned',
			flashcards: 'id, nextReviewAt, sourceNodeId',
			queue: 'id, createdAt, op'
		});
		this.version(4).stores({
			tasks: 'id, status, due_at, priority',
			notes: 'id, title, pinned',
			goals: 'id, status, target_date, updated_at',
			habits: 'id, enabled, updated_at',
			flashcards: 'id, nextReviewAt, sourceNodeId',
			queue: 'id, createdAt, op'
		});
	}
}

export const db = new MindVaultDB();
