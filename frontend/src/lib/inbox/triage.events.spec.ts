// @vitest-environment jsdom

import { describe, expect, it } from 'vitest';
import {
	dispatchInboxTriage,
	dispatchInboxTriageApplyTop,
	INBOX_TRIAGE_APPLY_TOP_EVENT_NAME,
	INBOX_TRIAGE_EVENT_NAME
} from './triage';

describe('inbox triage events', () => {
	it('dispatches triage event', () => {
		const received: Event[] = [];
		window.addEventListener(INBOX_TRIAGE_EVENT_NAME, (event) => {
			received.push(event);
		});
		expect(dispatchInboxTriage()).toBe(true);
		expect(received).toHaveLength(1);
	});

	it('dispatches apply-top triage event with normalized detail', () => {
		const details: Array<{ limit?: number } | undefined> = [];
		window.addEventListener(INBOX_TRIAGE_APPLY_TOP_EVENT_NAME, (event) => {
			const customEvent = event as CustomEvent<{ limit?: number }>;
			details.push(customEvent.detail);
		});
		expect(dispatchInboxTriageApplyTop(2.6)).toBe(true);
		expect(details).toEqual([{ limit: 3 }]);
	});
});
