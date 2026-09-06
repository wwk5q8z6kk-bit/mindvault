import type { KnowledgeNode, SearchResultDto } from '$lib/api/types';

const DEFAULT_NAMESPACE = 'default';

export function resultDestination(node: KnowledgeNode): string {
	if (node.kind === 'task') {
		return `/tasks?task=${encodeURIComponent(node.id)}`;
	}

	if (node.kind === 'fact' && node.tags.some((tag) => tag.startsWith('day:'))) {
		return '/daily';
	}

	return `/notes?note=${encodeURIComponent(node.id)}`;
}

export function graphDestination(nodeId: string): string {
	return `/notes?view=graph&node=${encodeURIComponent(nodeId)}`;
}

export function matchSourceLabel(matchSource: SearchResultDto['match_source']): string {
	switch (matchSource) {
		case 'hybrid':
			return 'Hybrid match';
		case 'vector':
			return 'Semantic match';
		case 'full_text':
		case 'fulltext':
			return 'Keyword match';
		case 'graph':
			return 'Related match';
		default:
			return 'Search match';
	}
}

export function namespaceLabel(namespace: string | null | undefined): string {
	if (!namespace || namespace === DEFAULT_NAMESPACE) {
		return 'Personal Vault';
	}

	return namespace;
}

export function sourceLabel(source: string | null | undefined): string | null {
	const trimmed = source?.trim();
	if (!trimmed) return null;

	if (trimmed.startsWith('builtin:')) return 'MindVault';
	if (trimmed.startsWith('assistant:')) return 'Assistant';
	if (trimmed === 'clip-import' || trimmed === 'extension') return 'Web clip';
	if (trimmed === 'manual') return 'Manual entry';

	try {
		const url = new URL(trimmed);
		return url.hostname.replace(/^www\./, '');
	} catch {
		// File and connector sources are intentionally reduced to a readable label.
	}

	const segments = trimmed.split(/[\\/]/).filter(Boolean);
	return segments.at(-1) ?? trimmed;
}

export function relativeUpdatedLabel(updatedAt: string, now = Date.now()): string {
	const timestamp = Date.parse(updatedAt);
	if (!Number.isFinite(timestamp)) return 'Updated recently';

	const elapsed = Math.max(0, now - timestamp);
	const minutes = Math.floor(elapsed / 60_000);
	if (minutes < 1) return 'Updated just now';
	if (minutes < 60) return `Updated ${minutes}m ago`;

	const hours = Math.floor(minutes / 60);
	if (hours < 24) return `Updated ${hours}h ago`;

	const days = Math.floor(hours / 24);
	if (days < 30) return `Updated ${days}d ago`;

	return `Updated ${new Intl.DateTimeFormat(undefined, {
		month: 'short',
		day: 'numeric',
		year: new Date(timestamp).getFullYear() === new Date(now).getFullYear() ? undefined : 'numeric'
	}).format(new Date(timestamp))}`;
}
