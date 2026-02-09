import { get } from 'svelte/store';
import { describe, it, expect, beforeEach } from 'vitest';
import { activeNamespace, availableNamespaces } from './namespace';

beforeEach(() => {
	activeNamespace.set(null);
	availableNamespaces.set([]);
});

describe('activeNamespace', () => {
	it('starts as null', () => {
		expect(get(activeNamespace)).toBeNull();
	});

	it('can be set to a namespace string', () => {
		activeNamespace.set('work');
		expect(get(activeNamespace)).toBe('work');
	});

	it('can be cleared back to null', () => {
		activeNamespace.set('work');
		activeNamespace.set(null);
		expect(get(activeNamespace)).toBeNull();
	});
});

describe('availableNamespaces', () => {
	it('starts empty', () => {
		expect(get(availableNamespaces)).toEqual([]);
	});

	it('holds a list of namespace strings', () => {
		availableNamespaces.set(['default', 'work', 'personal']);
		const result = get(availableNamespaces);
		expect(result).toHaveLength(3);
		expect(result).toContain('work');
	});
});
