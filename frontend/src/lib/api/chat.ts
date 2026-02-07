import { fetchJson, API_BASE_URL } from './client';
import type { SearchResultDto } from './types';

export interface ChatMessage {
	id: string;
	role: 'user' | 'assistant';
	content: string;
	sources?: ChatSource[];
	timestamp: string;
}

export interface ChatSource {
	node_id: string;
	title: string;
	kind: string;
	score: number;
	preview: string;
}

export interface ChatRequest {
	message: string;
	history?: Array<{ role: 'user' | 'assistant'; content: string }>;
	limit?: number;
	strategy?: 'hybrid' | 'vector' | 'fulltext';
}

export interface ChatResponse {
	answer: string;
	sources: ChatSource[];
	provider?: string;
}

/**
 * Send a chat message that uses RAG (retrieve + generate).
 * Calls POST /api/v1/chat which retrieves relevant nodes and generates an answer.
 */
export async function sendChatMessage(request: ChatRequest): Promise<ChatResponse> {
	return await fetchJson<ChatResponse>('/api/v1/chat', {
		method: 'POST',
		body: JSON.stringify(request)
	});
}

/**
 * Fallback: manually do RAG by recalling nodes then asking for a synthesis.
 * Used if /api/v1/chat endpoint doesn't exist yet.
 */
export async function ragFallback(
	question: string,
	history: Array<{ role: 'user' | 'assistant'; content: string }> = []
): Promise<ChatResponse> {
	// Step 1: Retrieve relevant nodes
	const results = await fetchJson<SearchResultDto[]>('/api/v1/recall', {
		method: 'POST',
		body: JSON.stringify({ text: question, strategy: 'hybrid', limit: 8 })
	});

	const sources: ChatSource[] = results.map((r) => ({
		node_id: r.node.id,
		title: r.node.title,
		kind: r.node.kind,
		score: r.score,
		preview: (r.node.content ?? '').slice(0, 200)
	}));

	// Step 2: Build context and ask LLM to synthesize
	const context = results
		.map((r, i) => `[${i + 1}] ${r.node.title}\n${(r.node.content ?? '').slice(0, 500)}`)
		.join('\n\n');

	const historyText = history
		.slice(-6)
		.map((m) => `${m.role}: ${m.content}`)
		.join('\n');

	const prompt = [
		'Answer the question using ONLY the context below. Cite sources using [1], [2], etc.',
		'If the context does not contain relevant information, say so.',
		'',
		'Context:',
		context,
		'',
		historyText ? `Conversation:\n${historyText}\n` : '',
		`Question: ${question}`
	].join('\n');

	const response = await fetchJson<{ transformed_text: string }>('/api/v1/assist/transform', {
		method: 'POST',
		body: JSON.stringify({ text: prompt, mode: 'refine' })
	});

	return {
		answer: response.transformed_text,
		sources
	};
}

/**
 * Try the dedicated chat endpoint first, fall back to manual RAG.
 */
export async function chat(
	message: string,
	history: Array<{ role: 'user' | 'assistant'; content: string }> = []
): Promise<ChatResponse> {
	try {
		return await sendChatMessage({ message, history, limit: 8, strategy: 'hybrid' });
	} catch {
		return await ragFallback(message, history);
	}
}
