<script lang="ts">
	import { onMount } from 'svelte';
	import { agentRunObservations } from '$lib/api/agent';
	import { pushToast } from '$lib/stores/toast';
	import {
		approveRun,
		artifactContentUrl,
		completeRun,
		getWorkOrder,
		listArtifacts,
		listGates,
		listRuns,
		listWorkOrders,
		AWAITING_APPROVAL,
		GATE_LABELS,
		type AgentRunView,
		type ArtifactView,
		type GateResultView,
		type WorkOrderSummary
	} from '$lib/api/workOrders';

	let workOrders: WorkOrderSummary[] = [];
	let selected: WorkOrderSummary | null = null;
	let runs: AgentRunView[] = [];
	let gatesByRun: Record<string, GateResultView[]> = {};
	let artifacts: ArtifactView[] = [];
	let loading = true;
	let busyRunId = '';
	let liveRefreshTimer: ReturnType<typeof setTimeout> | null = null;
	let lastNotifiedApprovalRunId = '';

	$: awaitingApproval = runs.filter((run) => run.status === AWAITING_APPROVAL);
	$: artifactsByRun = artifacts.reduce<Record<string, ArtifactView[]>>((acc, artifact) => {
		(acc[artifact.run_id] ??= []).push(artifact);
		return acc;
	}, {});

	onMount(() => {
		void loadWorkOrders();
		const unsub = agentRunObservations.subscribe((observation) => {
			if (!observation) return;
			scheduleLiveRefresh(observation.event.work_order_id);
			if (
				observation.event.kind === 'transition' &&
				observation.event.status === AWAITING_APPROVAL &&
				observation.event.run_id !== lastNotifiedApprovalRunId
			) {
				lastNotifiedApprovalRunId = observation.event.run_id;
				pushToast('A run is awaiting your approval', 'info');
			}
		});
		return () => {
			unsub();
			if (liveRefreshTimer) clearTimeout(liveRefreshTimer);
		};
	});

	async function loadWorkOrders() {
		loading = true;
		try {
			workOrders = await listWorkOrders();
			if (workOrders.length && !selected) await selectOrder(workOrders[0]);
		} catch {
			pushToast('Failed to load work orders', 'danger');
		} finally {
			loading = false;
		}
	}

	async function selectOrder(order: WorkOrderSummary) {
		selected = order;
		runs = [];
		gatesByRun = {};
		artifacts = [];
		try {
			[runs, artifacts] = await Promise.all([
				listRuns(order.work_order_id),
				listArtifacts(order.work_order_id)
			]);
			const collected = await Promise.all(
				runs.map(async (run) => [run.run_id, await listGates(order.work_order_id, run.run_id)] as const)
			);
			gatesByRun = Object.fromEntries(collected);
		} catch {
			pushToast('Failed to load run detail', 'danger');
		}
	}

	async function refreshSelected() {
		if (!selected) return;
		selected = await getWorkOrder(selected.work_order_id);
		await selectOrder(selected);
	}

	/** Coalesce bursty transition+gate pairs into one detail reload. */
	function scheduleLiveRefresh(workOrderId: string) {
		if (liveRefreshTimer) clearTimeout(liveRefreshTimer);
		liveRefreshTimer = setTimeout(() => {
			liveRefreshTimer = null;
			void (async () => {
				try {
					workOrders = await listWorkOrders();
					if (selected?.work_order_id === workOrderId) {
						await refreshSelected();
					} else if (selected) {
						const updated = workOrders.find(
							(order) => order.work_order_id === selected?.work_order_id
						);
						if (updated) selected = updated;
					}
				} catch {
					// Live refresh is best-effort; Refresh remains the explicit path.
				}
			})();
		}, 150);
	}

	async function handleApprove(run: AgentRunView) {
		if (!selected) return;
		busyRunId = run.run_id;
		try {
			await approveRun(selected.work_order_id, run.run_id);
			// Approval authorises the action, not a stale write set: the run
			// returns to the ready set and re-acquires leases under a fresh
			// conflict check rather than resuming mid-flight.
			pushToast('Approved — the run re-enters the ready set', 'success');
			await refreshSelected();
		} catch {
			pushToast('Failed to approve run', 'danger');
		} finally {
			busyRunId = '';
		}
	}

	async function handleComplete(run: AgentRunView) {
		if (!selected) return;
		busyRunId = run.run_id;
		try {
			const result = await completeRun(selected.work_order_id, run.run_id);
			if (result.completed) {
				pushToast('Run completed', 'success');
			} else {
				// Outstanding gates are normal progress, not an error: the next
				// action is to supply evidence.
				const missing = result.outstanding_gates
					.map((gate) => GATE_LABELS[gate] ?? gate)
					.join(', ');
				pushToast(`Evidence still required: ${missing}`, 'warning');
			}
			await refreshSelected();
		} catch {
			pushToast('Failed to complete run', 'danger');
		} finally {
			busyRunId = '';
		}
	}

	function runBadge(status: string): string {
		switch (status) {
			case 'completed':
			case 'verified':
				return 'bg-emerald-500/20 text-emerald-300';
			// Awaiting approval is the system working correctly — quiet hours and
			// a week away are expected. Never styled as a failure.
			case 'awaiting_approval':
				return 'bg-sky-500/20 text-sky-300';
			case 'running':
			case 'leased':
			case 'gated':
				return 'bg-violet-500/20 text-violet-300';
			case 'failed':
			case 'budget_exhausted':
				return 'bg-red-500/20 text-red-300';
			default:
				return 'bg-slate-500/20 text-slate-300';
		}
	}

	function gateBadge(outcome: string): string {
		switch (outcome) {
			case 'pass':
				return 'bg-emerald-500/20 text-emerald-300';
			case 'fail':
				return 'bg-red-500/20 text-red-300';
			default:
				return 'bg-amber-500/20 text-amber-300';
		}
	}

	function formatWhen(iso: string | null): string {
		return iso ? new Date(iso).toLocaleString() : '—';
	}

	function shortDigest(digest: string): string {
		return `${digest.slice(0, 12)}…`;
	}
</script>

<div class="mx-auto max-w-6xl space-y-6 p-6">
	<div class="flex items-center justify-between">
		<div>
			<h1 class="text-2xl font-bold text-[rgb(var(--mv-text))]">Work Orders</h1>
			<p class="text-sm text-[rgb(var(--mv-muted))]">
				Governed agent execution: declared scope, run attempts, gate evidence, and artifacts.
				Live updates arrive on the agent stream when your session is not namespace-scoped.
			</p>
		</div>
		<button
			class="rounded-lg border border-[rgb(var(--mv-border))] px-3 py-1.5 text-xs text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-panel-strong))]"
			on:click={loadWorkOrders}
			title="Always available — required if your auth token is namespace-scoped"
		>
			Refresh
		</button>
	</div>

	{#if awaitingApproval.length}
		<section
			class="rounded-xl border border-sky-500/40 bg-sky-500/10 p-4"
			aria-label="Runs awaiting your approval"
		>
			<h2 class="text-sm font-semibold text-[rgb(var(--mv-text))]">
				{awaitingApproval.length} run{awaitingApproval.length === 1 ? '' : 's'} awaiting your approval
			</h2>
			<p class="mt-1 text-xs text-[rgb(var(--mv-muted))]">
				A parked run holds no write leases and has no deadline. Nothing is blocked while you decide.
			</p>
			<div class="mt-3 space-y-2">
				{#each awaitingApproval as run (run.run_id)}
					<div
						class="flex items-center justify-between gap-3 rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/60 px-3 py-2"
					>
						<span class="truncate text-xs text-[rgb(var(--mv-muted))]">
							Attempt {run.attempt_no} · {run.actor}
						</span>
						<button
							class="rounded-md bg-sky-500/20 px-3 py-1 text-xs font-medium text-sky-200 hover:bg-sky-500/30 disabled:opacity-50"
							disabled={busyRunId === run.run_id}
							on:click={() => handleApprove(run)}
						>
							{busyRunId === run.run_id ? 'Approving…' : 'Approve'}
						</button>
					</div>
				{/each}
			</div>
		</section>
	{/if}

	{#if loading}
		<p class="text-sm text-[rgb(var(--mv-muted))]">Loading…</p>
	{:else if !workOrders.length}
		<p class="rounded-xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/40 p-6 text-sm text-[rgb(var(--mv-muted))]">
			No work orders yet. A proposal becomes schedulable only once its declared write scope
			resolves against a Tool Grant.
		</p>
	{:else}
		<div class="grid gap-4 md:grid-cols-[18rem_1fr]">
			<nav class="space-y-2" aria-label="Work orders">
				{#each workOrders as order (order.work_order_id)}
					<button
						class={`w-full rounded-xl border px-3 py-2 text-left transition ${
							selected?.work_order_id === order.work_order_id
								? 'border-sky-500/60 bg-sky-500/10'
								: 'border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/40 hover:bg-[rgb(var(--mv-panel-strong))]'
						}`}
						aria-pressed={selected?.work_order_id === order.work_order_id}
						on:click={() => selectOrder(order)}
					>
						<p class="truncate text-sm font-medium text-[rgb(var(--mv-text))]">{order.goal}</p>
						<p class="mt-0.5 text-[11px] text-[rgb(var(--mv-muted))]">
							{order.status} · rev {order.revision}
						</p>
					</button>
				{/each}
			</nav>

			{#if selected}
				<div class="space-y-4">
					<section class="rounded-xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/40 p-4">
						<h2 class="text-sm font-semibold text-[rgb(var(--mv-text))]">{selected.goal}</h2>
						{#if selected.non_goals.length}
							<p class="mt-2 text-xs text-[rgb(var(--mv-muted))]">
								<span class="font-medium">Non-goals:</span> {selected.non_goals.join('; ')}
							</p>
						{/if}
						<dl class="mt-3 grid grid-cols-2 gap-3 md:grid-cols-4">
							{#each [['Run attempts', selected.remaining.run_attempts, selected.budget.run_attempts], ['Model tokens', selected.remaining.model_tokens, selected.budget.model_tokens], ['Effect actions', selected.remaining.effect_actions, selected.budget.effect_actions], ['Wall clock (s)', selected.remaining.wall_clock_secs, selected.budget.wall_clock_secs]] as [label, remaining, total]}
								<div class="rounded-lg border border-[rgb(var(--mv-border))] p-2">
									<dt class="text-[10px] uppercase tracking-wide text-[rgb(var(--mv-muted))]/70">
										{label}
									</dt>
									<dd class="mt-1 text-sm font-semibold text-[rgb(var(--mv-text))]">
										{remaining}<span class="text-[rgb(var(--mv-muted))]">/{total}</span>
									</dd>
								</div>
							{/each}
						</dl>
						<p class="mt-2 text-[11px] text-[rgb(var(--mv-muted))]">
							Budgets are ceilings, never refilled. Exhaustion is terminal rather than a silent
							scope reduction.
						</p>
					</section>

					<section class="space-y-3">
						<h2 class="text-sm font-semibold text-[rgb(var(--mv-text))]">Runs</h2>
						{#if !runs.length}
							<p class="text-xs text-[rgb(var(--mv-muted))]">No run attempts yet.</p>
						{/if}
						{#each runs as run (run.run_id)}
							<article class="rounded-xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/40 p-4">
								<header class="flex flex-wrap items-center justify-between gap-2">
									<div>
										<p class="text-sm font-medium text-[rgb(var(--mv-text))]">
											Attempt {run.attempt_no}
										</p>
										<p class="text-[11px] text-[rgb(var(--mv-muted))]">
											{formatWhen(run.started_at)} · {run.actor}
										</p>
									</div>
									<div class="flex items-center gap-2">
										<span class={`rounded-md px-2 py-0.5 text-[11px] ${runBadge(run.status)}`}>
											{run.status.replace(/_/g, ' ')}
										</span>
										{#if run.failure_class}
											<span class="rounded-md bg-red-500/10 px-2 py-0.5 text-[11px] text-red-300">
												{run.failure_class}
											</span>
										{/if}
										{#if run.status === 'gated' || run.status === 'verified'}
											<button
												class="rounded-md border border-[rgb(var(--mv-border))] px-2 py-0.5 text-[11px] text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-panel-strong))] disabled:opacity-50"
												disabled={busyRunId === run.run_id}
												on:click={() => handleComplete(run)}
											>
												{busyRunId === run.run_id ? 'Checking…' : 'Complete'}
											</button>
										{/if}
									</div>
								</header>

								<div class="mt-3">
									<h3 class="text-[11px] uppercase tracking-wide text-[rgb(var(--mv-muted))]/70">
										Gate evidence
									</h3>
									{#if gatesByRun[run.run_id]?.length}
										<ul class="mt-2 space-y-1">
											{#each gatesByRun[run.run_id] as gate (gate.gate + gate.evaluated_at)}
												<li class="flex flex-wrap items-center gap-2 text-xs">
													<span class={`rounded px-1.5 py-0.5 text-[11px] ${gateBadge(gate.outcome)}`}>
														{gate.gate.toUpperCase()}
													</span>
													<span class="text-[rgb(var(--mv-text))]">
														{GATE_LABELS[gate.gate] ?? gate.gate}
													</span>
													{#if gate.detail}
														<span class="text-[rgb(var(--mv-muted))]">— {gate.detail}</span>
													{/if}
													<span class="ml-auto font-mono text-[10px] text-[rgb(var(--mv-muted))]/70">
														{shortDigest(gate.evidence_digest)}
													</span>
												</li>
											{/each}
										</ul>
										<p class="mt-2 text-[11px] text-[rgb(var(--mv-muted))]/80">
											Evidence is immutable. A run can never satisfy its own independent review,
											and a failed gate is not re-rolled — recovery is a new attempt.
										</p>
									{:else}
										<p class="mt-1 text-xs text-[rgb(var(--mv-muted))]">No gates evaluated yet.</p>
									{/if}
								</div>

								{#if artifactsByRun[run.run_id]?.length}
									<div class="mt-3">
										<h3 class="text-[11px] uppercase tracking-wide text-[rgb(var(--mv-muted))]/70">
											Artifacts
										</h3>
										<ul class="mt-2 space-y-2">
											{#each artifactsByRun[run.run_id] as artifact (artifact.artifact_id)}
												<li class="rounded-lg border border-[rgb(var(--mv-border))] p-2">
													<div class="flex flex-wrap items-center gap-2 text-xs">
														<span class="text-[rgb(var(--mv-text))]">{artifact.artifact_kind}</span>
														<span class="font-mono text-[10px] text-[rgb(var(--mv-muted))]/70">
															{shortDigest(artifact.content_digest)}
														</span>
														<a
															class="ml-auto rounded-md border border-[rgb(var(--mv-border))] px-2 py-0.5 text-[11px] text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-panel-strong))]"
															href={artifactContentUrl(selected.work_order_id, artifact.artifact_id)}
															download={artifact.artifact_id}
															rel="noopener"
														>
															Download
														</a>
													</div>
													{#if artifact.provenance.length}
														<p class="mt-1 text-[11px] text-[rgb(var(--mv-muted))]">
															Derived from {artifact.provenance
																.map((reference) => reference.resource)
																.join(', ')}
														</p>
													{/if}
												</li>
											{/each}
										</ul>
										<p class="mt-2 text-[11px] text-[rgb(var(--mv-muted))]/80">
											Content is downloaded, never rendered inline: agent output is not trusted
											markup.
										</p>
									</div>
								{/if}
							</article>
						{/each}
					</section>
				</div>
			{/if}
		</div>
	{/if}
</div>
