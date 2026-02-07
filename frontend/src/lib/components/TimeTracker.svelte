<script lang="ts">
	import { createEventDispatcher, onDestroy } from 'svelte';

	export let taskId: string;
	export let estimateMin: number | null = null;
	export let timeSpentMin: number = 0;

	const dispatch = createEventDispatcher<{
		timeUpdate: { taskId: string; timeSpentMin: number; entry: TimeEntry };
	}>();

	type TimeEntry = {
		started_at: string;
		ended_at: string | null;
		minutes: number;
	};

	let isRunning = false;
	let startedAt: Date | null = null;
	let elapsed = 0; // seconds
	let interval: ReturnType<typeof setInterval> | null = null;

	// Manual entry
	let showManualEntry = false;
	let manualMinutes = 15;

	$: totalMinutes = timeSpentMin + Math.floor(elapsed / 60);
	$: progressPercent = estimateMin && estimateMin > 0 ? Math.min(100, (totalMinutes / estimateMin) * 100) : 0;
	$: isOverEstimate = estimateMin && totalMinutes > estimateMin;

	function formatTime(seconds: number): string {
		const h = Math.floor(seconds / 3600);
		const m = Math.floor((seconds % 3600) / 60);
		const s = seconds % 60;
		if (h > 0) {
			return `${h}:${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`;
		}
		return `${m}:${s.toString().padStart(2, '0')}`;
	}

	function formatMinutes(min: number): string {
		if (min < 60) return `${min}m`;
		const h = Math.floor(min / 60);
		const m = min % 60;
		return m > 0 ? `${h}h ${m}m` : `${h}h`;
	}

	function startTimer() {
		if (isRunning) return;
		isRunning = true;
		startedAt = new Date();
		elapsed = 0;
		interval = setInterval(() => {
			if (startedAt) {
				elapsed = Math.floor((Date.now() - startedAt.getTime()) / 1000);
			}
		}, 1000);
	}

	function stopTimer() {
		if (!isRunning || !startedAt) return;
		isRunning = false;
		if (interval) {
			clearInterval(interval);
			interval = null;
		}
		const endedAt = new Date();
		const minutes = Math.max(1, Math.round(elapsed / 60));
		const entry: TimeEntry = {
			started_at: startedAt.toISOString(),
			ended_at: endedAt.toISOString(),
			minutes
		};
		dispatch('timeUpdate', {
			taskId,
			timeSpentMin: timeSpentMin + minutes,
			entry
		});
		elapsed = 0;
		startedAt = null;
	}

	function addManualTime() {
		if (manualMinutes <= 0) return;
		const now = new Date();
		const entry: TimeEntry = {
			started_at: now.toISOString(),
			ended_at: now.toISOString(),
			minutes: manualMinutes
		};
		dispatch('timeUpdate', {
			taskId,
			timeSpentMin: timeSpentMin + manualMinutes,
			entry
		});
		showManualEntry = false;
		manualMinutes = 15;
	}

	onDestroy(() => {
		if (interval) {
			clearInterval(interval);
		}
	});
</script>

<div class="rounded-xl border border-slate-800 bg-slate-900/40 p-4">
	<div class="flex items-center justify-between">
		<h4 class="text-xs font-semibold uppercase tracking-wider text-slate-400">Time Tracking</h4>
		{#if !isRunning}
			<button
				class="text-[10px] text-slate-500 hover:text-white"
				on:click={() => (showManualEntry = !showManualEntry)}
			>
				{showManualEntry ? 'Cancel' : '+ Add time'}
			</button>
		{/if}
	</div>

	<!-- Progress bar -->
	{#if estimateMin && estimateMin > 0}
		<div class="mt-3">
			<div class="flex items-center justify-between text-[10px]">
				<span class="text-slate-500">Progress</span>
				<span class={isOverEstimate ? 'text-red-400' : 'text-slate-400'}>
					{formatMinutes(totalMinutes)} / {formatMinutes(estimateMin)}
				</span>
			</div>
			<div class="mt-1 h-2 overflow-hidden rounded-full bg-slate-800">
				<div
					class="h-full transition-all duration-300 {isOverEstimate ? 'bg-red-500' : 'bg-emerald-500'}"
					style="width: {progressPercent}%"
				></div>
			</div>
		</div>
	{:else}
		<div class="mt-2 text-xs text-slate-400">
			Total: <span class="font-medium text-white">{formatMinutes(totalMinutes)}</span>
		</div>
	{/if}

	<!-- Timer display -->
	<div class="mt-4 flex items-center justify-center gap-4">
		{#if isRunning}
			<div class="text-center">
				<div class="font-mono text-3xl font-bold text-emerald-400">{formatTime(elapsed)}</div>
				<div class="mt-1 text-[10px] text-slate-500">Recording time...</div>
			</div>
		{:else}
			<div class="text-center">
				<div class="font-mono text-2xl font-medium text-slate-300">{formatTime(0)}</div>
				<div class="mt-1 text-[10px] text-slate-500">Timer ready</div>
			</div>
		{/if}
	</div>

	<!-- Timer controls -->
	<div class="mt-4 flex justify-center gap-2">
		{#if isRunning}
			<button
				class="rounded-lg bg-red-500 px-6 py-2 text-sm font-semibold text-white hover:bg-red-400"
				on:click={stopTimer}
			>
				Stop
			</button>
		{:else}
			<button
				class="rounded-lg bg-emerald-500 px-6 py-2 text-sm font-semibold text-white hover:bg-emerald-400"
				on:click={startTimer}
			>
				Start Timer
			</button>
		{/if}
	</div>

	<!-- Manual entry -->
	{#if showManualEntry && !isRunning}
		<div class="mt-4 rounded-lg border border-slate-700 bg-slate-800/50 p-3">
			<label class="text-[10px] uppercase tracking-wider text-slate-500" for="manual-time-input">Add time manually</label>
			<div class="mt-2 flex items-center gap-2">
				<input
					id="manual-time-input"
					type="number"
					min="1"
					max="480"
					class="w-20 rounded-lg border border-slate-700 bg-slate-900 px-2 py-1.5 text-sm text-white"
					bind:value={manualMinutes}
				/>
				<span class="text-xs text-slate-400">minutes</span>
				<button
					class="ml-auto rounded-lg bg-sky-500 px-3 py-1.5 text-xs font-semibold text-white hover:bg-sky-400"
					on:click={addManualTime}
				>
					Add
				</button>
			</div>
		</div>
	{/if}

	<!-- Quick presets when not running -->
	{#if !isRunning && !showManualEntry}
		<div class="mt-3 flex justify-center gap-1.5">
			{#each [5, 15, 30, 60] as mins}
				<button
					class="rounded-md border border-slate-700 px-2 py-1 text-[10px] text-slate-400 hover:border-slate-500 hover:text-white"
					on:click={() => {
						manualMinutes = mins;
						addManualTime();
					}}
				>
					+{mins}m
				</button>
			{/each}
		</div>
	{/if}
</div>
