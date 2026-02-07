export const INBOX_TRIAGE_SETTINGS_STORAGE_KEY = 'mv_inbox_triage_settings_v1';
export const INBOX_TRIAGE_SETTINGS_UPDATED_EVENT_NAME = 'mindvault:inbox-triage-settings-updated';

export interface InboxTriageSettings {
	auto_run_on_open: boolean;
	default_apply_limit: number;
}

export const DEFAULT_INBOX_TRIAGE_SETTINGS: InboxTriageSettings = {
	auto_run_on_open: false,
	default_apply_limit: 3
};

function clampApplyLimit(value: unknown): number {
	if (typeof value !== 'number' || !Number.isFinite(value)) {
		return DEFAULT_INBOX_TRIAGE_SETTINGS.default_apply_limit;
	}
	const rounded = Math.round(value);
	if (rounded < 1) return 1;
	if (rounded > 10) return 10;
	return rounded;
}

export function normalizeInboxTriageSettings(
	input: Partial<InboxTriageSettings> | null | undefined
): InboxTriageSettings {
	return {
		auto_run_on_open:
			typeof input?.auto_run_on_open === 'boolean'
				? input.auto_run_on_open
				: DEFAULT_INBOX_TRIAGE_SETTINGS.auto_run_on_open,
		default_apply_limit: clampApplyLimit(input?.default_apply_limit)
	};
}

export function loadInboxTriageSettings(): InboxTriageSettings {
	if (typeof localStorage === 'undefined') return DEFAULT_INBOX_TRIAGE_SETTINGS;
	const raw = localStorage.getItem(INBOX_TRIAGE_SETTINGS_STORAGE_KEY);
	if (!raw) return DEFAULT_INBOX_TRIAGE_SETTINGS;
	try {
		const parsed = JSON.parse(raw) as Partial<InboxTriageSettings>;
		return normalizeInboxTriageSettings(parsed);
	} catch {
		return DEFAULT_INBOX_TRIAGE_SETTINGS;
	}
}

export function saveInboxTriageSettings(settings: Partial<InboxTriageSettings>): InboxTriageSettings {
	const normalized = normalizeInboxTriageSettings(settings);
	if (typeof localStorage !== 'undefined') {
		localStorage.setItem(INBOX_TRIAGE_SETTINGS_STORAGE_KEY, JSON.stringify(normalized));
	}
	return normalized;
}
