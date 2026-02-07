/**
 * Operations module for the editor document model.
 * Defines all operations that can be performed on the document and their inverses.
 */

import type {
	Operation,
	InsertTextOp,
	DeleteTextOp,
	InsertNodeOp,
	DeleteNodeOp,
	MoveNodeOp,
	SetAttrOp,
	AddMarkOp,
	RemoveMarkOp,
	SplitNodeOp,
	MergeNodesOp,
	EditorNode,
	Mark,
	NodeAttrs,
	MarkType
} from './types';
import { Document, deepCloneNode } from './document';

// ============================================================================
// Operation Creators
// ============================================================================

/**
 * Create an insert text operation.
 */
export function insertTextOp(path: number[], offset: number, text: string): InsertTextOp {
	return {
		type: 'insert_text',
		path: [...path],
		offset,
		text
	};
}

/**
 * Create a delete text operation.
 */
export function deleteTextOp(
	path: number[],
	offset: number,
	length: number,
	deletedText?: string
): DeleteTextOp {
	return {
		type: 'delete_text',
		path: [...path],
		offset,
		length,
		deletedText
	};
}

/**
 * Create an insert node operation.
 */
export function insertNodeOp(path: number[], node: EditorNode): InsertNodeOp {
	return {
		type: 'insert_node',
		path: [...path],
		node: deepCloneNode(node)
	};
}

/**
 * Create a delete node operation.
 */
export function deleteNodeOp(path: number[], deletedNode?: EditorNode): DeleteNodeOp {
	return {
		type: 'delete_node',
		path: [...path],
		deletedNode: deletedNode ? deepCloneNode(deletedNode) : undefined
	};
}

/**
 * Create a move node operation.
 */
export function moveNodeOp(fromPath: number[], toPath: number[]): MoveNodeOp {
	return {
		type: 'move_node',
		fromPath: [...fromPath],
		toPath: [...toPath]
	};
}

/**
 * Create a set attribute operation.
 */
export function setAttrOp(
	path: number[],
	key: keyof NodeAttrs,
	value: unknown,
	previousValue?: unknown
): SetAttrOp {
	return {
		type: 'set_attr',
		path: [...path],
		key,
		value,
		previousValue
	};
}

/**
 * Create an add mark operation.
 */
export function addMarkOp(
	path: number[],
	startOffset: number,
	endOffset: number,
	mark: Mark
): AddMarkOp {
	return {
		type: 'add_mark',
		path: [...path],
		startOffset,
		endOffset,
		mark: { ...mark, attrs: mark.attrs ? { ...mark.attrs } : undefined }
	};
}

/**
 * Create a remove mark operation.
 */
export function removeMarkOp(
	path: number[],
	startOffset: number,
	endOffset: number,
	markType: MarkType
): RemoveMarkOp {
	return {
		type: 'remove_mark',
		path: [...path],
		startOffset,
		endOffset,
		markType
	};
}

/**
 * Create a split node operation.
 */
export function splitNodeOp(
	path: number[],
	offset: number,
	newNodeProps?: Partial<EditorNode>
): SplitNodeOp {
	return {
		type: 'split_node',
		path: [...path],
		offset,
		newNodeProps: newNodeProps ? { ...newNodeProps } : undefined
	};
}

/**
 * Create a merge nodes operation.
 */
export function mergeNodesOp(path: number[], mergeOffset?: number): MergeNodesOp {
	return {
		type: 'merge_nodes',
		path: [...path],
		mergeOffset
	};
}

// ============================================================================
// Inverse Operations
// ============================================================================

/**
 * Get the inverse of an operation (for undo).
 */
export function getInverseOperation(op: Operation): Operation {
	switch (op.type) {
		case 'insert_text':
			return deleteTextOp(op.path, op.offset, op.text.length, op.text);

		case 'delete_text':
			if (!op.deletedText) {
				throw new Error('Cannot invert delete_text without deletedText');
			}
			return insertTextOp(op.path, op.offset, op.deletedText);

		case 'insert_node':
			return deleteNodeOp(op.path, op.node);

		case 'delete_node':
			if (!op.deletedNode) {
				throw new Error('Cannot invert delete_node without deletedNode');
			}
			return insertNodeOp(op.path, op.deletedNode);

		case 'move_node':
			return moveNodeOp(op.toPath, op.fromPath);

		case 'set_attr':
			return setAttrOp(op.path, op.key, op.previousValue, op.value);

		case 'add_mark':
			return removeMarkOp(op.path, op.startOffset, op.endOffset, op.mark.type);

		case 'remove_mark':
			// Note: We lose the mark attrs here. Full implementation would store the removed mark.
			return addMarkOp(op.path, op.startOffset, op.endOffset, { type: op.markType });

		case 'split_node':
			// Inverse of split is merge at the next sibling
			return mergeNodesOp([...op.path.slice(0, -1), op.path[op.path.length - 1] + 1], op.offset);

		case 'merge_nodes':
			// Inverse of merge is split at the merge offset
			if (op.mergeOffset === undefined) {
				throw new Error('Cannot invert merge_nodes without mergeOffset');
			}
			return splitNodeOp(
				[...op.path.slice(0, -1), op.path[op.path.length - 1] - 1],
				op.mergeOffset
			);
	}
}

/**
 * Get the inverse of a list of operations (reversed order).
 */
export function getInverseOperations(ops: Operation[]): Operation[] {
	return ops.map(getInverseOperation).reverse();
}

// ============================================================================
// Operation Application
// ============================================================================

/**
 * Apply an operation to a document.
 * Returns the operation with any data needed for inversion filled in.
 */
export function applyOperation(doc: Document, op: Operation): Operation {
	switch (op.type) {
		case 'insert_text': {
			doc.insertText(op.path, op.offset, op.text);
			return op;
		}

		case 'delete_text': {
			// Get the text being deleted for undo
			const node = doc.getNodeAtPath(op.path);
			const deletedText = node?.text?.slice(op.offset, op.offset + op.length) ?? '';
			doc.deleteText(op.path, op.offset, op.length);
			return { ...op, deletedText };
		}

		case 'insert_node': {
			doc.insertNode(op.path, op.node);
			return op;
		}

		case 'delete_node': {
			// Get the node being deleted for undo
			const deletedNode = doc.getNodeAtPath(op.path);
			doc.deleteNode(op.path);
			return { ...op, deletedNode: deletedNode ?? undefined };
		}

		case 'move_node': {
			// Move is delete + insert at new location
			const node = doc.getNodeAtPath(op.fromPath);
			if (node) {
				doc.deleteNode(op.fromPath);
				// Adjust toPath if it was affected by the delete
				const adjustedToPath = adjustPathAfterDelete(op.toPath, op.fromPath);
				doc.insertNode(adjustedToPath, node);
			}
			return op;
		}

		case 'set_attr': {
			const node = doc.getNodeAtPath(op.path);
			const previousValue = node?.attrs[op.key];
			doc.setNodeAttrs(op.path, { [op.key]: op.value });
			return { ...op, previousValue };
		}

		case 'add_mark': {
			doc.addMark(op.path, op.mark);
			return op;
		}

		case 'remove_mark': {
			doc.removeMark(op.path, op.markType);
			return op;
		}

		case 'split_node': {
			const node = doc.getNodeAtPath(op.path);
			if (!node) return op;

			// For text nodes, split the text
			if (node.text !== undefined) {
				const beforeText = node.text.slice(0, op.offset);
				const afterText = node.text.slice(op.offset);

				// Update the original node with text before split
				doc.setNodeText(op.path, beforeText);

				// Create new node with text after split
				const newNode: EditorNode = {
					...deepCloneNode(node),
					...op.newNodeProps,
					text: afterText
				};

				// Insert the new node after the current one
				const newPath = [...op.path.slice(0, -1), op.path[op.path.length - 1] + 1];
				doc.insertNode(newPath, newNode);
			}
			// For container nodes, split the children
			else if (node.content) {
				const beforeContent = node.content.slice(0, op.offset);
				const afterContent = node.content.slice(op.offset);

				// Update original node with content before split
				doc.replaceNode(op.path, { ...node, content: beforeContent });

				// Create new node with content after split
				const newNode: EditorNode = {
					...deepCloneNode(node),
					...op.newNodeProps,
					content: afterContent
				};

				// Insert after current
				const newPath = [...op.path.slice(0, -1), op.path[op.path.length - 1] + 1];
				doc.insertNode(newPath, newNode);
			}

			return op;
		}

		case 'merge_nodes': {
			const node = doc.getNodeAtPath(op.path);
			const prevPath = [...op.path.slice(0, -1), op.path[op.path.length - 1] - 1];
			const prevNode = doc.getNodeAtPath(prevPath);

			if (!node || !prevNode) return op;

			let mergeOffset: number;

			// For text nodes, concatenate text
			if (prevNode.text !== undefined && node.text !== undefined) {
				mergeOffset = prevNode.text.length;
				doc.setNodeText(prevPath, prevNode.text + node.text);
				doc.deleteNode(op.path);
			}
			// For container nodes, merge children
			else if (prevNode.content && node.content) {
				mergeOffset = prevNode.content.length;
				const mergedContent = [...prevNode.content, ...node.content];
				doc.replaceNode(prevPath, { ...prevNode, content: mergedContent });
				doc.deleteNode(op.path);
			} else {
				mergeOffset = 0;
			}

			return { ...op, mergeOffset };
		}
	}
}

/**
 * Apply multiple operations to a document.
 */
export function applyOperations(doc: Document, ops: Operation[]): Operation[] {
	return ops.map((op) => applyOperation(doc, op));
}

// ============================================================================
// Path Utilities
// ============================================================================

/**
 * Adjust a path after a delete operation at another path.
 */
function adjustPathAfterDelete(path: number[], deletedPath: number[]): number[] {
	// If paths are in different branches, no adjustment needed
	const minLen = Math.min(path.length, deletedPath.length);

	for (let i = 0; i < minLen - 1; i++) {
		if (path[i] !== deletedPath[i]) {
			return path; // Different branch
		}
	}

	// Check if we need to adjust
	const compareIndex = deletedPath.length - 1;
	if (path.length > compareIndex && path[compareIndex] > deletedPath[compareIndex]) {
		const adjusted = [...path];
		adjusted[compareIndex]--;
		return adjusted;
	}

	return path;
}

/**
 * Adjust a path after an insert operation at another path.
 */
export function adjustPathAfterInsert(path: number[], insertedPath: number[]): number[] {
	const minLen = Math.min(path.length, insertedPath.length);

	for (let i = 0; i < minLen - 1; i++) {
		if (path[i] !== insertedPath[i]) {
			return path; // Different branch
		}
	}

	const compareIndex = insertedPath.length - 1;
	if (path.length > compareIndex && path[compareIndex] >= insertedPath[compareIndex]) {
		const adjusted = [...path];
		adjusted[compareIndex]++;
		return adjusted;
	}

	return path;
}

// ============================================================================
// Operation Transformation (for collaborative editing foundation)
// ============================================================================

/**
 * Transform an operation against another operation.
 * Used when operations are concurrent and need to be reconciled.
 * This is a simplified version - full OT would be more complex.
 */
export function transformOperation(op: Operation, against: Operation): Operation {
	// For now, return the operation unchanged
	// Full implementation would handle all operation type combinations
	return op;
}

// ============================================================================
// Operation Validation
// ============================================================================

/**
 * Check if an operation is valid for a document.
 */
export function isOperationValid(doc: Document, op: Operation): boolean {
	switch (op.type) {
		case 'insert_text':
		case 'delete_text': {
			const node = doc.getNodeAtPath(op.path);
			return node !== null && node.text !== undefined;
		}

		case 'insert_node':
		case 'delete_node': {
			// Check parent exists
			const parentPath = op.path.slice(0, -1);
			if (parentPath.length === 0) {
				return true; // Root level is always valid
			}
			const parent = doc.getNodeAtPath(parentPath);
			return parent !== null && parent.content !== null;
		}

		case 'move_node': {
			const node = doc.getNodeAtPath(op.fromPath);
			return node !== null;
		}

		case 'set_attr':
		case 'add_mark':
		case 'remove_mark':
		case 'split_node':
		case 'merge_nodes': {
			const node = doc.getNodeAtPath(op.path);
			return node !== null;
		}
	}
}

// ============================================================================
// Compound Operations
// ============================================================================

/**
 * Create operations to replace text in a range.
 */
export function replaceTextOps(
	path: number[],
	startOffset: number,
	endOffset: number,
	newText: string,
	oldText?: string
): Operation[] {
	const ops: Operation[] = [];

	// Delete the old text
	if (endOffset > startOffset) {
		ops.push(deleteTextOp(path, startOffset, endOffset - startOffset, oldText));
	}

	// Insert the new text
	if (newText.length > 0) {
		ops.push(insertTextOp(path, startOffset, newText));
	}

	return ops;
}

/**
 * Create operations to wrap nodes in a container.
 */
export function wrapNodesOps(
	paths: number[][],
	wrapperNode: EditorNode
): Operation[] {
	if (paths.length === 0) return [];

	const ops: Operation[] = [];

	// Sort paths in reverse order for safe deletion
	const sortedPaths = [...paths].sort((a, b) => {
		for (let i = 0; i < Math.min(a.length, b.length); i++) {
			if (a[i] !== b[i]) return b[i] - a[i];
		}
		return b.length - a.length;
	});

	// Delete nodes (will be stored for undo)
	for (const path of sortedPaths) {
		ops.push(deleteNodeOp(path));
	}

	// Insert the wrapper at the first path
	const firstPath = paths[0];
	ops.push(insertNodeOp(firstPath, wrapperNode));

	return ops;
}

/**
 * Create operations to unwrap nodes from a container.
 */
export function unwrapNodeOps(path: number[]): Operation[] {
	// This would extract children from the container and remove the container
	// Implementation depends on document state
	return [deleteNodeOp(path)];
}
