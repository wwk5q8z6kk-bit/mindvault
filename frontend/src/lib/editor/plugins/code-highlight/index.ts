/**
 * Code syntax highlighting plugin using highlight.js.
 *
 * Features:
 * - Detects code block language from ```lang annotation
 * - Applies highlighting on blur/language change
 * - Lazy-loads language grammars
 * - Dark theme styles
 */

import type { EditorPlugin, PluginContext, SlashItem } from '../types';

// Highlight.js will be loaded dynamically
let hljs: typeof import('highlight.js').default | null = null;
let hljsLoaded = false;
let loadPromise: Promise<void> | null = null;

// Languages to preload (most common)
const PRELOAD_LANGUAGES = ['javascript', 'typescript', 'python', 'css', 'html', 'json', 'bash', 'sql'];

// Language aliases
const LANGUAGE_ALIASES: Record<string, string> = {
	js: 'javascript',
	ts: 'typescript',
	py: 'python',
	rb: 'ruby',
	sh: 'bash',
	shell: 'bash',
	zsh: 'bash',
	yml: 'yaml',
	md: 'markdown'
};

/**
 * Load highlight.js core and common languages.
 */
async function loadHighlightJs(): Promise<void> {
	if (hljsLoaded) return;
	if (loadPromise) return loadPromise;

	loadPromise = (async () => {
		try {
			// Dynamic import of highlight.js
			const hljsModule = await import('highlight.js/lib/core');
			hljs = hljsModule.default;

			// Register common languages
			const [
				javascript,
				typescript,
				python,
				css,
				xml,
				json,
				bash,
				sql
			] = await Promise.all([
				import('highlight.js/lib/languages/javascript'),
				import('highlight.js/lib/languages/typescript'),
				import('highlight.js/lib/languages/python'),
				import('highlight.js/lib/languages/css'),
				import('highlight.js/lib/languages/xml'),
				import('highlight.js/lib/languages/json'),
				import('highlight.js/lib/languages/bash'),
				import('highlight.js/lib/languages/sql')
			]);

			hljs.registerLanguage('javascript', javascript.default);
			hljs.registerLanguage('typescript', typescript.default);
			hljs.registerLanguage('python', python.default);
			hljs.registerLanguage('css', css.default);
			hljs.registerLanguage('xml', xml.default);
			hljs.registerLanguage('html', xml.default);
			hljs.registerLanguage('json', json.default);
			hljs.registerLanguage('bash', bash.default);
			hljs.registerLanguage('sql', sql.default);

			hljsLoaded = true;
		} catch (error) {
			console.warn('Failed to load highlight.js:', error);
			// Reset so it can be retried
			loadPromise = null;
		}
	})();

	return loadPromise;
}

/**
 * Load a specific language if not already loaded.
 */
async function loadLanguage(lang: string): Promise<boolean> {
	if (!hljs) return false;

	// Check if already registered
	const normalizedLang = LANGUAGE_ALIASES[lang] || lang;
	try {
		hljs.getLanguage(normalizedLang);
		return true;
	} catch {
		// Not registered, try to load
	}

	try {
		const langModule = await import(`highlight.js/lib/languages/${normalizedLang}`);
		hljs.registerLanguage(normalizedLang, langModule.default);
		return true;
	} catch (error) {
		console.debug(`Language not found: ${normalizedLang}`);
		return false;
	}
}

/**
 * Highlight a code element.
 */
function highlightElement(codeEl: HTMLElement): void {
	if (!hljs || !hljsLoaded) return;

	// Get language from class
	const langClass = codeEl.className.match(/language-(\S+)/);
	const lang = langClass ? langClass[1] : '';
	const normalizedLang = LANGUAGE_ALIASES[lang] || lang;

	// Skip if no language or already highlighted
	if (!normalizedLang || codeEl.dataset.highlighted === 'true') {
		return;
	}

	try {
		// Get the text content
		const code = codeEl.textContent || '';

		// Check if language is available
		const hasLanguage = (() => {
			try {
				return !!hljs!.getLanguage(normalizedLang);
			} catch {
				return false;
			}
		})();

		if (hasLanguage) {
			const result = hljs.highlight(code, { language: normalizedLang });
			codeEl.innerHTML = result.value;
			codeEl.dataset.highlighted = 'true';
		} else {
			// Auto-detect language
			const result = hljs.highlightAuto(code);
			codeEl.innerHTML = result.value;
			codeEl.dataset.highlighted = 'true';
		}
	} catch (error) {
		console.debug('Highlight error:', error);
	}
}

/**
 * Highlight all code blocks in an element.
 */
function highlightAllCodeBlocks(container: HTMLElement): void {
	const codeBlocks = container.querySelectorAll('pre code');
	codeBlocks.forEach((codeEl) => {
		highlightElement(codeEl as HTMLElement);
	});
}

/**
 * Code highlighting plugin.
 */
export const codeHighlightPlugin: EditorPlugin = {
	id: 'code-highlight',
	name: 'Code Highlighting',
	description: 'Syntax highlighting for code blocks using highlight.js',
	version: '1.0.0',

	async onInit(ctx: PluginContext) {
		// Load highlight.js
		await loadHighlightJs();

		// Initial highlighting
		if (hljsLoaded) {
			highlightAllCodeBlocks(ctx.editorElement);
		}
	},

	onDestroy() {
		// Cleanup if needed
	},

	onDocumentChange(ctx: PluginContext) {
		// Re-highlight on document change (debounced in practice)
		if (hljsLoaded) {
			// Only highlight new/modified code blocks
			const codeBlocks = ctx.editorElement.querySelectorAll('pre code:not([data-highlighted="true"])');
			codeBlocks.forEach((codeEl) => {
				highlightElement(codeEl as HTMLElement);
			});
		}
	},

	onBlur(event: FocusEvent, ctx: PluginContext) {
		// Highlight all code blocks when editor loses focus
		if (hljsLoaded) {
			highlightAllCodeBlocks(ctx.editorElement);
		}
	},

	slashItems: [
		{
			id: 'code-js',
			label: 'JavaScript',
			description: 'JavaScript code block',
			icon: 'js',
			keywords: ['code', 'javascript', 'js'],
			group: 'code',
			action: (ctx) => {
				insertCodeBlockWithLanguage(ctx, 'javascript');
			}
		},
		{
			id: 'code-ts',
			label: 'TypeScript',
			description: 'TypeScript code block',
			icon: 'ts',
			keywords: ['code', 'typescript', 'ts'],
			group: 'code',
			action: (ctx) => {
				insertCodeBlockWithLanguage(ctx, 'typescript');
			}
		},
		{
			id: 'code-python',
			label: 'Python',
			description: 'Python code block',
			icon: 'py',
			keywords: ['code', 'python', 'py'],
			group: 'code',
			action: (ctx) => {
				insertCodeBlockWithLanguage(ctx, 'python');
			}
		},
		{
			id: 'code-css',
			label: 'CSS',
			description: 'CSS code block',
			icon: 'css',
			keywords: ['code', 'css', 'styles'],
			group: 'code',
			action: (ctx) => {
				insertCodeBlockWithLanguage(ctx, 'css');
			}
		},
		{
			id: 'code-html',
			label: 'HTML',
			description: 'HTML code block',
			icon: 'html',
			keywords: ['code', 'html', 'markup'],
			group: 'code',
			action: (ctx) => {
				insertCodeBlockWithLanguage(ctx, 'html');
			}
		},
		{
			id: 'code-json',
			label: 'JSON',
			description: 'JSON code block',
			icon: 'json',
			keywords: ['code', 'json', 'data'],
			group: 'code',
			action: (ctx) => {
				insertCodeBlockWithLanguage(ctx, 'json');
			}
		},
		{
			id: 'code-sql',
			label: 'SQL',
			description: 'SQL code block',
			icon: 'sql',
			keywords: ['code', 'sql', 'database', 'query'],
			group: 'code',
			action: (ctx) => {
				insertCodeBlockWithLanguage(ctx, 'sql');
			}
		},
		{
			id: 'code-bash',
			label: 'Bash/Shell',
			description: 'Shell script code block',
			icon: 'sh',
			keywords: ['code', 'bash', 'shell', 'terminal', 'sh'],
			group: 'code',
			action: (ctx) => {
				insertCodeBlockWithLanguage(ctx, 'bash');
			}
		}
	],

	commands: {
		insertCodeBlock: (ctx: PluginContext, ...args: unknown[]) => {
			const [lang] = args as [string | undefined];
			insertCodeBlockWithLanguage(ctx, lang || '');
		},
		highlightAll: (ctx: PluginContext) => {
			if (hljsLoaded) {
				highlightAllCodeBlocks(ctx.editorElement);
			}
		}
	}
};

/**
 * Insert a code block with a specific language.
 */
function insertCodeBlockWithLanguage(ctx: PluginContext, language: string): void {
	const sel = window.getSelection();
	if (!sel || sel.rangeCount === 0) return;

	const range = sel.getRangeAt(0);
	const text = sel.toString() || '// code here';

	const pre = document.createElement('pre');
	const code = document.createElement('code');
	code.className = language ? `language-${language}` : '';
	code.textContent = text;
	pre.appendChild(code);

	range.deleteContents();
	range.insertNode(pre);

	// Place cursor inside the code block
	const newRange = document.createRange();
	newRange.selectNodeContents(code);
	newRange.collapse(false);
	sel.removeAllRanges();
	sel.addRange(newRange);

	ctx.triggerChange();

	// Highlight after a short delay
	if (hljsLoaded && language) {
		setTimeout(() => {
			highlightElement(code);
		}, 50);
	}
}

export default codeHighlightPlugin;
