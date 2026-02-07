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
	import ImportExportPanel from '$lib/components/ImportExportPanel.svelte';

	let showServerExport = false;

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

	function saveNotificationSettings() {
		localStorage.setItem('mv_notification_lead_minutes', notifLeadMinutes);
		localStorage.setItem('mv_notification_check_interval', notifCheckInterval);
		localStorage.setItem('mv_notification_click_action', notifClickAction);
		resetNotifiedIds();
		stopNotifications();
		startNotifications();
		pushToast('Notification settings saved', 'success');
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

		<!-- AI Provider (BYOK) -->
		<section class="rounded-xl border border-slate-800 bg-slate-900/40 p-5">
			<h3 class="text-sm font-semibold text-white">AI Provider</h3>
			<p class="mt-1 text-[11px] text-slate-400">
				Bring your own API key or use a local LLM. Leave as "Server Default" to use the backend's
				configured provider.
			</p>

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

		<!-- Keyboard shortcuts -->
		<section class="rounded-xl border border-slate-800 bg-slate-900/40 p-5">
			<h3 class="text-sm font-semibold text-white">Keyboard Shortcuts</h3>
			<div class="mt-3 grid grid-cols-2 gap-2 text-xs">
				{#each [['Cmd/Ctrl + K', 'Command palette'], ['Cmd/Ctrl + Shift + N', 'Quick Capture task (global in desktop app)'], ['Cmd/Ctrl + Shift + M', 'Quick Capture note (global in desktop app)'], ['Cmd/Ctrl + Shift + L', 'Quick Capture link (global in desktop app)'], ['Cmd/Ctrl + Shift + V', 'Quick Capture voice (global in desktop app)'], ['Cmd/Ctrl + Shift + I', 'Quick Capture task to Inbox (global in desktop app)'], ['Cmd/Ctrl + Shift + D', 'Quick Capture note to Daily note (global in desktop app)'], ['Cmd/Ctrl + Shift + P', 'Quick Capture task to Planned (global in desktop app)'], ['Cmd/Ctrl + Shift + R', 'Quick Capture task to Review (global in desktop app)'], ['N', 'New task (in tasks/kanban)'], ['Cmd/Ctrl + B', 'Bold (in editor)'], ['Cmd/Ctrl + I', 'Italic (in editor)'], ['Cmd/Ctrl + K', 'Insert link (in editor)'], ['Cmd/Ctrl + S', 'Save (in editor)'], ['Cmd/Ctrl + Z', 'Undo (in editor)'], ['Cmd/Ctrl + F', 'Search in editor'], ['Cmd/Ctrl + H', 'Search & Replace'], ['/', 'Slash commands (in editor)'], ['Escape', 'Close modals/menus']] as [shortcut, action]}
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
