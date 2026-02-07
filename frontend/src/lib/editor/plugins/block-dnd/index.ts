/**
 * Block Drag-and-Drop plugin.
 *
 * Features:
 * - Hover shows drag handle on block left edge
 * - Ghost preview while dragging
 * - Drop indicator between blocks
 * - Move operation with undo support
 */

import type { EditorPlugin, PluginContext } from '../types';

// State for drag operations
interface DragState {
	isDragging: boolean;
	draggedBlock: HTMLElement | null;
	draggedIndex: number;
	dropIndex: number | null;
	hoveredBlock: HTMLElement | null;
	handlePosition: { top: number; left: number } | null;
}

let state: DragState = {
	isDragging: false,
	draggedBlock: null,
	draggedIndex: -1,
	dropIndex: null,
	hoveredBlock: null,
	handlePosition: null
};

// Callback for UI updates
let onStateChange: ((state: DragState) => void) | null = null;

// Editor context reference
let editorCtx: PluginContext | null = null;

// Block selectors (top-level blocks that can be dragged)
const BLOCK_SELECTOR = 'p, h1, h2, h3, h4, h5, h6, ul, ol, blockquote, pre, hr, table, .mv-math-block';

/**
 * Register a callback for state changes.
 */
export function onDragStateChange(callback: (state: DragState) => void): () => void {
	onStateChange = callback;
	return () => {
		onStateChange = null;
	};
}

/**
 * Get the current drag state.
 */
export function getDragState(): DragState {
	return { ...state };
}

/**
 * Update state and notify listeners.
 */
function updateState(updates: Partial<DragState>): void {
	state = { ...state, ...updates };
	onStateChange?.(state);
}

/**
 * Get all top-level blocks in the editor.
 */
function getBlocks(): HTMLElement[] {
	if (!editorCtx) return [];
	return Array.from(editorCtx.editorElement.querySelectorAll(`:scope > ${BLOCK_SELECTOR}`));
}

/**
 * Find the block containing a point.
 */
function findBlockAtPoint(x: number, y: number): HTMLElement | null {
	if (!editorCtx) return null;

	const blocks = getBlocks();
	for (const block of blocks) {
		const rect = block.getBoundingClientRect();
		if (y >= rect.top && y <= rect.bottom) {
			return block;
		}
	}
	return null;
}

/**
 * Find the insertion index for a drop.
 */
function findDropIndex(y: number): number | null {
	if (!editorCtx) return null;

	const blocks = getBlocks();
	if (blocks.length === 0) return 0;

	for (let i = 0; i < blocks.length; i++) {
		const rect = blocks[i].getBoundingClientRect();
		const midpoint = rect.top + rect.height / 2;

		if (y < midpoint) {
			return i;
		}
	}

	return blocks.length;
}

/**
 * Calculate handle position for a block.
 */
function calculateHandlePosition(block: HTMLElement): { top: number; left: number } | null {
	if (!editorCtx) return null;

	const blockRect = block.getBoundingClientRect();
	const editorRect = editorCtx.editorElement.getBoundingClientRect();

	return {
		top: blockRect.top - editorRect.top,
		left: -24 // Position to the left of the content
	};
}

/**
 * Handle mouse move over editor.
 */
function handleMouseMove(event: MouseEvent): void {
	if (!editorCtx) return;

	if (state.isDragging) {
		// Update drop index
		const dropIndex = findDropIndex(event.clientY);
		if (dropIndex !== state.dropIndex) {
			updateState({ dropIndex });
		}
	} else {
		// Update hovered block
		const block = findBlockAtPoint(event.clientX, event.clientY);
		if (block !== state.hoveredBlock) {
			const handlePosition = block ? calculateHandlePosition(block) : null;
			updateState({ hoveredBlock: block, handlePosition });
		}
	}
}

/**
 * Handle mouse leave from editor.
 */
function handleMouseLeave(): void {
	if (!state.isDragging) {
		updateState({ hoveredBlock: null, handlePosition: null });
	}
}

/**
 * Start dragging a block.
 */
export function startDrag(block: HTMLElement): void {
	if (!editorCtx) return;

	const blocks = getBlocks();
	const index = blocks.indexOf(block);
	if (index === -1) return;

	// Add dragging class
	block.classList.add('mv-block-dragging');

	updateState({
		isDragging: true,
		draggedBlock: block,
		draggedIndex: index,
		dropIndex: index
	});

	// Emit drag start event
	editorCtx.emit('blockDragStart', { block, index });
}

/**
 * End dragging.
 */
export function endDrag(): void {
	if (!editorCtx || !state.isDragging) return;

	const { draggedBlock, draggedIndex, dropIndex } = state;

	// Remove dragging class
	if (draggedBlock) {
		draggedBlock.classList.remove('mv-block-dragging');
	}

	// Check if we actually moved
	if (dropIndex !== null && dropIndex !== draggedIndex && draggedBlock) {
		// Perform the move
		moveBlock(draggedBlock, draggedIndex, dropIndex);
	}

	updateState({
		isDragging: false,
		draggedBlock: null,
		draggedIndex: -1,
		dropIndex: null
	});

	// Emit drag end event
	editorCtx.emit('blockDragEnd', {});
}

/**
 * Cancel dragging.
 */
export function cancelDrag(): void {
	if (!state.isDragging) return;

	if (state.draggedBlock) {
		state.draggedBlock.classList.remove('mv-block-dragging');
	}

	updateState({
		isDragging: false,
		draggedBlock: null,
		draggedIndex: -1,
		dropIndex: null
	});
}

/**
 * Move a block from one position to another.
 */
function moveBlock(block: HTMLElement, fromIndex: number, toIndex: number): void {
	if (!editorCtx) return;

	const blocks = getBlocks();

	// Adjust toIndex if moving down (since we're removing the element first)
	const adjustedToIndex = toIndex > fromIndex ? toIndex - 1 : toIndex;

	// Remove the block
	block.remove();

	// Insert at new position
	if (adjustedToIndex >= blocks.length - 1) {
		// Insert at end
		editorCtx.editorElement.appendChild(block);
	} else {
		// Insert before the target block
		const targetBlock = blocks[adjustedToIndex + (toIndex > fromIndex ? 1 : 0)];
		if (targetBlock && targetBlock !== block) {
			targetBlock.before(block);
		} else {
			editorCtx.editorElement.appendChild(block);
		}
	}

	// Trigger change
	editorCtx.triggerChange();

	// Emit move event
	editorCtx.emit('blockMoved', { fromIndex, toIndex });
}

/**
 * Block DnD plugin.
 */
export const blockDndPlugin: EditorPlugin = {
	id: 'block-dnd',
	name: 'Block Drag & Drop',
	description: 'Drag blocks to reorder content',
	version: '1.0.0',

	onInit(ctx: PluginContext) {
		editorCtx = ctx;

		// Add event listeners
		ctx.editorElement.addEventListener('mousemove', handleMouseMove);
		ctx.editorElement.addEventListener('mouseleave', handleMouseLeave);

		// Reset state
		state = {
			isDragging: false,
			draggedBlock: null,
			draggedIndex: -1,
			dropIndex: null,
			hoveredBlock: null,
			handlePosition: null
		};
	},

	onDestroy() {
		if (editorCtx) {
			editorCtx.editorElement.removeEventListener('mousemove', handleMouseMove);
			editorCtx.editorElement.removeEventListener('mouseleave', handleMouseLeave);
		}
		editorCtx = null;
		onStateChange = null;
	},

	onKeyDown(event: KeyboardEvent, ctx: PluginContext): boolean | void {
		// Cancel drag on Escape
		if (state.isDragging && event.key === 'Escape') {
			event.preventDefault();
			cancelDrag();
			return true;
		}
	},

	commands: {
		startBlockDrag: (ctx: PluginContext, ...args: unknown[]) => {
			const [block] = args as [HTMLElement | undefined];
			if (block) {
				startDrag(block);
			}
		},
		endBlockDrag: () => endDrag(),
		cancelBlockDrag: () => cancelDrag()
	}
};

export default blockDndPlugin;
