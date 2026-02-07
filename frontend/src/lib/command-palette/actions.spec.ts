// @vitest-environment jsdom

import { beforeEach, describe, expect, it } from 'vitest';
import { get } from 'svelte/store';
import { buildDynamicActions, registerBuiltInActions } from './actions';
import type { CommandContext } from './types';
import { CAPTURE_PRESETS_STORAGE_KEY } from '$lib/capture/presets';
import { QUICK_CAPTURE_EVENT_NAME } from '$lib/capture/quick-capture';
import { INBOX_TRIAGE_APPLY_TOP_EVENT_NAME, INBOX_TRIAGE_EVENT_NAME } from '$lib/inbox/triage';
import { actionsStore } from './registry';

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
		actionsStore.set([]);
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

	it('registers AI inbox triage action and dispatches triage event', async () => {
		registerBuiltInActions();
		const builtInAction = get(actionsStore).find((action) => action.id === 'ai-triage-inbox');
		expect(builtInAction).toBeDefined();

		const triageEvents: Event[] = [];
		window.addEventListener(INBOX_TRIAGE_EVENT_NAME, (event) => {
			triageEvents.push(event);
		});

		const navigations: string[] = [];
		await builtInAction?.handler({
			...makeContext(''),
			navigate: async (path: string) => {
				navigations.push(path);
			}
		});

		expect(navigations).toEqual(['/inbox']);
		expect(triageEvents).toHaveLength(1);
	});

	it('registers AI inbox apply-top action and dispatches apply event', async () => {
		registerBuiltInActions();
		const builtInAction = get(actionsStore).find((action) => action.id === 'ai-triage-inbox-apply');
		expect(builtInAction).toBeDefined();

		const applyEvents: Event[] = [];
		window.addEventListener(INBOX_TRIAGE_APPLY_TOP_EVENT_NAME, (event) => {
			applyEvents.push(event);
		});

		const navigations: string[] = [];
		await builtInAction?.handler({
			...makeContext(''),
			navigate: async (path: string) => {
				navigations.push(path);
			}
		});

		expect(navigations).toEqual(['/inbox']);
		expect(applyEvents).toHaveLength(1);
	});

	it('builds dynamic triage-top action with explicit limit', async () => {
		const applyEventDetails: Array<{ limit?: number } | undefined> = [];
		window.addEventListener(INBOX_TRIAGE_APPLY_TOP_EVENT_NAME, (event) => {
			const customEvent = event as CustomEvent<{ limit?: number }>;
			applyEventDetails.push(customEvent.detail);
		});

		const navigations: string[] = [];
		const actions = buildDynamicActions('triage top 7', makeContext('triage top 7'));
		const dynamicAction = actions.find((action) => action.id === 'ai-triage-inbox-top-7');
		expect(dynamicAction).toBeDefined();
		await dynamicAction?.handler({
			...makeContext('triage top 7'),
			navigate: async (path: string) => {
				navigations.push(path);
			}
		});

		expect(navigations).toEqual(['/inbox']);
		expect(applyEventDetails).toEqual([{ limit: 7 }]);
	});
});
