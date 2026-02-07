import { fetchJson } from './client';

export type AssistAutocompleteResponse = {
	completions: string[];
	source_nodes?: number;
	strategy?: string;
};

export type AssistSuggestionSource = {
	node_id: string;
	title: string;
	namespace: string;
	score: number;
};

export type AssistCompletionResponse = {
	suggestions: string[];
	sources?: AssistSuggestionSource[];
	source_nodes?: number;
	strategy?: string;
};

export type AssistTransformResponse = {
	transformed_text: string;
};

export type AssistLinkSuggestion = {
	node_id?: string;
	title: string;
	heading?: string;
	preview?: string;
	namespace?: string;
	score?: number;
	reason?: string;
};

export type AssistLinksResponse = {
	suggestions: AssistLinkSuggestion[];
	source_nodes?: number;
	strategy?: string;
};

export async function assistAutocomplete(payload: {
	text: string;
	limit?: number;
	namespace?: string;
}): Promise<AssistAutocompleteResponse> {
	return await fetchJson<AssistAutocompleteResponse>('/api/v1/assist/autocomplete', {
		method: 'POST',
		body: JSON.stringify(payload)
	});
}

export async function assistCompletion(payload: {
	text: string;
	limit?: number;
	namespace?: string;
}): Promise<AssistCompletionResponse> {
	return await fetchJson<AssistCompletionResponse>('/api/v1/assist/completion', {
		method: 'POST',
		body: JSON.stringify(payload)
	});
}

export async function assistTransform(payload: {
	text: string;
	mode: 'summarize' | 'action_items' | 'refine';
	limit?: number;
	namespace?: string;
}): Promise<AssistTransformResponse> {
	return await fetchJson<AssistTransformResponse>('/api/v1/assist/transform', {
		method: 'POST',
		body: JSON.stringify(payload)
	});
}

export type AssistAutoTagResponse = {
	tags: string[];
};

export async function assistAutoTag(payload: {
	text: string;
	existing_tags?: string[];
	limit?: number;
}): Promise<AssistAutoTagResponse> {
	// Use the transform endpoint to suggest tags
	const existingHint = payload.existing_tags?.length
		? `\nExisting tags in the system: ${payload.existing_tags.join(', ')}`
		: '';
	const prompt = `Suggest 2-4 short, lowercase tags for categorizing this content. Return ONLY a comma-separated list of tags, nothing else.${existingHint}\n\nContent: ${payload.text.slice(0, 500)}`;

	const result = await assistTransform({ text: prompt, mode: 'refine' });
	const tags = result.transformed_text
		.split(',')
		.map((t) => t.trim().toLowerCase().replace(/[^a-z0-9-_]/g, ''))
		.filter((t) => t.length > 0 && t.length < 30)
		.slice(0, payload.limit ?? 4);

	return { tags };
}

export async function assistLinks(payload: {
	text: string;
	limit?: number;
	namespace?: string;
	exclude_node_id?: string;
}): Promise<AssistLinksResponse> {
	return await fetchJson<AssistLinksResponse>('/api/v1/assist/links', {
		method: 'POST',
		body: JSON.stringify(payload)
	});
}
