/**
 * Math/LaTeX plugin using KaTeX.
 *
 * Features:
 * - Inline math: $e = mc^2$
 * - Block math: $$\int_0^\infty e^{-x^2} dx$$
 * - Slash command /math for block equations
 * - Live preview in modal
 */

import type { EditorPlugin, PluginContext, SlashItem } from '../types';

// KaTeX will be loaded dynamically
let katex: typeof import('katex') | null = null;
let katexLoaded = false;
let loadPromise: Promise<void> | null = null;

/**
 * Load KaTeX library.
 */
async function loadKatex(): Promise<void> {
	if (katexLoaded) return;
	if (loadPromise) return loadPromise;

	loadPromise = (async () => {
		try {
			katex = await import('katex');
			katexLoaded = true;
		} catch (error) {
			console.warn('Failed to load KaTeX:', error);
			loadPromise = null;
		}
	})();

	return loadPromise;
}

/**
 * Render LaTeX to HTML.
 */
function renderLatex(latex: string, displayMode: boolean): string {
	if (!katex || !katexLoaded) {
		return `<span class="mv-math-error">KaTeX not loaded</span>`;
	}

	try {
		return katex.renderToString(latex, {
			displayMode,
			throwOnError: false,
			errorColor: '#f87171',
			strict: false,
			trust: true
		});
	} catch (error) {
		return `<span class="mv-math-error">${escapeHtml(String(error))}</span>`;
	}
}

/**
 * Escape HTML characters.
 */
function escapeHtml(s: string): string {
	return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

/**
 * Find and render all math expressions in an element.
 */
function renderAllMath(container: HTMLElement): void {
	if (!katexLoaded) return;

	// Find inline math: $...$
	const inlineMathRegex = /\$([^$]+)\$/g;

	// Find block math: $$...$$
	const blockMathRegex = /\$\$([^$]+)\$\$/g;

	// Process text nodes
	const walker = document.createTreeWalker(
		container,
		NodeFilter.SHOW_TEXT,
		{
			acceptNode: (node) => {
				// Skip if inside a pre, code, or already rendered math
				const parent = node.parentElement;
				if (!parent) return NodeFilter.FILTER_REJECT;
				if (parent.closest('pre, code, .mv-math-rendered')) {
					return NodeFilter.FILTER_REJECT;
				}
				// Check if contains math
				const text = node.textContent || '';
				if (text.includes('$')) {
					return NodeFilter.FILTER_ACCEPT;
				}
				return NodeFilter.FILTER_REJECT;
			}
		}
	);

	const nodesToProcess: Text[] = [];
	let currentNode: Node | null;
	while ((currentNode = walker.nextNode())) {
		nodesToProcess.push(currentNode as Text);
	}

	// Process each text node
	for (const textNode of nodesToProcess) {
		const text = textNode.textContent || '';
		let html = escapeHtml(text);
		let hasMatch = false;

		// Replace block math first (so we don't match $$ as two inline $)
		html = html.replace(blockMathRegex, (match, latex) => {
			hasMatch = true;
			const rendered = renderLatex(latex.trim(), true);
			return `<div class="mv-math-rendered mv-math-block" data-latex="${escapeHtml(latex)}">${rendered}</div>`;
		});

		// Replace inline math
		html = html.replace(inlineMathRegex, (match, latex) => {
			hasMatch = true;
			const rendered = renderLatex(latex.trim(), false);
			return `<span class="mv-math-rendered mv-math-inline" data-latex="${escapeHtml(latex)}">${rendered}</span>`;
		});

		// Replace the text node with rendered HTML
		if (hasMatch) {
			const wrapper = document.createElement('span');
			wrapper.innerHTML = html;
			textNode.replaceWith(...wrapper.childNodes);
		}
	}
}

/**
 * Insert a math block at the current cursor.
 */
function insertMathBlock(ctx: PluginContext, latex = ''): void {
	const sel = window.getSelection();
	if (!sel || sel.rangeCount === 0) return;

	const range = sel.getRangeAt(0);
	const mathEl = document.createElement('div');
	mathEl.className = 'mv-math-block mv-math-editable';
	mathEl.contentEditable = 'false';
	mathEl.dataset.latex = latex || '\\int_0^\\infty e^{-x^2} dx = \\frac{\\sqrt{\\pi}}{2}';

	// Render the math
	if (katexLoaded) {
		mathEl.innerHTML = renderLatex(mathEl.dataset.latex, true);
	} else {
		mathEl.textContent = `$$${mathEl.dataset.latex}$$`;
	}

	// Make it clickable to edit
	mathEl.addEventListener('click', () => {
		ctx.emit('editMath', { element: mathEl, latex: mathEl.dataset.latex });
	});

	range.deleteContents();
	range.insertNode(mathEl);

	// Insert a paragraph after for continued typing
	const p = document.createElement('p');
	p.innerHTML = '<br>';
	mathEl.after(p);

	// Place cursor in the new paragraph
	const newRange = document.createRange();
	newRange.setStart(p, 0);
	newRange.collapse(true);
	sel.removeAllRanges();
	sel.addRange(newRange);

	ctx.triggerChange();
}

/**
 * Insert inline math at the current cursor.
 */
function insertInlineMath(ctx: PluginContext, latex = ''): void {
	const sel = window.getSelection();
	if (!sel || sel.rangeCount === 0) return;

	const range = sel.getRangeAt(0);
	const selectedText = sel.toString();

	// Use selected text as LaTeX if available
	const mathLatex = selectedText || latex || 'x^2';
	const mathText = `$${mathLatex}$`;

	range.deleteContents();
	range.insertNode(document.createTextNode(mathText));

	// Move cursor after the math
	range.collapse(false);
	sel.removeAllRanges();
	sel.addRange(range);

	ctx.triggerChange();

	// Render math after a short delay
	if (katexLoaded) {
		setTimeout(() => {
			renderAllMath(ctx.editorElement);
		}, 50);
	}
}

/**
 * Math plugin.
 */
export const mathPlugin: EditorPlugin = {
	id: 'math',
	name: 'Math/LaTeX',
	description: 'LaTeX math rendering using KaTeX',
	version: '1.0.0',

	async onInit(ctx: PluginContext) {
		await loadKatex();

		// Initial rendering
		if (katexLoaded) {
			renderAllMath(ctx.editorElement);
		}
	},

	onDestroy() {
		// Cleanup if needed
	},

	onDocumentChange(ctx: PluginContext) {
		// Re-render math on document change (debounced in practice by the editor)
		if (katexLoaded) {
			// Only process new/unrendered math
			const unrendered = ctx.editorElement.querySelectorAll(':not(.mv-math-rendered)');
			// This is a simplified approach - full implementation would be smarter
		}
	},

	slashItems: [
		{
			id: 'math-block',
			label: 'Math Block',
			description: 'Display equation',
			icon: '∑',
			keywords: ['math', 'equation', 'latex', 'katex', 'formula'],
			group: 'blocks',
			action: (ctx) => {
				insertMathBlock(ctx);
			}
		},
		{
			id: 'math-inline',
			label: 'Inline Math',
			description: 'Inline equation',
			icon: 'x²',
			keywords: ['math', 'inline', 'latex', 'formula'],
			group: 'formatting',
			action: (ctx) => {
				insertInlineMath(ctx);
			}
		}
	],

	commands: {
		insertMathBlock: (ctx: PluginContext, ...args: unknown[]) => {
			const [latex] = args as [string | undefined];
			insertMathBlock(ctx, latex);
		},
		insertInlineMath: (ctx: PluginContext, ...args: unknown[]) => {
			const [latex] = args as [string | undefined];
			insertInlineMath(ctx, latex);
		},
		renderMath: (ctx: PluginContext) => {
			if (katexLoaded) {
				renderAllMath(ctx.editorElement);
			}
		}
	},

	inputRules: [
		// Block math: $$...$$ (on Enter after $$...$$)
		{
			id: 'block-math',
			pattern: /^\$\$([^$]+)\$\$$/,
			handler: (ctx, match) => {
				// Would convert to rendered math block
				return false;
			}
		}
	],

	markdownExtension: {
		name: 'math',
		parse: (text: string) => {
			// Parse $...$ and $$...$$ in markdown
			return null;
		},
		serialize: (node) => {
			// Serialize math nodes back to markdown
			return null;
		}
	}
};

/**
 * Helper to check if KaTeX is loaded.
 */
export function isKatexLoaded(): boolean {
	return katexLoaded;
}

/**
 * Manually load KaTeX.
 */
export async function ensureKatexLoaded(): Promise<boolean> {
	await loadKatex();
	return katexLoaded;
}

/**
 * Render a single LaTeX expression.
 */
export function renderMath(latex: string, displayMode = false): string {
	return renderLatex(latex, displayMode);
}

export default mathPlugin;
