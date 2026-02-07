/**
 * Markdown parser.
 * Converts tokens into an AST (Abstract Syntax Tree).
 */

import type { Token, ASTNode, ParserOptions } from './types';
import { defaultParserOptions } from './types';
import { Tokenizer } from './tokenizer';

// ============================================================================
// Parser Class
// ============================================================================

export class Parser {
	private tokenizer: Tokenizer;
	private options: ParserOptions;

	constructor(options: Partial<ParserOptions> = {}) {
		this.options = { ...defaultParserOptions, ...options };
		this.tokenizer = new Tokenizer(this.options);
	}

	/**
	 * Parse markdown text into an AST.
	 */
	parse(markdown: string): ASTNode {
		const tokens = this.tokenizer.tokenize(markdown);
		return {
			type: 'document',
			children: this.tokensToNodes(tokens)
		};
	}

	/**
	 * Convert an array of tokens to AST nodes.
	 */
	private tokensToNodes(tokens: Token[]): ASTNode[] {
		return tokens.map((token) => this.tokenToNode(token)).filter((n): n is ASTNode => n !== null);
	}

	/**
	 * Convert a single token to an AST node.
	 */
	private tokenToNode(token: Token): ASTNode | null {
		switch (token.type) {
			case 'heading':
				return {
					type: 'heading',
					level: token.depth,
					children: this.tokensToNodes(token.tokens || [])
				};

			case 'paragraph':
				return {
					type: 'paragraph',
					children: this.tokensToNodes(token.tokens || [])
				};

			case 'code_block':
				return {
					type: 'code_block',
					lang: token.lang,
					value: token.text
				};

			case 'blockquote':
				return {
					type: 'blockquote',
					children: this.tokensToNodes(token.tokens || [])
				};

			case 'hr':
				return {
					type: 'horizontal_rule'
				};

			case 'list':
				return {
					type: 'list',
					ordered: token.ordered,
					start: token.start,
					children: this.tokensToNodes(token.tokens || [])
				};

			case 'list_item':
				return {
					type: 'list_item',
					children: this.tokensToNodes(token.tokens || [])
				};

			case 'task_item':
				return {
					type: 'task_item',
					checked: token.checked,
					children: this.tokensToNodes(token.tokens || [])
				};

			case 'table':
				return {
					type: 'table',
					align: token.align,
					children: this.tokensToNodes(token.tokens || [])
				};

			case 'table_row':
				return {
					type: 'table_row',
					header: token.header,
					children: token.cells?.[0]?.map((cell) => this.tokenToNode(cell)).filter(Boolean) as ASTNode[] || []
				};

			case 'table_cell':
				return {
					type: 'table_cell',
					children: this.tokensToNodes(token.tokens || [])
				};

			case 'text':
				return {
					type: 'text',
					value: token.text
				};

			case 'bold':
				return {
					type: 'bold',
					children: this.tokensToNodes(token.tokens || [])
				};

			case 'italic':
				return {
					type: 'italic',
					children: this.tokensToNodes(token.tokens || [])
				};

			case 'code':
				return {
					type: 'code',
					value: token.text
				};

			case 'strikethrough':
				return {
					type: 'strikethrough',
					children: this.tokensToNodes(token.tokens || [])
				};

			case 'link':
				return {
					type: 'link',
					url: token.href,
					title: token.title,
					children: this.tokensToNodes(token.tokens || [])
				};

			case 'image':
				return {
					type: 'image',
					url: token.href,
					alt: token.alt,
					title: token.title
				};

			case 'wiki_link':
				return {
					type: 'wiki_link',
					target: token.target,
					alias: token.alias
				};

			case 'math_inline':
				return {
					type: 'math_inline',
					value: token.text
				};

			case 'math_block':
				return {
					type: 'math_block',
					value: token.text
				};

			case 'line_break':
				return {
					type: 'line_break'
				};

			case 'soft_break':
				return {
					type: 'soft_break'
				};

			case 'html_block':
				return {
					type: 'html',
					value: token.text
				};

			default:
				return null;
		}
	}
}

/**
 * Create a parser with the given options.
 */
export function createParser(options?: Partial<ParserOptions>): Parser {
	return new Parser(options);
}

/**
 * Parse markdown to AST in one call.
 */
export function parseMarkdown(markdown: string, options?: Partial<ParserOptions>): ASTNode {
	return new Parser(options).parse(markdown);
}
