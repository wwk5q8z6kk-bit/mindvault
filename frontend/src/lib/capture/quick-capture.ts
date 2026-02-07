export const QUICK_CAPTURE_EVENT_NAME = 'mindvault:quick-capture';

export const QUICK_CAPTURE_MODES = ['task', 'note', 'link', 'voice'] as const;
export type QuickCaptureMode = (typeof QUICK_CAPTURE_MODES)[number];

export const QUICK_CAPTURE_TARGETS = ['default', 'inbox', 'daily', 'planned', 'review'] as const;
export type QuickCaptureTarget = (typeof QUICK_CAPTURE_TARGETS)[number];

export interface QuickCaptureRequest {
	mode?: QuickCaptureMode;
	target?: QuickCaptureTarget;
	prefill?: string;
}

export function isQuickCaptureMode(value: unknown): value is QuickCaptureMode {
	return (
		value === 'task' || value === 'note' || value === 'link' || value === 'voice'
	);
}

export function isQuickCaptureTarget(value: unknown): value is QuickCaptureTarget {
	return (
		value === 'default' ||
		value === 'inbox' ||
		value === 'daily' ||
		value === 'planned' ||
		value === 'review'
	);
}

export function dispatchQuickCapture(request: QuickCaptureRequest = {}): boolean {
	if (typeof window === 'undefined') return false;
	const detail: Record<string, unknown> = {};
	if (request.mode) detail.mode = request.mode;
	if (request.target) detail.target = request.target;
	if (request.prefill && request.prefill.trim()) detail.prefill = request.prefill.trim();
	window.dispatchEvent(new CustomEvent(QUICK_CAPTURE_EVENT_NAME, { detail }));
	return true;
}
