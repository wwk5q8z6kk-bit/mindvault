import { ApiError } from '$lib/api/client';

export function describeChatFailure(error: unknown): string {
	if (error instanceof ApiError) {
		if (error.status === 0) {
			if (/timed out/i.test(error.message)) {
				return 'The request timed out while searching your vault. Try a shorter question.';
			}
			return 'MindVault is offline. Reconnect to search your notes and generate an answer.';
		}
		if (error.status === 401 || error.status === 403) {
			return 'You are not authorized for AI chat. Check your auth token or role.';
		}
		if (error.status === 404) {
			return 'Chat retrieval endpoint is unavailable on this server.';
		}
		if (error.status === 429) {
			return 'Rate limit reached. Wait a moment and try again.';
		}
		if (error.status >= 500) {
			return 'The AI service returned a server error. Your vault search may still work — try again shortly.';
		}
		return error.message || 'Chat request failed.';
	}
	if (error instanceof Error && error.message) {
		return error.message;
	}
	return 'Unable to answer right now. Please try again.';
}
