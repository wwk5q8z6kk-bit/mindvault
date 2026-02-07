import { writable, derived, get } from 'svelte/store';

export type UndoActionType = 'task' | 'note' | 'relationship' | 'tag' | 'general';

export interface UndoEntry {
	id: string;
	description: string;
	actionType: UndoActionType;
	undoFn: () => Promise<void>;
	redoFn?: () => Promise<void>;
	timestamp: number;
}

const MAX_STACK_SIZE = 20;
const EXPIRE_MS = 30_000; // 30 seconds

// Main undo stack
export const undoStack = writable<UndoEntry[]>([]);

// Redo stack (populated when undo is called)
export const redoStack = writable<UndoEntry[]>([]);

// Derived stores for UI
export const canUndo = derived(undoStack, ($stack) => $stack.length > 0);
export const canRedo = derived(redoStack, ($stack) => $stack.length > 0);
export const lastUndoDescription = derived(undoStack, ($stack) => $stack[0]?.description ?? null);
export const lastRedoDescription = derived(redoStack, ($stack) => $stack[0]?.description ?? null);

/**
 * Push an undoable action onto the stack.
 * @param description Human-readable description (e.g., "Completed task")
 * @param undoFn Function to reverse the action
 * @param redoFn Optional function to redo the action (if not provided, redo won't work for this action)
 * @param actionType Category of action for filtering
 */
export function pushUndo(
	description: string,
	undoFn: () => Promise<void>,
	redoFn?: () => Promise<void>,
	actionType: UndoActionType = 'general'
): string {
	const id = crypto.randomUUID();
	const entry: UndoEntry = {
		id,
		description,
		actionType,
		undoFn,
		redoFn,
		timestamp: Date.now()
	};

	// Clear redo stack when new action is pushed
	redoStack.set([]);

	undoStack.update((stack) => {
		const updated = [entry, ...stack].slice(0, MAX_STACK_SIZE);
		return updated;
	});

	// Auto-expire after 30 seconds
	setTimeout(() => {
		undoStack.update((stack) => stack.filter((e) => e.id !== id));
	}, EXPIRE_MS);

	return id;
}

/**
 * Pop and execute the most recent undo action.
 * If the action has a redo function, it will be pushed to the redo stack.
 */
export async function popUndo(id?: string): Promise<boolean> {
	const stack = get(undoStack);
	const entry = id ? stack.find((e) => e.id === id) : stack[0];
	if (!entry) return false;

	// Remove from undo stack
	undoStack.update((s) => s.filter((e) => e.id !== entry.id));

	try {
		await entry.undoFn();

		// If there's a redo function, push to redo stack
		if (entry.redoFn) {
			redoStack.update((stack) => {
				const redoEntry: UndoEntry = {
					...entry,
					// Swap undo and redo for the redo stack
					undoFn: entry.redoFn!,
					redoFn: entry.undoFn
				};
				return [redoEntry, ...stack].slice(0, MAX_STACK_SIZE);
			});
		}

		return true;
	} catch {
		return false;
	}
}

/**
 * Pop and execute the most recent redo action.
 */
export async function popRedo(): Promise<boolean> {
	const stack = get(redoStack);
	const entry = stack[0];
	if (!entry) return false;

	// Remove from redo stack
	redoStack.update((s) => s.filter((e) => e.id !== entry.id));

	try {
		await entry.undoFn(); // In redo stack, undoFn is actually the redo function

		// Push back to undo stack
		if (entry.redoFn) {
			const undoEntry: UndoEntry = {
				...entry,
				undoFn: entry.redoFn,
				redoFn: entry.undoFn
			};
			undoStack.update((stack) => [undoEntry, ...stack].slice(0, MAX_STACK_SIZE));
		}

		return true;
	} catch {
		return false;
	}
}

/**
 * Clear all undo/redo history.
 */
export function clearUndoHistory(): void {
	undoStack.set([]);
	redoStack.set([]);
}

/**
 * Get the current undo stack (for debugging/display).
 */
export function getUndoStack(): UndoEntry[] {
	return get(undoStack);
}

/**
 * Get the current redo stack (for debugging/display).
 */
export function getRedoStack(): UndoEntry[] {
	return get(redoStack);
}

/**
 * Handle keyboard shortcuts for undo/redo.
 * Call this from a keydown handler in the layout.
 */
export async function handleUndoKeyboard(event: KeyboardEvent): Promise<boolean> {
	const isMac = navigator.platform.toUpperCase().indexOf('MAC') >= 0;
	const modifier = isMac ? event.metaKey : event.ctrlKey;

	if (!modifier || event.altKey) return false;

	// Cmd/Ctrl + Z = Undo
	if (event.key === 'z' && !event.shiftKey) {
		// Don't trigger if in an input field
		const target = event.target as HTMLElement;
		if (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable) {
			return false;
		}

		const success = await popUndo();
		if (success) {
			event.preventDefault();
			return true;
		}
	}

	// Cmd/Ctrl + Shift + Z = Redo (or Cmd/Ctrl + Y on Windows)
	if ((event.key === 'z' && event.shiftKey) || (event.key === 'y' && !isMac)) {
		const target = event.target as HTMLElement;
		if (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable) {
			return false;
		}

		const success = await popRedo();
		if (success) {
			event.preventDefault();
			return true;
		}
	}

	return false;
}
