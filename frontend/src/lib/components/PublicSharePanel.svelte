<script lang="ts">
	import { onMount } from 'svelte';
	import { pushToast } from '$lib/stores/toast';
	import {
		createPublicShare,
		listPublicShares,
		revokePublicShare,
		type PublicShareSummary,
		type CreatePublicShareResponse
	} from '$lib/api/shares';

	export let nodeId: string | null = null;
	export let nodeTitle: string | null = null;

	let shares: PublicShareSummary[] = [];
	let latestShare: CreatePublicShareResponse | null = null;
	let knownLinks = new Map<string, string>();
	let expiresAt = '';
	let creating = false;
	let refreshing = false;
	let revokingId: string | null = null;
	let loadedForNode: string | null = null;

	onMount(async () => {
		if (nodeId) {
			await refreshShares();
		}
	});

	$: if (nodeId && nodeId !== loadedForNode) {
		loadedForNode = nodeId;
		latestShare = null;
		knownLinks = new Map();
		void refreshShares();
	}

	$: if (!nodeId) {
		shares = [];
		latestShare = null;
		knownLinks = new Map();
		loadedForNode = null;
	}

	function resolveShareUrl(url: string): string {
		if (url.startsWith('http')) return url;
		return new URL(url, window.location.origin).toString();
	}

	function formatDateTime(iso?: string | null): string {
		if (!iso) return 'never';
		return new Date(iso).toLocaleString(undefined, {
			year: 'numeric',
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		});
	}

	function shareStatus(share: PublicShareSummary): { label: string; className: string } {
		if (share.revoked_at) {
			return { label: 'revoked', className: 'bg-red-500/10 text-red-300' };
		}
		if (share.expires_at && Date.parse(share.expires_at) < Date.now()) {
			return { label: 'expired', className: 'bg-amber-500/10 text-amber-300' };
		}
		return { label: 'active', className: 'bg-emerald-500/10 text-emerald-300' };
	}

	async function refreshShares() {
		if (!nodeId) return;
		refreshing = true;
		try {
			shares = await listPublicShares({ node_id: nodeId, include_revoked: true });
		} catch {
			pushToast('Failed to load shares', 'danger');
		} finally {
			refreshing = false;
		}
	}

	async function handleCreate() {
		if (!nodeId) {
			pushToast('Select a note or task first', 'warning');
			return;
		}
		creating = true;
		try {
			let expiresAtIso: string | undefined;
			if (expiresAt) {
				const parsed = new Date(expiresAt);
				if (Number.isNaN(parsed.getTime())) {
					pushToast('Invalid expiry date', 'warning');
					creating = false;
					return;
				}
				expiresAtIso = parsed.toISOString();
			}
			const created = await createPublicShare({
				node_id: nodeId,
				expires_at: expiresAtIso
			});
			const url = resolveShareUrl(created.url);
			latestShare = { ...created, url };
			const updatedLinks = new Map(knownLinks);
			updatedLinks.set(created.id, url);
			knownLinks = updatedLinks;
			expiresAt = '';
			pushToast('Share link created', 'success');
			await refreshShares();
		} catch (e: any) {
			pushToast(e?.message ?? 'Failed to create share', 'danger');
		} finally {
			creating = false;
		}
	}

	async function handleRevoke(shareId: string) {
		if (!shareId) return;
		if (!confirm('Revoke this share link?')) return;
		revokingId = shareId;
		try {
			await revokePublicShare(shareId);
			const updatedLinks = new Map(knownLinks);
			updatedLinks.delete(shareId);
			knownLinks = updatedLinks;
			pushToast('Share revoked', 'success');
			await refreshShares();
		} catch (e: any) {
			pushToast(e?.message ?? 'Failed to revoke share', 'danger');
		} finally {
			revokingId = null;
		}
	}

	async function copyLink(link?: string | null) {
		if (!link) return;
		if (!navigator.clipboard) {
			pushToast('Clipboard not available', 'warning');
			return;
		}
		try {
			await navigator.clipboard.writeText(link);
			pushToast('Share link copied', 'success');
		} catch {
			pushToast('Clipboard access denied', 'warning');
		}
	}
</script>

<div class="rounded-2xl border border-slate-800 bg-slate-900/50 p-4">
	<div class="flex items-start justify-between gap-3">
		<div>
			<h3 class="text-xs font-semibold text-white">Public share</h3>
			<p class="mt-1 text-[10px] text-slate-500">
				Read-only link for <span class="text-slate-300">{nodeTitle || 'this item'}</span>.
				Token is only shown once.
			</p>
		</div>
		<button
			class="rounded-lg border border-slate-700 px-2 py-1 text-[10px] text-slate-300 hover:bg-slate-800 disabled:opacity-50"
			on:click={refreshShares}
			disabled={refreshing || !nodeId}
			title="Refresh shares"
		>
			{refreshing ? 'Refreshing…' : 'Refresh'}
		</button>
	</div>

	{#if !nodeId}
		<div class="mt-3 rounded-xl border border-dashed border-slate-800 px-3 py-4 text-center text-[10px] text-slate-500">
			Select a note or task to manage public shares.
		</div>
	{:else}
		<div class="mt-3 grid gap-2">
			<label class="text-[10px] uppercase tracking-wide text-slate-500" for="share-expires">
				Expires at (optional)
			</label>
			<div class="flex flex-wrap items-center gap-2">
				<input
					id="share-expires"
					type="datetime-local"
					bind:value={expiresAt}
					class="min-w-[220px] rounded-lg border border-slate-700 bg-slate-950/40 px-3 py-2 text-xs text-slate-200 focus:border-sky-500 focus:outline-none"
				/>
				<button
					class="rounded-lg bg-sky-500/20 px-3 py-2 text-xs font-semibold text-sky-200 hover:bg-sky-500/30 disabled:opacity-50"
					on:click={handleCreate}
					disabled={creating}
				>
					{creating ? 'Creating…' : 'Create share'}
				</button>
			</div>
		</div>

		{#if latestShare}
			<div class="mt-3 rounded-xl border border-slate-800 bg-slate-950/30 p-3">
				<div class="flex items-center justify-between gap-2">
					<div>
						<p class="text-[10px] uppercase tracking-wide text-slate-500">Latest share</p>
						<p class="mt-1 text-xs text-slate-200 break-all">{latestShare.url}</p>
						<p class="mt-1 text-[10px] text-slate-500">
							Created {formatDateTime(latestShare.created_at)} · Expires {formatDateTime(latestShare.expires_at)}
						</p>
					</div>
					<button
						class="rounded-lg border border-slate-700 px-2 py-1 text-[10px] text-slate-300 hover:bg-slate-800"
						on:click={() => copyLink(latestShare?.url)}
					>
						Copy
					</button>
				</div>
			</div>
		{/if}

		<div class="mt-3 space-y-2">
			<p class="text-[10px] uppercase tracking-wide text-slate-500">All shares</p>
			{#if refreshing && shares.length === 0}
				<p class="text-[10px] text-slate-500">Loading shares…</p>
			{:else if shares.length === 0}
				<p class="text-[10px] text-slate-500">No shares created yet.</p>
			{:else}
				{#each shares as share (share.id)}
					{@const status = shareStatus(share)}
					<div class="rounded-xl border border-slate-800 bg-slate-950/30 p-3">
						<div class="flex items-center justify-between gap-2">
							<span class="rounded-full px-2 py-0.5 text-[9px] uppercase tracking-wide {status.className}">
								{status.label}
							</span>
							{#if !share.revoked_at}
								<button
									class="rounded-lg border border-red-500/30 px-2 py-1 text-[10px] text-red-300 hover:bg-red-500/10 disabled:opacity-50"
									on:click={() => handleRevoke(share.id)}
									disabled={revokingId === share.id}
								>
									{revokingId === share.id ? 'Revoking…' : 'Revoke'}
								</button>
							{/if}
						</div>
						<div class="mt-2 text-[10px] text-slate-500">
							Created {formatDateTime(share.created_at)} · Expires {formatDateTime(share.expires_at)}
							{#if share.revoked_at}
								· Revoked {formatDateTime(share.revoked_at)}
							{/if}
						</div>
						<div class="mt-2 flex items-center justify-between gap-2">
							{#if knownLinks.has(share.id)}
								<p class="text-xs text-slate-200 break-all">{knownLinks.get(share.id)}</p>
								<button
									class="rounded-lg border border-slate-700 px-2 py-1 text-[10px] text-slate-300 hover:bg-slate-800"
									on:click={() => copyLink(knownLinks.get(share.id))}
								>
									Copy
								</button>
							{:else}
								<p class="text-[10px] text-slate-500">
									Link not stored. Create a new share to rotate the link.
								</p>
							{/if}
						</div>
					</div>
				{/each}
			{/if}
		</div>
	{/if}
</div>
