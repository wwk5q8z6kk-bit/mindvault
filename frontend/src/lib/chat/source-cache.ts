import { STORAGE_KEYS } from '$lib/constants/storage-keys';
import type { ChatSource } from '$lib/api/chat';

type SourceMap = Record<string, ChatSource[]>;

function readMap(): SourceMap {
	if (typeof localStorage === 'undefined') return {};
	try {
		const raw = localStorage.getItem(STORAGE_KEYS.CHAT_SOURCES);
		if (!raw) return {};
		const parsed = JSON.parse(raw) as SourceMap;
		return parsed && typeof parsed === 'object' ? parsed : {};
	} catch {
		return {};
	}
}

function writeMap(map: SourceMap): void {
	if (typeof localStorage === 'undefined') return;
	try {
		localStorage.setItem(STORAGE_KEYS.CHAT_SOURCES, JSON.stringify(map));
	} catch {
		// ignore quota
	}
}

export function cacheChatSources(messageId: string, sources: ChatSource[]): void {
	if (!messageId || sources.length === 0) return;
	const map = readMap();
	map[messageId] = sources;
	// Cap cache size opportunistically.
	const keys = Object.keys(map);
	if (keys.length > 400) {
		for (const key of keys.slice(0, keys.length - 400)) {
			delete map[key];
		}
	}
	writeMap(map);
}

export function readCachedChatSources(messageId: string): ChatSource[] | undefined {
	if (!messageId) return undefined;
	const sources = readMap()[messageId];
	return Array.isArray(sources) ? sources : undefined;
}
