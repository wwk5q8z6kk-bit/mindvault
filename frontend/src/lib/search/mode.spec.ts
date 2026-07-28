// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { STORAGE_KEYS } from '$lib/constants/storage-keys';
import {
	DEFAULT_SEARCH_MODE,
	emptySearchHint,
	parseSearchMode,
	persistSearchMode,
	readStoredSearchMode,
	toApiSearchType
} from './mode';

describe('search mode preferences', () => {
	beforeEach(() => {
		localStorage.clear();
	});

	it('defaults to hybrid', () => {
		expect(DEFAULT_SEARCH_MODE).toBe('hybrid');
		expect(readStoredSearchMode()).toBe('hybrid');
	});

	it('parses known modes and vector alias', () => {
		expect(parseSearchMode('hybrid')).toBe('hybrid');
		expect(parseSearchMode('fulltext')).toBe('fulltext');
		expect(parseSearchMode('semantic')).toBe('semantic');
		expect(parseSearchMode('vector')).toBe('semantic');
		expect(parseSearchMode('nope')).toBeNull();
		expect(parseSearchMode(null)).toBeNull();
	});

	it('persists and restores preferred mode', () => {
		persistSearchMode('fulltext');
		expect(localStorage.getItem(STORAGE_KEYS.SEARCH_MODE)).toBe('fulltext');
		expect(readStoredSearchMode()).toBe('fulltext');
	});

	it('falls back when stored value is invalid', () => {
		localStorage.setItem(STORAGE_KEYS.SEARCH_MODE, 'bogus');
		expect(readStoredSearchMode()).toBe('hybrid');
	});

	it('returns mode-specific empty hints', () => {
		expect(emptySearchHint('fulltext')).toMatch(/Hybrid/i);
		expect(emptySearchHint('hybrid')).not.toMatch(/switch to Hybrid/i);
		expect(emptySearchHint('semantic')).toMatch(/Hybrid/i);
	});

	it('maps UI modes to API search_type', () => {
		expect(toApiSearchType('hybrid')).toBe('hybrid');
		expect(toApiSearchType('fulltext')).toBe('fulltext');
		expect(toApiSearchType('semantic')).toBe('vector');
	});

	it('ignores persist failures', () => {
		const spy = vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
			throw new Error('quota');
		});
		expect(() => persistSearchMode('semantic')).not.toThrow();
		spy.mockRestore();
	});
});
