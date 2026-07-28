// @vitest-environment jsdom
import { describe, expect, it } from 'vitest';
import { ApiError } from '$lib/api/client';
import { describeChatFailure } from './errors';

describe('describeChatFailure', () => {
	it('explains offline failures', () => {
		expect(describeChatFailure(new ApiError('Network unavailable', 0))).toMatch(/offline/i);
	});

	it('explains timeouts', () => {
		expect(describeChatFailure(new ApiError('Request timed out (10000ms)', 0))).toMatch(/timed out/i);
	});

	it('explains auth and rate limits', () => {
		expect(describeChatFailure(new ApiError('nope', 401))).toMatch(/authorized/i);
		expect(describeChatFailure(new ApiError('slow down', 429))).toMatch(/Rate limit/i);
	});
});
