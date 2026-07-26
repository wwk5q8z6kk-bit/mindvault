import type { SearchMode } from '$lib/api/search';
import { STORAGE_KEYS } from '$lib/constants/storage-keys';

/** Product default: hybrid matches ADR-003 and chat/recall behavior. */
export const DEFAULT_SEARCH_MODE: SearchMode = 'hybrid';

export function parseSearchMode(value: string | null | undefined): SearchMode | null {
	if (value === 'hybrid' || value === 'fulltext' || value === 'semantic') {
		return value;
	}
	// Saved searches and recall API use "vector" for semantic search.
	if (value === 'vector') {
		return 'semantic';
	}
	return null;
}

export function readStoredSearchMode(): SearchMode {
	if (typeof localStorage === 'undefined') {
		return DEFAULT_SEARCH_MODE;
	}
	try {
		return parseSearchMode(localStorage.getItem(STORAGE_KEYS.SEARCH_MODE)) ?? DEFAULT_SEARCH_MODE;
	} catch {
		return DEFAULT_SEARCH_MODE;
	}
}

export function persistSearchMode(mode: SearchMode): void {
	if (typeof localStorage === 'undefined') {
		return;
	}
	try {
		localStorage.setItem(STORAGE_KEYS.SEARCH_MODE, mode);
	} catch {
		// Ignore quota / private-mode failures.
	}
}

export function emptySearchHint(mode: SearchMode): string {
	switch (mode) {
		case 'fulltext':
			return 'Try different keywords, or switch to Hybrid for semantic matching.';
		case 'semantic':
			return 'Try a more conceptual query, or switch to Hybrid to also match exact terms.';
		case 'hybrid':
			return 'Try different keywords, synonyms, or a more specific phrase.';
		default: {
			const _exhaustive: never = mode;
			return _exhaustive;
		}
	}
}

/** Map UI search mode to API/saved-search `search_type` values. */
export function toApiSearchType(mode: SearchMode): string {
	return mode === 'semantic' ? 'vector' : mode;
}
