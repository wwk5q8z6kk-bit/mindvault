import { writable } from 'svelte/store';
import { browser } from '$app/environment';
import { fetchJson, API_BASE_URL } from './client';
import type {
    ChronicleEntry,
    CapturedIntent,
    ProactiveInsight,
    ModelRegistry,
    IntentStatus
} from './types';

export interface RelatedNode {
    id: string;
    title: string;
    updated_at: string;
}

export interface AgentContext {
    summary: string;
    relatedNodes: RelatedNode[];
}

export const agentStore = writable<AgentContext>({
    summary: 'The AI assistant is ready to help.',
    relatedNodes: []
});

export const enrichedNodes = writable<Set<string>>(new Set());

// --- Agentic Brain Stores ---
export const chronicles = writable<ChronicleEntry[]>([]);
export const intents = writable<CapturedIntent[]>([]);
export const insights = writable<ProactiveInsight[]>([]);

let socket: WebSocket | null = null;
let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
let stopped = false;

export function connectAgentStream() {
    if (!browser || socket) return;
    stopped = false;

    const wsBaseUrl = API_BASE_URL.replace(/^http/, 'ws');
    const url = `${wsBaseUrl}/ws/agent`;

    socket = new WebSocket(url);

    socket.onmessage = (event: MessageEvent) => {
        try {
            const data = JSON.parse(event.data);
            if (data.type === 'related_context') {
                agentStore.update(state => ({
                    ...state,
                    relatedNodes: data.nodes
                }));
            } else if (data.type === 'node_enriched') {
                enrichedNodes.update(s => {
                    const newSet = new Set(s);
                    newSet.add(data.node_id);
                    return newSet;
                });
            } else if (data.type === 'chronicle') {
                chronicles.update(list => [data.entry, ...list].slice(0, 100));
            } else if (data.type === 'intent') {
                intents.update(list => {
                    const filtered = list.filter(i => i.id !== data.intent.id);
                    return [data.intent, ...filtered];
                });
            } else if (data.type === 'insight_discovered') {
                insights.update(list => [data.insight, ...list]);
            }
        } catch (e) {
            console.warn('[ws/agent] Failed to parse message:', e);
        }
    };

    socket.onclose = () => {
        socket = null;
        if (!stopped) {
            reconnectTimer = setTimeout(connectAgentStream, 5000);
        }
    };
}

export function disconnectAgentStream() {
    stopped = true;
    if (reconnectTimer) {
        clearTimeout(reconnectTimer);
        reconnectTimer = null;
    }
    socket?.close();
    socket = null;
}

export interface AgentContextResponse {
    executive_summary: string;
    related_nodes: RelatedNode[];
}

export async function fetchAgentContext(basisNodeId?: string) {
    const params = new URLSearchParams();
    if (basisNodeId) params.set('basis_node_id', basisNodeId);
    const query = params.toString();
    const url = query ? `/api/agent/context?${query}` : '/api/agent/context';

    try {
        const data = await fetchJson<AgentContextResponse>(url);
        agentStore.set({
            summary: data.executive_summary,
            relatedNodes: data.related_nodes
        });
    } catch (e) {
        console.error('Failed to fetch agent context', e);
    }
}

// --- Agentic Brain API Methods ---

export async function fetchIntents(nodeId?: string, status?: IntentStatus) {
    const params = new URLSearchParams();
    if (nodeId) params.set('node_id', nodeId);
    if (status) params.set('status', status);
    const data = await fetchJson<CapturedIntent[]>(`/api/agent/intents?${params.toString()}`);
    intents.set(data);
    return data;
}

export async function applyIntent(id: string) {
    await fetchJson(`/api/agent/intents/${id}/apply`, { method: 'POST' });
    intents.update(list => list.filter(i => i.id !== id));
}

export async function dismissIntent(id: string) {
    await fetchJson(`/api/agent/intents/${id}/dismiss`, { method: 'POST' });
    intents.update(list => list.filter(i => i.id !== id));
}

export async function fetchInsights() {
    const data = await fetchJson<ProactiveInsight[]>('/api/proactive/insights');
    insights.set(data);
    return data;
}

export async function generateInsights() {
    const data = await fetchJson<ProactiveInsight[]>('/api/proactive/generate', { method: 'POST' });
    insights.update(list => [...data, ...list]);
    return data;
}

export async function fetchAiModels() {
    return await fetchJson<ModelRegistry>('/api/agent/models');
}
