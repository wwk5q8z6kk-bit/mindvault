import type { NodeKind } from '$lib/api/types';

const KIND_LABELS: Record<string, string> = {
	fact: 'Note', task: 'Task', event: 'Event', person: 'Person',
	place: 'Place', concept: 'Concept', reference: 'Reference',
	decision: 'Decision', preference: 'Preference', code_snippet: 'Code Snippet',
	project: 'Project', conversation: 'Conversation', procedure: 'Procedure',
	observation: 'Observation', bookmark: 'Bookmark'
};

const KIND_COLORS: Record<string, string> = {
	fact: '#38bdf8', task: '#a78bfa', event: '#fb923c', person: '#34d399',
	place: '#f472b6', concept: '#facc15', reference: '#94a3b8',
	decision: '#c084fc', preference: '#f97316', code_snippet: '#22d3ee',
	project: '#4ade80', conversation: '#e879f9', procedure: '#2dd4bf',
	observation: '#fbbf24', bookmark: '#60a5fa'
};

export function kindLabel(kind: string): string {
	return KIND_LABELS[kind] ?? kind;
}

export function kindColor(kind: string): string {
	return KIND_COLORS[kind] ?? '#94a3b8';
}

export function kindBadgeClass(kind: string): string {
	const classes: Record<string, string> = {
		fact: 'bg-sky-500/20 text-sky-300',
		task: 'bg-violet-500/20 text-violet-300',
		event: 'bg-orange-500/20 text-orange-300',
		person: 'bg-emerald-500/20 text-emerald-300',
		place: 'bg-pink-500/20 text-pink-300',
		concept: 'bg-yellow-500/20 text-yellow-300',
		reference: 'bg-slate-500/20 text-slate-300',
		decision: 'bg-purple-500/20 text-purple-300',
		preference: 'bg-orange-400/20 text-orange-200',
		code_snippet: 'bg-cyan-500/20 text-cyan-300',
		project: 'bg-green-500/20 text-green-300',
		conversation: 'bg-fuchsia-500/20 text-fuchsia-300',
		procedure: 'bg-teal-500/20 text-teal-300',
		observation: 'bg-amber-500/20 text-amber-300',
		bookmark: 'bg-blue-500/20 text-blue-300'
	};
	return classes[kind] ?? 'bg-slate-500/20 text-slate-300';
}

export const ALL_NODE_KINDS: NodeKind[] = [
	'fact', 'task', 'event', 'person', 'place', 'concept', 'reference',
	'decision', 'preference', 'code_snippet', 'project', 'conversation',
	'procedure', 'observation', 'bookmark'
];

export const RELATIONSHIP_TYPES = [
	'RelatesTo', 'DependsOn', 'DerivedFrom', 'Supersedes', 'Contradicts',
	'PartOf', 'Contains', 'References', 'SimilarTo', 'FollowsFrom'
] as const;

export type RelationshipType = typeof RELATIONSHIP_TYPES[number];
