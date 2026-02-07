<script lang="ts">
	import { onMount } from 'svelte';
	import { tasksStore, loadTasks } from '$lib/stores/tasks';
	import { notesStore, loadNotes } from '$lib/stores/notes';
	import { searchHybrid } from '$lib/api/search';
	import { assistTransform } from '$lib/api/assist';
	import { pushToast } from '$lib/stores/toast';
	import type { TaskRecord } from '$lib/db';
	import type { Note } from '$lib/api/notes';

	type DigestSection = {
		title: string;
		content: string;
		sources: Array<{ id: string; title: string; kind: string }>;
	};

	let loading = true;
	let generating = false;
	let digest: DigestSection[] = [];
	let reviewPrompts: string[] = [];
	let weekStats = { captured: 0, completed: 0, notesCreated: 0, activeProjects: 0 };

	// Time range
	let rangeLabel = 'This Week';
	let rangeStart: Date;
	let rangeEnd: Date;

	function setThisWeek() {
		const now = new Date();
		rangeEnd = now;
		rangeStart = new Date(now.getFullYear(), now.getMonth(), now.getDate() - 7);
		rangeLabel = 'This Week';
	}

	function setLastMonth() {
		const now = new Date();
		rangeEnd = now;
		rangeStart = new Date(now.getFullYear(), now.getMonth() - 1, now.getDate());
		rangeLabel = 'Last 30 Days';
	}

	setThisWeek();

	$: recentTasks = $tasksStore.filter((t) => {
		const created = new Date(t.created_at);
		return created >= rangeStart && created <= rangeEnd;
	});

	$: completedTasks = $tasksStore.filter((t) => {
		if (t.status !== 'done' || !t.completed_at) return false;
		const completed = new Date(t.completed_at);
		return completed >= rangeStart && completed <= rangeEnd;
	});

	$: recentNotes = $notesStore.filter((n) => {
		const created = new Date(n.created_at);
		return created >= rangeStart && created <= rangeEnd;
	});

	$: {
		const uniqueTags = new Set<string>();
		recentTasks.forEach((t) => t.labels?.forEach((l) => uniqueTags.add(l)));
		recentNotes.forEach((n) => n.tags?.forEach((t) => uniqueTags.add(t)));
		weekStats = {
			captured: recentTasks.length + recentNotes.length,
			completed: completedTasks.length,
			notesCreated: recentNotes.length,
			activeProjects: uniqueTags.size
		};
	}

	onMount(async () => {
		await Promise.all([loadTasks(), loadNotes()]);
		loading = false;
	});

	async function generateDigest() {
		generating = true;
		digest = [];
		reviewPrompts = [];

		try {
			// Build context from recent items
			const taskSummaries = recentTasks
				.slice(0, 20)
				.map((t) => `- [${t.status}] ${t.title}${t.due_at ? ` (due: ${t.due_at})` : ''}`)
				.join('\n');

			const noteSummaries = recentNotes
				.slice(0, 15)
				.map((n) => `- ${n.title ?? 'Untitled'}: ${(n.markdown ?? '').slice(0, 150)}`)
				.join('\n');

			const completedSummaries = completedTasks
				.slice(0, 10)
				.map((t) => `- ${t.title}`)
				.join('\n');

			const context = [
				`Period: ${rangeStart.toLocaleDateString()} - ${rangeEnd.toLocaleDateString()}`,
				'',
				`Recent Tasks (${recentTasks.length}):`,
				taskSummaries || '(none)',
				'',
				`Completed Tasks (${completedTasks.length}):`,
				completedSummaries || '(none)',
				'',
				`Recent Notes (${recentNotes.length}):`,
				noteSummaries || '(none)'
			].join('\n');

			// Generate weekly summary
			const summaryPrompt = [
				'You are a personal knowledge assistant reviewing the user\'s weekly activity.',
				'Based on the data below, create a concise weekly digest with these sections:',
				'1. Key Accomplishments (what was completed)',
				'2. Active Focus Areas (main themes and projects)',
				'3. Emerging Patterns (connections between items)',
				'4. Suggested Next Steps (what to prioritize)',
				'',
				'Be specific and reference actual items. Keep each section to 2-3 bullet points.',
				'',
				context
			].join('\n');

			const summaryResult = await assistTransform({
				text: summaryPrompt,
				mode: 'refine'
			});

			// Parse the response into sections
			const sections = parseSections(summaryResult.transformed_text);
			digest = sections.map((s) => ({
				...s,
				sources: findRelatedItems(s.content)
			}));

			// Generate review prompts
			const promptsText = [
				'Based on this activity data, generate 4 reflective review questions that help the user:',
				'1. One question about decision quality',
				'2. One question about knowledge gaps',
				'3. One question about connecting ideas',
				'4. One question about next week\'s priorities',
				'',
				'Format: one question per line, no numbering.',
				'',
				context
			].join('\n');

			const promptsResult = await assistTransform({
				text: promptsText,
				mode: 'refine'
			});

			reviewPrompts = promptsResult.transformed_text
				.split('\n')
				.map((line) => line.trim())
				.filter((line) => line.length > 10 && line.includes('?'));

			pushToast('Digest generated', 'success');
		} catch {
			pushToast('Failed to generate digest. Check AI provider settings.', 'danger');
		} finally {
			generating = false;
		}
	}

	function parseSections(text: string): Array<{ title: string; content: string }> {
		const sections: Array<{ title: string; content: string }> = [];
		const lines = text.split('\n');
		let currentTitle = 'Summary';
		let currentContent: string[] = [];

		for (const line of lines) {
			const headerMatch = line.match(/^#+\s+(.+)$/) || line.match(/^\*\*(.+?)\*\*\s*$/);
			if (headerMatch) {
				if (currentContent.length > 0) {
					sections.push({ title: currentTitle, content: currentContent.join('\n').trim() });
				}
				currentTitle = headerMatch[1].replace(/[*#]/g, '').trim();
				currentContent = [];
			} else {
				currentContent.push(line);
			}
		}
		if (currentContent.length > 0) {
			sections.push({ title: currentTitle, content: currentContent.join('\n').trim() });
		}

		return sections.filter((s) => s.content.length > 0);
	}

	function findRelatedItems(
		content: string
	): Array<{ id: string; title: string; kind: string }> {
		const related: Array<{ id: string; title: string; kind: string }> = [];
		const allItems = [
			...recentTasks.map((t) => ({ id: t.id, title: t.title, kind: 'task' as const })),
			...recentNotes.map((n) => ({ id: n.id, title: n.title ?? 'Untitled', kind: 'note' as const }))
		];

		for (const item of allItems) {
			if (item.title && content.toLowerCase().includes(item.title.toLowerCase().slice(0, 20))) {
				related.push(item);
				if (related.length >= 3) break;
			}
		}
		return related;
	}

	function itemLink(item: { id: string; kind: string }): string {
		if (item.kind === 'task') return `/tasks?task=${item.id}`;
		return `/notes?note=${item.id}`;
	}

	function formatDate(d: Date): string {
		return d.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
	}
</script>

<div class="mx-auto max-w-3xl">
	<div class="flex items-center justify-between">
		<div>
			<h2 class="text-lg font-semibold text-white">Proactive Review</h2>
			<p class="text-xs text-slate-400">
				{formatDate(rangeStart)} - {formatDate(rangeEnd)} &middot; AI-powered weekly digest with review prompts
			</p>
		</div>
		<div class="flex items-center gap-2">
			<div class="flex rounded-lg border border-slate-700 bg-slate-800 p-0.5">
				<button
					class="rounded-md px-2.5 py-1 text-[10px] font-medium transition {rangeLabel === 'This Week'
						? 'bg-sky-500/20 text-sky-300'
						: 'text-slate-400 hover:text-white'}"
					on:click={() => { setThisWeek(); digest = []; }}
				>
					Week
				</button>
				<button
					class="rounded-md px-2.5 py-1 text-[10px] font-medium transition {rangeLabel === 'Last 30 Days'
						? 'bg-sky-500/20 text-sky-300'
						: 'text-slate-400 hover:text-white'}"
					on:click={() => { setLastMonth(); digest = []; }}
				>
					Month
				</button>
			</div>
		</div>
	</div>

	<!-- Stats Row -->
	<div class="mt-4 grid grid-cols-4 gap-3">
		{#each [
			{ label: 'Captured', value: weekStats.captured, color: 'text-sky-300' },
			{ label: 'Completed', value: weekStats.completed, color: 'text-emerald-300' },
			{ label: 'Notes Created', value: weekStats.notesCreated, color: 'text-violet-300' },
			{ label: 'Tags Active', value: weekStats.activeProjects, color: 'text-amber-300' }
		] as stat}
			<div class="rounded-xl border border-slate-800/60 bg-slate-900/40 p-3 text-center">
				<div class="text-xl font-bold {stat.color}">{stat.value}</div>
				<div class="mt-0.5 text-[10px] text-slate-500">{stat.label}</div>
			</div>
		{/each}
	</div>

	<!-- Generate Button -->
	<div class="mt-4">
		<button
			class="w-full rounded-xl bg-gradient-to-r from-sky-500 to-violet-500 px-4 py-3 text-sm font-semibold text-white transition hover:from-sky-400 hover:to-violet-400 disabled:opacity-50"
			on:click={generateDigest}
			disabled={generating || loading}
		>
			{#if generating}
				Generating digest...
			{:else if digest.length > 0}
				Regenerate Digest
			{:else}
				Generate {rangeLabel} Digest
			{/if}
		</button>
	</div>

	{#if loading}
		<div class="mt-6 rounded-xl border border-slate-800 p-6 text-center text-xs text-slate-400">
			Loading activity data...
		</div>
	{:else if digest.length > 0}
		<!-- Digest Sections -->
		<div class="mt-6 flex flex-col gap-4">
			{#each digest as section, i}
				<div class="rounded-xl border border-slate-800/60 bg-slate-900/40 p-4">
					<h3 class="text-sm font-semibold text-white">{section.title}</h3>
					<div class="mt-2 whitespace-pre-wrap text-xs leading-relaxed text-slate-300">
						{section.content}
					</div>
					{#if section.sources.length > 0}
						<div class="mt-3 flex flex-wrap gap-1.5">
							{#each section.sources as source}
								<a
									href={itemLink(source)}
									class="inline-flex items-center gap-1 rounded-lg border border-slate-700/40 bg-slate-800/40 px-2 py-1 text-[10px] text-slate-300 transition hover:border-sky-500/30"
								>
									<span class="rounded px-1 py-0.5 text-[8px] font-medium {source.kind === 'task'
										? 'bg-violet-500/20 text-violet-300'
										: 'bg-sky-500/20 text-sky-300'}">
										{source.kind}
									</span>
									{source.title}
								</a>
							{/each}
						</div>
					{/if}
				</div>
			{/each}
		</div>

		<!-- Review Prompts -->
		{#if reviewPrompts.length > 0}
			<div class="mt-6 rounded-xl border border-amber-500/20 bg-amber-500/5 p-4">
				<h3 class="flex items-center gap-2 text-sm font-semibold text-amber-200">
					<svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"
							d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z" />
					</svg>
					Review Prompts
				</h3>
				<p class="mt-1 text-[10px] text-amber-300/60">
					Reflect on these to deepen your understanding and plan ahead.
				</p>
				<div class="mt-3 flex flex-col gap-2">
					{#each reviewPrompts as prompt, i}
						<div class="rounded-lg border border-amber-500/10 bg-amber-500/5 px-3 py-2 text-xs text-amber-100/80">
							{prompt}
						</div>
					{/each}
				</div>
			</div>
		{/if}
	{:else}
		<!-- Recent Activity Summary (before generating) -->
		<div class="mt-6">
			<h3 class="text-sm font-semibold text-white">Recent Activity</h3>
			<p class="mt-1 text-[10px] text-slate-500">Items captured in this period. Generate a digest for AI-powered insights.</p>

			{#if recentTasks.length === 0 && recentNotes.length === 0}
				<div class="mt-3 rounded-xl border border-dashed border-slate-800 p-6 text-center text-xs text-slate-400">
					No activity in this period. Try expanding the time range.
				</div>
			{:else}
				<div class="mt-3 flex flex-col gap-2">
					{#each [...recentTasks.slice(0, 5).map((t) => ({
						id: t.id, title: t.title, kind: 'task', status: t.status, date: t.created_at
					})), ...recentNotes.slice(0, 5).map((n) => ({
						id: n.id, title: n.title ?? 'Untitled', kind: 'note', status: null, date: n.created_at
					}))] as item (item.kind + item.id)}
						<div class="flex items-center gap-2 rounded-lg border border-slate-800/40 bg-slate-900/30 px-3 py-2">
							<span class="rounded px-1.5 py-0.5 text-[9px] font-medium {item.kind === 'task'
								? 'bg-violet-500/20 text-violet-300'
								: 'bg-sky-500/20 text-sky-300'}">
								{item.kind}
							</span>
							<span class="flex-1 truncate text-xs text-slate-300">{item.title}</span>
							{#if item.status}
								<span class="text-[9px] text-slate-500">{item.status}</span>
							{/if}
							<span class="text-[9px] text-slate-600">{new Date(item.date).toLocaleDateString()}</span>
						</div>
					{/each}
					{#if recentTasks.length + recentNotes.length > 10}
						<p class="text-[10px] text-slate-500">
							+ {recentTasks.length + recentNotes.length - 10} more items
						</p>
					{/if}
				</div>
			{/if}
		</div>
	{/if}
</div>
