import { writable } from 'svelte/store';
import { API_BASE_URL } from '$lib/api/client';
import { tasksStore } from '$lib/stores/tasks';
import { notesStore } from '$lib/stores/notes';
import { db } from '$lib/db';
import { pushToast } from '$lib/stores/toast';
import { nodeToTask, nodeToNote } from '$lib/api/mappers';
import { getNode } from '$lib/api/nodes';
import type { KnowledgeNode } from '$lib/api/types';

export type WsStatus = 'connected' | 'connecting' | 'disconnected';
export const wsStatus = writable<WsStatus>('disconnected');

let changesWs: WebSocket | null = null;
let remindersWs: WebSocket | null = null;
let changesRetryMs = 1000;
let remindersRetryMs = 1000;
let changesRetryTimer: ReturnType<typeof setTimeout> | null = null;
let remindersRetryTimer: ReturnType<typeof setTimeout> | null = null;

function getWsUrl(path: string): string {
	const base = API_BASE_URL.replace(/^http/, 'ws');
	return `${base}${path}`;
}

function removeNodeFromCaches(nodeId: string) {
	db.tasks.delete(nodeId);
	db.notes.delete(nodeId);
	tasksStore.update(items => items.filter(t => t.id !== nodeId));
	notesStore.update(items => items.filter(n => n.id !== nodeId));
}

function upsertNodeInCaches(node: KnowledgeNode, operation: 'create' | 'update' | 'enriched') {
	if (node.kind === 'task') {
		const task = nodeToTask(node);
		db.tasks.put(task);
		if (operation === 'create') {
			tasksStore.update(items => [task, ...items.filter(t => t.id !== task.id)]);
		} else {
			tasksStore.update(items => items.map(t => t.id === task.id ? task : t));
		}
		return;
	}

	if (node.kind === 'fact') {
		const note = nodeToNote(node);
		db.notes.put(note);
		if (operation === 'create') {
			notesStore.update(items => [note, ...items.filter(n => n.id !== note.id)]);
		} else {
			notesStore.update(items => items.map(n => n.id === note.id ? note : n));
		}
	}
}

async function handleChangeMessage(data: unknown) {
	const payload = data as {
		type?: string;
		node?: KnowledgeNode;
		node_id?: string;
		operation?: string;
	};

	if (!payload || typeof payload.type !== 'string') return;

	// Legacy format: { type: "node_created|node_updated|node_deleted", node: {...} }
	if (payload.node) {
		if (payload.type === 'node_deleted') {
			removeNodeFromCaches(payload.node.id);
			return;
		}
		if (payload.type === 'node_created') {
			upsertNodeInCaches(payload.node, 'create');
			return;
		}
		if (payload.type === 'node_updated') {
			upsertNodeInCaches(payload.node, 'update');
			return;
		}
	}

	// Current backend format: { type: "change", operation, node_id }
	if (payload.type !== 'change' || typeof payload.node_id !== 'string') {
		return;
	}

	const operation = (payload.operation ?? '').toLowerCase();
	if (operation === 'delete') {
		removeNodeFromCaches(payload.node_id);
		return;
	}
	if (operation !== 'create' && operation !== 'update' && operation !== 'enriched') {
		return;
	}

	try {
		const node = await getNode(payload.node_id);
		upsertNodeInCaches(node, operation as 'create' | 'update' | 'enriched');
	} catch {
		// Ignore transient races where node was removed between event and fetch.
	}
}

function connectChanges() {
	if (changesWs?.readyState === WebSocket.OPEN) return;
	wsStatus.set('connecting');

	try {
		changesWs = new WebSocket(getWsUrl('/ws/changes'));

		changesWs.onopen = () => {
			wsStatus.set('connected');
			changesRetryMs = 1000;
		};

		changesWs.onmessage = (event) => {
			try {
				const data = JSON.parse(event.data);
				void handleChangeMessage(data);
			} catch { /* ignore parse errors */ }
		};

		changesWs.onclose = () => {
			wsStatus.set('disconnected');
			changesRetryTimer = setTimeout(connectChanges, changesRetryMs);
			changesRetryMs = Math.min(changesRetryMs * 2, 30000);
		};

		changesWs.onerror = () => {
			changesWs?.close();
		};
	} catch {
		wsStatus.set('disconnected');
		changesRetryTimer = setTimeout(connectChanges, changesRetryMs);
		changesRetryMs = Math.min(changesRetryMs * 2, 30000);
	}
}

function connectReminders() {
	if (remindersWs?.readyState === WebSocket.OPEN) return;

	try {
		remindersWs = new WebSocket(getWsUrl('/ws/reminders'));

		remindersWs.onopen = () => { remindersRetryMs = 1000; };

		remindersWs.onmessage = (event) => {
			try {
				const data = JSON.parse(event.data);
				if ('Notification' in window && Notification.permission === 'granted') {
					new Notification('MindVault Reminder', {
						body: data.title || data.message || 'Task reminder',
						tag: data.task_id || 'reminder'
					});
				}
				pushToast(data.title || 'Task reminder', 'info');
			} catch { /* ignore */ }
		};

		remindersWs.onclose = () => {
			remindersRetryTimer = setTimeout(connectReminders, remindersRetryMs);
			remindersRetryMs = Math.min(remindersRetryMs * 2, 30000);
		};

		remindersWs.onerror = () => { remindersWs?.close(); };
	} catch {
		remindersRetryTimer = setTimeout(connectReminders, remindersRetryMs);
		remindersRetryMs = Math.min(remindersRetryMs * 2, 30000);
	}
}

export function startWebSocket() {
	connectChanges();
	connectReminders();
}

export function stopWebSocket() {
	if (changesRetryTimer) clearTimeout(changesRetryTimer);
	if (remindersRetryTimer) clearTimeout(remindersRetryTimer);
	changesWs?.close();
	remindersWs?.close();
	changesWs = null;
	remindersWs = null;
	wsStatus.set('disconnected');
}
