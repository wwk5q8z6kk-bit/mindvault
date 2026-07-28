import { fetchJson } from './client';
import type { ChatSource } from './chat';

export interface ConversationListItem {
	id: string;
	title?: string | null;
	updated_at: string;
}

export interface ConversationMessage {
	id: string;
	role: 'user' | 'assistant' | string;
	content: string;
	created_at: string;
	sources?: ChatSource[];
}

export async function listConversations(limit = 50, offset = 0): Promise<ConversationListItem[]> {
	const params = new URLSearchParams({
		limit: String(limit),
		offset: String(offset)
	});
	return await fetchJson<ConversationListItem[]>(`/api/v1/conversations?${params.toString()}`);
}

export async function createConversation(title?: string): Promise<{ id: string; title?: string | null }> {
	return await fetchJson<{ id: string; title?: string | null }>('/api/v1/conversations', {
		method: 'POST',
		body: JSON.stringify({ title })
	});
}

export async function deleteConversation(id: string): Promise<void> {
	await fetchJson<void>(`/api/v1/conversations/${encodeURIComponent(id)}`, {
		method: 'DELETE'
	});
}

export async function listConversationMessages(
	id: string,
	limit = 200
): Promise<ConversationMessage[]> {
	const params = new URLSearchParams({
		limit: String(limit)
	});
	return await fetchJson<ConversationMessage[]>(
		`/api/v1/conversations/${encodeURIComponent(id)}/messages?${params.toString()}`
	);
}

export async function appendConversationMessage(
	id: string,
	role: 'user' | 'assistant',
	content: string,
	sources?: ChatSource[]
): Promise<{ id: string; conversation_id: string; role: string; sources?: ChatSource[] }> {
	return await fetchJson<{
		id: string;
		conversation_id: string;
		role: string;
		sources?: ChatSource[];
	}>(`/api/v1/conversations/${encodeURIComponent(id)}/message`, {
		method: 'POST',
		body: JSON.stringify({
			role,
			content,
			...(sources && sources.length > 0 ? { sources } : {})
		})
	});
}
