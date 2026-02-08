import { fetchJson } from './client';

export type RuleType = 'global' | 'domain' | 'contact' | 'tag';
export type AutonomyDecision = 'auto_apply' | 'defer' | 'block' | 'queue_for_later';

export interface AutonomyRule {
	id: string;
	rule_type: RuleType;
	scope_key: string;
	auto_apply_threshold: number;
	max_actions_per_hour: number;
	allowed_intent_types: string[];
	blocked_intent_types: string[];
	quiet_hours_start: string | null;
	quiet_hours_end: string | null;
	quiet_hours_timezone: string | null;
	enabled: boolean;
	created_at: string;
}

export interface ActionLogEntry {
	id: string;
	rule_id: string | null;
	intent_type: string;
	decision: AutonomyDecision;
	confidence: number;
	reason: string;
	created_at: string;
}

export async function listRules(): Promise<AutonomyRule[]> {
	const res = await fetchJson<{ rules: AutonomyRule[] }>('/api/v1/autonomy/rules');
	return res.rules;
}

export async function createRule(data: Partial<AutonomyRule>): Promise<AutonomyRule> {
	return fetchJson<AutonomyRule>('/api/v1/autonomy/rules', {
		method: 'POST',
		body: JSON.stringify(data)
	});
}

export async function getRule(id: string): Promise<AutonomyRule> {
	return fetchJson<AutonomyRule>(`/api/v1/autonomy/rules/${id}`);
}

export async function updateRule(id: string, data: Partial<AutonomyRule>): Promise<AutonomyRule> {
	return fetchJson<AutonomyRule>(`/api/v1/autonomy/rules/${id}`, {
		method: 'PUT',
		body: JSON.stringify(data)
	});
}

export async function deleteRule(id: string): Promise<void> {
	await fetchJson<void>(`/api/v1/autonomy/rules/${id}`, { method: 'DELETE' });
}

export async function listActionLog(limit = 50, offset = 0): Promise<ActionLogEntry[]> {
	const res = await fetchJson<{ entries: ActionLogEntry[] }>(
		`/api/v1/autonomy/action-log?limit=${limit}&offset=${offset}`
	);
	return res.entries;
}
