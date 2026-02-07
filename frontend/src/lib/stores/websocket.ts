import { writable } from 'svelte/store';
import { API_BASE_URL } from '$lib/api/client';
import { tasksStore } from '$lib/stores/tasks';
import { notesStore } from '$lib/stores/notes';
import { db } from '$lib/db';
import { pushToast } from '$lib/stores/toast';
import { nodeToTask, nodeToNote } from '$lib/api/mappers';
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

function handleChangeMessage(data: { type: string; node: KnowledgeNode }) {
	const { type, node } = data;
	if (node.kind === 'task') {
		const task = nodeToTask(node);
		if (type === 'node_created') {
			db.tasks.put(task);
			tasksStore.update(items => [task, ...items.filter(t => t.id !== task.id)]);
		} else if (type === 'node_updated') {
			db.tasks.put(task);
			tasksStore.update(items => items.map(t => t.id === task.id ? task : t));
		} else if (type === 'node_deleted') {
			db.tasks.delete(node.id);
			tasksStore.update(items => items.filter(t => t.id !== node.id));
		}
	} else if (node.kind === 'fact') {
		const note = nodeToNote(node);
		if (type === 'node_created') {
			db.notes.put(note);
			notesStore.update(items => [note, ...items.filter(n => n.id !== note.id)]);
		} else if (type === 'node_updated') {
			db.notes.put(note);
			notesStore.update(items => items.map(n => n.id === note.id ? note : n));
		} else if (type === 'node_deleted') {
			db.notes.delete(node.id);
			notesStore.update(items => items.filter(n => n.id !== node.id));
		}
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
				handleChangeMessage(data);
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
