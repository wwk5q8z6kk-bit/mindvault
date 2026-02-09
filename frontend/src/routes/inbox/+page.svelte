<script lang="ts">
	import { onMount } from 'svelte';
	import { tasksStore, loadTasks, updateTaskOptimistic, completeTaskOptimistic, snoozeTaskOptimistic } from '$lib/stores/tasks';

	import { notesStore, loadNotes } from '$lib/stores/notes';
	import { pushToast } from '$lib/stores/toast';
	import type { TaskRecord } from '$lib/db';
	import type { Note } from '$lib/api/notes';
	import type { TaskStatus } from '$lib/api/tasks';
	import { updateNote } from '$lib/api/notes';
	import { assistAutoTag } from '$lib/api/assist';
	import { prioritizeTasks } from '$lib/api/ai';
	import { listProposals, approveProposal, rejectProposal, type Proposal } from '$lib/api/exchange';
	import { listConflicts, resolveConflict, type ConflictAlert } from '$lib/api/conflicts';
	import ProposalCard from '$lib/components/ProposalCard.svelte';
	import {
		buildInboxTriagePatch,
		buildInboxTriageSuggestions,
		INBOX_TRIAGE_APPLY_TOP_EVENT_NAME,
		INBOX_TRIAGE_EVENT_NAME,
		type InboxTriageSuggestion
	} from '$lib/inbox/triage';
	import {
		INBOX_TRIAGE_SETTINGS_STORAGE_KEY,
		INBOX_TRIAGE_SETTINGS_UPDATED_EVENT_NAME,
		loadInboxTriageSettings
	} from '$lib/inbox/triage-settings';
	import { createVirtualizer } from '@tanstack/svelte-virtual';

	type InboxItem =
		| { type: 'task'; data: TaskRecord; created: string }
		| { type: 'note'; data: Note; created: string };

	let items: InboxItem[] = [];
	let loading = true;
	let inboxListParentRef: HTMLDivElement | null = null;

	const inboxVirtualizer = createVirtualizer({
		get count() {
			return items.length;
		},
		getScrollElement: () => inboxListParentRef,
		estimateSize: () => 120,
		overscan: 5
	});
	let processedCount = 0;
	let snoozeOpenId: string | null = null;
	let suggestingTagsFor: string | null = null;
	let suggestedTags: Map<string, string[]> = new Map();
	let proposals: Proposal[] = [];
	let proposalsLoading = false;
	let conflicts: ConflictAlert[] = [];
	let conflictsLoading = false;
	let triageLoading = false;
	let triageSuggestions: Map<string, InboxTriageSuggestion> = new Map();
	let triageSettings = loadInboxTriageSettings();

	$: {
		const taskItems: InboxItem[] = $tasksStore
			.filter((t) => t.status === 'inbox')
			.map((t) => ({ type: 'task' as const, data: t, created: t.created_at }));
		const noteItems: InboxItem[] = $notesStore
			.filter((n) => !n.tags || n.tags.length === 0)
			.map((n) => ({ type: 'note' as const, data: n, created: n.created_at }));
		items = [...taskItems, ...noteItems].sort(
			(a, b) => new Date(b.created).getTime() - new Date(a.created).getTime()
		);
	}

	onMount(() => {
		const handleExternalTriage = () => {
			triageSettings = loadInboxTriageSettings();
			void runAiInboxTriage();
		};
		const handleExternalApplyTop = (event: Event) => {
			triageSettings = loadInboxTriageSettings();
			const customEvent = event as CustomEvent<{ limit?: number }>;
			const requestedLimit =
				typeof customEvent?.detail?.limit === 'number'
					? customEvent.detail.limit
					: triageSettings.default_apply_limit;
			const normalizedLimit = Math.min(10, Math.max(1, Math.round(requestedLimit)));
			void (async () => {
				if (triageSuggestions.size === 0) {
					await runAiInboxTriage();
				}
				await applyTopAiTriage(normalizedLimit);
			})();
		};
		const refreshTriageSettings = () => {
			triageSettings = loadInboxTriageSettings();
		};
		const handleStorageEvent = (event: StorageEvent) => {
			if (event.key && event.key !== INBOX_TRIAGE_SETTINGS_STORAGE_KEY) return;
			refreshTriageSettings();
		};
		window.addEventListener(INBOX_TRIAGE_EVENT_NAME, handleExternalTriage);
		window.addEventListener(INBOX_TRIAGE_APPLY_TOP_EVENT_NAME, handleExternalApplyTop as EventListener);
		window.addEventListener(INBOX_TRIAGE_SETTINGS_UPDATED_EVENT_NAME, refreshTriageSettings);
		window.addEventListener('storage', handleStorageEvent);
		void loadProposals();
		void loadConflicts();
		void Promise.all([loadTasks(), loadNotes()]).finally(() => {
			triageSettings = loadInboxTriageSettings();
			loading = false;
			if (triageSettings.auto_run_on_open) {
				void runAiInboxTriage();
			}
		});
		return () => {
			window.removeEventListener(INBOX_TRIAGE_EVENT_NAME, handleExternalTriage);
			window.removeEventListener(
				INBOX_TRIAGE_APPLY_TOP_EVENT_NAME,
				handleExternalApplyTop as EventListener
			);
			window.removeEventListener(INBOX_TRIAGE_SETTINGS_UPDATED_EVENT_NAME, refreshTriageSettings);
			window.removeEventListener('storage', handleStorageEvent);
		};
	});

	async function loadProposals() {
		proposalsLoading = true;
		try {
			proposals = await listProposals('pending', 20);
		} catch {
			// Silently fail — proposals are supplementary
		} finally {
			proposalsLoading = false;
		}
	}

	async function loadConflicts() {
		conflictsLoading = true;
		try {
			conflicts = await listConflicts(false, 20);
		} catch {
			// Silently fail — conflicts are supplementary
		} finally {
			conflictsLoading = false;
		}
	}

	async function handleResolveConflict(id: string) {
		try {
			await resolveConflict(id);
			conflicts = conflicts.filter((c) => c.id !== id);
			processedCount++;
			pushToast('Conflict resolved', 'success');
		} catch {
			pushToast('Failed to resolve conflict', 'danger');
		}
	}

	async function handleApproveProposal(event: CustomEvent<string>) {
		try {
			await approveProposal(event.detail);
			proposals = proposals.filter((p) => p.id !== event.detail);
			processedCount++;
			pushToast('Proposal approved', 'success');
		} catch {
			pushToast('Failed to approve proposal', 'danger');
		}
	}

	async function handleRejectProposal(event: CustomEvent<string>) {
		try {
			await rejectProposal(event.detail);
			proposals = proposals.filter((p) => p.id !== event.detail);
			processedCount++;
			pushToast('Proposal rejected', 'success');
		} catch {
			pushToast('Failed to reject proposal', 'danger');
		}
	}

	async function moveTask(taskId: string, status: TaskStatus) {
		try {
			await updateTaskOptimistic(taskId, { status });
			processedCount++;
			pushToast(`Moved to ${status.replace('_', ' ')}`, 'success');
		} catch {
			pushToast('Failed to update task', 'danger');
		}
	}

	async function markDone(taskId: string) {
		try {
			const undoId = await completeTaskOptimistic(taskId);
			processedCount++;
			pushToast('Marked done', 'success', 3000, undoId);
		} catch {
			pushToast('Failed to complete task', 'danger');
		}
	}

	async function handleSnooze(taskId: string, option: 'tomorrow' | 'next_week' | 'custom') {
		snoozeOpenId = null;
		try {
			if (option === 'tomorrow') {
				const tomorrow = new Date();
				tomorrow.setDate(tomorrow.getDate() + 1);
				tomorrow.setHours(9, 0, 0, 0);
				await snoozeTaskOptimistic(taskId, tomorrow.toISOString());
				pushToast('Snoozed until tomorrow', 'success');
			} else if (option === 'next_week') {
				const nextWeek = new Date();
				const daysUntilMonday = (8 - nextWeek.getDay()) % 7 || 7;
				nextWeek.setDate(nextWeek.getDate() + daysUntilMonday);
				nextWeek.setHours(9, 0, 0, 0);
				await snoozeTaskOptimistic(taskId, nextWeek.toISOString());
				pushToast('Snoozed until next week', 'success');
			} else {
				const input = prompt('Snooze until (YYYY-MM-DD):');
				if (!input?.trim()) return;
				const date = new Date(input.trim());
				date.setHours(9, 0, 0, 0);
				await snoozeTaskOptimistic(taskId, date.toISOString());
				pushToast(`Snoozed until ${input.trim()}`, 'success');
			}
			processedCount++;
		} catch {
			pushToast('Failed to snooze task', 'danger');
		}
	}

	async function tagNote(noteId: string) {
		const tag = prompt('Enter a tag for this note:');
		if (!tag?.trim()) return;
		try {
			const note = $notesStore.find((n) => n.id === noteId);
			if (!note) return;
			const existingTags = note.tags ?? [];
			await updateNote(noteId, { tags: [...existingTags, tag.trim()] });
			await loadNotes();
			processedCount++;
			pushToast(`Tagged with "${tag.trim()}"`, 'success');
		} catch {
			pushToast('Failed to tag note', 'danger');
		}
	}

	async function archiveNote(noteId: string) {
		try {
			await updateNote(noteId, { tags: ['archived'] });
			await loadNotes();
			processedCount++;
			pushToast('Note archived', 'success');
		} catch {
			pushToast('Failed to archive', 'danger');
		}
	}

	async function suggestTags(item: InboxItem) {
		const id = item.data.id;
		if (suggestingTagsFor === id) return;
		suggestingTagsFor = id;
		try {
			const text =
				item.type === 'task'
					? `${item.data.title}${item.data.description ? '\n' + item.data.description : ''}`
					: `${item.data.title ?? ''}${item.data.markdown ? '\n' + item.data.markdown : ''}`;
			// Gather existing tags from notes for context
			const existingTags = [...new Set($notesStore.flatMap((n) => n.tags ?? []))].slice(0, 30);
			const result = await assistAutoTag({ text, existing_tags: existingTags });
			suggestedTags = new Map(suggestedTags).set(id, result.tags);
		} catch {
			pushToast('Failed to suggest tags', 'danger');
		} finally {
			suggestingTagsFor = null;
		}
	}

	async function applyTag(itemId: string, tag: string, itemType: 'task' | 'note') {
		try {
			if (itemType === 'note') {
				const note = $notesStore.find((n) => n.id === itemId);
				if (!note) return;
				await updateNote(itemId, { tags: [...(note.tags ?? []), tag] });
				await loadNotes();
			} else {
				await updateTaskOptimistic(itemId, { labels: [...(($tasksStore.find((t) => t.id === itemId)?.labels) ?? []), tag] });
			}
			// Remove the applied tag from suggestions
			const remaining = (suggestedTags.get(itemId) ?? []).filter((t) => t !== tag);
			suggestedTags = new Map(suggestedTags);
			if (remaining.length === 0) {
				suggestedTags.delete(itemId);
			} else {
				suggestedTags.set(itemId, remaining);
			}
			processedCount++;
			pushToast(`Tagged with "${tag}"`, 'success');
		} catch {
			pushToast('Failed to apply tag', 'danger');
		}
	}

	async function runAiInboxTriage() {
		if (triageLoading) return;
		triageLoading = true;
		try {
			const response = await prioritizeTasks({
				statuses: ['inbox'],
				include_done: false,
				limit: 60
			});
			triageSuggestions = buildInboxTriageSuggestions(
				response.items.filter((item) => item.task.status === 'inbox')
			);
			if (triageSuggestions.size === 0) {
				pushToast('No inbox tasks available for AI triage', 'info');
			} else {
				pushToast(`Prepared AI triage suggestions for ${triageSuggestions.size} task(s)`, 'success');
			}
		} catch {
			pushToast('Failed to run AI inbox triage', 'danger');
		} finally {
			triageLoading = false;
		}
	}

	function clearAiInboxTriage() {
		triageSuggestions = new Map();
	}

	async function applyAiTriageSuggestion(taskId: string, silent = false): Promise<boolean> {
		const suggestion = triageSuggestions.get(taskId);
		const task = $tasksStore.find((entry) => entry.id === taskId);
		if (!suggestion || !task) return false;
		const patch = buildInboxTriagePatch(task, suggestion);
		triageSuggestions = new Map(triageSuggestions);
		triageSuggestions.delete(taskId);
		if (!patch) {
			if (!silent) pushToast('Task already matches AI triage suggestion', 'info');
			return false;
		}
		try {
			await updateTaskOptimistic(taskId, patch);
			processedCount++;
			if (!silent) pushToast('Applied AI triage suggestion', 'success');
			return true;
		} catch {
			if (!silent) pushToast('Failed to apply AI triage suggestion', 'danger');
			return false;
		}
	}

	async function applyTopAiTriage(limit = triageSettings.default_apply_limit) {
		const ordered = [...triageSuggestions.values()]
			.sort((a, b) => a.rank - b.rank)
			.slice(0, limit);
		let applied = 0;
		for (const suggestion of ordered) {
			const success = await applyAiTriageSuggestion(suggestion.taskId, true);
			if (success) applied++;
		}
		if (applied > 0) {
			pushToast(`Applied ${applied} AI triage suggestion(s)`, 'success');
		} else {
			pushToast('No AI triage suggestions were applicable', 'info');
		}
	}

	function relativeTime(dateStr: string): string {
		const now = Date.now();
		const date = new Date(dateStr).getTime();
		const diffMin = Math.floor((now - date) / 60000);
		if (diffMin < 1) return 'just now';
		if (diffMin < 60) return `${diffMin}m ago`;
		const diffHr = Math.floor(diffMin / 60);
		if (diffHr < 24) return `${diffHr}h ago`;
		const diffDay = Math.floor(diffHr / 24);
		return `${diffDay}d ago`;
	}

	function statusLabel(status: TaskStatus): string {
		switch (status) {
			case 'in_progress':
				return 'In progress';
			case 'planned':
				return 'Planned';
			case 'review':
				return 'Review';
			case 'waiting':
				return 'Waiting';
			case 'done':
				return 'Done';
			default:
				return 'Inbox';
		}
	}

	const STATUS_ACTIONS: Array<{ status: TaskStatus; label: string; color: string }> = [
		{ status: 'planned', label: 'Plan', color: 'border-blue-500/30 text-blue-300 hover:bg-blue-500/10' },
		{ status: 'in_progress', label: 'Start', color: 'border-amber-500/30 text-amber-300 hover:bg-amber-500/10' }
	];
</script>

<div class="mx-auto max-w-3xl">
	<div class="flex items-center justify-between">
		<div>
			<h2 class="text-lg font-semibold text-white">Smart Inbox</h2>
			<p class="text-xs text-slate-400">
				{items.length} items to triage
				{#if processedCount > 0}
					&middot; {processedCount} processed this session
				{/if}
			</p>
		</div>
		<div class="flex items-center gap-2">
			<button
				class="rounded-lg border border-indigo-500/30 px-3 py-1.5 text-[11px] font-medium text-indigo-200 transition hover:bg-indigo-500/10 disabled:opacity-50"
				on:click={runAiInboxTriage}
				disabled={triageLoading}
			>
				{triageLoading ? 'Triaging...' : 'AI Triage'}
			</button>
			{#if triageSuggestions.size > 0}
				<button
					class="rounded-lg border border-violet-500/30 px-3 py-1.5 text-[11px] font-medium text-violet-200 transition hover:bg-violet-500/10"
					on:click={() => applyTopAiTriage()}
				>
					Apply Top {triageSettings.default_apply_limit}
				</button>
				<button
					class="rounded-lg border border-slate-700 px-3 py-1.5 text-[11px] text-slate-300 transition hover:bg-slate-800"
					on:click={clearAiInboxTriage}
				>
					Clear
				</button>
			{/if}
		</div>
	</div>

	{#if proposals.length > 0}
		<div class="mt-4">
			<div class="mb-2 flex items-center gap-2">
				<span class="text-xs font-semibold text-slate-300">Pending Proposals</span>
				<span class="rounded-full bg-amber-500/20 px-2 py-0.5 text-[10px] font-medium text-amber-300">
					{proposals.length}
				</span>
			</div>
			<div class="space-y-2">
				{#each proposals as proposal (proposal.id)}
					<ProposalCard
						{proposal}
						on:approve={handleApproveProposal}
						on:reject={handleRejectProposal}
					/>
				{/each}
			</div>
		</div>
	{/if}

	{#if conflicts.length > 0}
		<div class="mt-4">
			<div class="mb-2 flex items-center gap-2">
				<span class="text-xs font-semibold text-slate-300">Conflict Alerts</span>
				<span class="rounded-full bg-red-500/20 px-2 py-0.5 text-[10px] font-medium text-red-300">
					{conflicts.length}
				</span>
			</div>
			<div class="space-y-2">
				{#each conflicts as conflict (conflict.id)}
					<div class="rounded-xl border border-red-900/40 bg-red-950/20 p-4">
						<div class="flex items-start justify-between gap-3">
							<div class="min-w-0 flex-1">
								<div class="flex items-center gap-2 mb-1">
									<span class="rounded bg-red-500/20 px-1.5 py-0.5 text-[10px] font-medium text-red-300">
										{conflict.conflict_type}
									</span>
									<span class="text-[10px] text-slate-500">
										score: {conflict.score.toFixed(2)}
									</span>
								</div>
								<p class="text-xs text-slate-300">{conflict.explanation}</p>
								<p class="mt-1 text-[10px] text-slate-500">
									Nodes: {conflict.node_a.slice(0, 8)}... vs {conflict.node_b.slice(0, 8)}...
								</p>
							</div>
							<button
								class="shrink-0 rounded-lg border border-slate-700 px-2.5 py-1 text-[11px] text-slate-300 transition hover:bg-slate-800"
								on:click={() => handleResolveConflict(conflict.id)}
							>
								Dismiss
							</button>
						</div>
					</div>
				{/each}
			</div>
		</div>
	{/if}

	<div class="mt-4">
		{#if loading}
			<div class="rounded-xl border border-slate-800 p-6 text-center text-xs text-slate-400">
				Loading inbox...
			</div>
		{:else if items.length === 0}
			<div class="rounded-xl border border-dashed border-slate-800 p-8 text-center">
				<div class="flex justify-center">
					<div class="flex h-14 w-14 items-center justify-center rounded-2xl bg-emerald-500/10 text-emerald-300">
						<svg class="h-7 w-7" fill="none" stroke="currentColor" viewBox="0 0 24 24">
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M5 13l4 4L19 7" />
						</svg>
					</div>
				</div>
				<h3 class="mt-3 text-sm font-semibold text-white">Inbox zero</h3>
				<p class="mt-1 text-xs text-slate-400">
					All items have been triaged. New captures will appear here.
				</p>
			</div>
		{:else}
			<div bind:this={inboxListParentRef} style="max-height: 80vh; overflow-y: auto;">
				<div style="height: {$inboxVirtualizer.getTotalSize()}px; width: 100%; position: relative;">
					{#each $inboxVirtualizer.getVirtualItems() as row (row.key)}
						{@const item = items[row.index]}
						<div
							style="position: absolute; top: 0; left: 0; width: 100%; transform: translateY({row.start}px);"
						>
							<div class="rounded-xl border border-slate-800/60 bg-slate-900/40 p-4 mb-3 transition hover:border-slate-700">
								<div class="flex items-start justify-between gap-3">
									<div class="min-w-0 flex-1">
										<div class="flex items-center gap-2">
											<span class="rounded px-1.5 py-0.5 text-[9px] font-medium uppercase {item.type === 'task'
												? 'bg-violet-500/20 text-violet-300'
												: 'bg-sky-500/20 text-sky-300'}">
												{item.type}
											</span>
											<span class="text-[10px] text-slate-500">{relativeTime(item.created)}</span>
										</div>
										<h4 class="mt-1 text-sm font-medium text-white">
											{item.type === 'task' ? item.data.title : item.data.title ?? 'Untitled Note'}
										</h4>
										{#if item.type === 'task' && item.data.description}
											<p class="mt-1 line-clamp-2 text-[11px] text-slate-400">{item.data.description}</p>
										{/if}
										{#if item.type === 'note' && item.data.markdown}
											<p class="mt-1 line-clamp-2 text-[11px] text-slate-400">{item.data.markdown.slice(0, 150)}</p>
										{/if}
										{#if item.type === 'task' && triageSuggestions.has(item.data.id)}
											{@const suggestion = triageSuggestions.get(item.data.id)}
											{#if suggestion}
												<div class="mt-2 rounded-lg border border-indigo-500/30 bg-indigo-500/5 p-2">
													<div class="flex items-center gap-2 text-[10px] text-indigo-200">
														<span class="rounded bg-indigo-500/20 px-1.5 py-0.5 font-medium">AI triage</span>
														<span>Rank #{suggestion.rank}</span>
														<span>Priority P{suggestion.suggestedPriority}</span>
														<span>{statusLabel(suggestion.suggestedStatus)}</span>
													</div>
													{#if suggestion.reason}
														<p class="mt-1 line-clamp-2 text-[10px] text-indigo-100/90">{suggestion.reason}</p>
													{/if}
												</div>
											{/if}
										{/if}
									</div>
								</div>

								<div class="mt-3 flex flex-wrap gap-1.5">
									{#if item.type === 'task'}
										{#each STATUS_ACTIONS as action (action.status)}
											<button
												class="rounded-lg border px-2.5 py-1 text-[10px] font-medium transition {action.color}"
												on:click={() => moveTask(item.data.id, action.status)}
											>
												{action.label}
											</button>
										{/each}
										<div class="relative">
											<button
												class="rounded-lg border border-orange-500/30 px-2.5 py-1 text-[10px] font-medium text-orange-300 transition hover:bg-orange-500/10"
												on:click={() => { snoozeOpenId = snoozeOpenId === item.data.id ? null : item.data.id; }}
											>
												Snooze
											</button>
											{#if snoozeOpenId === item.data.id}
												<div class="absolute left-0 top-full z-20 mt-1 w-36 rounded-lg border border-slate-700 bg-slate-800 py-1 shadow-xl">
													<button
														class="w-full px-3 py-1.5 text-left text-[10px] text-slate-300 hover:bg-slate-700"
														on:click={() => handleSnooze(item.data.id, 'tomorrow')}
													>
														Tomorrow
													</button>
													<button
														class="w-full px-3 py-1.5 text-left text-[10px] text-slate-300 hover:bg-slate-700"
														on:click={() => handleSnooze(item.data.id, 'next_week')}
													>
														Next Week
													</button>
													<button
														class="w-full px-3 py-1.5 text-left text-[10px] text-slate-300 hover:bg-slate-700"
														on:click={() => handleSnooze(item.data.id, 'custom')}
													>
														Custom...
													</button>
												</div>
											{/if}
										</div>
										<button
											class="rounded-lg border border-emerald-500/30 px-2.5 py-1 text-[10px] font-medium text-emerald-300 transition hover:bg-emerald-500/10"
											on:click={() => markDone(item.data.id)}
										>
											Done
										</button>
										<button
											class="rounded-lg border border-teal-500/30 px-2.5 py-1 text-[10px] font-medium text-teal-300 transition hover:bg-teal-500/10 disabled:opacity-50"
											disabled={suggestingTagsFor === item.data.id}
											on:click={() => suggestTags(item)}
										>
											{suggestingTagsFor === item.data.id ? 'Thinking...' : 'AI Tag'}
										</button>
										{#if triageSuggestions.has(item.data.id)}
											<button
												class="rounded-lg border border-indigo-500/30 px-2.5 py-1 text-[10px] font-medium text-indigo-200 transition hover:bg-indigo-500/10"
												on:click={() => applyAiTriageSuggestion(item.data.id)}
											>
												Apply AI
											</button>
										{/if}
										<a
											href="/tasks?task={item.data.id}"
											class="rounded-lg border border-slate-700 px-2.5 py-1 text-[10px] text-slate-300 transition hover:bg-slate-800"
										>
											Open
										</a>
									{:else}
										<button
											class="rounded-lg border border-sky-500/30 px-2.5 py-1 text-[10px] font-medium text-sky-300 transition hover:bg-sky-500/10"
											on:click={() => tagNote(item.data.id)}
										>
											Tag
										</button>
										<button
											class="rounded-lg border border-teal-500/30 px-2.5 py-1 text-[10px] font-medium text-teal-300 transition hover:bg-teal-500/10 disabled:opacity-50"
											disabled={suggestingTagsFor === item.data.id}
											on:click={() => suggestTags(item)}
										>
											{suggestingTagsFor === item.data.id ? 'Thinking...' : 'AI Tag'}
										</button>
										<button
											class="rounded-lg border border-slate-600 px-2.5 py-1 text-[10px] text-slate-300 transition hover:bg-slate-800"
											on:click={() => archiveNote(item.data.id)}
										>
											Archive
										</button>
										<a
											href="/notes?note={item.data.id}"
											class="rounded-lg border border-slate-700 px-2.5 py-1 text-[10px] text-slate-300 transition hover:bg-slate-800"
										>
											Open
										</a>
									{/if}
								</div>

								{#if suggestedTags.has(item.data.id)}
									<div class="mt-2 flex flex-wrap gap-1.5">
										<span class="text-[10px] text-slate-500">Suggested:</span>
										{#each suggestedTags.get(item.data.id) ?? [] as tag}
											<button
												class="rounded-full border border-teal-500/30 bg-teal-500/10 px-2 py-0.5 text-[10px] text-teal-300 transition hover:bg-teal-500/20"
												on:click={() => applyTag(item.data.id, tag, item.type)}
											>
												+ {tag}
											</button>
										{/each}
									</div>
								{/if}
							</div>
						</div>
					{/each}
				</div>
			</div>
		{/if}
	</div>
</div>
