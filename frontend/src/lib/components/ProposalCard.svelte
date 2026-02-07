<script lang="ts">
	import type { Proposal } from '$lib/api/exchange';
	import { createEventDispatcher } from 'svelte';

	export let proposal: Proposal;

	const dispatch = createEventDispatcher<{ approve: string; reject: string }>();

	const senderLabels: Record<string, string> = {
		agent: 'Agent',
		mcp: 'MCP',
		webhook: 'Webhook',
		watcher: 'Watcher',
		relay: 'Relay',
		self: 'Self'
	};

	const senderColors: Record<string, string> = {
		agent: 'bg-violet-500/20 text-violet-300',
		mcp: 'bg-cyan-500/20 text-cyan-300',
		webhook: 'bg-amber-500/20 text-amber-300',
		watcher: 'bg-emerald-500/20 text-emerald-300',
		relay: 'bg-blue-500/20 text-blue-300',
		self: 'bg-slate-500/20 text-slate-300'
	};

	const actionLabels: Record<string, string> = {
		create_node: 'Create Node',
		update_node: 'Update Node',
		delete_node: 'Delete Node',
		suggest_tag: 'Suggest Tag',
		suggest_link: 'Suggest Link',
		schedule_reminder: 'Schedule Reminder'
	};

	$: senderLabel = senderLabels[proposal.sender] ?? proposal.sender;
	$: senderColor = senderColors[proposal.sender] ?? senderColors['self'];
	$: actionLabel = actionLabels[proposal.action] ?? proposal.action;
	$: isPending = proposal.state === 'pending';
	$: confidencePct = Math.round(proposal.confidence * 100);
	$: confidenceColor =
		confidencePct >= 80
			? 'text-emerald-400'
			: confidencePct >= 50
				? 'text-amber-400'
				: 'text-red-400';

	function formatDate(iso: string): string {
		const d = new Date(iso);
		const now = new Date();
		const diffMs = now.getTime() - d.getTime();
		const diffMin = Math.floor(diffMs / 60000);
		if (diffMin < 1) return 'just now';
		if (diffMin < 60) return `${diffMin}m ago`;
		const diffHr = Math.floor(diffMin / 60);
		if (diffHr < 24) return `${diffHr}h ago`;
		return d.toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
	}
</script>

<div
	class="rounded-xl border border-slate-800 bg-slate-900/60 p-4 transition hover:border-slate-700"
>
	<div class="mb-2 flex items-center justify-between gap-2">
		<div class="flex items-center gap-2">
			<span class={`rounded-full px-2 py-0.5 text-xs font-medium ${senderColor}`}>
				{senderLabel}
			</span>
			<span class="text-sm text-slate-300">{actionLabel}</span>
		</div>
		<div class="flex items-center gap-2 text-xs text-slate-500">
			<span class={confidenceColor}>{confidencePct}%</span>
			<span>{formatDate(proposal.created_at)}</span>
		</div>
	</div>

	{#if proposal.diff_preview}
		<pre
			class="mb-3 max-h-32 overflow-auto rounded-lg bg-slate-950 p-3 text-xs text-slate-400">{proposal.diff_preview}</pre>
	{/if}

	{#if proposal.target_node_id}
		<div class="mb-3 text-xs text-slate-500">
			Target: <span class="font-mono text-slate-400">{proposal.target_node_id}</span>
		</div>
	{/if}

	{#if isPending}
		<div class="flex gap-2">
			<button
				class="rounded-lg bg-emerald-600 px-3 py-1.5 text-sm font-medium text-white transition hover:bg-emerald-500"
				on:click={() => dispatch('approve', proposal.id)}
			>
				Approve
			</button>
			<button
				class="rounded-lg bg-slate-700 px-3 py-1.5 text-sm font-medium text-slate-300 transition hover:bg-slate-600"
				on:click={() => dispatch('reject', proposal.id)}
			>
				Reject
			</button>
		</div>
	{:else}
		<div class="text-xs text-slate-500">
			{#if proposal.state === 'approved' || proposal.state === 'auto_approved'}
				<span class="text-emerald-400">Approved</span>
			{:else if proposal.state === 'rejected'}
				<span class="text-red-400">Rejected</span>
			{:else if proposal.state === 'expired'}
				<span class="text-amber-400">Expired</span>
			{/if}
			{#if proposal.resolved_at}
				&middot; {formatDate(proposal.resolved_at)}
			{/if}
		</div>
	{/if}
</div>
