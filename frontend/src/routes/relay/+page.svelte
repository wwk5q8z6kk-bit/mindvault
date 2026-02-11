<script lang="ts">
	import { onMount, tick } from 'svelte';
	import { pushToast } from '$lib/stores/toast';
	import { listProposals, approveProposal, rejectProposal, type Proposal } from '$lib/api/exchange';
	import { addBlockedSender } from '$lib/api/safeguards';
	import {
		listContacts,
		listChannels,
		listMessages,
		sendMessage,
		createContact,
		createChannel,
		updateContact,
		markRead,
		getUnreadCount,
		type RelayContact,
		type RelayChannel,
		type RelayMessage
	} from '$lib/api/relay';

	let contacts: RelayContact[] = [];
	let channels: RelayChannel[] = [];
	let selectedChannelId = '';
	let messages: RelayMessage[] = [];
	let input = '';
	let loading = false;
	let loadingMessages = false;
	let messagesContainer: HTMLDivElement | null = null;
	let inputEl: HTMLTextAreaElement | null = null;
	let showNewContact = false;
	let newContactName = '';
	let newContactKey = '';
	let newContactAddress = '';
	let newContactTrust: RelayContact['trust_level'] = 'relay_only';
	let newContactNotes = '';
	let unreadTotal = 0;
	let relayReplyProposals: RelayReplyProposal[] = [];
	let expandedSuggestionMessageId: string | null = null;
	let pendingSuggestionProposalId: string | null = null;
	let pendingSuggestionMessageId: string | null = null;
	let actingProposalId: string | null = null;
	let blockingMessageId: string | null = null;

	type RelayReplyProposal = {
		id: string;
		channelId: string;
		basisMessageId: string;
		suggestion: string;
		confidence: number;
		contextSnippets: string[];
		createdAt: string;
	};

	function parseString(value: unknown): string | null {
		if (typeof value === 'string') {
			const trimmed = value.trim();
			return trimmed.length > 0 ? trimmed : null;
		}
		return null;
	}

	function parseStringArray(value: unknown): string[] {
		if (!Array.isArray(value)) return [];
		return value.map((item) => (typeof item === 'string' ? item : '')).filter((item) => item.length > 0);
	}

	function extractRelayReplyProposal(proposal: Proposal): RelayReplyProposal | null {
		if (proposal.action !== 'relay.reply') return null;
		const payload = proposal.payload ?? {};
		const channelId = parseString((payload as Record<string, unknown>)['channel_id']);
		const basisMessageId = parseString((payload as Record<string, unknown>)['basis_message_id']);
		if (!channelId || !basisMessageId) return null;
		const suggestion =
			parseString((payload as Record<string, unknown>)['content']) ?? proposal.diff_preview ?? '';
		return {
			id: proposal.id,
			channelId,
			basisMessageId,
			suggestion,
			confidence: proposal.confidence,
			contextSnippets: parseStringArray((payload as Record<string, unknown>)['context_snippets']),
			createdAt: proposal.created_at
		};
	}

	$: relayReplyMap = new Map(
		relayReplyProposals
			.filter((proposal) => !selectedChannelId || proposal.channelId === selectedChannelId)
			.map((proposal) => [proposal.basisMessageId, proposal])
	);

	$: pendingSuggestionMessage = pendingSuggestionMessageId
		? messages.find((m) => m.id === pendingSuggestionMessageId) ?? null
		: null;

	let showContactSettings = false;
	let editingContactId = '';
	let contactEditor = {
		display_name: '',
		vault_address: '',
		trust_level: 'relay_only' as RelayContact['trust_level'],
		notes: ''
	};

	$: activeChannel = channels.find((c) => c.id === selectedChannelId) ?? null;
	$: activeContact =
		activeChannel && activeChannel.channel_type === 'direct' && activeChannel.member_contact_ids.length > 0
			? contacts.find((c) => c.id === activeChannel.member_contact_ids[0]) ?? null
			: null;

	$: if (activeContact && activeContact.id !== editingContactId) {
		editingContactId = activeContact.id;
		contactEditor = {
			display_name: activeContact.display_name,
			vault_address: activeContact.vault_address ?? '',
			trust_level: activeContact.trust_level,
			notes: activeContact.notes ?? ''
		};
	}

	onMount(async () => {
		await loadData();
	});

	async function loadData() {
		loading = true;
		try {
			[contacts, channels] = await Promise.all([listContacts(), listChannels()]);
			unreadTotal = await getUnreadCount();
			if (channels.length > 0 && !selectedChannelId) {
				selectedChannelId = channels[0].id;
				await loadMessages();
			}
		} catch {
			pushToast('Failed to load relay data', 'danger');
		} finally {
			loading = false;
		}
	}

	async function loadMessages() {
		if (!selectedChannelId) return;
		loadingMessages = true;
		try {
			messages = await listMessages(selectedChannelId);
			// Mark inbound unread messages as read
			for (const msg of messages) {
				if (msg.direction === 'inbound' && msg.status !== 'read') {
					await markRead(msg.id);
				}
			}
			await loadRelayProposals();
			await scrollToBottom();
		} catch {
			pushToast('Failed to load messages', 'danger');
		} finally {
			loadingMessages = false;
		}
	}

	async function loadRelayProposals() {
		try {
			const proposals = await listProposals('pending', 50, 0);
			relayReplyProposals = proposals
				.map(extractRelayReplyProposal)
				.filter((proposal): proposal is RelayReplyProposal => Boolean(proposal));
		} catch {
			relayReplyProposals = [];
		}
	}

	async function scrollToBottom() {
		await tick();
		if (messagesContainer) {
			messagesContainer.scrollTop = messagesContainer.scrollHeight;
		}
	}

	async function selectChannel(channelId: string) {
		if (pendingSuggestionProposalId) {
			pendingSuggestionProposalId = null;
			pendingSuggestionMessageId = null;
			input = '';
			pushToast('Cleared pending suggestion after channel switch', 'info');
		}
		selectedChannelId = channelId;
		await loadMessages();
		await tick();
		inputEl?.focus();
	}

	async function handleSend() {
		const text = input.trim();
		if (!text || !selectedChannelId) return;

		const pendingProposalId = pendingSuggestionProposalId;
		input = '';
		try {
			const sent = await sendMessage(selectedChannelId, text);
			messages = [...messages, sent];
			if (pendingProposalId) {
				try {
					await rejectProposal(pendingProposalId);
					await loadRelayProposals();
					pushToast('Suggestion dismissed after manual reply', 'info');
				} catch {
					pushToast('Sent reply, but failed to dismiss suggestion', 'warning');
				} finally {
					pendingSuggestionProposalId = null;
					pendingSuggestionMessageId = null;
				}
			}
			await scrollToBottom();
		} catch {
			pushToast('Failed to send message', 'danger');
		}
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter' && !e.shiftKey) {
			e.preventDefault();
			handleSend();
		}
	}

	function suggestionForMessage(message: RelayMessage): RelayReplyProposal | null {
		return relayReplyMap.get(message.id) ?? null;
	}

	async function sendSuggestedReply(proposal: RelayReplyProposal) {
		actingProposalId = proposal.id;
		try {
			await approveProposal(proposal.id);
			await loadMessages();
			pushToast('Suggestion sent', 'success');
		} catch {
			pushToast('Failed to send suggestion', 'danger');
		} finally {
			actingProposalId = null;
		}
	}

	async function editSuggestedReply(proposal: RelayReplyProposal) {
		pendingSuggestionProposalId = proposal.id;
		pendingSuggestionMessageId = proposal.basisMessageId;
		input = proposal.suggestion;
		await tick();
		inputEl?.focus();
	}

	async function dismissSuggestedReply(proposal: RelayReplyProposal) {
		actingProposalId = proposal.id;
		try {
			await rejectProposal(proposal.id);
			await loadRelayProposals();
			pushToast('Suggestion dismissed', 'info');
		} catch {
			pushToast('Failed to dismiss suggestion', 'danger');
		} finally {
			actingProposalId = null;
		}
	}

	async function blockSender(message: RelayMessage) {
		if (!message.sender_contact_id) {
			pushToast('No sender to block', 'warning');
			return;
		}
		const contact = contacts.find((c) => c.id === message.sender_contact_id);
		if (!contact) {
			pushToast('Sender contact not found', 'warning');
			return;
		}
		const pattern = contact.vault_address?.trim() || contact.public_key?.trim() || contact.display_name;
		blockingMessageId = message.id;
		try {
			await addBlockedSender({
				sender_type: 'relay',
				sender_pattern: pattern,
				reason: `Blocked from relay by ${contact.display_name}`
			});
			pushToast(`Blocked ${contact.display_name}`, 'success');
		} catch {
			pushToast('Failed to block sender', 'danger');
		} finally {
			blockingMessageId = null;
		}
	}

	async function handleAddContact() {
		if (!newContactName.trim() || !newContactKey.trim()) return;
		try {
			const contact = await createContact({
				display_name: newContactName.trim(),
				public_key: newContactKey.trim(),
				vault_address: newContactAddress.trim() || undefined,
				trust_level: newContactTrust,
				notes: newContactNotes.trim() || undefined
			});
			contacts = [...contacts, contact];

			// Auto-create a direct channel
			const channel = await createChannel({
				name: contact.display_name,
				channel_type: 'direct',
				member_contact_ids: [contact.id]
			});
			channels = [...channels, channel];

			newContactName = '';
			newContactKey = '';
			newContactAddress = '';
			newContactTrust = 'relay_only';
			newContactNotes = '';
			showNewContact = false;
			selectedChannelId = channel.id;
			await loadMessages();
			pushToast(`Added contact: ${contact.display_name}`, 'success');
		} catch {
			pushToast('Failed to add contact', 'danger');
		}
	}

	async function saveContactSettings() {
		if (!activeContact) return;
		try {
			const updated = await updateContact(activeContact.id, {
				display_name: contactEditor.display_name.trim(),
				vault_address: contactEditor.vault_address.trim() || undefined,
				trust_level: contactEditor.trust_level,
				notes: contactEditor.notes.trim() || undefined
			});
			contacts = contacts.map((c) => (c.id === updated.id ? updated : c));
			pushToast('Contact updated', 'success');
		} catch {
			pushToast('Failed to update contact', 'danger');
		}
	}

	function contactName(channelOrId: RelayChannel): string {
		if (channelOrId.name) return channelOrId.name;
		if (channelOrId.channel_type === 'direct' && channelOrId.member_contact_ids.length > 0) {
			const contact = contacts.find((c) => c.id === channelOrId.member_contact_ids[0]);
			return contact?.display_name ?? 'Unknown';
		}
		return 'Unnamed channel';
	}

	function senderName(msg: RelayMessage): string {
		if (msg.direction === 'outbound') return 'You';
		if (msg.sender_contact_id) {
			const contact = contacts.find((c) => c.id === msg.sender_contact_id);
			return contact?.display_name ?? 'Unknown';
		}
		return 'Unknown';
	}

	function formatTime(iso: string): string {
		const d = new Date(iso);
		const now = new Date();
		if (d.toDateString() === now.toDateString()) {
			return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
		}
		return d.toLocaleDateString([], { month: 'short', day: 'numeric' }) + ' ' +
			d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
	}
</script>

<div class="flex h-full">
	<!-- Sidebar: Channels + Contacts -->
	<div class="flex w-64 flex-shrink-0 flex-col border-r border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]">
		<div class="flex items-center justify-between border-b border-[rgb(var(--mv-border))] p-3">
			<h2 class="text-sm font-semibold text-[rgb(var(--mv-text))]">
				Relay
				{#if unreadTotal > 0}
					<span class="ml-1 rounded-full bg-blue-500 px-1.5 py-0.5 text-[10px] text-white">{unreadTotal}</span>
				{/if}
			</h2>
			<button
				class="rounded p-1 text-xs text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-hover))]"
				onclick={() => (showNewContact = !showNewContact)}
			>
				+ New
			</button>
		</div>

		{#if showNewContact}
			<div class="border-b border-[rgb(var(--mv-border))] p-3">
				<input
					class="mb-2 w-full rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-bg))] px-2 py-1 text-xs text-[rgb(var(--mv-text))]"
					placeholder="Display name"
					bind:value={newContactName}
				/>
				<input
					class="mb-2 w-full rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-bg))] px-2 py-1 text-xs text-[rgb(var(--mv-text))]"
					placeholder="Public key"
					bind:value={newContactKey}
				/>
				<input
					class="mb-2 w-full rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-bg))] px-2 py-1 text-xs text-[rgb(var(--mv-text))]"
					placeholder="Vault address (optional)"
					bind:value={newContactAddress}
				/>
				<select
					class="mb-2 w-full rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-bg))] px-2 py-1 text-xs text-[rgb(var(--mv-text))]"
					bind:value={newContactTrust}
				>
					<option value="relay_only">Relay only</option>
					<option value="context_inject">Context inject</option>
					<option value="full">Full autonomy</option>
				</select>
				<textarea
					class="mb-2 w-full rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-bg))] px-2 py-1 text-xs text-[rgb(var(--mv-text))]"
					placeholder="Notes (optional)"
					rows="2"
					bind:value={newContactNotes}
				></textarea>
				<button
					class="w-full rounded bg-blue-600 px-2 py-1 text-xs text-white hover:bg-blue-700"
					onclick={handleAddContact}
				>
					Add Contact
				</button>
			</div>
		{/if}

		<div class="flex-1 overflow-y-auto">
			{#if loading}
				<p class="p-3 text-xs text-[rgb(var(--mv-muted))]">Loading...</p>
			{:else if channels.length === 0}
				<p class="p-3 text-xs text-[rgb(var(--mv-muted))]">No channels yet. Add a contact to start.</p>
			{:else}
				{#each channels as channel (channel.id)}
					<button
						class="flex w-full items-center gap-2 px-3 py-2 text-left text-sm transition-colors hover:bg-[rgb(var(--mv-hover))]"
						class:bg-[rgb(var(--mv-hover))]={channel.id === selectedChannelId}
						onclick={() => selectChannel(channel.id)}
					>
						<span class="h-8 w-8 flex-shrink-0 rounded-full bg-[rgb(var(--mv-border))] flex items-center justify-center text-xs font-medium text-[rgb(var(--mv-text))]">
							{contactName(channel).charAt(0).toUpperCase()}
						</span>
						<div class="min-w-0 flex-1">
							<div class="truncate text-[rgb(var(--mv-text))]">{contactName(channel)}</div>
							<div class="truncate text-xs text-[rgb(var(--mv-muted))]">
								{channel.channel_type === 'group' ? `${channel.member_contact_ids.length} members` : 'Direct'}
							</div>
						</div>
					</button>
				{/each}
			{/if}
		</div>
	</div>

	<!-- Main: Messages -->
	<div class="flex flex-1 flex-col">
		{#if !selectedChannelId}
			<div class="flex flex-1 items-center justify-center text-[rgb(var(--mv-muted))]">
				<p>Select a channel to start messaging</p>
			</div>
		{:else}
			<!-- Channel header -->
			<div class="flex items-center border-b border-[rgb(var(--mv-border))] px-4 py-3">
				<h3 class="font-medium text-[rgb(var(--mv-text))]">
					{contactName(channels.find((c) => c.id === selectedChannelId) ?? channels[0])}
				</h3>
				{#if activeContact}
					<button
						class="ml-auto rounded border border-[rgb(var(--mv-border))] px-2 py-1 text-xs text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-hover))]"
						onclick={() => (showContactSettings = !showContactSettings)}
					>
						{showContactSettings ? 'Hide settings' : 'Contact settings'}
					</button>
				{/if}
			</div>

			{#if showContactSettings && activeContact}
				<div class="border-b border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))] px-4 py-3">
					<div class="grid gap-2 md:grid-cols-2">
						<div>
							<label class="text-[10px] uppercase tracking-wide text-[rgb(var(--mv-muted))]"
								>Display name</label
							>
							<input
								class="mt-1 w-full rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-bg))] px-2 py-1 text-xs text-[rgb(var(--mv-text))]"
								bind:value={contactEditor.display_name}
							/>
						</div>
						<div>
							<label class="text-[10px] uppercase tracking-wide text-[rgb(var(--mv-muted))]"
								>Vault address</label
							>
							<input
								class="mt-1 w-full rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-bg))] px-2 py-1 text-xs text-[rgb(var(--mv-text))]"
								placeholder="mailto:..."
								bind:value={contactEditor.vault_address}
							/>
						</div>
						<div>
							<label class="text-[10px] uppercase tracking-wide text-[rgb(var(--mv-muted))]"
								>Trust level</label
							>
							<select
								class="mt-1 w-full rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-bg))] px-2 py-1 text-xs text-[rgb(var(--mv-text))]"
								bind:value={contactEditor.trust_level}
							>
								<option value="relay_only">Relay only</option>
								<option value="context_inject">Context inject</option>
								<option value="full">Full autonomy</option>
							</select>
						</div>
						<div>
							<label class="text-[10px] uppercase tracking-wide text-[rgb(var(--mv-muted))]"
								>Notes</label
							>
							<textarea
								class="mt-1 w-full rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-bg))] px-2 py-1 text-xs text-[rgb(var(--mv-text))]"
								rows="2"
								bind:value={contactEditor.notes}
							></textarea>
						</div>
					</div>
					<div class="mt-3 flex gap-2">
						<button
							class="rounded bg-blue-600 px-3 py-1.5 text-xs text-white hover:bg-blue-700"
							onclick={saveContactSettings}
						>
							Save settings
						</button>
						<button
							class="rounded border border-[rgb(var(--mv-border))] px-3 py-1.5 text-xs text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-hover))]"
							onclick={() => (showContactSettings = false)}
						>
							Close
						</button>
					</div>
				</div>
			{/if}

			<!-- Messages list -->
			<div
				bind:this={messagesContainer}
				class="flex flex-1 flex-col gap-2 overflow-y-auto p-4"
			>
				{#if loadingMessages}
					<p class="text-center text-sm text-[rgb(var(--mv-muted))]">Loading messages...</p>
				{:else if messages.length === 0}
					<p class="text-center text-sm text-[rgb(var(--mv-muted))]">No messages yet. Send the first one!</p>
				{:else}
					{#each messages as msg (msg.id)}
						<div
							class="flex flex-col {msg.direction === 'outbound' ? 'items-end' : 'items-start'}"
						>
							<div class="mb-0.5 text-xs text-[rgb(var(--mv-muted))]">
								{senderName(msg)} &middot; {formatTime(msg.created_at)}
							</div>
							<div
								class="max-w-[70%] rounded-lg px-3 py-2 text-sm {msg.direction === 'outbound'
									? 'bg-blue-600 text-white'
									: 'bg-[rgb(var(--mv-hover))] text-[rgb(var(--mv-text))]'}"
							>
								{msg.content}
							</div>
							{#if msg.status === 'failed'}
								<div class="mt-0.5 text-xs text-red-400">Failed to send</div>
							{:else if msg.direction === 'outbound' && msg.status === 'delivered'}
								<div class="mt-0.5 text-xs text-[rgb(var(--mv-muted))]">Delivered</div>
							{:else if msg.direction === 'outbound' && msg.status === 'read'}
								<div class="mt-0.5 text-xs text-blue-400">Read</div>
							{:else if msg.direction === 'inbound' && msg.status === 'deferred'}
								<div class="mt-0.5 text-xs text-amber-400">Deferred</div>
							{:else if msg.direction === 'inbound' && msg.status === 'auto_replied'}
								<div class="mt-0.5 text-xs text-emerald-400">Auto-replied</div>
							{/if}

							{#if msg.direction === 'inbound'}
								{@const suggestion = suggestionForMessage(msg)}
								{#if suggestion}
									<div class="mt-2 w-full max-w-[70%] rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))] px-3 py-2">
										<div class="flex items-center justify-between text-[11px] text-[rgb(var(--mv-muted))]">
											<span class="font-medium text-[rgb(var(--mv-text))]">Suggested reply</span>
											<span>{Math.round(suggestion.confidence * 100)}% confidence</span>
										</div>
										<div class="mt-2 whitespace-pre-wrap text-xs text-[rgb(var(--mv-text))]">
											{suggestion.suggestion}
										</div>
										{#if suggestion.contextSnippets.length > 0}
											<button
												class="mt-2 text-[11px] text-blue-400 hover:text-blue-300"
												onclick={() =>
													(expandedSuggestionMessageId =
														expandedSuggestionMessageId === msg.id ? null : msg.id)}
											>
												{expandedSuggestionMessageId === msg.id ? 'Hide context' : 'Show context'}
											</button>
											{#if expandedSuggestionMessageId === msg.id}
												<div class="mt-2 space-y-1">
													{#each suggestion.contextSnippets as snippet}
														<div class="rounded bg-[rgb(var(--mv-hover))] px-2 py-1 text-[11px] text-[rgb(var(--mv-muted))]">
															{snippet}
														</div>
													{/each}
												</div>
											{/if}
										{/if}
										<div class="mt-2 flex flex-wrap gap-2">
											<button
												class="rounded bg-emerald-600 px-2 py-1 text-[11px] text-white hover:bg-emerald-500 disabled:opacity-50"
												onclick={() => sendSuggestedReply(suggestion)}
												disabled={actingProposalId === suggestion.id}
											>
												Send suggestion
											</button>
											<button
												class="rounded border border-[rgb(var(--mv-border))] px-2 py-1 text-[11px] text-[rgb(var(--mv-text))] hover:bg-[rgb(var(--mv-hover))]"
												onclick={() => editSuggestedReply(suggestion)}
											>
												Edit
											</button>
											<button
												class="rounded border border-[rgb(var(--mv-border))] px-2 py-1 text-[11px] text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-hover))] disabled:opacity-50"
												onclick={() => dismissSuggestedReply(suggestion)}
												disabled={actingProposalId === suggestion.id}
											>
												Dismiss
											</button>
											<button
												class="rounded border border-red-500/40 px-2 py-1 text-[11px] text-red-400 hover:bg-red-500/10 disabled:opacity-50"
												onclick={() => blockSender(msg)}
												disabled={blockingMessageId === msg.id}
											>
												Block sender
											</button>
										</div>
									</div>
								{/if}
							{/if}
						</div>
					{/each}
				{/if}
			</div>

			<!-- Compose area -->
			<div class="border-t border-[rgb(var(--mv-border))] p-3">
				{#if pendingSuggestionProposalId}
					<div class="mb-2 flex items-center justify-between rounded border border-blue-500/30 bg-blue-500/10 px-3 py-2 text-[11px] text-blue-200">
						<div>
							Editing suggestion for {pendingSuggestionMessage ? senderName(pendingSuggestionMessage) : 'message'}
						</div>
						<button
							class="text-blue-200 hover:text-blue-100"
							onclick={() => {
								pendingSuggestionProposalId = null;
								pendingSuggestionMessageId = null;
								input = '';
							}}
						>
							Clear
						</button>
					</div>
				{/if}
				<div class="flex gap-2">
					<textarea
						bind:this={inputEl}
						bind:value={input}
						onkeydown={handleKeydown}
						placeholder="Type a message..."
						rows="1"
						class="flex-1 resize-none rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-bg))] px-3 py-2 text-sm text-[rgb(var(--mv-text))] placeholder-[rgb(var(--mv-muted))] focus:border-blue-500 focus:outline-none"
					></textarea>
					<button
						class="rounded bg-blue-600 px-4 py-2 text-sm text-white hover:bg-blue-700 disabled:opacity-50"
						onclick={handleSend}
						disabled={!input.trim()}
					>
						Send
					</button>
				</div>
			</div>
		{/if}
	</div>
</div>
