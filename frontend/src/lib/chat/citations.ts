export type CitationSegment =
	| { type: 'text'; value: string }
	| { type: 'citation'; index: number; raw: string };

const CITATION_RE = /\[(\d+)\]/g;

/** Split answer text into plain text and [n] citation markers. */
export function splitCitationSegments(answer: string): CitationSegment[] {
	if (!answer) return [];
	const segments: CitationSegment[] = [];
	let last = 0;
	for (const match of answer.matchAll(CITATION_RE)) {
		const start = match.index ?? 0;
		if (start > last) {
			segments.push({ type: 'text', value: answer.slice(last, start) });
		}
		segments.push({
			type: 'citation',
			index: Number.parseInt(match[1], 10),
			raw: match[0]
		});
		last = start + match[0].length;
	}
	if (last < answer.length) {
		segments.push({ type: 'text', value: answer.slice(last) });
	}
	return segments;
}

/** Format retrieval scores without inventing fake percentages. */
export function formatRelevanceScore(score: number): string | null {
	if (!Number.isFinite(score)) return null;
	// Normalized similarity scores.
	if (score >= 0.05 && score <= 1) {
		return `${Math.round(score * 100)}%`;
	}
	// Explicit whole-number percents from some backends.
	if (score > 1 && score <= 100 && Number.isInteger(score)) {
		return `${score}%`;
	}
	// Tiny RRF-style fusion scores: keep absolute magnitude.
	if (score > 0 && score < 0.05) {
		return score.toFixed(3);
	}
	return score.toFixed(2);
}

export function sourceHref(kind: string, nodeId: string, title: string): string {
	if (kind === 'task') return `/tasks?task=${nodeId}`;
	if (kind === 'event') return `/tasks?view=calendar`;
	if (kind === 'bookmark' || kind === 'reference') return `/bookmarks?id=${nodeId}`;
	if (kind === 'fact' || kind === 'note' || kind === 'decision' || kind === 'preference') {
		return `/notes?note=${nodeId}`;
	}
	return `/search?q=${encodeURIComponent(title || nodeId)}`;
}
