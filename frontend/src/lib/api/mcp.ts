import { fetchJson } from './client';

export interface McpConnector {
	id: string;
	name: string;
	description: string | null;
	publisher: string | null;
	version: string;
	homepage_url: string | null;
	repository_url: string | null;
	config_schema: Record<string, unknown>;
	capabilities: string[];
	verified: boolean;
	created_at: string;
	updated_at: string;
}

export interface ListMcpConnectorsParams {
	publisher?: string;
	verified?: boolean;
	limit?: number;
	offset?: number;
}

export interface CreateMcpConnectorRequest {
	name: string;
	version: string;
	description?: string | null;
	publisher?: string | null;
	homepage_url?: string | null;
	repository_url?: string | null;
	config_schema?: Record<string, unknown> | null;
	capabilities?: string[] | null;
	verified?: boolean | null;
}

export interface UpdateMcpConnectorRequest {
	name?: string | null;
	version?: string | null;
	description?: string | null;
	publisher?: string | null;
	homepage_url?: string | null;
	repository_url?: string | null;
	config_schema?: Record<string, unknown> | null;
	capabilities?: string[] | null;
	verified?: boolean | null;
}

function buildQuery(params?: ListMcpConnectorsParams): string {
	if (!params) return '';
	const search = new URLSearchParams();
	if (params.publisher) search.set('publisher', params.publisher);
	if (typeof params.verified === 'boolean') search.set('verified', String(params.verified));
	if (typeof params.limit === 'number') search.set('limit', String(params.limit));
	if (typeof params.offset === 'number') search.set('offset', String(params.offset));
	const qs = search.toString();
	return qs ? `?${qs}` : '';
}

export async function listMcpConnectors(
	params?: ListMcpConnectorsParams
): Promise<McpConnector[]> {
	return fetchJson(`/api/v1/mcp/connectors${buildQuery(params)}`);
}

export async function getMcpConnector(id: string): Promise<McpConnector> {
	return fetchJson(`/api/v1/mcp/connectors/${encodeURIComponent(id)}`);
}

export async function createMcpConnector(
	payload: CreateMcpConnectorRequest
): Promise<McpConnector> {
	return fetchJson('/api/v1/mcp/connectors', {
		method: 'POST',
		body: JSON.stringify(payload)
	});
}

export async function updateMcpConnector(
	id: string,
	payload: UpdateMcpConnectorRequest
): Promise<McpConnector> {
	return fetchJson(`/api/v1/mcp/connectors/${encodeURIComponent(id)}`, {
		method: 'PUT',
		body: JSON.stringify(payload)
	});
}

export async function deleteMcpConnector(id: string): Promise<void> {
	await fetchJson(`/api/v1/mcp/connectors/${encodeURIComponent(id)}`, {
		method: 'DELETE'
	});
}
