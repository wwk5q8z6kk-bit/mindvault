import { fetchJson } from './client';

export interface PluginSummary {
	id: string;
	name: string;
	version: string;
	description: string | null;
	author: string | null;
	hooks: string[];
}

export interface HookPointInfo {
	name: string;
	description: string;
}

export async function listPlugins(): Promise<{ plugins: PluginSummary[]; count: number }> {
	return fetchJson('/api/v1/plugins');
}

export async function listHookPoints(): Promise<{ hooks: HookPointInfo[] }> {
	return fetchJson('/api/v1/plugins/hooks');
}
