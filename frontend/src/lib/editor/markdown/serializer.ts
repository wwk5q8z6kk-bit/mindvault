/**
 * Markdown serializer.
 * Converts an AST back to markdown text.
 */

import type { ASTNode, SerializerOptions } from './types';
import { defaultSerializerOptions } from './types';

// ============================================================================
// Serializer Class
// ============================================================================

export class Serializer {
	private options: SerializerOptions;

	constructor(options: Partial<SerializerOptions> = {}) {
		this.options = { ...defaultSerializerOptions, ...options };
	}

	/**
	 * Serialize an AST to markdown.
	 */
	serialize(ast: ASTNode): string {
		return this.serializeNode(ast).trim();
	}

	/**
	 * Serialize a single node.
	 */
	private serializeNode(node: ASTNode, context: SerializerContext = {}): string {
		switch (node.type) {
			case 'document':
				return this.serializeChildren(node, context, '\n\n');

			case 'heading':
				return this.serializeHeading(node, context);

			case 'paragraph':
				return this.serializeChildren(node, context);

			case 'code_block':
				return this.serializeCodeBlock(node);

			case 'blockquote':
				return this.serializeBlockquote(node, context);

			case 'horizontal_rule':
				return this.options.horizontalRule || '---';

			case 'list':
				return this.serializeList(node, context);

			case 'list_item':
				return this.serializeChildren(node, context);

			case 'task_item':
				return this.serializeTaskItem(node, context);

			case 'table':
				return this.serializeTable(node);

			case 'table_row':
				return this.serializeTableRow(node);

			case 'table_cell':
				return this.serializeChildren(node, context);

			case 'text':
				return this.escapeText(node.value || '');

			case 'bold':
				return `${this.options.strong}${this.serializeChildren(node, context)}${this.options.strong}`;

			case 'italic':
				return `${this.options.emphasis}${this.serializeChildren(node, context)}${this.options.emphasis}`;

			case 'code':
				return this.serializeInlineCode(node);

			case 'strikethrough':
				return `~~${this.serializeChildren(node, context)}~~`;

			case 'link':
				return this.serializeLink(node, context);

			case 'image':
				return this.serializeImage(node);

			case 'wiki_link':
				return this.serializeWikiLink(node);

			case 'math_inline':
				return `$${node.value || ''}$`;

			case 'math_block':
				return `$$\n${node.value || ''}\n$$`;

			case 'line_break':
				return '  \n';

			case 'soft_break':
				return this.options.softBreak || '\n';

			case 'html':
				return node.value || '';

			default:
				return '';
		}
	}

	/**
	 * Serialize all children of a node.
	 */
	private serializeChildren(
		node: ASTNode,
		context: SerializerContext,
		separator = ''
	): string {
		if (!node.children) return '';
		return node.children.map((child) => this.serializeNode(child, context)).join(separator);
	}

	/**
	 * Serialize a heading.
	 */
	private serializeHeading(node: ASTNode, context: SerializerContext): string {
		const level = node.level || 1;
		const prefix = '#'.repeat(level);
		const content = this.serializeChildren(node, context);
		return `${prefix} ${content}`;
	}

	/**
	 * Serialize a code block.
	 */
	private serializeCodeBlock(node: ASTNode): string {
		const fence = this.options.fence || '```';
		const lang = node.lang || '';
		const code = node.value || '';
		return `${fence}${lang}\n${code}\n${fence}`;
	}

	/**
	 * Serialize a blockquote.
	 */
	private serializeBlockquote(node: ASTNode, context: SerializerContext): string {
		const content = this.serializeChildren(node, context, '\n\n');
		return content
			.split('\n')
			.map((line) => `> ${line}`)
			.join('\n');
	}

	/**
	 * Serialize a list.
	 */
	private serializeList(node: ASTNode, context: SerializerContext): string {
		const ordered = node.ordered;
		const start = node.start || 1;
		const bullet = this.options.bullet || '-';

		return (node.children || [])
			.map((child, index) => {
				const content = this.serializeNode(child, { ...context, inList: true });
				if (child.type === 'task_item') {
					const checkbox = child.checked ? '[x]' : '[ ]';
					return `${bullet} ${checkbox} ${content}`;
				}
				if (ordered) {
					return `${start + index}. ${content}`;
				}
				return `${bullet} ${content}`;
			})
			.join('\n');
	}

	/**
	 * Serialize a task item.
	 */
	private serializeTaskItem(node: ASTNode, context: SerializerContext): string {
		return this.serializeChildren(node, context);
	}

	/**
	 * Serialize a table.
	 */
	private serializeTable(node: ASTNode): string {
		if (!node.children || node.children.length === 0) return '';

		const alignments = node.align || [];
		const rows: string[] = [];

		// Header row
		const headerRow = node.children.find((r) => r.header);
		if (headerRow) {
			rows.push(this.serializeTableRow(headerRow));

			// Separator row
			const separators = alignments.map((align) => {
				switch (align) {
					case 'left':
						return ':---';
					case 'center':
						return ':---:';
					case 'right':
						return '---:';
					default:
						return '---';
				}
			});
			rows.push(`| ${separators.join(' | ')} |`);
		}

		// Body rows
		const bodyRows = node.children.filter((r) => !r.header);
		for (const row of bodyRows) {
			rows.push(this.serializeTableRow(row));
		}

		return rows.join('\n');
	}

	/**
	 * Serialize a table row.
	 */
	private serializeTableRow(node: ASTNode): string {
		const cells = (node.children || []).map((cell) =>
			this.serializeNode(cell, {}).replace(/\|/g, '\\|')
		);
		return `| ${cells.join(' | ')} |`;
	}

	/**
	 * Serialize inline code.
	 */
	private serializeInlineCode(node: ASTNode): string {
		const code = node.value || '';
		// Use double backticks if code contains single backtick
		if (code.includes('`')) {
			return `\`\` ${code} \`\``;
		}
		return `\`${code}\``;
	}

	/**
	 * Serialize a link.
	 */
	private serializeLink(node: ASTNode, context: SerializerContext): string {
		const text = this.serializeChildren(node, context);
		const url = node.url || '';
		const title = node.title;

		if (title) {
			return `[${text}](${url} "${title}")`;
		}
		return `[${text}](${url})`;
	}

	/**
	 * Serialize an image.
	 */
	private serializeImage(node: ASTNode): string {
		const alt = node.alt || '';
		const url = node.url || '';
		const title = node.title;

		if (title) {
			return `![${alt}](${url} "${title}")`;
		}
		return `![${alt}](${url})`;
	}

	/**
	 * Serialize a wiki-link.
	 */
	private serializeWikiLink(node: ASTNode): string {
		const target = node.target || '';
		const alias = node.alias;

		if (alias && alias !== target) {
			return `[[${target}|${alias}]]`;
		}
		return `[[${target}]]`;
	}

	/**
	 * Escape special characters in text.
	 */
	private escapeText(text: string): string {
		// Only escape characters that would be interpreted as markdown
		return text
			.replace(/\\/g, '\\\\')
			.replace(/([*_`~\[\]<>])/g, '\\$1');
	}
}

interface SerializerContext {
	inList?: boolean;
	listDepth?: number;
}

/**
 * Create a serializer with the given options.
 */
export function createSerializer(options?: Partial<SerializerOptions>): Serializer {
	return new Serializer(options);
}

/**
 * Serialize an AST to markdown in one call.
 */
export function serializeMarkdown(ast: ASTNode, options?: Partial<SerializerOptions>): string {
	return new Serializer(options).serialize(ast);
}
