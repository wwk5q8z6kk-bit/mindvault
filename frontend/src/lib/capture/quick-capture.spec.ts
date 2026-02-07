// @vitest-environment jsdom

import { describe, expect, it, vi } from 'vitest';
import {
	QUICK_CAPTURE_EVENT_NAME,
	dispatchQuickCapture,
	isQuickCaptureMode,
	isQuickCaptureTarget
} from './quick-capture';

describe('quick capture dispatch', () => {
	it('validates mode and target helpers', () => {
		expect(isQuickCaptureMode('task')).toBe(true);
		expect(isQuickCaptureMode('daily')).toBe(false);
		expect(isQuickCaptureTarget('inbox')).toBe(true);
		expect(isQuickCaptureTarget('planned')).toBe(true);
		expect(isQuickCaptureTarget('review')).toBe(true);
		expect(isQuickCaptureTarget('archive')).toBe(false);
	});

	it('dispatches quick-capture custom event with normalized detail', () => {
		const listener = vi.fn();
		window.addEventListener(QUICK_CAPTURE_EVENT_NAME, listener);

		dispatchQuickCapture({
			mode: 'task',
			target: 'inbox',
			prefill: '   Follow up: Test reminder   '
		});

		expect(listener).toHaveBeenCalledTimes(1);
		const event = listener.mock.calls[0][0] as CustomEvent;
		expect(event.detail).toEqual({
			mode: 'task',
			target: 'inbox',
			prefill: 'Follow up: Test reminder'
		});
		window.removeEventListener(QUICK_CAPTURE_EVENT_NAME, listener);
	});
});
