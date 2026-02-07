/**
 * Core types for the MindVault Editor document model.
 * These types define the structure of the editor's internal representation.
 */

// ============================================================================
// Node Types
// ============================================================================

/** Supported block-level node types */
export type BlockType =
	| 'paragraph'
	| 'heading'
	| 'list'
	| 'list-item'
	| 'task-list'
	| 'task-item'
	| 'code-block'
	| 'blockquote'
	| 'table'
	| 'table-row'
	| 'table-cell'
	| 'image'
	| 'hr';

/** Supported inline mark types */
export type MarkType = 'bold' | 'italic' | 'strikethrough' | 'code' | 'link' | 'wikilink';

/**
 * A mark applied to a text range (inline formatting).
 */
export interface Mark {
	type: MarkType;
	attrs?: MarkAttrs;
}

/** Attributes that can be attached to marks */
export interface MarkAttrs {
	/** For links: the URL */
	href?: string;
	/** For wiki-links: the target note identifier */
	target?: string;
	/** For wiki-links: optional alias text */
	alias?: string;
}

/**
 * Attributes for different node types.
 */
export interface NodeAttrs {
	/** For headings: level 1-6 */
	level?: 1 | 2 | 3 | 4 | 5 | 6;
	/** For lists: ordered or unordered */
	ordered?: boolean;
	/** For task items: checked state */
	checked?: boolean;
	/** For code blocks: language identifier */
	language?: string;
	/** For images: source URL */
	src?: string;
	/** For images: alt text */
	alt?: string;
	/** For table cells: header cell */
	header?: boolean;
	/** For table cells: column span */
	colspan?: number;
	/** For table cells: row span */
	rowspan?: number;
}

/**
 * A node in the editor document tree.
 * Nodes can be block-level containers or text leaves.
 */
export interface EditorNode {
	/** Unique identifier for this node */
	id: string;
	/** The type of block this node represents */
	type: BlockType;
	/** Type-specific attributes */
	attrs: NodeAttrs;
	/** Child nodes (null for leaf nodes that contain only text) */
	content: EditorNode[] | null;
	/** Inline marks applied to this node's text */
	marks?: Mark[];
	/** Text content (only for text-bearing nodes) */
	text?: string;
}

// ============================================================================
// Position & Selection
// ============================================================================

/**
 * A position in the document, represented as a path of indices
 * from the root to the target node, plus an offset within that node.
 */
export interface Position {
	/** Path of indices from root to the node */
	path: number[];
	/** Offset within the node (character offset for text nodes) */
	offset: number;
}

/**
 * A selection in the document, defined by anchor and focus positions.
 * The anchor is where the selection started, focus is where it ended.
 */
export interface EditorSelection {
	/** The starting point of the selection */
	anchor: Position;
	/** The ending point of the selection (may be before anchor if selecting backwards) */
	focus: Position;
}

/**
 * A collapsed selection (cursor position).
 */
export interface CursorPosition {
	path: number[];
	offset: number;
}

// ============================================================================
// Operations
// ============================================================================

/** Types of operations that can be performed on the document */
export type OperationType =
	| 'insert_text'
	| 'delete_text'
	| 'insert_node'
	| 'delete_node'
	| 'move_node'
	| 'set_attr'
	| 'add_mark'
	| 'remove_mark'
	| 'split_node'
	| 'merge_nodes';

/**
 * Base operation interface. All operations have a type and can generate their inverse.
 */
export interface BaseOperation {
	type: OperationType;
}

/** Insert text at a position */
export interface InsertTextOp extends BaseOperation {
	type: 'insert_text';
	path: number[];
	offset: number;
	text: string;
}

/** Delete text at a position */
export interface DeleteTextOp extends BaseOperation {
	type: 'delete_text';
	path: number[];
	offset: number;
	length: number;
	/** Stored for undo */
	deletedText?: string;
}

/** Insert a node at a path */
export interface InsertNodeOp extends BaseOperation {
	type: 'insert_node';
	path: number[];
	node: EditorNode;
}

/** Delete a node at a path */
export interface DeleteNodeOp extends BaseOperation {
	type: 'delete_node';
	path: number[];
	/** Stored for undo */
	deletedNode?: EditorNode;
}

/** Move a node from one path to another */
export interface MoveNodeOp extends BaseOperation {
	type: 'move_node';
	fromPath: number[];
	toPath: number[];
}

/** Set attributes on a node */
export interface SetAttrOp extends BaseOperation {
	type: 'set_attr';
	path: number[];
	key: keyof NodeAttrs;
	value: unknown;
	/** Stored for undo */
	previousValue?: unknown;
}

/** Add a mark to a text range */
export interface AddMarkOp extends BaseOperation {
	type: 'add_mark';
	path: number[];
	startOffset: number;
	endOffset: number;
	mark: Mark;
}

/** Remove a mark from a text range */
export interface RemoveMarkOp extends BaseOperation {
	type: 'remove_mark';
	path: number[];
	startOffset: number;
	endOffset: number;
	markType: MarkType;
}

/** Split a node at a position */
export interface SplitNodeOp extends BaseOperation {
	type: 'split_node';
	path: number[];
	offset: number;
	/** Properties for the new node created by the split */
	newNodeProps?: Partial<EditorNode>;
}

/** Merge two adjacent nodes */
export interface MergeNodesOp extends BaseOperation {
	type: 'merge_nodes';
	path: number[];
	/** Stored for undo: the offset where the merge happened */
	mergeOffset?: number;
}

/** Union type of all operations */
export type Operation =
	| InsertTextOp
	| DeleteTextOp
	| InsertNodeOp
	| DeleteNodeOp
	| MoveNodeOp
	| SetAttrOp
	| AddMarkOp
	| RemoveMarkOp
	| SplitNodeOp
	| MergeNodesOp;

// ============================================================================
// Transaction
// ============================================================================

/**
 * A transaction groups multiple operations into a single undoable unit.
 */
export interface Transaction {
	/** Unique identifier for this transaction */
	id: string;
	/** Operations in this transaction, in order */
	operations: Operation[];
	/** Selection state before the transaction */
	selectionBefore: EditorSelection | null;
	/** Selection state after the transaction */
	selectionAfter: EditorSelection | null;
	/** Timestamp when the transaction was created */
	timestamp: number;
}

// ============================================================================
// Document State
// ============================================================================

/**
 * The complete state of the editor document.
 */
export interface DocumentState {
	/** Root nodes of the document */
	content: EditorNode[];
	/** Current selection */
	selection: EditorSelection | null;
	/** Document version (incremented on each change) */
	version: number;
}

// ============================================================================
// Utility Types
// ============================================================================

/**
 * Options for creating a new node.
 */
export interface CreateNodeOptions {
	type: BlockType;
	attrs?: NodeAttrs;
	content?: EditorNode[] | null;
	marks?: Mark[];
	text?: string;
}

/**
 * Result of a path resolution operation.
 */
export interface PathResolution {
	/** The node at the path */
	node: EditorNode;
	/** The parent node (null for root-level nodes) */
	parent: EditorNode | null;
	/** Index of this node in its parent's content array */
	index: number;
}

/**
 * Range in the document for operations that span multiple positions.
 */
export interface DocumentRange {
	start: Position;
	end: Position;
}
