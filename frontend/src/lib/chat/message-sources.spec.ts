// @vitest-environment jsdom
import { describe, expect, it } from 'vitest';
import { resolveMessageSources } from './message-sources';

describe('resolveMessageSources', () => {
	const server = [
		{ node_id: 'n1', title: 'Server', kind: 'fact', score: 0.9, preview: 'from server' }
	];
	const cached = [
		{ node_id: 'n2', title: 'Cached', kind: 'fact', score: 0.8, preview: 'from cache' }
	];

	it('prefers server sources over cache', () => {
		expect(resolveMessageSources(server, cached)?.[0]?.title).toBe('Server');
	});

	it('falls back to cache when server is empty', () => {
		expect(resolveMessageSources([], cached)?.[0]?.title).toBe('Cached');
		expect(resolveMessageSources(undefined, cached)?.[0]?.title).toBe('Cached');
	});

	it('returns undefined when neither has sources', () => {
		expect(resolveMessageSources(undefined, undefined)).toBeUndefined();
		expect(resolveMessageSources([], [])).toBeUndefined();
	});
});
