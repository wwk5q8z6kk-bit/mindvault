/**
 * Round-trip tests for Markdown parser and serializer.
 * Verifies that markdown -> AST -> markdown preserves content.
 */

import { describe, it, expect } from 'vitest';
import { parseMarkdown, serializeMarkdown, normalizeMarkdown } from '../index';

describe('Markdown Round-trip', () => {
	function roundtrip(input: string): string {
		return normalizeMarkdown(input);
	}

	describe('headings', () => {
		it('should preserve h1', () => {
			const result = roundtrip('# Heading 1');
			expect(result).toBe('# Heading 1');
		});

		it('should preserve all heading levels', () => {
			for (let i = 1; i <= 6; i++) {
				const prefix = '#'.repeat(i);
				const result = roundtrip(`${prefix} Level ${i}`);
				expect(result).toBe(`${prefix} Level ${i}`);
			}
		});
	});

	describe('paragraphs', () => {
		it('should preserve simple paragraph', () => {
			const result = roundtrip('This is a paragraph.');
			expect(result).toBe('This is a paragraph.');
		});

		it('should preserve multiple paragraphs', () => {
			const input = 'First paragraph.\n\nSecond paragraph.';
			const result = roundtrip(input);
			expect(result).toContain('First paragraph.');
			expect(result).toContain('Second paragraph.');
		});
	});

	describe('code blocks', () => {
		it('should preserve code block', () => {
			const input = '```\ncode\n```';
			const result = roundtrip(input);
			expect(result).toContain('```');
			expect(result).toContain('code');
		});

		it('should preserve language', () => {
			const input = '```javascript\nconst x = 1;\n```';
			const result = roundtrip(input);
			expect(result).toContain('javascript');
			expect(result).toContain('const x = 1;');
		});
	});

	describe('blockquotes', () => {
		it('should preserve blockquote', () => {
			const input = '> This is quoted';
			const result = roundtrip(input);
			expect(result).toContain('>');
			expect(result).toContain('This is quoted');
		});
	});

	describe('lists', () => {
		it('should preserve unordered list', () => {
			const input = '- Item 1\n- Item 2';
			const result = roundtrip(input);
			expect(result).toContain('- Item 1');
			expect(result).toContain('- Item 2');
		});

		it('should preserve ordered list', () => {
			const input = '1. First\n2. Second';
			const result = roundtrip(input);
			expect(result).toContain('1.');
			expect(result).toContain('2.');
		});

		it('should preserve task list', () => {
			const input = '- [ ] Todo\n- [x] Done';
			const result = roundtrip(input);
			expect(result).toContain('[ ]');
			expect(result).toContain('[x]');
		});
	});

	describe('inline formatting', () => {
		it('should preserve bold', () => {
			const result = roundtrip('**bold text**');
			expect(result).toContain('**bold text**');
		});

		it('should preserve italic', () => {
			const result = roundtrip('*italic text*');
			expect(result).toContain('*italic text*');
		});

		it('should preserve inline code', () => {
			const result = roundtrip('`code`');
			expect(result).toContain('`code`');
		});

		it('should preserve strikethrough', () => {
			const result = roundtrip('~~strike~~');
			expect(result).toContain('~~strike~~');
		});

		it('should preserve links', () => {
			const result = roundtrip('[text](https://example.com)');
			expect(result).toContain('[text]');
			expect(result).toContain('https://example.com');
		});

		it('should preserve images', () => {
			const result = roundtrip('![alt](image.png)');
			expect(result).toContain('![alt]');
			expect(result).toContain('image.png');
		});
	});

	describe('wiki links', () => {
		it('should preserve simple wiki link', () => {
			const result = roundtrip('[[Target]]');
			expect(result).toBe('[[Target]]');
		});

		it('should preserve wiki link with alias', () => {
			const result = roundtrip('[[Target|Alias]]');
			expect(result).toBe('[[Target|Alias]]');
		});
	});

	describe('math', () => {
		it('should preserve inline math', () => {
			const result = roundtrip('$x^2$');
			expect(result).toContain('$x^2$');
		});

		it('should preserve block math', () => {
			const input = '$$\nx^2 + y^2 = z^2\n$$';
			const result = roundtrip(input);
			expect(result).toContain('$$');
			expect(result).toContain('x^2 + y^2 = z^2');
		});
	});

	describe('horizontal rule', () => {
		it('should preserve hr', () => {
			const result = roundtrip('---');
			expect(result).toBe('---');
		});
	});

	describe('complex documents', () => {
		it('should preserve mixed content', () => {
			const input = `# Title

This is a **bold** paragraph with *italic* and \`code\`.

## Subsection

- List item 1
- List item 2

> A quote

\`\`\`js
const x = 1;
\`\`\`

[[Wiki Link]]`;

			const result = roundtrip(input);
			expect(result).toContain('# Title');
			expect(result).toContain('**bold**');
			expect(result).toContain('*italic*');
			expect(result).toContain('## Subsection');
			expect(result).toContain('- List item');
			expect(result).toContain('>');
			expect(result).toContain('```');
			expect(result).toContain('[[Wiki Link]]');
		});
	});
});

describe('AST operations', () => {
	it('should parse to valid AST', () => {
		const ast = parseMarkdown('# Hello\n\nWorld');
		expect(ast.type).toBe('document');
		expect(ast.children).toHaveLength(2);
		expect(ast.children![0].type).toBe('heading');
		expect(ast.children![1].type).toBe('paragraph');
	});

	it('should serialize AST to markdown', () => {
		const ast = {
			type: 'document' as const,
			children: [
				{ type: 'heading' as const, level: 1, children: [{ type: 'text' as const, value: 'Title' }] },
				{ type: 'paragraph' as const, children: [{ type: 'text' as const, value: 'Content' }] }
			]
		};
		const result = serializeMarkdown(ast);
		expect(result).toContain('# Title');
		expect(result).toContain('Content');
	});
});
