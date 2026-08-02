import { describe, expect, it } from 'vitest';
import type { KnowledgeNode } from '$lib/api/types';
import {
	graphDestination,
	matchSourceLabel,
	namespaceLabel,
	relativeUpdatedLabel,
	resultDestination,
	sourceLabel
} from './results';

function node(overrides: Partial<KnowledgeNode> = {}): KnowledgeNode {
	return {
		id: 'node/1',
		kind: 'fact',
		title: 'Research note',
		content: null,
		source: null,
		namespace: 'default',
		tags: [],
		importance: 0.5,
		temporal: {
			created_at: '2026-07-01T12:00:00Z',
			updated_at: '2026-07-01T12:00:00Z'
		},
		metadata: {},
		...overrides
	};
}

describe('search result presentation', () => {
	it('routes supported result kinds to their existing destinations', () => {
		expect(resultDestination(node())).toBe('/notes?note=node%2F1');
		expect(resultDestination(node({ kind: 'task' }))).toBe('/tasks?task=node%2F1');
		expect(resultDestination(node({ tags: ['day:2026-07-01'] }))).toBe('/daily');
		expect(graphDestination('node/1')).toBe('/notes?view=graph&node=node%2F1');
	});

	it('uses plain-language match labels instead of exposing raw ranking scores', () => {
		expect(matchSourceLabel('hybrid')).toBe('Hybrid match');
		expect(matchSourceLabel('vector')).toBe('Semantic match');
		expect(matchSourceLabel('full_text')).toBe('Keyword match');
		expect(matchSourceLabel('graph')).toBe('Related match');
		expect(matchSourceLabel('future-source')).toBe('Search match');
	});

	it('formats current vault and source boundaries without exposing full file paths', () => {
		expect(namespaceLabel(null)).toBe('Personal Vault');
		expect(namespaceLabel('default')).toBe('Personal Vault');
		expect(namespaceLabel('research')).toBe('research');
		expect(sourceLabel('https://www.example.com/article')).toBe('example.com');
		expect(sourceLabel('/Users/person/private/research.md')).toBe('research.md');
		expect(sourceLabel('builtin:daily-flow')).toBe('MindVault');
		expect(sourceLabel(null)).toBeNull();
	});

	it('formats update recency defensively', () => {
		const now = Date.parse('2026-07-01T12:30:00Z');
		expect(relativeUpdatedLabel('2026-07-01T12:29:30Z', now)).toBe('Updated just now');
		expect(relativeUpdatedLabel('2026-07-01T12:00:00Z', now)).toBe('Updated 30m ago');
		expect(relativeUpdatedLabel('not-a-date', now)).toBe('Updated recently');
	});
});
