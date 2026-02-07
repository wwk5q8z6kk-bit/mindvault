/**
 * Document container with tree operations.
 * Provides methods for manipulating the editor's node tree.
 */

import type {
	EditorNode,
	BlockType,
	NodeAttrs,
	Mark,
	Position,
	EditorSelection,
	DocumentState,
	PathResolution,
	CreateNodeOptions
} from './types';

// ============================================================================
// ID Generation
// ============================================================================

let nodeIdCounter = 0;

/**
 * Generate a unique node ID.
 */
export function generateNodeId(): string {
	return `node-${Date.now().toString(36)}-${(nodeIdCounter++).toString(36)}`;
}

// ============================================================================
// Node Creation
// ============================================================================

/**
 * Create a new editor node with a unique ID.
 */
export function createNode(options: CreateNodeOptions): EditorNode {
	return {
		id: generateNodeId(),
		type: options.type,
		attrs: options.attrs ?? {},
		content: options.content ?? null,
		marks: options.marks,
		text: options.text
	};
}

/**
 * Create a paragraph node with optional text content.
 */
export function createParagraph(text = ''): EditorNode {
	return createNode({
		type: 'paragraph',
		text: text || undefined,
		content: text ? null : []
	});
}

/**
 * Create a heading node.
 */
export function createHeading(level: 1 | 2 | 3 | 4 | 5 | 6, text = ''): EditorNode {
	return createNode({
		type: 'heading',
		attrs: { level },
		text: text || undefined,
		content: text ? null : []
	});
}

/**
 * Create a code block node.
 */
export function createCodeBlock(language = '', code = ''): EditorNode {
	return createNode({
		type: 'code-block',
		attrs: { language },
		text: code
	});
}

/**
 * Create a blockquote node.
 */
export function createBlockquote(content: EditorNode[] = []): EditorNode {
	return createNode({
		type: 'blockquote',
		content: content.length > 0 ? content : [createParagraph()]
	});
}

/**
 * Create a list node.
 */
export function createList(ordered = false, items: EditorNode[] = []): EditorNode {
	return createNode({
		type: 'list',
		attrs: { ordered },
		content: items.length > 0 ? items : [createListItem()]
	});
}

/**
 * Create a list item node.
 */
export function createListItem(content: EditorNode[] = []): EditorNode {
	return createNode({
		type: 'list-item',
		content: content.length > 0 ? content : [createParagraph()]
	});
}

/**
 * Create a task list node.
 */
export function createTaskList(items: EditorNode[] = []): EditorNode {
	return createNode({
		type: 'task-list',
		content: items.length > 0 ? items : [createTaskItem()]
	});
}

/**
 * Create a task item node.
 */
export function createTaskItem(checked = false, text = ''): EditorNode {
	return createNode({
		type: 'task-item',
		attrs: { checked },
		text: text || undefined,
		content: text ? null : []
	});
}

/**
 * Create a horizontal rule node.
 */
export function createHorizontalRule(): EditorNode {
	return createNode({
		type: 'hr',
		content: null
	});
}

/**
 * Create an image node.
 */
export function createImage(src: string, alt = ''): EditorNode {
	return createNode({
		type: 'image',
		attrs: { src, alt },
		content: null
	});
}

/**
 * Create a table node.
 */
export function createTable(rows: number, cols: number): EditorNode {
	const tableRows: EditorNode[] = [];
	for (let r = 0; r < rows; r++) {
		const cells: EditorNode[] = [];
		for (let c = 0; c < cols; c++) {
			cells.push(createTableCell(r === 0, r === 0 ? `Header ${c + 1}` : ''));
		}
		tableRows.push(createTableRow(cells));
	}
	return createNode({
		type: 'table',
		content: tableRows
	});
}

/**
 * Create a table row node.
 */
export function createTableRow(cells: EditorNode[] = []): EditorNode {
	return createNode({
		type: 'table-row',
		content: cells
	});
}

/**
 * Create a table cell node.
 */
export function createTableCell(header = false, text = ''): EditorNode {
	return createNode({
		type: 'table-cell',
		attrs: { header },
		text: text || undefined,
		content: text ? null : []
	});
}

// ============================================================================
// Document Class
// ============================================================================

/**
 * Document container that manages the editor's node tree.
 */
export class Document {
	private state: DocumentState;

	constructor(content: EditorNode[] = []) {
		this.state = {
			content: content.length > 0 ? content : [createParagraph()],
			selection: null,
			version: 0
		};
	}

	// --- State Access ---

	/**
	 * Get the current document state.
	 */
	getState(): DocumentState {
		return this.state;
	}

	/**
	 * Get the document content (root nodes).
	 */
	getContent(): EditorNode[] {
		return this.state.content;
	}

	/**
	 * Get the current selection.
	 */
	getSelection(): EditorSelection | null {
		return this.state.selection;
	}

	/**
	 * Get the document version.
	 */
	getVersion(): number {
		return this.state.version;
	}

	// --- Path Resolution ---

	/**
	 * Get a node at a given path.
	 * @param path - Array of indices from root to target
	 * @returns The node at the path, or null if not found
	 */
	getNodeAtPath(path: number[]): EditorNode | null {
		if (path.length === 0) {
			return null;
		}

		let current: EditorNode[] | null = this.state.content;
		let node: EditorNode | null = null;

		for (let i = 0; i < path.length; i++) {
			const index = path[i];
			if (!current || index < 0 || index >= current.length) {
				return null;
			}
			node = current[index];
			current = node.content;
		}

		return node;
	}

	/**
	 * Resolve a path to get the node, its parent, and index.
	 */
	resolvePath(path: number[]): PathResolution | null {
		if (path.length === 0) {
			return null;
		}

		const parentPath = path.slice(0, -1);
		const index = path[path.length - 1];

		const parent = parentPath.length > 0 ? this.getNodeAtPath(parentPath) : null;
		const siblings = parent ? parent.content : this.state.content;

		if (!siblings || index < 0 || index >= siblings.length) {
			return null;
		}

		return {
			node: siblings[index],
			parent,
			index
		};
	}

	/**
	 * Find the path to a node by its ID.
	 */
	findPathById(nodeId: string): number[] | null {
		const search = (nodes: EditorNode[], currentPath: number[]): number[] | null => {
			for (let i = 0; i < nodes.length; i++) {
				const node = nodes[i];
				const path = [...currentPath, i];

				if (node.id === nodeId) {
					return path;
				}

				if (node.content) {
					const found = search(node.content, path);
					if (found) {
						return found;
					}
				}
			}
			return null;
		};

		return search(this.state.content, []);
	}

	// --- Node Queries ---

	/**
	 * Check if a node is a block type.
	 */
	isBlockNode(node: EditorNode): boolean {
		return [
			'paragraph',
			'heading',
			'list',
			'list-item',
			'task-list',
			'task-item',
			'code-block',
			'blockquote',
			'table',
			'image',
			'hr'
		].includes(node.type);
	}

	/**
	 * Check if a node can contain other nodes.
	 */
	isContainerNode(node: EditorNode): boolean {
		return [
			'list',
			'list-item',
			'task-list',
			'task-item',
			'blockquote',
			'table',
			'table-row'
		].includes(node.type);
	}

	/**
	 * Check if a node is a text-bearing leaf node.
	 */
	isTextNode(node: EditorNode): boolean {
		return node.text !== undefined || (node.content === null && node.type !== 'hr' && node.type !== 'image');
	}

	/**
	 * Get the text content of a node and its descendants.
	 */
	getTextContent(node: EditorNode): string {
		if (node.text !== undefined) {
			return node.text;
		}

		if (node.content) {
			return node.content.map((child) => this.getTextContent(child)).join('');
		}

		return '';
	}

	/**
	 * Count words in the document.
	 */
	getWordCount(): number {
		const text = this.state.content.map((node) => this.getTextContent(node)).join(' ');
		const words = text.trim().split(/\s+/).filter(Boolean);
		return words.length;
	}

	// --- Immutable Updates ---

	/**
	 * Create a new document state with updated content.
	 */
	private updateState(
		updater: (state: DocumentState) => Partial<DocumentState>
	): DocumentState {
		const updates = updater(this.state);
		this.state = {
			...this.state,
			...updates,
			version: this.state.version + 1
		};
		return this.state;
	}

	/**
	 * Set the current selection.
	 */
	setSelection(selection: EditorSelection | null): DocumentState {
		return this.updateState(() => ({ selection }));
	}

	/**
	 * Insert a node at a path.
	 */
	insertNode(path: number[], node: EditorNode): DocumentState {
		return this.updateState((state) => {
			const newContent = deepCloneContent(state.content);
			insertNodeAtPath(newContent, path, node);
			return { content: newContent };
		});
	}

	/**
	 * Delete a node at a path.
	 */
	deleteNode(path: number[]): DocumentState {
		return this.updateState((state) => {
			const newContent = deepCloneContent(state.content);
			deleteNodeAtPath(newContent, path);
			return { content: newContent };
		});
	}

	/**
	 * Replace a node at a path.
	 */
	replaceNode(path: number[], node: EditorNode): DocumentState {
		return this.updateState((state) => {
			const newContent = deepCloneContent(state.content);
			replaceNodeAtPath(newContent, path, node);
			return { content: newContent };
		});
	}

	/**
	 * Update attributes on a node at a path.
	 */
	setNodeAttrs(path: number[], attrs: Partial<NodeAttrs>): DocumentState {
		return this.updateState((state) => {
			const newContent = deepCloneContent(state.content);
			const node = getNodeAtPath(newContent, path);
			if (node) {
				node.attrs = { ...node.attrs, ...attrs };
			}
			return { content: newContent };
		});
	}

	/**
	 * Set text content of a node at a path.
	 */
	setNodeText(path: number[], text: string): DocumentState {
		return this.updateState((state) => {
			const newContent = deepCloneContent(state.content);
			const node = getNodeAtPath(newContent, path);
			if (node) {
				node.text = text;
			}
			return { content: newContent };
		});
	}

	/**
	 * Insert text at a position within a node.
	 */
	insertText(path: number[], offset: number, text: string): DocumentState {
		return this.updateState((state) => {
			const newContent = deepCloneContent(state.content);
			const node = getNodeAtPath(newContent, path);
			if (node && node.text !== undefined) {
				node.text = node.text.slice(0, offset) + text + node.text.slice(offset);
			}
			return { content: newContent };
		});
	}

	/**
	 * Delete text at a position within a node.
	 */
	deleteText(path: number[], offset: number, length: number): DocumentState {
		return this.updateState((state) => {
			const newContent = deepCloneContent(state.content);
			const node = getNodeAtPath(newContent, path);
			if (node && node.text !== undefined) {
				node.text = node.text.slice(0, offset) + node.text.slice(offset + length);
			}
			return { content: newContent };
		});
	}

	/**
	 * Add a mark to a node.
	 */
	addMark(path: number[], mark: Mark): DocumentState {
		return this.updateState((state) => {
			const newContent = deepCloneContent(state.content);
			const node = getNodeAtPath(newContent, path);
			if (node) {
				node.marks = node.marks ? [...node.marks, mark] : [mark];
			}
			return { content: newContent };
		});
	}

	/**
	 * Remove a mark from a node.
	 */
	removeMark(path: number[], markType: string): DocumentState {
		return this.updateState((state) => {
			const newContent = deepCloneContent(state.content);
			const node = getNodeAtPath(newContent, path);
			if (node && node.marks) {
				node.marks = node.marks.filter((m) => m.type !== markType);
				if (node.marks.length === 0) {
					delete node.marks;
				}
			}
			return { content: newContent };
		});
	}

	/**
	 * Replace the entire document content.
	 */
	setContent(content: EditorNode[]): DocumentState {
		return this.updateState(() => ({
			content: content.length > 0 ? content : [createParagraph()]
		}));
	}

	/**
	 * Clone the document.
	 */
	clone(): Document {
		return new Document(deepCloneContent(this.state.content));
	}
}

// ============================================================================
// Helper Functions (operate on mutable content for internal use)
// ============================================================================

/**
 * Deep clone document content.
 */
export function deepCloneContent(content: EditorNode[]): EditorNode[] {
	return content.map(deepCloneNode);
}

/**
 * Deep clone a single node.
 */
export function deepCloneNode(node: EditorNode): EditorNode {
	return {
		...node,
		attrs: { ...node.attrs },
		content: node.content ? deepCloneContent(node.content) : null,
		marks: node.marks ? node.marks.map((m) => ({ ...m, attrs: m.attrs ? { ...m.attrs } : undefined })) : undefined
	};
}

/**
 * Get a node at a path (mutable access).
 */
function getNodeAtPath(content: EditorNode[], path: number[]): EditorNode | null {
	if (path.length === 0) {
		return null;
	}

	let current: EditorNode[] | null = content;
	let node: EditorNode | null = null;

	for (let i = 0; i < path.length; i++) {
		const index = path[i];
		if (!current || index < 0 || index >= current.length) {
			return null;
		}
		node = current[index];
		current = node.content;
	}

	return node;
}

/**
 * Get the parent content array for a path.
 */
function getParentContent(content: EditorNode[], path: number[]): EditorNode[] | null {
	if (path.length === 0) {
		return null;
	}

	if (path.length === 1) {
		return content;
	}

	const parentPath = path.slice(0, -1);
	const parent = getNodeAtPath(content, parentPath);
	return parent?.content ?? null;
}

/**
 * Insert a node at a path.
 */
function insertNodeAtPath(content: EditorNode[], path: number[], node: EditorNode): void {
	const parentContent = getParentContent(content, path);
	if (!parentContent) {
		return;
	}
	const index = path[path.length - 1];
	parentContent.splice(index, 0, node);
}

/**
 * Delete a node at a path.
 */
function deleteNodeAtPath(content: EditorNode[], path: number[]): void {
	const parentContent = getParentContent(content, path);
	if (!parentContent) {
		return;
	}
	const index = path[path.length - 1];
	parentContent.splice(index, 1);
}

/**
 * Replace a node at a path.
 */
function replaceNodeAtPath(content: EditorNode[], path: number[], node: EditorNode): void {
	const parentContent = getParentContent(content, path);
	if (!parentContent) {
		return;
	}
	const index = path[path.length - 1];
	parentContent[index] = node;
}

// ============================================================================
// Export Document Creation
// ============================================================================

/**
 * Create an empty document.
 */
export function createDocument(content: EditorNode[] = []): Document {
	return new Document(content);
}

/**
 * Create a document from a plain content array.
 */
export function documentFromContent(content: EditorNode[]): Document {
	return new Document(deepCloneContent(content));
}
