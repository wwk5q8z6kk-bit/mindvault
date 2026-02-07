/**
 * Document model exports.
 */

// Types
export type {
	BlockType,
	MarkType,
	Mark,
	MarkAttrs,
	NodeAttrs,
	EditorNode,
	Position,
	EditorSelection,
	CursorPosition,
	OperationType,
	BaseOperation,
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
	Operation,
	Transaction,
	DocumentState,
	CreateNodeOptions,
	PathResolution,
	DocumentRange
} from './types';

// Document
export {
	Document,
	createDocument,
	documentFromContent,
	generateNodeId,
	createNode,
	createParagraph,
	createHeading,
	createCodeBlock,
	createBlockquote,
	createList,
	createListItem,
	createTaskList,
	createTaskItem,
	createHorizontalRule,
	createImage,
	createTable,
	createTableRow,
	createTableCell,
	deepCloneContent,
	deepCloneNode
} from './document';

// Selection
export {
	createPosition,
	positionsEqual,
	comparePositions,
	isPositionInPath,
	createSelection,
	createCollapsedSelection,
	selectionFromCursor,
	isSelectionCollapsed,
	selectionsEqual,
	getSelectionRange,
	isSelectionForward,
	isPositionInSelection,
	expandSelection,
	SelectionState,
	createSelectionState,
	adjustPositionAfterInsert,
	adjustPositionAfterDelete,
	adjustSelectionAfterInsert,
	adjustSelectionAfterDelete
} from './selection';

// Operations
export {
	insertTextOp,
	deleteTextOp,
	insertNodeOp,
	deleteNodeOp,
	moveNodeOp,
	setAttrOp,
	addMarkOp,
	removeMarkOp,
	splitNodeOp,
	mergeNodesOp,
	getInverseOperation,
	getInverseOperations,
	applyOperation,
	applyOperations,
	adjustPathAfterInsert,
	isOperationValid,
	replaceTextOps,
	wrapNodesOps,
	unwrapNodeOps
} from './operations';
