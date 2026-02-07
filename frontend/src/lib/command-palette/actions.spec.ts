// @vitest-environment jsdom

import { beforeEach, describe, expect, it } from 'vitest';
import { buildDynamicActions } from './actions';
import type { CommandContext } from './types';
import { CAPTURE_PRESETS_STORAGE_KEY } from '$lib/capture/presets';
import { QUICK_CAPTURE_EVENT_NAME } from '$lib/capture/quick-capture';

function makeContext(query: string): CommandContext {
	return {
		query,
		selectedTaskId: null,
		selectedNoteId: null,
		openTaskModal: () => {},
		focusQuickAdd: () => {},
		setQuery: () => {},
		closePalette: () => {},
		navigate: async () => {},
		refreshData: async () => {},
		updateTaskStatus: async () => {},
		addTaskLabel: async () => {},
		selectTask: () => {},
		searchFts: async () => [],
		toast: () => {},
		tasks: []
	};
}

describe('command palette quick-capture presets', () => {
	beforeEach(() => {
		localStorage.removeItem(CAPTURE_PRESETS_STORAGE_KEY);
	});

	it('exposes enabled capture presets as dynamic actions', () => {
		localStorage.setItem(
			CAPTURE_PRESETS_STORAGE_KEY,
			JSON.stringify([
				{
					id: 'preset-1',
					name: 'Morning planning',
					mode: 'task',
					target: 'planned',
					prefill: 'Plan:',
					shortcut: '1',
					enabled: true
				},
				{
					id: 'preset-2',
					name: 'Disabled',
					mode: 'note',
					target: 'daily',
					prefill: '',
					shortcut: 'none',
					enabled: false
				}
			])
		);

		const actions = buildDynamicActions('capture', makeContext('capture'));
		const presetAction = actions.find((action) => action.id === 'quick-capture-preset-preset-1');
		expect(presetAction?.title).toBe('Quick Capture: Morning planning');
		expect(presetAction?.group).toBe('Quick Capture Presets');
		expect(actions.some((action) => action.id === 'quick-capture-preset-preset-2')).toBe(false);
	});

	it('dispatches quick-capture event with preset payload', async () => {
		localStorage.setItem(
			CAPTURE_PRESETS_STORAGE_KEY,
			JSON.stringify([
				{
					id: 'preset-3',
					name: 'Inbox triage',
					mode: 'task',
					target: 'inbox',
					prefill: 'Triage:',
					shortcut: '2',
					enabled: true
				}
			])
		);

		const payloads: Array<{ mode?: string; target?: string; prefill?: string }> = [];
		window.addEventListener(QUICK_CAPTURE_EVENT_NAME, (event) => {
			const detail = event instanceof CustomEvent ? event.detail : null;
			payloads.push(detail ?? {});
		});

		const actions = buildDynamicActions('preset', makeContext('preset'));
		const presetAction = actions.find((action) => action.id === 'quick-capture-preset-preset-3');
		expect(presetAction).toBeDefined();
		await presetAction?.handler(makeContext('preset'));

		expect(payloads).toHaveLength(1);
		expect(payloads[0]).toEqual({
			mode: 'task',
			target: 'inbox',
			prefill: 'Triage:'
		});
	});
});
