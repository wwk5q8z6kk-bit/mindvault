// @vitest-environment jsdom

import { beforeEach, describe, expect, it } from 'vitest';
import { CAPTURE_PRESETS_STORAGE_KEY, findCapturePresetForEvent, loadCapturePresets, saveCapturePresets, type CapturePreset } from './presets';

describe('capture presets', () => {
	beforeEach(() => {
		localStorage.removeItem(CAPTURE_PRESETS_STORAGE_KEY);
	});

	it('loads and saves capture presets with validation', () => {
		const presets: CapturePreset[] = [
			{
				id: 'preset-1',
				name: 'Follow-up',
				mode: 'task',
				target: 'planned',
				prefill: 'Follow up:',
				shortcut: '1',
				enabled: true
			}
		];
		saveCapturePresets(presets);
		expect(loadCapturePresets()).toEqual(presets);
	});

	it('matches keyboard shortcut to enabled preset', () => {
		const preset: CapturePreset = {
			id: 'preset-1',
			name: 'Review queue',
			mode: 'task',
			target: 'review',
			prefill: '',
			shortcut: '2',
			enabled: true
		};
		const event = new KeyboardEvent('keydown', {
			key: '2',
			ctrlKey: true,
			shiftKey: true
		});
		expect(findCapturePresetForEvent(event, [preset])).toEqual(preset);
	});

	it('matches shortcut using digit code even when key is symbol', () => {
		const preset: CapturePreset = {
			id: 'preset-2',
			name: 'Inbox',
			mode: 'task',
			target: 'inbox',
			prefill: '',
			shortcut: '1',
			enabled: true
		};
		const event = new KeyboardEvent('keydown', {
			key: '!',
			code: 'Digit1',
			metaKey: true,
			shiftKey: true
		});
		expect(findCapturePresetForEvent(event, [preset])).toEqual(preset);
	});
});
