import { describe, expect, it, vi, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import { toastStore, pushToast } from './toast';

beforeEach(() => {
	toastStore.set([]);
	vi.useFakeTimers();
});

describe('pushToast', () => {
	it('adds a toast to the store', () => {
		pushToast('Hello');
		const toasts = get(toastStore);
		expect(toasts).toHaveLength(1);
		expect(toasts[0].message).toBe('Hello');
		expect(toasts[0].variant).toBe('info');
	});

	it('respects the variant parameter', () => {
		pushToast('Error occurred', 'danger');
		expect(get(toastStore)[0].variant).toBe('danger');
	});

	it('stores undoId when provided', () => {
		pushToast('Deleted', 'success', 3000, 'undo-123');
		expect(get(toastStore)[0].undoId).toBe('undo-123');
	});

	it('auto-removes toast after timeout', () => {
		pushToast('Temporary', 'info', 2000);
		expect(get(toastStore)).toHaveLength(1);
		vi.advanceTimersByTime(2001);
		expect(get(toastStore)).toHaveLength(0);
	});

	it('uses default 3000ms timeout', () => {
		pushToast('Default timeout');
		vi.advanceTimersByTime(2999);
		expect(get(toastStore)).toHaveLength(1);
		vi.advanceTimersByTime(2);
		expect(get(toastStore)).toHaveLength(0);
	});

	it('supports multiple toasts simultaneously', () => {
		pushToast('First');
		pushToast('Second');
		pushToast('Third');
		expect(get(toastStore)).toHaveLength(3);
	});

	it('removes only the expired toast, not others', () => {
		pushToast('Short', 'info', 1000);
		pushToast('Long', 'info', 5000);
		vi.advanceTimersByTime(1001);
		const toasts = get(toastStore);
		expect(toasts).toHaveLength(1);
		expect(toasts[0].message).toBe('Long');
	});

	it('assigns unique ids to each toast', () => {
		pushToast('A');
		pushToast('B');
		const toasts = get(toastStore);
		expect(toasts[0].id).not.toBe(toasts[1].id);
	});
});
