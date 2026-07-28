import type { ChatSource } from '$lib/api/chat';

/** Prefer server-persisted sources; fall back to local cache for older turns. */
export function resolveMessageSources(
	serverSources: ChatSource[] | undefined,
	cachedSources: ChatSource[] | undefined
): ChatSource[] | undefined {
	if (Array.isArray(serverSources) && serverSources.length > 0) {
		return serverSources;
	}
	if (Array.isArray(cachedSources) && cachedSources.length > 0) {
		return cachedSources;
	}
	return undefined;
}
