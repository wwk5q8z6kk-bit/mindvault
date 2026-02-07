// @vitest-environment jsdom

import { beforeEach, describe, expect, it } from 'vitest';
import {
	DEFAULT_INBOX_TRIAGE_SETTINGS,
	INBOX_TRIAGE_SETTINGS_STORAGE_KEY,
	loadInboxTriageSettings,
	normalizeInboxTriageSettings,
	saveInboxTriageSettings
} from './triage-settings';

describe('inbox triage settings', () => {
	beforeEach(() => {
		localStorage.removeItem(INBOX_TRIAGE_SETTINGS_STORAGE_KEY);
	});

	it('returns defaults when settings are missing', () => {
		expect(loadInboxTriageSettings()).toEqual(DEFAULT_INBOX_TRIAGE_SETTINGS);
	});

	it('normalizes invalid values and clamps apply limit', () => {
		const normalized = normalizeInboxTriageSettings({
			auto_run_on_open: true,
			default_apply_limit: 99
		});
		expect(normalized).toEqual({
			auto_run_on_open: true,
			default_apply_limit: 10
		});
	});

	it('saves and reloads settings', () => {
		const saved = saveInboxTriageSettings({
			auto_run_on_open: true,
			default_apply_limit: 4
		});
		expect(saved).toEqual({
			auto_run_on_open: true,
			default_apply_limit: 4
		});
		expect(loadInboxTriageSettings()).toEqual(saved);
	});
});
