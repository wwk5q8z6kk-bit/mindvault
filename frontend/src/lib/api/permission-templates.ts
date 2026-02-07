import { fetchJson } from './client';

export interface PermissionTemplate {
	id: string;
	name: string;
	description?: string | null;
	tier: 'view' | 'edit' | 'action' | 'admin';
	allowed_namespaces?: string[] | null;
	allowed_tags?: string[] | null;
	allowed_kinds?: string[] | null;
	allowed_actions?: string[] | null;
	created_at: string;
}

export interface CreatePermissionTemplatePayload {
	name: string;
	description?: string;
	tier: 'view' | 'edit' | 'action' | 'admin';
	allowed_namespaces?: string[];
	allowed_tags?: string[];
	allowed_kinds?: string[];
	allowed_actions?: string[];
}

export async function listPermissionTemplates(): Promise<PermissionTemplate[]> {
	return fetchJson<PermissionTemplate[]>('/api/v1/permission-templates');
}

export async function createPermissionTemplate(payload: CreatePermissionTemplatePayload): Promise<PermissionTemplate> {
	return fetchJson<PermissionTemplate>('/api/v1/permission-templates', {
		method: 'POST',
		body: JSON.stringify(payload)
	});
}

export async function updatePermissionTemplate(id: string, payload: Partial<CreatePermissionTemplatePayload>): Promise<PermissionTemplate> {
	return fetchJson<PermissionTemplate>(`/api/v1/permission-templates/${id}`, {
		method: 'PUT',
		body: JSON.stringify(payload)
	});
}

export async function deletePermissionTemplate(id: string): Promise<void> {
	await fetchJson<{ deleted: boolean }>(`/api/v1/permission-templates/${id}`, { method: 'DELETE' });
}
