import { describe, expect, it } from 'vitest';
import {
	kindLabel,
	kindColor,
	kindBadgeClass,
	ALL_NODE_KINDS,
	RELATIONSHIP_TYPES
} from './kind-helpers';

describe('kindLabel', () => {
	it('returns human-readable labels for known kinds', () => {
		expect(kindLabel('fact')).toBe('Note');
		expect(kindLabel('task')).toBe('Task');
		expect(kindLabel('code_snippet')).toBe('Code Snippet');
		expect(kindLabel('person')).toBe('Person');
	});

	it('returns the raw kind string for unknown kinds', () => {
		expect(kindLabel('unknown_kind')).toBe('unknown_kind');
	});
});

describe('kindColor', () => {
	it('returns hex colors for known kinds', () => {
		expect(kindColor('fact')).toBe('#38bdf8');
		expect(kindColor('task')).toBe('#a78bfa');
		expect(kindColor('event')).toBe('#fb923c');
	});

	it('returns fallback slate color for unknown kinds', () => {
		expect(kindColor('unknown')).toBe('#94a3b8');
	});

	it('returns a color for every kind in ALL_NODE_KINDS', () => {
		for (const kind of ALL_NODE_KINDS) {
			const color = kindColor(kind);
			expect(color).toMatch(/^#[0-9a-f]{6}$/i);
		}
	});
});

describe('kindBadgeClass', () => {
	it('returns Tailwind classes for known kinds', () => {
		expect(kindBadgeClass('fact')).toBe('bg-sky-500/20 text-sky-300');
		expect(kindBadgeClass('task')).toBe('bg-violet-500/20 text-violet-300');
	});

	it('returns fallback slate classes for unknown kinds', () => {
		expect(kindBadgeClass('nonexistent')).toBe('bg-slate-500/20 text-slate-300');
	});

	it('has a badge class for every kind in ALL_NODE_KINDS', () => {
		for (const kind of ALL_NODE_KINDS) {
			const cls = kindBadgeClass(kind);
			expect(cls).toContain('bg-');
			expect(cls).toContain('text-');
		}
	});
});

describe('ALL_NODE_KINDS', () => {
	it('contains 15 node kinds', () => {
		expect(ALL_NODE_KINDS).toHaveLength(15);
	});

	it('includes core kinds', () => {
		expect(ALL_NODE_KINDS).toContain('fact');
		expect(ALL_NODE_KINDS).toContain('task');
		expect(ALL_NODE_KINDS).toContain('event');
		expect(ALL_NODE_KINDS).toContain('person');
	});

	it('includes extended kinds', () => {
		expect(ALL_NODE_KINDS).toContain('decision');
		expect(ALL_NODE_KINDS).toContain('code_snippet');
		expect(ALL_NODE_KINDS).toContain('project');
		expect(ALL_NODE_KINDS).toContain('bookmark');
	});
});

describe('RELATIONSHIP_TYPES', () => {
	it('contains 10 relationship types', () => {
		expect(RELATIONSHIP_TYPES).toHaveLength(10);
	});

	it('includes core relationship types', () => {
		expect(RELATIONSHIP_TYPES).toContain('RelatesTo');
		expect(RELATIONSHIP_TYPES).toContain('DependsOn');
		expect(RELATIONSHIP_TYPES).toContain('PartOf');
		expect(RELATIONSHIP_TYPES).toContain('Contains');
	});
});
