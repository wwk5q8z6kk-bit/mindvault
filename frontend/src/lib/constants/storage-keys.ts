/**
 * Centralised localStorage key constants.
 *
 * Prefer importing from here instead of scattering raw strings across modules.
 * Not every key has been migrated yet — new code should always use these constants.
 */
export const STORAGE_KEYS = {
	// Theme & UI
	THEME: 'mindvault.theme',
	VIEW_PREFERENCES: 'mv-view-preferences',
	ONBOARDING: 'mv_onboarding',
	RECENT_ITEMS: 'mv_recent_items',
	SEARCH_MODE: 'mv_search_mode',
	CHAT_SOURCES: 'mv_chat_sources_v1',
	ACTIVE_NAMESPACE: 'mv_active_namespace',
	COMMAND_PALETTE_USAGE: 'mindvault.commandPalette.usage',
	API_HEALTH_STATE: 'mv_api_health_state',

	// AI settings
	AI_PROVIDER: 'mv_ai_provider',
	AI_MODEL: 'mv_ai_model',
	AI_API_KEY: 'mv_ai_api_key',
	AI_BASE_URL: 'mv_ai_base_url',

	// Notifications
	NOTIFICATION_LEAD_MINUTES: 'mv_notification_lead_minutes',
	NOTIFICATION_CHECK_INTERVAL: 'mv_notification_check_interval',
	NOTIFICATION_CLICK_ACTION: 'mv_notification_click_action',

	// Capture
	QUICK_CAPTURE_TARGET: 'mv_quick_capture_target',
	QUICK_CAPTURE_MODE_TARGETS: 'mv_quick_capture_mode_targets_v1',
	CAPTURE_PRESETS: 'mv_quick_capture_presets_v1',
	INBOX_TRIAGE_SETTINGS: 'mv_inbox_triage_settings_v1',

	// Daily / habits
	HABITS: 'mv_habits',
	HABIT_COMPLETIONS: 'mv_habit_completions',

	// Canvas
	CANVAS_GRID: 'mv_canvas_grid',
	CANVAS_SNAP: 'mv_canvas_snap',
	CANVAS_POSITIONS: 'mv_canvas_positions',

	// Bookmarks
	BOOKMARK_FOLDERS: 'mindvault-bookmark-folders',

	// Feature toggles
	FEATURE_VOICE: 'mv_feature_voice',
	FEATURE_NOTIFICATIONS: 'mv_feature_notifications',
	FEATURE_FLASHCARDS: 'mv_feature_flashcards',
	FEATURE_HABITS: 'mv_feature_habits',
	FEATURE_GRAPH: 'mv_feature_graph',
	FEATURE_CANVAS: 'mv_feature_canvas',
	FEATURE_BOOKMARKS: 'mv_feature_bookmarks',
	FEATURE_AUTOTAG: 'mv_feature_autotag',
	FEATURE_CONNECTIONS: 'mv_feature_connections',
	FEATURE_REVIEW: 'mv_feature_review',
} as const;
