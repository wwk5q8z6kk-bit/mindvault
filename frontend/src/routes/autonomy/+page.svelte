<script lang="ts">
	import { onMount } from 'svelte';
	import { pushToast } from '$lib/stores/toast';
	import {
		listRules,
		createRule,
		deleteRule,
		updateRule,
		listActionLog,
		type AutonomyRule,
		type ActionLogEntry
	} from '$lib/api/autonomy';
	import {
		listProxyApprovals,
		decideProxyApproval,
		type ProxyApproval
	} from '$lib/api/proxy-approvals';

	let rules: AutonomyRule[] = [];
	let actionLog: ActionLogEntry[] = [];
	let approvals: ProxyApproval[] = [];
	let loading = true;
	let tab: 'rules' | 'log' | 'approvals' = 'rules';
	let showCreateForm = false;
	let creating = false;
	let decidingApprovalId = '';

	let newRule = {
		rule_type: 'global' as const,
		scope_key: '*',
		auto_apply_threshold: 0.9,
		max_actions_per_hour: 10,
		enabled: true,
		allowed_intent_types: '',
		blocked_intent_types: ''
	};

	$: stats = {
		totalActions: actionLog.length,
		autoApplied: actionLog.filter((entry) => entry.decision === 'auto_apply').length,
		deferred: actionLog.filter((entry) => entry.decision === 'defer').length,
		blocked: actionLog.filter((entry) => entry.decision === 'block').length
	};

	onMount(async () => {
		await loadData();
	});

	async function loadData() {
		loading = true;
		try {
			const [nextRules, nextLog, nextApprovals] = await Promise.all([
				listRules(),
				listActionLog(),
				listProxyApprovals()
			]);
			rules = nextRules;
			actionLog = nextLog;
			approvals = nextApprovals;
		} catch {
			pushToast('Failed to load autonomy data', 'danger');
		} finally {
			loading = false;
		}
	}

	function parseCsv(raw: string): string[] {
		return raw
			.split(',')
			.map((value) => value.trim())
			.filter(Boolean);
	}

	async function handleCreate() {
		creating = true;
		try {
			const rule = await createRule({
				rule_type: newRule.rule_type,
				scope_key: newRule.rule_type === 'global' ? '*' : newRule.scope_key,
				auto_apply_threshold: newRule.auto_apply_threshold,
				max_actions_per_hour: newRule.max_actions_per_hour,
				enabled: newRule.enabled,
				allowed_intent_types: parseCsv(newRule.allowed_intent_types),
				blocked_intent_types: parseCsv(newRule.blocked_intent_types)
			});
			rules = [rule, ...rules];
			showCreateForm = false;
			newRule = {
				rule_type: 'global',
				scope_key: '*',
				auto_apply_threshold: 0.9,
				max_actions_per_hour: 10,
				enabled: true,
				allowed_intent_types: '',
				blocked_intent_types: ''
			};
			pushToast('Rule created', 'success');
		} catch {
			pushToast('Failed to create rule', 'danger');
		} finally {
			creating = false;
		}
	}

	async function handleToggle(rule: AutonomyRule) {
		try {
			await updateRule(rule.id, { enabled: !rule.enabled });
			rules = rules.map((item) =>
				item.id === rule.id ? { ...item, enabled: !item.enabled } : item
			);
		} catch {
			pushToast('Failed to update rule', 'danger');
		}
	}

	async function handleDelete(id: string) {
		if (!confirm('Delete this autonomy rule?')) return;
		try {
			await deleteRule(id);
			rules = rules.filter((item) => item.id !== id);
			pushToast('Rule deleted', 'success');
		} catch {
			pushToast('Failed to delete rule', 'danger');
		}
	}

	async function handleApprovalDecision(approvalId: string, approved: boolean) {
		decidingApprovalId = approvalId;
		try {
			await decideProxyApproval(approvalId, approved, approved ? undefined : 'Denied in autonomy panel');
			approvals = approvals.filter((item) => item.id !== approvalId);
			pushToast(approved ? 'Approval granted' : 'Approval denied', 'success');
		} catch {
			pushToast('Failed to decide approval (admin permissions may be required)', 'danger');
		} finally {
			decidingApprovalId = '';
		}
	}

	function decisionBadge(decision: string): string {
		switch (decision) {
			case 'auto_apply':
				return 'bg-green-500/20 text-green-400';
			case 'defer':
				return 'bg-yellow-500/20 text-yellow-300';
			case 'block':
				return 'bg-red-500/20 text-red-300';
			default:
				return 'bg-slate-500/20 text-slate-300';
		}
	}

	function formatWhen(iso: string): string {
		return new Date(iso).toLocaleString();
	}
</script>

<div class="mx-auto max-w-5xl space-y-6 p-6">
	<div class="flex items-center justify-between">
		<div>
			<h1 class="text-2xl font-bold text-[rgb(var(--mv-text))]">Autonomy Rules Builder</h1>
			<p class="text-sm text-[rgb(var(--mv-muted))]">Define condition -> action rules and handle pending proxy approvals.</p>
		</div>
		<button
			class="rounded-lg border border-[rgb(var(--mv-border))] px-3 py-1.5 text-xs text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-panel-strong))]"
			on:click={loadData}
		>
			Refresh
		</button>
	</div>

	<div class="grid grid-cols-2 gap-3 md:grid-cols-4">
		<div class="rounded-xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/40 p-3">
			<p class="text-[10px] uppercase tracking-wide text-[rgb(var(--mv-muted))]/70">Actions</p>
			<p class="mt-1 text-xl font-semibold text-[rgb(var(--mv-text))]">{stats.totalActions}</p>
		</div>
		<div class="rounded-xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/40 p-3">
			<p class="text-[10px] uppercase tracking-wide text-[rgb(var(--mv-muted))]/70">Auto Applied</p>
			<p class="mt-1 text-xl font-semibold text-green-300">{stats.autoApplied}</p>
		</div>
		<div class="rounded-xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/40 p-3">
			<p class="text-[10px] uppercase tracking-wide text-[rgb(var(--mv-muted))]/70">Deferred</p>
			<p class="mt-1 text-xl font-semibold text-amber-300">{stats.deferred}</p>
		</div>
		<div class="rounded-xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/40 p-3">
			<p class="text-[10px] uppercase tracking-wide text-[rgb(var(--mv-muted))]/70">Pending Approvals</p>
			<p class="mt-1 text-xl font-semibold text-sky-300">{approvals.length}</p>
		</div>
	</div>

	<div class="flex gap-2 border-b border-[rgb(var(--mv-border))] pb-2">
		<button
			class="rounded-lg px-3 py-1.5 text-xs {tab === 'rules' ? 'bg-sky-500/20 text-sky-200' : 'text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-panel-strong))]'}"
			on:click={() => (tab = 'rules')}
		>
			Rules
		</button>
		<button
			class="rounded-lg px-3 py-1.5 text-xs {tab === 'log' ? 'bg-sky-500/20 text-sky-200' : 'text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-panel-strong))]'}"
			on:click={() => (tab = 'log')}
		>
			Execution Log
		</button>
		<button
			class="rounded-lg px-3 py-1.5 text-xs {tab === 'approvals' ? 'bg-sky-500/20 text-sky-200' : 'text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-panel-strong))]'}"
			on:click={() => (tab = 'approvals')}
		>
			Approvals
		</button>
	</div>

	{#if loading}
		<p class="text-sm text-[rgb(var(--mv-muted))]">Loading autonomy controls...</p>
	{:else if tab === 'rules'}
		<div class="space-y-3">
			<button
				class="rounded-lg bg-sky-500 px-3 py-1.5 text-xs font-medium text-white hover:bg-sky-400"
				on:click={() => (showCreateForm = !showCreateForm)}
			>
				{showCreateForm ? 'Cancel' : '+ New Rule'}
			</button>

			{#if showCreateForm}
				<div class="rounded-xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/40 p-4">
					<h3 class="text-sm font-semibold text-[rgb(var(--mv-text))]">Condition -> Action</h3>
					<div class="mt-3 grid gap-3 md:grid-cols-2">
						<div>
							<label for="rule-type" class="mb-1 block text-xs text-[rgb(var(--mv-muted))]">Condition Type</label>
							<select
								id="rule-type"
								bind:value={newRule.rule_type}
								class="w-full rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-bg))] px-3 py-2 text-sm text-[rgb(var(--mv-text))]"
							>
								<option value="global">Global</option>
								<option value="domain">Domain</option>
								<option value="contact">Contact</option>
								<option value="tag">Tag</option>
							</select>
						</div>
						<div>
							<label for="rule-scope" class="mb-1 block text-xs text-[rgb(var(--mv-muted))]">Scope Key</label>
							<input
								id="rule-scope"
								bind:value={newRule.scope_key}
								class="w-full rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-bg))] px-3 py-2 text-sm text-[rgb(var(--mv-text))]"
								placeholder="example.com / alice@example.com / project-tag"
								disabled={newRule.rule_type === 'global'}
							/>
						</div>
						<div>
							<label for="rule-threshold" class="mb-1 block text-xs text-[rgb(var(--mv-muted))]">Auto-Apply Threshold</label>
							<input
								id="rule-threshold"
								type="number"
								min="0"
								max="1"
								step="0.05"
								bind:value={newRule.auto_apply_threshold}
								class="w-full rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-bg))] px-3 py-2 text-sm text-[rgb(var(--mv-text))]"
							/>
						</div>
						<div>
							<label for="rule-max-hour" class="mb-1 block text-xs text-[rgb(var(--mv-muted))]">Max Actions / Hour</label>
							<input
								id="rule-max-hour"
								type="number"
								min="1"
								bind:value={newRule.max_actions_per_hour}
								class="w-full rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-bg))] px-3 py-2 text-sm text-[rgb(var(--mv-text))]"
							/>
						</div>
						<div>
							<label for="rule-allowed" class="mb-1 block text-xs text-[rgb(var(--mv-muted))]">Allowed Intent Types</label>
							<input
								id="rule-allowed"
								bind:value={newRule.allowed_intent_types}
								class="w-full rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-bg))] px-3 py-2 text-sm text-[rgb(var(--mv-text))]"
								placeholder="extract_task,suggest_tag"
							/>
						</div>
						<div>
							<label for="rule-blocked" class="mb-1 block text-xs text-[rgb(var(--mv-muted))]">Blocked Intent Types</label>
							<input
								id="rule-blocked"
								bind:value={newRule.blocked_intent_types}
								class="w-full rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-bg))] px-3 py-2 text-sm text-[rgb(var(--mv-text))]"
								placeholder="delete_node"
							/>
						</div>
					</div>
					<div class="mt-4 flex justify-end">
						<button
							class="rounded-lg bg-sky-500 px-4 py-2 text-xs font-semibold text-white hover:bg-sky-400 disabled:opacity-60"
							disabled={creating}
							on:click={handleCreate}
						>
							{creating ? 'Creating...' : 'Create Rule'}
						</button>
					</div>
				</div>
			{/if}

			{#if rules.length === 0}
				<p class="text-sm text-[rgb(var(--mv-muted))]">No autonomy rules configured yet.</p>
			{/if}

			{#each rules as rule (rule.id)}
				<div class="rounded-xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/40 p-4">
					<div class="flex flex-wrap items-start justify-between gap-3">
						<div class="min-w-0 flex-1">
							<div class="flex items-center gap-2">
								<span class="rounded bg-[rgb(var(--mv-hover))] px-1.5 py-0.5 text-[10px] uppercase text-[rgb(var(--mv-text))]">{rule.rule_type}</span>
								<span class="text-sm font-medium text-[rgb(var(--mv-text))]">{rule.scope_key ?? '*'}</span>
							</div>
							<p class="mt-2 text-xs text-[rgb(var(--mv-muted))]">When confidence >= {rule.auto_apply_threshold.toFixed(2)}, allow up to {rule.max_actions_per_hour}/hr.</p>
							{#if rule.allowed_intent_types.length > 0 || rule.blocked_intent_types.length > 0}
								<div class="mt-2 flex flex-wrap gap-1.5">
									{#each rule.allowed_intent_types as allowed (allowed)}
										<span class="rounded border border-green-500/30 bg-green-500/10 px-2 py-0.5 text-[10px] text-green-300">allow: {allowed}</span>
									{/each}
									{#each rule.blocked_intent_types as blocked (blocked)}
										<span class="rounded border border-red-500/30 bg-red-500/10 px-2 py-0.5 text-[10px] text-red-300">block: {blocked}</span>
									{/each}
								</div>
							{/if}
						</div>
						<div class="flex items-center gap-2">
							<button
								class="rounded px-2 py-1 text-xs {rule.enabled ? 'bg-green-500/20 text-green-300' : 'bg-slate-500/20 text-slate-300'}"
								on:click={() => handleToggle(rule)}
							>
								{rule.enabled ? 'Enabled' : 'Disabled'}
							</button>
							<button
								class="rounded border border-red-500/30 px-2 py-1 text-xs text-red-300 hover:bg-red-500/10"
								on:click={() => handleDelete(rule.id)}
							>
								Delete
							</button>
						</div>
					</div>
				</div>
			{/each}
		</div>
	{:else if tab === 'log'}
		<div class="space-y-2">
			{#if actionLog.length === 0}
				<p class="text-sm text-[rgb(var(--mv-muted))]">No actions logged yet.</p>
			{:else}
				{#each actionLog as entry (entry.id)}
					<div class="flex flex-wrap items-center justify-between gap-3 rounded-xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/40 p-3">
						<div class="min-w-0 flex-1">
							<p class="text-sm font-medium text-[rgb(var(--mv-text))]">{entry.intent_type}</p>
							<p class="text-xs text-[rgb(var(--mv-muted))]">{entry.reason}</p>
							<p class="text-[10px] text-[rgb(var(--mv-muted))]/70">{formatWhen(entry.created_at)}</p>
						</div>
						<div class="flex items-center gap-2">
							<span class="text-[11px] text-[rgb(var(--mv-muted))]">{(entry.confidence * 100).toFixed(0)}%</span>
							<span class="rounded px-2 py-0.5 text-[10px] {decisionBadge(entry.decision)}">{entry.decision}</span>
						</div>
					</div>
				{/each}
			{/if}
		</div>
	{:else}
		<div class="space-y-2">
			{#if approvals.length === 0}
				<p class="text-sm text-[rgb(var(--mv-muted))]">No pending proxy approvals.</p>
			{:else}
				{#each approvals as approval (approval.id)}
					<div class="rounded-xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/40 p-4">
						<div class="flex flex-wrap items-start justify-between gap-3">
							<div class="min-w-0 flex-1">
								<div class="flex items-center gap-2">
									<span class="rounded bg-amber-500/20 px-1.5 py-0.5 text-[10px] text-amber-300">{approval.state}</span>
									<span class="text-xs text-[rgb(var(--mv-muted))]">{approval.consumer}</span>
								</div>
								<p class="mt-2 text-sm font-medium text-[rgb(var(--mv-text))]">{approval.intent}</p>
								<p class="mt-1 text-xs text-[rgb(var(--mv-muted))]">{approval.request_summary}</p>
								<p class="mt-1 text-[10px] text-[rgb(var(--mv-muted))]/70">Created {formatWhen(approval.created_at)}</p>
							</div>
							<div class="flex gap-1.5">
								<button
									class="rounded border border-green-500/30 px-2.5 py-1 text-[10px] font-medium text-green-300 hover:bg-green-500/10 disabled:opacity-60"
									disabled={decidingApprovalId === approval.id}
									on:click={() => handleApprovalDecision(approval.id, true)}
								>
									Approve
								</button>
								<button
									class="rounded border border-red-500/30 px-2.5 py-1 text-[10px] font-medium text-red-300 hover:bg-red-500/10 disabled:opacity-60"
									disabled={decidingApprovalId === approval.id}
									on:click={() => handleApprovalDecision(approval.id, false)}
								>
									Deny
								</button>
							</div>
						</div>
					</div>
				{/each}
			{/if}
		</div>
	{/if}
</div>
