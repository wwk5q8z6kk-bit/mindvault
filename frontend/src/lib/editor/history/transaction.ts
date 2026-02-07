/**
 * Transaction builder for grouping operations into undoable units.
 */

import type { Operation, Transaction, EditorSelection, EditorNode } from '../model/types';
import type { Document } from '../model/document';
import { applyOperation, applyOperations } from '../model/operations';

// ============================================================================
// Types
// ============================================================================

/**
 * Builder for creating transactions.
 */
export interface TransactionBuilder {
	/** Add an operation to the transaction */
	addOperation(op: Operation): TransactionBuilder;
	/** Add multiple operations */
	addOperations(ops: Operation[]): TransactionBuilder;
	/** Set the selection after the transaction */
	setSelection(selection: EditorSelection | null): TransactionBuilder;
	/** Apply the transaction to a document */
	apply(doc: Document): AppliedTransaction;
	/** Get the operations without applying */
	getOperations(): Operation[];
	/** Check if the transaction has any operations */
	isEmpty(): boolean;
}

/**
 * Result of applying a transaction.
 */
export interface AppliedTransaction {
	/** The transaction that was applied */
	transaction: Transaction;
	/** Operations with undo data filled in */
	appliedOperations: Operation[];
	/** The selection after the transaction */
	selectionAfter: EditorSelection | null;
}

// ============================================================================
// Transaction Builder Implementation
// ============================================================================

let transactionIdCounter = 0;

/**
 * Create a new transaction builder.
 */
export function createTransaction(
	selectionBefore: EditorSelection | null = null
): TransactionBuilder {
	const operations: Operation[] = [];
	let selectionAfter: EditorSelection | null = null;

	const builder: TransactionBuilder = {
		addOperation(op: Operation): TransactionBuilder {
			operations.push(op);
			return builder;
		},

		addOperations(ops: Operation[]): TransactionBuilder {
			operations.push(...ops);
			return builder;
		},

		setSelection(selection: EditorSelection | null): TransactionBuilder {
			selectionAfter = selection;
			return builder;
		},

		apply(doc: Document): AppliedTransaction {
			// Apply all operations, collecting undo data
			const appliedOperations = applyOperations(doc, operations);

			const transaction: Transaction = {
				id: `txn-${Date.now().toString(36)}-${(transactionIdCounter++).toString(36)}`,
				operations: appliedOperations,
				selectionBefore,
				selectionAfter,
				timestamp: Date.now()
			};

			return {
				transaction,
				appliedOperations,
				selectionAfter
			};
		},

		getOperations(): Operation[] {
			return [...operations];
		},

		isEmpty(): boolean {
			return operations.length === 0;
		}
	};

	return builder;
}

// ============================================================================
// Transaction Composition
// ============================================================================

/**
 * Compose multiple transactions into a single transaction.
 */
export function composeTransactions(transactions: Transaction[]): Transaction | null {
	if (transactions.length === 0) return null;
	if (transactions.length === 1) return transactions[0];

	const operations: Operation[] = [];
	for (const txn of transactions) {
		operations.push(...txn.operations);
	}

	return {
		id: `txn-composed-${Date.now().toString(36)}-${(transactionIdCounter++).toString(36)}`,
		operations,
		selectionBefore: transactions[0].selectionBefore,
		selectionAfter: transactions[transactions.length - 1].selectionAfter,
		timestamp: Date.now()
	};
}

// ============================================================================
// Common Transaction Patterns
// ============================================================================

/**
 * Create a transaction that inserts text at the current cursor.
 */
export function insertTextTransaction(
	path: number[],
	offset: number,
	text: string,
	selectionBefore: EditorSelection | null
): TransactionBuilder {
	const { createPosition, createCollapsedSelection } = require('../model/selection');
	const { insertTextOp } = require('../model/operations');

	return createTransaction(selectionBefore)
		.addOperation(insertTextOp(path, offset, text))
		.setSelection(createCollapsedSelection(createPosition(path, offset + text.length)));
}

/**
 * Create a transaction that deletes text.
 */
export function deleteTextTransaction(
	path: number[],
	offset: number,
	length: number,
	selectionBefore: EditorSelection | null
): TransactionBuilder {
	const { createPosition, createCollapsedSelection } = require('../model/selection');
	const { deleteTextOp } = require('../model/operations');

	return createTransaction(selectionBefore)
		.addOperation(deleteTextOp(path, offset, length))
		.setSelection(createCollapsedSelection(createPosition(path, offset)));
}

/**
 * Create a transaction that replaces the selection with text.
 */
export function replaceSelectionTransaction(
	selection: EditorSelection,
	text: string
): TransactionBuilder {
	const { getSelectionRange, createPosition, createCollapsedSelection } = require('../model/selection');
	const { deleteTextOp, insertTextOp } = require('../model/operations');

	const { start, end } = getSelectionRange(selection);
	const builder = createTransaction(selection);

	// If selection spans different paths, this is more complex
	// For now, handle same-path selections
	if (start.path.join(',') === end.path.join(',')) {
		if (end.offset > start.offset) {
			builder.addOperation(deleteTextOp(start.path, start.offset, end.offset - start.offset));
		}
		if (text.length > 0) {
			builder.addOperation(insertTextOp(start.path, start.offset, text));
		}
		builder.setSelection(createCollapsedSelection(createPosition(start.path, start.offset + text.length)));
	}

	return builder;
}

/**
 * Create a transaction that inserts a node.
 */
export function insertNodeTransaction(
	path: number[],
	node: EditorNode,
	selectionBefore: EditorSelection | null,
	cursorPath?: number[]
): TransactionBuilder {
	const { createPosition, createCollapsedSelection } = require('../model/selection');
	const { insertNodeOp } = require('../model/operations');

	const builder = createTransaction(selectionBefore)
		.addOperation(insertNodeOp(path, node));

	if (cursorPath) {
		builder.setSelection(createCollapsedSelection(createPosition(cursorPath, 0)));
	}

	return builder;
}

/**
 * Create a transaction that deletes a node.
 */
export function deleteNodeTransaction(
	path: number[],
	selectionBefore: EditorSelection | null,
	cursorPath?: number[]
): TransactionBuilder {
	const { createPosition, createCollapsedSelection } = require('../model/selection');
	const { deleteNodeOp } = require('../model/operations');

	const builder = createTransaction(selectionBefore)
		.addOperation(deleteNodeOp(path));

	if (cursorPath) {
		builder.setSelection(createCollapsedSelection(createPosition(cursorPath, 0)));
	}

	return builder;
}

/**
 * Create a transaction that sets a node attribute.
 */
export function setAttrTransaction(
	path: number[],
	key: string,
	value: unknown,
	selectionBefore: EditorSelection | null
): TransactionBuilder {
	const { setAttrOp } = require('../model/operations');

	return createTransaction(selectionBefore)
		.addOperation(setAttrOp(path, key as any, value))
		.setSelection(selectionBefore);
}

/**
 * Create a transaction that toggles a mark on a selection.
 */
export function toggleMarkTransaction(
	selection: EditorSelection,
	markType: string,
	isActive: boolean,
	markAttrs?: Record<string, unknown>
): TransactionBuilder {
	const { getSelectionRange } = require('../model/selection');
	const { addMarkOp, removeMarkOp } = require('../model/operations');

	const { start, end } = getSelectionRange(selection);
	const builder = createTransaction(selection);

	// Handle same-path selections
	if (start.path.join(',') === end.path.join(',')) {
		if (isActive) {
			builder.addOperation(removeMarkOp(start.path, start.offset, end.offset, markType as any));
		} else {
			builder.addOperation(addMarkOp(start.path, start.offset, end.offset, {
				type: markType as any,
				attrs: markAttrs
			}));
		}
	}

	return builder.setSelection(selection);
}

/**
 * Create a transaction that splits a node (e.g., pressing Enter).
 */
export function splitNodeTransaction(
	path: number[],
	offset: number,
	selectionBefore: EditorSelection | null
): TransactionBuilder {
	const { createPosition, createCollapsedSelection } = require('../model/selection');
	const { splitNodeOp } = require('../model/operations');

	// After split, cursor goes to start of new node
	const newPath = [...path.slice(0, -1), path[path.length - 1] + 1];

	return createTransaction(selectionBefore)
		.addOperation(splitNodeOp(path, offset))
		.setSelection(createCollapsedSelection(createPosition(newPath, 0)));
}

/**
 * Create a transaction that merges nodes (e.g., Backspace at start of node).
 */
export function mergeNodesTransaction(
	path: number[],
	selectionBefore: EditorSelection | null,
	mergeOffset: number
): TransactionBuilder {
	const { createPosition, createCollapsedSelection } = require('../model/selection');
	const { mergeNodesOp } = require('../model/operations');

	// After merge, cursor goes to the merge point in the previous node
	const prevPath = [...path.slice(0, -1), path[path.length - 1] - 1];

	return createTransaction(selectionBefore)
		.addOperation(mergeNodesOp(path, mergeOffset))
		.setSelection(createCollapsedSelection(createPosition(prevPath, mergeOffset)));
}
