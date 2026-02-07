import { writable, get } from 'svelte/store';

export interface UndoEntry {
	id: string;
	description: string;
	undoFn: () => Promise<void>;
	timestamp: number;
}

const MAX_STACK_SIZE = 10;
const EXPIRE_MS = 10_000;

export const undoStack = writable<UndoEntry[]>([]);

export function pushUndo(description: string, undoFn: () => Promise<void>): string {
	const id = crypto.randomUUID();
	const entry: UndoEntry = { id, description, undoFn, timestamp: Date.now() };

	undoStack.update((stack) => {
		const updated = [entry, ...stack].slice(0, MAX_STACK_SIZE);
		return updated;
	});

	// Auto-expire after 10 seconds
	setTimeout(() => {
		undoStack.update((stack) => stack.filter((e) => e.id !== id));
	}, EXPIRE_MS);

	return id;
}

export async function popUndo(id?: string): Promise<boolean> {
	const stack = get(undoStack);
	const entry = id ? stack.find((e) => e.id === id) : stack[0];
	if (!entry) return false;

	undoStack.update((s) => s.filter((e) => e.id !== entry.id));

	try {
		await entry.undoFn();
		return true;
	} catch {
		return false;
	}
}
