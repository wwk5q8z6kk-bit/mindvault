<script lang="ts">
	import { onMount, tick } from 'svelte';
	import { pushToast } from '$lib/stores/toast';
	import {
		listContacts,
		listChannels,
		listMessages,
		sendMessage,
		createContact,
		createChannel,
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
	let unreadTotal = 0;

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
			await scrollToBottom();
		} catch {
			pushToast('Failed to load messages', 'danger');
		} finally {
			loadingMessages = false;
		}
	}

	async function scrollToBottom() {
		await tick();
		if (messagesContainer) {
			messagesContainer.scrollTop = messagesContainer.scrollHeight;
		}
	}

	async function selectChannel(channelId: string) {
		selectedChannelId = channelId;
		await loadMessages();
		await tick();
		inputEl?.focus();
	}

	async function handleSend() {
		const text = input.trim();
		if (!text || !selectedChannelId) return;

		input = '';
		try {
			const sent = await sendMessage(selectedChannelId, text);
			messages = [...messages, sent];
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

	async function handleAddContact() {
		if (!newContactName.trim() || !newContactKey.trim()) return;
		try {
			const contact = await createContact({
				display_name: newContactName.trim(),
				public_key: newContactKey.trim(),
				vault_address: newContactAddress.trim() || undefined
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
			showNewContact = false;
			selectedChannelId = channel.id;
			await loadMessages();
			pushToast(`Added contact: ${contact.display_name}`, 'success');
		} catch {
			pushToast('Failed to add contact', 'danger');
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
			</div>

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
							{/if}
						</div>
					{/each}
				{/if}
			</div>

			<!-- Compose area -->
			<div class="border-t border-[rgb(var(--mv-border))] p-3">
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
