import { get } from 'svelte/store';
import { tasksStore } from './tasks';
import { pushToast } from './toast';

let checkInterval: ReturnType<typeof setInterval> | null = null;
let notifiedIds = new Set<string>();

function isEnabled(): boolean {
	return localStorage.getItem('mv_feature_notifications') !== 'false';
}

function getLeadMinutes(): number {
	return parseInt(localStorage.getItem('mv_notification_lead_minutes') ?? '30', 10);
}

function getCheckIntervalMs(): number {
	return parseInt(localStorage.getItem('mv_notification_check_interval') ?? '60000', 10);
}

async function requestPermission(): Promise<boolean> {
	if (!('Notification' in window)) return false;
	if (Notification.permission === 'granted') return true;
	if (Notification.permission === 'denied') return false;
	const result = await Notification.requestPermission();
	return result === 'granted';
}

function sendNotification(title: string, body: string) {
	if (Notification.permission === 'granted') {
		new Notification(title, {
			body,
			icon: '/favicon.png',
			tag: 'mindvault-reminder'
		});
	}
}

function checkDueTasks() {
	if (!isEnabled()) return;

	const tasks = get(tasksStore);
	const now = new Date();
	const leadMs = getLeadMinutes() * 60 * 1000;

	for (const task of tasks) {
		if (!task.due_at || task.status === 'done') continue;
		if (notifiedIds.has(task.id)) continue;

		const due = new Date(task.due_at);
		const diff = due.getTime() - now.getTime();

		// Overdue
		if (diff < 0) {
			notifiedIds.add(task.id);
			sendNotification('Overdue Task', `"${task.title}" was due ${formatRelative(due)}`);
			pushToast(`Overdue: ${task.title}`, 'warning');
		}
		// Due soon (within lead time)
		else if (diff <= leadMs) {
			notifiedIds.add(task.id);
			sendNotification('Task Due Soon', `"${task.title}" is due ${formatRelative(due)}`);
		}
	}
}

function formatRelative(date: Date): string {
	const now = new Date();
	const diffMs = now.getTime() - date.getTime();
	const absDiff = Math.abs(diffMs);
	const minutes = Math.floor(absDiff / 60000);
	const hours = Math.floor(minutes / 60);

	if (diffMs > 0) {
		if (minutes < 60) return `${minutes}m ago`;
		if (hours < 24) return `${hours}h ago`;
		return `${Math.floor(hours / 24)}d ago`;
	} else {
		if (minutes < 60) return `in ${minutes}m`;
		if (hours < 24) return `in ${hours}h`;
		return `in ${Math.floor(hours / 24)}d`;
	}
}

export function startNotifications() {
	if (!isEnabled()) return;
	void requestPermission();
	stopNotifications();
	checkDueTasks();
	checkInterval = setInterval(checkDueTasks, getCheckIntervalMs());
}

export function stopNotifications() {
	if (checkInterval) {
		clearInterval(checkInterval);
		checkInterval = null;
	}
}

export function resetNotifiedIds() {
	notifiedIds = new Set();
}
