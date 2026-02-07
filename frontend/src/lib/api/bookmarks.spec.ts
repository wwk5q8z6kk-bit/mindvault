import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
	createBookmarkNote,
	createBookmark,
	listBookmarks,
	normalizeBookmarkUrl,
	setBookmarkRead
} from './bookmarks';

const API_BASE = 'http://127.0.0.1:9470';

describe('bookmarks api client', () => {
	const fetchMock = vi.fn();

	beforeEach(() => {
		vi.stubGlobal('fetch', fetchMock as unknown as typeof fetch);
	});

	afterEach(() => {
		vi.unstubAllGlobals();
		fetchMock.mockReset();
	});

	it('normalizes URLs by removing hash and trailing slash', () => {
		expect(normalizeBookmarkUrl('https://Example.com/path/#frag')).toBe(
			'https://example.com/path'
		);
		expect(normalizeBookmarkUrl('example.com/guide/')).toBe('https://example.com/guide');
	});

	it('lists reference + bookmark nodes while filtering saved searches', async () => {
		fetchMock
			.mockResolvedValueOnce(
				new Response(
					JSON.stringify([
						{
							id: 'ref-1',
							kind: 'reference',
							title: 'Article',
							content: 'https://example.com/a',
							source: null,
							namespace: 'default',
							tags: ['web-clip'],
							importance: 0.5,
							temporal: { created_at: '2026-02-06T10:00:00Z', updated_at: '2026-02-06T10:00:00Z' },
							metadata: {}
						}
					]),
					{ status: 200, headers: { 'Content-Type': 'application/json' } }
				)
			)
			.mockResolvedValueOnce(
				new Response(
					JSON.stringify([
						{
							id: 'bookmark-1',
							kind: 'bookmark',
							title: 'Web Clip',
							content: 'excerpt',
							source: 'https://example.com/b',
							namespace: 'default',
							tags: ['web-clip'],
							importance: 0.5,
							temporal: { created_at: '2026-02-06T11:00:00Z', updated_at: '2026-02-06T11:00:00Z' },
							metadata: { read: false }
						},
						{
							id: 'saved-search-1',
							kind: 'bookmark',
							title: 'Saved Search',
							content: 'query text',
							source: null,
							namespace: 'default',
							tags: ['saved-search'],
							importance: 0.5,
							temporal: { created_at: '2026-02-06T12:00:00Z', updated_at: '2026-02-06T12:00:00Z' },
							metadata: { saved_search_query: 'x' }
						}
					]),
					{ status: 200, headers: { 'Content-Type': 'application/json' } }
				)
			);

		const bookmarks = await listBookmarks(50, 'default');
		expect(bookmarks).toHaveLength(2);
		expect(bookmarks[0].id).toBe('bookmark-1');
		expect(bookmarks[1].id).toBe('ref-1');
	});

	it('creates clip via clip-import endpoint with dedupe enabled', async () => {
		fetchMock.mockResolvedValueOnce(
			new Response(
				JSON.stringify({
					created: false,
					bookmark: {
						id: 'bookmark-1',
						kind: 'bookmark',
						title: 'Existing',
						content: '',
						source: 'https://example.com/path',
						namespace: 'default',
						tags: ['web-clip'],
						importance: 0.5,
						temporal: { created_at: '2026-02-06T11:00:00Z', updated_at: '2026-02-06T11:00:00Z' },
						metadata: {}
					}
				}),
				{ status: 200, headers: { 'Content-Type': 'application/json' } }
			)
		);

		const result = await createBookmark({
			url: 'https://example.com/path#top',
			title: 'Dup',
			namespace: 'default'
		});

		expect(result.created).toBe(false);
		expect(result.bookmark.id).toBe('bookmark-1');
		expect(fetchMock).toHaveBeenCalledTimes(1);
		const [url, options] = fetchMock.mock.calls[0] as [string, RequestInit];
		expect(url).toBe(`${API_BASE}/api/v1/clips/import`);
		expect(options.method).toBe('POST');
		expect(JSON.parse(String(options.body))).toMatchObject({
			url: 'https://example.com/path',
			title: 'Dup',
			namespace: 'default',
			dedupe: true
		});
	});

	it('updates bookmark read status with merged metadata', async () => {
		fetchMock
			.mockResolvedValueOnce(
				new Response(
					JSON.stringify({
						id: 'bookmark-1',
						kind: 'bookmark',
						title: 'Item',
						content: '',
						source: 'https://example.com',
						namespace: 'default',
						tags: ['web-clip'],
						importance: 0.5,
						temporal: { created_at: '2026-02-06T10:00:00Z', updated_at: '2026-02-06T10:00:00Z' },
						metadata: { foo: 'bar' }
					}),
					{ status: 200, headers: { 'Content-Type': 'application/json' } }
				)
			)
			.mockResolvedValueOnce(
				new Response(
					JSON.stringify({
						id: 'bookmark-1',
						kind: 'bookmark',
						title: 'Item',
						content: '',
						source: 'https://example.com',
						namespace: 'default',
						tags: ['web-clip'],
						importance: 0.5,
						temporal: { created_at: '2026-02-06T10:00:00Z', updated_at: '2026-02-06T11:00:00Z' },
						metadata: { foo: 'bar', read: true }
					}),
					{ status: 200, headers: { 'Content-Type': 'application/json' } }
				)
			);

		const bookmark = await setBookmarkRead('bookmark-1', true);
		expect(bookmark.read).toBe(true);

		const [putUrl, putOptions] = fetchMock.mock.calls[1] as [string, RequestInit];
		expect(putUrl).toBe(`${API_BASE}/api/v1/nodes/bookmark-1`);
		expect(putOptions.method).toBe('PUT');
		expect(JSON.parse(String(putOptions.body)).metadata).toEqual({
			foo: 'bar',
			read: true
		});
	});

	it('creates a linked note for existing bookmark via dedicated endpoint', async () => {
		fetchMock.mockResolvedValueOnce(
			new Response(
				JSON.stringify({
					created: true,
					note: {
						id: 'note-1',
						kind: 'fact',
						title: 'Clip Note: Existing',
						content: 'body',
						source: 'clip-import',
						namespace: 'default',
						tags: ['clip-note', 'web-clip'],
						importance: 0.5,
						temporal: { created_at: '2026-02-06T11:00:00Z', updated_at: '2026-02-06T11:00:00Z' },
						metadata: { clip_bookmark_id: 'bookmark-1' }
					}
				}),
				{ status: 201, headers: { 'Content-Type': 'application/json' } }
			)
		);

		const result = await createBookmarkNote(
			'bookmark-1',
			{
				excerpt: 'capture',
				tags: ['Research', 'web clip'],
				namespace: 'default'
			},
			{ dedupe: true }
		);

		expect(result.created).toBe(true);
		expect(result.note.id).toBe('note-1');
		const [url, options] = fetchMock.mock.calls[0] as [string, RequestInit];
		expect(url).toBe(`${API_BASE}/api/v1/clips/bookmark-1/note`);
		expect(options.method).toBe('POST');
		expect(JSON.parse(String(options.body))).toMatchObject({
			excerpt: 'capture',
			tags: ['research', 'webclip'],
			namespace: 'default',
			dedupe: true
		});
	});
});
