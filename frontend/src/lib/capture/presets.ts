import {
	isQuickCaptureMode,
	isQuickCaptureTarget,
	type QuickCaptureMode,
	type QuickCaptureTarget
} from './quick-capture';

export const CAPTURE_PRESETS_STORAGE_KEY = 'mv_quick_capture_presets_v1';

export type CapturePresetShortcut = 'none' | '1' | '2' | '3' | '4' | '5';

export interface CapturePreset {
	id: string;
	name: string;
	mode: QuickCaptureMode;
	target: QuickCaptureTarget;
	prefill: string;
	shortcut: CapturePresetShortcut;
	enabled: boolean;
}

export function isCapturePresetShortcut(value: unknown): value is CapturePresetShortcut {
	return value === 'none' || value === '1' || value === '2' || value === '3' || value === '4' || value === '5';
}

function normalizeCapturePreset(input: Partial<CapturePreset>): CapturePreset | null {
	if (typeof input.id !== 'string' || !input.id.trim()) return null;
	const name = typeof input.name === 'string' && input.name.trim() ? input.name.trim() : 'Untitled preset';
	const mode = isQuickCaptureMode(input.mode) ? input.mode : 'task';
	const target = isQuickCaptureTarget(input.target) ? input.target : 'default';
	const prefill = typeof input.prefill === 'string' ? input.prefill : '';
	const shortcut = isCapturePresetShortcut(input.shortcut) ? input.shortcut : 'none';
	const enabled = input.enabled !== false;
	return {
		id: input.id,
		name,
		mode,
		target,
		prefill,
		shortcut,
		enabled
	};
}

export function loadCapturePresets(): CapturePreset[] {
	if (typeof localStorage === 'undefined') return [];
	const raw = localStorage.getItem(CAPTURE_PRESETS_STORAGE_KEY);
	if (!raw) return [];
	try {
		const parsed = JSON.parse(raw);
		if (!Array.isArray(parsed)) return [];
		const presets = parsed
			.map((entry) => normalizeCapturePreset(entry as Partial<CapturePreset>))
			.filter((entry): entry is CapturePreset => Boolean(entry));
		return presets.slice(0, 20);
	} catch {
		return [];
	}
}

export function saveCapturePresets(presets: CapturePreset[]): void {
	if (typeof localStorage === 'undefined') return;
	localStorage.setItem(CAPTURE_PRESETS_STORAGE_KEY, JSON.stringify(presets));
}

export function createCapturePresetDraft(index = 1): CapturePreset {
	return {
		id: crypto.randomUUID(),
		name: `Preset ${index}`,
		mode: 'task',
		target: 'default',
		prefill: '',
		shortcut: 'none',
		enabled: true
	};
}

export function keyboardDigitShortcut(event: KeyboardEvent): CapturePresetShortcut | null {
	if (!(event.metaKey || event.ctrlKey) || !event.shiftKey) return null;
	const codeMatch = event.code.match(/^Digit([1-5])$/);
	if (codeMatch) {
		return codeMatch[1] as CapturePresetShortcut;
	}
	return isCapturePresetShortcut(event.key) && event.key !== 'none' ? event.key : null;
}

export function findCapturePresetForEvent(
	event: KeyboardEvent,
	presets: CapturePreset[]
): CapturePreset | null {
	const shortcut = keyboardDigitShortcut(event);
	if (!shortcut || shortcut === 'none') return null;
	for (const preset of presets) {
		if (!preset.enabled) continue;
		if (preset.shortcut !== shortcut) continue;
		return preset;
	}
	return null;
}
