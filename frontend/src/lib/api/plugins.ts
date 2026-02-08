import { API_BASE_URL, ApiError, fetchJson } from './client';

export interface PluginSummary {
	id: string;
	name: string;
	version: string;
	description: string | null;
	author: string | null;
	hooks: string[];
	status?: string;
}

export interface HookPointInfo {
	name: string;
	description: string;
}

export interface InstallResult {
	id: string;
	name: string;
	status: string;
}

export interface ReloadResult {
	status: string;
	count: number;
	plugins: string[];
}

export async function listPlugins(): Promise<{ plugins: PluginSummary[]; count: number }> {
	return fetchJson('/api/v1/plugins');
}

export async function listHookPoints(): Promise<{ hooks: HookPointInfo[] }> {
	return fetchJson('/api/v1/plugins/hooks');
}

export async function installPlugin(
	manifestJson: string,
	wasmFile: File
): Promise<InstallResult> {
	const form = new FormData();
	form.append('manifest', new Blob([manifestJson], { type: 'application/json' }), 'manifest.json');
	form.append('wasm', wasmFile);

	const res = await fetch(`${API_BASE_URL}/api/v1/plugins`, {
		method: 'POST',
		body: form
	});
	if (!res.ok) {
		const body = await res.text();
		throw new ApiError(`Install failed (${res.status})`, res.status, body);
	}
	return res.json();
}

export async function uninstallPlugin(name: string): Promise<{ name: string; status: string }> {
	return fetchJson(`/api/v1/plugins/${encodeURIComponent(name)}`, { method: 'DELETE' });
}

export async function reloadPlugins(): Promise<ReloadResult> {
	return fetchJson('/api/v1/plugins/reload', { method: 'POST' });
}
