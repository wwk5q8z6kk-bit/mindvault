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

/** Identifiers and status only — never artifact bytes or declared write scope. */
export type AgentRunTransitionObservation = {
    kind: 'transition';
    run_id: string;
    work_order_id: string;
    status: string;
    failure_class: string | null;
};

export type AgentRunGateObservation = {
    kind: 'gate';
    run_id: string;
    work_order_id: string;
    gate: string;
    outcome: string;
};

export type AgentRunObservation = AgentRunTransitionObservation | AgentRunGateObservation;

/**
 * Latest governed-run observation from `/ws/agent`.
 *
 * `seq` is monotonic so identical successive statuses still notify subscribers.
 * Namespace-scoped sockets never receive these events (server filter); those
 * clients must poll the query API — see WORK_ORDER_MODEL.md.
 */
export const agentRunObservations = writable<{
    seq: number;
    event: AgentRunObservation;
} | null>(null);

let agentRunObservationSeq = 0;

let socket: WebSocket | null = null;
let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
let stopped = false;

/** Pure dispatcher for unit tests and the live WebSocket handler. */
export function dispatchAgentNotification(data: unknown): void {
    if (!data || typeof data !== 'object') return;
    const message = data as Record<string, unknown>;
    const type = message.type;

    if (type === 'related_context') {
        agentStore.update((state) => ({
            ...state,
            relatedNodes: (message.nodes as RelatedNode[]) ?? []
        }));
        return;
    }
    if (type === 'node_enriched' && typeof message.node_id === 'string') {
        enrichedNodes.update((s) => {
            const next = new Set(s);
            next.add(message.node_id as string);
            return next;
        });
        return;
    }
    if (type === 'chronicle' && message.entry) {
        chronicles.update((list) => [message.entry as ChronicleEntry, ...list].slice(0, 100));
        return;
    }
    if (type === 'intent' && message.intent) {
        const intent = message.intent as CapturedIntent;
        intents.update((list) => {
            const filtered = list.filter((i) => i.id !== intent.id);
            return [intent, ...filtered];
        });
        return;
    }
    if (type === 'insight_discovered' && message.insight) {
        insights.update((list) => [message.insight as ProactiveInsight, ...list]);
        return;
    }
    if (
        type === 'agent_run_transitioned' &&
        typeof message.run_id === 'string' &&
        typeof message.work_order_id === 'string' &&
        typeof message.status === 'string'
    ) {
        agentRunObservationSeq += 1;
        agentRunObservations.set({
            seq: agentRunObservationSeq,
            event: {
                kind: 'transition',
                run_id: message.run_id,
                work_order_id: message.work_order_id,
                status: message.status,
                failure_class:
                    typeof message.failure_class === 'string' ? message.failure_class : null
            }
        });
        return;
    }
    if (
        type === 'agent_run_gate_recorded' &&
        typeof message.run_id === 'string' &&
        typeof message.work_order_id === 'string' &&
        typeof message.gate === 'string' &&
        typeof message.outcome === 'string'
    ) {
        agentRunObservationSeq += 1;
        agentRunObservations.set({
            seq: agentRunObservationSeq,
            event: {
                kind: 'gate',
                run_id: message.run_id,
                work_order_id: message.work_order_id,
                gate: message.gate,
                outcome: message.outcome
            }
        });
    }
}

export function connectAgentStream() {
    if (!browser || socket) return;
    stopped = false;

    const wsBaseUrl = API_BASE_URL.replace(/^http/, 'ws');
    const url = `${wsBaseUrl}/ws/agent`;

    socket = new WebSocket(url);

    socket.onmessage = (event: MessageEvent) => {
        try {
            dispatchAgentNotification(JSON.parse(event.data));
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

export interface ReflectionStats {
	intent_type: string;
	total_count: number;
	applied_count: number;
	dismissed_count: number;
	acceptance_rate: number;
	avg_confidence: number;
}

export async function fetchAgentContext(basisNodeId?: string) {
    const params = new URLSearchParams();
    if (basisNodeId) params.set('basis_node_id', basisNodeId);
    const query = params.toString();
    const url = query ? `/api/v1/agent/context?${query}` : '/api/v1/agent/context';

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
    const query = params.toString();
    const url = query ? `/api/v1/agent/intents?${query}` : '/api/v1/agent/intents';
    const data = await fetchJson<CapturedIntent[]>(url);
    intents.set(data);
    return data;
}

export async function applyIntent(id: string) {
    await fetchJson(`/api/v1/agent/intents/${id}/apply`, { method: 'POST' });
    intents.update(list => list.filter(i => i.id !== id));
}

export async function dismissIntent(id: string) {
    await fetchJson(`/api/v1/agent/intents/${id}/dismiss`, { method: 'POST' });
    intents.update(list => list.filter(i => i.id !== id));
}

export async function fetchInsights() {
    const data = await fetchJson<ProactiveInsight[]>('/api/v1/proactive/insights');
    insights.set(data);
    return data;
}

export async function generateInsights() {
    const data = await fetchJson<ProactiveInsight[]>('/api/v1/proactive/generate', { method: 'POST' });
    insights.update(list => [...data, ...list]);
    return data;
}

export async function fetchAiModels() {
    return await fetchJson<ModelRegistry>('/api/v1/agent/models');
}

export async function fetchReflectionStats(intentType: string): Promise<ReflectionStats> {
	const params = new URLSearchParams({ intent_type: intentType });
	return await fetchJson<ReflectionStats>(`/api/v1/agent/reflection/stats?${params.toString()}`);
}
