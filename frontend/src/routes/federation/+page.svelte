<script lang="ts">
	import { onMount } from 'svelte';
	import { pushToast } from '$lib/stores/toast';
	import {
		listPeers,
		addPeer,
		handshakePeer,
		removePeer,
		peerHealth,
		federatedQuery,
		type FederationPeer,
		type FederationHandshakeResponse,
		type FederatedResult
	} from '$lib/api/federation';

	let peers: FederationPeer[] = [];
	let loading = true;
	let showAddForm = false;
	let queryText = '';
	let queryResults: FederatedResult[] = [];
	let querying = false;
	let healthStatus: Record<string, boolean | null> = {};
	let handshaking = false;
	let lastHandshake: FederationHandshakeResponse | null = null;

	let newPeer = {
		vault_id: '',
		display_name: '',
		endpoint: '',
		max_results: 50
	};

	let handshakeForm = {
		endpoint: '',
		shared_secret: ''
	};

	onMount(async () => {
		await loadPeers();
	});

	async function loadPeers() {
		loading = true;
		try {
			const res = await listPeers();
			peers = res.peers;
		} catch {
			pushToast('Failed to load federation peers', 'danger');
		} finally {
			loading = false;
		}
	}

	async function handleAdd() {
		if (!newPeer.vault_id.trim() || !newPeer.display_name.trim() || !newPeer.endpoint.trim())
			return;
		try {
			await addPeer(newPeer);
			await loadPeers();
			showAddForm = false;
			newPeer = { vault_id: '', display_name: '', endpoint: '', max_results: 50 };
			pushToast('Peer added', 'success');
		} catch {
			pushToast('Failed to add peer', 'danger');
		}
	}

	async function handleHandshake() {
		if (!handshakeForm.endpoint.trim()) {
			pushToast('Endpoint URL is required', 'warning');
			return;
		}
		handshaking = true;
		try {
			const result = await handshakePeer({
				endpoint: handshakeForm.endpoint.trim(),
				shared_secret: handshakeForm.shared_secret.trim() || undefined
			});
			lastHandshake = result;
			await loadPeers();
			handshakeForm = { endpoint: '', shared_secret: '' };
			pushToast(`Handshake complete with ${result.display_name}`, 'success');
		} catch {
			pushToast('Federation handshake failed', 'danger');
		} finally {
			handshaking = false;
		}
	}

	async function handleRemove(id: string) {
		try {
			await removePeer(id);
			peers = peers.filter((p) => p.id !== id);
			pushToast('Peer removed', 'success');
		} catch {
			pushToast('Failed to remove peer', 'danger');
		}
	}

	async function handleHealthCheck(id: string) {
		healthStatus = { ...healthStatus, [id]: null };
		try {
			const res = await peerHealth(id);
			healthStatus = { ...healthStatus, [id]: res.healthy };
		} catch {
			healthStatus = { ...healthStatus, [id]: false };
		}
	}

	async function handleQuery() {
		if (!queryText.trim()) return;
		querying = true;
		try {
			const res = await federatedQuery(queryText);
			queryResults = res.results;
			if (queryResults.length === 0) {
				pushToast('No results from federated peers', 'info');
			}
		} catch {
			pushToast('Federated query failed', 'danger');
		} finally {
			querying = false;
		}
	}
</script>

<div class="mx-auto max-w-4xl space-y-6 p-6">
	<h1 class="text-2xl font-bold text-[rgb(var(--mv-text))]">Federation</h1>
	<p class="text-sm text-[rgb(var(--mv-muted))]">
		Connect with trusted peer vaults for read-only knowledge queries.
	</p>

	<!-- Peers list -->
	<div class="space-y-3">
		<div class="flex items-center justify-between">
			<h2 class="text-lg font-semibold text-[rgb(var(--mv-text))]">Peers</h2>
			<button
				class="rounded bg-blue-600 px-3 py-1.5 text-sm text-white hover:bg-blue-700"
				onclick={() => (showAddForm = !showAddForm)}
			>{showAddForm ? 'Cancel' : '+ Add Peer'}</button>
		</div>

		<div class="space-y-3 rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))] p-4">
			<h3 class="text-sm font-semibold text-[rgb(var(--mv-text))]">Handshake (recommended)</h3>
			<p class="text-xs text-[rgb(var(--mv-muted))]">
				Provide a peer endpoint to auto-discover vault identity and register the peer.
			</p>
			<div class="space-y-2">
				<input
					bind:value={handshakeForm.endpoint}
					class="w-full rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-bg))] px-2 py-1 text-sm text-[rgb(var(--mv-text))]"
					placeholder="http://192.168.1.50:9470"
				/>
				<input
					bind:value={handshakeForm.shared_secret}
					class="w-full rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-bg))] px-2 py-1 text-sm text-[rgb(var(--mv-text))]"
					placeholder="Shared secret (optional)"
				/>
				<button
					class="rounded bg-blue-600 px-3 py-1.5 text-xs text-white hover:bg-blue-700 disabled:opacity-50"
					onclick={handleHandshake}
					disabled={handshaking}
				>{handshaking ? 'Handshaking...' : 'Handshake & Register'}</button>
			</div>
			{#if lastHandshake}
				<p class="text-xs text-[rgb(var(--mv-muted))]">
					Last handshake: {lastHandshake.display_name} ({lastHandshake.vault_id})
				</p>
			{/if}
		</div>

		{#if showAddForm}
			<div class="space-y-3 rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))] p-4">
				<div class="grid grid-cols-2 gap-3">
					<div>
						<label for="peer-vault-id" class="mb-1 block text-xs text-[rgb(var(--mv-muted))]">Vault ID</label>
						<input
							id="peer-vault-id"
							bind:value={newPeer.vault_id}
							class="w-full rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-bg))] px-2 py-1 text-sm text-[rgb(var(--mv-text))]"
							placeholder="peer-vault-id"
						/>
					</div>
					<div>
						<label for="peer-name" class="mb-1 block text-xs text-[rgb(var(--mv-muted))]">Display Name</label>
						<input
							id="peer-name"
							bind:value={newPeer.display_name}
							class="w-full rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-bg))] px-2 py-1 text-sm text-[rgb(var(--mv-text))]"
							placeholder="Alice's vault"
						/>
					</div>
				</div>
				<div>
					<label for="peer-endpoint" class="mb-1 block text-xs text-[rgb(var(--mv-muted))]">Endpoint URL</label>
					<input
						id="peer-endpoint"
						bind:value={newPeer.endpoint}
						class="w-full rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-bg))] px-2 py-1 text-sm text-[rgb(var(--mv-text))]"
						placeholder="http://192.168.1.50:9470"
					/>
				</div>
				<button
					class="rounded bg-blue-600 px-3 py-1.5 text-sm text-white hover:bg-blue-700"
					onclick={handleAdd}
				>Add Peer</button>
			</div>
		{/if}

		{#if loading}
			<p class="text-[rgb(var(--mv-muted))]">Loading...</p>
		{:else if peers.length === 0}
			<p class="text-sm text-[rgb(var(--mv-muted))]">No federation peers configured.</p>
		{:else}
			{#each peers as peer (peer.id)}
				<div class="flex items-center justify-between rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))] p-3">
					<div>
						<div class="font-medium text-[rgb(var(--mv-text))]">{peer.display_name}</div>
						<div class="text-xs text-[rgb(var(--mv-muted))]">
							{peer.endpoint}
							{#if peer.last_seen}
								&middot; Last seen: {new Date(peer.last_seen).toLocaleString()}
							{/if}
						</div>
					</div>
					<div class="flex items-center gap-2">
						{#if healthStatus[peer.id] === true}
							<span class="text-xs text-green-400">Healthy</span>
						{:else if healthStatus[peer.id] === false}
							<span class="text-xs text-red-400">Unreachable</span>
						{/if}
						<button
							class="rounded px-2 py-1 text-xs text-blue-400 hover:bg-blue-500/10"
							onclick={() => handleHealthCheck(peer.id)}
						>Check</button>
						<button
							class="rounded px-2 py-1 text-xs text-red-400 hover:bg-red-500/10"
							onclick={() => handleRemove(peer.id)}
						>Remove</button>
					</div>
				</div>
			{/each}
		{/if}
	</div>

	<!-- Federated Query -->
	<div class="space-y-3">
		<h2 class="text-lg font-semibold text-[rgb(var(--mv-text))]">Federated Search</h2>
		<div class="flex gap-2">
			<input
				bind:value={queryText}
				class="flex-1 rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-bg))] px-3 py-2 text-sm text-[rgb(var(--mv-text))] placeholder-[rgb(var(--mv-muted))]"
				placeholder="Search across peer vaults..."
				onkeydown={(e) => { if (e.key === 'Enter') handleQuery(); }}
			/>
			<button
				class="rounded bg-blue-600 px-4 py-2 text-sm text-white hover:bg-blue-700 disabled:opacity-50"
				onclick={handleQuery}
				disabled={querying || !queryText.trim()}
			>{querying ? 'Searching...' : 'Search'}</button>
		</div>

		{#if queryResults.length > 0}
			<div class="space-y-2">
				{#each queryResults as result}
					<div class="rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))] p-3">
						<div class="flex items-center justify-between">
							<span class="text-sm font-medium text-[rgb(var(--mv-text))]">
								{typeof result.node === 'object' && result.node !== null && 'title' in result.node
									? result.node.title
									: 'Untitled'}
							</span>
							<span class="text-xs text-[rgb(var(--mv-muted))]">
								from {result.source_peer_name} ({(result.relevance_score * 100).toFixed(0)}%)
							</span>
						</div>
					</div>
				{/each}
			</div>
		{/if}
	</div>
</div>
