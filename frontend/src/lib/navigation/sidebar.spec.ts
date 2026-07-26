import { describe, expect, it } from 'vitest';
import {
	isSidebarNavigationItemActive,
	sidebarNavigationGroups,
	sidebarNavigationHref,
	type SidebarNavigationItem
} from './sidebar';

function url(path: string): URL {
	return new URL(path, 'https://mindvault.local');
}

const notes: SidebarNavigationItem = {
	label: 'Notes',
	href: '/notes',
	match: { excludeViews: ['graph'] }
};
const graph: SidebarNavigationItem = {
	label: 'Graph',
	href: '/notes?view=graph',
	match: { aliases: ['/graph'] }
};

describe('sidebar knowledge navigation', () => {
	it('keeps the Notes destination stable while giving Graph a distinct active state', () => {
		expect(isSidebarNavigationItemActive(notes, url('/notes'))).toBe(true);
		expect(isSidebarNavigationItemActive(notes, url('/notes?view=canvas'))).toBe(true);
		expect(isSidebarNavigationItemActive(notes, url('/notes?view=graph'))).toBe(false);
		expect(isSidebarNavigationItemActive(graph, url('/notes?view=graph'))).toBe(true);
		expect(isSidebarNavigationItemActive(graph, url('/notes?view=canvas'))).toBe(false);
		expect(isSidebarNavigationItemActive(graph, url('/graph'))).toBe(true);
	});

	it('classifies Graph and Review under Knowledge instead of system controls', () => {
		const knowledge = sidebarNavigationGroups.find((group) => group.label === 'Knowledge');
		const control = sidebarNavigationGroups.find((group) => group.label === 'Control');

		expect(knowledge?.items.map((item) => item.label)).toEqual(['Graph', 'Review']);
		expect(control?.items.map((item) => item.label)).not.toContain('Review');
	});

	it('preserves subview preferences only for broad destinations', () => {
		const review = sidebarNavigationGroups
			.flatMap((group) => group.items)
			.find((item) => item.label === 'Review');

		expect(review).toBeDefined();
		expect(
			sidebarNavigationHref(review!, {
				tasks: 'list',
				review: 'insights',
				resources: 'bookmarks'
			})
		).toBe('/review?view=insights');
		expect(
			sidebarNavigationHref(notes, {
				tasks: 'board',
				review: 'digest',
				resources: 'flashcards'
			})
		).toBe('/notes');
	});
});
