/** Backend KnowledgeNode model — matches the Rust struct exactly. */
export interface KnowledgeNode {
	id: string;
	kind: NodeKind;
	title: string;
	content?: string | null;
	source?: string | null;
	namespace?: string | null;
	tags: string[];
	importance: number;
	temporal: NodeTemporal;
	metadata: Record<string, unknown>;
}

export type NodeKind =
	| 'fact'
	| 'task'
	| 'event'
	| 'person'
	| 'place'
	| 'concept'
	| 'reference'
	| 'decision'
	| 'preference'
	| 'code_snippet'
	| 'project'
	| 'conversation'
	| 'procedure'
	| 'observation'
	| 'bookmark'
	| 'template'
	| 'saved_view';

export interface NodeTemporal {
	created_at: string;
	updated_at: string;
	accessed_at?: string | null;
	expires_at?: string | null;
}

/** Payload for POST /api/v1/nodes */
export interface StoreNodeRequest {
	kind: NodeKind;
	title: string;
	content?: string | null;
	source?: string | null;
	namespace?: string | null;
	tags?: string[];
	importance?: number;
	metadata?: Record<string, unknown>;
}

/** Response from GET /api/v1/search and POST /api/v1/recall */
export interface SearchResultDto {
	node: KnowledgeNode;
	score: number;
	match_source: string;
}

// --- Metadata key constants ---
export const TASK_DUE_AT = 'task_due_at';
export const TASK_COMPLETED = 'task_completed';
export const TASK_COMPLETED_AT = 'task_completed_at';
export const TASK_STATUS = 'task_status';
export const TASK_RECURRENCE = 'task_recurrence';
export const TASK_ESTIMATE_MIN = 'task_estimate_min';
export const TASK_ASSIGNEE = 'task_assignee';
export const TASK_DEPENDENCIES = 'task_dependencies';
export const NOTE_PINNED = 'pinned';
