<script lang="ts">
	/* eslint-disable svelte/no-navigation-without-resolve -- Artifact links target the absolute API download surface, not an app route. */
	import { onMount } from 'svelte';
	import {
		Bot,
		Check,
		CircleAlert,
		Clock3,
		RefreshCw,
		ShieldCheck,
		Target,
		X
	} from '@lucide/svelte';
	import { agentRunObservations } from '$lib/api/agent';
	import Button from '$lib/components/Button.svelte';
	import EmptyState from '$lib/components/EmptyState.svelte';
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
		recordArtifact,
		recordGate,
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
	let listError = '';
	let detailLoading = false;
	let detailError = '';
	let busyRunId = '';
	let liveRefreshTimer: ReturnType<typeof setTimeout> | null = null;
	let lastNotifiedApprovalRunId = '';
	let detailRequestSequence = 0;

	$: awaitingApproval = runs.filter((run) => run.status === AWAITING_APPROVAL);
	$: activeWorkOrders = workOrders.filter((order) =>
		['admitted', 'running'].includes(order.status)
	).length;
	$: completedWorkOrders = workOrders.filter((order) => order.status === 'completed').length;
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
		listError = '';
		try {
			const nextWorkOrders = await listWorkOrders();
			workOrders = nextWorkOrders;
			const nextSelection = selected
				? nextWorkOrders.find((order) => order.work_order_id === selected?.work_order_id)
				: nextWorkOrders[0];

			if (nextSelection) {
				await selectOrder(nextSelection);
			} else {
				selected = null;
				runs = [];
				gatesByRun = {};
				artifacts = [];
			}
		} catch {
			listError = 'MindVault could not reach the governed execution service.';
			pushToast('Failed to load work orders', 'danger');
		} finally {
			loading = false;
		}
	}

	async function selectOrder(order: WorkOrderSummary) {
		const requestSequence = ++detailRequestSequence;
		selected = order;
		detailLoading = true;
		detailError = '';
		runs = [];
		gatesByRun = {};
		artifacts = [];
		try {
			const [nextRuns, nextArtifacts] = await Promise.all([
				listRuns(order.work_order_id),
				listArtifacts(order.work_order_id)
			]);
			const collected = await Promise.all(
				nextRuns.map(
					async (run) => [run.run_id, await listGates(order.work_order_id, run.run_id)] as const
				)
			);
			if (requestSequence !== detailRequestSequence) return;
			runs = nextRuns;
			artifacts = nextArtifacts;
			gatesByRun = Object.fromEntries(collected);
		} catch {
			if (requestSequence !== detailRequestSequence) return;
			detailError = 'Run evidence and artifacts are temporarily unavailable.';
			pushToast('Failed to load run detail', 'danger');
		} finally {
			if (requestSequence === detailRequestSequence) detailLoading = false;
		}
	}

	async function refreshSelected() {
		if (!selected) return;
		selected = await getWorkOrder(selected.work_order_id);
		await selectOrder(selected);
	}

	function retrySelected() {
		if (selected) void selectOrder(selected);
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

	/**
	 * Base64 without blowing the call stack.
	 *
	 * `String.fromCharCode(...bytes)` throws on large inputs, and an artifact
	 * may be a diff or a binary rather than a short note.
	 */
	function toBase64(bytes: Uint8Array): string {
		let binary = '';
		const chunk = 0x8000;
		for (let i = 0; i < bytes.length; i += chunk) {
			binary += String.fromCharCode(...bytes.subarray(i, i + chunk));
		}
		return btoa(binary);
	}

	async function handleAttach(run: AgentRunView, event: Event) {
		const input = event.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		if (!file || !selected) return;
		busyRunId = run.run_id;
		try {
			const bytes = new Uint8Array(await file.arrayBuffer());
			await recordArtifact(selected.work_order_id, run.run_id, {
				artifact_kind: file.type || 'application/octet-stream',
				content_base64: toBase64(bytes),
				// The run generated it. The server derives the digest from the
				// bytes, so there is nothing for the operator to attest to here.
				provenance: [{ relation: 'WasGeneratedBy', resource: run.run_uri }]
			});
			pushToast(`Attached ${file.name}`, 'success');
			await refreshSelected();
		} catch {
			pushToast('Failed to attach output', 'danger');
		} finally {
			busyRunId = '';
			input.value = '';
		}
	}

	async function handleVerifyOutputs(run: AgentRunView) {
		if (!selected) return;
		busyRunId = run.run_id;
		try {
			// The server evaluates G2 against stored content and records its own
			// verdict; the values sent here are placeholders it discards.
			const result = await recordGate(selected.work_order_id, run.run_id, {
				gate: 'g2',
				outcome: 'fail',
				evaluator_actor: run.actor,
				evidence_digest: '0'.repeat(64)
			});
			pushToast(
				result.outcome === 'pass'
					? 'Outputs verified against stored content'
					: `G2 failed: ${result.detail ?? 'no artifacts to verify'}`,
				result.outcome === 'pass' ? 'success' : 'warning'
			);
			await refreshSelected();
		} catch {
			pushToast('Failed to verify outputs', 'danger');
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

	/** Terminal runs accept no further evidence; the server refuses it too. */
	function isTerminal(status: string): boolean {
		return ['completed', 'failed', 'cancelled', 'budget_exhausted'].includes(status);
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

	function orderBadge(status: WorkOrderSummary['status']): string {
		switch (status) {
			case 'completed':
				return 'bg-emerald-500/15 text-emerald-300 ring-emerald-500/25';
			case 'admitted':
			case 'running':
				return 'bg-violet-500/15 text-violet-300 ring-violet-500/25';
			case 'failed':
			case 'budget_exhausted':
			case 'rejected':
				return 'bg-red-500/15 text-red-300 ring-red-500/25';
			case 'cancelled':
				return 'bg-slate-500/15 text-slate-300 ring-slate-500/25';
			default:
				return 'bg-amber-500/15 text-amber-300 ring-amber-500/25';
		}
	}

	function readableStatus(status: string): string {
		return status.replace(/_/g, ' ').replace(/\b\w/g, (character) => character.toUpperCase());
	}

	function readableActor(actor: string): string {
		const segment = actor.split('/').filter(Boolean).at(-1) ?? actor;
		return segment.replace(/[-_]/g, ' ');
	}

	function formatWhen(iso: string | null): string {
		return iso ? new Date(iso).toLocaleString() : '—';
	}

	function shortDigest(digest: string): string {
		return `${digest.slice(0, 12)}…`;
	}
</script>

<div class="mx-auto max-w-6xl space-y-6 p-4 sm:p-6">
	<section
		class="rounded-2xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/45 p-4 shadow-[var(--mv-shadow-sm)] sm:p-5"
		aria-labelledby="execution-overview-title"
	>
		<div class="flex flex-wrap items-start justify-between gap-4">
			<div class="max-w-2xl">
				<div class="mb-2 flex items-center gap-2 text-xs font-semibold text-violet-300">
					<ShieldCheck size={15} aria-hidden="true" />
					<span>Human-governed execution</span>
				</div>
				<h2 id="execution-overview-title" class="text-lg font-semibold text-[rgb(var(--mv-text))]">
					Review agent work before it becomes an outcome
				</h2>
				<p class="mt-1 text-sm leading-6 text-[rgb(var(--mv-muted))]">
					Inspect the goal, success criteria, scope boundaries, evidence, and outputs for every run.
					Approval authorizes a fresh conflict check; it never bypasses policy gates.
				</p>
			</div>
			<Button variant="secondary" size="md" disabled={loading} on:click={loadWorkOrders}>
				<RefreshCw size={15} class={loading ? 'animate-spin' : ''} aria-hidden="true" />
				{loading ? 'Refreshing…' : 'Refresh'}
			</Button>
		</div>

		<dl class="mt-5 grid grid-cols-3 gap-2 sm:gap-3" aria-label="Work order summary">
			<div
				class="rounded-xl border border-[rgb(var(--mv-border))]/70 bg-[rgb(var(--mv-bg))]/25 p-3"
			>
				<dt
					class="text-[10px] font-semibold uppercase tracking-wider text-[rgb(var(--mv-muted))]/75"
				>
					All orders
				</dt>
				<dd class="mt-1 text-lg font-semibold text-[rgb(var(--mv-text))]">{workOrders.length}</dd>
			</div>
			<div
				class="rounded-xl border border-[rgb(var(--mv-border))]/70 bg-[rgb(var(--mv-bg))]/25 p-3"
			>
				<dt
					class="text-[10px] font-semibold uppercase tracking-wider text-[rgb(var(--mv-muted))]/75"
				>
					Active
				</dt>
				<dd class="mt-1 text-lg font-semibold text-violet-300">{activeWorkOrders}</dd>
			</div>
			<div
				class="rounded-xl border border-[rgb(var(--mv-border))]/70 bg-[rgb(var(--mv-bg))]/25 p-3"
			>
				<dt
					class="text-[10px] font-semibold uppercase tracking-wider text-[rgb(var(--mv-muted))]/75"
				>
					Completed
				</dt>
				<dd class="mt-1 text-lg font-semibold text-emerald-300">{completedWorkOrders}</dd>
			</div>
		</dl>
	</section>
	{#if listError && workOrders.length}
		<div
			class="flex flex-wrap items-center justify-between gap-3 rounded-xl border border-amber-500/30 bg-amber-500/10 p-3"
			role="alert"
		>
			<div class="flex items-start gap-2 text-xs text-amber-100">
				<CircleAlert size={16} class="mt-0.5 shrink-0" aria-hidden="true" />
				<span>{listError} Showing the last available view.</span>
			</div>
			<Button variant="secondary" size="sm" on:click={loadWorkOrders}>Try again</Button>
		</div>
	{/if}

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
						<span class="flex min-w-0 items-center gap-2 text-xs text-[rgb(var(--mv-muted))]">
							<Bot size={15} class="shrink-0 text-violet-300" aria-hidden="true" />
							<span class="truncate"
								>Attempt {run.attempt_no} · Agent {readableActor(run.actor)}</span
							>
						</span>
						<button
							class="min-h-9 shrink-0 rounded-lg bg-sky-500/20 px-3 text-xs font-medium text-sky-200 transition hover:bg-sky-500/30 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-sky-400/70 disabled:opacity-50"
							disabled={busyRunId === run.run_id}
							on:click={() => handleApprove(run)}
						>
							{busyRunId === run.run_id ? 'Approving…' : 'Approve run'}
						</button>
					</div>
				{/each}
			</div>
		</section>
	{/if}

	{#if loading && !workOrders.length}
		<div
			class="grid gap-3 md:grid-cols-[18rem_1fr]"
			role="status"
			aria-label="Loading governed work"
		>
			<div class="h-40 animate-pulse rounded-xl bg-[rgb(var(--mv-panel))]/45"></div>
			<div class="h-64 animate-pulse rounded-xl bg-[rgb(var(--mv-panel))]/45"></div>
		</div>
	{:else if listError && !workOrders.length}
		<EmptyState
			title="Governed work is unavailable"
			description={`${listError} Your existing work remains unchanged.`}
			icon="generic"
			tone="amber"
			actionLabel="Try again"
			onAction={loadWorkOrders}
		/>
	{:else if !workOrders.length}
		<EmptyState
			title="No governed work yet"
			description="Work appears here after a proposal declares its goal, success criteria, limits, and permitted write scope."
			icon="tasks"
			tone="violet"
		/>
	{:else}
		<div class="grid gap-4 md:grid-cols-[18rem_1fr]">
			<nav class="space-y-2" aria-label="Work order queue">
				<div class="flex items-center justify-between px-1 pb-1">
					<h2 class="text-xs font-semibold uppercase tracking-wider text-[rgb(var(--mv-muted))]/75">
						Order queue
					</h2>
					<span class="text-xs text-[rgb(var(--mv-muted))]">{workOrders.length}</span>
				</div>
				{#each workOrders as order (order.work_order_id)}
					<button
						class={`min-h-14 w-full rounded-xl border px-3 py-2.5 text-left transition focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[rgb(var(--mv-ring))]/70 ${
							selected?.work_order_id === order.work_order_id
								? 'border-violet-500/60 bg-violet-500/10'
								: 'border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/40 hover:bg-[rgb(var(--mv-panel-strong))]'
						}`}
						aria-pressed={selected?.work_order_id === order.work_order_id}
						on:click={() => selectOrder(order)}
					>
						<p class="truncate text-sm font-medium text-[rgb(var(--mv-text))]">{order.goal}</p>
						<div class="mt-1 flex min-w-0 items-center justify-between gap-2">
							<span
								class="min-w-0 truncate text-[11px] text-[rgb(var(--mv-muted))]"
								title={`Updated ${formatWhen(order.updated_at)}`}
							>
								Updated {formatWhen(order.updated_at)}
							</span>
							<span
								class={`shrink-0 rounded-md px-1.5 py-0.5 text-[10px] font-semibold ring-1 ring-inset ${orderBadge(order.status)}`}
							>
								{readableStatus(order.status)}
							</span>
						</div>
					</button>
				{/each}
			</nav>

			{#if selected}
				<div class="space-y-4">
					<section
						class="rounded-xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/40 p-4 sm:p-5"
						aria-labelledby="selected-order-title"
					>
						<div class="flex flex-wrap items-start justify-between gap-3">
							<div class="min-w-0">
								<p
									class="text-[10px] font-semibold uppercase tracking-wider text-[rgb(var(--mv-muted))]/75"
								>
									Selected work order
								</p>
								<h2
									id="selected-order-title"
									class="mt-1 text-base font-semibold leading-6 text-[rgb(var(--mv-text))]"
								>
									{selected.goal}
								</h2>
								<p class="mt-1 text-[11px] text-[rgb(var(--mv-muted))]">
									Revision {selected.revision} · Updated {formatWhen(selected.updated_at)}
								</p>
							</div>
							<span
								class={`shrink-0 rounded-lg px-2.5 py-1 text-xs font-semibold ring-1 ring-inset ${orderBadge(selected.status)}`}
							>
								{readableStatus(selected.status)}
							</span>
						</div>

						{#if selected.status_reason}
							<div
								class="mt-4 flex gap-2 rounded-lg border border-amber-500/30 bg-amber-500/10 p-3 text-xs text-amber-100"
								role="status"
							>
								<CircleAlert size={16} class="mt-0.5 shrink-0" aria-hidden="true" />
								<span>{selected.status_reason}</span>
							</div>
						{/if}

						<div class="mt-5 grid gap-3 lg:grid-cols-2">
							<div class="rounded-xl border border-emerald-500/20 bg-emerald-500/5 p-3.5">
								<div class="flex items-center gap-2 text-xs font-semibold text-emerald-300">
									<Target size={15} aria-hidden="true" />
									<h3>What success means</h3>
								</div>
								<ul class="mt-3 space-y-2">
									{#each selected.success_criteria as criterion, criterionIndex (criterionIndex)}
										<li class="flex gap-2 text-xs leading-5 text-[rgb(var(--mv-text))]/85">
											<Check
												size={14}
												class="mt-0.5 shrink-0 text-emerald-300"
												aria-hidden="true"
											/>
											<span>{criterion}</span>
										</li>
									{/each}
								</ul>
							</div>
							<div
								class="rounded-xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-bg))]/20 p-3.5"
							>
								<div
									class="flex items-center gap-2 text-xs font-semibold text-[rgb(var(--mv-muted))]"
								>
									<X size={15} aria-hidden="true" />
									<h3>Outside this order</h3>
								</div>
								{#if selected.non_goals.length}
									<ul class="mt-3 space-y-2">
										{#each selected.non_goals as nonGoal, nonGoalIndex (nonGoalIndex)}
											<li class="text-xs leading-5 text-[rgb(var(--mv-muted))]">{nonGoal}</li>
										{/each}
									</ul>
								{:else}
									<p class="mt-3 text-xs leading-5 text-[rgb(var(--mv-muted))]">
										No additional non-goals were declared.
									</p>
								{/if}
							</div>
						</div>

						<h3 class="mt-5 text-xs font-semibold text-[rgb(var(--mv-text))]">Remaining limits</h3>
						<dl class="mt-2 grid grid-cols-2 gap-3 md:grid-cols-4">
							{#each [['Run attempts', selected.remaining.run_attempts, selected.budget.run_attempts], ['Model tokens', selected.remaining.model_tokens, selected.budget.model_tokens], ['Effect actions', selected.remaining.effect_actions, selected.budget.effect_actions], ['Wall clock (s)', selected.remaining.wall_clock_secs, selected.budget.wall_clock_secs]] as [label, remaining, total] (label)}
								<div
									class="rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-bg))]/20 p-2.5"
								>
									<dt class="text-[10px] uppercase tracking-wide text-[rgb(var(--mv-muted))]/70">
										{label}
									</dt>
									<dd class="mt-1 text-sm font-semibold text-[rgb(var(--mv-text))]">
										{remaining}<span class="text-[rgb(var(--mv-muted))]">/{total}</span>
									</dd>
								</div>
							{/each}
						</dl>
						<p
							class="mt-2 flex items-start gap-1.5 text-[11px] leading-4 text-[rgb(var(--mv-muted))]"
						>
							<Clock3 size={13} class="mt-0.5 shrink-0" aria-hidden="true" />
							<span
								>Limits never refill. Exhaustion ends the run instead of silently narrowing scope.</span
							>
						</p>
					</section>

					<section class="space-y-3">
						<div class="flex items-center justify-between gap-3">
							<h2 class="text-sm font-semibold text-[rgb(var(--mv-text))]">Run history</h2>
							<span class="text-xs text-[rgb(var(--mv-muted))]">
								{runs.length} attempt{runs.length === 1 ? '' : 's'}
							</span>
						</div>
						{#if detailLoading}
							<div
								class="h-32 animate-pulse rounded-xl bg-[rgb(var(--mv-panel))]/45"
								role="status"
								aria-label="Loading run evidence"
							></div>
						{:else if detailError}
							<div class="rounded-xl border border-amber-500/30 bg-amber-500/10 p-4" role="alert">
								<p class="text-sm font-medium text-amber-100">Run detail is unavailable</p>
								<p class="mt-1 text-xs text-[rgb(var(--mv-muted))]">{detailError}</p>
								<div class="mt-3">
									<Button variant="secondary" size="sm" on:click={retrySelected}>Try again</Button>
								</div>
							</div>
						{:else if !runs.length}
							<EmptyState
								title="No run attempts yet"
								description="This order has not started an agent run. Its declared goal and limits remain unchanged."
								icon="tasks"
								tone="slate"
								compact
							/>
						{/if}
						{#if !detailLoading && !detailError}
							{#each runs as run (run.run_id)}
								<article
									class="rounded-xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/40 p-4 sm:p-5"
								>
									<header class="flex flex-wrap items-start justify-between gap-3">
										<div>
											<p class="text-sm font-medium text-[rgb(var(--mv-text))]">
												Attempt {run.attempt_no}
											</p>
											<div
												class="mt-1 flex flex-wrap items-center gap-2 text-[11px] text-[rgb(var(--mv-muted))]"
											>
												<span>{formatWhen(run.started_at)}</span>
												<span aria-hidden="true">·</span>
												<span
													class="inline-flex items-center gap-1.5 rounded-md bg-violet-500/10 px-1.5 py-0.5 text-violet-300"
												>
													<Bot size={12} aria-hidden="true" />
													Agent {readableActor(run.actor)}
												</span>
											</div>
										</div>
										<div class="flex flex-wrap items-center justify-end gap-2">
											<span
												class={`rounded-md px-2 py-1 text-[11px] font-medium ${runBadge(run.status)}`}
											>
												{readableStatus(run.status)}
											</span>
											{#if run.failure_class}
												<span class="rounded-md bg-red-500/10 px-2 py-1 text-[11px] text-red-300">
													{readableStatus(run.failure_class)}
												</span>
											{/if}
										</div>
									</header>

									{#if !isTerminal(run.status)}
										<div
											class="mt-4 flex flex-wrap items-center gap-2 border-t border-[rgb(var(--mv-border))]/70 pt-3"
										>
											<label
												class="inline-flex min-h-9 cursor-pointer items-center rounded-lg border border-[rgb(var(--mv-border))] px-3 text-xs font-medium text-[rgb(var(--mv-muted))] transition hover:bg-[rgb(var(--mv-panel-strong))] focus-within:ring-2 focus-within:ring-[rgb(var(--mv-ring))]/70"
											>
												Attach output
												<input
													type="file"
													class="sr-only"
													disabled={busyRunId === run.run_id}
													on:change={(event) => handleAttach(run, event)}
												/>
											</label>
											<button
												class="min-h-9 rounded-lg border border-[rgb(var(--mv-border))] px-3 text-xs font-medium text-[rgb(var(--mv-muted))] transition hover:bg-[rgb(var(--mv-panel-strong))] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[rgb(var(--mv-ring))]/70 disabled:opacity-50"
												disabled={busyRunId === run.run_id}
												on:click={() => handleVerifyOutputs(run)}
												title="Gate G2 is evaluated by the server against stored content"
											>
												Verify outputs
											</button>
											{#if run.status === 'gated' || run.status === 'verified'}
												<button
													class="min-h-9 rounded-lg border border-[rgb(var(--mv-border))] px-3 text-xs font-medium text-[rgb(var(--mv-muted))] transition hover:bg-[rgb(var(--mv-panel-strong))] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[rgb(var(--mv-ring))]/70 disabled:opacity-50"
													disabled={busyRunId === run.run_id}
													on:click={() => handleComplete(run)}
												>
													{busyRunId === run.run_id ? 'Checking…' : 'Complete'}
												</button>
											{/if}
										</div>
									{/if}

									<div class="mt-3">
										<h3 class="text-[11px] uppercase tracking-wide text-[rgb(var(--mv-muted))]/70">
											Gate evidence
										</h3>
										{#if gatesByRun[run.run_id]?.length}
											<ul class="mt-2 space-y-1">
												{#each gatesByRun[run.run_id] as gate (gate.gate + gate.evaluated_at)}
													<li class="flex flex-wrap items-center gap-2 text-xs">
														<span
															class={`rounded px-1.5 py-0.5 text-[11px] ${gateBadge(gate.outcome)}`}
														>
															{gate.gate.toUpperCase()}
														</span>
														<span class="text-[rgb(var(--mv-text))]">
															{GATE_LABELS[gate.gate] ?? gate.gate}
														</span>
														{#if gate.detail}
															<span class="text-[rgb(var(--mv-muted))]">— {gate.detail}</span>
														{/if}
														<span
															class="ml-auto font-mono text-[10px] text-[rgb(var(--mv-muted))]/70"
														>
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
											<p class="mt-1 text-xs text-[rgb(var(--mv-muted))]">
												No gates evaluated yet.
											</p>
										{/if}
									</div>

									{#if artifactsByRun[run.run_id]?.length}
										<div class="mt-3">
											<h3
												class="text-[11px] uppercase tracking-wide text-[rgb(var(--mv-muted))]/70"
											>
												Artifacts
											</h3>
											<ul class="mt-2 space-y-2">
												{#each artifactsByRun[run.run_id] as artifact (artifact.artifact_id)}
													<li class="rounded-lg border border-[rgb(var(--mv-border))] p-2">
														<div class="flex flex-wrap items-center gap-2 text-xs">
															<span class="text-[rgb(var(--mv-text))]"
																>{artifact.artifact_kind}</span
															>
															<span class="font-mono text-[10px] text-[rgb(var(--mv-muted))]/70">
																{shortDigest(artifact.content_digest)}
															</span>
															<a
																class="ml-auto inline-flex min-h-9 items-center rounded-lg border border-[rgb(var(--mv-border))] px-3 text-xs font-medium text-[rgb(var(--mv-muted))] transition hover:bg-[rgb(var(--mv-panel-strong))] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[rgb(var(--mv-ring))]/70"
																href={artifactContentUrl(
																	selected.work_order_id,
																	artifact.artifact_id
																)}
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
						{/if}
					</section>
				</div>
			{/if}
		</div>
	{/if}
</div>
