/**
 * EditorSelection abstraction that wraps browser Selection API.
 * Provides consistent selection handling across browsers and IME input.
 */

import type { Position, EditorSelection, CursorPosition, EditorNode } from './types';

// ============================================================================
// Position Utilities
// ============================================================================

/**
 * Create a position.
 */
export function createPosition(path: number[], offset: number): Position {
	return { path, offset };
}

/**
 * Check if two positions are equal.
 */
export function positionsEqual(a: Position, b: Position): boolean {
	if (a.offset !== b.offset) return false;
	if (a.path.length !== b.path.length) return false;
	return a.path.every((v, i) => v === b.path[i]);
}

/**
 * Compare two positions. Returns:
 * - negative if a < b
 * - 0 if a === b
 * - positive if a > b
 */
export function comparePositions(a: Position, b: Position): number {
	const minLen = Math.min(a.path.length, b.path.length);

	for (let i = 0; i < minLen; i++) {
		if (a.path[i] !== b.path[i]) {
			return a.path[i] - b.path[i];
		}
	}

	// If paths are equal up to minLen, the shorter path comes first
	if (a.path.length !== b.path.length) {
		return a.path.length - b.path.length;
	}

	// Paths are equal, compare offsets
	return a.offset - b.offset;
}

/**
 * Check if a position is within a path (i.e., the path is an ancestor).
 */
export function isPositionInPath(position: Position, path: number[]): boolean {
	if (position.path.length < path.length) return false;
	return path.every((v, i) => position.path[i] === v);
}

// ============================================================================
// Selection Utilities
// ============================================================================

/**
 * Create a selection from anchor and focus positions.
 */
export function createSelection(anchor: Position, focus: Position): EditorSelection {
	return { anchor, focus };
}

/**
 * Create a collapsed selection (cursor) at a position.
 */
export function createCollapsedSelection(position: Position): EditorSelection {
	return { anchor: position, focus: position };
}

/**
 * Create a collapsed selection from a cursor position.
 */
export function selectionFromCursor(cursor: CursorPosition): EditorSelection {
	const position = createPosition(cursor.path, cursor.offset);
	return createCollapsedSelection(position);
}

/**
 * Check if a selection is collapsed (cursor).
 */
export function isSelectionCollapsed(selection: EditorSelection): boolean {
	return positionsEqual(selection.anchor, selection.focus);
}

/**
 * Check if two selections are equal.
 */
export function selectionsEqual(a: EditorSelection | null, b: EditorSelection | null): boolean {
	if (a === b) return true;
	if (!a || !b) return false;
	return positionsEqual(a.anchor, b.anchor) && positionsEqual(a.focus, b.focus);
}

/**
 * Get the start and end of a selection (normalized so start <= end).
 */
export function getSelectionRange(selection: EditorSelection): { start: Position; end: Position } {
	const cmp = comparePositions(selection.anchor, selection.focus);
	if (cmp <= 0) {
		return { start: selection.anchor, end: selection.focus };
	}
	return { start: selection.focus, end: selection.anchor };
}

/**
 * Check if a selection is forward (anchor before focus).
 */
export function isSelectionForward(selection: EditorSelection): boolean {
	return comparePositions(selection.anchor, selection.focus) <= 0;
}

/**
 * Check if a position is within a selection.
 */
export function isPositionInSelection(position: Position, selection: EditorSelection): boolean {
	const { start, end } = getSelectionRange(selection);
	return comparePositions(position, start) >= 0 && comparePositions(position, end) <= 0;
}

/**
 * Expand a selection to encompass a position.
 */
export function expandSelection(selection: EditorSelection, position: Position): EditorSelection {
	const { start, end } = getSelectionRange(selection);

	if (comparePositions(position, start) < 0) {
		return createSelection(position, selection.focus);
	}
	if (comparePositions(position, end) > 0) {
		return createSelection(selection.anchor, position);
	}

	return selection;
}

// ============================================================================
// Selection State Class
// ============================================================================

/**
 * Manages selection state and provides operations on selections.
 */
export class SelectionState {
	private selection: EditorSelection | null = null;
	private savedSelection: EditorSelection | null = null;

	/**
	 * Get the current selection.
	 */
	get(): EditorSelection | null {
		return this.selection;
	}

	/**
	 * Set the current selection.
	 */
	set(selection: EditorSelection | null): void {
		this.selection = selection;
	}

	/**
	 * Check if there is a selection.
	 */
	hasSelection(): boolean {
		return this.selection !== null;
	}

	/**
	 * Check if the selection is collapsed.
	 */
	isCollapsed(): boolean {
		return this.selection !== null && isSelectionCollapsed(this.selection);
	}

	/**
	 * Get the cursor position (only valid if selection is collapsed).
	 */
	getCursor(): CursorPosition | null {
		if (!this.selection || !isSelectionCollapsed(this.selection)) {
			return null;
		}
		return {
			path: this.selection.anchor.path,
			offset: this.selection.anchor.offset
		};
	}

	/**
	 * Set a collapsed selection (cursor) at a position.
	 */
	setCursor(path: number[], offset: number): void {
		this.selection = createCollapsedSelection(createPosition(path, offset));
	}

	/**
	 * Save the current selection for later restoration.
	 */
	save(): void {
		this.savedSelection = this.selection;
	}

	/**
	 * Restore the previously saved selection.
	 */
	restore(): boolean {
		if (this.savedSelection) {
			this.selection = this.savedSelection;
			this.savedSelection = null;
			return true;
		}
		return false;
	}

	/**
	 * Clear the saved selection.
	 */
	clearSaved(): void {
		this.savedSelection = null;
	}

	/**
	 * Get the normalized range (start <= end).
	 */
	getRange(): { start: Position; end: Position } | null {
		if (!this.selection) return null;
		return getSelectionRange(this.selection);
	}

	/**
	 * Collapse the selection to the start.
	 */
	collapseToStart(): void {
		if (!this.selection) return;
		const { start } = getSelectionRange(this.selection);
		this.selection = createCollapsedSelection(start);
	}

	/**
	 * Collapse the selection to the end.
	 */
	collapseToEnd(): void {
		if (!this.selection) return;
		const { end } = getSelectionRange(this.selection);
		this.selection = createCollapsedSelection(end);
	}

	/**
	 * Move the cursor forward by a number of characters.
	 * This is a simplified version - actual implementation would need
	 * to traverse the document tree.
	 */
	moveCursorForward(offset: number = 1): void {
		if (!this.selection) return;
		const { end } = getSelectionRange(this.selection);
		this.selection = createCollapsedSelection(
			createPosition(end.path, end.offset + offset)
		);
	}

	/**
	 * Move the cursor backward by a number of characters.
	 */
	moveCursorBackward(offset: number = 1): void {
		if (!this.selection) return;
		const { start } = getSelectionRange(this.selection);
		this.selection = createCollapsedSelection(
			createPosition(start.path, Math.max(0, start.offset - offset))
		);
	}

	/**
	 * Extend the selection forward.
	 */
	extendForward(offset: number = 1): void {
		if (!this.selection) return;
		const newFocus = createPosition(
			this.selection.focus.path,
			this.selection.focus.offset + offset
		);
		this.selection = createSelection(this.selection.anchor, newFocus);
	}

	/**
	 * Extend the selection backward.
	 */
	extendBackward(offset: number = 1): void {
		if (!this.selection) return;
		const newFocus = createPosition(
			this.selection.focus.path,
			Math.max(0, this.selection.focus.offset - offset)
		);
		this.selection = createSelection(this.selection.anchor, newFocus);
	}
}

// ============================================================================
// Path Adjustment Utilities
// ============================================================================

/**
 * Adjust a position after an insertion at another position.
 */
export function adjustPositionAfterInsert(
	position: Position,
	insertPath: number[],
	insertOffset: number,
	insertLength: number
): Position {
	// If the position is in a different path, no adjustment needed
	if (!pathsEqual(position.path, insertPath)) {
		return position;
	}

	// If the position is before or at the insert point, no adjustment
	if (position.offset <= insertOffset) {
		return position;
	}

	// Position is after the insert, shift it forward
	return createPosition(position.path, position.offset + insertLength);
}

/**
 * Adjust a position after a deletion at another position.
 */
export function adjustPositionAfterDelete(
	position: Position,
	deletePath: number[],
	deleteOffset: number,
	deleteLength: number
): Position {
	// If the position is in a different path, no adjustment needed
	if (!pathsEqual(position.path, deletePath)) {
		return position;
	}

	// If the position is before the delete point, no adjustment
	if (position.offset <= deleteOffset) {
		return position;
	}

	// If the position is within the deleted range, move to delete point
	if (position.offset <= deleteOffset + deleteLength) {
		return createPosition(position.path, deleteOffset);
	}

	// Position is after the deleted range, shift it backward
	return createPosition(position.path, position.offset - deleteLength);
}

/**
 * Adjust a selection after an insertion.
 */
export function adjustSelectionAfterInsert(
	selection: EditorSelection,
	insertPath: number[],
	insertOffset: number,
	insertLength: number
): EditorSelection {
	return createSelection(
		adjustPositionAfterInsert(selection.anchor, insertPath, insertOffset, insertLength),
		adjustPositionAfterInsert(selection.focus, insertPath, insertOffset, insertLength)
	);
}

/**
 * Adjust a selection after a deletion.
 */
export function adjustSelectionAfterDelete(
	selection: EditorSelection,
	deletePath: number[],
	deleteOffset: number,
	deleteLength: number
): EditorSelection {
	return createSelection(
		adjustPositionAfterDelete(selection.anchor, deletePath, deleteOffset, deleteLength),
		adjustPositionAfterDelete(selection.focus, deletePath, deleteOffset, deleteLength)
	);
}

/**
 * Check if two paths are equal.
 */
function pathsEqual(a: number[], b: number[]): boolean {
	if (a.length !== b.length) return false;
	return a.every((v, i) => v === b[i]);
}

// ============================================================================
// Export
// ============================================================================

export function createSelectionState(): SelectionState {
	return new SelectionState();
}
