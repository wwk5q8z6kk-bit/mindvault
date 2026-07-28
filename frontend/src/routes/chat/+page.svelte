<script lang="ts">
	import { onMount, tick } from 'svelte';
	import { page } from '$app/stores';
	import { get } from 'svelte/store';
	import {
		chat,
		type ChatMessage,
		type ChatProgressStage,
		type ChatSource
	} from '$lib/api/chat';
	import {
		listConversations,
		createConversation,
		deleteConversation,
		listConversationMessages,
		appendConversationMessage,
		type ConversationListItem
	} from '$lib/api/conversations';
	import { getNeighbors, type GraphNeighbor } from '$lib/api/graph';
	import { pushToast } from '$lib/stores/toast';
	import EmptyState from '$lib/components/EmptyState.svelte';
	import {
		formatRelevanceScore,
		sourceHref,
		splitCitationSegments
	} from '$lib/chat/citations';
	import { describeChatFailure } from '$lib/chat/errors';
	import { cacheChatSources, readCachedChatSources } from '$lib/chat/source-cache';
	import { resolveMessageSources } from '$lib/chat/message-sources';

	const HISTORY_WINDOW = 12;

	let conversations: ConversationListItem[] = [];
	let conversationsLoading = false;
	let activeConversationId = '';
	let messages: ChatMessage[] = [];
	let input = '';
	let loading = false;
	let loadingStage: ChatProgressStage | null = null;
	let loadingConversation = false;
	let messagesContainer: HTMLDivElement | null = null;
	let inputEl: HTMLTextAreaElement | null = null;
	let highlightedSource: number | null = null;

	let noteContextId: string | null = null;
	let noteContextLabel = '';
	let relatedNodes: GraphNeighbor[] = [];
	let relatedLoading = false;

	onMount(() => {
		const url = get(page).url;
		noteContextId = url.searchParams.get('node') ?? url.searchParams.get('note');
		if (noteContextId) {
			noteContextLabel = `Node ${noteContextId.slice(0, 8)}`;
		}
		void loadConversations();
		inputEl?.focus();
	});

	function generateId(): string {
		return Date.now().toString(36) + Math.random().toString(36).slice(2, 8);
	}

	function buildHistory(): Array<{ role: 'user' | 'assistant'; content: string }> {
		return messages
			.filter((m) => !m.isError)
			.slice(-HISTORY_WINDOW)
			.map((m) => ({ role: m.role as 'user' | 'assistant', content: m.content }));
	}

	async function scrollToBottom() {
		await tick();
		if (messagesContainer) {
			messagesContainer.scrollTo({ top: messagesContainer.scrollHeight, behavior: 'smooth' });
		}
	}

	async function loadConversations() {
		conversationsLoading = true;
		try {
			conversations = await listConversations(50, 0);
			if (conversations.length > 0 && !activeConversationId) {
				await selectConversation(conversations[0].id);
			}
		} catch {
			pushToast('Failed to load conversation history', 'warning');
		} finally {
			conversationsLoading = false;
		}
	}

	async function ensureConversation(): Promise<string | null> {
		if (activeConversationId) return activeConversationId;
		try {
			const title = noteContextId ? `Note ${noteContextId.slice(0, 8)}` : 'New chat';
			const created = await createConversation(title);
			activeConversationId = created.id;
			conversations = [
				{
					id: created.id,
					title: created.title,
					updated_at: new Date().toISOString()
				},
				...conversations
			];
			return created.id;
		} catch {
			pushToast('Failed to create conversation', 'danger');
			return null;
		}
	}

	async function selectConversation(id: string) {
		activeConversationId = id;
		loadingConversation = true;
		relatedNodes = [];
		highlightedSource = null;
		try {
			const history = await listConversationMessages(id, 200);
			messages = history.map((message) => ({
				id: message.id,
				role: message.role === 'assistant' ? 'assistant' : 'user',
				content: message.content,
				timestamp: message.created_at,
				sources: resolveMessageSources(message.sources, readCachedChatSources(message.id))
			}));
			const lastWithSources = [...messages].reverse().find((m) => m.sources && m.sources.length > 0);
			if (lastWithSources?.sources) {
				void loadRelatedNodesFromSources(lastWithSources.sources);
			}
			await scrollToBottom();
		} catch {
			pushToast('Failed to load conversation', 'danger');
		} finally {
			loadingConversation = false;
		}
	}

	async function startNewConversation() {
		activeConversationId = '';
		messages = [];
		relatedNodes = [];
		highlightedSource = null;
		const id = await ensureConversation();
		if (id) {
			activeConversationId = id;
		}
	}

	async function removeConversation(id: string) {
		if (!confirm('Delete this conversation?')) return;
		try {
			await deleteConversation(id);
			conversations = conversations.filter((conv) => conv.id !== id);
			if (activeConversationId === id) {
				activeConversationId = '';
				messages = [];
				relatedNodes = [];
				highlightedSource = null;
			}
		} catch {
			pushToast('Failed to delete conversation', 'danger');
		}
	}

	async function persistMessage(
		role: 'user' | 'assistant',
		content: string,
		sources?: ChatSource[]
	): Promise<string | null> {
		if (!activeConversationId) return null;
		try {
			const saved = await appendConversationMessage(
				activeConversationId,
				role,
				content,
				sources
			);
			// Keep local cache as a compatibility fallback for older clients/servers.
			if (sources && sources.length > 0 && saved.id) {
				cacheChatSources(saved.id, sources);
			}
			return saved.id;
		} catch {
			return null;
		}
	}

	async function loadRelatedNodesFromSources(sources: ChatSource[] | undefined) {
		if (!sources || sources.length === 0) {
			relatedNodes = [];
			return;
		}
		relatedLoading = true;
		try {
			const primary = sources[0];
			const neighbors = await getNeighbors(primary.node_id, 1);
			relatedNodes = neighbors.neighbors;
		} catch {
			relatedNodes = [];
		} finally {
			relatedLoading = false;
		}
	}

	async function sendMessage() {
		const text = input.trim();
		if (!text || loading) return;

		const conversationId = await ensureConversation();
		if (!conversationId) return;

		const userMsg: ChatMessage = {
			id: generateId(),
			role: 'user',
			content: text,
			timestamp: new Date().toISOString()
		};
		messages = [...messages, userMsg];
		input = '';
		loading = true;
		loadingStage = 'retrieving';
		highlightedSource = null;
		await scrollToBottom();
		void persistMessage('user', text);

		try {
			const history = buildHistory().slice(0, -1);
			const query = noteContextId
				? `Context node id: ${noteContextId}\nQuestion: ${text}`
				: text;
			const response = await chat(query, history, (stage) => {
				loadingStage = stage;
			});
			const assistantMsg: ChatMessage = {
				id: generateId(),
				role: 'assistant',
				content: response.answer,
				sources: response.sources,
				timestamp: new Date().toISOString(),
				mode: response.mode
			};
			messages = [...messages, assistantMsg];
			const persistedId = await persistMessage('assistant', response.answer, response.sources);
			if (persistedId) {
				assistantMsg.id = persistedId;
				messages = messages.map((m) => (m === assistantMsg ? { ...assistantMsg } : m));
			}
			void loadRelatedNodesFromSources(response.sources);
			if (!response.grounded) {
				pushToast('No matching vault sources found for that question', 'warning');
			}
			if (activeConversationId) {
				conversations = conversations.map((conv) =>
					conv.id === activeConversationId
						? { ...conv, updated_at: new Date().toISOString() }
						: conv
				);
			}
		} catch (error) {
			const detail = describeChatFailure(error);
			pushToast(detail, 'danger');
			const errorMsg: ChatMessage = {
				id: generateId(),
				role: 'assistant',
				content: detail,
				timestamp: new Date().toISOString(),
				isError: true
			};
			messages = [...messages, errorMsg];
		} finally {
			loading = false;
			loadingStage = null;
			await scrollToBottom();
			inputEl?.focus();
		}
	}

	function clearCurrentMessages() {
		messages = [];
		relatedNodes = [];
		highlightedSource = null;
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Enter' && !event.shiftKey) {
			event.preventDefault();
			void sendMessage();
		}
	}

	function kindBadgeClass(kind: string): string {
		switch (kind) {
			case 'task':
				return 'bg-violet-500/20 text-violet-300';
			case 'fact':
				return 'bg-sky-500/20 text-sky-300';
			case 'event':
				return 'bg-amber-500/20 text-amber-300';
			default:
				return 'bg-slate-500/20 text-slate-300';
		}
	}

	function kindLabel(kind: string): string {
		return kind === 'fact' ? 'note' : kind;
	}

	function focusSource(index: number) {
		highlightedSource = index;
		const el = document.getElementById(`chat-source-${index}`);
		el?.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
	}

	function applySuggestion(text: string) {
		input = text;
		inputEl?.focus();
	}

	function formatRelativeTime(dateStr: string): string {
		const diffMs = Date.now() - new Date(dateStr).getTime();
		const minutes = Math.floor(diffMs / 60000);
		if (minutes < 1) return 'just now';
		if (minutes < 60) return `${minutes}m`;
		const hours = Math.floor(minutes / 60);
		if (hours < 24) return `${hours}h`;
		return `${Math.floor(hours / 24)}d`;
	}

	$: historyTruncated = messages.filter((m) => !m.isError).length > HISTORY_WINDOW;
	$: loadingLabel =
		loadingStage === 'generating'
			? 'Writing an answer from your sources…'
			: 'Searching your knowledge base…';
</script>

<div class="grid gap-4 lg:grid-cols-[260px_1fr_280px]">
	<aside class="rounded-2xl border border-[rgb(var(--mv-border))]/60 bg-[rgb(var(--mv-panel))]/30 p-3">
		<div class="mb-2 flex items-center justify-between">
			<h3 class="text-xs font-semibold uppercase tracking-wide text-[rgb(var(--mv-muted))]">Conversations</h3>
			<button
				class="rounded border border-[rgb(var(--mv-border))] px-2 py-1 text-[10px] text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-panel-strong))]"
				on:click={startNewConversation}
			>
				New
			</button>
		</div>
		{#if conversationsLoading}
			<p class="text-xs text-[rgb(var(--mv-muted))]/70">Loading…</p>
		{:else if conversations.length === 0}
			<p class="text-xs text-[rgb(var(--mv-muted))]/70">No saved conversations yet.</p>
		{:else}
			<div class="space-y-1.5">
				{#each conversations as conv (conv.id)}
					<div
						role="button"
						tabindex="0"
						class={`w-full rounded-lg border px-2.5 py-2 text-left transition ${activeConversationId === conv.id ? 'border-sky-500/40 bg-sky-500/10' : 'border-[rgb(var(--mv-border))] hover:bg-[rgb(var(--mv-panel-strong))]/40'}`}
						on:click={() => selectConversation(conv.id)}
						on:keydown={(e) => (e.key === 'Enter' || e.key === ' ') && selectConversation(conv.id)}
					>
						<div class="flex items-center justify-between gap-2">
							<p class="truncate text-xs font-medium text-[rgb(var(--mv-text))]">
								{conv.title || `Conversation ${conv.id.slice(0, 8)}`}
							</p>
							<span class="text-[10px] text-[rgb(var(--mv-muted))]/60">{formatRelativeTime(conv.updated_at)}</span>
						</div>
						<div class="mt-1 flex justify-end">
							<button
								type="button"
								class="text-[10px] text-[rgb(var(--mv-muted))]/70 hover:text-red-300"
								on:click|stopPropagation={() => removeConversation(conv.id)}
							>
								Delete
							</button>
						</div>
					</div>
				{/each}
			</div>
		{/if}
	</aside>

	<div class="flex h-[calc(100vh-11rem)] flex-col">
		<div class="flex items-center justify-between gap-3">
			<div>
				<h2 class="text-lg font-semibold text-[rgb(var(--mv-text))]">AI Chat</h2>
				<p class="text-xs text-[rgb(var(--mv-muted))]/60">
					Ask questions grounded in your notes and tasks with clickable source citations.
				</p>
			</div>
			<div class="flex items-center gap-2">
				{#if noteContextId}
					<div class="rounded-lg border border-sky-500/30 bg-sky-500/10 px-2.5 py-1 text-[10px] text-sky-200">
						Context: {noteContextLabel}
					</div>
					<a
						href={`/notes?note=${noteContextId}`}
						class="rounded-lg border border-[rgb(var(--mv-border))] px-2.5 py-1 text-[10px] text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-panel-strong))]"
					>
						Open Note
					</a>
				{/if}
				{#if messages.length > 0}
					<button
						class="rounded-lg border border-[rgb(var(--mv-border))] px-3 py-1.5 text-xs text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-panel-strong))]"
						on:click={clearCurrentMessages}
					>
						Clear
					</button>
				{/if}
			</div>
		</div>

		{#if historyTruncated}
			<div class="mt-2 rounded-lg border border-amber-500/20 bg-amber-500/10 px-3 py-1.5 text-[11px] text-amber-100">
				Using the latest {HISTORY_WINDOW} messages as model context. Older turns stay visible but are not resent.
			</div>
		{/if}

		<div
			class="mt-3 flex-1 overflow-y-auto rounded-2xl border border-[rgb(var(--mv-border))]/60 bg-[rgb(var(--mv-panel))]/30 p-4"
			bind:this={messagesContainer}
		>
			{#if loadingConversation}
				<div class="flex h-full items-center justify-center text-xs text-[rgb(var(--mv-muted))]/70">Loading conversation...</div>
			{:else if messages.length === 0}
				<EmptyState
					icon="search"
					tone="sky"
					title="Ask your vault"
					description="Hybrid search finds relevant notes and tasks, then the assistant answers with citations you can open."
				>
					<button
						class="rounded-lg border border-[rgb(var(--mv-border))] px-3 py-1.5 text-xs text-[rgb(var(--mv-muted))] hover:border-sky-500/40 hover:text-[rgb(var(--mv-text))]"
						on:click={() => applySuggestion('What am I working on this week?')}
					>
						What am I working on this week?
					</button>
					<button
						class="rounded-lg border border-[rgb(var(--mv-border))] px-3 py-1.5 text-xs text-[rgb(var(--mv-muted))] hover:border-sky-500/40 hover:text-[rgb(var(--mv-text))]"
						on:click={() => applySuggestion('Summarize open decisions in my notes')}
					>
						Summarize open decisions
					</button>
				</EmptyState>
			{:else}
				<div class="flex flex-col gap-4">
					{#each messages as message (message.id)}
						<div class="flex gap-3 {message.role === 'user' ? 'justify-end' : 'justify-start'}">
							{#if message.role === 'assistant'}
								<div class="flex h-7 w-7 shrink-0 items-center justify-center rounded-lg bg-sky-500/20 text-[10px] font-bold text-sky-300">
									MV
								</div>
							{/if}
							<div
								class="max-w-[80%] {message.role === 'user'
									? 'rounded-2xl rounded-tr-md bg-sky-500/20 px-4 py-3 text-sky-100'
									: message.isError
										? 'rounded-2xl rounded-tl-md border border-rose-500/30 bg-rose-500/10 px-4 py-3 text-rose-100'
										: 'rounded-2xl rounded-tl-md bg-[rgb(var(--mv-panel-strong))]/60 px-4 py-3 text-[rgb(var(--mv-text))]/80'}"
							>
								{#if message.role === 'assistant' && !message.isError}
									<div class="whitespace-pre-wrap text-sm leading-relaxed">
										{#each splitCitationSegments(message.content) as segment, segIdx (segIdx)}
											{#if segment.type === 'text'}
												{segment.value}
											{:else if message.sources && message.sources[segment.index - 1]}
												<button
													type="button"
													class="mx-0.5 inline-flex translate-y-[-1px] items-center rounded bg-sky-500/20 px-1 py-0.5 font-mono text-[10px] font-semibold text-sky-300 underline decoration-sky-400/50 underline-offset-2 hover:bg-sky-500/30"
													title={`Open source ${segment.index}: ${message.sources[segment.index - 1].title}`}
													on:click={() => focusSource(segment.index)}
												>
													{segment.raw}
												</button>
											{:else}
												<span class="font-mono text-[10px] text-[rgb(var(--mv-muted))]">{segment.raw}</span>
											{/if}
										{/each}
									</div>
								{:else}
									<div class="whitespace-pre-wrap text-sm leading-relaxed">{message.content}</div>
								{/if}

								{#if message.role === 'assistant' && !message.isError}
									{#if message.sources && message.sources.length > 0}
										<div class="mt-3 border-t border-[rgb(var(--mv-border))]/40 pt-2">
											<div class="flex items-center justify-between gap-2">
												<p class="text-[10px] font-medium uppercase tracking-wider text-[rgb(var(--mv-muted))]/60">
													Sources ({message.sources.length})
												</p>
												{#if message.mode}
													<span class="text-[9px] uppercase tracking-wide text-[rgb(var(--mv-muted))]/40">
														{message.mode === 'native' ? 'native chat' : 'hybrid recall'}
													</span>
												{/if}
											</div>
											<div class="mt-1.5 flex flex-col gap-1.5">
												{#each message.sources as source, i}
													{@const relevance = formatRelevanceScore(source.score)}
													<a
														id={`chat-source-${i + 1}`}
														href={sourceHref(source.kind, source.node_id, source.title)}
														class={`group flex items-start gap-2 rounded-lg border px-2.5 py-2 text-[11px] transition ${
															highlightedSource === i + 1
																? 'border-sky-500/50 bg-sky-500/10'
																: 'border-[rgb(var(--mv-border))]/40 bg-[rgb(var(--mv-panel))]/40 hover:border-sky-500/40'
														}`}
													>
														<span class="mt-0.5 font-mono text-[rgb(var(--mv-muted))]/60">[{i + 1}]</span>
														<div class="min-w-0 flex-1">
															<div class="flex items-center gap-2">
																<span class="rounded px-1.5 py-0.5 text-[9px] font-medium {kindBadgeClass(source.kind)}">
																	{kindLabel(source.kind)}
																</span>
																<span class="truncate font-medium text-[rgb(var(--mv-text))] underline-offset-2 group-hover:underline">
																	{source.title}
																</span>
																<span class="ml-auto text-[rgb(var(--mv-muted))]/50 transition group-hover:text-sky-300">↗</span>
															</div>
															{#if source.preview}
																<p class="mt-1 line-clamp-2 text-[10px] text-[rgb(var(--mv-muted))]/70">
																	{source.preview}
																</p>
															{/if}
														</div>
														{#if relevance}
															<span class="mt-0.5 shrink-0 text-[9px] text-[rgb(var(--mv-muted))]/40">{relevance}</span>
														{/if}
													</a>
												{/each}
											</div>
										</div>
									{:else}
										<div class="mt-3 rounded-lg border border-amber-500/20 bg-amber-500/10 px-2.5 py-2 text-[11px] text-amber-100">
											No grounding sources were attached to this answer. Treat it as unverified.
										</div>
									{/if}
								{/if}
							</div>
							{#if message.role === 'user'}
								<div class="flex h-7 w-7 shrink-0 items-center justify-center rounded-lg bg-[rgb(var(--mv-panel-strong))] text-[10px] font-bold text-[rgb(var(--mv-muted))]">
									You
								</div>
							{/if}
						</div>
					{/each}

					{#if loading}
						<div class="flex gap-3">
							<div class="flex h-7 w-7 shrink-0 items-center justify-center rounded-lg bg-sky-500/20 text-[10px] font-bold text-sky-300">
								MV
							</div>
							<div class="rounded-2xl rounded-tl-md bg-[rgb(var(--mv-panel-strong))]/60 px-4 py-3">
								<div class="flex items-center gap-2" role="status" aria-live="polite" aria-label={loadingLabel}>
									<div class="flex items-center gap-1.5">
										<div class="h-2 w-2 animate-pulse rounded-full bg-sky-400"></div>
										<div class="h-2 w-2 animate-pulse rounded-full bg-sky-400" style="animation-delay: 0.15s"></div>
										<div class="h-2 w-2 animate-pulse rounded-full bg-sky-400" style="animation-delay: 0.3s"></div>
									</div>
									<span class="text-xs text-[rgb(var(--mv-muted))]">{loadingLabel}</span>
								</div>
							</div>
						</div>
					{/if}
				</div>
			{/if}
		</div>

		<div class="mt-3 flex gap-2">
			<textarea
				class="flex-1 resize-none rounded-xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))] px-4 py-3 text-sm text-[rgb(var(--mv-text))] placeholder-[rgb(var(--mv-muted))]/60 outline-none transition focus:border-sky-500"
				placeholder={noteContextId ? `Ask about ${noteContextLabel}...` : 'Ask a question about your knowledge...'}
				rows="2"
				bind:value={input}
				bind:this={inputEl}
				on:keydown={handleKeydown}
				disabled={loading}
			></textarea>
			<button
				class="self-end rounded-xl bg-sky-500 px-4 py-3 text-sm font-semibold text-white transition hover:bg-sky-400 disabled:opacity-50"
				on:click={sendMessage}
				disabled={loading || !input.trim()}
			>
				{loading ? '...' : 'Send'}
			</button>
		</div>
	</div>

	<aside class="rounded-2xl border border-[rgb(var(--mv-border))]/60 bg-[rgb(var(--mv-panel))]/30 p-4">
		<h3 class="text-sm font-semibold text-[rgb(var(--mv-text))]">Related Graph Nodes</h3>
		<p class="mt-1 text-[11px] text-[rgb(var(--mv-muted))]">Connections around the top cited source.</p>
		<div class="mt-3 space-y-2">
			{#if relatedLoading}
				<p class="text-xs text-[rgb(var(--mv-muted))]/70">Loading graph neighbors…</p>
			{:else if relatedNodes.length === 0}
				<p class="text-xs text-[rgb(var(--mv-muted))]/70">Cited sources will populate related context here.</p>
			{:else}
				{#each relatedNodes.slice(0, 12) as neighbor (neighbor.node.id + neighbor.relationship_kind)}
					<a
						href={sourceHref(neighbor.node.kind, neighbor.node.id, neighbor.node.title)}
						class="block rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))]/30 px-3 py-2 text-xs transition hover:border-sky-500/30"
					>
						<div class="flex items-center gap-2">
							<span class={`rounded-full px-1.5 py-0.5 text-[9px] ${kindBadgeClass(neighbor.node.kind)}`}>
								{kindLabel(neighbor.node.kind)}
							</span>
							<span class="truncate text-[rgb(var(--mv-text))]">{neighbor.node.title || 'Untitled'}</span>
						</div>
						<div class="mt-1 text-[10px] text-[rgb(var(--mv-muted))]/70">
							{neighbor.direction} via {neighbor.relationship_kind}
						</div>
					</a>
				{/each}
			{/if}
		</div>
	</aside>
</div>
