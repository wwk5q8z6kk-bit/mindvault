import { fetchJson } from './client';

export type ClipEnrichPayload = {
	url: string;
	html?: string;
};

export type ClipEnrichResult = {
	normalized_url: string;
	title?: string;
	description?: string;
	site_name?: string;
	content_preview?: string;
	estimated_reading_minutes?: number;
	suggested_tags: string[];
	fetched: boolean;
	content_type?: string;
};

export async function enrichClip(payload: ClipEnrichPayload): Promise<ClipEnrichResult> {
	const url = String(payload.url || '').trim();
	if (!url) throw new Error('URL is required');
	const html = typeof payload.html === 'string' ? payload.html.trim() : '';

	return await fetchJson<ClipEnrichResult>('/api/v1/clips/enrich', {
		method: 'POST',
		body: JSON.stringify({
			url,
			html: html || undefined
		})
	});
}
