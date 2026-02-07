/**
 * HTML converter.
 * Converts between AST and HTML.
 */

import type { ASTNode, HTMLConverterOptions, NodeType } from './types';
import { defaultHTMLConverterOptions } from './types';

// ============================================================================
// AST to HTML Converter
// ============================================================================

export class HTMLRenderer {
	private options: HTMLConverterOptions;
	private renderers: Partial<Record<NodeType, (node: ASTNode) => string>>;

	constructor(options: Partial<HTMLConverterOptions> = {}) {
		this.options = { ...defaultHTMLConverterOptions, ...options };
		this.renderers = { ...defaultRenderers, ...this.options.renderers };
	}

	/**
	 * Render an AST to HTML.
	 */
	render(ast: ASTNode): string {
		return this.renderNode(ast);
	}

	/**
	 * Render a single node to HTML.
	 */
	private renderNode(node: ASTNode): string {
		const renderer = this.renderers[node.type];
		if (renderer) {
			return renderer.call(this, node);
		}

		// Default: render children
		return this.renderChildren(node);
	}

	/**
	 * Render all children of a node.
	 */
	renderChildren(node: ASTNode): string {
		if (!node.children) return '';
		return node.children.map((child) => this.renderNode(child)).join('');
	}

	/**
	 * Escape HTML characters.
	 */
	escape(text: string): string {
		if (!this.options.sanitize) return text;
		return text
			.replace(/&/g, '&amp;')
			.replace(/</g, '&lt;')
			.replace(/>/g, '&gt;')
			.replace(/"/g, '&quot;')
			.replace(/'/g, '&#39;');
	}

	/**
	 * Create a class name with optional prefix.
	 */
	className(name: string): string {
		return `${this.options.classPrefix || ''}${name}`;
	}
}

// ============================================================================
// Default Renderers
// ============================================================================

const defaultRenderers: Partial<Record<NodeType, (this: HTMLRenderer, node: ASTNode) => string>> = {
	document(node) {
		return this.renderChildren(node);
	},

	heading(node) {
		const level = node.level || 1;
		const content = this.renderChildren(node);
		return `<h${level}>${content}</h${level}>\n`;
	},

	paragraph(node) {
		const content = this.renderChildren(node);
		return `<p>${content}</p>\n`;
	},

	code_block(node) {
		const code = this.escape(node.value || '');
		const lang = node.lang ? ` class="language-${this.escape(node.lang)}"` : '';
		return `<pre><code${lang}>${code}</code></pre>\n`;
	},

	blockquote(node) {
		const content = this.renderChildren(node);
		return `<blockquote>\n${content}</blockquote>\n`;
	},

	horizontal_rule() {
		return '<hr>\n';
	},

	list(node) {
		const tag = node.ordered ? 'ol' : 'ul';
		const start = node.ordered && node.start !== 1 ? ` start="${node.start}"` : '';
		const content = this.renderChildren(node);
		return `<${tag}${start}>\n${content}</${tag}>\n`;
	},

	list_item(node) {
		const content = this.renderChildren(node);
		return `<li>${content}</li>\n`;
	},

	task_item(node) {
		const checked = node.checked ? ' checked' : '';
		const content = this.renderChildren(node);
		return `<li class="${this.className('task-item')}"><input type="checkbox"${checked} disabled> ${content}</li>\n`;
	},

	table(node) {
		const content = this.renderChildren(node);
		return `<table>\n${content}</table>\n`;
	},

	table_row(node) {
		const tag = node.header ? 'thead' : 'tbody';
		const cellTag = node.header ? 'th' : 'td';
		const cells = (node.children || [])
			.map((cell) => {
				const content = this.renderChildren(cell);
				return `<${cellTag}>${content}</${cellTag}>`;
			})
			.join('');

		if (node.header) {
			return `<thead><tr>${cells}</tr></thead>\n`;
		}
		return `<tr>${cells}</tr>\n`;
	},

	table_cell(node) {
		const content = this.renderChildren(node);
		return content;
	},

	text(node) {
		return this.escape(node.value || '');
	},

	bold(node) {
		const content = this.renderChildren(node);
		return `<strong>${content}</strong>`;
	},

	italic(node) {
		const content = this.renderChildren(node);
		return `<em>${content}</em>`;
	},

	code(node) {
		const code = this.escape(node.value || '');
		return `<code>${code}</code>`;
	},

	strikethrough(node) {
		const content = this.renderChildren(node);
		return `<del>${content}</del>`;
	},

	link(node) {
		const href = this.escape(node.url || '');
		const title = node.title ? ` title="${this.escape(node.title)}"` : '';
		const content = this.renderChildren(node);
		return `<a href="${href}"${title}>${content}</a>`;
	},

	image(node) {
		const src = this.escape(node.url || '');
		const alt = this.escape(node.alt || '');
		const title = node.title ? ` title="${this.escape(node.title)}"` : '';
		return `<img src="${src}" alt="${alt}"${title}>`;
	},

	wiki_link(node) {
		const target = this.escape(node.target || '');
		const alias = this.escape(node.alias || node.target || '');
		return `<a href="#" class="${this.className('wiki-link')}" data-target="${target}">${alias}</a>`;
	},

	math_inline(node) {
		const math = this.escape(node.value || '');
		return `<span class="${this.className('math-inline')}" data-latex="${math}">$${math}$</span>`;
	},

	math_block(node) {
		const math = this.escape(node.value || '');
		return `<div class="${this.className('math-block')}" data-latex="${math}">$$${math}$$</div>\n`;
	},

	line_break() {
		return '<br>\n';
	},

	soft_break() {
		return '\n';
	},

	html(node) {
		// Return raw HTML (potentially unsafe)
		return node.value || '';
	}
};

// ============================================================================
// HTML to AST Converter
// ============================================================================

export class HTMLParser {
	private options: HTMLConverterOptions;

	constructor(options: Partial<HTMLConverterOptions> = {}) {
		this.options = { ...defaultHTMLConverterOptions, ...options };
	}

	/**
	 * Parse HTML into an AST.
	 */
	parse(html: string): ASTNode {
		const container = document.createElement('div');
		container.innerHTML = html;
		return {
			type: 'document',
			children: this.parseChildren(container)
		};
	}

	/**
	 * Parse children of an element.
	 */
	private parseChildren(element: Element): ASTNode[] {
		const nodes: ASTNode[] = [];

		for (const child of Array.from(element.childNodes)) {
			const node = this.parseNode(child);
			if (node) {
				nodes.push(node);
			}
		}

		return nodes;
	}

	/**
	 * Parse a single DOM node.
	 */
	private parseNode(domNode: Node): ASTNode | null {
		if (domNode.nodeType === Node.TEXT_NODE) {
			const text = domNode.textContent || '';
			if (!text.trim()) return null;
			return { type: 'text', value: text };
		}

		if (domNode.nodeType !== Node.ELEMENT_NODE) {
			return null;
		}

		const element = domNode as Element;
		const tagName = element.tagName.toLowerCase();

		switch (tagName) {
			case 'h1':
			case 'h2':
			case 'h3':
			case 'h4':
			case 'h5':
			case 'h6':
				return {
					type: 'heading',
					level: parseInt(tagName[1], 10),
					children: this.parseChildren(element)
				};

			case 'p':
				return {
					type: 'paragraph',
					children: this.parseChildren(element)
				};

			case 'pre': {
				const code = element.querySelector('code');
				const text = code ? code.textContent : element.textContent;
				const lang = code?.className.match(/language-(\w+)/)?.[1];
				return {
					type: 'code_block',
					lang,
					value: text || ''
				};
			}

			case 'blockquote':
				return {
					type: 'blockquote',
					children: this.parseChildren(element)
				};

			case 'hr':
				return { type: 'horizontal_rule' };

			case 'ul':
			case 'ol':
				return {
					type: 'list',
					ordered: tagName === 'ol',
					start: tagName === 'ol' ? parseInt(element.getAttribute('start') || '1', 10) : undefined,
					children: this.parseChildren(element)
				};

			case 'li': {
				const checkbox = element.querySelector('input[type="checkbox"]');
				if (checkbox) {
					return {
						type: 'task_item',
						checked: (checkbox as HTMLInputElement).checked,
						children: this.parseInlineChildren(element)
					};
				}
				return {
					type: 'list_item',
					children: this.parseChildren(element)
				};
			}

			case 'table':
				return {
					type: 'table',
					children: this.parseTableChildren(element)
				};

			case 'strong':
			case 'b':
				return {
					type: 'bold',
					children: this.parseChildren(element)
				};

			case 'em':
			case 'i':
				return {
					type: 'italic',
					children: this.parseChildren(element)
				};

			case 'code':
				return {
					type: 'code',
					value: element.textContent || ''
				};

			case 'del':
			case 's':
				return {
					type: 'strikethrough',
					children: this.parseChildren(element)
				};

			case 'a': {
				const isWikiLink = element.classList.contains(`${this.options.classPrefix}wiki-link`);
				if (isWikiLink) {
					return {
						type: 'wiki_link',
						target: element.getAttribute('data-target') || '',
						alias: element.textContent || ''
					};
				}
				return {
					type: 'link',
					url: element.getAttribute('href') || '',
					title: element.getAttribute('title') || undefined,
					children: this.parseChildren(element)
				};
			}

			case 'img':
				return {
					type: 'image',
					url: element.getAttribute('src') || '',
					alt: element.getAttribute('alt') || '',
					title: element.getAttribute('title') || undefined
				};

			case 'br':
				return { type: 'line_break' };

			case 'span': {
				// Check for math
				if (element.classList.contains(`${this.options.classPrefix}math-inline`)) {
					return {
						type: 'math_inline',
						value: element.getAttribute('data-latex') || element.textContent?.replace(/^\$|\$$/g, '') || ''
					};
				}
				// Otherwise, parse as inline content
				return {
					type: 'text',
					value: element.textContent || ''
				};
			}

			case 'div': {
				// Check for math block
				if (element.classList.contains(`${this.options.classPrefix}math-block`)) {
					return {
						type: 'math_block',
						value: element.getAttribute('data-latex') || element.textContent?.replace(/^\$\$|\$\$$/g, '') || ''
					};
				}
				// Otherwise, parse as block content
				return {
					type: 'paragraph',
					children: this.parseChildren(element)
				};
			}

			default:
				// Unknown element - try to parse children
				return {
					type: 'paragraph',
					children: this.parseChildren(element)
				};
		}
	}

	/**
	 * Parse inline children (skip block elements in list items).
	 */
	private parseInlineChildren(element: Element): ASTNode[] {
		const nodes: ASTNode[] = [];

		for (const child of Array.from(element.childNodes)) {
			if (child.nodeType === Node.TEXT_NODE) {
				const text = child.textContent || '';
				if (text.trim()) {
					nodes.push({ type: 'text', value: text });
				}
			} else if (child.nodeType === Node.ELEMENT_NODE) {
				const el = child as Element;
				const tagName = el.tagName.toLowerCase();

				// Skip checkbox
				if (tagName === 'input') continue;

				const node = this.parseNode(child);
				if (node) {
					nodes.push(node);
				}
			}
		}

		return nodes;
	}

	/**
	 * Parse table children (thead, tbody, tr).
	 */
	private parseTableChildren(table: Element): ASTNode[] {
		const rows: ASTNode[] = [];

		// Parse thead
		const thead = table.querySelector('thead');
		if (thead) {
			const tr = thead.querySelector('tr');
			if (tr) {
				rows.push({
					type: 'table_row',
					header: true,
					children: this.parseTableCells(tr, true)
				});
			}
		}

		// Parse tbody
		const tbody = table.querySelector('tbody');
		const rowElements = tbody ? tbody.querySelectorAll('tr') : table.querySelectorAll('tr');
		for (const tr of Array.from(rowElements)) {
			if (thead && tr.closest('thead')) continue;
			rows.push({
				type: 'table_row',
				header: false,
				children: this.parseTableCells(tr, false)
			});
		}

		return rows;
	}

	/**
	 * Parse table cells (th/td).
	 */
	private parseTableCells(tr: Element, isHeader: boolean): ASTNode[] {
		const cells: ASTNode[] = [];
		const cellElements = tr.querySelectorAll(isHeader ? 'th' : 'td');

		for (const cell of Array.from(cellElements)) {
			cells.push({
				type: 'table_cell',
				children: this.parseChildren(cell)
			});
		}

		return cells;
	}
}

// ============================================================================
// Factory Functions
// ============================================================================

/**
 * Create an HTML renderer.
 */
export function createHTMLRenderer(options?: Partial<HTMLConverterOptions>): HTMLRenderer {
	return new HTMLRenderer(options);
}

/**
 * Create an HTML parser.
 */
export function createHTMLParser(options?: Partial<HTMLConverterOptions>): HTMLParser {
	return new HTMLParser(options);
}

/**
 * Render an AST to HTML.
 */
export function renderHTML(ast: ASTNode, options?: Partial<HTMLConverterOptions>): string {
	return new HTMLRenderer(options).render(ast);
}

/**
 * Parse HTML to an AST.
 */
export function parseHTML(html: string, options?: Partial<HTMLConverterOptions>): ASTNode {
	return new HTMLParser(options).parse(html);
}
