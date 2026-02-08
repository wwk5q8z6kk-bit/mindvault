<script lang="ts">
	import { onMount } from 'svelte';
	import { pushToast } from '$lib/stores/toast';
	import {
		listAdapters,
		registerAdapter,
		removeAdapter,
		healthCheckAdapter,
		sendAdapterMessage,
		type AdapterConfig
	} from '$lib/api/adapters';

	let adapters: AdapterConfig[] = [];
	let loading = true;
	let showAddForm = false;
	let healthStatus: Record<string, boolean | null> = {};

	let newAdapter = {
		adapter_type: 'email',
		name: '',
		settings: {} as Record<string, string>
	};

	// Dynamic settings fields based on adapter type
	const settingsFields: Record<string, { key: string; label: string; placeholder: string }[]> = {
		email: [
			{ key: 'smtp_host', label: 'SMTP Host', placeholder: 'smtp.gmail.com' },
			{ key: 'smtp_port', label: 'SMTP Port', placeholder: '587' },
			{ key: 'smtp_user', label: 'SMTP User', placeholder: 'user@example.com' },
			{ key: 'smtp_pass', label: 'SMTP Password', placeholder: 'app-password' },
			{ key: 'from_address', label: 'From Address', placeholder: 'you@example.com' },
			{ key: 'default_to', label: 'Default To', placeholder: 'recipient@example.com' }
		],
		slack: [
			{
				key: 'webhook_url',
				label: 'Webhook URL',
				placeholder: 'https://hooks.slack.com/services/...'
			},
			{ key: 'bot_token', label: 'Bot Token (optional)', placeholder: 'xoxb-...' },
			{ key: 'channel_id', label: 'Channel ID (optional)', placeholder: 'C0123456789' }
		],
		discord: [
			{
				key: 'webhook_url',
				label: 'Webhook URL',
				placeholder: 'https://discord.com/api/webhooks/...'
			}
		]
	};

	// Send message form state
	let sendFormAdapterId: string | null = null;
	let sendChannel = '';
	let sendContent = '';
	let sending = false;

	onMount(async () => {
		await loadAdapters();
	});

	async function loadAdapters() {
		loading = true;
		try {
			adapters = await listAdapters();
		} catch {
			pushToast('Failed to load adapters', 'danger');
		} finally {
			loading = false;
		}
	}

	async function handleAdd() {
		if (!newAdapter.name.trim()) return;
		try {
			await registerAdapter({
				adapter_type: newAdapter.adapter_type,
				name: newAdapter.name,
				settings: newAdapter.settings
			});
			await loadAdapters();
			showAddForm = false;
			newAdapter = { adapter_type: 'email', name: '', settings: {} };
			pushToast('Adapter registered', 'success');
		} catch {
			pushToast('Failed to register adapter', 'danger');
		}
	}

	async function handleRemove(id: string) {
		try {
			await removeAdapter(id);
			adapters = adapters.filter((a) => a.id !== id);
			pushToast('Adapter removed', 'success');
		} catch {
			pushToast('Failed to remove adapter', 'danger');
		}
	}

	async function handleHealthCheck(id: string) {
		healthStatus = { ...healthStatus, [id]: null };
		try {
			const res = await healthCheckAdapter(id);
			healthStatus = { ...healthStatus, [id]: res.healthy };
		} catch {
			healthStatus = { ...healthStatus, [id]: false };
		}
	}

	async function handleSend() {
		if (!sendFormAdapterId || !sendContent.trim()) return;
		sending = true;
		try {
			await sendAdapterMessage(sendFormAdapterId, {
				channel: sendChannel,
				content: sendContent
			});
			pushToast('Message sent', 'success');
			sendContent = '';
		} catch {
			pushToast('Failed to send message', 'danger');
		} finally {
			sending = false;
		}
	}

	function adapterTypeLabel(t: string): string {
		switch (t) {
			case 'email':
				return 'Email (SMTP)';
			case 'slack':
				return 'Slack';
			case 'discord':
				return 'Discord';
			default:
				return t;
		}
	}

	function currentSettingsFields() {
		return settingsFields[newAdapter.adapter_type] ?? [];
	}
</script>

<div class="mx-auto max-w-4xl space-y-6 p-6">
	<h1 class="text-2xl font-bold text-[rgb(var(--mv-text))]">Adapters</h1>
	<p class="text-sm text-[rgb(var(--mv-muted))]">
		Bridge external messaging platforms into the relay engine.
	</p>

	<!-- Adapter list -->
	<div class="space-y-3">
		<div class="flex items-center justify-between">
			<h2 class="text-lg font-semibold text-[rgb(var(--mv-text))]">
				Registered ({adapters.length})
			</h2>
			<button
				class="rounded bg-blue-600 px-3 py-1.5 text-sm text-white hover:bg-blue-700"
				onclick={() => (showAddForm = !showAddForm)}
			>{showAddForm ? 'Cancel' : '+ Add Adapter'}</button>
		</div>

		{#if showAddForm}
			<div
				class="space-y-3 rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))] p-4"
			>
				<div class="grid grid-cols-2 gap-3">
					<div>
						<label for="adapter-type" class="mb-1 block text-xs text-[rgb(var(--mv-muted))]"
							>Type</label
						>
						<select
							id="adapter-type"
							bind:value={newAdapter.adapter_type}
							onchange={() => (newAdapter.settings = {})}
							class="w-full rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-bg))] px-2 py-1 text-sm text-[rgb(var(--mv-text))]"
						>
							<option value="email">Email (SMTP)</option>
							<option value="slack">Slack</option>
							<option value="discord">Discord</option>
						</select>
					</div>
					<div>
						<label for="adapter-name" class="mb-1 block text-xs text-[rgb(var(--mv-muted))]"
							>Name</label
						>
						<input
							id="adapter-name"
							bind:value={newAdapter.name}
							class="w-full rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-bg))] px-2 py-1 text-sm text-[rgb(var(--mv-text))]"
							placeholder="My Email Adapter"
						/>
					</div>
				</div>

				<!-- Dynamic settings fields -->
				<div class="grid grid-cols-2 gap-3">
					{#each currentSettingsFields() as field (field.key)}
						<div>
							<label for="setting-{field.key}" class="mb-1 block text-xs text-[rgb(var(--mv-muted))]"
								>{field.label}</label
							>
							<input
								id="setting-{field.key}"
								bind:value={newAdapter.settings[field.key]}
								class="w-full rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-bg))] px-2 py-1 text-sm text-[rgb(var(--mv-text))]"
								placeholder={field.placeholder}
								type={field.key.includes('pass') ? 'password' : 'text'}
							/>
						</div>
					{/each}
				</div>

				<button
					class="rounded bg-blue-600 px-3 py-1.5 text-sm text-white hover:bg-blue-700"
					onclick={handleAdd}>Register</button
				>
			</div>
		{/if}

		{#if loading}
			<p class="text-[rgb(var(--mv-muted))]">Loading...</p>
		{:else if adapters.length === 0}
			<div
				class="rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))] p-6 text-center"
			>
				<p class="text-[rgb(var(--mv-muted))]">No adapters registered.</p>
				<p class="mt-1 text-xs text-[rgb(var(--mv-muted))]">
					Register an adapter to bridge external platforms into your relay.
				</p>
			</div>
		{:else}
			{#each adapters as adapter (adapter.id)}
				<div
					class="rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))] p-4"
				>
					<div class="flex items-center justify-between">
						<div>
							<div class="flex items-center gap-2">
								<span class="font-medium text-[rgb(var(--mv-text))]">{adapter.name}</span>
								<span
									class="rounded bg-[rgb(var(--mv-hover))] px-1.5 py-0.5 text-xs text-[rgb(var(--mv-muted))]"
									>{adapterTypeLabel(adapter.adapter_type)}</span
								>
							</div>
							<div class="mt-0.5 text-xs text-[rgb(var(--mv-muted))]">
								Added {new Date(adapter.created_at).toLocaleDateString()}
								{#if !adapter.enabled}
									&middot; <span class="text-amber-400">Disabled</span>
								{/if}
							</div>
						</div>
						<div class="flex items-center gap-2">
							{#if healthStatus[adapter.id] === true}
								<span class="text-xs text-green-400">Healthy</span>
							{:else if healthStatus[adapter.id] === false}
								<span class="text-xs text-red-400">Unhealthy</span>
							{/if}
							<button
								class="rounded px-2 py-1 text-xs text-blue-400 hover:bg-blue-500/10"
								onclick={() => handleHealthCheck(adapter.id)}>Check</button
							>
							<button
								class="rounded px-2 py-1 text-xs text-sky-400 hover:bg-sky-500/10"
								onclick={() => {
									sendFormAdapterId = sendFormAdapterId === adapter.id ? null : adapter.id;
								}}>Send</button
							>
							<button
								class="rounded px-2 py-1 text-xs text-red-400 hover:bg-red-500/10"
								onclick={() => handleRemove(adapter.id)}>Remove</button
							>
						</div>
					</div>

					<!-- Inline send form -->
					{#if sendFormAdapterId === adapter.id}
						<div class="mt-3 flex gap-2 border-t border-[rgb(var(--mv-border))] pt-3">
							<input
								bind:value={sendChannel}
								class="w-40 rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-bg))] px-2 py-1 text-sm text-[rgb(var(--mv-text))]"
								placeholder={adapter.adapter_type === 'email'
									? 'to@email.com'
									: '#channel'}
							/>
							<input
								bind:value={sendContent}
								class="flex-1 rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-bg))] px-2 py-1 text-sm text-[rgb(var(--mv-text))]"
								placeholder="Message content..."
								onkeydown={(e) => {
									if (e.key === 'Enter') handleSend();
								}}
							/>
							<button
								class="rounded bg-blue-600 px-3 py-1 text-sm text-white hover:bg-blue-700 disabled:opacity-50"
								onclick={handleSend}
								disabled={sending || !sendContent.trim()}
							>{sending ? 'Sending...' : 'Send'}</button>
						</div>
					{/if}
				</div>
			{/each}
		{/if}
	</div>
</div>
