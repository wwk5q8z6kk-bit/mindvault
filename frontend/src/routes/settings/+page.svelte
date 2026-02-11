<script lang="ts">
	import { themeMode, setTheme, type ThemeMode } from '$lib/stores/theme';
	import { API_BASE_URL } from '$lib/api/client';
	import { pushToast } from '$lib/stores/toast';
	import { db } from '$lib/db';
	import { syncQueue, loadTasks } from '$lib/stores/tasks';
	import { loadNotes } from '$lib/stores/notes';
	import {
		startNotifications,
		stopNotifications,
		resetNotifiedIds,
		getNotificationClickAction,
		type NotificationClickAction
	} from '$lib/stores/notifications';
	import {
		createCapturePresetDraft,
		isCapturePresetShortcut,
		loadCapturePresets,
		saveCapturePresets,
		type CapturePreset,
		type CapturePresetShortcut
	} from '$lib/capture/presets';
	import type { QuickCaptureMode, QuickCaptureTarget } from '$lib/capture/quick-capture';
	import {
		INBOX_TRIAGE_SETTINGS_UPDATED_EVENT_NAME,
		loadInboxTriageSettings,
		saveInboxTriageSettings
	} from '$lib/inbox/triage-settings';
	import ImportExportPanel from '$lib/components/ImportExportPanel.svelte';
	import McpConnectorsPanel from '$lib/components/McpConnectorsPanel.svelte';
	import {
		getSecretStatus,
		setSecret,
		deleteSecret,
		KNOWN_SECRETS,
		type BackendStatus
	} from '$lib/api/secrets';
	import {
		listBlockedSenders,
		addBlockedSender,
		removeBlockedSender,
		listAutoApproveRules,
		addAutoApproveRule,
		updateAutoApproveRule,
		removeAutoApproveRule,
		type BlockedSender,
		type AutoApproveRule
	} from '$lib/api/safeguards';
	import { listAdapterStatuses, type AdapterStatus } from '$lib/api/adapters';
	import { fetchAiModels } from '$lib/api/agent';
	import type { ModelRegistry } from '$lib/api/types';

	let showServerExport = false;

	// Adapter status
	let adapterStatuses: AdapterStatus[] = [];
	let adapterStatusLoading = false;

	// Backend AI model info
	let backendModels: ModelRegistry | null = null;

	const themeOptions: Array<{ value: ThemeMode; label: string; description: string }> = [
		{ value: 'system', label: 'System', description: 'Follow operating system preference' },
		{ value: 'dark', label: 'Dark', description: 'Dark background for low-light environments' },
		{ value: 'light', label: 'Light', description: 'Light background for bright environments' }
	];

	let apiEndpoint = API_BASE_URL;
	let isSyncing = false;
	let isExporting = false;
	let importFileInput: HTMLInputElement | null = null;

	// BYOK / LLM Provider settings
	const AI_PROVIDERS = [
		{
			value: 'default',
			label: 'Server Default',
			description: 'Use the backend configured provider'
		},
		{ value: 'openai', label: 'OpenAI', description: 'GPT-4o, GPT-4, GPT-3.5' },
		{ value: 'anthropic', label: 'Anthropic', description: 'Claude Opus, Sonnet, Haiku' },
		{ value: 'ollama', label: 'Ollama (Local)', description: 'Self-hosted open models' }
	];

	let aiProvider = localStorage.getItem('mv_ai_provider') ?? 'default';
	let aiModel = localStorage.getItem('mv_ai_model') ?? '';
	let aiApiKey = localStorage.getItem('mv_ai_api_key') ?? '';
	let aiBaseUrl = localStorage.getItem('mv_ai_base_url') ?? '';
	let showApiKey = false;

	function saveAiSettings() {
		localStorage.setItem('mv_ai_provider', aiProvider);
		localStorage.setItem('mv_ai_model', aiModel);
		localStorage.setItem('mv_ai_api_key', aiApiKey);
		localStorage.setItem('mv_ai_base_url', aiBaseUrl);
		pushToast('AI settings saved', 'success');
	}

	function clearAiSettings() {
		aiProvider = 'default';
		aiModel = '';
		aiApiKey = '';
		aiBaseUrl = '';
		localStorage.removeItem('mv_ai_provider');
		localStorage.removeItem('mv_ai_model');
		localStorage.removeItem('mv_ai_api_key');
		localStorage.removeItem('mv_ai_base_url');
		pushToast('AI settings cleared', 'success');
	}

	// ── Server Credentials (Keychain) ─────────────────────────────
	let credBackends: BackendStatus[] = [];
	let credLoading = false;
	let credError = '';
	let addSecretKey = '';
	let addSecretValue = '';
	let addSecretBusy = false;
	let showSecretValue = false;
	let deletingKey = '';

	/** All keys that are currently stored across any backend. */
	$: storedKeys = new Set(credBackends.flatMap((b) => b.keys));

	// Blocked senders
	let blockedSenders: BlockedSender[] = [];
	let blockedLoading = false;
	let blockedError = '';
	let addingBlocked = false;
	let removingBlockedId = '';
	let newBlockedType: BlockedSender['sender_type'] = 'relay';
	let newBlockedPattern = '';
	let newBlockedReason = '';
	let newBlockedExpires = '';

	// Auto-approve rules
	let autoApproveRules: AutoApproveRule[] = [];
	let autoApproveLoading = false;
	let autoApproveError = '';
	let addingAutoApprove = false;
	let removingAutoApproveId = '';
	let savingAutoApproveId = '';
	let editingAutoApproveId = '';
	let newAutoApproveName = '';
	let newAutoApproveSender = '';
	let newAutoApproveActions = '';
	let newAutoApproveConfidence = '0.9';
	let editAutoApprove = {
		name: '',
		sender_pattern: '',
		action_types: '',
		min_confidence: '0.9'
	};

	function parseActionTypes(input: string): string[] {
		return input
			.split(',')
			.map((item) => item.trim())
			.filter((item) => item.length > 0);
	}

	function clampConfidence(value: string, fallback: number): number {
		const parsed = Number.parseFloat(value);
		if (Number.isNaN(parsed)) return fallback;
		return Math.min(1, Math.max(0, parsed));
	}

	function formatActionTypes(rule: AutoApproveRule): string {
		return rule.action_types.length > 0 ? rule.action_types.join(', ') : 'any';
	}

	function formatSenderPattern(rule: AutoApproveRule): string {
		return rule.sender_pattern?.trim() || 'any';
	}

	async function loadBlockedSenders() {
		blockedLoading = true;
		blockedError = '';
		try {
			blockedSenders = await listBlockedSenders();
		} catch (e: any) {
			blockedError = e?.message ?? 'Failed to load blocked senders';
		} finally {
			blockedLoading = false;
		}
	}

	async function loadAutoApproveRules() {
		autoApproveLoading = true;
		autoApproveError = '';
		try {
			autoApproveRules = await listAutoApproveRules();
		} catch (e: any) {
			autoApproveError = e?.message ?? 'Failed to load auto-approve rules';
		} finally {
			autoApproveLoading = false;
		}
	}

	async function handleAddAutoApprove() {
		if (!newAutoApproveName.trim()) return;
		addingAutoApprove = true;
		try {
			const actions = parseActionTypes(newAutoApproveActions);
			const payload = {
				name: newAutoApproveName.trim(),
				min_confidence: clampConfidence(newAutoApproveConfidence, 0.9),
				sender_pattern: newAutoApproveSender.trim() || undefined,
				action_types: actions.length > 0 ? actions : undefined
			};
			const created = await addAutoApproveRule(payload);
			autoApproveRules = [created, ...autoApproveRules];
			newAutoApproveName = '';
			newAutoApproveSender = '';
			newAutoApproveActions = '';
			newAutoApproveConfidence = '0.9';
			pushToast('Auto-approve rule added', 'success');
		} catch (e: any) {
			pushToast(e?.message ?? 'Failed to add auto-approve rule', 'danger');
		} finally {
			addingAutoApprove = false;
		}
	}

	async function handleToggleAutoApprove(rule: AutoApproveRule) {
		savingAutoApproveId = rule.id;
		try {
			const updated = await updateAutoApproveRule(rule.id, { enabled: !rule.enabled });
			autoApproveRules = autoApproveRules.map((item) => (item.id === rule.id ? updated : item));
		} catch (e: any) {
			pushToast(e?.message ?? 'Failed to update rule', 'danger');
		} finally {
			savingAutoApproveId = '';
		}
	}

	function startEditAutoApprove(rule: AutoApproveRule) {
		editingAutoApproveId = rule.id;
		editAutoApprove = {
			name: rule.name,
			sender_pattern: rule.sender_pattern ?? '',
			action_types: rule.action_types.join(', '),
			min_confidence: rule.min_confidence.toString()
		};
	}

	function cancelEditAutoApprove() {
		editingAutoApproveId = '';
	}

	async function handleSaveAutoApprove(rule: AutoApproveRule) {
		if (!editAutoApprove.name.trim()) {
			pushToast('Rule name is required', 'warning');
			return;
		}
		savingAutoApproveId = rule.id;
		try {
			const updated = await updateAutoApproveRule(rule.id, {
				name: editAutoApprove.name.trim(),
				sender_pattern: editAutoApprove.sender_pattern.trim()
					? editAutoApprove.sender_pattern.trim()
					: null,
				action_types: parseActionTypes(editAutoApprove.action_types),
				min_confidence: clampConfidence(editAutoApprove.min_confidence, rule.min_confidence)
			});
			autoApproveRules = autoApproveRules.map((item) => (item.id === updated.id ? updated : item));
			editingAutoApproveId = '';
			pushToast('Rule updated', 'success');
		} catch (e: any) {
			pushToast(e?.message ?? 'Failed to update rule', 'danger');
		} finally {
			savingAutoApproveId = '';
		}
	}

	async function handleRemoveAutoApprove(id: string) {
		removingAutoApproveId = id;
		try {
			await removeAutoApproveRule(id);
			autoApproveRules = autoApproveRules.filter((item) => item.id !== id);
			pushToast('Rule removed', 'success');
		} catch (e: any) {
			pushToast(e?.message ?? 'Failed to remove rule', 'danger');
		} finally {
			removingAutoApproveId = '';
		}
	}

	async function handleAddBlocked() {
		if (!newBlockedPattern.trim()) return;
		addingBlocked = true;
		try {
			const expiresAt = newBlockedExpires
				? new Date(newBlockedExpires).toISOString()
				: undefined;
			const created = await addBlockedSender({
				sender_type: newBlockedType,
				sender_pattern: newBlockedPattern.trim(),
				reason: newBlockedReason.trim() || undefined,
				expires_at: expiresAt
			});
			blockedSenders = [created, ...blockedSenders];
			newBlockedPattern = '';
			newBlockedReason = '';
			newBlockedExpires = '';
			pushToast('Sender blocked', 'success');
		} catch (e: any) {
			pushToast(e?.message ?? 'Failed to block sender', 'danger');
		} finally {
			addingBlocked = false;
		}
	}

	async function handleRemoveBlocked(id: string) {
		removingBlockedId = id;
		try {
			await removeBlockedSender(id);
			blockedSenders = blockedSenders.filter((item) => item.id !== id);
			pushToast('Sender unblocked', 'success');
		} catch (e: any) {
			pushToast(e?.message ?? 'Failed to remove block', 'danger');
		} finally {
			removingBlockedId = '';
		}
	}

	async function loadCredentials() {
		credLoading = true;
		credError = '';
		try {
			const res = await getSecretStatus();
			credBackends = res.backends;
		} catch (e: any) {
			credError = e?.message ?? 'Failed to load credential status';
		} finally {
			credLoading = false;
		}
	}

	async function handleAddSecret() {
		if (!addSecretKey || !addSecretValue) return;
		addSecretBusy = true;
		try {
			const res = await setSecret(addSecretKey, addSecretValue);
			pushToast(`${res.key} stored in ${res.stored_in}`, 'success');
			addSecretKey = '';
			addSecretValue = '';
			showSecretValue = false;
			await loadCredentials();
		} catch (e: any) {
			pushToast(e?.message ?? 'Failed to store secret', 'danger');
		} finally {
			addSecretBusy = false;
		}
	}

	async function handleDeleteSecret(key: string) {
		deletingKey = key;
		try {
			const res = await deleteSecret(key);
			const where = res.deleted_from.join(', ') || 'nowhere';
			pushToast(`${key} deleted from ${where}`, 'success');
			await loadCredentials();
		} catch (e: any) {
			pushToast(e?.message ?? 'Failed to delete secret', 'danger');
		} finally {
			deletingKey = '';
		}
	}

	// Load credentials on mount
	loadCredentials();

	// Load adapter statuses
	async function loadAdapterStatuses() {
		adapterStatusLoading = true;
		try {
			adapterStatuses = await listAdapterStatuses();
		} catch {
			// Adapters may not be configured — fail silently
		} finally {
			adapterStatusLoading = false;
		}
	}
	loadAdapterStatuses();

	// Load backend AI model info
	async function loadBackendModels() {
		try {
			backendModels = await fetchAiModels();
		} catch {
			// Not critical — fail silently
		}
	}
	loadBackendModels();

	// Load blocked senders on mount
	loadBlockedSenders();
	loadAutoApproveRules();

	function handleThemeChange(mode: ThemeMode) {
		setTheme(mode);
		pushToast(`Theme: ${mode}`, 'success');
	}

	async function forceSync() {
		isSyncing = true;
		try {
			await syncQueue();
			await Promise.all([loadTasks(), loadNotes()]);
			pushToast('Sync complete', 'success');
		} catch {
			pushToast('Sync failed', 'danger');
		} finally {
			isSyncing = false;
		}
	}

	async function exportData() {
		isExporting = true;
		try {
			const tasks = await db.tasks.toArray();
			const notes = await db.notes.toArray();
			const queue = await db.queue.toArray();
			const payload = {
				version: 1,
				exported_at: new Date().toISOString(),
				tasks,
				notes,
				queue
			};
			const blob = new Blob([JSON.stringify(payload, null, 2)], { type: 'application/json' });
			const url = URL.createObjectURL(blob);
			const a = document.createElement('a');
			a.href = url;
			a.download = `mindvault-export-${new Date().toISOString().slice(0, 10)}.json`;
			document.body.appendChild(a);
			a.click();
			document.body.removeChild(a);
			URL.revokeObjectURL(url);
			pushToast('Data exported', 'success');
		} catch {
			pushToast('Export failed', 'danger');
		} finally {
			isExporting = false;
		}
	}

	async function handleImport(event: Event) {
		const input = event.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		if (!file) return;

		try {
			const text = await file.text();
			const data = JSON.parse(text);
			if (!data.version || !Array.isArray(data.tasks) || !Array.isArray(data.notes)) {
				pushToast('Invalid export file format', 'danger');
				return;
			}
			let tasksImported = 0;
			let notesImported = 0;

			for (const task of data.tasks) {
				if (task.id) {
					await db.tasks.put(task);
					tasksImported++;
				}
			}
			for (const note of data.notes) {
				if (note.id) {
					await db.notes.put(note);
					notesImported++;
				}
			}

			await Promise.all([loadTasks(), loadNotes()]);
			pushToast(`Imported ${tasksImported} tasks, ${notesImported} notes`, 'success');
		} catch {
			pushToast('Import failed — check file format', 'danger');
		} finally {
			if (importFileInput) importFileInput.value = '';
		}
	}

	let isExportingMd = false;

	async function exportMarkdown() {
		isExportingMd = true;
		try {
			const notes = await db.notes.toArray();
			if (notes.length === 0) {
				pushToast('No notes to export', 'warning');
				return;
			}
			// Build a combined markdown file (one per note, separated by ---)
			const parts = notes.map((note) => {
				const frontmatter = [
					'---',
					`title: "${(note.title ?? 'Untitled').replace(/"/g, '\\"')}"`,
					`id: ${note.id}`,
					`created: ${note.created_at}`,
					`updated: ${note.updated_at}`,
					note.tags?.length ? `tags: [${note.tags.join(', ')}]` : null,
					note.pinned ? 'pinned: true' : null,
					'---'
				]
					.filter(Boolean)
					.join('\n');
				return `${frontmatter}\n\n# ${note.title ?? 'Untitled'}\n\n${note.markdown ?? ''}`;
			});

			const content = parts.join('\n\n---\n\n');
			const blob = new Blob([content], { type: 'text/markdown' });
			const url = URL.createObjectURL(blob);
			const a = document.createElement('a');
			a.href = url;
			a.download = `mindvault-notes-${new Date().toISOString().slice(0, 10)}.md`;
			document.body.appendChild(a);
			a.click();
			document.body.removeChild(a);
			URL.revokeObjectURL(url);
			pushToast(`Exported ${notes.length} notes as Markdown`, 'success');
		} catch {
			pushToast('Markdown export failed', 'danger');
		} finally {
			isExportingMd = false;
		}
	}

	// Feature toggles
	type FeatureToggle = {
		key: string;
		label: string;
		description: string;
		default: boolean;
	};

	const FEATURE_TOGGLES: FeatureToggle[] = [
		{
			key: 'mv_feature_voice',
			label: 'Voice Notes',
			description: 'Record audio in Quick Capture',
			default: true
		},
		{
			key: 'mv_feature_notifications',
			label: 'Task Reminders',
			description: 'Browser notifications for due/overdue tasks',
			default: true
		},
		{
			key: 'mv_feature_flashcards',
			label: 'Flashcards',
			description: 'Spaced repetition learning cards',
			default: true
		},
		{
			key: 'mv_feature_habits',
			label: 'Habit Tracking',
			description: 'Daily habit checklist on Daily Notes page',
			default: true
		},
		{
			key: 'mv_feature_graph',
			label: 'Knowledge Graph',
			description: 'Visual node graph explorer',
			default: true
		},
		{
			key: 'mv_feature_canvas',
			label: 'Canvas / Mind Map',
			description: 'Freeform spatial workspace',
			default: true
		},
		{
			key: 'mv_feature_bookmarks',
			label: 'Reading List',
			description: 'Web clip and reference management',
			default: true
		},
		{
			key: 'mv_feature_autotag',
			label: 'AI Auto-Tagging',
			description: 'Suggest tags for notes using AI',
			default: true
		},
		{
			key: 'mv_feature_connections',
			label: 'Suggested Connections',
			description: 'AI-powered similar item discovery',
			default: true
		},
		{
			key: 'mv_feature_review',
			label: 'Proactive Review',
			description: 'Weekly AI digest and review prompts',
			default: true
		}
	];

	function isFeatureEnabled(key: string, defaultVal: boolean): boolean {
		const stored = localStorage.getItem(key);
		if (stored === null) return defaultVal;
		return stored !== 'false';
	}

	function toggleFeature(key: string, defaultVal: boolean) {
		const current = isFeatureEnabled(key, defaultVal);
		localStorage.setItem(key, String(!current));
		// Re-trigger reactivity
		featureStates = FEATURE_TOGGLES.map((f) => ({
			...f,
			enabled: isFeatureEnabled(f.key, f.default)
		}));
		// Handle notification toggle specially
		if (key === 'mv_feature_notifications') {
			if (!current) {
				startNotifications();
			} else {
				stopNotifications();
			}
		}
		pushToast(`${!current ? 'Enabled' : 'Disabled'} feature`, 'success');
	}

	let featureStates = FEATURE_TOGGLES.map((f) => ({
		...f,
		enabled: isFeatureEnabled(f.key, f.default)
	}));

	// Notification settings
	let notifLeadMinutes = localStorage.getItem('mv_notification_lead_minutes') ?? '30';
	let notifCheckInterval = localStorage.getItem('mv_notification_check_interval') ?? '60000';
	let notifClickAction: NotificationClickAction = getNotificationClickAction();
	let capturePresets: CapturePreset[] = loadCapturePresets();
	let inboxTriageSettings = loadInboxTriageSettings();
	let inboxTriageAutoRunOnOpen = inboxTriageSettings.auto_run_on_open;
	let inboxTriageDefaultApplyLimit = String(inboxTriageSettings.default_apply_limit);

	const captureModeOptions: Array<{ value: QuickCaptureMode; label: string }> = [
		{ value: 'task', label: 'Task' },
		{ value: 'note', label: 'Note' },
		{ value: 'link', label: 'Link' },
		{ value: 'voice', label: 'Voice' }
	];
	const captureTargetOptions: Array<{ value: QuickCaptureTarget; label: string }> = [
		{ value: 'default', label: 'Default' },
		{ value: 'inbox', label: 'Inbox' },
		{ value: 'daily', label: 'Daily note' },
		{ value: 'planned', label: 'Planned' },
		{ value: 'review', label: 'Review' }
	];
	const captureShortcutOptions: CapturePresetShortcut[] = ['none', '1', '2', '3', '4', '5'];

	function addCapturePreset() {
		capturePresets = [...capturePresets, createCapturePresetDraft(capturePresets.length + 1)];
	}

	function removeCapturePreset(id: string) {
		capturePresets = capturePresets.filter((preset) => preset.id !== id);
	}

	function updateCapturePreset(id: string, patch: Partial<CapturePreset>) {
		capturePresets = capturePresets.map((preset) => (preset.id === id ? { ...preset, ...patch } : preset));
	}

	function saveCapturePresetSettings() {
		const seenShortcuts = new Set<string>();
		for (const preset of capturePresets) {
			if (!preset.enabled || preset.shortcut === 'none') continue;
			if (seenShortcuts.has(preset.shortcut)) {
				pushToast(`Shortcut ${preset.shortcut} is assigned more than once`, 'danger');
				return;
			}
			seenShortcuts.add(preset.shortcut);
		}

		const normalized = capturePresets.map((preset) => ({
			...preset,
			name: preset.name.trim() || 'Untitled preset',
			prefill: preset.prefill.trim(),
			shortcut: isCapturePresetShortcut(preset.shortcut) ? preset.shortcut : 'none'
		}));
		saveCapturePresets(normalized);
		capturePresets = normalized;
		window.dispatchEvent(new CustomEvent('mindvault:capture-presets-updated'));
		pushToast('Quick capture presets saved', 'success');
	}

	function saveNotificationSettings() {
		localStorage.setItem('mv_notification_lead_minutes', notifLeadMinutes);
		localStorage.setItem('mv_notification_check_interval', notifCheckInterval);
		localStorage.setItem('mv_notification_click_action', notifClickAction);
		resetNotifiedIds();
		stopNotifications();
		startNotifications();
		pushToast('Notification settings saved', 'success');
	}

	function saveInboxTriagePreferences() {
		const parsedLimit = Number.parseInt(inboxTriageDefaultApplyLimit, 10);
		const normalized = saveInboxTriageSettings({
			auto_run_on_open: inboxTriageAutoRunOnOpen,
			default_apply_limit: Number.isFinite(parsedLimit)
				? parsedLimit
				: inboxTriageSettings.default_apply_limit
		});
		inboxTriageSettings = normalized;
		inboxTriageAutoRunOnOpen = normalized.auto_run_on_open;
		inboxTriageDefaultApplyLimit = String(normalized.default_apply_limit);
		window.dispatchEvent(new CustomEvent(INBOX_TRIAGE_SETTINGS_UPDATED_EVENT_NAME));
		pushToast('Inbox AI triage settings saved', 'success');
	}

	function clearLocalData() {
		if (
			confirm('This will clear all local cached data. Data on the server will not be affected.')
		) {
			localStorage.clear();
			indexedDB.deleteDatabase('mindvault');
			pushToast('Local data cleared. Reload to re-sync.', 'info');
		}
	}
</script>

<div class="mx-auto max-w-2xl">
	<h2 class="text-lg font-semibold text-white">Settings</h2>
	<p class="mt-1 text-xs text-slate-400">Configure MindVault preferences.</p>

	<div class="mt-6 flex flex-col gap-6">
		<!-- Theme -->
		<section class="rounded-xl border border-slate-800 bg-slate-900/40 p-5">
			<h3 class="text-sm font-semibold text-white">Appearance</h3>
			<p class="mt-1 text-[11px] text-slate-400">Choose your preferred color scheme.</p>
			<div class="mt-3 flex gap-2">
				{#each themeOptions as option (option.value)}
					<button
						class={`flex-1 rounded-lg border px-3 py-3 text-left transition ${
							$themeMode === option.value
								? 'border-sky-500/60 bg-sky-500/10'
								: 'border-slate-800 hover:border-slate-600'
						}`}
						on:click={() => handleThemeChange(option.value)}
					>
						<div class="text-xs font-semibold text-white">{option.label}</div>
						<div class="mt-0.5 text-[10px] text-slate-400">{option.description}</div>
					</button>
				{/each}
			</div>
		</section>

		<!-- Connection -->
		<section class="rounded-xl border border-slate-800 bg-slate-900/40 p-5">
			<h3 class="text-sm font-semibold text-white">Connection</h3>
			<p class="mt-1 text-[11px] text-slate-400">Backend API endpoint for data sync.</p>
			<div class="mt-3">
				<input
					class="w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white outline-none focus:border-sky-500"
					value={apiEndpoint}
					readonly
				/>
				<p class="mt-1 text-[10px] text-slate-500">
					Set via VITE_API_BASE_URL environment variable.
				</p>
			</div>
			<div class="mt-3">
				<button
					class="rounded-lg border border-sky-500/30 bg-sky-500/10 px-3 py-2 text-xs text-sky-300 transition hover:bg-sky-500/20 disabled:opacity-50"
					on:click={forceSync}
					disabled={isSyncing}
				>
					{isSyncing ? 'Syncing...' : 'Force Sync Now'}
				</button>
				<p class="mt-1 text-[10px] text-slate-500">
					Flush offline queue and re-fetch all data from server.
				</p>
			</div>
		</section>

			<!-- Connected Adapters -->
			<section class="rounded-xl border border-slate-800 bg-slate-900/40 p-5">
			<div class="flex items-center justify-between">
				<div>
					<h3 class="text-sm font-semibold text-white">Connected Adapters</h3>
					<p class="mt-1 text-[11px] text-slate-400">External integrations (Slack, email, webhooks, etc.)</p>
				</div>
				<button
					class="rounded-lg border border-slate-700 px-2 py-1 text-[10px] text-slate-300 hover:bg-slate-800"
					on:click={loadAdapterStatuses}
					disabled={adapterStatusLoading}
				>
					{adapterStatusLoading ? 'Checking...' : 'Refresh'}
				</button>
			</div>
			{#if adapterStatusLoading && adapterStatuses.length === 0}
				<div class="mt-3 text-xs text-slate-500">Checking adapter status...</div>
			{:else if adapterStatuses.length === 0}
				<div class="mt-3 rounded-lg border border-dashed border-slate-800 p-3 text-center text-[11px] text-slate-500">
					No adapters configured. Adapters connect MindVault to external services.
				</div>
			{:else}
				<div class="mt-3 space-y-2">
					{#each adapterStatuses as adapter (adapter.name)}
						<div class="flex items-center gap-3 rounded-lg border border-slate-800 bg-slate-900/60 px-3 py-2">
							<div class="h-2 w-2 rounded-full {adapter.connected ? 'bg-emerald-400' : 'bg-red-400'}"></div>
							<div class="flex-1 min-w-0">
								<div class="flex items-center gap-2">
									<span class="text-xs font-medium text-white">{adapter.name}</span>
									<span class="rounded bg-slate-800 px-1.5 py-0.5 text-[9px] text-slate-400">{adapter.adapter_type}</span>
								</div>
								{#if adapter.error}
									<p class="mt-0.5 text-[10px] text-red-400 truncate">{adapter.error}</p>
								{:else if adapter.last_receive}
									<p class="mt-0.5 text-[10px] text-slate-500">
										Last activity: {new Date(adapter.last_receive).toLocaleString()}
									</p>
								{/if}
							</div>
							<span class="text-[10px] {adapter.connected ? 'text-emerald-400' : 'text-red-400'}">
								{adapter.connected ? 'Connected' : 'Disconnected'}
							</span>
						</div>
					{/each}
				</div>
			{/if}
			</section>

			<!-- MCP Connectors -->
			<McpConnectorsPanel />

		<!-- AI Provider (BYOK) -->
		<section class="rounded-xl border border-slate-800 bg-slate-900/40 p-5">
			<h3 class="text-sm font-semibold text-white">AI Provider</h3>
			<p class="mt-1 text-[11px] text-slate-400">
				Bring your own API key or use a local LLM. Leave as "Server Default" to use the backend's
				configured provider.
			</p>

			{#if backendModels}
				<div class="mt-3 rounded-lg border border-sky-500/20 bg-sky-500/5 px-3 py-2">
					<div class="text-[10px] uppercase tracking-wider text-sky-400">Server Embedding Model</div>
					<div class="mt-1 flex items-center gap-2 text-xs text-white">
						<span class="font-medium">{backendModels.embedding.provider}</span>
						<span class="text-slate-500">/</span>
						<span class="font-mono text-slate-300">{backendModels.embedding.model}</span>
					</div>
				</div>
			{/if}

			<div class="mt-3 grid grid-cols-2 gap-2">
				{#each AI_PROVIDERS as provider (provider.value)}
					<button
						class={`rounded-lg border px-3 py-2.5 text-left transition ${
							aiProvider === provider.value
								? 'border-sky-500/60 bg-sky-500/10'
								: 'border-slate-800 hover:border-slate-600'
						}`}
						on:click={() => {
							aiProvider = provider.value;
						}}
					>
						<div class="text-xs font-semibold text-white">{provider.label}</div>
						<div class="mt-0.5 text-[10px] text-slate-400">{provider.description}</div>
					</button>
				{/each}
			</div>

			{#if aiProvider !== 'default'}
				<div class="mt-3 flex flex-col gap-3">
					<div>
						<label class="text-[10px] uppercase tracking-wider text-slate-500" for="ai-model"
							>Model</label
						>
						<input
							id="ai-model"
							class="mt-1 w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white outline-none focus:border-sky-500"
							placeholder={aiProvider === 'openai'
								? 'gpt-4o'
								: aiProvider === 'anthropic'
									? 'claude-sonnet-4-5-20250929'
									: 'llama3.1'}
							bind:value={aiModel}
						/>
					</div>
					<div>
						<label class="text-[10px] uppercase tracking-wider text-slate-500" for="ai-key"
							>API Key</label
						>
						<div class="relative mt-1">
							<input
								id="ai-key"
								class="w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 pr-16 text-xs text-white outline-none focus:border-sky-500"
								type={showApiKey ? 'text' : 'password'}
								placeholder="sk-..."
								bind:value={aiApiKey}
							/>
							<button
								class="absolute right-2 top-1/2 -translate-y-1/2 text-[10px] text-slate-400 hover:text-white"
								on:click={() => {
									showApiKey = !showApiKey;
								}}
							>
								{showApiKey ? 'Hide' : 'Show'}
							</button>
						</div>
					</div>
					{#if aiProvider === 'ollama'}
						<div>
							<label class="text-[10px] uppercase tracking-wider text-slate-500" for="ai-url"
								>Base URL</label
							>
							<input
								id="ai-url"
								class="mt-1 w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white outline-none focus:border-sky-500"
								placeholder="http://localhost:11434"
								bind:value={aiBaseUrl}
							/>
						</div>
					{/if}
				</div>
			{/if}

			<div class="mt-3 flex gap-2">
				<button
					class="rounded-lg bg-sky-500 px-3 py-2 text-xs font-semibold text-white hover:bg-sky-400"
					on:click={saveAiSettings}
				>
					Save AI settings
				</button>
				{#if aiProvider !== 'default'}
					<button
						class="rounded-lg border border-slate-700 px-3 py-2 text-xs text-slate-300 hover:bg-slate-800"
						on:click={clearAiSettings}
					>
						Reset to default
					</button>
				{/if}
			</div>
		</section>

		<!-- Owner Profile -->
		<section class="rounded-xl border border-slate-800 bg-slate-900/40 p-5">
			<h3 class="text-sm font-semibold text-white">Owner Profile</h3>
			<p class="mt-1 text-[11px] text-slate-400">
				Your identity for signing nodes, email headers, and federation.
			</p>
			<a href="/settings/profile" class="mt-1 inline-block text-xs text-sky-400 hover:text-sky-300"
				>Edit Profile &rarr;</a
			>
		</section>

		<!-- Server Credentials (Keychain) -->
		<section class="rounded-xl border border-slate-800 bg-slate-900/40 p-5">
			<div class="flex items-center justify-between">
				<div>
					<h3 class="text-sm font-semibold text-white">Server Credentials</h3>
					<p class="mt-1 text-[11px] text-slate-400">
						API keys and secrets stored securely via the server's credential backends (OS
						Keychain, environment variables).
					</p>
					<a href="/settings/keychain" class="mt-1 inline-block text-xs text-sky-400 hover:text-sky-300"
						>Manage Sovereign Keychain &rarr;</a
					>
				</div>
				<button
					class="rounded-lg border border-slate-700 px-2.5 py-1.5 text-[10px] text-slate-300 hover:bg-slate-800"
					on:click={loadCredentials}
					disabled={credLoading}
				>
					{credLoading ? 'Loading...' : 'Refresh'}
				</button>
			</div>

			{#if credError}
				<div
					class="mt-3 rounded-lg border border-red-500/30 bg-red-500/10 px-3 py-2 text-xs text-red-300"
				>
					{credError}
				</div>
			{/if}

			<!-- Backend status table -->
			{#if credBackends.length > 0}
				<div class="mt-3 space-y-2">
					{#each credBackends as backend (backend.name)}
						<div class="rounded-lg border border-slate-800/60 px-3 py-2.5">
							<div class="flex items-center gap-2">
								<span
									class={`h-2 w-2 rounded-full ${backend.available ? 'bg-emerald-400' : 'bg-slate-600'}`}
								></span>
								<span class="text-xs font-medium text-white">{backend.name}</span>
								<span class="text-[10px] text-slate-500"
									>{backend.available ? 'available' : 'unavailable'}</span
								>
							</div>
							{#if backend.keys.length > 0}
								<div class="mt-2 flex flex-wrap gap-1.5">
									{#each backend.keys as key (key)}
										<span
											class="inline-flex items-center gap-1 rounded-md border border-slate-700 bg-slate-800 px-2 py-0.5 text-[10px] text-slate-300"
										>
											{key}
											<button
												class="ml-0.5 text-slate-500 hover:text-red-400"
												title="Delete {key}"
												disabled={deletingKey === key}
												on:click={() => handleDeleteSecret(key)}
											>
												{deletingKey === key ? '...' : '×'}
											</button>
										</span>
									{/each}
								</div>
							{/if}
						</div>
					{/each}
				</div>
			{/if}

			<!-- Known secrets quick-add -->
			<div class="mt-4">
				<h4 class="text-[10px] uppercase tracking-wider text-slate-500">Add Secret</h4>
				<div class="mt-2 flex flex-wrap gap-1.5">
					{#each KNOWN_SECRETS as secret (secret.key)}
						<button
							class={`rounded-md border px-2 py-1 text-[10px] transition ${
								storedKeys.has(secret.key)
									? 'border-emerald-500/30 bg-emerald-500/10 text-emerald-300'
									: addSecretKey === secret.key
										? 'border-sky-500/60 bg-sky-500/10 text-sky-300'
										: 'border-slate-700 text-slate-400 hover:border-slate-500'
							}`}
							title={secret.description}
							disabled={storedKeys.has(secret.key)}
							on:click={() => {
								addSecretKey = secret.key;
							}}
						>
							{secret.label}
							{#if secret.required}
								<span class="text-amber-400">*</span>
							{/if}
							{#if storedKeys.has(secret.key)}
								<span class="ml-0.5">&#10003;</span>
							{/if}
						</button>
					{/each}
				</div>

				<div class="mt-3 flex gap-2">
					<input
						class="flex-1 rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white outline-none focus:border-sky-500"
						placeholder="Key name (e.g. OPENAI_API_KEY)"
						bind:value={addSecretKey}
					/>
					<div class="relative flex-1">
						<input
							class="w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 pr-14 text-xs text-white outline-none focus:border-sky-500"
							type={showSecretValue ? 'text' : 'password'}
							placeholder="Secret value"
							bind:value={addSecretValue}
						/>
						<button
							class="absolute right-2 top-1/2 -translate-y-1/2 text-[10px] text-slate-400 hover:text-white"
							on:click={() => {
								showSecretValue = !showSecretValue;
							}}
						>
							{showSecretValue ? 'Hide' : 'Show'}
						</button>
					</div>
					<button
						class="rounded-lg bg-sky-500 px-3 py-2 text-xs font-semibold text-white hover:bg-sky-400 disabled:opacity-50"
						disabled={!addSecretKey || !addSecretValue || addSecretBusy}
						on:click={handleAddSecret}
					>
						{addSecretBusy ? 'Storing...' : 'Store'}
					</button>
				</div>
			</div>
		</section>

		<!-- Blocked Senders -->
		<section class="rounded-xl border border-slate-800 bg-slate-900/40 p-5">
			<div class="flex items-center justify-between">
				<div>
					<h3 class="text-sm font-semibold text-white">Blocked Senders</h3>
					<p class="mt-1 text-[11px] text-slate-400">
						Block inbound relay messages or proposal senders by pattern (glob match).
					</p>
				</div>
				<button
					class="rounded-lg border border-slate-700 px-2.5 py-1.5 text-[10px] text-slate-300 hover:bg-slate-800"
					on:click={loadBlockedSenders}
					disabled={blockedLoading}
				>
					{blockedLoading ? 'Loading...' : 'Refresh'}
				</button>
			</div>

			{#if blockedError}
				<div
					class="mt-3 rounded-lg border border-red-500/30 bg-red-500/10 px-3 py-2 text-xs text-red-300"
				>
					{blockedError}
				</div>
			{/if}

			<div class="mt-3 space-y-2">
				{#if blockedSenders.length === 0}
					<p class="text-xs text-slate-500">No blocked senders configured.</p>
				{:else}
					{#each blockedSenders as sender (sender.id)}
						<div class="rounded-lg border border-slate-800/60 px-3 py-2.5">
							<div class="flex items-center justify-between gap-2">
								<div class="min-w-0">
									<div class="flex items-center gap-2">
										<span class="rounded bg-slate-800 px-1.5 py-0.5 text-[10px] text-slate-300">
											{sender.sender_type}
										</span>
										<span class="truncate text-xs text-white">{sender.sender_pattern}</span>
									</div>
									<div class="mt-1 text-[10px] text-slate-500">
										{#if sender.reason}
											Reason: {sender.reason}
										{:else}
											No reason specified
										{/if}
										{#if sender.expires_at}
											&nbsp;· Expires {new Date(sender.expires_at).toLocaleString()}
										{/if}
									</div>
								</div>
								<button
									class="rounded-md border border-slate-700 px-2 py-1 text-[10px] text-slate-300 hover:bg-slate-800 disabled:opacity-50"
									on:click={() => handleRemoveBlocked(sender.id)}
									disabled={removingBlockedId === sender.id}
								>
									{removingBlockedId === sender.id ? 'Removing...' : 'Remove'}
								</button>
							</div>
						</div>
					{/each}
				{/if}
			</div>

			<div class="mt-4 border-t border-slate-800/60 pt-4">
				<h4 class="text-[10px] uppercase tracking-wider text-slate-500">Add Block</h4>
				<div class="mt-2 grid grid-cols-1 gap-3 md:grid-cols-2">
					<div>
						<label class="text-[10px] uppercase tracking-wider text-slate-500" for="blocked-type"
							>Sender Type</label
						>
						<select
							id="blocked-type"
							class="mt-1 w-full rounded border border-slate-700 bg-slate-900 px-2 py-1 text-xs text-slate-200"
							bind:value={newBlockedType}
						>
							<option value="relay">Relay</option>
							<option value="agent">Agent</option>
							<option value="mcp">MCP</option>
							<option value="webhook">Webhook</option>
							<option value="watcher">Watcher</option>
						</select>
					</div>
					<div>
						<label class="text-[10px] uppercase tracking-wider text-slate-500" for="blocked-pattern"
							>Pattern</label
						>
						<input
							id="blocked-pattern"
							class="mt-1 w-full rounded border border-slate-700 bg-slate-900 px-2 py-1 text-xs text-slate-200"
							placeholder="e.g. alice@example.com or *spam*"
							bind:value={newBlockedPattern}
						/>
					</div>
					<div>
						<label class="text-[10px] uppercase tracking-wider text-slate-500" for="blocked-reason"
							>Reason (optional)</label
						>
						<input
							id="blocked-reason"
							class="mt-1 w-full rounded border border-slate-700 bg-slate-900 px-2 py-1 text-xs text-slate-200"
							placeholder="Why this sender is blocked"
							bind:value={newBlockedReason}
						/>
					</div>
					<div>
						<label class="text-[10px] uppercase tracking-wider text-slate-500" for="blocked-expires"
							>Expires (optional)</label
						>
						<input
							id="blocked-expires"
							type="datetime-local"
							class="mt-1 w-full rounded border border-slate-700 bg-slate-900 px-2 py-1 text-xs text-slate-200"
							bind:value={newBlockedExpires}
						/>
					</div>
				</div>
				<button
					class="mt-3 rounded-lg bg-sky-500 px-3 py-2 text-xs font-semibold text-white hover:bg-sky-400 disabled:opacity-50"
					on:click={handleAddBlocked}
					disabled={addingBlocked || !newBlockedPattern.trim()}
				>
					{addingBlocked ? 'Blocking...' : 'Block Sender'}
				</button>
			</div>
		</section>

		<!-- Auto-Approve Rules -->
		<section class="rounded-xl border border-slate-800 bg-slate-900/40 p-5">
			<div class="flex items-center justify-between">
				<div>
					<h3 class="text-sm font-semibold text-white">Auto-Approve Rules</h3>
					<p class="mt-1 text-[11px] text-slate-400">
						Automatically approve matching proposals. Leave sender/actions empty to match any.
					</p>
				</div>
				<button
					class="rounded-lg border border-slate-700 px-2.5 py-1.5 text-[10px] text-slate-300 hover:bg-slate-800"
					on:click={loadAutoApproveRules}
					disabled={autoApproveLoading}
				>
					{autoApproveLoading ? 'Loading...' : 'Refresh'}
				</button>
			</div>

			{#if autoApproveError}
				<div
					class="mt-3 rounded-lg border border-red-500/30 bg-red-500/10 px-3 py-2 text-xs text-red-300"
				>
					{autoApproveError}
				</div>
			{/if}

			<div class="mt-3 space-y-2">
				{#if autoApproveRules.length === 0}
					<p class="text-xs text-slate-500">No auto-approve rules configured.</p>
				{:else}
					{#each autoApproveRules as rule (rule.id)}
						<div class="rounded-lg border border-slate-800/60 px-3 py-2.5">
							<div class="flex items-center justify-between gap-3">
								<div class="min-w-0">
									<div class="flex items-center gap-2">
										<span class="truncate text-xs font-semibold text-white">{rule.name}</span>
										<span
											class="rounded px-1.5 py-0.5 text-[10px] {rule.enabled
												? 'bg-emerald-500/15 text-emerald-300'
												: 'bg-slate-700 text-slate-400'}"
										>
											{rule.enabled ? 'enabled' : 'disabled'}
										</span>
									</div>
									<div class="mt-1 text-[10px] text-slate-500">
										Sender: {formatSenderPattern(rule)} · Actions: {formatActionTypes(rule)} · Min
										conf: {(rule.min_confidence * 100).toFixed(0)}%
									</div>
								</div>
								<div class="flex items-center gap-2">
									<button
										class="rounded-md border border-slate-700 px-2 py-1 text-[10px] text-slate-300 hover:bg-slate-800 disabled:opacity-50"
										on:click={() => handleToggleAutoApprove(rule)}
										disabled={savingAutoApproveId === rule.id}
									>
										{rule.enabled ? 'Disable' : 'Enable'}
									</button>
									<button
										class="rounded-md border border-slate-700 px-2 py-1 text-[10px] text-slate-300 hover:bg-slate-800"
										on:click={() => startEditAutoApprove(rule)}
									>
										Edit
									</button>
									<button
										class="rounded-md border border-slate-700 px-2 py-1 text-[10px] text-slate-300 hover:bg-slate-800 disabled:opacity-50"
										on:click={() => handleRemoveAutoApprove(rule.id)}
										disabled={removingAutoApproveId === rule.id}
									>
										{removingAutoApproveId === rule.id ? 'Removing...' : 'Remove'}
									</button>
								</div>
							</div>

							{#if editingAutoApproveId === rule.id}
								<div class="mt-3 grid grid-cols-1 gap-3 md:grid-cols-2">
									<div>
										<label class="text-[10px] uppercase tracking-wider text-slate-500" for="auto-name"
											>Name</label
										>
										<input
											id="auto-name"
											class="mt-1 w-full rounded border border-slate-700 bg-slate-900 px-2 py-1 text-xs text-slate-200"
											bind:value={editAutoApprove.name}
										/>
									</div>
									<div>
										<label class="text-[10px] uppercase tracking-wider text-slate-500" for="auto-sender"
											>Sender Pattern</label
										>
										<input
											id="auto-sender"
											class="mt-1 w-full rounded border border-slate-700 bg-slate-900 px-2 py-1 text-xs text-slate-200"
											placeholder="e.g. relay* or agent-*"
											bind:value={editAutoApprove.sender_pattern}
										/>
									</div>
									<div>
										<label
											class="text-[10px] uppercase tracking-wider text-slate-500"
											for="auto-actions"
											>Action Types</label
										>
										<input
											id="auto-actions"
											class="mt-1 w-full rounded border border-slate-700 bg-slate-900 px-2 py-1 text-xs text-slate-200"
											placeholder="relay.reply, create_node"
											bind:value={editAutoApprove.action_types}
										/>
									</div>
									<div>
										<label
											class="text-[10px] uppercase tracking-wider text-slate-500"
											for="auto-confidence"
											>Min Confidence</label
										>
										<input
											id="auto-confidence"
											type="number"
											min="0"
											max="1"
											step="0.05"
											class="mt-1 w-full rounded border border-slate-700 bg-slate-900 px-2 py-1 text-xs text-slate-200"
											bind:value={editAutoApprove.min_confidence}
										/>
									</div>
								</div>
								<div class="mt-3 flex gap-2">
									<button
										class="rounded-lg bg-sky-500 px-3 py-2 text-xs font-semibold text-white hover:bg-sky-400 disabled:opacity-50"
										on:click={() => handleSaveAutoApprove(rule)}
										disabled={savingAutoApproveId === rule.id}
									>
										{savingAutoApproveId === rule.id ? 'Saving...' : 'Save'}
									</button>
									<button
										class="rounded-lg border border-slate-700 px-3 py-2 text-xs text-slate-300 hover:bg-slate-800"
										on:click={cancelEditAutoApprove}
									>
										Cancel
									</button>
								</div>
							{/if}
						</div>
					{/each}
				{/if}
			</div>

			<div class="mt-4 border-t border-slate-800/60 pt-4">
				<h4 class="text-[10px] uppercase tracking-wider text-slate-500">Add Rule</h4>
				<div class="mt-2 grid grid-cols-1 gap-3 md:grid-cols-2">
					<div>
						<label class="text-[10px] uppercase tracking-wider text-slate-500" for="new-auto-name"
							>Name</label
						>
						<input
							id="new-auto-name"
							class="mt-1 w-full rounded border border-slate-700 bg-slate-900 px-2 py-1 text-xs text-slate-200"
							placeholder="Relay suggestions"
							bind:value={newAutoApproveName}
						/>
					</div>
					<div>
						<label class="text-[10px] uppercase tracking-wider text-slate-500" for="new-auto-sender"
							>Sender Pattern</label
						>
						<input
							id="new-auto-sender"
							class="mt-1 w-full rounded border border-slate-700 bg-slate-900 px-2 py-1 text-xs text-slate-200"
							placeholder="relay* or agent-*"
							bind:value={newAutoApproveSender}
						/>
					</div>
					<div>
						<label class="text-[10px] uppercase tracking-wider text-slate-500" for="new-auto-actions"
							>Action Types</label
						>
						<input
							id="new-auto-actions"
							class="mt-1 w-full rounded border border-slate-700 bg-slate-900 px-2 py-1 text-xs text-slate-200"
							placeholder="relay.reply, create_node"
							bind:value={newAutoApproveActions}
						/>
					</div>
					<div>
						<label class="text-[10px] uppercase tracking-wider text-slate-500" for="new-auto-confidence"
							>Min Confidence</label
						>
						<input
							id="new-auto-confidence"
							type="number"
							min="0"
							max="1"
							step="0.05"
							class="mt-1 w-full rounded border border-slate-700 bg-slate-900 px-2 py-1 text-xs text-slate-200"
							bind:value={newAutoApproveConfidence}
						/>
					</div>
				</div>
				<button
					class="mt-3 rounded-lg bg-sky-500 px-3 py-2 text-xs font-semibold text-white hover:bg-sky-400 disabled:opacity-50"
					on:click={handleAddAutoApprove}
					disabled={addingAutoApprove || !newAutoApproveName.trim()}
				>
					{addingAutoApprove ? 'Adding...' : 'Add Rule'}
				</button>
			</div>
		</section>

		<!-- Keyboard shortcuts -->
		<section class="rounded-xl border border-slate-800 bg-slate-900/40 p-5">
			<h3 class="text-sm font-semibold text-white">Keyboard Shortcuts</h3>
			<div class="mt-3 grid grid-cols-2 gap-2 text-xs">
				{#each [['Cmd/Ctrl + K', 'Command palette'], ['Cmd/Ctrl + Shift + N', 'Quick Capture task (global in desktop app)'], ['Cmd/Ctrl + Shift + M', 'Quick Capture note (global in desktop app)'], ['Cmd/Ctrl + Shift + L', 'Quick Capture link (global in desktop app)'], ['Cmd/Ctrl + Shift + V', 'Quick Capture voice (global in desktop app)'], ['Cmd/Ctrl + Shift + I', 'Quick Capture task to Inbox (global in desktop app)'], ['Cmd/Ctrl + Shift + D', 'Quick Capture note to Daily note (global in desktop app)'], ['Cmd/Ctrl + Shift + P', 'Quick Capture task to Planned (global in desktop app)'], ['Cmd/Ctrl + Shift + R', 'Quick Capture task to Review (global in desktop app)'], ['Cmd/Ctrl + Shift + 1..5', 'Custom quick capture preset (configurable below)'], ['N', 'New task (in tasks/kanban)'], ['Cmd/Ctrl + B', 'Bold (in editor)'], ['Cmd/Ctrl + I', 'Italic (in editor)'], ['Cmd/Ctrl + K', 'Insert link (in editor)'], ['Cmd/Ctrl + S', 'Save (in editor)'], ['Cmd/Ctrl + Z', 'Undo (in editor)'], ['Cmd/Ctrl + F', 'Search in editor'], ['Cmd/Ctrl + H', 'Search & Replace'], ['/', 'Slash commands (in editor)'], ['Escape', 'Close modals/menus']] as [shortcut, action]}
					<div class="flex items-center gap-2 rounded-lg border border-slate-800/60 px-3 py-2">
						<kbd
							class="rounded border border-slate-700 bg-slate-800 px-1.5 py-0.5 text-[10px] text-slate-300"
							>{shortcut}</kbd
						>
						<span class="text-slate-400">{action}</span>
					</div>
				{/each}
			</div>
		</section>

		<!-- Feature Toggles -->
		<section class="rounded-xl border border-slate-800 bg-slate-900/40 p-5">
			<h3 class="text-sm font-semibold text-white">Feature Toggles</h3>
			<p class="mt-1 text-[11px] text-slate-400">
				Enable or disable optional features. Changes apply immediately.
			</p>
			<div class="mt-3 grid grid-cols-1 gap-2">
				{#each featureStates as feature (feature.key)}
					<div
						class="flex items-center justify-between rounded-lg border border-slate-800/60 px-3 py-2.5"
					>
						<div>
							<div class="text-xs font-medium text-white">{feature.label}</div>
							<div class="text-[10px] text-slate-400">{feature.description}</div>
						</div>
						<button
							class="relative h-5 w-9 rounded-full transition {feature.enabled
								? 'bg-sky-500'
								: 'bg-slate-700'}"
							on:click={() => toggleFeature(feature.key, feature.default)}
							aria-label="Toggle {feature.label}"
						>
							<span
								class="absolute top-0.5 h-4 w-4 rounded-full bg-white transition-transform {feature.enabled
									? 'translate-x-4'
									: 'translate-x-0.5'}"
							></span>
						</button>
					</div>
				{/each}
			</div>
		</section>

		<!-- Notification Settings -->
		<section class="rounded-xl border border-slate-800 bg-slate-900/40 p-5">
			<h3 class="text-sm font-semibold text-white">Task Reminders</h3>
			<p class="mt-1 text-[11px] text-slate-400">
				Configure when and how you get reminded about due tasks.
			</p>
			<div class="mt-3 flex flex-col gap-3">
				<div>
					<label class="text-[10px] uppercase tracking-wider text-slate-500" for="notif-lead"
						>Remind me this many minutes before due</label
					>
					<select
						id="notif-lead"
						class="mt-1 w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white"
						bind:value={notifLeadMinutes}
					>
						<option value="5">5 minutes</option>
						<option value="15">15 minutes</option>
						<option value="30">30 minutes</option>
						<option value="60">1 hour</option>
						<option value="120">2 hours</option>
						<option value="1440">1 day</option>
					</select>
				</div>
				<div>
					<label class="text-[10px] uppercase tracking-wider text-slate-500" for="notif-interval"
						>Check frequency</label
					>
					<select
						id="notif-interval"
						class="mt-1 w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white"
						bind:value={notifCheckInterval}
					>
						<option value="30000">Every 30 seconds</option>
						<option value="60000">Every minute</option>
						<option value="300000">Every 5 minutes</option>
						<option value="600000">Every 10 minutes</option>
					</select>
				</div>
				<div>
					<label class="text-[10px] uppercase tracking-wider text-slate-500" for="notif-click-action"
						>When a reminder is clicked</label
					>
					<select
						id="notif-click-action"
						class="mt-1 w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white"
						bind:value={notifClickAction}
					>
						<option value="inbox">Open task capture in Inbox</option>
						<option value="daily">Open note capture in Daily note</option>
						<option value="planned">Open task capture in Planned</option>
						<option value="review">Open task capture in Review</option>
						<option value="none">Do nothing</option>
					</select>
					<p class="mt-1 text-[10px] text-slate-500">
						Applies to due/overdue notifications. Capture opens with a follow-up prefill.
					</p>
				</div>
				<button
					class="self-start rounded-lg bg-sky-500 px-3 py-2 text-xs font-semibold text-white hover:bg-sky-400"
					on:click={saveNotificationSettings}
				>
					Save reminder settings
				</button>
			</div>
		</section>

		<!-- Inbox AI triage -->
		<section class="rounded-xl border border-slate-800 bg-slate-900/40 p-5">
			<h3 class="text-sm font-semibold text-white">Inbox AI Triage</h3>
			<p class="mt-1 text-[11px] text-slate-400">
				Control how Smart Inbox runs and applies AI triage suggestions.
			</p>
			<div class="mt-3 flex flex-col gap-3">
				<label class="flex items-center gap-2 text-xs text-slate-300">
					<input
						type="checkbox"
						checked={inboxTriageAutoRunOnOpen}
						on:change={(event) =>
							(inboxTriageAutoRunOnOpen = (event.currentTarget as HTMLInputElement).checked)}
					/>
					Auto-run AI triage when opening Smart Inbox
				</label>
				<div>
					<label class="text-[10px] uppercase tracking-wider text-slate-500" for="inbox-triage-limit"
						>Default "Apply Top" count</label
					>
					<select
						id="inbox-triage-limit"
						class="mt-1 w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white"
						bind:value={inboxTriageDefaultApplyLimit}
					>
						{#each ['1', '2', '3', '4', '5', '6', '7', '8', '9', '10'] as option}
							<option value={option}>{option}</option>
						{/each}
					</select>
				</div>
				<button
					class="self-start rounded-lg bg-sky-500 px-3 py-2 text-xs font-semibold text-white hover:bg-sky-400"
					on:click={saveInboxTriagePreferences}
				>
					Save inbox triage settings
				</button>
			</div>
		</section>

		<!-- Quick Capture Presets -->
		<section class="rounded-xl border border-slate-800 bg-slate-900/40 p-5">
			<h3 class="text-sm font-semibold text-white">Quick Capture Presets</h3>
			<p class="mt-1 text-[11px] text-slate-400">
				Create reusable capture presets and optionally assign `Cmd/Ctrl + Shift + 1..5` shortcuts.
			</p>
			<div class="mt-3 space-y-3">
				{#if capturePresets.length === 0}
					<p class="rounded-lg border border-dashed border-slate-700 px-3 py-2 text-xs text-slate-500">
						No custom presets yet.
					</p>
				{:else}
					{#each capturePresets as preset (preset.id)}
						<div class="rounded-lg border border-slate-800 bg-slate-900/60 p-3">
							<div class="flex items-center gap-2">
								<input
									class="flex-1 rounded-lg border border-slate-700 bg-slate-800 px-2 py-1.5 text-xs text-white"
									value={preset.name}
									on:input={(event) =>
										updateCapturePreset(preset.id, {
											name: (event.currentTarget as HTMLInputElement).value
										})}
									placeholder="Preset name"
								/>
								<label class="flex items-center gap-1 text-[11px] text-slate-400">
									<input
										type="checkbox"
										checked={preset.enabled}
										on:change={(event) =>
											updateCapturePreset(preset.id, {
												enabled: (event.currentTarget as HTMLInputElement).checked
											})}
									/>
									Enabled
								</label>
								<button
									class="rounded border border-red-500/30 px-2 py-1 text-[11px] text-red-300 hover:bg-red-500/10"
									on:click={() => removeCapturePreset(preset.id)}
								>
									Remove
								</button>
							</div>
							<div class="mt-2 grid grid-cols-1 gap-2 sm:grid-cols-3">
								<select
									class="rounded-lg border border-slate-700 bg-slate-800 px-2 py-1.5 text-xs text-white"
									value={preset.mode}
									on:change={(event) =>
										updateCapturePreset(preset.id, {
											mode: (event.currentTarget as HTMLSelectElement).value as QuickCaptureMode
										})}
								>
									{#each captureModeOptions as option (option.value)}
										<option value={option.value}>{option.label}</option>
									{/each}
								</select>
								<select
									class="rounded-lg border border-slate-700 bg-slate-800 px-2 py-1.5 text-xs text-white"
									value={preset.target}
									on:change={(event) =>
										updateCapturePreset(preset.id, {
											target: (event.currentTarget as HTMLSelectElement).value as QuickCaptureTarget
										})}
								>
									{#each captureTargetOptions as option (option.value)}
										<option value={option.value}>{option.label}</option>
									{/each}
								</select>
								<select
									class="rounded-lg border border-slate-700 bg-slate-800 px-2 py-1.5 text-xs text-white"
									value={preset.shortcut}
									on:change={(event) =>
										updateCapturePreset(preset.id, {
											shortcut: (event.currentTarget as HTMLSelectElement).value as CapturePresetShortcut
										})}
								>
									{#each captureShortcutOptions as option (option)}
										<option value={option}>
											{option === 'none'
												? 'No shortcut'
												: `Cmd/Ctrl+Shift+${option}`}
										</option>
									{/each}
								</select>
							</div>
							<input
								class="mt-2 w-full rounded-lg border border-slate-700 bg-slate-800 px-2 py-1.5 text-xs text-white"
								value={preset.prefill}
								on:input={(event) =>
									updateCapturePreset(preset.id, {
										prefill: (event.currentTarget as HTMLInputElement).value
									})}
								placeholder="Optional prefill text"
							/>
						</div>
					{/each}
				{/if}
			</div>
			<div class="mt-3 flex items-center gap-2">
				<button
					class="rounded-lg border border-slate-700 px-3 py-2 text-xs text-slate-200 hover:bg-slate-800"
					on:click={addCapturePreset}
				>
					Add preset
				</button>
				<button
					class="rounded-lg bg-sky-500 px-3 py-2 text-xs font-semibold text-white hover:bg-sky-400"
					on:click={saveCapturePresetSettings}
				>
					Save presets
				</button>
			</div>
		</section>

		<!-- Data management -->
		<section class="rounded-xl border border-slate-800 bg-slate-900/40 p-5">
			<h3 class="text-sm font-semibold text-white">Data Management</h3>
			<p class="mt-1 text-[11px] text-slate-400">Export, import, or clear local data.</p>
			<div class="mt-3 mb-3 flex flex-wrap gap-2">
				<a
					href="/settings/views"
					class="inline-flex items-center gap-2 rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-slate-200 transition hover:bg-slate-700"
				>
					<svg
						class="h-3.5 w-3.5 text-slate-400"
						fill="none"
						stroke="currentColor"
						viewBox="0 0 24 24"
					>
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							stroke-width="2"
							d="M4 6h16M4 10h16M4 14h16M4 18h16"
						/>
					</svg>
					Saved Views
				</a>
				<a
					href="/settings/audit"
					class="inline-flex items-center gap-2 rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-slate-200 transition hover:bg-slate-700"
				>
					<svg
						class="h-3.5 w-3.5 text-slate-400"
						fill="none"
						stroke="currentColor"
						viewBox="0 0 24 24"
					>
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							stroke-width="2"
							d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2m-3 7h3m-3 4h3m-6-4h.01M9 16h.01"
						/>
					</svg>
					Audit Log
				</a>
			</div>
			<div class="mt-3 flex flex-wrap gap-2">
				<button
					class="rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-slate-200 transition hover:bg-slate-700 disabled:opacity-50"
					on:click={exportData}
					disabled={isExporting}
				>
					{isExporting ? 'Exporting...' : 'Export JSON'}
				</button>
				<button
					class="rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-slate-200 transition hover:bg-slate-700 disabled:opacity-50"
					on:click={exportMarkdown}
					disabled={isExportingMd}
				>
					{isExportingMd ? 'Exporting...' : 'Export Markdown'}
				</button>
				<label
					class="rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-slate-200 transition hover:bg-slate-700 cursor-pointer"
				>
					Import JSON
					<input
						type="file"
						accept=".json"
						class="hidden"
						bind:this={importFileInput}
						on:change={handleImport}
					/>
				</label>
				<button
					class="rounded-lg border border-sky-500/30 bg-sky-500/10 px-3 py-2 text-xs text-sky-300 transition hover:bg-sky-500/20"
					on:click={() => {
						showServerExport = true;
					}}
				>
					Server Export/Import
				</button>
				<button
					class="rounded-lg border border-red-500/30 bg-red-500/10 px-3 py-2 text-xs text-red-300 transition hover:bg-red-500/20"
					on:click={clearLocalData}
				>
					Clear Local Cache
				</button>
			</div>
			<p class="mt-2 text-[10px] text-slate-500">
				Export JSON saves all tasks and notes from local cache. Export Markdown downloads notes as a
				.md file with YAML frontmatter. Import merges data into local cache (overwrites by ID).
				Server Export/Import uses the backend API for full data portability. Clear removes IndexedDB
				and localStorage (server data unaffected).
			</p>
		</section>

		<!-- About -->
		<section class="rounded-xl border border-slate-800 bg-slate-900/40 p-5">
			<h3 class="text-sm font-semibold text-white">About</h3>
			<div class="mt-2 text-xs text-slate-400">
				<p><strong class="text-white">MindVault</strong> - Proprietary Second Brain Platform</p>
				<p class="mt-1">Version 0.1.0</p>
				<p class="mt-1">Local-first, AI-native, desktop-optimized knowledge management.</p>
			</div>
		</section>
	</div>
</div>

<ImportExportPanel
	open={showServerExport}
	on:close={() => {
		showServerExport = false;
	}}
	on:exportComplete={() => {
		pushToast('Export complete', 'success');
	}}
	on:importComplete={async () => {
		await Promise.all([loadTasks(), loadNotes()]);
		pushToast('Import complete', 'success');
		showServerExport = false;
	}}
/>
