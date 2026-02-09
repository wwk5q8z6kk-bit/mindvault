import { describe, it, expect } from 'vitest';
import {
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
import { Document, createNode, createParagraph } from './document';
import type { EditorNode, Operation } from './types';

// Helpers
function makeTextNode(id: string, text: string): EditorNode {
	return { id, type: 'paragraph', attrs: {}, content: null, text };
}

function makeContainerNode(id: string, children: EditorNode[]): EditorNode {
	return { id, type: 'list', attrs: {}, content: children };
}

function docWith(...nodes: EditorNode[]): Document {
	return new Document(nodes);
}

// ============================================================================
// Operation Creators
// ============================================================================

describe('operation creators', () => {
	it('insertTextOp creates correct op', () => {
		const op = insertTextOp([0], 5, 'hello');
		expect(op.type).toBe('insert_text');
		expect(op.path).toEqual([0]);
		expect(op.offset).toBe(5);
		expect(op.text).toBe('hello');
	});

	it('insertTextOp clones the path', () => {
		const path = [0, 1];
		const op = insertTextOp(path, 0, 'x');
		path[0] = 99;
		expect(op.path).toEqual([0, 1]);
	});

	it('deleteTextOp creates correct op', () => {
		const op = deleteTextOp([1], 3, 4, 'test');
		expect(op.type).toBe('delete_text');
		expect(op.path).toEqual([1]);
		expect(op.offset).toBe(3);
		expect(op.length).toBe(4);
		expect(op.deletedText).toBe('test');
	});

	it('deleteTextOp works without deletedText', () => {
		const op = deleteTextOp([0], 0, 2);
		expect(op.deletedText).toBeUndefined();
	});

	it('insertNodeOp deep-clones the node', () => {
		const node = makeTextNode('n1', 'hi');
		const op = insertNodeOp([0], node);
		expect(op.type).toBe('insert_node');
		expect(op.node.id).toBe('n1');
		expect(op.node).not.toBe(node); // cloned
	});

	it('deleteNodeOp stores deletedNode when provided', () => {
		const node = makeTextNode('n1', 'hi');
		const op = deleteNodeOp([0], node);
		expect(op.deletedNode).toBeDefined();
		expect(op.deletedNode!.id).toBe('n1');
		expect(op.deletedNode).not.toBe(node);
	});

	it('deleteNodeOp works without deletedNode', () => {
		const op = deleteNodeOp([0]);
		expect(op.deletedNode).toBeUndefined();
	});

	it('moveNodeOp clones both paths', () => {
		const from = [0, 1];
		const to = [1, 0];
		const op = moveNodeOp(from, to);
		from[0] = 99;
		to[0] = 99;
		expect(op.fromPath).toEqual([0, 1]);
		expect(op.toPath).toEqual([1, 0]);
	});

	it('setAttrOp stores key, value, previousValue', () => {
		const op = setAttrOp([0], 'level', 2, 1);
		expect(op.type).toBe('set_attr');
		expect(op.key).toBe('level');
		expect(op.value).toBe(2);
		expect(op.previousValue).toBe(1);
	});

	it('addMarkOp clones mark attrs', () => {
		const attrs = { href: 'http://example.com' };
		const op = addMarkOp([0], 0, 5, { type: 'link', attrs });
		attrs.href = 'changed';
		expect(op.mark.attrs!.href).toBe('http://example.com');
	});

	it('removeMarkOp creates correct op', () => {
		const op = removeMarkOp([0], 2, 8, 'bold');
		expect(op.type).toBe('remove_mark');
		expect(op.markType).toBe('bold');
		expect(op.startOffset).toBe(2);
		expect(op.endOffset).toBe(8);
	});

	it('splitNodeOp stores offset and newNodeProps', () => {
		const op = splitNodeOp([0], 5, { type: 'paragraph' } as Partial<EditorNode>);
		expect(op.type).toBe('split_node');
		expect(op.offset).toBe(5);
		expect(op.newNodeProps).toBeDefined();
	});

	it('mergeNodesOp stores mergeOffset', () => {
		const op = mergeNodesOp([1], 10);
		expect(op.type).toBe('merge_nodes');
		expect(op.mergeOffset).toBe(10);
	});
});

// ============================================================================
// Inverse Operations
// ============================================================================

describe('getInverseOperation', () => {
	it('insert_text → delete_text', () => {
		const op = insertTextOp([0], 5, 'hello');
		const inv = getInverseOperation(op);
		expect(inv.type).toBe('delete_text');
		if (inv.type === 'delete_text') {
			expect(inv.path).toEqual([0]);
			expect(inv.offset).toBe(5);
			expect(inv.length).toBe(5);
			expect(inv.deletedText).toBe('hello');
		}
	});

	it('delete_text → insert_text', () => {
		const op = deleteTextOp([0], 3, 4, 'test');
		const inv = getInverseOperation(op);
		expect(inv.type).toBe('insert_text');
		if (inv.type === 'insert_text') {
			expect(inv.offset).toBe(3);
			expect(inv.text).toBe('test');
		}
	});

	it('delete_text throws without deletedText', () => {
		const op = deleteTextOp([0], 0, 2);
		expect(() => getInverseOperation(op)).toThrow('Cannot invert delete_text');
	});

	it('insert_node → delete_node', () => {
		const node = makeTextNode('n1', 'hi');
		const op = insertNodeOp([0], node);
		const inv = getInverseOperation(op);
		expect(inv.type).toBe('delete_node');
		if (inv.type === 'delete_node') {
			expect(inv.deletedNode!.id).toBe('n1');
		}
	});

	it('delete_node → insert_node', () => {
		const node = makeTextNode('n1', 'hi');
		const op = deleteNodeOp([0], node);
		const inv = getInverseOperation(op);
		expect(inv.type).toBe('insert_node');
		if (inv.type === 'insert_node') {
			expect(inv.node.id).toBe('n1');
		}
	});

	it('delete_node throws without deletedNode', () => {
		const op = deleteNodeOp([0]);
		expect(() => getInverseOperation(op)).toThrow('Cannot invert delete_node');
	});

	it('move_node swaps paths', () => {
		const op = moveNodeOp([0], [2]);
		const inv = getInverseOperation(op);
		expect(inv.type).toBe('move_node');
		if (inv.type === 'move_node') {
			expect(inv.fromPath).toEqual([2]);
			expect(inv.toPath).toEqual([0]);
		}
	});

	it('set_attr swaps value and previousValue', () => {
		const op = setAttrOp([0], 'level', 3, 1);
		const inv = getInverseOperation(op);
		expect(inv.type).toBe('set_attr');
		if (inv.type === 'set_attr') {
			expect(inv.value).toBe(1);
			expect(inv.previousValue).toBe(3);
		}
	});

	it('add_mark → remove_mark', () => {
		const op = addMarkOp([0], 0, 5, { type: 'bold' });
		const inv = getInverseOperation(op);
		expect(inv.type).toBe('remove_mark');
		if (inv.type === 'remove_mark') {
			expect(inv.markType).toBe('bold');
			expect(inv.startOffset).toBe(0);
			expect(inv.endOffset).toBe(5);
		}
	});

	it('remove_mark → add_mark', () => {
		const op = removeMarkOp([0], 0, 5, 'italic');
		const inv = getInverseOperation(op);
		expect(inv.type).toBe('add_mark');
		if (inv.type === 'add_mark') {
			expect(inv.mark.type).toBe('italic');
		}
	});

	it('split_node → merge_nodes (next sibling)', () => {
		const op = splitNodeOp([0, 2], 5);
		const inv = getInverseOperation(op);
		expect(inv.type).toBe('merge_nodes');
		if (inv.type === 'merge_nodes') {
			expect(inv.path).toEqual([0, 3]); // next sibling
			expect(inv.mergeOffset).toBe(5);
		}
	});

	it('merge_nodes → split_node (previous sibling)', () => {
		const op = mergeNodesOp([0, 3], 5);
		const inv = getInverseOperation(op);
		expect(inv.type).toBe('split_node');
		if (inv.type === 'split_node') {
			expect(inv.path).toEqual([0, 2]); // previous sibling
			expect(inv.offset).toBe(5);
		}
	});

	it('merge_nodes throws without mergeOffset', () => {
		const op = mergeNodesOp([1]);
		expect(() => getInverseOperation(op)).toThrow('Cannot invert merge_nodes');
	});
});

describe('getInverseOperations', () => {
	it('reverses order and inverts each', () => {
		const ops: Operation[] = [
			insertTextOp([0], 0, 'a'),
			insertTextOp([0], 1, 'b')
		];
		const inverses = getInverseOperations(ops);
		expect(inverses).toHaveLength(2);
		// reversed: second op inverted first
		expect(inverses[0].type).toBe('delete_text');
		expect(inverses[1].type).toBe('delete_text');
		if (inverses[0].type === 'delete_text') {
			expect(inverses[0].offset).toBe(1); // inverse of insert at 1
		}
		if (inverses[1].type === 'delete_text') {
			expect(inverses[1].offset).toBe(0); // inverse of insert at 0
		}
	});
});

// ============================================================================
// Operation Application
// ============================================================================

describe('applyOperation', () => {
	it('insert_text inserts into text node', () => {
		const doc = docWith(makeTextNode('p1', 'hello'));
		applyOperation(doc, insertTextOp([0], 5, ' world'));
		expect(doc.getNodeAtPath([0])!.text).toBe('hello world');
	});

	it('delete_text removes text and fills deletedText', () => {
		const doc = docWith(makeTextNode('p1', 'hello world'));
		const result = applyOperation(doc, deleteTextOp([0], 5, 6));
		expect(doc.getNodeAtPath([0])!.text).toBe('hello');
		expect(result.type === 'delete_text' && result.deletedText).toBe(' world');
	});

	it('insert_node adds a node', () => {
		const doc = docWith(makeTextNode('p1', 'first'));
		const newNode = makeTextNode('p2', 'second');
		applyOperation(doc, insertNodeOp([1], newNode));
		expect(doc.getContent()).toHaveLength(2);
		expect(doc.getNodeAtPath([1])!.text).toBe('second');
	});

	it('delete_node removes a node and fills deletedNode', () => {
		const doc = docWith(makeTextNode('p1', 'first'), makeTextNode('p2', 'second'));
		const result = applyOperation(doc, deleteNodeOp([1]));
		expect(doc.getContent()).toHaveLength(1);
		expect(result.type === 'delete_node' && result.deletedNode!.text).toBe('second');
	});

	it('set_attr updates node attrs and fills previousValue', () => {
		const heading: EditorNode = { id: 'h1', type: 'heading', attrs: { level: 1 }, content: null, text: 'Title' };
		const doc = docWith(heading);
		const result = applyOperation(doc, setAttrOp([0], 'level', 2));
		expect(doc.getNodeAtPath([0])!.attrs.level).toBe(2);
		expect(result.type === 'set_attr' && result.previousValue).toBe(1);
	});

	it('add_mark adds mark to node', () => {
		const doc = docWith(makeTextNode('p1', 'hello'));
		applyOperation(doc, addMarkOp([0], 0, 5, { type: 'bold' }));
		const node = doc.getNodeAtPath([0])!;
		expect(node.marks).toHaveLength(1);
		expect(node.marks![0].type).toBe('bold');
	});

	it('remove_mark removes mark from node', () => {
		const node = makeTextNode('p1', 'hello');
		node.marks = [{ type: 'bold' }, { type: 'italic' }];
		const doc = docWith(node);
		applyOperation(doc, removeMarkOp([0], 0, 5, 'bold'));
		const updated = doc.getNodeAtPath([0])!;
		expect(updated.marks).toHaveLength(1);
		expect(updated.marks![0].type).toBe('italic');
	});

	it('split_node splits text node', () => {
		const doc = docWith(makeTextNode('p1', 'hello world'));
		applyOperation(doc, splitNodeOp([0], 5));
		expect(doc.getContent()).toHaveLength(2);
		expect(doc.getNodeAtPath([0])!.text).toBe('hello');
		expect(doc.getNodeAtPath([1])!.text).toBe(' world');
	});

	it('split_node splits container node children', () => {
		const child1 = makeTextNode('c1', 'a');
		const child2 = makeTextNode('c2', 'b');
		const child3 = makeTextNode('c3', 'c');
		const container = makeContainerNode('list1', [child1, child2, child3]);
		const doc = docWith(container);
		applyOperation(doc, splitNodeOp([0], 1)); // split after first child
		expect(doc.getContent()).toHaveLength(2);
		expect(doc.getNodeAtPath([0])!.content).toHaveLength(1);
		expect(doc.getNodeAtPath([1])!.content).toHaveLength(2);
	});

	it('merge_nodes merges text nodes', () => {
		const doc = docWith(makeTextNode('p1', 'hello'), makeTextNode('p2', ' world'));
		const result = applyOperation(doc, mergeNodesOp([1]));
		expect(doc.getContent()).toHaveLength(1);
		expect(doc.getNodeAtPath([0])!.text).toBe('hello world');
		expect(result.type === 'merge_nodes' && result.mergeOffset).toBe(5);
	});

	it('merge_nodes merges container children', () => {
		const c1 = makeTextNode('c1', 'a');
		const c2 = makeTextNode('c2', 'b');
		const c3 = makeTextNode('c3', 'c');
		const list1 = makeContainerNode('l1', [c1]);
		const list2 = makeContainerNode('l2', [c2, c3]);
		const doc = docWith(list1, list2);
		const result = applyOperation(doc, mergeNodesOp([1]));
		expect(doc.getContent()).toHaveLength(1);
		expect(doc.getNodeAtPath([0])!.content).toHaveLength(3);
		expect(result.type === 'merge_nodes' && result.mergeOffset).toBe(1);
	});

	it('move_node moves from one position to another', () => {
		const doc = docWith(
			makeTextNode('p1', 'first'),
			makeTextNode('p2', 'second'),
			makeTextNode('p3', 'third')
		);
		applyOperation(doc, moveNodeOp([0], [2]));
		// After deleting [0], indices shift. "first" moves after "third"
		expect(doc.getNodeAtPath([0])!.text).toBe('second');
	});
});

describe('applyOperations', () => {
	it('applies multiple ops in order', () => {
		const doc = docWith(makeTextNode('p1', 'hello'));
		const ops: Operation[] = [
			insertTextOp([0], 5, ' world'),
			insertTextOp([0], 11, '!')
		];
		const results = applyOperations(doc, ops);
		expect(doc.getNodeAtPath([0])!.text).toBe('hello world!');
		expect(results).toHaveLength(2);
	});
});

// ============================================================================
// Path Utilities
// ============================================================================

describe('adjustPathAfterInsert', () => {
	it('increments same-level sibling after insert', () => {
		expect(adjustPathAfterInsert([2], [1])).toEqual([3]);
	});

	it('increments sibling at insert position', () => {
		expect(adjustPathAfterInsert([1], [1])).toEqual([2]);
	});

	it('does not adjust sibling before insert', () => {
		expect(adjustPathAfterInsert([0], [1])).toEqual([0]);
	});

	it('does not adjust path in different branch', () => {
		expect(adjustPathAfterInsert([0, 1], [1, 0])).toEqual([0, 1]);
	});

	it('adjusts nested path when ancestor shifts', () => {
		expect(adjustPathAfterInsert([2, 0], [1])).toEqual([3, 0]);
	});

	it('does not adjust deeper path in earlier branch', () => {
		expect(adjustPathAfterInsert([0, 3], [1])).toEqual([0, 3]);
	});
});

// ============================================================================
// Operation Validation
// ============================================================================

describe('isOperationValid', () => {
	it('insert_text valid for text node', () => {
		const doc = docWith(makeTextNode('p1', 'hello'));
		expect(isOperationValid(doc, insertTextOp([0], 0, 'x'))).toBe(true);
	});

	it('insert_text invalid for container node', () => {
		const doc = docWith(makeContainerNode('l1', [makeTextNode('c1', 'a')]));
		expect(isOperationValid(doc, insertTextOp([0], 0, 'x'))).toBe(false);
	});

	it('insert_text invalid for nonexistent path', () => {
		const doc = docWith(makeTextNode('p1', 'hello'));
		expect(isOperationValid(doc, insertTextOp([5], 0, 'x'))).toBe(false);
	});

	it('delete_node valid at root level', () => {
		const doc = docWith(makeTextNode('p1', 'hello'));
		expect(isOperationValid(doc, deleteNodeOp([0]))).toBe(true);
	});

	it('insert_node valid at root level', () => {
		const doc = docWith(makeTextNode('p1', 'hello'));
		expect(isOperationValid(doc, insertNodeOp([1], makeTextNode('p2', 'x')))).toBe(true);
	});

	it('move_node valid when source exists', () => {
		const doc = docWith(makeTextNode('p1', 'hello'), makeTextNode('p2', 'world'));
		expect(isOperationValid(doc, moveNodeOp([0], [1]))).toBe(true);
	});

	it('move_node invalid when source missing', () => {
		const doc = docWith(makeTextNode('p1', 'hello'));
		expect(isOperationValid(doc, moveNodeOp([5], [0]))).toBe(false);
	});

	it('set_attr valid for existing node', () => {
		const heading: EditorNode = { id: 'h1', type: 'heading', attrs: { level: 1 }, content: null, text: 'Title' };
		const doc = docWith(heading);
		expect(isOperationValid(doc, setAttrOp([0], 'level', 2))).toBe(true);
	});

	it('split_node valid for existing node', () => {
		const doc = docWith(makeTextNode('p1', 'hello'));
		expect(isOperationValid(doc, splitNodeOp([0], 3))).toBe(true);
	});

	it('split_node invalid for nonexistent node', () => {
		const doc = docWith(makeTextNode('p1', 'hello'));
		expect(isOperationValid(doc, splitNodeOp([9], 3))).toBe(false);
	});
});

// ============================================================================
// Compound Operations
// ============================================================================

describe('replaceTextOps', () => {
	it('generates delete + insert for replacement', () => {
		const ops = replaceTextOps([0], 0, 5, 'world', 'hello');
		expect(ops).toHaveLength(2);
		expect(ops[0].type).toBe('delete_text');
		expect(ops[1].type).toBe('insert_text');
	});

	it('generates only insert when range is empty', () => {
		const ops = replaceTextOps([0], 3, 3, 'new');
		expect(ops).toHaveLength(1);
		expect(ops[0].type).toBe('insert_text');
	});

	it('generates only delete when replacement is empty', () => {
		const ops = replaceTextOps([0], 0, 5, '', 'hello');
		expect(ops).toHaveLength(1);
		expect(ops[0].type).toBe('delete_text');
	});

	it('returns empty array when both are empty', () => {
		const ops = replaceTextOps([0], 3, 3, '');
		expect(ops).toHaveLength(0);
	});
});

describe('wrapNodesOps', () => {
	it('returns empty for no paths', () => {
		const wrapper = makeContainerNode('w1', []);
		expect(wrapNodesOps([], wrapper)).toHaveLength(0);
	});

	it('generates delete ops + one insert for wrapper', () => {
		const wrapper = makeContainerNode('w1', []);
		const ops = wrapNodesOps([[0], [1]], wrapper);
		// 2 deletes (reverse order) + 1 insert
		expect(ops).toHaveLength(3);
		expect(ops[0].type).toBe('delete_node');
		expect(ops[1].type).toBe('delete_node');
		expect(ops[2].type).toBe('insert_node');
	});

	it('inserts wrapper at first path', () => {
		const wrapper = makeContainerNode('w1', []);
		const ops = wrapNodesOps([[1], [2]], wrapper);
		const insertOp = ops[ops.length - 1];
		if (insertOp.type === 'insert_node') {
			expect(insertOp.path).toEqual([1]);
		}
	});
});

describe('unwrapNodeOps', () => {
	it('generates a delete op for the container', () => {
		const ops = unwrapNodeOps([0]);
		expect(ops).toHaveLength(1);
		expect(ops[0].type).toBe('delete_node');
	});
});
