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

	let rules: AutonomyRule[] = [];
	let actionLog: ActionLogEntry[] = [];
	let loading = true;
	let tab: 'rules' | 'log' = 'rules';
	let showCreateForm = false;
	let newRule = {
		rule_type: 'global' as const,
		scope_key: '*',
		auto_apply_threshold: 0.9,
		max_actions_per_hour: 10,
		enabled: true
	};

	onMount(async () => {
		await loadData();
	});

	async function loadData() {
		loading = true;
		try {
			[rules, actionLog] = await Promise.all([listRules(), listActionLog()]);
		} catch {
			pushToast('Failed to load autonomy data', 'danger');
		} finally {
			loading = false;
		}
	}

	async function handleCreate() {
		try {
			const rule = await createRule(newRule);
			rules = [...rules, rule];
			showCreateForm = false;
			pushToast('Rule created', 'success');
		} catch {
			pushToast('Failed to create rule', 'danger');
		}
	}

	async function handleToggle(rule: AutonomyRule) {
		try {
			await updateRule(rule.id, { enabled: !rule.enabled });
			rules = rules.map((r) => (r.id === rule.id ? { ...r, enabled: !r.enabled } : r));
		} catch {
			pushToast('Failed to update rule', 'danger');
		}
	}

	async function handleDelete(id: string) {
		try {
			await deleteRule(id);
			rules = rules.filter((r) => r.id !== id);
			pushToast('Rule deleted', 'success');
		} catch {
			pushToast('Failed to delete rule', 'danger');
		}
	}

	function decisionBadge(decision: string): string {
		switch (decision) {
			case 'auto_apply': return 'bg-green-500/20 text-green-400';
			case 'defer': return 'bg-yellow-500/20 text-yellow-400';
			case 'block': return 'bg-red-500/20 text-red-400';
			default: return 'bg-gray-500/20 text-gray-400';
		}
	}
</script>

<div class="mx-auto max-w-4xl space-y-6 p-6">
	<h1 class="text-2xl font-bold text-[rgb(var(--mv-text))]">Autonomy Controls</h1>
	<p class="text-sm text-[rgb(var(--mv-muted))]">
		Configure when agents may act autonomously vs. defer to your approval.
	</p>

	<!-- Tabs -->
	<div class="flex gap-4 border-b border-[rgb(var(--mv-border))]">
		<button
			class="border-b-2 px-2 pb-2 text-sm {tab === 'rules'
				? 'border-blue-500 text-blue-500'
				: 'border-transparent text-[rgb(var(--mv-muted))]'}"
			onclick={() => (tab = 'rules')}
		>Rules</button>
		<button
			class="border-b-2 px-2 pb-2 text-sm {tab === 'log'
				? 'border-blue-500 text-blue-500'
				: 'border-transparent text-[rgb(var(--mv-muted))]'}"
			onclick={() => (tab = 'log')}
		>Action Log</button>
	</div>

	{#if loading}
		<p class="text-[rgb(var(--mv-muted))]">Loading...</p>
	{:else if tab === 'rules'}
		<div class="space-y-3">
			<button
				class="rounded bg-blue-600 px-3 py-1.5 text-sm text-white hover:bg-blue-700"
				onclick={() => (showCreateForm = !showCreateForm)}
			>
				{showCreateForm ? 'Cancel' : '+ New Rule'}
			</button>

			{#if showCreateForm}
				<div class="space-y-3 rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))] p-4">
					<div class="grid grid-cols-2 gap-3">
						<div>
							<label for="rule-type" class="mb-1 block text-xs text-[rgb(var(--mv-muted))]">Type</label>
							<select
								id="rule-type"
								bind:value={newRule.rule_type}
								class="w-full rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-bg))] px-2 py-1 text-sm text-[rgb(var(--mv-text))]"
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
								class="w-full rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-bg))] px-2 py-1 text-sm text-[rgb(var(--mv-text))]"
								placeholder="* for all"
							/>
						</div>
						<div>
							<label for="rule-threshold" class="mb-1 block text-xs text-[rgb(var(--mv-muted))]">Threshold</label>
							<input
								id="rule-threshold"
								type="number"
								min="0"
								max="1"
								step="0.05"
								bind:value={newRule.auto_apply_threshold}
								class="w-full rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-bg))] px-2 py-1 text-sm text-[rgb(var(--mv-text))]"
							/>
						</div>
						<div>
							<label for="rule-max-hour" class="mb-1 block text-xs text-[rgb(var(--mv-muted))]">Max/Hour</label>
							<input
								id="rule-max-hour"
								type="number"
								min="0"
								bind:value={newRule.max_actions_per_hour}
								class="w-full rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-bg))] px-2 py-1 text-sm text-[rgb(var(--mv-text))]"
							/>
						</div>
					</div>
					<button
						class="rounded bg-blue-600 px-3 py-1.5 text-sm text-white hover:bg-blue-700"
						onclick={handleCreate}
					>Create</button>
				</div>
			{/if}

			{#if rules.length === 0}
				<p class="text-sm text-[rgb(var(--mv-muted))]">No autonomy rules configured. The agent will defer all actions by default.</p>
			{/if}

			{#each rules as rule (rule.id)}
				<div class="flex items-center justify-between rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))] p-3">
					<div>
						<div class="flex items-center gap-2">
							<span class="rounded bg-[rgb(var(--mv-hover))] px-1.5 py-0.5 text-xs text-[rgb(var(--mv-text))]">{rule.rule_type}</span>
							<span class="text-sm font-medium text-[rgb(var(--mv-text))]">{rule.scope_key}</span>
						</div>
						<div class="mt-1 text-xs text-[rgb(var(--mv-muted))]">
							Threshold: {rule.auto_apply_threshold} | Max: {rule.max_actions_per_hour}/hr
						</div>
					</div>
					<div class="flex items-center gap-2">
						<button
							class="rounded px-2 py-1 text-xs {rule.enabled
								? 'bg-green-500/20 text-green-400'
								: 'bg-gray-500/20 text-gray-400'}"
							onclick={() => handleToggle(rule)}
						>
							{rule.enabled ? 'Enabled' : 'Disabled'}
						</button>
						<button
							class="rounded px-2 py-1 text-xs text-red-400 hover:bg-red-500/10"
							onclick={() => handleDelete(rule.id)}
						>Delete</button>
					</div>
				</div>
			{/each}
		</div>
	{:else}
		<div class="space-y-2">
			{#if actionLog.length === 0}
				<p class="text-sm text-[rgb(var(--mv-muted))]">No actions logged yet.</p>
			{/if}
			{#each actionLog as entry (entry.id)}
				<div class="flex items-center justify-between rounded border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))] p-3 text-sm">
					<div>
						<span class="font-medium text-[rgb(var(--mv-text))]">{entry.intent_type}</span>
						<span class="ml-2 text-xs text-[rgb(var(--mv-muted))]">{entry.reason}</span>
					</div>
					<div class="flex items-center gap-2">
						<span class="text-xs text-[rgb(var(--mv-muted))]">{(entry.confidence * 100).toFixed(0)}%</span>
						<span class="rounded px-1.5 py-0.5 text-xs {decisionBadge(entry.decision)}">{entry.decision}</span>
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>
