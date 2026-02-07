import { describe, expect, it, vi, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import {
	undoStack,
	redoStack,
	canUndo,
	canRedo,
	lastUndoDescription,
	lastRedoDescription,
	pushUndo,
	popUndo,
	popRedo,
	clearUndoHistory,
	getUndoStack,
	getRedoStack
} from './undo';

beforeEach(() => {
	clearUndoHistory();
	vi.useFakeTimers();
});

describe('pushUndo', () => {
	it('adds an entry to the undo stack', () => {
		pushUndo('Delete item', vi.fn());
		expect(get(undoStack)).toHaveLength(1);
		expect(get(canUndo)).toBe(true);
		expect(get(lastUndoDescription)).toBe('Delete item');
	});

	it('clears the redo stack on push', () => {
		// Manually seed the redo stack via undo+redo cycle
		const undoFn = vi.fn();
		const redoFn = vi.fn();
		pushUndo('action', undoFn, redoFn);
		// popUndo is async — we test redo clearing directly
		redoStack.set([
			{
				id: 'fake',
				description: 'old redo',
				actionType: 'general',
				undoFn: vi.fn(),
				timestamp: Date.now()
			}
		]);
		pushUndo('new action', vi.fn());
		expect(get(redoStack)).toHaveLength(0);
	});

	it('limits stack size to 20', () => {
		for (let i = 0; i < 25; i++) {
			pushUndo(`action ${i}`, vi.fn());
		}
		expect(get(undoStack)).toHaveLength(20);
		expect(get(undoStack)[0].description).toBe('action 24');
	});

	it('auto-expires entries after 30 seconds', () => {
		pushUndo('expiring', vi.fn());
		expect(get(undoStack)).toHaveLength(1);
		vi.advanceTimersByTime(31_000);
		expect(get(undoStack)).toHaveLength(0);
	});

	it('returns a unique id', () => {
		const id1 = pushUndo('a', vi.fn());
		const id2 = pushUndo('b', vi.fn());
		expect(id1).toBeTruthy();
		expect(id2).toBeTruthy();
		expect(id1).not.toBe(id2);
	});
});

describe('popUndo', () => {
	it('calls the undo function and removes from stack', async () => {
		const undoFn = vi.fn().mockResolvedValue(undefined);
		pushUndo('test', undoFn);
		const result = await popUndo();
		expect(result).toBe(true);
		expect(undoFn).toHaveBeenCalledOnce();
		expect(get(undoStack)).toHaveLength(0);
	});

	it('returns false when stack is empty', async () => {
		const result = await popUndo();
		expect(result).toBe(false);
	});

	it('pops a specific entry by id', async () => {
		const fn1 = vi.fn().mockResolvedValue(undefined);
		const fn2 = vi.fn().mockResolvedValue(undefined);
		pushUndo('first', fn1);
		const id2 = pushUndo('second', fn2);
		await popUndo(id2);
		expect(fn2).toHaveBeenCalledOnce();
		expect(fn1).not.toHaveBeenCalled();
		expect(get(undoStack)).toHaveLength(1);
		expect(get(undoStack)[0].description).toBe('first');
	});

	it('moves entry to redo stack when redoFn is provided', async () => {
		const undoFn = vi.fn().mockResolvedValue(undefined);
		const redoFn = vi.fn().mockResolvedValue(undefined);
		pushUndo('reversible', undoFn, redoFn);
		await popUndo();
		expect(get(redoStack)).toHaveLength(1);
		expect(get(canRedo)).toBe(true);
		expect(get(lastRedoDescription)).toBe('reversible');
	});

	it('does not add to redo stack when no redoFn', async () => {
		pushUndo('one-way', vi.fn().mockResolvedValue(undefined));
		await popUndo();
		expect(get(redoStack)).toHaveLength(0);
	});

	it('returns false when undoFn throws', async () => {
		pushUndo(
			'fails',
			vi.fn().mockRejectedValue(new Error('boom'))
		);
		const result = await popUndo();
		expect(result).toBe(false);
		expect(get(undoStack)).toHaveLength(0);
	});
});

describe('popRedo', () => {
	it('calls the redo function and moves back to undo stack', async () => {
		const undoFn = vi.fn().mockResolvedValue(undefined);
		const redoFn = vi.fn().mockResolvedValue(undefined);
		pushUndo('action', undoFn, redoFn);
		await popUndo();
		expect(get(redoStack)).toHaveLength(1);

		const result = await popRedo();
		expect(result).toBe(true);
		expect(get(redoStack)).toHaveLength(0);
		expect(get(undoStack)).toHaveLength(1);
	});

	it('returns false when redo stack is empty', async () => {
		const result = await popRedo();
		expect(result).toBe(false);
	});
});

describe('clearUndoHistory', () => {
	it('clears both stacks', () => {
		pushUndo('a', vi.fn());
		pushUndo('b', vi.fn());
		clearUndoHistory();
		expect(get(undoStack)).toHaveLength(0);
		expect(get(redoStack)).toHaveLength(0);
		expect(get(canUndo)).toBe(false);
		expect(get(canRedo)).toBe(false);
	});
});

describe('getUndoStack / getRedoStack', () => {
	it('returns current snapshots', () => {
		pushUndo('x', vi.fn());
		expect(getUndoStack()).toHaveLength(1);
		expect(getRedoStack()).toHaveLength(0);
	});
});
