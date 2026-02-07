<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { fade } from 'svelte/transition';
	import { getNode } from '$lib/api/nodes';
	import type { KnowledgeNode } from '$lib/api/types';

	export let enabled = true;

	type PreviewData = {
		type: 'note' | 'task';
		title: string;
		content?: string;
		status?: string;
		tags?: string[];
		loading: boolean;
	};

	let visible = false;
	let x = 0;
	let y = 0;
	let currentPreview: PreviewData | null = null;
	let currentLinkId: string | null = null;
	let hideTimeout: ReturnType<typeof setTimeout> | null = null;
	let showTimeout: ReturnType<typeof setTimeout> | null = null;

	const SHOW_DELAY = 400;
	const HIDE_DELAY = 200;

	function clearTimeouts() {
		if (hideTimeout) {
			clearTimeout(hideTimeout);
			hideTimeout = null;
		}
		if (showTimeout) {
			clearTimeout(showTimeout);
			showTimeout = null;
		}
	}

	async function handleMouseEnter(event: MouseEvent) {
		if (!enabled) return;

		const target = event.target as HTMLElement;
		const link = target.closest('a[href]') as HTMLAnchorElement | null;
		if (!link) return;

		const href = link.getAttribute('href');
		if (!href) return;

		// Match internal note links: /notes?note=<id> or /note/<id>
		const noteMatch = href.match(/\/notes?\??(?:note=)?([a-f0-9-]+)/i);
		// Match internal task links: /tasks?task=<id>
		const taskMatch = href.match(/\/tasks?\??(?:task=)?([a-f0-9-]+)/i);

		if (!noteMatch && !taskMatch) return;

		const linkId = noteMatch?.[1] ?? taskMatch?.[1];
		if (!linkId) return;

		clearTimeouts();

		// Delay showing preview
		showTimeout = setTimeout(async () => {
			if (currentLinkId === linkId && visible) return;

			currentLinkId = linkId;
			const rect = link.getBoundingClientRect();

			// Position preview above or below the link
			const viewportHeight = window.innerHeight;
			const spaceBelow = viewportHeight - rect.bottom;
			const preferBelow = spaceBelow > 200;

			x = Math.min(rect.left, window.innerWidth - 320);
			y = preferBelow ? rect.bottom + 8 : rect.top - 8;

			currentPreview = {
				type: noteMatch ? 'note' : 'task',
				title: 'Loading...',
				loading: true
			};
			visible = true;

			try {
				const node = await getNode(linkId);
				if (currentLinkId !== linkId) return;

				const isTask = node.kind === 'task';
				const meta = node.metadata ?? {};

				currentPreview = {
					type: isTask ? 'task' : 'note',
					title: node.title ?? 'Untitled',
					content: node.content?.slice(0, 200) ?? '',
					status: isTask ? (meta.status as string | undefined) : undefined,
					tags: node.tags,
					loading: false
				};
			} catch {
				if (currentLinkId === linkId) {
					currentPreview = {
						type: noteMatch ? 'note' : 'task',
						title: 'Unable to load',
						loading: false
					};
				}
			}
		}, SHOW_DELAY);
	}

	function handleMouseLeave() {
		clearTimeouts();
		hideTimeout = setTimeout(() => {
			visible = false;
			currentLinkId = null;
			currentPreview = null;
		}, HIDE_DELAY);
	}

	function handlePreviewMouseEnter() {
		clearTimeouts();
	}

	function handlePreviewMouseLeave() {
		handleMouseLeave();
	}

	onMount(() => {
		document.addEventListener('mouseover', handleMouseEnter);
		document.addEventListener('mouseout', handleMouseLeave);
	});

	onDestroy(() => {
		clearTimeouts();
		document.removeEventListener('mouseover', handleMouseEnter);
		document.removeEventListener('mouseout', handleMouseLeave);
	});

	const statusColors: Record<string, string> = {
		inbox: 'bg-slate-500',
		planned: 'bg-sky-500',
		in_progress: 'bg-amber-500',
		done: 'bg-emerald-500'
	};
</script>

{#if visible && currentPreview}
	<div
		class="fixed z-50 w-72 rounded-xl border border-slate-700 bg-slate-900/95 shadow-xl backdrop-blur-sm"
		style="left: {x}px; top: {y}px; transform: translateY({y < 200 ? '0' : '-100%'});"
		on:mouseenter={handlePreviewMouseEnter}
		on:mouseleave={handlePreviewMouseLeave}
		role="tooltip"
		transition:fade={{ duration: 100 }}
	>
		<div class="p-3">
			{#if currentPreview.loading}
				<div class="flex items-center gap-2 text-xs text-slate-400">
					<div class="h-4 w-4 animate-spin rounded-full border-2 border-slate-500 border-t-sky-500"></div>
					Loading...
				</div>
			{:else}
				<div class="flex items-start gap-2">
					<div class={`mt-1 h-2 w-2 shrink-0 rounded-full ${
						currentPreview.type === 'note'
							? 'bg-violet-500'
							: statusColors[currentPreview.status ?? 'inbox'] ?? 'bg-slate-500'
					}`}></div>
					<div class="min-w-0 flex-1">
						<div class="flex items-center gap-2">
							<span class="text-[9px] uppercase tracking-wide text-slate-500">
								{currentPreview.type === 'note' ? 'Note' : 'Task'}
							</span>
							{#if currentPreview.status}
								<span class="rounded bg-slate-800 px-1.5 py-0.5 text-[9px] text-slate-400">
									{currentPreview.status.replace('_', ' ')}
								</span>
							{/if}
						</div>
						<h4 class="mt-1 text-sm font-medium text-white">{currentPreview.title}</h4>
						{#if currentPreview.content}
							<p class="mt-1 line-clamp-3 text-xs text-slate-400">
								{currentPreview.content}
							</p>
						{/if}
						{#if currentPreview.tags && currentPreview.tags.length > 0}
							<div class="mt-2 flex flex-wrap gap-1">
								{#each currentPreview.tags.slice(0, 4) as tag}
									<span class="rounded bg-slate-800 px-1.5 py-0.5 text-[9px] text-slate-500">{tag}</span>
								{/each}
							</div>
						{/if}
					</div>
				</div>
			{/if}
		</div>
	</div>
{/if}
