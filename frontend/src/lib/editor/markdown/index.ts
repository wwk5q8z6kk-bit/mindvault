/**
 * Markdown parser/serializer module.
 *
 * Provides robust markdown parsing with:
 * - Proper tokenization (not fragile regex)
 * - AST representation
 * - Round-trip safe conversions
 * - Extension points for custom syntax
 */

// Types
export type {
	Token,
	TokenType,
	ASTNode,
	NodeType,
	ParserOptions,
	SerializerOptions,
	HTMLConverterOptions,
	BlockExtension,
	InlineExtension
} from './types';

export {
	defaultParserOptions,
	defaultSerializerOptions,
	defaultHTMLConverterOptions
} from './types';

// Tokenizer
export { Tokenizer, createTokenizer } from './tokenizer';

// Parser
export { Parser, createParser, parseMarkdown } from './parser';

// Serializer
export { Serializer, createSerializer, serializeMarkdown } from './serializer';

// HTML Converter
export {
	HTMLRenderer,
	HTMLParser,
	createHTMLRenderer,
	createHTMLParser,
	renderHTML,
	parseHTML
} from './html-converter';

// ============================================================================
// Convenience Functions
// ============================================================================

import { parseMarkdown } from './parser';
import { serializeMarkdown } from './serializer';
import { renderHTML, parseHTML } from './html-converter';
import type { ParserOptions, SerializerOptions, HTMLConverterOptions, ASTNode } from './types';

/**
 * Convert markdown to HTML.
 */
export function markdownToHTML(
	markdown: string,
	parserOptions?: Partial<ParserOptions>,
	htmlOptions?: Partial<HTMLConverterOptions>
): string {
	const ast = parseMarkdown(markdown, parserOptions);
	return renderHTML(ast, htmlOptions);
}

/**
 * Convert HTML to markdown.
 */
export function htmlToMarkdown(
	html: string,
	htmlOptions?: Partial<HTMLConverterOptions>,
	serializerOptions?: Partial<SerializerOptions>
): string {
	const ast = parseHTML(html, htmlOptions);
	return serializeMarkdown(ast, serializerOptions);
}

/**
 * Round-trip markdown (parse and re-serialize).
 * Useful for normalizing formatting.
 */
export function normalizeMarkdown(
	markdown: string,
	parserOptions?: Partial<ParserOptions>,
	serializerOptions?: Partial<SerializerOptions>
): string {
	const ast = parseMarkdown(markdown, parserOptions);
	return serializeMarkdown(ast, serializerOptions);
}

/**
 * Transform an AST with a visitor function.
 */
export function transformAST(
	ast: ASTNode,
	visitor: (node: ASTNode, parent?: ASTNode) => ASTNode | null
): ASTNode {
	function transform(node: ASTNode, parent?: ASTNode): ASTNode | null {
		const result = visitor(node, parent);
		if (result === null) return null;

		if (result.children) {
			result.children = result.children
				.map((child) => transform(child, result))
				.filter((n): n is ASTNode => n !== null);
		}

		return result;
	}

	return transform(ast) || { type: 'document', children: [] };
}

/**
 * Walk an AST and call a visitor for each node.
 */
export function walkAST(
	ast: ASTNode,
	visitor: (node: ASTNode, parent?: ASTNode, index?: number) => void
): void {
	function walk(node: ASTNode, parent?: ASTNode, index?: number): void {
		visitor(node, parent, index);

		if (node.children) {
			node.children.forEach((child, i) => walk(child, node, i));
		}
	}

	walk(ast);
}

/**
 * Find all nodes matching a predicate.
 */
export function findNodes(
	ast: ASTNode,
	predicate: (node: ASTNode) => boolean
): ASTNode[] {
	const results: ASTNode[] = [];

	walkAST(ast, (node) => {
		if (predicate(node)) {
			results.push(node);
		}
	});

	return results;
}

/**
 * Extract plain text from an AST.
 */
export function extractText(ast: ASTNode): string {
	const parts: string[] = [];

	walkAST(ast, (node) => {
		if (node.type === 'text' && node.value) {
			parts.push(node.value);
		} else if (node.type === 'code' && node.value) {
			parts.push(node.value);
		} else if (node.type === 'code_block' && node.value) {
			parts.push(node.value);
		} else if (node.type === 'soft_break' || node.type === 'line_break') {
			parts.push(' ');
		}
	});

	return parts.join('').replace(/\s+/g, ' ').trim();
}
