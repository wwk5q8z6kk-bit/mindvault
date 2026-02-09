import { describe, it, expect, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import {
	toggleSelection,
	replaceSelection,
	selectRange,
	clearSelection,
	isSelected,
	getSelectedIds,
	selectionByKind,
	selectedCount,
	selectedKinds
} from './selection';

// Reset selection before each test
beforeEach(() => {
	clearSelection();
});

describe('selection store', () => {
	describe('toggleSelection', () => {
		it('adds an item to selection', () => {
			toggleSelection('task', 't1');
			expect(isSelected('task', 't1')).toBe(true);
		});

		it('removes an item when toggled again', () => {
			toggleSelection('task', 't1');
			toggleSelection('task', 't1');
			expect(isSelected('task', 't1')).toBe(false);
		});

		it('can select multiple items', () => {
			toggleSelection('task', 't1');
			toggleSelection('task', 't2');
			expect(isSelected('task', 't1')).toBe(true);
			expect(isSelected('task', 't2')).toBe(true);
		});

		it('handles different kinds independently', () => {
			toggleSelection('task', 'x');
			toggleSelection('note', 'x');
			expect(isSelected('task', 'x')).toBe(true);
			expect(isSelected('note', 'x')).toBe(true);
		});

		it('toggling one kind does not affect other kind', () => {
			toggleSelection('task', 't1');
			toggleSelection('note', 'n1');
			toggleSelection('task', 't1'); // remove task
			expect(isSelected('task', 't1')).toBe(false);
			expect(isSelected('note', 'n1')).toBe(true);
		});
	});

	describe('replaceSelection', () => {
		it('replaces all items of a kind', () => {
			toggleSelection('task', 't1');
			toggleSelection('task', 't2');
			replaceSelection('task', ['t3', 't4']);
			expect(isSelected('task', 't1')).toBe(false);
			expect(isSelected('task', 't2')).toBe(false);
			expect(isSelected('task', 't3')).toBe(true);
			expect(isSelected('task', 't4')).toBe(true);
		});

		it('preserves other kind entries', () => {
			toggleSelection('note', 'n1');
			replaceSelection('task', ['t1']);
			expect(isSelected('note', 'n1')).toBe(true);
			expect(isSelected('task', 't1')).toBe(true);
		});

		it('clears kind when given empty array', () => {
			toggleSelection('task', 't1');
			replaceSelection('task', []);
			expect(isSelected('task', 't1')).toBe(false);
		});
	});

	describe('selectRange', () => {
		it('selects a range of items based on anchor', () => {
			const orderedIds = ['a', 'b', 'c', 'd', 'e'];
			toggleSelection('task', 'b'); // sets anchor to 'b'
			selectRange('task', orderedIds, 'd');
			// Should select b, c, d
			expect(isSelected('task', 'b')).toBe(true);
			expect(isSelected('task', 'c')).toBe(true);
			expect(isSelected('task', 'd')).toBe(true);
			expect(isSelected('task', 'a')).toBe(false);
			expect(isSelected('task', 'e')).toBe(false);
		});

		it('selects range in reverse order', () => {
			const orderedIds = ['a', 'b', 'c', 'd', 'e'];
			toggleSelection('task', 'd'); // anchor at d
			selectRange('task', orderedIds, 'b');
			// Should select b, c, d
			expect(isSelected('task', 'b')).toBe(true);
			expect(isSelected('task', 'c')).toBe(true);
			expect(isSelected('task', 'd')).toBe(true);
		});

		it('falls back to toggle when anchor not in ordered list', () => {
			const orderedIds = ['a', 'b', 'c'];
			// No anchor set, so selectRange should fall back
			selectRange('task', orderedIds, 'b');
			expect(isSelected('task', 'b')).toBe(true);
		});

		it('falls back to toggle when target not in ordered list', () => {
			const orderedIds = ['a', 'b', 'c'];
			toggleSelection('task', 'a'); // set anchor
			selectRange('task', orderedIds, 'z'); // z not in list
			expect(isSelected('task', 'z')).toBe(true);
		});

		it('merges with existing selection', () => {
			const orderedIds = ['a', 'b', 'c', 'd', 'e'];
			toggleSelection('task', 'a'); // select a, anchor at a
			selectRange('task', orderedIds, 'b'); // range a-b
			// Now a and b should be selected
			expect(isSelected('task', 'a')).toBe(true);
			expect(isSelected('task', 'b')).toBe(true);
		});
	});

	describe('clearSelection', () => {
		it('clears all selections when no kind specified', () => {
			toggleSelection('task', 't1');
			toggleSelection('note', 'n1');
			clearSelection();
			expect(isSelected('task', 't1')).toBe(false);
			expect(isSelected('note', 'n1')).toBe(false);
		});

		it('clears only the specified kind', () => {
			toggleSelection('task', 't1');
			toggleSelection('note', 'n1');
			clearSelection('task');
			expect(isSelected('task', 't1')).toBe(false);
			expect(isSelected('note', 'n1')).toBe(true);
		});
	});

	describe('getSelectedIds', () => {
		it('returns Set of selected ids for a kind', () => {
			toggleSelection('task', 't1');
			toggleSelection('task', 't2');
			const ids = getSelectedIds('task');
			expect(ids).toBeInstanceOf(Set);
			expect(ids.has('t1')).toBe(true);
			expect(ids.has('t2')).toBe(true);
		});

		it('returns empty Set when nothing selected', () => {
			const ids = getSelectedIds('note');
			expect(ids.size).toBe(0);
		});
	});

	describe('derived stores', () => {
		it('selectionByKind groups by kind', () => {
			toggleSelection('task', 't1');
			toggleSelection('note', 'n1');
			toggleSelection('task', 't2');
			const byKind = get(selectionByKind);
			expect(byKind.task.size).toBe(2);
			expect(byKind.note.size).toBe(1);
		});

		it('selectedCount reflects total count', () => {
			expect(get(selectedCount)).toBe(0);
			toggleSelection('task', 't1');
			expect(get(selectedCount)).toBe(1);
			toggleSelection('note', 'n1');
			expect(get(selectedCount)).toBe(2);
			toggleSelection('task', 't1'); // deselect
			expect(get(selectedCount)).toBe(1);
		});

		it('selectedKinds lists active kinds', () => {
			expect(get(selectedKinds)).toEqual([]);
			toggleSelection('task', 't1');
			expect(get(selectedKinds)).toEqual(['task']);
			toggleSelection('note', 'n1');
			const kinds = get(selectedKinds);
			expect(kinds).toContain('task');
			expect(kinds).toContain('note');
		});
	});

	describe('isSelected', () => {
		it('returns false for unselected items', () => {
			expect(isSelected('task', 'x')).toBe(false);
		});

		it('returns true for selected items', () => {
			toggleSelection('task', 'x');
			expect(isSelected('task', 'x')).toBe(true);
		});

		it('is kind-specific', () => {
			toggleSelection('task', 'x');
			expect(isSelected('note', 'x')).toBe(false);
		});
	});
});
