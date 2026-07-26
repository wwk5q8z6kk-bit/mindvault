// @vitest-environment jsdom
import { describe, expect, it } from 'vitest';
import { filterNotesForSidebar, normalizeNotesListTab } from './sidebar';

describe('notes sidebar helpers', () => {
	const notes = [
		{ kind: 'fact', title: 'Alpha', markdown: 'ship defaults', pinned: true, tags: ['core'] },
		{ kind: 'decision', title: 'Beta', markdown: 'choose hybrid', pinned: false, tags: ['core'] },
		{ kind: 'fact', title: 'Gamma', markdown: 'unrelated', pinned: false, tags: ['misc'] }
	];

	it('normalizes list tabs', () => {
		expect(normalizeNotesListTab('PINNED')).toBe('pinned');
		expect(normalizeNotesListTab('fact', ['fact'])).toBe('fact');
		expect(normalizeNotesListTab('???')).toBe('all');
	});

	it('filters by pinned and kind tabs', () => {
		expect(filterNotesForSidebar(notes, { tab: 'pinned' }).map((n) => n.title)).toEqual(['Alpha']);
		expect(filterNotesForSidebar(notes, { tab: 'decision' }).map((n) => n.title)).toEqual(['Beta']);
	});

	it('applies search and tag filters with tabs', () => {
		const result = filterNotesForSidebar(notes, {
			tab: 'fact',
			searchQuery: 'ship',
			tagFilter: 'core'
		});
		expect(result.map((n) => n.title)).toEqual(['Alpha']);
	});
});
