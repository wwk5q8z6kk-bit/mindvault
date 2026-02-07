/**
 * Operation-based undo/redo history with cursor preservation.
 */

import type { Operation, Transaction, EditorSelection } from '../model/types';
import { getInverseOperations } from '../model/operations';

// ============================================================================
// Types
// ============================================================================

/**
 * Options for the history stack.
 */
export interface HistoryStackOptions {
	/** Maximum number of undo steps to keep */
	maxUndoLevels?: number;
	/** Delay in ms before creating a new undo group */
	mergeDelay?: number;
}

/**
 * History entry containing a transaction and its inverse.
 */
interface HistoryEntry {
	/** The transaction that was applied */
	transaction: Transaction;
	/** The inverse operations for undo */
	inverse: Operation[];
}

// ============================================================================
// History Stack Class
// ============================================================================

/**
 * Manages undo/redo history using operation-based transactions.
 */
export class HistoryStack {
	private undoStack: HistoryEntry[] = [];
	private redoStack: HistoryEntry[] = [];
	private maxUndoLevels: number;
	private mergeDelay: number;
	private lastTransactionTime: number = 0;
	private pendingTransaction: Transaction | null = null;
	private pendingInverse: Operation[] = [];

	constructor(options: HistoryStackOptions = {}) {
		this.maxUndoLevels = options.maxUndoLevels ?? 100;
		this.mergeDelay = options.mergeDelay ?? 300;
	}

	/**
	 * Record an operation that was applied to the document.
	 * Operations within the merge delay are grouped together.
	 */
	record(
		operations: Operation[],
		selectionBefore: EditorSelection | null,
		selectionAfter: EditorSelection | null
	): void {
		const now = Date.now();
		const timeSinceLastTransaction = now - this.lastTransactionTime;

		// If we're within the merge window and have a pending transaction, extend it
		if (this.pendingTransaction && timeSinceLastTransaction < this.mergeDelay) {
			this.pendingTransaction.operations.push(...operations);
			this.pendingTransaction.selectionAfter = selectionAfter;
			this.pendingTransaction.timestamp = now;
			this.pendingInverse.push(...getInverseOperations(operations));
			this.lastTransactionTime = now;
			return;
		}

		// Flush any pending transaction first
		this.flushPendingTransaction();

		// Start a new pending transaction
		this.pendingTransaction = {
			id: generateTransactionId(),
			operations: [...operations],
			selectionBefore,
			selectionAfter,
			timestamp: now
		};
		this.pendingInverse = getInverseOperations(operations);
		this.lastTransactionTime = now;

		// Clear redo stack when new operations are recorded
		this.redoStack = [];
	}

	/**
	 * Flush the pending transaction to the undo stack.
	 */
	private flushPendingTransaction(): void {
		if (!this.pendingTransaction) return;

		this.undoStack.push({
			transaction: this.pendingTransaction,
			inverse: this.pendingInverse
		});

		// Enforce max undo levels
		while (this.undoStack.length > this.maxUndoLevels) {
			this.undoStack.shift();
		}

		this.pendingTransaction = null;
		this.pendingInverse = [];
	}

	/**
	 * Get the operations to undo.
	 * Returns null if there's nothing to undo.
	 */
	getUndoOperations(): {
		operations: Operation[];
		selectionAfter: EditorSelection | null;
	} | null {
		// First flush any pending transaction
		this.flushPendingTransaction();

		const entry = this.undoStack.pop();
		if (!entry) return null;

		// Push to redo stack
		this.redoStack.push(entry);

		return {
			operations: entry.inverse,
			selectionAfter: entry.transaction.selectionBefore
		};
	}

	/**
	 * Get the operations to redo.
	 * Returns null if there's nothing to redo.
	 */
	getRedoOperations(): {
		operations: Operation[];
		selectionAfter: EditorSelection | null;
	} | null {
		const entry = this.redoStack.pop();
		if (!entry) return null;

		// Push back to undo stack
		this.undoStack.push(entry);

		return {
			operations: entry.transaction.operations,
			selectionAfter: entry.transaction.selectionAfter
		};
	}

	/**
	 * Check if undo is available.
	 */
	canUndo(): boolean {
		return this.undoStack.length > 0 || this.pendingTransaction !== null;
	}

	/**
	 * Check if redo is available.
	 */
	canRedo(): boolean {
		return this.redoStack.length > 0;
	}

	/**
	 * Get the number of undo steps available.
	 */
	getUndoCount(): number {
		return this.undoStack.length + (this.pendingTransaction ? 1 : 0);
	}

	/**
	 * Get the number of redo steps available.
	 */
	getRedoCount(): number {
		return this.redoStack.length;
	}

	/**
	 * Clear all history.
	 */
	clear(): void {
		this.undoStack = [];
		this.redoStack = [];
		this.pendingTransaction = null;
		this.pendingInverse = [];
		this.lastTransactionTime = 0;
	}

	/**
	 * Force flush any pending transaction.
	 * Call this before explicit undo/redo requests.
	 */
	flush(): void {
		this.flushPendingTransaction();
	}

	/**
	 * Mark a save point. Can be used to show "unsaved changes" status.
	 */
	markSavePoint(): string {
		this.flushPendingTransaction();
		const lastEntry = this.undoStack[this.undoStack.length - 1];
		return lastEntry?.transaction.id ?? 'initial';
	}

	/**
	 * Check if there are unsaved changes since a save point.
	 */
	hasChangesSince(savePointId: string): boolean {
		if (savePointId === 'initial' && this.undoStack.length === 0) {
			return this.pendingTransaction !== null;
		}

		const lastEntry = this.undoStack[this.undoStack.length - 1];
		if (!lastEntry) {
			return this.pendingTransaction !== null;
		}

		return lastEntry.transaction.id !== savePointId || this.pendingTransaction !== null;
	}

	/**
	 * Get a summary of the last transaction (for debugging/display).
	 */
	getLastTransactionSummary(): string | null {
		if (this.pendingTransaction) {
			return summarizeTransaction(this.pendingTransaction);
		}

		const lastEntry = this.undoStack[this.undoStack.length - 1];
		if (!lastEntry) return null;

		return summarizeTransaction(lastEntry.transaction);
	}
}

// ============================================================================
// Helper Functions
// ============================================================================

let transactionIdCounter = 0;

/**
 * Generate a unique transaction ID.
 */
function generateTransactionId(): string {
	return `txn-${Date.now().toString(36)}-${(transactionIdCounter++).toString(36)}`;
}

/**
 * Summarize a transaction for debugging/display.
 */
function summarizeTransaction(transaction: Transaction): string {
	const opCounts = new Map<string, number>();

	for (const op of transaction.operations) {
		opCounts.set(op.type, (opCounts.get(op.type) ?? 0) + 1);
	}

	const parts: string[] = [];
	for (const [type, count] of opCounts) {
		parts.push(count > 1 ? `${type} x${count}` : type);
	}

	return parts.join(', ');
}

// ============================================================================
// Factory Function
// ============================================================================

/**
 * Create a new HistoryStack instance.
 */
export function createHistoryStack(options?: HistoryStackOptions): HistoryStack {
	return new HistoryStack(options);
}
