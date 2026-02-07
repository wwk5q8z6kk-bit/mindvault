/**
 * Selection Manager - Abstracts browser Selection API inconsistencies.
 * Provides consistent selection handling across browsers and IME input.
 */

import type { Position, EditorSelection, EditorNode } from '../model/types';
import {
	createPosition,
	createSelection,
	createCollapsedSelection,
	isSelectionCollapsed
} from '../model/selection';
import type { Document } from '../model/document';

// ============================================================================
// Types
// ============================================================================

/**
 * State for tracking IME composition.
 */
export interface CompositionState {
	isComposing: boolean;
	startPath: number[] | null;
	startOffset: number;
	compositionText: string;
}

/**
 * Options for the selection manager.
 */
export interface SelectionManagerOptions {
	/** The contenteditable element */
	editorElement: HTMLElement;
	/** The document model */
	document: Document;
	/** Callback when selection changes */
	onSelectionChange?: (selection: EditorSelection | null) => void;
}

// ============================================================================
// DOM ↔ Model Position Conversion
// ============================================================================

/**
 * Convert a DOM node and offset to a model path.
 * This traverses from the DOM node up to the editor root,
 * building a path of indices.
 */
export function domToModelPath(
	node: Node,
	editorElement: HTMLElement
): number[] | null {
	const path: number[] = [];
	let current: Node | null = node;

	// If we're in a text node, start from its parent
	if (current.nodeType === Node.TEXT_NODE) {
		current = current.parentNode;
	}

	while (current && current !== editorElement) {
		const parent = current.parentNode;
		if (!parent) return null;

		// Find index among siblings that are elements
		let index = 0;
		let sibling: Node | null = parent.firstChild;
		while (sibling && sibling !== current) {
			if (sibling.nodeType === Node.ELEMENT_NODE) {
				index++;
			}
			sibling = sibling.nextSibling;
		}

		path.unshift(index);
		current = parent;
	}

	// If we didn't reach the editor element, the node is outside
	if (current !== editorElement) {
		return null;
	}

	return path;
}

/**
 * Convert a DOM Range to a model Position.
 */
export function domRangeToPosition(
	node: Node,
	offset: number,
	editorElement: HTMLElement
): Position | null {
	const path = domToModelPath(node, editorElement);
	if (!path) return null;

	// For text nodes, use the offset directly
	// For element nodes, offset is child index
	let modelOffset = offset;

	if (node.nodeType === Node.ELEMENT_NODE) {
		// If offset points to a child, we might need to adjust
		// For now, use 0 for element selections
		modelOffset = 0;
	}

	return createPosition(path, modelOffset);
}

/**
 * Convert a browser Selection to a model EditorSelection.
 */
export function domSelectionToModel(
	domSelection: Selection,
	editorElement: HTMLElement
): EditorSelection | null {
	if (!domSelection.anchorNode || !domSelection.focusNode) {
		return null;
	}

	// Check if selection is within the editor
	if (!editorElement.contains(domSelection.anchorNode)) {
		return null;
	}

	const anchor = domRangeToPosition(
		domSelection.anchorNode,
		domSelection.anchorOffset,
		editorElement
	);
	const focus = domRangeToPosition(
		domSelection.focusNode,
		domSelection.focusOffset,
		editorElement
	);

	if (!anchor || !focus) {
		return null;
	}

	return createSelection(anchor, focus);
}

/**
 * Convert a model path to a DOM element.
 */
export function modelPathToDomElement(
	path: number[],
	editorElement: HTMLElement
): Element | null {
	let current: Element = editorElement;

	for (const index of path) {
		const children = Array.from(current.children);
		if (index < 0 || index >= children.length) {
			return null;
		}
		current = children[index];
	}

	return current;
}

/**
 * Convert a model Position to a DOM node and offset.
 */
export function modelPositionToDOM(
	position: Position,
	editorElement: HTMLElement
): { node: Node; offset: number } | null {
	const element = modelPathToDomElement(position.path, editorElement);
	if (!element) return null;

	// For text content, find the text node
	const textNode = findTextNode(element);
	if (textNode) {
		const offset = Math.min(position.offset, textNode.textContent?.length ?? 0);
		return { node: textNode, offset };
	}

	// For elements without text, return the element itself
	return { node: element, offset: 0 };
}

/**
 * Find the first text node in an element.
 */
function findTextNode(element: Element): Text | null {
	const walker = document.createTreeWalker(
		element,
		NodeFilter.SHOW_TEXT,
		null
	);
	return walker.nextNode() as Text | null;
}

/**
 * Apply a model EditorSelection to the DOM.
 */
export function applySelectionToDOM(
	selection: EditorSelection | null,
	editorElement: HTMLElement
): boolean {
	const domSelection = window.getSelection();
	if (!domSelection) return false;

	if (!selection) {
		domSelection.removeAllRanges();
		return true;
	}

	const anchorDOM = modelPositionToDOM(selection.anchor, editorElement);
	const focusDOM = modelPositionToDOM(selection.focus, editorElement);

	if (!anchorDOM || !focusDOM) {
		return false;
	}

	try {
		const range = document.createRange();

		if (isSelectionCollapsed(selection)) {
			range.setStart(anchorDOM.node, anchorDOM.offset);
			range.collapse(true);
		} else {
			range.setStart(anchorDOM.node, anchorDOM.offset);
			range.setEnd(focusDOM.node, focusDOM.offset);
		}

		domSelection.removeAllRanges();
		domSelection.addRange(range);
		return true;
	} catch (e) {
		console.warn('Failed to apply selection to DOM:', e);
		return false;
	}
}

// ============================================================================
// Selection Manager Class
// ============================================================================

/**
 * Manages selection state and synchronization between DOM and model.
 */
export class SelectionManager {
	private editorElement: HTMLElement;
	private document: Document;
	private onSelectionChange?: (selection: EditorSelection | null) => void;

	private currentSelection: EditorSelection | null = null;
	private savedSelection: EditorSelection | null = null;
	private compositionState: CompositionState = {
		isComposing: false,
		startPath: null,
		startOffset: 0,
		compositionText: ''
	};

	private selectionObserver: MutationObserver | null = null;
	private isUpdatingDOM = false;

	constructor(options: SelectionManagerOptions) {
		this.editorElement = options.editorElement;
		this.document = options.document;
		this.onSelectionChange = options.onSelectionChange;

		this.setupEventListeners();
	}

	/**
	 * Set up DOM event listeners.
	 */
	private setupEventListeners(): void {
		// Selection change
		document.addEventListener('selectionchange', this.handleSelectionChange);

		// IME composition events
		this.editorElement.addEventListener('compositionstart', this.handleCompositionStart);
		this.editorElement.addEventListener('compositionupdate', this.handleCompositionUpdate);
		this.editorElement.addEventListener('compositionend', this.handleCompositionEnd);
	}

	/**
	 * Clean up event listeners.
	 */
	destroy(): void {
		document.removeEventListener('selectionchange', this.handleSelectionChange);
		this.editorElement.removeEventListener('compositionstart', this.handleCompositionStart);
		this.editorElement.removeEventListener('compositionupdate', this.handleCompositionUpdate);
		this.editorElement.removeEventListener('compositionend', this.handleCompositionEnd);

		if (this.selectionObserver) {
			this.selectionObserver.disconnect();
		}
	}

	// --- Event Handlers ---

	private handleSelectionChange = (): void => {
		// Skip if we're in the middle of updating the DOM
		if (this.isUpdatingDOM) return;

		// Skip during IME composition
		if (this.compositionState.isComposing) return;

		const domSelection = window.getSelection();
		if (!domSelection || domSelection.rangeCount === 0) {
			this.setCurrentSelection(null);
			return;
		}

		const selection = this.fromDOM();
		this.setCurrentSelection(selection);
	};

	private handleCompositionStart = (event: CompositionEvent): void => {
		const selection = this.fromDOM();
		this.compositionState = {
			isComposing: true,
			startPath: selection?.anchor.path ?? null,
			startOffset: selection?.anchor.offset ?? 0,
			compositionText: ''
		};
	};

	private handleCompositionUpdate = (event: CompositionEvent): void => {
		this.compositionState.compositionText = event.data ?? '';
	};

	private handleCompositionEnd = (event: CompositionEvent): void => {
		const finalText = event.data ?? '';
		this.compositionState = {
			isComposing: false,
			startPath: null,
			startOffset: 0,
			compositionText: ''
		};

		// The compositionend is followed by an input event that handles the actual insertion
		// We just need to update our selection after composition
		requestAnimationFrame(() => {
			this.handleSelectionChange();
		});
	};

	// --- Selection State ---

	private setCurrentSelection(selection: EditorSelection | null): void {
		const changed = !this.selectionsEqual(this.currentSelection, selection);
		this.currentSelection = selection;

		if (changed && this.onSelectionChange) {
			this.onSelectionChange(selection);
		}
	}

	private selectionsEqual(a: EditorSelection | null, b: EditorSelection | null): boolean {
		if (a === b) return true;
		if (!a || !b) return false;
		return (
			a.anchor.offset === b.anchor.offset &&
			a.focus.offset === b.focus.offset &&
			a.anchor.path.length === b.anchor.path.length &&
			a.focus.path.length === b.focus.path.length &&
			a.anchor.path.every((v, i) => v === b.anchor.path[i]) &&
			a.focus.path.every((v, i) => v === b.focus.path[i])
		);
	}

	// --- Public API ---

	/**
	 * Get the current selection from DOM.
	 */
	fromDOM(): EditorSelection | null {
		const domSelection = window.getSelection();
		if (!domSelection || domSelection.rangeCount === 0) {
			return null;
		}
		return domSelectionToModel(domSelection, this.editorElement);
	}

	/**
	 * Apply a selection to the DOM.
	 */
	toDOM(selection: EditorSelection | null): boolean {
		this.isUpdatingDOM = true;
		try {
			const success = applySelectionToDOM(selection, this.editorElement);
			if (success) {
				this.currentSelection = selection;
			}
			return success;
		} finally {
			this.isUpdatingDOM = false;
		}
	}

	/**
	 * Get the current selection.
	 */
	getSelection(): EditorSelection | null {
		return this.currentSelection;
	}

	/**
	 * Set the selection (updates both model and DOM).
	 */
	setSelection(selection: EditorSelection | null): void {
		this.currentSelection = selection;
		this.toDOM(selection);
	}

	/**
	 * Save the current selection for later restoration.
	 */
	saveSelection(): void {
		this.savedSelection = this.fromDOM();
	}

	/**
	 * Restore the previously saved selection.
	 */
	restoreSelection(): boolean {
		if (!this.savedSelection) return false;

		const success = this.toDOM(this.savedSelection);
		this.savedSelection = null;
		return success;
	}

	/**
	 * Get the saved selection without restoring it.
	 */
	getSavedSelection(): EditorSelection | null {
		return this.savedSelection;
	}

	/**
	 * Clear the saved selection.
	 */
	clearSavedSelection(): void {
		this.savedSelection = null;
	}

	/**
	 * Check if IME composition is in progress.
	 */
	isComposing(): boolean {
		return this.compositionState.isComposing;
	}

	/**
	 * Get the composition state.
	 */
	getCompositionState(): CompositionState {
		return { ...this.compositionState };
	}

	/**
	 * Check if the selection is collapsed (cursor).
	 */
	isCollapsed(): boolean {
		return this.currentSelection !== null && isSelectionCollapsed(this.currentSelection);
	}

	/**
	 * Check if there is a selection.
	 */
	hasSelection(): boolean {
		return this.currentSelection !== null;
	}

	/**
	 * Get the cursor position (only if collapsed).
	 */
	getCursorPosition(): Position | null {
		if (!this.currentSelection || !isSelectionCollapsed(this.currentSelection)) {
			return null;
		}
		return this.currentSelection.anchor;
	}

	/**
	 * Set a collapsed selection (cursor) at a position.
	 */
	setCursor(path: number[], offset: number): void {
		const selection = createCollapsedSelection(createPosition(path, offset));
		this.setSelection(selection);
	}

	/**
	 * Get the bounding rectangle of the current selection.
	 */
	getBoundingRect(): DOMRect | null {
		const domSelection = window.getSelection();
		if (!domSelection || domSelection.rangeCount === 0) {
			return null;
		}

		const range = domSelection.getRangeAt(0);
		return range.getBoundingClientRect();
	}

	/**
	 * Update the document reference.
	 */
	setDocument(doc: Document): void {
		this.document = doc;
	}

	/**
	 * Update the editor element reference.
	 */
	setEditorElement(element: HTMLElement): void {
		// Clean up old listeners
		this.editorElement.removeEventListener('compositionstart', this.handleCompositionStart);
		this.editorElement.removeEventListener('compositionupdate', this.handleCompositionUpdate);
		this.editorElement.removeEventListener('compositionend', this.handleCompositionEnd);

		// Update reference
		this.editorElement = element;

		// Set up new listeners
		this.editorElement.addEventListener('compositionstart', this.handleCompositionStart);
		this.editorElement.addEventListener('compositionupdate', this.handleCompositionUpdate);
		this.editorElement.addEventListener('compositionend', this.handleCompositionEnd);
	}
}

// ============================================================================
// Factory Function
// ============================================================================

/**
 * Create a new SelectionManager instance.
 */
export function createSelectionManager(options: SelectionManagerOptions): SelectionManager {
	return new SelectionManager(options);
}
