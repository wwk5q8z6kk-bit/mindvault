/**
 * Unit tests for the Document model.
 */

import { describe, it, expect, beforeEach } from 'vitest';
import {
	Document,
	createParagraph,
	createHeading,
	createCodeBlock,
	createListItem,
	createList,
	createBlockquote
} from '../document';

describe('Document', () => {
	describe('constructor', () => {
		it('should create a document with default paragraph', () => {
			const doc = new Document();
			expect(doc.getContent()).toHaveLength(1);
			expect(doc.getContent()[0].type).toBe('paragraph');
		});

		it('should create a document with initial content', () => {
			const paragraph = createParagraph('Hello world');
			const doc = new Document([paragraph]);
			expect(doc.getContent()).toHaveLength(1);
			expect(doc.getContent()[0].text).toBe('Hello world');
		});
	});

	describe('node creation', () => {
		it('should create a paragraph node', () => {
			const p = createParagraph('Test paragraph');
			expect(p.type).toBe('paragraph');
			expect(p.text).toBe('Test paragraph');
			expect(p.id).toBeDefined();
		});

		it('should create a heading node', () => {
			const h1 = createHeading(1, 'Main Title');
			expect(h1.type).toBe('heading');
			expect(h1.attrs.level).toBe(1);
			expect(h1.text).toBe('Main Title');

			const h3 = createHeading(3, 'Subsection');
			expect(h3.attrs.level).toBe(3);
		});

		it('should create a code block node', () => {
			const code = createCodeBlock('typescript', 'const x = 1;');
			expect(code.type).toBe('code-block');
			expect(code.text).toBe('const x = 1;');
			expect(code.attrs.language).toBe('typescript');
		});

		it('should create a list node', () => {
			const items = [
				createListItem([createParagraph('Item 1')]),
				createListItem([createParagraph('Item 2')]),
				createListItem([createParagraph('Item 3')])
			];
			const list = createList(false, items);
			expect(list.type).toBe('list');
			expect(list.attrs.ordered).toBe(false);
			expect(list.content).toHaveLength(3);
		});

		it('should create an ordered list', () => {
			const items = [
				createListItem([createParagraph('First')]),
				createListItem([createParagraph('Second')])
			];
			const list = createList(true, items);
			expect(list.attrs.ordered).toBe(true);
		});

		it('should create a blockquote node', () => {
			const quote = createBlockquote([createParagraph('Quoted text')]);
			expect(quote.type).toBe('blockquote');
			expect(quote.content).toHaveLength(1);
		});
	});

	describe('getNodeAtPath', () => {
		it('should get node at root level', () => {
			const doc = new Document([
				createParagraph('First'),
				createParagraph('Second')
			]);
			const node = doc.getNodeAtPath([0]);
			expect(node?.text).toBe('First');
		});

		it('should get nested node', () => {
			const list = createList(false, [
				createListItem([createParagraph('Item 1')]),
				createListItem([createParagraph('Item 2')])
			]);
			const doc = new Document([list]);

			const node = doc.getNodeAtPath([0, 0]);
			expect(node?.type).toBe('list-item');
		});

		it('should return null for invalid path', () => {
			const doc = new Document([createParagraph('Only one')]);
			const node = doc.getNodeAtPath([10]);
			expect(node).toBeNull();
		});

		it('should return null for empty path', () => {
			const doc = new Document();
			const node = doc.getNodeAtPath([]);
			expect(node).toBeNull();
		});
	});

	describe('findPathById', () => {
		it('should find path to a node by ID', () => {
			const target = createParagraph('Target');
			const doc = new Document([
				createParagraph('Other'),
				target
			]);

			const path = doc.findPathById(target.id);
			expect(path).toEqual([1]);
		});

		it('should find nested nodes', () => {
			const item = createListItem([createParagraph('Target item')]);
			const list = createList(false, [item]);
			const doc = new Document([list]);

			const path = doc.findPathById(item.id);
			expect(path).toEqual([0, 0]);
		});

		it('should return null for non-existent ID', () => {
			const doc = new Document();
			const path = doc.findPathById('non-existent');
			expect(path).toBeNull();
		});
	});

	describe('resolvePath', () => {
		it('should resolve a valid path', () => {
			const doc = new Document([
				createParagraph('First'),
				createParagraph('Second')
			]);

			const result = doc.resolvePath([1]);
			expect(result?.node.text).toBe('Second');
			expect(result?.index).toBe(1);
		});

		it('should return null for empty path', () => {
			const doc = new Document();
			const result = doc.resolvePath([]);
			expect(result).toBeNull();
		});

		it('should return null for invalid path', () => {
			const doc = new Document([createParagraph('Only one')]);
			const result = doc.resolvePath([10]);
			expect(result).toBeNull();
		});
	});

	describe('getWordCount', () => {
		it('should return 0 for empty paragraph', () => {
			const doc = new Document();
			expect(doc.getWordCount()).toBe(0);
		});

		it('should count words correctly', () => {
			const doc = new Document([createParagraph('Hello world test')]);
			expect(doc.getWordCount()).toBe(3);
		});
	});

	describe('getVersion', () => {
		it('should start at version 0', () => {
			const doc = new Document();
			expect(doc.getVersion()).toBe(0);
		});
	});

	describe('getSelection', () => {
		it('should return null initially', () => {
			const doc = new Document();
			expect(doc.getSelection()).toBeNull();
		});
	});
});
