import type { TaskCreatePayload } from '$lib/api/tasks';

export type ParsedActionItem = {
	title: string;
	priority: number;
	due_at: string | null;
	estimate_min: number | null;
	labels: string[];
};

type ParseOptions = {
	now?: Date;
	maxItems?: number;
};

const LINE_PREFIX_RE = /^\s*(?:[-*]|\d+[.)])\s*(?:\[[ xX]\]\s*)?/;
const HEADING_RE = /^(?:action items?|tasks?|next steps?|todo|to-do)s?:?\s*$/i;
const PLAIN_PRIORITY_RE = /(?:^|\s)(?:p|!)([1-5])(?:\s|$)/i;
const BRACKET_PRIORITY_RE = /\[\s*p(?:riority)?\s*[:=]?\s*([1-5])\s*\]/i;
const WORD_PRIORITY_RE = /\bpriority\s*[:=]?\s*(critical|high|medium|low|minimal)\b/i;
const ESTIMATE_RE = /(?:\best(?:imate)?\s*[:=]?\s*|~\s*)(\d{1,3})\s*(?:m|min|mins|minutes)\b/i;
const DUE_RE = /\bdue\s*[:=]\s*([a-z0-9-_/]+)\b/i;
const TAG_RE = /#([a-z0-9][a-z0-9-_]*)/gi;

const WORD_PRIORITY_MAP: Record<string, number> = {
	critical: 1,
	high: 2,
	medium: 3,
	low: 4,
	minimal: 5
};

function toDateOnly(date: Date): string {
	return date.toISOString().slice(0, 10);
}

function resolveDue(raw: string, now: Date): string | null {
	const token = raw.trim().toLowerCase();
	if (token === 'today') {
		return toDateOnly(now);
	}
	if (token === 'tomorrow') {
		const tomorrow = new Date(now);
		tomorrow.setDate(now.getDate() + 1);
		return toDateOnly(tomorrow);
	}
	if (/^\d{4}-\d{2}-\d{2}$/.test(token)) {
		return token;
	}
	const parsed = new Date(token);
	if (Number.isNaN(parsed.getTime())) {
		return null;
	}
	return toDateOnly(parsed);
}

function parsePriority(line: string): number {
	const bracket = line.match(BRACKET_PRIORITY_RE);
	if (bracket) {
		return Number(bracket[1]);
	}
	const plain = line.match(PLAIN_PRIORITY_RE);
	if (plain) {
		return Number(plain[1]);
	}
	const words = line.match(WORD_PRIORITY_RE);
	if (words) {
		return WORD_PRIORITY_MAP[words[1].toLowerCase()] ?? 3;
	}
	return 3;
}

function cleanTitle(line: string): string {
	let value = line.replace(LINE_PREFIX_RE, '').trim();
	value = value.replace(BRACKET_PRIORITY_RE, ' ');
	value = value.replace(PLAIN_PRIORITY_RE, ' ');
	value = value.replace(WORD_PRIORITY_RE, ' ');
	value = value.replace(ESTIMATE_RE, ' ');
	value = value.replace(DUE_RE, ' ');
	value = value.replace(TAG_RE, ' ');
	value = value.replace(/\s+/g, ' ').trim();
	value = value.replace(/^[\s:;-]+/, '').replace(/[\s.;,]+$/, '');
	return value.slice(0, 180);
}

function parseLabels(line: string): string[] {
	const labels = new Set<string>();
	let match: RegExpExecArray | null;
	while ((match = TAG_RE.exec(line)) !== null) {
		labels.add(match[1].toLowerCase());
	}
	TAG_RE.lastIndex = 0;
	return [...labels];
}

function parseEstimate(line: string): number | null {
	const match = line.match(ESTIMATE_RE);
	if (!match) return null;
	const value = Number(match[1]);
	if (!Number.isFinite(value) || value <= 0) return null;
	return Math.min(value, 999);
}

function parseDue(line: string, now: Date): string | null {
	const match = line.match(DUE_RE);
	if (!match) return null;
	return resolveDue(match[1], now);
}

export function parseActionItems(text: string, options: ParseOptions = {}): ParsedActionItem[] {
	const now = options.now ?? new Date();
	const maxItems = options.maxItems ?? 10;
	const seenTitles = new Set<string>();
	const output: ParsedActionItem[] = [];
	const lines = text.split('\n').map((line) => line.trim());

	for (const rawLine of lines) {
		if (output.length >= maxItems) break;
		if (!rawLine) continue;
		if (HEADING_RE.test(rawLine)) continue;

		const candidate = rawLine.match(LINE_PREFIX_RE) ? rawLine : `- ${rawLine}`;
		const title = cleanTitle(candidate);
		if (title.length < 3) continue;

		const normalized = title.toLowerCase();
		if (seenTitles.has(normalized)) continue;
		seenTitles.add(normalized);

		output.push({
			title,
			priority: parsePriority(candidate),
			due_at: parseDue(candidate, now),
			estimate_min: parseEstimate(candidate),
			labels: parseLabels(candidate)
		});
	}

	return output;
}

type BuildOptions = {
	status?: TaskCreatePayload['status'];
	defaultLabels?: string[];
	baseMetadata?: Record<string, unknown>;
};

export function buildTaskPayloadsFromActionItems(
	items: ParsedActionItem[],
	options: BuildOptions = {}
): TaskCreatePayload[] {
	const status = options.status ?? 'inbox';
	const defaultLabels = options.defaultLabels ?? [];
	const baseMetadata = options.baseMetadata ?? {};

	return items.map((item) => {
		const labels = [...new Set([...defaultLabels, ...item.labels])];
		return {
			title: item.title,
			status,
			priority: item.priority,
			due_at: item.due_at,
			estimate_min: item.estimate_min,
			labels,
			metadata: { ...baseMetadata, ai_extracted_action_item: true }
		};
	});
}
