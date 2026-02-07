/**
 * Unit tests for the Markdown tokenizer.
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { Tokenizer, createTokenizer } from '../tokenizer';

describe('Tokenizer', () => {
	let tokenizer: Tokenizer;

	beforeEach(() => {
		tokenizer = createTokenizer();
	});

	describe('headings', () => {
		it('should tokenize ATX headings', () => {
			const tokens = tokenizer.tokenize('# Heading 1');
			expect(tokens).toHaveLength(1);
			expect(tokens[0].type).toBe('heading');
			expect(tokens[0].depth).toBe(1);
			expect(tokens[0].text).toBe('Heading 1');
		});

		it('should tokenize h1-h6', () => {
			for (let i = 1; i <= 6; i++) {
				const prefix = '#'.repeat(i);
				const tokens = tokenizer.tokenize(`${prefix} Level ${i}`);
				expect(tokens[0].depth).toBe(i);
			}
		});

		it('should handle trailing hashes', () => {
			const tokens = tokenizer.tokenize('## Heading ##');
			expect(tokens[0].text).toBe('Heading');
		});

		it('should tokenize setext h1', () => {
			const tokens = tokenizer.tokenize('Heading\n===');
			expect(tokens[0].type).toBe('heading');
			expect(tokens[0].depth).toBe(1);
		});

		it('should tokenize setext h2', () => {
			const tokens = tokenizer.tokenize('Heading\n---');
			expect(tokens[0].type).toBe('heading');
			expect(tokens[0].depth).toBe(2);
		});
	});

	describe('paragraphs', () => {
		it('should tokenize simple paragraph', () => {
			const tokens = tokenizer.tokenize('This is a paragraph.');
			expect(tokens).toHaveLength(1);
			expect(tokens[0].type).toBe('paragraph');
		});

		it('should tokenize multi-line paragraph', () => {
			const tokens = tokenizer.tokenize('Line one\nLine two');
			expect(tokens).toHaveLength(1);
			expect(tokens[0].type).toBe('paragraph');
		});

		it('should split paragraphs on blank lines', () => {
			const tokens = tokenizer.tokenize('First paragraph\n\nSecond paragraph');
			expect(tokens).toHaveLength(2);
			expect(tokens[0].type).toBe('paragraph');
			expect(tokens[1].type).toBe('paragraph');
		});
	});

	describe('code blocks', () => {
		it('should tokenize fenced code block with backticks', () => {
			const tokens = tokenizer.tokenize('```\ncode here\n```');
			expect(tokens).toHaveLength(1);
			expect(tokens[0].type).toBe('code_block');
			expect(tokens[0].text).toBe('code here');
		});

		it('should tokenize fenced code block with language', () => {
			const tokens = tokenizer.tokenize('```javascript\nconst x = 1;\n```');
			expect(tokens[0].type).toBe('code_block');
			expect(tokens[0].lang).toBe('javascript');
			expect(tokens[0].text).toBe('const x = 1;');
		});

		it('should tokenize fenced code block with tildes', () => {
			const tokens = tokenizer.tokenize('~~~\ncode\n~~~');
			expect(tokens[0].type).toBe('code_block');
		});

		it('should handle multi-line code', () => {
			const code = '```\nline 1\nline 2\nline 3\n```';
			const tokens = tokenizer.tokenize(code);
			expect(tokens[0].text).toBe('line 1\nline 2\nline 3');
		});
	});

	describe('blockquotes', () => {
		it('should tokenize simple blockquote', () => {
			const tokens = tokenizer.tokenize('> Quote');
			expect(tokens).toHaveLength(1);
			expect(tokens[0].type).toBe('blockquote');
		});

		it('should tokenize multi-line blockquote', () => {
			const tokens = tokenizer.tokenize('> Line 1\n> Line 2');
			expect(tokens[0].type).toBe('blockquote');
		});

		it('should handle nested content', () => {
			const tokens = tokenizer.tokenize('> # Heading in quote');
			expect(tokens[0].type).toBe('blockquote');
			expect(tokens[0].tokens).toBeDefined();
		});
	});

	describe('horizontal rules', () => {
		it('should tokenize hr with dashes', () => {
			const tokens = tokenizer.tokenize('---');
			expect(tokens[0].type).toBe('hr');
		});

		it('should tokenize hr with asterisks', () => {
			const tokens = tokenizer.tokenize('***');
			expect(tokens[0].type).toBe('hr');
		});

		it('should tokenize hr with underscores', () => {
			const tokens = tokenizer.tokenize('___');
			expect(tokens[0].type).toBe('hr');
		});
	});

	describe('lists', () => {
		it('should tokenize unordered list', () => {
			const tokens = tokenizer.tokenize('- Item 1\n- Item 2');
			expect(tokens).toHaveLength(1);
			expect(tokens[0].type).toBe('list');
			expect(tokens[0].ordered).toBe(false);
			expect(tokens[0].tokens).toHaveLength(2);
		});

		it('should tokenize ordered list', () => {
			const tokens = tokenizer.tokenize('1. First\n2. Second');
			expect(tokens[0].type).toBe('list');
			expect(tokens[0].ordered).toBe(true);
		});

		it('should handle different bullet markers', () => {
			expect(tokenizer.tokenize('* Item')[0].type).toBe('list');
			expect(tokenizer.tokenize('+ Item')[0].type).toBe('list');
			expect(tokenizer.tokenize('- Item')[0].type).toBe('list');
		});

		it('should tokenize task list', () => {
			const tokens = tokenizer.tokenize('- [ ] Unchecked\n- [x] Checked');
			expect(tokens[0].type).toBe('list');
			expect(tokens[0].tokens![0].type).toBe('task_item');
			expect(tokens[0].tokens![0].checked).toBe(false);
			expect(tokens[0].tokens![1].checked).toBe(true);
		});
	});

	describe('tables', () => {
		it('should tokenize simple table', () => {
			const table = '| A | B |\n|---|---|\n| 1 | 2 |';
			const tokens = tokenizer.tokenize(table);
			expect(tokens[0].type).toBe('table');
		});

		it('should parse column alignment', () => {
			const table = '| Left | Center | Right |\n|:---|:---:|---:|\n| 1 | 2 | 3 |';
			const tokens = tokenizer.tokenize(table);
			expect(tokens[0].align).toEqual(['left', 'center', 'right']);
		});
	});

	describe('math blocks', () => {
		it('should tokenize math block', () => {
			const tokens = tokenizer.tokenize('$$\nx^2\n$$');
			expect(tokens[0].type).toBe('math_block');
			expect(tokens[0].text).toBe('x^2');
		});
	});

	describe('inline tokenization', () => {
		it('should tokenize bold', () => {
			const tokens = tokenizer.tokenizeInline('**bold**');
			expect(tokens[0].type).toBe('bold');
			expect(tokens[0].text).toBe('bold');
		});

		it('should tokenize italic', () => {
			const tokens = tokenizer.tokenizeInline('*italic*');
			expect(tokens[0].type).toBe('italic');
		});

		it('should tokenize code', () => {
			const tokens = tokenizer.tokenizeInline('`code`');
			expect(tokens[0].type).toBe('code');
			expect(tokens[0].text).toBe('code');
		});

		it('should tokenize strikethrough', () => {
			const tokens = tokenizer.tokenizeInline('~~strike~~');
			expect(tokens[0].type).toBe('strikethrough');
		});

		it('should tokenize links', () => {
			const tokens = tokenizer.tokenizeInline('[text](url)');
			expect(tokens[0].type).toBe('link');
			expect(tokens[0].text).toBe('text');
			expect(tokens[0].href).toBe('url');
		});

		it('should tokenize links with title', () => {
			const tokens = tokenizer.tokenizeInline('[text](url "title")');
			expect(tokens[0].title).toBe('title');
		});

		it('should tokenize images', () => {
			const tokens = tokenizer.tokenizeInline('![alt](src)');
			expect(tokens[0].type).toBe('image');
			expect(tokens[0].alt).toBe('alt');
			expect(tokens[0].href).toBe('src');
		});

		it('should tokenize wiki links', () => {
			const tokens = tokenizer.tokenizeInline('[[Target]]');
			expect(tokens[0].type).toBe('wiki_link');
			expect(tokens[0].target).toBe('Target');
		});

		it('should tokenize wiki links with alias', () => {
			const tokens = tokenizer.tokenizeInline('[[Target|Display]]');
			expect(tokens[0].target).toBe('Target');
			expect(tokens[0].alias).toBe('Display');
		});

		it('should tokenize inline math', () => {
			const tokens = tokenizer.tokenizeInline('$x^2$');
			expect(tokens[0].type).toBe('math_inline');
			expect(tokens[0].text).toBe('x^2');
		});

		it('should handle escaped characters', () => {
			const tokens = tokenizer.tokenizeInline('\\*not bold\\*');
			expect(tokens.some(t => t.type === 'bold')).toBe(false);
		});

		it('should handle mixed inline content', () => {
			const tokens = tokenizer.tokenizeInline('**bold** and *italic*');
			expect(tokens.filter(t => t.type === 'bold')).toHaveLength(1);
			expect(tokens.filter(t => t.type === 'italic')).toHaveLength(1);
		});
	});
});
