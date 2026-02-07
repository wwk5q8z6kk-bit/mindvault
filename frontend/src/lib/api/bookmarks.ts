import { fetchJson } from './client';
import { deleteNode, listNodes } from './nodes';
import type { KnowledgeNode } from './types';

export type Bookmark = {
	id: string;
	title: string;
	url: string;
	tags: string[];
	created_at: string;
	updated_at: string;
	read: boolean;
	excerpt: string;
	kind: 'reference' | 'bookmark';
	metadata: Record<string, unknown>;
};

export type CreateBookmarkPayload = {
	url: string;
	title?: string;
	excerpt?: string;
	tags?: string[];
	namespace?: string | null;
	clip_source?: string;
	create_note?: boolean;
};

export type CreateBookmarkResult = {
	bookmark: Bookmark;
	created: boolean;
	note?: KnowledgeNode | null;
};

type ClipImportResponse = {
	bookmark: KnowledgeNode;
	created: boolean;
	note?: KnowledgeNode | null;
};

function normalizeTagList(tags: string[]): string[] {
	const seen = new Set<string>();
	for (const tag of tags) {
		const normalized = String(tag || '')
			.trim()
			.toLowerCase()
			.replace(/[^a-z0-9:_-]/g, '');
		if (normalized) seen.add(normalized);
	}
	return [...seen];
}

export function normalizeBookmarkUrl(raw: string): string {
	const input = raw.trim();
	if (!input) return '';
	const withProtocol = /^https?:\/\//i.test(input) ? input : `https://${input}`;
	try {
		const parsed = new URL(withProtocol);
		parsed.hash = '';
		if (parsed.pathname.length > 1 && parsed.pathname.endsWith('/')) {
			parsed.pathname = parsed.pathname.slice(0, -1);
		}
		parsed.hostname = parsed.hostname.toLowerCase();
		return parsed.toString();
	} catch {
		return withProtocol;
	}
}

function looksLikeSavedSearchNode(node: KnowledgeNode): boolean {
	const tags = node.tags.map((tag) => tag.toLowerCase());
	if (tags.includes('saved-search') || tags.includes('saved_search')) return true;
	return Object.keys(node.metadata ?? {}).some((key) => key.toLowerCase().startsWith('saved_search_'));
}

function extractUrl(node: KnowledgeNode): string {
	const source = typeof node.source === 'string' ? node.source.trim() : '';
	if (source) return source;
	const content = typeof node.content === 'string' ? node.content.trim() : '';
	if (!content) return '';
	const match = content.match(/https?:\/\/\S+/i);
	return match ? match[0] : '';
}

function nodeToBookmark(node: KnowledgeNode): Bookmark | null {
	const url = extractUrl(node);
	if (!url) return null;
	if (node.kind !== 'reference' && node.kind !== 'bookmark') return null;
	if (looksLikeSavedSearchNode(node)) return null;
	return {
		id: node.id,
		title: node.title || 'Untitled',
		url,
		tags: node.tags ?? [],
		created_at: node.temporal.created_at,
		updated_at: node.temporal.updated_at,
		read: node.metadata?.read === true,
		excerpt: typeof node.content === 'string' ? node.content : '',
		kind: node.kind,
		metadata: node.metadata
	};
}

export async function listBookmarks(limit = 300, namespace?: string | null): Promise<Bookmark[]> {
	const [referenceNodes, bookmarkNodes] = await Promise.all([
		listNodes({ kind: 'reference', limit, namespace: namespace ?? undefined }),
		listNodes({ kind: 'bookmark', limit, namespace: namespace ?? undefined })
	]);

	const mapped = [...referenceNodes, ...bookmarkNodes]
		.map(nodeToBookmark)
		.filter((item): item is Bookmark => Boolean(item));

	const dedupedById = new Map(mapped.map((item) => [item.id, item]));
	return [...dedupedById.values()].sort(
		(a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime()
	);
}

export async function findBookmarkByNormalizedUrl(
	url: string,
	namespace?: string | null
): Promise<Bookmark | null> {
	const normalized = normalizeBookmarkUrl(url);
	if (!normalized) return null;
	const bookmarks = await listBookmarks(400, namespace);
	return (
		bookmarks.find((bookmark) => normalizeBookmarkUrl(bookmark.url) === normalized) ?? null
	);
}

export async function createBookmark(
	payload: CreateBookmarkPayload,
	{ dedupe = true }: { dedupe?: boolean } = {}
): Promise<CreateBookmarkResult> {
	const normalizedUrl = normalizeBookmarkUrl(payload.url);
	if (!normalizedUrl) {
		throw new Error('Invalid URL');
	}

	const response = await fetchJson<ClipImportResponse>('/api/v1/clips/import', {
		method: 'POST',
		body: JSON.stringify({
			url: normalizedUrl,
			title: payload.title?.trim() || undefined,
			excerpt: payload.excerpt?.trim() || undefined,
			tags: normalizeTagList(payload.tags ?? []),
			namespace: payload.namespace ?? undefined,
			clip_source: payload.clip_source ?? 'manual',
			dedupe,
			create_note: payload.create_note ?? false
		})
	});
	const bookmark = nodeToBookmark(response.bookmark);
	if (!bookmark) {
		throw new Error('Bookmark creation returned invalid payload');
	}
	return { bookmark, created: response.created, note: response.note };
}

export async function setBookmarkRead(bookmarkId: string, read: boolean): Promise<Bookmark> {
	const existing = await fetchJson<KnowledgeNode>(`/api/v1/nodes/${bookmarkId}`);
	const updated = await fetchJson<KnowledgeNode>(`/api/v1/nodes/${bookmarkId}`, {
		method: 'PUT',
		body: JSON.stringify({
			...existing,
			metadata: {
				...(existing.metadata ?? {}),
				read
			}
		})
	});
	const bookmark = nodeToBookmark(updated);
	if (!bookmark) {
		throw new Error('Updated bookmark payload was invalid');
	}
	return bookmark;
}

export async function deleteBookmark(bookmarkId: string): Promise<void> {
	await deleteNode(bookmarkId);
}
