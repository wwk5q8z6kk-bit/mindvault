import { get } from 'svelte/store';
import { tasksStore } from './tasks';
import { pushToast } from './toast';
import { dispatchQuickCapture, type QuickCaptureMode, type QuickCaptureTarget } from '$lib/capture/quick-capture';

let checkInterval: ReturnType<typeof setInterval> | null = null;
let notifiedIds = new Set<string>();
const NOTIFICATION_CLICK_ACTION_STORAGE_KEY = 'mv_notification_click_action';

export type NotificationClickAction = 'none' | 'inbox' | 'daily';

function isEnabled(): boolean {
	return localStorage.getItem('mv_feature_notifications') !== 'false';
}

function getLeadMinutes(): number {
	return parseInt(localStorage.getItem('mv_notification_lead_minutes') ?? '30', 10);
}

function getCheckIntervalMs(): number {
	return parseInt(localStorage.getItem('mv_notification_check_interval') ?? '60000', 10);
}

export function getNotificationClickAction(): NotificationClickAction {
	const stored = localStorage.getItem(NOTIFICATION_CLICK_ACTION_STORAGE_KEY);
	if (stored === 'none' || stored === 'daily') return stored;
	return 'inbox';
}

function resolveNotificationCaptureAction(): {
	mode: QuickCaptureMode;
	target: QuickCaptureTarget;
} | null {
	const action = getNotificationClickAction();
	if (action === 'none') return null;
	if (action === 'daily') {
		return { mode: 'note', target: 'daily' };
	}
	return { mode: 'task', target: 'inbox' };
}

async function requestPermission(): Promise<boolean> {
	if (!('Notification' in window)) return false;
	if (Notification.permission === 'granted') return true;
	if (Notification.permission === 'denied') return false;
	const result = await Notification.requestPermission();
	return result === 'granted';
}

function openReminderQuickCapture(taskTitle: string) {
	const action = resolveNotificationCaptureAction();
	if (!action) return;
	window.focus();
	dispatchQuickCapture({
		mode: action.mode,
		target: action.target,
		prefill: `Follow up: ${taskTitle}`
	});
	pushToast('Reminder opened quick capture', 'info', 2000);
}

function sendNotification(title: string, body: string, taskTitle?: string) {
	if (Notification.permission === 'granted') {
		const clickableAction = taskTitle ? resolveNotificationCaptureAction() : null;
		const notification = new Notification(title, {
			body: clickableAction ? `${body}\nClick to capture follow-up.` : body,
			icon: '/favicon.png',
			tag: 'mindvault-reminder'
		});
		if (taskTitle && clickableAction) {
			notification.onclick = () => {
				openReminderQuickCapture(taskTitle);
				notification.close();
			};
		}
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
			sendNotification('Overdue Task', `"${task.title}" was due ${formatRelative(due)}`, task.title);
			pushToast(`Overdue: ${task.title}`, 'warning');
		}
		// Due soon (within lead time)
		else if (diff <= leadMs) {
			notifiedIds.add(task.id);
			sendNotification('Task Due Soon', `"${task.title}" is due ${formatRelative(due)}`, task.title);
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
