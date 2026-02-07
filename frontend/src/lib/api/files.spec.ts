import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ApiError } from './client';
import {
	attachmentDownloadUrl,
	attachmentInlineUrl,
	deleteNodeAttachment,
	getAttachmentChunks,
	listNodeAttachments,
	type AttachmentChunkListResponse,
	type NodeAttachment
} from './files';

const API_BASE = 'http://127.0.0.1:9470';

describe('files api client', () => {
	const fetchMock = vi.fn();

	beforeEach(() => {
		vi.stubGlobal('fetch', fetchMock as unknown as typeof fetch);
	});

	afterEach(() => {
		vi.unstubAllGlobals();
		fetchMock.mockReset();
	});

	it('lists node attachments', async () => {
		const payload: NodeAttachment[] = [
			{
				attachment_id: 'att-1',
				file_name: 'file.pdf',
				size_bytes: 1024,
				download_url: '/api/v1/files/node-1/att-1'
			}
		];
		fetchMock.mockResolvedValueOnce(
			new Response(JSON.stringify(payload), {
				status: 200,
				headers: { 'Content-Type': 'application/json' }
			})
		);

		const result = await listNodeAttachments('node-1');
		expect(result).toEqual(payload);
		expect(fetchMock).toHaveBeenCalledWith(`${API_BASE}/api/v1/files/node-1`);
	});

	it('deletes node attachment', async () => {
		fetchMock.mockResolvedValueOnce(
			new Response(
				JSON.stringify({ attachment_id: 'att-1', file_deleted: true, remaining_attachments: 0 }),
				{ status: 200, headers: { 'Content-Type': 'application/json' } }
			)
		);

		const result = await deleteNodeAttachment('node-1', 'att-1');
		expect(result.file_deleted).toBe(true);
		const [url, options] = fetchMock.mock.calls[0] as [string, RequestInit];
		expect(url).toBe(`${API_BASE}/api/v1/files/node-1/att-1`);
		expect(options.method).toBe('DELETE');
	});

	it('throws ApiError on failed attachment fetch', async () => {
		fetchMock.mockResolvedValueOnce(new Response('nope', { status: 500 }));
		await expect(listNodeAttachments('node-1')).rejects.toBeInstanceOf(ApiError);
	});

	it('builds download URLs with encoding', () => {
		expect(attachmentDownloadUrl('node id', 'file/one')).toBe(
			`${API_BASE}/api/v1/files/node%20id/file%2Fone`
		);
	});

	it('builds inline preview URLs', () => {
		expect(attachmentInlineUrl('node id', 'file/one')).toBe(
			`${API_BASE}/api/v1/files/node%20id/file%2Fone?inline=true`
		);
	});

	it('fetches attachment chunks', async () => {
		const payload: AttachmentChunkListResponse = {
			node_id: 'node-1',
			attachment_id: 'att-1',
			extraction_status: 'indexed_text',
			extracted_chars: 42,
			total_chunks: 2,
			offset: 0,
			limit: 2,
			returned_chunks: 2,
			chunks: [
				{ index: 0, text: 'alpha', char_count: 5 },
				{ index: 1, text: 'beta', char_count: 4 }
			]
		};
		fetchMock.mockResolvedValueOnce(
			new Response(JSON.stringify(payload), {
				status: 200,
				headers: { 'Content-Type': 'application/json' }
			})
		);

		const result = await getAttachmentChunks('node-1', 'att-1', { limit: 2, offset: 0 });
		expect(result).toEqual(payload);
		expect(fetchMock).toHaveBeenCalledWith(
			`${API_BASE}/api/v1/files/node-1/att-1/chunks?limit=2&offset=0`
		);
	});
});
