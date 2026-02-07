/**
 * Markdown tokenizer.
 * Converts markdown text into tokens for parsing.
 */

import type { Token, ParserOptions, BlockExtension, InlineExtension } from './types';
import { defaultParserOptions } from './types';

// ============================================================================
// Block Patterns
// ============================================================================

const PATTERNS = {
	// Headings: # to ######
	heading: /^(#{1,6})\s+(.+?)(?:\s+#+)?$/,
	// ATX heading (alternate)
	setextHeading1: /^(.+)\n={3,}$/,
	setextHeading2: /^(.+)\n-{3,}$/,
	// Code block (fenced)
	codeBlockStart: /^(`{3,}|~{3,})(\w*)?$/,
	// Blockquote
	blockquote: /^>\s?(.*)$/,
	// Horizontal rule
	hr: /^(?:[-*_]){3,}\s*$/,
	// Unordered list item
	ulItem: /^([\t ]*)([*+-])\s+(.*)$/,
	// Ordered list item
	olItem: /^([\t ]*)(\d+)([.)])\s+(.*)$/,
	// Task list item
	taskItem: /^([\t ]*)([*+-])\s+\[([ xX])\]\s+(.*)$/,
	// Table separator
	tableSeparator: /^\|?(?:\s*:?-+:?\s*\|)+\s*:?-+:?\s*\|?\s*$/,
	// Table row
	tableRow: /^\|(.+)\|?\s*$/,
	// HTML block (simplified)
	htmlBlock: /^<(\/?[a-zA-Z][a-zA-Z0-9]*)[^>]*>/,
	// Blank line
	blank: /^\s*$/,
	// Math block
	mathBlockStart: /^\$\$\s*$/
};

// ============================================================================
// Inline Patterns
// ============================================================================

const INLINE_PATTERNS = {
	// Escape
	escape: /^\\([\\`*_{}[\]()#+\-.!|])/,
	// Code span
	code: /^(`+)([^`]|[^`][\s\S]*?[^`])\1(?!`)/,
	// Bold (strong)
	bold: /^\*\*(?=\S)([\s\S]*?\S)\*\*(?!\*)|^__(?=\S)([\s\S]*?\S)__(?!_)/,
	// Italic (emphasis)
	italic: /^\*(?=\S)([\s\S]*?\S)\*(?!\*)|^_(?=\S)([\s\S]*?\S)_(?!_)/,
	// Strikethrough
	strikethrough: /^~~(?=\S)([\s\S]*?\S)~~/,
	// Link
	link: /^\[([^\]]+)\]\(([^)\s]+)(?:\s+"([^"]*)")?\)/,
	// Image
	image: /^!\[([^\]]*)\]\(([^)\s]+)(?:\s+"([^"]*)")?\)/,
	// Wiki-link
	wikiLink: /^\[\[([^\]|]+)(?:\|([^\]]+))?\]\]/,
	// Math inline
	mathInline: /^\$([^$\n]+)\$/,
	// Auto-link
	autolink: /^<([a-zA-Z][a-zA-Z0-9+.-]*:[^\s>]+)>/,
	// Line break (two spaces + newline)
	lineBreak: /^  \n/,
	// Soft break
	softBreak: /^\n/
};

// ============================================================================
// Tokenizer Class
// ============================================================================

export class Tokenizer {
	private options: ParserOptions;
	private blockExtensions: BlockExtension[];
	private inlineExtensions: InlineExtension[];

	constructor(options: Partial<ParserOptions> = {}) {
		this.options = { ...defaultParserOptions, ...options };
		this.blockExtensions = this.options.blockExtensions || [];
		this.inlineExtensions = (this.options.inlineExtensions || []).sort(
			(a, b) => (b.priority || 0) - (a.priority || 0)
		);
	}

	/**
	 * Tokenize markdown text into block tokens.
	 */
	tokenize(markdown: string): Token[] {
		const lines = markdown.split('\n');
		const tokens: Token[] = [];
		let i = 0;

		while (i < lines.length) {
			const result = this.tokenizeBlock(lines, i);
			if (result) {
				tokens.push(result.token);
				i += result.linesConsumed;
			} else {
				// Skip blank lines
				i++;
			}
		}

		return tokens;
	}

	/**
	 * Tokenize a block starting at the given line index.
	 */
	private tokenizeBlock(
		lines: string[],
		startIndex: number
	): { token: Token; linesConsumed: number } | null {
		const line = lines[startIndex];
		const nextLine = lines[startIndex + 1];

		// Skip blank lines
		if (PATTERNS.blank.test(line)) {
			return null;
		}

		// Try custom block extensions first
		for (const ext of this.blockExtensions) {
			if (ext.match(line, nextLine)) {
				const result = ext.parse(lines, startIndex);
				if (result) return result;
			}
		}

		// Heading
		const headingMatch = line.match(PATTERNS.heading);
		if (headingMatch) {
			return {
				token: {
					type: 'heading',
					raw: line,
					depth: headingMatch[1].length,
					text: headingMatch[2],
					tokens: this.tokenizeInline(headingMatch[2])
				},
				linesConsumed: 1
			};
		}

		// Setext heading (h1)
		if (nextLine && /^={3,}\s*$/.test(nextLine)) {
			return {
				token: {
					type: 'heading',
					raw: line + '\n' + nextLine,
					depth: 1,
					text: line,
					tokens: this.tokenizeInline(line)
				},
				linesConsumed: 2
			};
		}

		// Setext heading (h2)
		if (nextLine && /^-{3,}\s*$/.test(nextLine) && !PATTERNS.ulItem.test(line)) {
			return {
				token: {
					type: 'heading',
					raw: line + '\n' + nextLine,
					depth: 2,
					text: line,
					tokens: this.tokenizeInline(line)
				},
				linesConsumed: 2
			};
		}

		// Horizontal rule
		if (PATTERNS.hr.test(line)) {
			return {
				token: { type: 'hr', raw: line },
				linesConsumed: 1
			};
		}

		// Code block (fenced)
		const codeMatch = line.match(PATTERNS.codeBlockStart);
		if (codeMatch) {
			return this.tokenizeCodeBlock(lines, startIndex, codeMatch[1], codeMatch[2] || '');
		}

		// Math block
		if (this.options.math && PATTERNS.mathBlockStart.test(line)) {
			return this.tokenizeMathBlock(lines, startIndex);
		}

		// Blockquote
		if (PATTERNS.blockquote.test(line)) {
			return this.tokenizeBlockquote(lines, startIndex);
		}

		// Task list item (must check before regular list)
		if (this.options.taskLists) {
			const taskMatch = line.match(PATTERNS.taskItem);
			if (taskMatch) {
				return this.tokenizeList(lines, startIndex, false, true);
			}
		}

		// Unordered list
		if (PATTERNS.ulItem.test(line)) {
			return this.tokenizeList(lines, startIndex, false);
		}

		// Ordered list
		const olMatch = line.match(PATTERNS.olItem);
		if (olMatch) {
			return this.tokenizeList(lines, startIndex, true);
		}

		// Table
		if (this.options.tables && this.isTableStart(lines, startIndex)) {
			return this.tokenizeTable(lines, startIndex);
		}

		// Paragraph (default)
		return this.tokenizeParagraph(lines, startIndex);
	}

	/**
	 * Tokenize a fenced code block.
	 */
	private tokenizeCodeBlock(
		lines: string[],
		startIndex: number,
		fence: string,
		lang: string
	): { token: Token; linesConsumed: number } {
		const codeLines: string[] = [];
		let i = startIndex + 1;

		while (i < lines.length) {
			const line = lines[i];
			if (line.startsWith(fence) && line.trim() === fence) {
				break;
			}
			codeLines.push(line);
			i++;
		}

		const raw = lines.slice(startIndex, i + 1).join('\n');
		return {
			token: {
				type: 'code_block',
				raw,
				lang: lang || undefined,
				text: codeLines.join('\n')
			},
			linesConsumed: i - startIndex + 1
		};
	}

	/**
	 * Tokenize a math block ($$...$$).
	 */
	private tokenizeMathBlock(
		lines: string[],
		startIndex: number
	): { token: Token; linesConsumed: number } {
		const mathLines: string[] = [];
		let i = startIndex + 1;

		while (i < lines.length) {
			const line = lines[i];
			if (/^\$\$\s*$/.test(line)) {
				break;
			}
			mathLines.push(line);
			i++;
		}

		const raw = lines.slice(startIndex, i + 1).join('\n');
		return {
			token: {
				type: 'math_block',
				raw,
				text: mathLines.join('\n')
			},
			linesConsumed: i - startIndex + 1
		};
	}

	/**
	 * Tokenize a blockquote.
	 */
	private tokenizeBlockquote(
		lines: string[],
		startIndex: number
	): { token: Token; linesConsumed: number } {
		const quoteLines: string[] = [];
		let i = startIndex;

		while (i < lines.length) {
			const line = lines[i];
			const match = line.match(PATTERNS.blockquote);
			if (match) {
				quoteLines.push(match[1]);
				i++;
			} else if (PATTERNS.blank.test(line)) {
				// Check if next non-blank line continues the quote
				let j = i + 1;
				while (j < lines.length && PATTERNS.blank.test(lines[j])) j++;
				if (j < lines.length && PATTERNS.blockquote.test(lines[j])) {
					quoteLines.push('');
					i++;
				} else {
					break;
				}
			} else {
				break;
			}
		}

		const raw = lines.slice(startIndex, i).join('\n');
		const innerContent = quoteLines.join('\n');

		return {
			token: {
				type: 'blockquote',
				raw,
				tokens: this.tokenize(innerContent)
			},
			linesConsumed: i - startIndex
		};
	}

	/**
	 * Tokenize a list (ordered or unordered).
	 */
	private tokenizeList(
		lines: string[],
		startIndex: number,
		ordered: boolean,
		isTaskList = false
	): { token: Token; linesConsumed: number } {
		const items: Token[] = [];
		let i = startIndex;
		let startNum = 1;

		if (ordered) {
			const match = lines[startIndex].match(PATTERNS.olItem);
			if (match) {
				startNum = parseInt(match[2], 10);
			}
		}

		while (i < lines.length) {
			const line = lines[i];

			// Check for task item
			if (isTaskList || this.options.taskLists) {
				const taskMatch = line.match(PATTERNS.taskItem);
				if (taskMatch) {
					const checked = taskMatch[3].toLowerCase() === 'x';
					items.push({
						type: 'task_item',
						raw: line,
						checked,
						text: taskMatch[4],
						tokens: this.tokenizeInline(taskMatch[4])
					});
					i++;
					continue;
				}
			}

			// Check for list item
			const ulMatch = line.match(PATTERNS.ulItem);
			const olMatch = line.match(PATTERNS.olItem);

			if ((ordered && olMatch) || (!ordered && ulMatch)) {
				const match = ordered ? olMatch! : ulMatch!;
				const text = ordered ? match[4] : match[3];
				items.push({
					type: 'list_item',
					raw: line,
					text,
					tokens: this.tokenizeInline(text)
				});
				i++;
			} else if (PATTERNS.blank.test(line)) {
				// Blank line might continue the list
				let j = i + 1;
				while (j < lines.length && PATTERNS.blank.test(lines[j])) j++;
				if (j < lines.length) {
					const nextLine = lines[j];
					const hasListItem =
						(ordered && PATTERNS.olItem.test(nextLine)) ||
						(!ordered && (PATTERNS.ulItem.test(nextLine) || PATTERNS.taskItem.test(nextLine)));
					if (hasListItem) {
						i++;
						continue;
					}
				}
				break;
			} else {
				break;
			}
		}

		const raw = lines.slice(startIndex, i).join('\n');
		return {
			token: {
				type: 'list',
				raw,
				ordered,
				start: ordered ? startNum : undefined,
				tokens: items
			},
			linesConsumed: i - startIndex
		};
	}

	/**
	 * Check if a table starts at the given line.
	 */
	private isTableStart(lines: string[], startIndex: number): boolean {
		if (startIndex + 1 >= lines.length) return false;
		const line = lines[startIndex];
		const nextLine = lines[startIndex + 1];
		return PATTERNS.tableRow.test(line) && PATTERNS.tableSeparator.test(nextLine);
	}

	/**
	 * Tokenize a table.
	 */
	private tokenizeTable(
		lines: string[],
		startIndex: number
	): { token: Token; linesConsumed: number } {
		const headerLine = lines[startIndex];
		const separatorLine = lines[startIndex + 1];

		// Parse alignment from separator
		const alignments: ('left' | 'center' | 'right' | null)[] = [];
		const sepCells = separatorLine.split('|').filter((c) => c.trim());
		for (const cell of sepCells) {
			const trimmed = cell.trim();
			const leftColon = trimmed.startsWith(':');
			const rightColon = trimmed.endsWith(':');
			if (leftColon && rightColon) {
				alignments.push('center');
			} else if (rightColon) {
				alignments.push('right');
			} else if (leftColon) {
				alignments.push('left');
			} else {
				alignments.push(null);
			}
		}

		// Parse header
		const headerCells = this.parseTableRow(headerLine);

		// Parse body rows
		const rows: Token[][] = [];
		let i = startIndex + 2;
		while (i < lines.length && PATTERNS.tableRow.test(lines[i])) {
			rows.push(this.parseTableRow(lines[i]));
			i++;
		}

		const raw = lines.slice(startIndex, i).join('\n');
		return {
			token: {
				type: 'table',
				raw,
				align: alignments,
				tokens: [
					{
						type: 'table_row',
						raw: headerLine,
						header: true,
						cells: [headerCells]
					},
					...rows.map((cells, idx) => ({
						type: 'table_row' as const,
						raw: lines[startIndex + 2 + idx],
						header: false,
						cells: [cells]
					}))
				]
			},
			linesConsumed: i - startIndex
		};
	}

	/**
	 * Parse a table row into cell tokens.
	 */
	private parseTableRow(line: string): Token[] {
		const cells = line
			.replace(/^\||\|$/g, '')
			.split('|')
			.map((cell) => cell.trim());

		return cells.map((cell) => ({
			type: 'table_cell' as const,
			raw: cell,
			text: cell,
			tokens: this.tokenizeInline(cell)
		}));
	}

	/**
	 * Tokenize a paragraph.
	 */
	private tokenizeParagraph(
		lines: string[],
		startIndex: number
	): { token: Token; linesConsumed: number } {
		const paragraphLines: string[] = [];
		let i = startIndex;

		while (i < lines.length) {
			const line = lines[i];
			const nextLine = lines[i + 1];

			// Stop at blank line
			if (PATTERNS.blank.test(line)) break;

			// Stop at block-level elements
			if (
				PATTERNS.heading.test(line) ||
				PATTERNS.hr.test(line) ||
				PATTERNS.codeBlockStart.test(line) ||
				PATTERNS.blockquote.test(line) ||
				PATTERNS.ulItem.test(line) ||
				PATTERNS.olItem.test(line) ||
				(this.options.math && PATTERNS.mathBlockStart.test(line))
			) {
				if (i === startIndex) {
					// This line is the block element, shouldn't happen but handle it
					break;
				}
				break;
			}

			// Check for setext heading
			if (nextLine && (/^={3,}\s*$/.test(nextLine) || /^-{3,}\s*$/.test(nextLine))) {
				if (paragraphLines.length === 0) {
					// Let the heading handler deal with it
					break;
				}
			}

			paragraphLines.push(line);
			i++;
		}

		if (paragraphLines.length === 0) {
			return {
				token: { type: 'paragraph', raw: '', tokens: [] },
				linesConsumed: 1
			};
		}

		const text = paragraphLines.join('\n');
		return {
			token: {
				type: 'paragraph',
				raw: text,
				text,
				tokens: this.tokenizeInline(text)
			},
			linesConsumed: paragraphLines.length
		};
	}

	/**
	 * Tokenize inline content.
	 */
	tokenizeInline(text: string): Token[] {
		const tokens: Token[] = [];
		let remaining = text;

		while (remaining.length > 0) {
			let matched = false;

			// Try custom inline extensions first
			for (const ext of this.inlineExtensions) {
				const match = remaining.match(ext.pattern);
				if (match && match.index === 0) {
					const token = ext.parse(match);
					if (token) {
						tokens.push(token);
						remaining = remaining.slice(match[0].length);
						matched = true;
						break;
					}
				}
			}

			if (matched) continue;

			// Escape
			const escapeMatch = remaining.match(INLINE_PATTERNS.escape);
			if (escapeMatch) {
				tokens.push({ type: 'text', raw: escapeMatch[0], text: escapeMatch[1] });
				remaining = remaining.slice(escapeMatch[0].length);
				continue;
			}

			// Code span
			const codeMatch = remaining.match(INLINE_PATTERNS.code);
			if (codeMatch) {
				tokens.push({ type: 'code', raw: codeMatch[0], text: codeMatch[2].trim() });
				remaining = remaining.slice(codeMatch[0].length);
				continue;
			}

			// Wiki-link (before regular link)
			if (this.options.wikiLinks) {
				const wikiMatch = remaining.match(INLINE_PATTERNS.wikiLink);
				if (wikiMatch) {
					tokens.push({
						type: 'wiki_link',
						raw: wikiMatch[0],
						target: wikiMatch[1],
						alias: wikiMatch[2] || wikiMatch[1]
					});
					remaining = remaining.slice(wikiMatch[0].length);
					continue;
				}
			}

			// Math inline
			if (this.options.math) {
				const mathMatch = remaining.match(INLINE_PATTERNS.mathInline);
				if (mathMatch) {
					tokens.push({ type: 'math_inline', raw: mathMatch[0], text: mathMatch[1] });
					remaining = remaining.slice(mathMatch[0].length);
					continue;
				}
			}

			// Image (before link to prevent conflict)
			const imageMatch = remaining.match(INLINE_PATTERNS.image);
			if (imageMatch) {
				tokens.push({
					type: 'image',
					raw: imageMatch[0],
					alt: imageMatch[1],
					href: imageMatch[2],
					title: imageMatch[3]
				});
				remaining = remaining.slice(imageMatch[0].length);
				continue;
			}

			// Link
			const linkMatch = remaining.match(INLINE_PATTERNS.link);
			if (linkMatch) {
				tokens.push({
					type: 'link',
					raw: linkMatch[0],
					text: linkMatch[1],
					href: linkMatch[2],
					title: linkMatch[3],
					tokens: this.tokenizeInline(linkMatch[1])
				});
				remaining = remaining.slice(linkMatch[0].length);
				continue;
			}

			// Bold
			const boldMatch = remaining.match(INLINE_PATTERNS.bold);
			if (boldMatch) {
				const text = boldMatch[1] || boldMatch[2];
				tokens.push({
					type: 'bold',
					raw: boldMatch[0],
					text,
					tokens: this.tokenizeInline(text)
				});
				remaining = remaining.slice(boldMatch[0].length);
				continue;
			}

			// Italic
			const italicMatch = remaining.match(INLINE_PATTERNS.italic);
			if (italicMatch) {
				const text = italicMatch[1] || italicMatch[2];
				tokens.push({
					type: 'italic',
					raw: italicMatch[0],
					text,
					tokens: this.tokenizeInline(text)
				});
				remaining = remaining.slice(italicMatch[0].length);
				continue;
			}

			// Strikethrough
			const strikeMatch = remaining.match(INLINE_PATTERNS.strikethrough);
			if (strikeMatch) {
				tokens.push({
					type: 'strikethrough',
					raw: strikeMatch[0],
					text: strikeMatch[1],
					tokens: this.tokenizeInline(strikeMatch[1])
				});
				remaining = remaining.slice(strikeMatch[0].length);
				continue;
			}

			// Line break
			const lineBreakMatch = remaining.match(INLINE_PATTERNS.lineBreak);
			if (lineBreakMatch) {
				tokens.push({ type: 'line_break', raw: lineBreakMatch[0] });
				remaining = remaining.slice(lineBreakMatch[0].length);
				continue;
			}

			// Soft break
			const softBreakMatch = remaining.match(INLINE_PATTERNS.softBreak);
			if (softBreakMatch) {
				tokens.push({ type: 'soft_break', raw: softBreakMatch[0] });
				remaining = remaining.slice(softBreakMatch[0].length);
				continue;
			}

			// Plain text (consume until next potential token)
			const textEnd = this.findNextTokenStart(remaining);
			const textContent = remaining.slice(0, textEnd);
			if (textContent) {
				tokens.push({ type: 'text', raw: textContent, text: textContent });
			}
			remaining = remaining.slice(textEnd);
		}

		return tokens;
	}

	/**
	 * Find the index of the next potential token start.
	 */
	private findNextTokenStart(text: string): number {
		const markers = ['\\', '`', '*', '_', '~', '[', '!', '$', '\n'];
		let minIndex = text.length;

		for (const marker of markers) {
			const idx = text.indexOf(marker);
			if (idx !== -1 && idx < minIndex && idx > 0) {
				minIndex = idx;
			}
		}

		return minIndex || 1;
	}
}

/**
 * Create a tokenizer with the given options.
 */
export function createTokenizer(options?: Partial<ParserOptions>): Tokenizer {
	return new Tokenizer(options);
}
