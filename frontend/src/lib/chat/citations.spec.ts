// @vitest-environment jsdom
import { describe, expect, it } from 'vitest';
import { formatRelevanceScore, sourceHref, splitCitationSegments } from './citations';

describe('splitCitationSegments', () => {
	it('keeps plain text when there are no citations', () => {
		expect(splitCitationSegments('hello world')).toEqual([
			{ type: 'text', value: 'hello world' }
		]);
	});

	it('splits inline citation markers', () => {
		expect(splitCitationSegments('See [1] and also [2].')).toEqual([
			{ type: 'text', value: 'See ' },
			{ type: 'citation', index: 1, raw: '[1]' },
			{ type: 'text', value: ' and also ' },
			{ type: 'citation', index: 2, raw: '[2]' },
			{ type: 'text', value: '.' }
		]);
	});
});

describe('formatRelevanceScore', () => {
	it('formats unit-interval scores as percent', () => {
		expect(formatRelevanceScore(0.84)).toBe('84%');
	});

	it('avoids fake percentages for RRF-like scores', () => {
		expect(formatRelevanceScore(0.016)).toBe('0.016');
		expect(formatRelevanceScore(1.5)).toBe('1.50');
	});
});

describe('sourceHref', () => {
	it('routes common kinds', () => {
		expect(sourceHref('task', 't1', 'Task')).toBe('/tasks?task=t1');
		expect(sourceHref('fact', 'n1', 'Note')).toBe('/notes?note=n1');
		expect(sourceHref('bookmark', 'b1', 'Link')).toBe('/bookmarks?id=b1');
	});
});
