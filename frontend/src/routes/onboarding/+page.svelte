<script lang="ts">
	import { goto } from '$app/navigation';
	import { onboarding, type OnboardingStep } from '$lib/stores/onboarding';
	import { quickAddTaskOptimistic } from '$lib/stores/tasks';
	import { createNoteOptimistic } from '$lib/stores/notes';
	import { pushToast } from '$lib/stores/toast';
	import { fade, fly } from 'svelte/transition';

	let noteTitle = '';
	let noteContent = '';
	let taskTitle = '';
	let creating = false;

	$: step = $onboarding.currentStep;

	async function createFirstNote() {
		if (!noteTitle.trim()) {
			pushToast('Enter a note title', 'warning');
			return;
		}
		creating = true;
		try {
			await createNoteOptimistic(noteContent || 'My first note in MindVault!', noteTitle.trim());
			onboarding.markNoteCreated();
			pushToast('Note created!', 'success');
			onboarding.nextStep();
		} catch {
			pushToast('Failed to create note', 'danger');
		} finally {
			creating = false;
		}
	}

	async function createFirstTask() {
		if (!taskTitle.trim()) {
			pushToast('Enter a task', 'warning');
			return;
		}
		creating = true;
		try {
			await quickAddTaskOptimistic(taskTitle.trim());
			onboarding.markTaskCreated();
			pushToast('Task created!', 'success');
			onboarding.nextStep();
		} catch {
			pushToast('Failed to create task', 'danger');
		} finally {
			creating = false;
		}
	}

	function finish() {
		onboarding.nextStep();
		goto('/');
	}

	function skip() {
		onboarding.skip();
		goto('/');
	}
</script>

<div class="flex min-h-[80vh] items-center justify-center">
	<div class="w-full max-w-xl">
		{#if step === 'welcome'}
			<div
				class="rounded-2xl border border-slate-800 bg-slate-900/60 p-8 text-center"
				in:fade={{ duration: 200 }}
			>
				<div class="mx-auto mb-6 flex h-16 w-16 items-center justify-center rounded-2xl bg-sky-500/20 text-2xl text-sky-300">
					MV
				</div>
				<h1 class="text-2xl font-bold text-white">Welcome to MindVault</h1>
				<p class="mt-3 text-sm text-slate-400">
					Your local-first knowledge and execution workspace.
					<br />
					Let's get you started in under a minute.
				</p>

				<div class="mt-8 flex justify-center gap-3">
					<button
						class="rounded-lg border border-slate-700 px-4 py-2 text-sm text-slate-300 hover:bg-slate-800"
						on:click={skip}
					>
						Skip for now
					</button>
					<button
						class="rounded-lg bg-sky-500 px-6 py-2 text-sm font-semibold text-white hover:bg-sky-400"
						on:click={() => onboarding.nextStep()}
					>
						Let's go
					</button>
				</div>
			</div>
		{:else if step === 'create-note'}
			<div
				class="rounded-2xl border border-slate-800 bg-slate-900/60 p-8"
				in:fly={{ x: 20, duration: 200 }}
			>
				<div class="flex items-center gap-2">
					<span class="rounded-full bg-sky-500/20 px-2.5 py-1 text-[10px] font-medium text-sky-300">Step 1 of 3</span>
				</div>
				<h2 class="mt-4 text-xl font-semibold text-white">Create your first note</h2>
				<p class="mt-2 text-sm text-slate-400">
					Notes are the building blocks of your knowledge. They can be linked together, searched, and organized with tags.
				</p>

				<div class="mt-6">
					<label class="text-xs uppercase tracking-wide text-slate-500" for="note-title">
						Title
					</label>
					<input
						id="note-title"
						class="mt-1 w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-sm text-white placeholder-slate-500"
						placeholder="e.g., Ideas for the weekend"
						bind:value={noteTitle}
					/>
				</div>

				<div class="mt-4">
					<label class="text-xs uppercase tracking-wide text-slate-500" for="note-content">
						Content (optional)
					</label>
					<textarea
						id="note-content"
						class="mt-1 h-24 w-full resize-none rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-sm text-white placeholder-slate-500"
						placeholder="Write something..."
						bind:value={noteContent}
					></textarea>
				</div>

				<div class="mt-6 flex justify-between">
					<button
						class="text-sm text-slate-500 hover:text-white"
						on:click={skip}
					>
						Skip onboarding
					</button>
					<div class="flex gap-2">
						<button
							class="rounded-lg border border-slate-700 px-4 py-2 text-sm text-slate-300 hover:bg-slate-800"
							on:click={() => onboarding.nextStep()}
						>
							Skip this step
						</button>
						<button
							class="rounded-lg bg-sky-500 px-4 py-2 text-sm font-semibold text-white hover:bg-sky-400 disabled:opacity-50"
							on:click={createFirstNote}
							disabled={creating}
						>
							{creating ? 'Creating...' : 'Create note'}
						</button>
					</div>
				</div>
			</div>
		{:else if step === 'create-task'}
			<div
				class="rounded-2xl border border-slate-800 bg-slate-900/60 p-8"
				in:fly={{ x: 20, duration: 200 }}
			>
				<div class="flex items-center gap-2">
					<span class="rounded-full bg-violet-500/20 px-2.5 py-1 text-[10px] font-medium text-violet-300">Step 2 of 3</span>
				</div>
				<h2 class="mt-4 text-xl font-semibold text-white">Add your first task</h2>
				<p class="mt-2 text-sm text-slate-400">
					MindVault understands natural language. Try adding a due date, priority, or tags.
				</p>

				<div class="mt-6">
					<label class="text-xs uppercase tracking-wide text-slate-500" for="task-title">
						Task
					</label>
					<input
						id="task-title"
						class="mt-1 w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-sm text-white placeholder-slate-500"
						placeholder="e.g., Buy groceries tomorrow 5pm #personal"
						bind:value={taskTitle}
					/>
					<p class="mt-2 text-[10px] text-slate-500">
						Try: "Meeting with Alex next Monday 2pm p2 #work"
					</p>
				</div>

				<div class="mt-6 flex justify-between">
					<button
						class="text-sm text-slate-500 hover:text-white"
						on:click={() => onboarding.prevStep()}
					>
						Back
					</button>
					<div class="flex gap-2">
						<button
							class="rounded-lg border border-slate-700 px-4 py-2 text-sm text-slate-300 hover:bg-slate-800"
							on:click={() => onboarding.nextStep()}
						>
							Skip this step
						</button>
						<button
							class="rounded-lg bg-violet-500 px-4 py-2 text-sm font-semibold text-white hover:bg-violet-400 disabled:opacity-50"
							on:click={createFirstTask}
							disabled={creating}
						>
							{creating ? 'Creating...' : 'Add task'}
						</button>
					</div>
				</div>
			</div>
		{:else if step === 'shortcuts'}
			<div
				class="rounded-2xl border border-slate-800 bg-slate-900/60 p-8"
				in:fly={{ x: 20, duration: 200 }}
			>
				<div class="flex items-center gap-2">
					<span class="rounded-full bg-emerald-500/20 px-2.5 py-1 text-[10px] font-medium text-emerald-300">Step 3 of 3</span>
				</div>
				<h2 class="mt-4 text-xl font-semibold text-white">Key shortcuts</h2>
				<p class="mt-2 text-sm text-slate-400">
					Master these shortcuts to work at the speed of thought.
				</p>

				<div class="mt-6 space-y-3">
					<div class="flex items-center justify-between rounded-lg border border-slate-800 bg-slate-800/50 px-4 py-3">
						<div>
							<div class="text-sm font-medium text-white">Command Palette</div>
							<div class="text-[11px] text-slate-500">Access any action instantly</div>
						</div>
						<kbd class="rounded border border-slate-700 bg-slate-800 px-2 py-1 text-xs text-slate-300">Cmd+K</kbd>
					</div>
					<div class="flex items-center justify-between rounded-lg border border-slate-800 bg-slate-800/50 px-4 py-3">
						<div>
							<div class="text-sm font-medium text-white">Quick Capture</div>
							<div class="text-[11px] text-slate-500">Capture notes, tasks, or links</div>
						</div>
						<kbd class="rounded border border-slate-700 bg-slate-800 px-2 py-1 text-xs text-slate-300">Cmd+Shift+N</kbd>
					</div>
					<div class="flex items-center justify-between rounded-lg border border-slate-800 bg-slate-800/50 px-4 py-3">
						<div>
							<div class="text-sm font-medium text-white">Quick Search</div>
							<div class="text-[11px] text-slate-500">Find anything fast</div>
						</div>
						<kbd class="rounded border border-slate-700 bg-slate-800 px-2 py-1 text-xs text-slate-300">Cmd+/</kbd>
					</div>
				</div>

				<div class="mt-6 flex justify-between">
					<button
						class="text-sm text-slate-500 hover:text-white"
						on:click={() => onboarding.prevStep()}
					>
						Back
					</button>
					<button
						class="rounded-lg bg-emerald-500 px-6 py-2 text-sm font-semibold text-white hover:bg-emerald-400"
						on:click={finish}
					>
						Get started
					</button>
				</div>
			</div>
		{:else}
			<div
				class="rounded-2xl border border-slate-800 bg-slate-900/60 p-8 text-center"
				in:fade={{ duration: 200 }}
			>
				<div class="mx-auto mb-4 flex h-12 w-12 items-center justify-center rounded-full bg-emerald-500/20 text-xl text-emerald-300">
					&#10003;
				</div>
				<h2 class="text-xl font-semibold text-white">You're all set!</h2>
				<p class="mt-2 text-sm text-slate-400">
					Start exploring MindVault and build your knowledge base.
				</p>
				<button
					class="mt-6 rounded-lg bg-sky-500 px-6 py-2 text-sm font-semibold text-white hover:bg-sky-400"
					on:click={() => goto('/')}
				>
					Go to Home
				</button>
			</div>
		{/if}

		<!-- Progress indicator -->
		{#if step !== 'welcome' && step !== 'complete'}
			<div class="mt-6 flex justify-center gap-2">
				{#each ['create-note', 'create-task', 'shortcuts'] as s, i}
					<div
						class="h-1.5 w-8 rounded-full transition {
							step === s
								? 'bg-sky-500'
								: ['create-note', 'create-task', 'shortcuts'].indexOf(step) > i
									? 'bg-emerald-500'
									: 'bg-slate-700'
						}"
					></div>
				{/each}
			</div>
		{/if}
	</div>
</div>
