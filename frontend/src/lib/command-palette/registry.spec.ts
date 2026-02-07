import { get } from 'svelte/store';
import { describe, it, expect, beforeEach } from 'vitest';
import { actionsStore, registerAction, removeAction } from './registry';
import type { CommandAction } from './types';

const sampleAction: CommandAction = {
	id: 'test-action',
	title: 'Test Action',
	handler: () => {}
};

beforeEach(() => {
	actionsStore.set([]);
});

describe('command palette registry', () => {
	it('registers an action', () => {
		registerAction(sampleAction);
		const actions = get(actionsStore);
		expect(actions.find((action) => action.id === 'test-action')).toBeTruthy();
	});

	it('removes an action', () => {
		registerAction(sampleAction);
		removeAction('test-action');
		const actions = get(actionsStore);
		expect(actions.find((action) => action.id === 'test-action')).toBeFalsy();
	});
});
