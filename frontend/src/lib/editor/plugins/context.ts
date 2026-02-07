/**
 * Plugin context implementation.
 * Provides the API that plugins use to interact with the editor.
 */

import type { PluginContext, SlashItem } from './types';
import type { EditorSelection, EditorNode, MarkType, BlockType } from '../model/types';
import type { Document } from '../model/document';
import type { HistoryStack } from '../history/stack';
import type { SelectionManager } from '../selection/manager';
import {
	createPosition,
	createCollapsedSelection,
	isSelectionCollapsed,
	getSelectionRange
} from '../model/selection';
import {
	insertTextOp,
	deleteTextOp,
	insertNodeOp,
	setAttrOp,
	addMarkOp,
	removeMarkOp,
	applyOperation
} from '../model/operations';
import { createParagraph, createHeading, createCodeBlock, createBlockquote } from '../model/document';

// ============================================================================
// Types
// ============================================================================

/**
 * Options for creating a plugin context.
 */
export interface PluginContextOptions {
	document: Document;
	history: HistoryStack;
	selection: SelectionManager;
	editorElement: HTMLElement;
	getMarkdown: () => string;
	setMarkdown: (markdown: string) => void;
	onEmit?: (event: string, data?: unknown) => void;
	onTriggerChange?: () => void;
}

// ============================================================================
// Context Implementation
// ============================================================================

/**
 * Create a plugin context with the given dependencies.
 */
export function createPluginContext(options: PluginContextOptions): PluginContext {
	const {
		document,
		history,
		selection,
		editorElement,
		getMarkdown,
		setMarkdown,
		onEmit,
		onTriggerChange
	} = options;

	const context: PluginContext = {
		// Dependencies
		document,
		history,
		selection,
		editorElement,

		// --- Selection Operations ---

		getSelection(): EditorSelection | null {
			return selection.getSelection();
		},

		setSelection(sel: EditorSelection | null): void {
			selection.setSelection(sel);
		},

		hasTextSelection(): boolean {
			const sel = selection.getSelection();
			return sel !== null && !isSelectionCollapsed(sel);
		},

		// --- Document Operations ---

		getNodeAtPath(path: number[]): EditorNode | null {
			return document.getNodeAtPath(path);
		},

		insertText(text: string): void {
			const sel = selection.getSelection();
			if (!sel) return;

			const cursor = sel.anchor;
			const selBefore = sel;

			// If there's a text selection, delete it first
			if (!isSelectionCollapsed(sel)) {
				context.deleteSelection();
			}

			// Insert the text
			const op = insertTextOp(cursor.path, cursor.offset, text);
			const appliedOp = applyOperation(document, op);

			// Record in history
			history.record(
				[appliedOp],
				selBefore,
				createCollapsedSelection(createPosition(cursor.path, cursor.offset + text.length))
			);

			// Update selection
			selection.setCursor(cursor.path, cursor.offset + text.length);

			onTriggerChange?.();
		},

		deleteSelection(): void {
			const sel = selection.getSelection();
			if (!sel) return;

			if (isSelectionCollapsed(sel)) {
				// Delete character before cursor
				if (sel.anchor.offset > 0) {
					const op = deleteTextOp(sel.anchor.path, sel.anchor.offset - 1, 1);
					const appliedOp = applyOperation(document, op);

					history.record(
						[appliedOp],
						sel,
						createCollapsedSelection(createPosition(sel.anchor.path, sel.anchor.offset - 1))
					);

					selection.setCursor(sel.anchor.path, sel.anchor.offset - 1);
				}
			} else {
				// Delete the selected range
				const { start, end } = getSelectionRange(sel);

				// For same-path selections, just delete the text
				if (start.path.join(',') === end.path.join(',')) {
					const op = deleteTextOp(start.path, start.offset, end.offset - start.offset);
					const appliedOp = applyOperation(document, op);

					history.record(
						[appliedOp],
						sel,
						createCollapsedSelection(start)
					);

					selection.setCursor(start.path, start.offset);
				}
				// For cross-path selections, this would be more complex
			}

			onTriggerChange?.();
		},

		insertBlock(node: EditorNode): void {
			const sel = selection.getSelection();
			if (!sel) return;

			// Insert at the path after current block
			const path = [...sel.anchor.path.slice(0, 1), sel.anchor.path[0] + 1];
			const op = insertNodeOp(path, node);
			const appliedOp = applyOperation(document, op);

			history.record([appliedOp], sel, createCollapsedSelection(createPosition(path, 0)));
			selection.setCursor(path, 0);

			onTriggerChange?.();
		},

		setBlockType(type: BlockType, attrs?: Record<string, unknown>): void {
			const sel = selection.getSelection();
			if (!sel) return;

			const path = sel.anchor.path.slice(0, 1);
			const node = document.getNodeAtPath(path);
			if (!node) return;

			// Create a new node of the requested type
			let newNode: EditorNode;
			switch (type) {
				case 'paragraph':
					newNode = createParagraph(node.text ?? '');
					break;
				case 'heading':
					newNode = createHeading((attrs?.level as 1 | 2 | 3 | 4 | 5 | 6) ?? 1, node.text ?? '');
					break;
				case 'code-block':
					newNode = createCodeBlock((attrs?.language as string) ?? '', node.text ?? '');
					break;
				case 'blockquote':
					newNode = createBlockquote();
					break;
				default:
					return;
			}

			document.replaceNode(path, newNode);
			history.record([], sel, sel);

			onTriggerChange?.();
		},

		toggleMark(type: MarkType, attrs?: Record<string, unknown>): void {
			const sel = selection.getSelection();
			if (!sel || isSelectionCollapsed(sel)) return;

			const { start, end } = getSelectionRange(sel);

			// Check if mark is active
			const isActive = context.isMarkActive(type);

			let op;
			if (isActive) {
				op = removeMarkOp(start.path, start.offset, end.offset, type);
			} else {
				op = addMarkOp(start.path, start.offset, end.offset, { type, attrs });
			}

			const appliedOp = applyOperation(document, op);
			history.record([appliedOp], sel, sel);

			onTriggerChange?.();
		},

		isMarkActive(type: MarkType): boolean {
			const sel = selection.getSelection();
			if (!sel) return false;

			const node = document.getNodeAtPath(sel.anchor.path);
			if (!node || !node.marks) return false;

			return node.marks.some((m) => m.type === type);
		},

		getCurrentBlockType(): BlockType | null {
			const sel = selection.getSelection();
			if (!sel) return null;

			const path = sel.anchor.path.slice(0, 1);
			const node = document.getNodeAtPath(path);
			return node?.type ?? null;
		},

		// --- History Operations ---

		undo(): boolean {
			history.flush();
			const undoData = history.getUndoOperations();
			if (!undoData) return false;

			for (const op of undoData.operations) {
				applyOperation(document, op);
			}

			if (undoData.selectionAfter) {
				selection.setSelection(undoData.selectionAfter);
			}

			onTriggerChange?.();
			return true;
		},

		redo(): boolean {
			const redoData = history.getRedoOperations();
			if (!redoData) return false;

			for (const op of redoData.operations) {
				applyOperation(document, op);
			}

			if (redoData.selectionAfter) {
				selection.setSelection(redoData.selectionAfter);
			}

			onTriggerChange?.();
			return true;
		},

		// --- View Operations ---

		focus(): void {
			editorElement.focus();
		},

		blur(): void {
			editorElement.blur();
		},

		scrollToSelection(): void {
			const rect = selection.getBoundingRect();
			if (!rect) return;

			const editorRect = editorElement.getBoundingClientRect();
			const scrollTop = rect.top - editorRect.top;

			editorElement.scrollTo({
				top: editorElement.scrollTop + scrollTop - 100,
				behavior: 'smooth'
			});
		},

		// --- State Access ---

		isEmpty(): boolean {
			const content = document.getContent();
			if (content.length === 0) return true;
			if (content.length === 1) {
				const node = content[0];
				return node.type === 'paragraph' && (!node.text || node.text.trim() === '');
			}
			return false;
		},

		getMarkdown(): string {
			return getMarkdown();
		},

		setMarkdown(markdown: string): void {
			setMarkdown(markdown);
		},

		getWordCount(): number {
			return document.getWordCount();
		},

		// --- Events ---

		emit(event: string, data?: unknown): void {
			onEmit?.(event, data);
		},

		triggerChange(): void {
			onTriggerChange?.();
		}
	};

	return context;
}

// ============================================================================
// Null Context (for testing)
// ============================================================================

/**
 * Create a minimal context for testing.
 */
export function createNullContext(): PluginContext {
	const noop = () => {};
	const noopReturn = <T>(val: T) => () => val;

	return {
		document: null as any,
		history: null as any,
		selection: null as any,
		editorElement: document.createElement('div'),
		getSelection: noopReturn(null),
		setSelection: noop,
		hasTextSelection: noopReturn(false),
		getNodeAtPath: noopReturn(null),
		insertText: noop,
		deleteSelection: noop,
		insertBlock: noop,
		setBlockType: noop,
		toggleMark: noop,
		isMarkActive: noopReturn(false),
		getCurrentBlockType: noopReturn(null),
		undo: noopReturn(false),
		redo: noopReturn(false),
		focus: noop,
		blur: noop,
		scrollToSelection: noop,
		isEmpty: noopReturn(true),
		getMarkdown: noopReturn(''),
		setMarkdown: noop,
		getWordCount: noopReturn(0),
		emit: noop,
		triggerChange: noop
	};
}
