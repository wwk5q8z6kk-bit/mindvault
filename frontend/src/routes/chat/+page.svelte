<script lang="ts">
	import { onMount, tick } from 'svelte';
	import { chat, type ChatMessage, type ChatSource } from '$lib/api/chat';
	import { pushToast } from '$lib/stores/toast';

	let messages: ChatMessage[] = [];
	let input = '';
	let loading = false;
	let messagesContainer: HTMLDivElement | null = null;
	let inputEl: HTMLTextAreaElement | null = null;

	onMount(() => {
		inputEl?.focus();
	});

	function generateId(): string {
		return Date.now().toString(36) + Math.random().toString(36).slice(2, 8);
	}

	function buildHistory(): Array<{ role: 'user' | 'assistant'; content: string }> {
		return messages.slice(-10).map((m) => ({ role: m.role, content: m.content }));
	}

	async function scrollToBottom() {
		await tick();
		if (messagesContainer) {
			messagesContainer.scrollTop = messagesContainer.scrollHeight;
		}
	}

	async function sendMessage() {
		const text = input.trim();
		if (!text || loading) return;

		const userMsg: ChatMessage = {
			id: generateId(),
			role: 'user',
			content: text,
			timestamp: new Date().toISOString()
		};
		messages = [...messages, userMsg];
		input = '';
		loading = true;
		await scrollToBottom();

		try {
			const history = buildHistory().slice(0, -1);
			const response = await chat(text, history);
			const assistantMsg: ChatMessage = {
				id: generateId(),
				role: 'assistant',
				content: response.answer,
				sources: response.sources,
				timestamp: new Date().toISOString()
			};
			messages = [...messages, assistantMsg];
		} catch {
			pushToast('Failed to get response. Check your connection.', 'danger');
			const errorMsg: ChatMessage = {
				id: generateId(),
				role: 'assistant',
				content: 'Sorry, I was unable to process your question. Please try again.',
				timestamp: new Date().toISOString()
			};
			messages = [...messages, errorMsg];
		} finally {
			loading = false;
			await scrollToBottom();
			inputEl?.focus();
		}
	}

	function clearChat() {
		messages = [];
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Enter' && !event.shiftKey) {
			event.preventDefault();
			sendMessage();
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

	function sourceLink(source: ChatSource): string {
		if (source.kind === 'task') return `/tasks?task=${source.node_id}`;
		if (source.kind === 'fact') return `/notes?note=${source.node_id}`;
		return `/search?q=${encodeURIComponent(source.title)}`;
	}
</script>

<div class="flex h-[calc(100vh-10rem)] flex-col">
	<div class="flex items-center justify-between">
		<div>
			<h2 class="text-lg font-semibold text-white">AI Chat</h2>
			<p class="text-xs text-slate-400">Ask questions about your knowledge base. Answers cite your notes and tasks.</p>
		</div>
		{#if messages.length > 0}
			<button
				class="rounded-lg border border-slate-700 px-3 py-1.5 text-xs text-slate-300 hover:bg-slate-800"
				on:click={clearChat}
			>
				Clear chat
			</button>
		{/if}
	</div>

	<div
		class="mt-4 flex-1 overflow-y-auto rounded-2xl border border-slate-800/60 bg-slate-900/30 p-4"
		bind:this={messagesContainer}
	>
		{#if messages.length === 0}
			<div class="flex h-full flex-col items-center justify-center text-center">
				<div class="flex h-16 w-16 items-center justify-center rounded-2xl bg-sky-500/10 text-sky-300">
					<svg class="h-8 w-8" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"
							d="M8 10h.01M12 10h.01M16 10h.01M9 16H5a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v8a2 2 0 01-2 2h-5l-5 5v-5z" />
					</svg>
				</div>
				<h3 class="mt-4 text-sm font-semibold text-white">Chat with your knowledge</h3>
				<p class="mt-2 max-w-md text-xs text-slate-400">
					Ask questions and get answers grounded in your notes, tasks, and documents.
					Sources are cited so you can verify and explore further.
				</p>
				<div class="mt-4 flex flex-wrap justify-center gap-2">
					{#each [
						'What tasks are overdue?',
						'Summarize my recent notes',
						'What do I know about...',
						'Find connections between...'
					] as suggestion}
						<button
							class="rounded-lg border border-slate-700/60 bg-slate-800/40 px-3 py-1.5 text-[11px] text-slate-300 transition hover:border-sky-500/40 hover:text-sky-200"
							on:click={() => { input = suggestion; inputEl?.focus(); }}
						>
							{suggestion}
						</button>
					{/each}
				</div>
			</div>
		{:else}
			<div class="flex flex-col gap-4">
				{#each messages as message (message.id)}
					<div class="flex gap-3 {message.role === 'user' ? 'justify-end' : 'justify-start'}">
						{#if message.role === 'assistant'}
							<div class="flex h-7 w-7 shrink-0 items-center justify-center rounded-lg bg-sky-500/20 text-[10px] font-bold text-sky-300">
								MV
							</div>
						{/if}
						<div class="max-w-[80%] {message.role === 'user'
							? 'rounded-2xl rounded-tr-md bg-sky-500/20 px-4 py-3 text-sky-100'
							: 'rounded-2xl rounded-tl-md bg-slate-800/60 px-4 py-3 text-slate-200'}">
							<div class="whitespace-pre-wrap text-sm leading-relaxed">{message.content}</div>

							{#if message.sources && message.sources.length > 0}
								<div class="mt-3 border-t border-slate-700/40 pt-2">
									<p class="text-[10px] font-medium uppercase tracking-wider text-slate-500">Sources</p>
									<div class="mt-1.5 flex flex-col gap-1.5">
										{#each message.sources as source, i}
											<a
												href={sourceLink(source)}
												class="flex items-center gap-2 rounded-lg border border-slate-700/40 bg-slate-900/40 px-2.5 py-1.5 text-[11px] transition hover:border-sky-500/30"
											>
												<span class="font-mono text-slate-500">[{i + 1}]</span>
												<span class="rounded px-1.5 py-0.5 text-[9px] font-medium {kindBadgeClass(source.kind)}">
													{kindLabel(source.kind)}
												</span>
												<span class="truncate text-slate-300">{source.title}</span>
												<span class="ml-auto text-[9px] text-slate-600">{(source.score * 100).toFixed(0)}%</span>
											</a>
										{/each}
									</div>
								</div>
							{/if}
						</div>
						{#if message.role === 'user'}
							<div class="flex h-7 w-7 shrink-0 items-center justify-center rounded-lg bg-slate-700 text-[10px] font-bold text-slate-300">
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
						<div class="rounded-2xl rounded-tl-md bg-slate-800/60 px-4 py-3">
							<div class="flex items-center gap-1.5">
								<div class="h-2 w-2 animate-pulse rounded-full bg-sky-400"></div>
								<div class="h-2 w-2 animate-pulse rounded-full bg-sky-400" style="animation-delay: 0.15s"></div>
								<div class="h-2 w-2 animate-pulse rounded-full bg-sky-400" style="animation-delay: 0.3s"></div>
							</div>
						</div>
					</div>
				{/if}
			</div>
		{/if}
	</div>

	<div class="mt-3 flex gap-2">
		<textarea
			class="flex-1 resize-none rounded-xl border border-slate-700 bg-slate-900 px-4 py-3 text-sm text-white placeholder-slate-500 outline-none transition focus:border-sky-500"
			placeholder="Ask a question about your knowledge..."
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
