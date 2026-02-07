/**
 * Inline AI Assistance plugin.
 *
 * Features:
 * - Ghost completions: Query after 300ms pause, Tab to accept
 * - Transform menu: On selection, show Summarize, Extract Actions, Refine
 * - Link suggestions: Auto-suggest wiki-links
 */

import type { EditorPlugin, PluginContext } from '../types';
import { assistCompletion, assistTransform, type AssistTransformResponse } from '$lib/api/assist';

// State for AI assistance
interface AIAssistState {
	// Ghost completion
	showGhost: boolean;
	ghostText: string;
	ghostPosition: { node: Node; offset: number } | null;
	isLoadingCompletion: boolean;

	// Transform menu
	showTransformMenu: boolean;
	transformMenuPosition: { x: number; y: number };
	selectedText: string;
	isTransforming: boolean;
	transformResult: string | null;
}

let state: AIAssistState = {
	showGhost: false,
	ghostText: '',
	ghostPosition: null,
	isLoadingCompletion: false,
	showTransformMenu: false,
	transformMenuPosition: { x: 0, y: 0 },
	selectedText: '',
	isTransforming: false,
	transformResult: null
};

// Timers
let completionTimer: ReturnType<typeof setTimeout> | null = null;
const COMPLETION_DELAY = 300;

// Callback for UI updates
let onStateChange: ((state: AIAssistState) => void) | null = null;

// Editor context reference
let editorCtx: PluginContext | null = null;

// Ghost element
let ghostElement: HTMLSpanElement | null = null;

/**
 * Register a callback for state changes.
 */
export function onAIAssistStateChange(callback: (state: AIAssistState) => void): () => void {
	onStateChange = callback;
	return () => {
		onStateChange = null;
	};
}

/**
 * Get the current AI assist state.
 */
export function getAIAssistState(): AIAssistState {
	return { ...state };
}

/**
 * Update state and notify listeners.
 */
function updateState(updates: Partial<AIAssistState>): void {
	state = { ...state, ...updates };
	onStateChange?.(state);
}

/**
 * Clear the completion timer.
 */
function clearCompletionTimer(): void {
	if (completionTimer) {
		clearTimeout(completionTimer);
		completionTimer = null;
	}
}

/**
 * Get the text before the cursor for completion context.
 */
function getContextBeforeCursor(): string | null {
	const sel = window.getSelection();
	if (!sel || sel.rangeCount === 0 || !sel.isCollapsed) return null;

	const range = sel.getRangeAt(0);
	const node = range.startContainer;
	if (node.nodeType !== Node.TEXT_NODE) return null;

	const text = node.textContent || '';
	const offset = range.startOffset;

	// Get text before cursor in this node
	const textBefore = text.slice(0, offset);

	// Get some context from previous sibling nodes (up to 500 chars)
	let context = textBefore;
	let current = node.previousSibling;
	while (current && context.length < 500) {
		if (current.nodeType === Node.TEXT_NODE) {
			context = (current.textContent || '') + context;
		} else if (current.nodeType === Node.ELEMENT_NODE) {
			context = (current as Element).textContent + '\n' + context;
		}
		current = current.previousSibling;
	}

	return context.slice(-500);
}

/**
 * Fetch a completion from the API.
 */
async function fetchCompletion(): Promise<void> {
	if (!editorCtx) return;

	const context = getContextBeforeCursor();
	if (!context || context.trim().length < 10) {
		hideGhost();
		return;
	}

	// Don't fetch if already loading
	if (state.isLoadingCompletion) return;

	updateState({ isLoadingCompletion: true });

	try {
		const response = await assistCompletion({
			text: context,
			limit: 1
		});

		if (response.suggestions.length > 0) {
			const suggestion = response.suggestions[0];
			if (suggestion.trim()) {
				showGhostCompletion(suggestion);
			} else {
				hideGhost();
			}
		} else {
			hideGhost();
		}
	} catch (error) {
		console.debug('AI completion error:', error);
		hideGhost();
	} finally {
		updateState({ isLoadingCompletion: false });
	}
}

/**
 * Show the ghost completion.
 */
function showGhostCompletion(text: string): void {
	if (!editorCtx) return;

	const sel = window.getSelection();
	if (!sel || sel.rangeCount === 0 || !sel.isCollapsed) return;

	const range = sel.getRangeAt(0);

	// Create or update ghost element
	if (!ghostElement) {
		ghostElement = document.createElement('span');
		ghostElement.className = 'mv-ai-ghost';
		ghostElement.contentEditable = 'false';
	}
	ghostElement.textContent = text;

	// Insert ghost at cursor
	range.insertNode(ghostElement);

	// Move cursor before ghost (so it's still at the original position)
	range.setStartBefore(ghostElement);
	range.collapse(true);
	sel.removeAllRanges();
	sel.addRange(range);

	updateState({
		showGhost: true,
		ghostText: text,
		ghostPosition: { node: range.startContainer, offset: range.startOffset }
	});
}

/**
 * Hide the ghost completion.
 */
function hideGhost(): void {
	if (ghostElement && ghostElement.parentNode) {
		ghostElement.remove();
	}
	updateState({
		showGhost: false,
		ghostText: '',
		ghostPosition: null
	});
}

/**
 * Accept the ghost completion.
 */
export function acceptGhostCompletion(): boolean {
	if (!state.showGhost || !state.ghostText || !editorCtx) return false;

	const text = state.ghostText;

	// Remove ghost element
	if (ghostElement && ghostElement.parentNode) {
		ghostElement.remove();
	}

	// Insert the actual text
	document.execCommand('insertText', false, text);

	updateState({
		showGhost: false,
		ghostText: '',
		ghostPosition: null
	});

	editorCtx.triggerChange();
	return true;
}

/**
 * Dismiss the ghost completion.
 */
export function dismissGhostCompletion(): void {
	hideGhost();
}

/**
 * Schedule a completion fetch.
 */
function scheduleCompletion(): void {
	clearCompletionTimer();
	completionTimer = setTimeout(() => {
		fetchCompletion();
	}, COMPLETION_DELAY);
}

/**
 * Show the transform menu.
 */
export function showTransformMenu(): void {
	if (!editorCtx) return;

	const sel = window.getSelection();
	if (!sel || sel.isCollapsed) return;

	const selectedText = sel.toString().trim();
	if (!selectedText) return;

	const range = sel.getRangeAt(0);
	const rect = range.getBoundingClientRect();
	const editorRect = editorCtx.editorElement.getBoundingClientRect();

	updateState({
		showTransformMenu: true,
		transformMenuPosition: {
			x: rect.left - editorRect.left + rect.width / 2,
			y: rect.top - editorRect.top - 8
		},
		selectedText,
		transformResult: null
	});
}

/**
 * Hide the transform menu.
 */
export function hideTransformMenu(): void {
	updateState({
		showTransformMenu: false,
		selectedText: '',
		isTransforming: false,
		transformResult: null
	});
}

/**
 * Transform the selected text.
 */
export async function transformText(
	mode: 'summarize' | 'action_items' | 'refine'
): Promise<void> {
	if (!editorCtx || !state.selectedText) return;

	updateState({ isTransforming: true });

	try {
		const response = await assistTransform({
			text: state.selectedText,
			mode
		});

		updateState({
			transformResult: response.transformed_text,
			isTransforming: false
		});
	} catch (error) {
		console.error('Transform error:', error);
		updateState({ isTransforming: false });
	}
}

/**
 * Apply the transform result.
 */
export function applyTransformResult(): void {
	if (!editorCtx || !state.transformResult) return;

	const sel = window.getSelection();
	if (!sel) return;

	// Replace selected text with result
	document.execCommand('insertText', false, state.transformResult);

	hideTransformMenu();
	editorCtx.triggerChange();
}

/**
 * AI Assist plugin.
 */
export const aiAssistPlugin: EditorPlugin = {
	id: 'ai-assist',
	name: 'AI Assistance',
	description: 'Ghost completions and text transformations',
	version: '1.0.0',

	onInit(ctx: PluginContext) {
		editorCtx = ctx;

		// Reset state
		state = {
			showGhost: false,
			ghostText: '',
			ghostPosition: null,
			isLoadingCompletion: false,
			showTransformMenu: false,
			transformMenuPosition: { x: 0, y: 0 },
			selectedText: '',
			isTransforming: false,
			transformResult: null
		};
	},

	onDestroy() {
		clearCompletionTimer();
		hideGhost();
		editorCtx = null;
		onStateChange = null;
	},

	onKeyDown(event: KeyboardEvent, ctx: PluginContext): boolean | void {
		// Cmd+. or Ctrl+. to show transform menu (when text is selected)
		if ((event.metaKey || event.ctrlKey) && event.key === '.') {
			const sel = window.getSelection();
			if (sel && !sel.isCollapsed && sel.toString().trim()) {
				event.preventDefault();
				showTransformMenu();
				return true;
			}
		}

		// Tab to accept ghost completion
		if (event.key === 'Tab' && state.showGhost && !event.shiftKey) {
			event.preventDefault();
			acceptGhostCompletion();
			return true;
		}

		// Escape to dismiss ghost
		if (event.key === 'Escape' && state.showGhost) {
			event.preventDefault();
			dismissGhostCompletion();
			return true;
		}

		// Escape to hide transform menu
		if (event.key === 'Escape' && state.showTransformMenu) {
			event.preventDefault();
			hideTransformMenu();
			return true;
		}

		// Any other key dismisses ghost
		if (state.showGhost && event.key.length === 1 && !event.metaKey && !event.ctrlKey) {
			dismissGhostCompletion();
		}

		// Clear timer on any key
		clearCompletionTimer();
	},

	onDocumentChange(ctx: PluginContext) {
		// Hide ghost on content change (unless we're loading)
		if (state.showGhost && !state.isLoadingCompletion) {
			hideGhost();
		}

		// Schedule new completion
		if (!state.isLoadingCompletion) {
			scheduleCompletion();
		}
	},

	onSelectionChange(ctx: PluginContext) {
		// Hide transform menu if selection is cleared
		if (state.showTransformMenu) {
			const sel = window.getSelection();
			if (!sel || sel.isCollapsed) {
				hideTransformMenu();
			}
		}

		// Hide ghost if cursor moves away
		if (state.showGhost) {
			const sel = window.getSelection();
			if (!sel || !sel.isCollapsed) {
				hideGhost();
			}
		}
	},

	onBlur(event: FocusEvent, ctx: PluginContext) {
		clearCompletionTimer();
		hideGhost();
		// Delay hiding transform menu to allow click handling
		setTimeout(() => {
			if (state.showTransformMenu) {
				hideTransformMenu();
			}
		}, 200);
	},

	slashItems: [
		{
			id: 'ai-summarize',
			label: 'Summarize',
			description: 'Summarize selected text',
			icon: '📝',
			keywords: ['ai', 'summarize', 'summary', 'tldr'],
			group: 'ai',
			action: async (ctx) => {
				const sel = window.getSelection();
				if (sel && !sel.isCollapsed) {
					showTransformMenu();
				}
			}
		},
		{
			id: 'ai-actions',
			label: 'Extract Actions',
			description: 'Extract action items from text',
			icon: '✅',
			keywords: ['ai', 'actions', 'todos', 'tasks', 'extract'],
			group: 'ai',
			action: async (ctx) => {
				const sel = window.getSelection();
				if (sel && !sel.isCollapsed) {
					await transformText('action_items');
					applyTransformResult();
				}
			}
		},
		{
			id: 'ai-refine',
			label: 'Refine Writing',
			description: 'Improve the selected text',
			icon: '✨',
			keywords: ['ai', 'refine', 'improve', 'rewrite', 'polish'],
			group: 'ai',
			action: async (ctx) => {
				const sel = window.getSelection();
				if (sel && !sel.isCollapsed) {
					await transformText('refine');
					applyTransformResult();
				}
			}
		}
	],

	commands: {
		acceptCompletion: () => { acceptGhostCompletion(); },
		dismissCompletion: () => { dismissGhostCompletion(); },
		showTransformMenu: () => { showTransformMenu(); },
		hideTransformMenu: () => { hideTransformMenu(); },
		summarize: () => transformText('summarize'),
		extractActions: () => transformText('action_items'),
		refine: () => transformText('refine'),
		applyTransform: () => { applyTransformResult(); }
	}
};

export default aiAssistPlugin;
