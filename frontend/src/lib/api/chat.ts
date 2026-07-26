import { ApiError, fetchJson } from './client';
import type { SearchResultDto } from './types';

export interface ChatMessage {
	id: string;
	role: 'user' | 'assistant';
	content: string;
	sources?: ChatSource[];
	timestamp: string;
	/** True when the assistant turn failed and contains an error explanation. */
	isError?: boolean;
	/** Retrieval/generation mode used for this answer. */
	mode?: ChatMode;
}

export interface ChatSource {
	node_id: string;
	title: string;
	kind: string;
	score: number;
	preview: string;
}

export type ChatMode = 'native' | 'rag';
export type ChatProgressStage = 'retrieving' | 'generating';

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
	mode: ChatMode;
	grounded: boolean;
}

export type ChatProgressHandler = (stage: ChatProgressStage) => void;

/** Cached probe: whether native /api/v1/chat is available on this server. */
let nativeChatAvailable: boolean | null = null;

function toSources(results: SearchResultDto[]): ChatSource[] {
	return results.map((r) => ({
		node_id: r.node.id,
		title: r.node.title || 'Untitled',
		kind: r.node.kind,
		score: r.score,
		preview: (r.node.content ?? '').slice(0, 200)
	}));
}

/**
 * Send a chat message that uses native RAG if the server exposes /api/v1/chat.
 */
export async function sendChatMessage(request: ChatRequest): Promise<ChatResponse> {
	const raw = await fetchJson<{
		answer: string;
		sources?: ChatSource[];
		provider?: string;
		mode?: ChatMode;
		grounded?: boolean;
	}>('/api/v1/chat', {
		method: 'POST',
		body: JSON.stringify(request)
	});
	const sources = raw.sources ?? [];
	return {
		answer: raw.answer,
		sources,
		provider: raw.provider,
		mode: raw.mode === 'rag' ? 'rag' : 'native',
		grounded: typeof raw.grounded === 'boolean' ? raw.grounded : sources.length > 0
	};
}

/**
 * Primary grounded chat path: hybrid recall + assist transform synthesis.
 */
export async function ragChat(
	question: string,
	history: Array<{ role: 'user' | 'assistant'; content: string }> = [],
	onProgress?: ChatProgressHandler
): Promise<ChatResponse> {
	onProgress?.('retrieving');

	const results = await fetchJson<SearchResultDto[]>('/api/v1/recall', {
		method: 'POST',
		body: JSON.stringify({ text: question, strategy: 'hybrid', limit: 8 })
	});

	const sources = toSources(results);

	if (sources.length === 0) {
		return {
			answer:
				'I could not find relevant notes or tasks in your vault for that question. Try different keywords, or capture related material first.',
			sources: [],
			mode: 'rag',
			grounded: false,
			provider: 'retrieval-empty'
		};
	}

	onProgress?.('generating');

	const context = results
		.map((r, i) => `[${i + 1}] ${r.node.title}
${(r.node.content ?? '').slice(0, 500)}`)
		.join('\n\n');

	const historyText = history
		.slice(-6)
		.map((m) => `${m.role}: ${m.content}`)
		.join('\n');

	const prompt = [
		'Answer the question using ONLY the context below. Cite sources using [1], [2], etc.',
		'If the context does not contain relevant information, say so clearly.',
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
		sources,
		mode: 'rag',
		grounded: true,
		provider: 'recall+assist'
	};
}

/** @deprecated Use ragChat — kept for callers that still import the old name. */
export async function ragFallback(
	question: string,
	history: Array<{ role: 'user' | 'assistant'; content: string }> = []
): Promise<ChatResponse> {
	return ragChat(question, history);
}

/**
 * Prefer native /api/v1/chat when available; otherwise use grounded recall+assist.
 * Avoids double network failures when the backend is offline.
 */
export async function chat(
	message: string,
	history: Array<{ role: 'user' | 'assistant'; content: string }> = [],
	onProgress?: ChatProgressHandler
): Promise<ChatResponse> {
	if (nativeChatAvailable !== false) {
		try {
			onProgress?.('retrieving');
			const response = await sendChatMessage({
				message,
				history,
				limit: 8,
				strategy: 'hybrid'
			});
			nativeChatAvailable = true;
			onProgress?.('generating');
			return response;
		} catch (error) {
			if (error instanceof ApiError) {
				// Missing endpoint on current MindVault builds — fall through once.
				if (error.status === 404 || error.status === 405) {
					nativeChatAvailable = false;
				} else if (error.status === 0) {
					// Offline/timeout: do not burn a second request path.
					throw error;
				} else {
					// Unexpected server error on native chat — try RAG path.
					nativeChatAvailable = null;
				}
			}
		}
	}

	return ragChat(message, history, onProgress);
}

/** Test seam: reset native endpoint probe cache. */
export function __resetNativeChatProbeForTests(): void {
	nativeChatAvailable = null;
}
