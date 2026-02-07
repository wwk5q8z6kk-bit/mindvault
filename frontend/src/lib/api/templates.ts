import { fetchJson } from './client';
import type { KnowledgeNode } from './types';

export interface TemplatePackSummary {
	pack_id: string;
	name: string;
	description: string;
	template_count: number;
}

export interface InstallTemplatePackResponse {
	pack_id: string;
	namespace: string;
	installed_templates: number;
	updated_templates: number;
	skipped_templates: number;
	template_ids: string[];
}

export interface TemplateVersionSummary {
	version_id: string;
	captured_at: string;
	kind: string;
	namespace: string;
	title?: string | null;
	source?: string | null;
	importance: number;
	tag_count: number;
	content_preview: string;
}

export interface TemplateVersionFieldChange {
	field: string;
	changed: boolean;
	version_value: string;
	current_value: string;
}

export interface TemplateVersionDiffSummary {
	version_line_count: number;
	current_line_count: number;
	added_line_count: number;
	removed_line_count: number;
	added_line_samples: string[];
	removed_line_samples: string[];
}

export interface TemplateVersionDetail {
	version: {
		version_id: string;
		captured_at: string;
		kind: string;
		namespace: string;
		title?: string | null;
		content: string;
		source?: string | null;
		tags: string[];
		importance: number;
		metadata: Record<string, unknown>;
	};
	current: {
		kind: string;
		namespace: string;
		title?: string | null;
		content: string;
		source?: string | null;
		tags: string[];
		importance: number;
	};
	diff: TemplateVersionDiffSummary;
	field_changes: TemplateVersionFieldChange[];
}

export interface ListTemplatesParams {
	namespace?: string;
	kind?: string;
	limit?: number;
	offset?: number;
}

export interface CreateTemplatePayload {
	kind: string;
	content: string;
	title?: string;
	source?: string;
	namespace?: string;
	tags?: string[];
	importance?: number;
	metadata?: Record<string, unknown>;
	template_key?: string;
	template_variables?: string[];
}

export interface InstantiateTemplatePayload {
	namespace?: string;
	title?: string;
	tags?: string[];
	values?: Record<string, string>;
	metadata?: Record<string, unknown>;
}

export interface ApplyTemplatePayload {
	target_node_id?: string;
	target_kind?: string;
	overwrite?: boolean;
}

export interface ApplyTemplateResponse {
	node: KnowledgeNode;
	created: boolean;
	filled_fields: string[];
	overwritten_fields: string[];
}

export interface InstallTemplatePackPayload {
	namespace?: string;
	overwrite_existing?: boolean;
	additional_tags?: string[];
}

export async function listTemplates(params: ListTemplatesParams = {}): Promise<KnowledgeNode[]> {
	const query = new URLSearchParams();
	if (params.namespace) query.set('namespace', params.namespace);
	if (params.kind) query.set('kind', params.kind);
	query.set('limit', String(params.limit ?? 200));
	query.set('offset', String(params.offset ?? 0));
	return await fetchJson<KnowledgeNode[]>(`/api/v1/templates?${query.toString()}`);
}

export async function createTemplate(payload: CreateTemplatePayload): Promise<KnowledgeNode> {
	return await fetchJson<KnowledgeNode>('/api/v1/templates', {
		method: 'POST',
		body: JSON.stringify(payload)
	});
}

export async function instantiateTemplate(
	templateId: string,
	payload: InstantiateTemplatePayload
): Promise<KnowledgeNode> {
	return await fetchJson<KnowledgeNode>(`/api/v1/templates/${templateId}/instantiate`, {
		method: 'POST',
		body: JSON.stringify(payload)
	});
}

export async function applyTemplate(
	templateId: string,
	payload: ApplyTemplatePayload
): Promise<ApplyTemplateResponse> {
	return await fetchJson<ApplyTemplateResponse>(`/api/v1/templates/${templateId}/apply`, {
		method: 'POST',
		body: JSON.stringify(payload)
	});
}

export async function deleteTemplate(templateId: string): Promise<{ deleted: boolean }> {
	return await fetchJson<{ deleted: boolean }>(`/api/v1/templates/${templateId}`, {
		method: 'DELETE'
	});
}

export async function listTemplatePacks(): Promise<TemplatePackSummary[]> {
	return await fetchJson<TemplatePackSummary[]>('/api/v1/template-packs');
}

export async function installTemplatePack(
	packId: string,
	payload: InstallTemplatePackPayload
): Promise<InstallTemplatePackResponse> {
	return await fetchJson<InstallTemplatePackResponse>(
		`/api/v1/template-packs/${encodeURIComponent(packId)}/install`,
		{
			method: 'POST',
			body: JSON.stringify(payload)
		}
	);
}

export async function listTemplateVersions(templateId: string): Promise<TemplateVersionSummary[]> {
	return await fetchJson<TemplateVersionSummary[]>(
		`/api/v1/templates/${encodeURIComponent(templateId)}/versions`
	);
}

export async function getTemplateVersion(
	templateId: string,
	versionId: string
): Promise<TemplateVersionDetail> {
	return await fetchJson<TemplateVersionDetail>(
		`/api/v1/templates/${encodeURIComponent(templateId)}/versions/${encodeURIComponent(versionId)}`
	);
}

export async function restoreTemplateVersion(
	templateId: string,
	versionId: string
): Promise<KnowledgeNode> {
	return await fetchJson<KnowledgeNode>(
		`/api/v1/templates/${encodeURIComponent(templateId)}/versions/${encodeURIComponent(versionId)}/restore`,
		{
			method: 'POST'
		}
	);
}

export function readTemplateVariables(node: KnowledgeNode): string[] {
	const raw = node.metadata?.template_variables;
	if (!Array.isArray(raw)) return [];
	return raw
		.map((item) => (typeof item === 'string' ? item.trim() : ''))
		.filter((item) => item.length > 0);
}

export function readTemplateKey(node: KnowledgeNode): string | null {
	const raw = node.metadata?.template_key;
	return typeof raw === 'string' && raw.trim().length > 0 ? raw.trim() : null;
}

export function readTemplateTargetKind(node: KnowledgeNode): string | null {
	const raw = node.metadata?.template_target_kind;
	return typeof raw === 'string' && raw.trim().length > 0 ? raw.trim() : null;
}
