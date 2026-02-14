import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
	getTemplateVersion,
	listTemplateVersions,
	readTemplateKey,
	readTemplateVariables,
	restoreTemplateVersion
} from './templates';
import type { KnowledgeNode } from './types';
import { markApiSuccess } from '$lib/api/client';

const API_BASE = 'http://127.0.0.1:9470';

describe('templates api client', () => {
	const fetchMock = vi.fn();

	beforeEach(() => {
		vi.stubGlobal('fetch', fetchMock as unknown as typeof fetch);
		markApiSuccess('/test-init');
	});

	afterEach(() => {
		vi.unstubAllGlobals();
		fetchMock.mockReset();
	});

	it('lists template versions', async () => {
		fetchMock.mockResolvedValueOnce(
			new Response(JSON.stringify([{ version_id: 'v1' }]), {
				status: 200,
				headers: { 'Content-Type': 'application/json' }
			})
		);
		const versions = await listTemplateVersions('tpl-1');
		expect(versions).toEqual([{ version_id: 'v1' }]);
		expect(fetchMock.mock.calls[0][0]).toBe(`${API_BASE}/api/v1/templates/tpl-1/versions`);
	});

	it('gets version detail and restores version', async () => {
		fetchMock
			.mockResolvedValueOnce(
				new Response(JSON.stringify({ version: { version_id: 'v1' }, current: {}, diff: {}, field_changes: [] }), {
					status: 200,
					headers: { 'Content-Type': 'application/json' }
				})
			)
			.mockResolvedValueOnce(
				new Response(
					JSON.stringify({
						id: 'node-1',
						kind: 'fact',
						title: 'Template',
						content: 'Hello',
						tags: [],
						importance: 0.5,
						temporal: { created_at: '2026-02-06T00:00:00Z', updated_at: '2026-02-06T00:00:00Z' },
						metadata: {}
					}),
					{ status: 200, headers: { 'Content-Type': 'application/json' } }
				)
			);

		const detail = await getTemplateVersion('tpl-1', 'v1');
		expect(detail.version.version_id).toBe('v1');

		const restored = await restoreTemplateVersion('tpl-1', 'v1');
		expect(restored.id).toBe('node-1');
		const [url, options] = fetchMock.mock.calls[1] as [string, RequestInit];
		expect(url).toBe(`${API_BASE}/api/v1/templates/tpl-1/versions/v1/restore`);
		expect(options.method).toBe('POST');
	});

	it('reads template metadata helpers safely', () => {
		const node: KnowledgeNode = {
			id: 'node-1',
			kind: 'fact' as any,
			title: 'Template',
			content: 'Hello',
			tags: [],
			importance: 0.5,
			temporal: { created_at: '2026-02-06T00:00:00Z', updated_at: '2026-02-06T00:00:00Z' },
			metadata: {
				template_key: 'plan.weekly',
				template_variables: ['week_label', 'focus_area', 42]
			}
		};
		expect(readTemplateKey(node)).toBe('plan.weekly');
		expect(readTemplateVariables(node)).toEqual(['week_label', 'focus_area']);

		const invalidNode: KnowledgeNode = {
			...node,
			metadata: { template_key: '  ', template_variables: 'invalid' as any }
		};
		expect(readTemplateKey(invalidNode)).toBeNull();
		expect(readTemplateVariables(invalidNode)).toEqual([]);
	});
});
