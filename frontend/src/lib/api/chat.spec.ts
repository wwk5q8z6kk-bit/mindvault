import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { markApiSuccess } from '$lib/api/client';
import {
	__resetNativeChatProbeForTests,
	chat,
	ragChat
} from './chat';

describe('chat api client', () => {
	const fetchMock = vi.fn();

	beforeEach(() => {
		vi.stubGlobal('fetch', fetchMock as unknown as typeof fetch);
		markApiSuccess('/test-init');
		__resetNativeChatProbeForTests();
	});

	afterEach(() => {
		vi.unstubAllGlobals();
		fetchMock.mockReset();
	});

	it('uses native path when /api/v1/chat is available', async () => {
		const stages: string[] = [];
		fetchMock.mockResolvedValueOnce(
			new Response(
				JSON.stringify({
					answer: 'Ship hybrid search defaults [1].',
					sources: [
						{
							node_id: 'n1',
							title: 'Launch plan',
							kind: 'fact',
							score: 0.91,
							preview: 'Ship hybrid search defaults.'
						}
					],
					provider: 'native-rag-heuristic',
					mode: 'native',
					grounded: true
				}),
				{ status: 200, headers: { 'Content-Type': 'application/json' } }
			)
		);

		const result = await chat('What should we ship?', [], (stage) => stages.push(stage));
		expect(result.mode).toBe('native');
		expect(result.grounded).toBe(true);
		expect(result.provider).toBe('native-rag-heuristic');
		expect(result.sources[0]?.title).toBe('Launch plan');
		expect(result.answer).toContain('[1]');
		expect(stages).toContain('retrieving');
		expect(fetchMock).toHaveBeenCalledTimes(1);
		expect(String(fetchMock.mock.calls[0]?.[0] ?? '')).toContain('/api/v1/chat');
	});

	it('uses rag path when native chat is missing', async () => {
		const stages: string[] = [];
		fetchMock
			// native /api/v1/chat
			.mockResolvedValueOnce(new Response('not found', { status: 404 }))
			// recall
			.mockResolvedValueOnce(
				new Response(
					JSON.stringify([
						{
							score: 0.9,
							node: {
								id: 'n1',
								title: 'Launch plan',
								kind: 'fact',
								content: 'Ship hybrid search defaults.',
								tags: [],
								namespace: 'default',
								temporal: { created_at: '', updated_at: '' }
							}
						}
					]),
					{ status: 200, headers: { 'Content-Type': 'application/json' } }
				)
			)
			// assist transform
			.mockResolvedValueOnce(
				new Response(
					JSON.stringify({ transformed_text: 'Ship the defaults [1].' }),
					{ status: 200, headers: { 'Content-Type': 'application/json' } }
				)
			);

		const result = await chat('What should we ship?', [], (stage) => stages.push(stage));
		expect(result.mode).toBe('rag');
		expect(result.grounded).toBe(true);
		expect(result.sources[0]?.title).toBe('Launch plan');
		expect(result.answer).toContain('[1]');
		expect(stages).toContain('retrieving');
		expect(stages).toContain('generating');
	});

	it('returns an ungrounded empty-vault answer without calling transform', async () => {
		fetchMock.mockResolvedValueOnce(
			new Response(JSON.stringify([]), {
				status: 200,
				headers: { 'Content-Type': 'application/json' }
			})
		);

		const result = await ragChat('obscure topic');
		expect(result.grounded).toBe(false);
		expect(result.sources).toEqual([]);
		expect(result.answer).toMatch(/could not find relevant/i);
		expect(fetchMock).toHaveBeenCalledTimes(1);
	});

	it('does not fall back when the backend is offline', async () => {
		fetchMock.mockRejectedValueOnce(new TypeError('Failed to fetch'));
		await expect(chat('hello')).rejects.toThrow(/Network unavailable|Failed to fetch|unavailable/i);
		expect(fetchMock).toHaveBeenCalledTimes(1);
	});
});
