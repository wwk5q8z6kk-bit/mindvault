import { registerActions } from './registry';
import type { CommandAction, CommandContext } from './types';
import { toggleTheme } from '$lib/stores/theme';
import { createNote } from '$lib/api/notes';
import { prioritizeTasks } from '$lib/api/ai';
import { quickAddTaskOptimistic } from '$lib/stores/tasks';
import { taskFilter } from '$lib/stores/tasks';
import { focusPlannerState } from '$lib/stores/ui';
import { createSavedSearch, deleteSavedSearch, listSavedSearches, runSavedSearch, type SavedSearch } from '$lib/api/search';
import {
	dispatchQuickCapture,
	type QuickCaptureMode,
	type QuickCaptureTarget
} from '$lib/capture/quick-capture';
import { dispatchInboxTriage, dispatchInboxTriageApplyTop } from '$lib/inbox/triage';
import { loadCapturePresets } from '$lib/capture/presets';

let cachedSavedSearches: SavedSearch[] = [];
let savedSearchesLoaded = false;

async function ensureSavedSearchesLoaded(): Promise<SavedSearch[]> {
	if (!savedSearchesLoaded) {
		try {
			cachedSavedSearches = await listSavedSearches(50, 0);
			savedSearchesLoaded = true;
		} catch {
			cachedSavedSearches = [];
		}
	}
	return cachedSavedSearches;
}

// Refresh cache when a search is saved (call from search page)
export function invalidateSavedSearchesCache() {
	savedSearchesLoaded = false;
}

const STATUS_MAP: Record<string, string> = {
	inbox: 'inbox',
	todo: 'inbox',
	planned: 'planned',
	progress: 'in_progress',
	'in progress': 'in_progress',
	waiting: 'waiting',
	review: 'review',
	done: 'done'
};

function dispatchQuickCaptureEvent(mode: QuickCaptureMode, target: QuickCaptureTarget) {
	dispatchQuickCapture({ mode, target });
}

function targetLabel(target: QuickCaptureTarget): string {
	switch (target) {
		case 'default':
			return 'Default';
		case 'inbox':
			return 'Inbox';
		case 'daily':
			return 'Daily note';
		case 'planned':
			return 'Planned';
		case 'review':
			return 'Review';
		default:
			return 'Default';
	}
}

export function registerBuiltInActions() {
	const actions: CommandAction[] = [
		{
			id: 'create-task',
			title: 'Create new task',
			subtitle: 'Open full task form',
			keywords: ['new task', 'add task', 'create task'],
			handler: (ctx) => {
				ctx.openTaskModal('create');
			}
		},
		{
			id: 'create-note',
			title: 'Create new note',
			subtitle: 'Blank note (markdown)',
			keywords: ['new note', 'add note', 'create note'],
			handler: async (ctx) => {
				const note = await createNote({ title: 'New note', markdown: '# New note' });
				ctx.toast('Note created', 'success');
				await ctx.navigate(`/notes?note=${note.id}`);
			}
		},
		{
			id: 'quick-add',
			title: 'Quick add',
			subtitle: 'Focus quick‑add input',
			keywords: ['quick add', 'capture', 'inbox'],
			handler: (ctx) => {
				ctx.focusQuickAdd();
			}
		},
		{
			id: 'open-tasks',
			title: 'Open Tasks',
			subtitle: 'Task Management Center',
			keywords: ['tasks', 'task list'],
			handler: async (ctx) => {
				taskFilter.set({ status: 'all', query: '', view: 'all', tags: [], sort: null });
				await ctx.navigate('/tasks');
			}
		},
		{
			id: 'open-goals',
			title: 'Open Goals & Habits',
			subtitle: 'Long-term planning and streak tracking',
			keywords: ['goals', 'habits', 'streaks', 'milestones'],
			handler: (ctx) => ctx.navigate('/goals')
		},
		{
			id: 'open-notes',
			title: 'Open Notes',
			subtitle: 'Notes workspace',
			keywords: ['notes', 'note list'],
			handler: (ctx) => ctx.navigate('/notes')
		},
		{
			id: 'open-media',
			title: 'Open Media Library',
			subtitle: 'Browse attachments across notes and tasks',
			keywords: ['media', 'attachments', 'files'],
			handler: (ctx) => ctx.navigate('/media')
		},
		{
			id: 'open-inbox',
			title: 'Open Inbox',
			subtitle: 'Tasks with inbox status',
			keywords: ['inbox', 'triage'],
			handler: async (ctx) => {
				taskFilter.set({ status: 'all', query: '', view: 'inbox', tags: [], sort: null });
				await ctx.navigate('/tasks');
			}
		},
		{
			id: 'open-today',
			title: 'Open Today',
			subtitle: 'Tasks due today',
			keywords: ['today', 'due today'],
			handler: async (ctx) => {
				taskFilter.set({ status: 'all', query: '', view: 'today', tags: [], sort: null });
				await ctx.navigate('/tasks');
			}
		},
		{
			id: 'open-upcoming',
			title: 'Open Upcoming',
			subtitle: 'Tasks due next 7 days',
			keywords: ['upcoming', 'next'],
			handler: async (ctx) => {
				taskFilter.set({ status: 'all', query: '', view: 'upcoming', tags: [], sort: null });
				await ctx.navigate('/tasks');
			}
		},
		{
			id: 'open-kanban',
			title: 'Open Kanban',
			subtitle: 'Board view',
			keywords: ['board', 'kanban'],
			handler: (ctx) => ctx.navigate('/kanban')
		},
		{
			id: 'open-calendar',
			title: 'Open Calendar',
			subtitle: 'Time blocks and events',
			keywords: ['calendar', 'schedule'],
			handler: (ctx) => ctx.navigate('/calendar')
		},
		{
			id: 'open-timeline',
			title: 'Open Timeline',
			subtitle: 'Activity feed',
			keywords: ['timeline', 'activity'],
			handler: (ctx) => ctx.navigate('/timeline')
		},
		{
			id: 'search-global',
			title: 'Search tasks & notes',
			subtitle: 'Use FTS (type to search)',
			keywords: ['search', 'find', 'fts'],
			closeOnRun: false,
			handler: (ctx) => ctx.setQuery('search ')
		},
		{
			id: 'refresh-data',
			title: 'Refresh data / Sync now',
			subtitle: 'Force refresh and sync queue',
			keywords: ['refresh', 'sync'],
			handler: async (ctx) => {
				await ctx.refreshData();
				ctx.toast('Data refreshed', 'success');
			}
		},
		{
			id: 'ai-focus',
			title: 'AI: Prioritize tasks',
			subtitle: 'Generate focus list for today',
			keywords: ['ai', 'focus', 'prioritize', 'plan day'],
			handler: async (ctx) => {
				try {
					ctx.toast('Generating focus list…', 'info');
					const response = await prioritizeTasks({ limit: 8 });
					if (!response.items.length) {
						ctx.toast('No tasks to prioritize', 'warning');
						return;
					}
					focusPlannerState.set({
						open: true,
						generatedAt: response.generated_at,
						items: response.items
					});
				} catch {
					ctx.toast('Focus planner failed', 'danger');
				}
			}
		},
		{
			id: 'toggle-theme',
			title: 'Toggle dark mode',
			subtitle: 'Switch light/dark',
			keywords: ['theme', 'dark mode', 'light mode'],
			handler: () => toggleTheme()
		},
		{
			id: 'open-settings',
			title: 'Open Settings',
			subtitle: 'Preferences & configuration',
			keywords: ['settings', 'preferences'],
			handler: (ctx) => ctx.navigate('/settings')
		},
		{
			id: 'create-template',
			title: 'Open Template Studio',
			subtitle: 'Open Template Studio',
			keywords: ['template', 'preset'],
			handler: async (ctx) => {
				await ctx.navigate('/bookmarks?view=templates');
			}
		},
		{
			id: 'open-daily',
			title: 'Open Daily Note',
			subtitle: "Today's journal entry",
			keywords: ['daily', 'journal', 'today note', 'diary'],
			handler: (ctx) => ctx.navigate('/daily')
		},
		{
			id: 'open-dashboard',
			title: 'Open Dashboard',
			subtitle: 'Overview & stats',
			keywords: ['dashboard', 'home', 'overview'],
			handler: (ctx) => ctx.navigate('/')
		},
		{
			id: 'export-data',
			title: 'Export data',
			subtitle: 'Download JSON backup',
			keywords: ['export', 'backup', 'download', 'json'],
			handler: (ctx) => ctx.navigate('/settings')
		},
		{
			id: 'open-chat',
			title: 'Open AI Chat',
			subtitle: 'Ask questions about your knowledge',
			keywords: ['chat', 'ai', 'ask', 'rag', 'question'],
			handler: (ctx) => ctx.navigate('/chat')
		},
		{
			id: 'open-smart-inbox',
			title: 'Open Smart Inbox',
			subtitle: 'Triage new captures',
			keywords: ['inbox', 'triage', 'capture', 'unsorted'],
			handler: (ctx) => ctx.navigate('/inbox')
		},
		{
			id: 'ai-triage-inbox',
			title: 'AI: Triage Inbox',
			subtitle: 'Open Inbox and generate AI triage suggestions',
			keywords: ['ai', 'triage', 'inbox', 'prioritize'],
			handler: async (ctx) => {
				await ctx.navigate('/inbox');
				dispatchInboxTriage();
			}
		},
		{
			id: 'ai-triage-inbox-apply',
			title: 'AI: Apply Top Inbox Triage',
			subtitle: 'Open Inbox, run AI triage, and apply top suggestions',
			keywords: ['ai', 'triage', 'inbox', 'apply top', 'auto triage'],
			handler: async (ctx) => {
				await ctx.navigate('/inbox');
				dispatchInboxTriageApplyTop();
			}
		},
		{
			id: 'quick-capture',
			title: 'Quick Capture',
			subtitle: 'Capture a thought instantly (Cmd+Shift+N)',
			keywords: ['capture', 'quick', 'jot', 'thought'],
			handler: () => {
				dispatchQuickCaptureEvent('task', 'default');
			}
		},
		{
			id: 'quick-capture-inbox',
			title: 'Quick Capture to Inbox',
			subtitle: 'Capture task and auto-tag inbox (Cmd+Shift+I)',
			keywords: ['capture', 'inbox', 'quick', 'triage'],
			handler: () => {
				dispatchQuickCaptureEvent('task', 'inbox');
			}
		},
		{
			id: 'quick-capture-daily',
			title: 'Quick Capture to Daily Note',
			subtitle: "Capture note linked to today's daily note (Cmd+Shift+D)",
			keywords: ['capture', 'daily', 'journal', 'quick'],
			handler: () => {
				dispatchQuickCaptureEvent('note', 'daily');
			}
		},
		{
			id: 'quick-capture-planned',
			title: 'Quick Capture to Planned',
			subtitle: 'Capture task routed to Planned queue (Cmd+Shift+P)',
			keywords: ['capture', 'planned', 'queue', 'quick'],
			handler: () => {
				dispatchQuickCaptureEvent('task', 'planned');
			}
		},
		{
			id: 'quick-capture-review',
			title: 'Quick Capture to Review',
			subtitle: 'Capture task routed to Review queue (Cmd+Shift+R)',
			keywords: ['capture', 'review', 'queue', 'quick'],
			handler: () => {
				dispatchQuickCaptureEvent('task', 'review');
			}
		},
		{
			id: 'open-review',
			title: 'Open Proactive Review',
			subtitle: 'Weekly digest and review prompts',
			keywords: ['review', 'digest', 'weekly', 'reflect', 'spaced'],
			handler: (ctx) => ctx.navigate('/review')
		},
		{
			id: 'open-tags',
			title: 'Open Tag Manager',
			subtitle: 'Browse and manage tags',
			keywords: ['tags', 'tag manager', 'labels'],
			handler: (ctx) => ctx.navigate('/tags')
		},
		{
			id: 'open-bookmarks',
			title: 'Open Reading List',
			subtitle: 'Web clips and references',
			keywords: ['bookmarks', 'reading', 'web clips', 'references'],
			handler: (ctx) => ctx.navigate('/bookmarks')
		},
		{
			id: 'open-flashcards',
			title: 'Open Flashcards',
			subtitle: 'Spaced repetition review',
			keywords: ['flashcards', 'cards', 'review', 'spaced repetition', 'sm2'],
			handler: (ctx) => ctx.navigate('/bookmarks?view=flashcards')
		},
		{
			id: 'open-graph',
			title: 'Open Knowledge Graph',
			subtitle: 'Visual node connections',
			keywords: ['graph', 'knowledge graph', 'connections', 'visualization'],
			handler: (ctx) => ctx.navigate('/notes?view=graph')
		},
		{
			id: 'open-canvas',
			title: 'Open Canvas',
			subtitle: 'Freeform mind map workspace',
			keywords: ['canvas', 'mind map', 'spatial', 'whiteboard'],
			handler: (ctx) => ctx.navigate('/canvas')
		},
		{
			id: 'manage-profiles',
			title: 'Manage Profiles & Access Keys',
			subtitle: 'API keys and permission templates',
			keywords: ['profiles', 'access keys', 'api keys', 'permissions'],
			handler: (ctx) => ctx.navigate('/settings/profiles')
		},
		{
			id: 'open-search',
			title: 'Open Search Page',
			subtitle: 'Full search with saved searches',
			keywords: ['search', 'find', 'saved searches'],
			handler: (ctx) => ctx.navigate('/search')
		},
		{
			id: 'manage-saved-searches',
			title: 'Manage Saved Searches',
			subtitle: 'View, edit, and delete saved searches',
			keywords: ['saved', 'searches', 'manage', 'edit', 'delete'],
			handler: (ctx) => ctx.navigate('/search/saved')
		},
		{
			id: 'new-saved-search',
			title: 'New Saved Search',
			subtitle: 'Create a new saved search (opens search page)',
			keywords: ['saved', 'search', 'new', 'create'],
			handler: (ctx) => ctx.navigate('/search')
		},
		{
			id: 'quick-search',
			title: 'Quick Search',
			subtitle: 'Instant search modal (Cmd+/)',
			keywords: ['search', 'find', 'quick', 'instant'],
			handler: () => {
				window.dispatchEvent(
					new KeyboardEvent('keydown', { key: '/', metaKey: true })
				);
			}
		},
		{
			id: 'open-onboarding',
			title: 'Restart Onboarding',
			subtitle: 'Go through setup wizard again',
			keywords: ['onboarding', 'setup', 'tutorial', 'getting started'],
			handler: async (ctx) => {
				localStorage.removeItem('mv_onboarding');
				await ctx.navigate('/onboarding');
			}
		},
		{
			id: 'show-shortcuts',
			title: 'Show Keyboard Shortcuts',
			subtitle: 'View all available shortcuts',
			keywords: ['shortcuts', 'keyboard', 'keys', 'help', 'hotkeys'],
			handler: () => {
				// Trigger the ? key to open shortcuts modal
				window.dispatchEvent(
					new KeyboardEvent('keydown', { key: '?' })
				);
			}
		}
	];

	registerActions(actions);
}

export function buildDynamicActions(query: string, ctx: CommandContext): CommandAction[] {
	const raw = query.trim();
	const trimmed = raw.toLowerCase();
	const actions: CommandAction[] = [];

	const triageTopMatch = trimmed.match(/^(?:ai\s+)?triage(?:\s+inbox)?\s+top\s+(\d{1,2})$/);
	if (triageTopMatch) {
		const requested = Number.parseInt(triageTopMatch[1], 10);
		if (Number.isFinite(requested)) {
			const normalizedLimit = Math.min(10, Math.max(1, requested));
			actions.push({
				id: `ai-triage-inbox-top-${normalizedLimit}`,
				title: `AI: Apply Top ${normalizedLimit} Inbox Triage`,
				subtitle: 'Open Inbox, run AI triage, then apply top suggestions',
				group: 'Inbox Automation',
				keywords: ['ai', 'triage', 'inbox', 'top', String(normalizedLimit)],
				handler: async (innerCtx) => {
					await innerCtx.navigate('/inbox');
					dispatchInboxTriageApplyTop(normalizedLimit);
				}
			});
		}
	}

	if (trimmed.length === 0 || trimmed.includes('capture') || trimmed.includes('preset')) {
		const presets = loadCapturePresets()
			.filter((preset) => preset.enabled)
			.slice(0, 5);
		for (const preset of presets) {
			const shortcutHint =
				preset.shortcut === 'none' ? '' : ` · Cmd/Ctrl+Shift+${preset.shortcut}`;
			actions.push({
				id: `quick-capture-preset-${preset.id}`,
				title: `Quick Capture: ${preset.name}`,
				subtitle: `${preset.mode} -> ${targetLabel(preset.target)}${shortcutHint}`,
				group: 'Quick Capture Presets',
				keywords: [
					'quick capture',
					'capture preset',
					'preset',
					preset.name.toLowerCase(),
					preset.mode,
					preset.target
				],
				handler: () => {
					dispatchQuickCapture({
						mode: preset.mode,
						target: preset.target,
						prefill: preset.prefill
					});
				}
			});
		}
	}

	if (trimmed.startsWith('add tag ') || trimmed.startsWith('tag ')) {
		const label = trimmed.replace(/^add tag\s+|^tag\s+/, '').trim();
		if (label && ctx.selectedTaskId) {
			actions.push({
				id: `add-tag-${label}`,
				title: `Add tag "${label}"`,
				subtitle: 'Apply to selected task',
				keywords: ['tag', 'label'],
				isAvailable: () => true,
				handler: async (innerCtx) => {
					await innerCtx.addTaskLabel(innerCtx.selectedTaskId as string, label);
					innerCtx.toast(`Added tag ${label}`, 'success');
				}
			});
		}
	}

	if (trimmed.startsWith('mark ') || trimmed.startsWith('status ')) {
		const target = trimmed.replace(/^mark\s+|^status\s+/, '').trim();
		const status = STATUS_MAP[target];
		if (status && ctx.selectedTaskId) {
			actions.push({
				id: `status-${status}`,
				title: `Set status: ${status.replace('_', ' ')}`,
				subtitle: 'Selected task',
				handler: async (innerCtx) => {
					await innerCtx.updateTaskStatus(innerCtx.selectedTaskId as string, status);
					innerCtx.toast('Status updated', 'success');
				}
			});
		}
	}

	if (trimmed.startsWith('go ') || trimmed.startsWith('open ')) {
		const term = trimmed.replace(/^go\s+|^open\s+/, '').trim();
		if (term.length >= 2) {
			const matches = ctx.tasks.filter(
				(task) => task.title.toLowerCase().startsWith(term) || task.id.startsWith(term)
			);
			for (const match of matches.slice(0, 5)) {
				actions.push({
					id: `go-task-${match.id}`,
					title: `Open task: ${match.title}`,
					subtitle: match.id,
					keywords: ['open task', 'go task'],
					handler: async (innerCtx) => {
						await innerCtx.navigate('/tasks');
						innerCtx.selectTask(match.id);
						innerCtx.toast('Task selected', 'success');
					}
				});
			}
		}
	}

	if (trimmed.startsWith('open note ') || trimmed.startsWith('note ')) {
		const term = trimmed.replace(/^open note\s+|^note\s+/, '').trim();
		if (term.length >= 4) {
			actions.push({
				id: `go-note-${term}`,
				title: `Open note ${term}`,
				subtitle: 'Navigate to note by ID',
				keywords: ['open note', 'go note'],
				handler: async (innerCtx) => {
					await innerCtx.navigate(`/notes?note=${term}`);
					innerCtx.toast('Note opened', 'success');
				}
			});
		}
	}

	// "View all results" for search queries
	if (trimmed.startsWith('search ') && raw.length > 7) {
		const searchTerm = raw.replace(/^search\s+/, '').trim();
		actions.push({
			id: 'view-all-search',
			title: `View all results for "${searchTerm}"`,
			subtitle: 'Open search page',
			keywords: ['search', 'results'],
			handler: async (innerCtx) => {
				await innerCtx.navigate(`/search?q=${encodeURIComponent(searchTerm)}`);
			}
		});
	}

	// "save search <name>" - create a saved search from palette
	if (trimmed.startsWith('save search ')) {
		const searchName = raw.replace(/^save search\s+/i, '').trim();
		if (searchName.length >= 2) {
			actions.push({
				id: `create-saved-search-${searchName}`,
				title: `Save search: "${searchName}"`,
				subtitle: 'Create new saved search with this name',
				group: 'Saved Searches',
				keywords: ['save', 'search', 'create'],
				closeOnRun: false,
				handler: async (innerCtx) => {
					// Prompt user to enter query
					innerCtx.setQuery(`save search ${searchName} query:`);
					innerCtx.toast('Enter your search query after "query:"', 'info');
				}
			});
		}
	}

	// "save search <name> query:<query>" - complete saved search creation
	const saveSearchMatch = raw.match(/^save search (.+?) query:\s*(.+)$/i);
	if (saveSearchMatch) {
		const [, searchName, searchQuery] = saveSearchMatch;
		if (searchName.trim() && searchQuery.trim()) {
			actions.push({
				id: 'create-saved-search-confirm',
				title: `Create saved search "${searchName.trim()}"`,
				subtitle: `Query: "${searchQuery.trim()}"`,
				group: 'Saved Searches',
				keywords: ['save', 'search', 'create', 'confirm'],
				handler: async (innerCtx) => {
					try {
						await createSavedSearch({
							name: searchName.trim(),
							query: searchQuery.trim(),
							search_type: 'hybrid',
							limit: 50
						});
						invalidateSavedSearchesCache();
						innerCtx.toast(`Saved search "${searchName.trim()}" created`, 'success');
					} catch {
						innerCtx.toast('Failed to create saved search', 'danger');
					}
				}
			});
		}
	}

	// "delete saved <name>" - delete a saved search
	if (trimmed.startsWith('delete saved ') || trimmed.startsWith('remove saved ')) {
		const searchTerm = trimmed.replace(/^delete saved\s+|^remove saved\s+/, '').trim();
		if (searchTerm.length >= 2) {
			void ensureSavedSearchesLoaded().then((searches) => {
				const matches = searches.filter(s =>
					s.name.toLowerCase().includes(searchTerm) ||
					s.query.toLowerCase().includes(searchTerm)
				);
				for (const search of matches.slice(0, 5)) {
					if (!actions.some(a => a.id === `delete-saved-${search.id}`)) {
						actions.push({
							id: `delete-saved-${search.id}`,
							title: `Delete saved search: ${search.name}`,
							subtitle: `"${search.query}"`,
							group: 'Delete Saved Search',
							keywords: ['delete', 'remove', 'saved', search.name],
							handler: async (innerCtx) => {
								try {
									await deleteSavedSearch(search.id);
									invalidateSavedSearchesCache();
									innerCtx.toast(`Deleted "${search.name}"`, 'success');
								} catch {
									innerCtx.toast('Failed to delete saved search', 'danger');
								}
							}
						});
					}
				}
			});
		}
	}

	// Saved searches - show when query matches or starts with "saved" or "run"
	if (trimmed.startsWith('saved') || trimmed.startsWith('run ') || trimmed.length === 0) {
		// Load saved searches asynchronously and add them as actions
		void ensureSavedSearchesLoaded().then((searches) => {
			const searchTerm = trimmed.replace(/^saved\s*|^run\s*/, '').trim();
			const filtered = searchTerm
				? searches.filter(s => s.name.toLowerCase().includes(searchTerm) || s.query.toLowerCase().includes(searchTerm))
				: searches;

			for (const search of filtered.slice(0, 5)) {
				if (!actions.some(a => a.id === `saved-search-${search.id}`)) {
					actions.push({
						id: `saved-search-${search.id}`,
						title: `Run: ${search.name}`,
						subtitle: `"${search.query}" (${search.search_type})`,
						group: 'Saved Searches',
						keywords: ['saved', 'search', search.name, search.query],
						handler: async (innerCtx) => {
							try {
								const result = await runSavedSearch(search.id);
								innerCtx.toast(`Found ${result.results.length} results`, 'success');
								await innerCtx.navigate(`/search?q=${encodeURIComponent(search.query)}`);
							} catch {
								innerCtx.toast('Failed to run saved search', 'danger');
							}
						}
					});
				}
			}
		});
	}

	if (trimmed.startsWith('quick ') && raw.length > 6) {
		const value = raw.replace(/^quick\s+/, '').trim();
		actions.push({
			id: 'quick-add-inline',
			title: `Quick add "${value}"`,
			subtitle: 'Create via quick add parser',
			handler: async (innerCtx) => {
				await quickAddTaskOptimistic(value);
				innerCtx.toast('Quick add created', 'success');
			}
		});
	}

	return actions;
}
