/**
 * @mentions plugin for wiki-link autocomplete.
 *
 * Features:
 * - Triggers on @ character or [[ sequence
 * - Queries assistLinks() API for suggestions
 * - Displays floating menu with note suggestions
 * - Inserts wiki-link [[target|alias]] on selection
 */

import type { EditorPlugin, PluginContext } from '../types';
import { assistLinks, type AssistLinkSuggestion } from '$lib/api/assist';

// Trigger type: '@' or '[['
type TriggerType = '@' | '[[';

// State for the mentions popup
interface MentionState {
	isOpen: boolean;
	query: string;
	suggestions: AssistLinkSuggestion[];
	activeIndex: number;
	loading: boolean;
	triggerRange: Range | null;
	position: { x: number; y: number };
	debounceTimer: ReturnType<typeof setTimeout> | null;
	triggerType: TriggerType;
}

let state: MentionState = {
	isOpen: false,
	query: '',
	suggestions: [],
	activeIndex: 0,
	loading: false,
	triggerRange: null,
	position: { x: 0, y: 0 },
	debounceTimer: null,
	triggerType: '@'
};

// Callbacks for UI updates
let onStateChange: ((state: MentionState) => void) | null = null;

/**
 * Register a callback for state changes.
 * Used by the UI component to react to state updates.
 */
export function onMentionStateChange(callback: (state: MentionState) => void): () => void {
	onStateChange = callback;
	// Return cleanup function
	return () => {
		onStateChange = null;
	};
}

/**
 * Get the current mention state.
 */
export function getMentionState(): MentionState {
	return { ...state };
}

/**
 * Update state and notify listeners.
 */
function updateState(updates: Partial<MentionState>): void {
	state = { ...state, ...updates };
	onStateChange?.(state);
}

/**
 * Open the mention menu.
 */
function openMentionMenu(editorElement: HTMLElement, triggerType: TriggerType = '@'): void {
	const sel = window.getSelection();
	if (!sel || sel.rangeCount === 0) return;

	const range = sel.getRangeAt(0);
	const rect = range.getBoundingClientRect();
	const editorRect = editorElement.getBoundingClientRect();

	updateState({
		isOpen: true,
		query: '',
		suggestions: [],
		activeIndex: 0,
		loading: false,
		triggerRange: range.cloneRange(),
		position: {
			x: rect.left - editorRect.left,
			y: rect.bottom - editorRect.top + 4
		},
		triggerType
	});
}

/**
 * Close the mention menu.
 */
function closeMentionMenu(): void {
	if (state.debounceTimer) {
		clearTimeout(state.debounceTimer);
	}
	updateState({
		isOpen: false,
		query: '',
		suggestions: [],
		activeIndex: 0,
		loading: false,
		triggerRange: null,
		debounceTimer: null,
		triggerType: '@'
	});
}

/**
 * Search for suggestions.
 */
async function searchSuggestions(query: string, excludeNodeId?: string): Promise<void> {
	if (query.length < 1) {
		updateState({ suggestions: [], loading: false });
		return;
	}

	updateState({ loading: true });

	try {
		const response = await assistLinks({
			text: query,
			limit: 8,
			exclude_node_id: excludeNodeId
		});
		updateState({
			suggestions: response.suggestions,
			loading: false
		});
	} catch (error) {
		console.debug('Mention search error:', error);
		updateState({ suggestions: [], loading: false });
	}
}

/**
 * Handle query change with debouncing.
 */
function handleQueryChange(query: string, excludeNodeId?: string): void {
	updateState({ query, activeIndex: 0 });

	if (state.debounceTimer) {
		clearTimeout(state.debounceTimer);
	}

	state.debounceTimer = setTimeout(() => {
		searchSuggestions(query, excludeNodeId);
	}, 200);
}

/**
 * Select a suggestion and insert wiki-link.
 */
function selectSuggestion(suggestion: AssistLinkSuggestion, ctx: PluginContext): void {
	if (!state.triggerRange) return;

	const sel = window.getSelection();
	if (!sel) return;

	// Calculate how much to delete based on trigger type
	// @ = 1 char, [[ = 2 chars
	const triggerLength = state.triggerType === '[[' ? 2 : 1;
	const deleteLength = triggerLength + state.query.length;

	// Restore selection to the trigger point
	const range = state.triggerRange.cloneRange();
	const startOffset = Math.max(0, range.startOffset - deleteLength);
	range.setStart(range.startContainer, startOffset);
	sel.removeAllRanges();
	sel.addRange(range);

	// Delete the trigger and query
	document.execCommand('delete');

	// Insert the wiki-link
	const target = suggestion.node_id || suggestion.title;
	const alias = suggestion.heading ? `${suggestion.title} > ${suggestion.heading}` : suggestion.title;
	const wikiLink = target === alias ? `[[${target}]]` : `[[${target}|${alias}]]`;

	document.execCommand('insertText', false, wikiLink);

	// Close the menu
	closeMentionMenu();

	// Trigger change
	ctx.triggerChange();
}

/**
 * Create a new note with the query as title.
 */
function createNewNote(title: string, ctx: PluginContext): void {
	if (!state.triggerRange) return;

	const sel = window.getSelection();
	if (!sel) return;

	// Calculate how much to delete based on trigger type
	const triggerLength = state.triggerType === '[[' ? 2 : 1;
	const deleteLength = triggerLength + state.query.length;

	// Restore selection to the trigger point
	const range = state.triggerRange.cloneRange();
	const startOffset = Math.max(0, range.startOffset - deleteLength);
	range.setStart(range.startContainer, startOffset);
	sel.removeAllRanges();
	sel.addRange(range);

	// Delete the trigger and query
	document.execCommand('delete');

	// Insert a wiki-link (the note will be created when clicked)
	const wikiLink = `[[${title}]]`;
	document.execCommand('insertText', false, wikiLink);

	// Close the menu
	closeMentionMenu();

	// Emit event to create the note
	ctx.emit('createNote', { title });

	// Trigger change
	ctx.triggerChange();
}

/**
 * Handle selection of a suggestion by external UI.
 */
export function handleMentionSelect(suggestion: AssistLinkSuggestion, ctx: PluginContext): void {
	selectSuggestion(suggestion, ctx);
}

/**
 * Handle creation of a new note by external UI.
 */
export function handleMentionCreateNew(title: string, ctx: PluginContext): void {
	createNewNote(title, ctx);
}

/**
 * Handle menu close by external UI.
 */
export function handleMentionClose(): void {
	closeMentionMenu();
}

/**
 * Navigate suggestions with arrow keys.
 */
function navigateSuggestions(direction: 'up' | 'down'): void {
	const totalItems = state.suggestions.length + (state.query.length > 0 ? 1 : 0);
	if (totalItems === 0) return;

	let newIndex = state.activeIndex;
	if (direction === 'down') {
		newIndex = (state.activeIndex + 1) % totalItems;
	} else {
		newIndex = (state.activeIndex - 1 + totalItems) % totalItems;
	}

	updateState({ activeIndex: newIndex });
}

/**
 * Mentions plugin.
 */
export const mentionsPlugin: EditorPlugin = {
	id: 'mentions',
	name: 'Wiki Links',
	description: 'Wiki-link autocomplete triggered by @ or [[',
	version: '1.1.0',

	onInit(ctx: PluginContext) {
		// Initialize state
		closeMentionMenu();
	},

	onDestroy() {
		if (state.debounceTimer) {
			clearTimeout(state.debounceTimer);
		}
		onStateChange = null;
	},

	onKeyDown(event: KeyboardEvent, ctx: PluginContext): boolean | void {
		// Handle menu navigation when open
		if (state.isOpen) {
			if (event.key === 'ArrowDown') {
				event.preventDefault();
				navigateSuggestions('down');
				return true;
			}
			if (event.key === 'ArrowUp') {
				event.preventDefault();
				navigateSuggestions('up');
				return true;
			}
			if (event.key === 'Enter' || event.key === 'Tab') {
				event.preventDefault();
				const totalItems = state.suggestions.length + (state.query.length > 0 ? 1 : 0);
				if (state.activeIndex < state.suggestions.length) {
					selectSuggestion(state.suggestions[state.activeIndex], ctx);
				} else if (state.activeIndex === state.suggestions.length && state.query.length > 0) {
					createNewNote(state.query, ctx);
				}
				return true;
			}
			if (event.key === 'Escape') {
				event.preventDefault();
				closeMentionMenu();
				return true;
			}
		}

		// Trigger on @ at start of word
		if (event.key === '@' && !event.metaKey && !event.ctrlKey && !state.isOpen) {
			const sel = window.getSelection();
			if (sel && sel.rangeCount > 0) {
				const range = sel.getRangeAt(0);
				const textNode = range.startContainer;
				const offset = range.startOffset;
				const text = textNode.textContent ?? '';
				const charBefore = offset > 0 ? text[offset - 1] : '';

				// Only trigger at word start
				if (offset === 0 || charBefore === ' ' || charBefore === '\n') {
					// Delay to let the @ be inserted first
					setTimeout(() => openMentionMenu(ctx.editorElement, '@'), 10);
				}
			}
		}

		// Trigger on [[ (second bracket)
		if (event.key === '[' && !event.metaKey && !event.ctrlKey && !state.isOpen) {
			const sel = window.getSelection();
			if (sel && sel.rangeCount > 0) {
				const range = sel.getRangeAt(0);
				const textNode = range.startContainer;
				const offset = range.startOffset;
				const text = textNode.textContent ?? '';
				const charBefore = offset > 0 ? text[offset - 1] : '';

				// Trigger if the character before is also [
				if (charBefore === '[') {
					// Delay to let the second [ be inserted first
					setTimeout(() => openMentionMenu(ctx.editorElement, '[['), 10);
				}
			}
		}
	},

	onDocumentChange(ctx: PluginContext) {
		// Update query while menu is open
		if (state.isOpen && state.triggerRange) {
			const sel = window.getSelection();
			if (sel && sel.rangeCount > 0) {
				const range = sel.getRangeAt(0);
				const textNode = range.startContainer;
				if (textNode.nodeType === Node.TEXT_NODE) {
					const text = textNode.textContent ?? '';
					const cursorPos = range.startOffset;

					// Find trigger position based on trigger type
					let triggerIdx = -1;
					let triggerLength = 1;

					if (state.triggerType === '[[') {
						// Look for [[ before cursor
						const searchStart = Math.max(0, cursorPos - 32);
						const searchText = text.substring(searchStart, cursorPos);
						const bracketIdx = searchText.lastIndexOf('[[');
						if (bracketIdx >= 0) {
							triggerIdx = searchStart + bracketIdx;
							triggerLength = 2;
						}
					} else {
						// Look for @ before cursor
						triggerIdx = text.lastIndexOf('@', cursorPos - 1);
					}

					if (triggerIdx >= 0) {
						const query = text.substring(triggerIdx + triggerLength, cursorPos);
						// If query contains space, closing bracket, or we've moved too far, close menu
						if (query.includes(' ') || query.includes(']') || cursorPos - triggerIdx > 32) {
							closeMentionMenu();
						} else {
							handleQueryChange(query);
						}
					} else {
						closeMentionMenu();
					}
				}
			}
		}
	},

	onBlur(event: FocusEvent, ctx: PluginContext) {
		// Close menu on blur (with delay to allow click handling)
		setTimeout(() => {
			if (state.isOpen) {
				closeMentionMenu();
			}
		}, 200);
	}
};

export default mentionsPlugin;
