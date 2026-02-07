/**
 * Search & Replace plugin.
 *
 * Features:
 * - Ctrl/Cmd+F opens search bar
 * - Highlight all matches
 * - Next/Previous navigation
 * - Replace / Replace All with undo support
 */

import type { EditorPlugin, PluginContext } from '../types';

// State for search
interface SearchState {
	isOpen: boolean;
	query: string;
	replaceText: string;
	caseSensitive: boolean;
	useRegex: boolean;
	matches: SearchMatch[];
	currentMatchIndex: number;
	showReplace: boolean;
}

interface SearchMatch {
	node: Text;
	start: number;
	end: number;
	text: string;
}

let state: SearchState = {
	isOpen: false,
	query: '',
	replaceText: '',
	caseSensitive: false,
	useRegex: false,
	matches: [],
	currentMatchIndex: -1,
	showReplace: false
};

// Callback for UI updates
let onStateChange: ((state: SearchState) => void) | null = null;

// Reference to the editor context
let editorCtx: PluginContext | null = null;

// Highlight elements
const highlightClass = 'mv-search-highlight';
const currentHighlightClass = 'mv-search-highlight-current';

/**
 * Register a callback for state changes.
 */
export function onSearchStateChange(callback: (state: SearchState) => void): () => void {
	onStateChange = callback;
	return () => {
		onStateChange = null;
	};
}

/**
 * Get the current search state.
 */
export function getSearchState(): SearchState {
	return { ...state, matches: [...state.matches] };
}

/**
 * Update state and notify listeners.
 */
function updateState(updates: Partial<SearchState>): void {
	state = { ...state, ...updates };
	onStateChange?.(state);
}

/**
 * Open the search bar.
 */
export function openSearch(showReplace = false): void {
	updateState({ isOpen: true, showReplace });
}

/**
 * Close the search bar.
 */
export function closeSearch(): void {
	clearHighlights();
	updateState({
		isOpen: false,
		query: '',
		replaceText: '',
		matches: [],
		currentMatchIndex: -1
	});
}

/**
 * Set the search query.
 */
export function setSearchQuery(query: string): void {
	updateState({ query });
	if (editorCtx) {
		performSearch(editorCtx);
	}
}

/**
 * Set replace text.
 */
export function setReplaceText(text: string): void {
	updateState({ replaceText: text });
}

/**
 * Toggle case sensitivity.
 */
export function toggleCaseSensitive(): void {
	updateState({ caseSensitive: !state.caseSensitive });
	if (editorCtx) {
		performSearch(editorCtx);
	}
}

/**
 * Toggle regex mode.
 */
export function toggleRegex(): void {
	updateState({ useRegex: !state.useRegex });
	if (editorCtx) {
		performSearch(editorCtx);
	}
}

/**
 * Clear all highlights from the editor.
 */
function clearHighlights(): void {
	if (!editorCtx) return;

	const highlights = editorCtx.editorElement.querySelectorAll(`.${highlightClass}`);
	highlights.forEach((el) => {
		const parent = el.parentNode;
		if (parent) {
			const text = document.createTextNode(el.textContent || '');
			parent.replaceChild(text, el);
			parent.normalize();
		}
	});
}

/**
 * Find all text nodes in an element.
 */
function getTextNodes(element: HTMLElement): Text[] {
	const nodes: Text[] = [];
	const walker = document.createTreeWalker(element, NodeFilter.SHOW_TEXT, {
		acceptNode: (node) => {
			// Skip if inside a code block or pre
			const parent = node.parentElement;
			if (parent?.closest('pre, code, .mv-search-highlight')) {
				return NodeFilter.FILTER_REJECT;
			}
			return NodeFilter.FILTER_ACCEPT;
		}
	});

	let node: Node | null;
	while ((node = walker.nextNode())) {
		nodes.push(node as Text);
	}
	return nodes;
}

/**
 * Perform the search.
 */
function performSearch(ctx: PluginContext): void {
	clearHighlights();

	if (!state.query) {
		updateState({ matches: [], currentMatchIndex: -1 });
		return;
	}

	const matches: SearchMatch[] = [];
	const textNodes = getTextNodes(ctx.editorElement);

	let pattern: RegExp;
	try {
		const flags = state.caseSensitive ? 'g' : 'gi';
		if (state.useRegex) {
			pattern = new RegExp(state.query, flags);
		} else {
			// Escape special regex characters
			const escaped = state.query.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
			pattern = new RegExp(escaped, flags);
		}
	} catch {
		// Invalid regex
		updateState({ matches: [], currentMatchIndex: -1 });
		return;
	}

	for (const node of textNodes) {
		const text = node.textContent || '';
		let match;
		while ((match = pattern.exec(text)) !== null) {
			matches.push({
				node,
				start: match.index,
				end: match.index + match[0].length,
				text: match[0]
			});
		}
	}

	// Apply highlights
	applyHighlights(matches);

	const newIndex = matches.length > 0 ? 0 : -1;
	updateState({ matches, currentMatchIndex: newIndex });

	if (newIndex >= 0) {
		scrollToMatch(newIndex);
	}
}

/**
 * Apply highlight markup to matches.
 */
function applyHighlights(matches: SearchMatch[]): void {
	// Group matches by node to process from end to start (to preserve indices)
	const nodeMatches = new Map<Text, SearchMatch[]>();
	for (const match of matches) {
		const existing = nodeMatches.get(match.node) || [];
		existing.push(match);
		nodeMatches.set(match.node, existing);
	}

	for (const [node, nodeMatchList] of nodeMatches) {
		// Sort by position descending to preserve indices
		nodeMatchList.sort((a, b) => b.start - a.start);

		let currentNode = node;
		for (const match of nodeMatchList) {
			const text = currentNode.textContent || '';
			const before = text.slice(0, match.start);
			const matched = text.slice(match.start, match.end);
			const after = text.slice(match.end);

			// Create highlight span
			const span = document.createElement('span');
			span.className = highlightClass;
			span.textContent = matched;

			// Replace the text node
			const parent = currentNode.parentNode;
			if (parent) {
				const fragment = document.createDocumentFragment();
				if (before) fragment.appendChild(document.createTextNode(before));
				fragment.appendChild(span);
				if (after) {
					const afterNode = document.createTextNode(after);
					fragment.appendChild(afterNode);
					currentNode = afterNode;
				}
				parent.replaceChild(fragment, currentNode === node ? node : currentNode);
			}
		}
	}
}

/**
 * Scroll to a specific match.
 */
function scrollToMatch(index: number): void {
	if (!editorCtx) return;

	// Remove current highlight from all
	const highlights = editorCtx.editorElement.querySelectorAll(`.${highlightClass}`);
	highlights.forEach((el) => el.classList.remove(currentHighlightClass));

	// Add current highlight to the match
	if (index >= 0 && index < highlights.length) {
		const current = highlights[index];
		current.classList.add(currentHighlightClass);
		current.scrollIntoView({ behavior: 'smooth', block: 'center' });
	}
}

/**
 * Go to next match.
 */
export function nextMatch(): void {
	if (state.matches.length === 0) return;

	const newIndex = (state.currentMatchIndex + 1) % state.matches.length;
	updateState({ currentMatchIndex: newIndex });
	scrollToMatch(newIndex);
}

/**
 * Go to previous match.
 */
export function prevMatch(): void {
	if (state.matches.length === 0) return;

	const newIndex = (state.currentMatchIndex - 1 + state.matches.length) % state.matches.length;
	updateState({ currentMatchIndex: newIndex });
	scrollToMatch(newIndex);
}

/**
 * Replace the current match.
 */
export function replaceCurrentMatch(): void {
	if (!editorCtx || state.currentMatchIndex < 0) return;

	const highlights = editorCtx.editorElement.querySelectorAll(`.${highlightClass}`);
	if (state.currentMatchIndex >= highlights.length) return;

	const current = highlights[state.currentMatchIndex];
	const parent = current.parentNode;
	if (parent) {
		const replacement = document.createTextNode(state.replaceText);
		parent.replaceChild(replacement, current);
		parent.normalize();
	}

	editorCtx.triggerChange();

	// Re-search to update matches
	performSearch(editorCtx);
}

/**
 * Replace all matches.
 */
export function replaceAllMatches(): void {
	if (!editorCtx || state.matches.length === 0) return;

	const highlights = editorCtx.editorElement.querySelectorAll(`.${highlightClass}`);
	highlights.forEach((el) => {
		const parent = el.parentNode;
		if (parent) {
			const replacement = document.createTextNode(state.replaceText);
			parent.replaceChild(replacement, el);
			parent.normalize();
		}
	});

	editorCtx.triggerChange();

	// Clear search
	updateState({ matches: [], currentMatchIndex: -1 });
}

/**
 * Search & Replace plugin.
 */
export const searchReplacePlugin: EditorPlugin = {
	id: 'search-replace',
	name: 'Search & Replace',
	description: 'Find and replace text in the editor',
	version: '1.0.0',

	onInit(ctx: PluginContext) {
		editorCtx = ctx;
		// Reset state
		state = {
			isOpen: false,
			query: '',
			replaceText: '',
			caseSensitive: false,
			useRegex: false,
			matches: [],
			currentMatchIndex: -1,
			showReplace: false
		};
	},

	onDestroy() {
		clearHighlights();
		editorCtx = null;
		onStateChange = null;
	},

	onKeyDown(event: KeyboardEvent, ctx: PluginContext): boolean | void {
		const isMod = event.metaKey || event.ctrlKey;

		// Cmd/Ctrl+F: Open search
		if (isMod && event.key === 'f' && !event.shiftKey) {
			event.preventDefault();
			openSearch(false);
			return true;
		}

		// Cmd/Ctrl+H: Open search with replace
		if (isMod && event.key === 'h') {
			event.preventDefault();
			openSearch(true);
			return true;
		}

		// When search is open
		if (state.isOpen) {
			// Escape: Close search
			if (event.key === 'Escape') {
				event.preventDefault();
				closeSearch();
				return true;
			}

			// Enter: Next match
			if (event.key === 'Enter' && !event.shiftKey) {
				event.preventDefault();
				nextMatch();
				return true;
			}

			// Shift+Enter: Previous match
			if (event.key === 'Enter' && event.shiftKey) {
				event.preventDefault();
				prevMatch();
				return true;
			}

			// F3 / Cmd+G: Next match
			if (event.key === 'F3' || (isMod && event.key === 'g' && !event.shiftKey)) {
				event.preventDefault();
				nextMatch();
				return true;
			}

			// Shift+F3 / Cmd+Shift+G: Previous match
			if (
				(event.key === 'F3' && event.shiftKey) ||
				(isMod && event.key === 'g' && event.shiftKey)
			) {
				event.preventDefault();
				prevMatch();
				return true;
			}
		}
	},

	keyboardShortcuts: [
		{
			key: 'f',
			mod: true,
			handler: (_ctx, _event) => {
				openSearch(false);
				return true;
			}
		},
		{
			key: 'h',
			mod: true,
			handler: (_ctx, _event) => {
				openSearch(true);
				return true;
			}
		}
	],

	commands: {
		openSearch: () => openSearch(false),
		openSearchReplace: () => openSearch(true),
		closeSearch: () => closeSearch(),
		nextMatch: () => nextMatch(),
		prevMatch: () => prevMatch(),
		replaceCurrentMatch: () => replaceCurrentMatch(),
		replaceAll: () => replaceAllMatches()
	}
};

export default searchReplacePlugin;
