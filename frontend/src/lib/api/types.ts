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
	| 'saved_view'
	| 'proposal';

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

// ---------------------------------------------------------------------------
// Agentic Intelligence Types
// ---------------------------------------------------------------------------

export type IntentStatus = 'suggested' | 'applied' | 'dismissed';

export type IntentType =
	| 'schedule_reminder'
	| 'extract_task'
	| 'link_to_project'
	| 'suggest_tag'
	| 'suggest_link'
	| string; // For custom intents

export type InsightType =
	| 'connection'
	| 'trend'
	| 'gap'
	| 'stale'
	| 'reminder'
	| 'cluster'
	| 'general'
	| 'temporal_pattern'
	| 'knowledge_gap'
	| 'cross_domain'
	| 'unlinked_cluster'
	| 'ambient_link'
	| 'conflict';

export interface CapturedIntent {
	id: string;
	node_id: string;
	intent_type: IntentType;
	confidence: number;
	parameters: Record<string, unknown> | null;
	status: IntentStatus;
	created_at: string;
	updated_at: string | null;
}

export interface ProactiveInsight {
	id: string;
	title: string;
	content: string;
	insight_type: InsightType;
	related_node_ids: string[];
	importance: number;
	created_at: string;
	dismissed_at: string | null;
}

export interface ChronicleEntry {
	id: string;
	node_id: string | null;
	step_name: string;
	logic: string;
	input_snapshot: string | null;
	output_snapshot: string | null;
	timestamp: string;
}

export interface ModelRegistry {
	embedding: {
		provider: string;
		model: string;
	};
}

// ---------------------------------------------------------------------------
// Exchange / Proposal Types (Phase 1.3)
// ---------------------------------------------------------------------------

export type ProposalSender = 'agent' | 'mcp' | 'webhook' | 'watcher' | 'self' | 'relay';

export type ProposalAction =
	| 'create_node'
	| 'update_node'
	| 'delete_node'
	| 'suggest_tag'
	| 'suggest_link'
	| 'schedule_reminder'
	| 'custom';

export type ProposalState =
	| 'pending'
	| 'approved'
	| 'rejected'
	| 'expired'
	| 'auto_approved';

export interface Proposal {
	id: string;
	node_id: string | null;
	target_node_id: string | null;
	sender: ProposalSender;
	action: ProposalAction;
	state: ProposalState;
	confidence: number;
	diff_preview: string | null;
	payload: Record<string, unknown>;
	created_at: string;
	updated_at: string | null;
	resolved_at: string | null;
}
