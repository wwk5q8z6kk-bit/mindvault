import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { HistoryStack, createHistoryStack } from './stack';
import type { Operation, EditorSelection } from '../model/types';

// Helper to make a simple insert_text operation
function makeInsertOp(text: string, offset = 0): Operation {
	return {
		type: 'insert_text',
		path: [0],
		offset,
		text
	};
}

function makeDeleteOp(offset: number, length: number, deletedText: string): Operation {
	return {
		type: 'delete_text',
		path: [0],
		offset,
		length,
		deletedText
	};
}

function makeSelection(offset: number): EditorSelection {
	return {
		anchor: { path: [0], offset },
		focus: { path: [0], offset }
	};
}

describe('HistoryStack', () => {
	beforeEach(() => {
		vi.useFakeTimers();
	});

	afterEach(() => {
		vi.useRealTimers();
	});

	describe('basic undo/redo', () => {
		it('starts empty', () => {
			const stack = new HistoryStack();
			expect(stack.canUndo()).toBe(false);
			expect(stack.canRedo()).toBe(false);
			expect(stack.getUndoCount()).toBe(0);
			expect(stack.getRedoCount()).toBe(0);
		});

		it('records an operation and allows undo', () => {
			const stack = new HistoryStack();
			stack.record([makeInsertOp('a')], null, makeSelection(1));
			expect(stack.canUndo()).toBe(true);

			const undo = stack.getUndoOperations();
			expect(undo).not.toBeNull();
			expect(undo!.operations).toHaveLength(1);
			expect(undo!.operations[0].type).toBe('delete_text');
		});

		it('undo restores selectionBefore', () => {
			const stack = new HistoryStack();
			const before = makeSelection(0);
			const after = makeSelection(5);
			stack.record([makeInsertOp('hello')], before, after);

			const undo = stack.getUndoOperations();
			expect(undo!.selectionAfter).toEqual(before);
		});

		it('redo returns original operations with selectionAfter', () => {
			const stack = new HistoryStack();
			const before = makeSelection(0);
			const after = makeSelection(5);
			stack.record([makeInsertOp('hello')], before, after);

			stack.getUndoOperations(); // undo first
			expect(stack.canRedo()).toBe(true);

			const redo = stack.getRedoOperations();
			expect(redo).not.toBeNull();
			expect(redo!.operations).toHaveLength(1);
			expect(redo!.operations[0].type).toBe('insert_text');
			expect(redo!.selectionAfter).toEqual(after);
		});

		it('returns null when nothing to undo', () => {
			const stack = new HistoryStack();
			expect(stack.getUndoOperations()).toBeNull();
		});

		it('returns null when nothing to redo', () => {
			const stack = new HistoryStack();
			expect(stack.getRedoOperations()).toBeNull();
		});

		it('redo stack is cleared when new operations recorded', () => {
			const stack = new HistoryStack();
			vi.setSystemTime(1000);
			stack.record([makeInsertOp('a')], null, null);

			vi.setSystemTime(2000);
			stack.getUndoOperations();
			expect(stack.canRedo()).toBe(true);

			vi.setSystemTime(3000);
			stack.record([makeInsertOp('b')], null, null);
			expect(stack.canRedo()).toBe(false);
		});
	});

	describe('merge delay', () => {
		it('merges operations within delay window', () => {
			const stack = new HistoryStack({ mergeDelay: 300 });

			vi.setSystemTime(1000);
			stack.record([makeInsertOp('a', 0)], null, makeSelection(1));

			vi.setSystemTime(1100); // within 300ms
			stack.record([makeInsertOp('b', 1)], null, makeSelection(2));

			// Should be merged into one undo step
			expect(stack.getUndoCount()).toBe(1);
		});

		it('creates separate entries beyond delay window', () => {
			const stack = new HistoryStack({ mergeDelay: 300 });

			vi.setSystemTime(1000);
			stack.record([makeInsertOp('a', 0)], null, makeSelection(1));

			vi.setSystemTime(2000); // beyond 300ms
			stack.record([makeInsertOp('b', 1)], null, makeSelection(2));

			expect(stack.getUndoCount()).toBe(2);
		});

		it('merged entry undoes all merged operations', () => {
			const stack = new HistoryStack({ mergeDelay: 300 });

			vi.setSystemTime(1000);
			stack.record([makeInsertOp('a', 0)], makeSelection(0), makeSelection(1));

			vi.setSystemTime(1100);
			stack.record([makeInsertOp('b', 1)], null, makeSelection(2));

			const undo = stack.getUndoOperations();
			expect(undo).not.toBeNull();
			// Inverse of [insertA, insertB] → [deleteB, deleteA] (reversed)
			expect(undo!.operations).toHaveLength(2);
		});
	});

	describe('max undo levels', () => {
		it('enforces max undo levels', () => {
			const stack = new HistoryStack({ maxUndoLevels: 3, mergeDelay: 0 });

			for (let i = 0; i < 5; i++) {
				vi.setSystemTime(i * 1000);
				stack.record([makeInsertOp(String(i))], null, null);
			}

			stack.flush();
			// After flush, should have at most 3 entries
			expect(stack.getUndoCount()).toBeLessThanOrEqual(3);
		});
	});

	describe('clear', () => {
		it('clears all history', () => {
			const stack = new HistoryStack();
			vi.setSystemTime(1000);
			stack.record([makeInsertOp('a')], null, null);
			vi.setSystemTime(2000);
			stack.record([makeInsertOp('b')], null, null);

			stack.clear();
			expect(stack.canUndo()).toBe(false);
			expect(stack.canRedo()).toBe(false);
			expect(stack.getUndoCount()).toBe(0);
			expect(stack.getRedoCount()).toBe(0);
		});
	});

	describe('flush', () => {
		it('flushes pending transaction', () => {
			const stack = new HistoryStack();
			vi.setSystemTime(1000);
			stack.record([makeInsertOp('a')], null, null);
			// Pending transaction exists but not yet on stack
			stack.flush();
			// After flush it should be on the undo stack
			expect(stack.getUndoCount()).toBe(1);
		});

		it('flush is idempotent', () => {
			const stack = new HistoryStack();
			vi.setSystemTime(1000);
			stack.record([makeInsertOp('a')], null, null);
			stack.flush();
			stack.flush();
			expect(stack.getUndoCount()).toBe(1);
		});
	});

	describe('save points', () => {
		it('markSavePoint returns initial when empty', () => {
			const stack = new HistoryStack();
			expect(stack.markSavePoint()).toBe('initial');
		});

		it('markSavePoint returns transaction id', () => {
			const stack = new HistoryStack();
			vi.setSystemTime(1000);
			stack.record([makeInsertOp('a')], null, null);
			const id = stack.markSavePoint();
			expect(id).toContain('txn-');
		});

		it('hasChangesSince detects no changes at save point', () => {
			const stack = new HistoryStack();
			vi.setSystemTime(1000);
			stack.record([makeInsertOp('a')], null, null);
			const saveId = stack.markSavePoint();
			expect(stack.hasChangesSince(saveId)).toBe(false);
		});

		it('hasChangesSince detects changes after save point', () => {
			const stack = new HistoryStack();
			vi.setSystemTime(1000);
			stack.record([makeInsertOp('a')], null, null);
			const saveId = stack.markSavePoint();

			vi.setSystemTime(2000);
			stack.record([makeInsertOp('b')], null, null);
			expect(stack.hasChangesSince(saveId)).toBe(true);
		});

		it('hasChangesSince detects pending transaction', () => {
			const stack = new HistoryStack();
			const saveId = stack.markSavePoint(); // 'initial'
			vi.setSystemTime(1000);
			stack.record([makeInsertOp('a')], null, null);
			// pending transaction exists
			expect(stack.hasChangesSince(saveId)).toBe(true);
		});

		it('hasChangesSince returns false for initial when empty', () => {
			const stack = new HistoryStack();
			expect(stack.hasChangesSince('initial')).toBe(false);
		});
	});

	describe('getLastTransactionSummary', () => {
		it('returns null when empty', () => {
			const stack = new HistoryStack();
			expect(stack.getLastTransactionSummary()).toBeNull();
		});

		it('returns summary of pending transaction', () => {
			const stack = new HistoryStack();
			vi.setSystemTime(1000);
			stack.record([makeInsertOp('a')], null, null);
			const summary = stack.getLastTransactionSummary();
			expect(summary).toContain('insert_text');
		});

		it('returns summary of flushed transaction', () => {
			const stack = new HistoryStack();
			vi.setSystemTime(1000);
			stack.record([makeInsertOp('a')], null, null);
			stack.flush();
			const summary = stack.getLastTransactionSummary();
			expect(summary).toContain('insert_text');
		});

		it('summarizes multiple op types', () => {
			const stack = new HistoryStack({ mergeDelay: 500 });
			vi.setSystemTime(1000);
			stack.record([makeInsertOp('a')], null, null);
			vi.setSystemTime(1100);
			stack.record([makeDeleteOp(0, 1, 'a')], null, null);
			const summary = stack.getLastTransactionSummary();
			expect(summary).toContain('insert_text');
			expect(summary).toContain('delete_text');
		});
	});

	describe('multi-step undo/redo', () => {
		it('supports multiple undo then redo', () => {
			const stack = new HistoryStack({ mergeDelay: 0 });

			vi.setSystemTime(1000);
			stack.record([makeInsertOp('a', 0)], makeSelection(0), makeSelection(1));

			vi.setSystemTime(2000);
			stack.record([makeInsertOp('b', 1)], makeSelection(1), makeSelection(2));

			vi.setSystemTime(3000);
			stack.record([makeInsertOp('c', 2)], makeSelection(2), makeSelection(3));

			// Undo all 3
			expect(stack.getUndoOperations()).not.toBeNull();
			expect(stack.getUndoOperations()).not.toBeNull();
			expect(stack.getUndoOperations()).not.toBeNull();
			expect(stack.canUndo()).toBe(false);

			// Redo all 3
			expect(stack.getRedoOperations()).not.toBeNull();
			expect(stack.getRedoOperations()).not.toBeNull();
			expect(stack.getRedoOperations()).not.toBeNull();
			expect(stack.canRedo()).toBe(false);
		});
	});

	describe('createHistoryStack factory', () => {
		it('creates a stack with options', () => {
			const stack = createHistoryStack({ maxUndoLevels: 5 });
			expect(stack).toBeInstanceOf(HistoryStack);
		});

		it('creates a stack without options', () => {
			const stack = createHistoryStack();
			expect(stack).toBeInstanceOf(HistoryStack);
		});
	});
});
