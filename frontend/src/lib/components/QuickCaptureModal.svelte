<script lang="ts">
	import { tick } from 'svelte';
	import { quickAddTaskOptimistic } from '$lib/stores/tasks';
	import { createNoteOptimistic } from '$lib/stores/notes';
	import { createNode } from '$lib/api/nodes';
	import { pushToast } from '$lib/stores/toast';
	import { uploadVoiceNote, type VoiceUploadProgress } from '$lib/api/voice';
	import type { NodeKind } from '$lib/api/types';

	const NOTE_KINDS: { value: NodeKind; label: string; description: string }[] = [
		{ value: 'fact', label: 'Note', description: 'General note or thought' },
		{ value: 'decision', label: 'Decision', description: 'Choice made with rationale' },
		{ value: 'observation', label: 'Observation', description: 'Insight or learning' },
		{ value: 'procedure', label: 'Procedure', description: 'Step-by-step process' },
		{ value: 'code_snippet', label: 'Code', description: 'Reusable code snippet' },
		{ value: 'preference', label: 'Preference', description: 'Personal preference or setting' },
		{ value: 'concept', label: 'Concept', description: 'Abstract idea or definition' }
	];

	let open = false;
	let captureType: 'task' | 'note' | 'link' | 'voice' = 'task';
	let noteKind: NodeKind = 'fact';
	let showKindPicker = false;
	let text = '';
	let saving = false;
	let inputEl: HTMLInputElement | null = null;

	// Voice recording state
	let isRecording = false;
	let recorder: MediaRecorder | null = null;
	let audioChunks: Blob[] = [];
	let recordingDuration = 0;
	let recordingTimer: ReturnType<typeof setInterval> | null = null;
	let audioBlob: Blob | null = null;
	let audioUrl: string | null = null;
	let voiceEnabled = localStorage.getItem('mv_feature_voice') !== 'false';
	let uploadPhase: 'idle' | 'uploading' | 'transcribing' = 'idle';

	function formatRecordingTime(seconds: number): string {
		const m = Math.floor(seconds / 60);
		const s = seconds % 60;
		return `${m}:${s.toString().padStart(2, '0')}`;
	}

	async function startRecording() {
		try {
			const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
			recorder = new MediaRecorder(stream);
			audioChunks = [];
			recordingDuration = 0;

			recorder.ondataavailable = (e) => {
				if (e.data.size > 0) audioChunks.push(e.data);
			};

			recorder.onstop = () => {
				audioBlob = new Blob(audioChunks, { type: 'audio/webm' });
				audioUrl = URL.createObjectURL(audioBlob);
				stream.getTracks().forEach((t) => t.stop());
				if (recordingTimer) clearInterval(recordingTimer);
			};

			recorder.start();
			isRecording = true;
			recordingTimer = setInterval(() => {
				recordingDuration++;
			}, 1000);
		} catch {
			pushToast('Microphone access denied', 'danger');
		}
	}

	function stopRecording() {
		if (recorder && recorder.state === 'recording') {
			recorder.stop();
			isRecording = false;
		}
	}

	function discardRecording() {
		if (audioUrl) URL.revokeObjectURL(audioUrl);
		audioBlob = null;
		audioUrl = null;
		recordingDuration = 0;
	}

	function handleGlobalKeydown(event: KeyboardEvent) {
		if ((event.metaKey || event.ctrlKey) && event.shiftKey && event.key === 'n') {
			event.preventDefault();
			toggle();
		}
		if (event.key === 'Escape' && open) {
			close();
		}
	}

	async function toggle() {
		open = !open;
		if (open) {
			text = '';
			captureType = 'task';
			noteKind = 'fact';
			showKindPicker = false;
			discardRecording();
			await tick();
			inputEl?.focus();
		}
	}

	function close() {
		open = false;
		text = '';
		if (isRecording) stopRecording();
		discardRecording();
	}

	async function save() {
		if (captureType === 'voice') {
			return saveVoiceNote();
		}
		const value = text.trim();
		if (!value || saving) return;
		saving = true;
		try {
			if (captureType === 'task') {
				await quickAddTaskOptimistic(value);
				pushToast('Task captured', 'success');
			} else if (captureType === 'link') {
				const url = value.startsWith('http') ? value : `https://${value}`;
				const title = new URL(url).hostname.replace('www.', '');
				await createNode({
					kind: 'reference',
					title: title,
					content: '',
					source: url,
					tags: ['inbox', 'web-clip']
				});
				pushToast('Link saved', 'success');
			} else {
				const title = value.split('\n')[0].slice(0, 100) || 'Quick note';
				if (noteKind === 'fact') {
					await createNoteOptimistic(value, title);
				} else {
					await createNode({
						kind: noteKind,
						title: title,
						content: value,
						tags: ['inbox']
					});
				}
				const kindLabel = NOTE_KINDS.find(k => k.value === noteKind)?.label ?? 'Note';
				pushToast(`${kindLabel} captured`, 'success');
			}
			close();
		} catch {
			pushToast('Capture failed', 'danger');
		} finally {
			saving = false;
		}
	}

	async function saveVoiceNote() {
		if (!audioBlob || saving) return;
		saving = true;
		uploadPhase = 'idle';
		try {
			const title = text.trim() || `Voice note ${new Date().toLocaleString()}`;
			const isOnline = navigator.onLine;

			if (isOnline) {
				const file = new File([audioBlob], 'voice-note.webm', { type: 'audio/webm' });
				uploadPhase = 'uploading';
				const result = await uploadVoiceNote(file, {
					title,
					tags: [],
					onProgress: (p: VoiceUploadProgress) => {
						uploadPhase = p.phase;
					}
				});
				const preview = result.transcript.length > 80
					? result.transcript.slice(0, 80) + '...'
					: result.transcript;
				pushToast(`Voice note saved: "${preview}"`, 'success');
			} else {
				// Offline fallback: save locally without transcription
				const durationLabel = formatRecordingTime(recordingDuration);
				const content = `[Voice recording - ${durationLabel}]\n\n${text.trim() ? text.trim() : '(No transcription available)'}`;
				await createNoteOptimistic(content, title);
				pushToast('Voice note saved locally (offline)', 'success');
			}
			close();
		} catch {
			pushToast('Voice capture failed', 'danger');
		} finally {
			saving = false;
			uploadPhase = 'idle';
		}
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Enter' && (event.metaKey || event.ctrlKey)) {
			event.preventDefault();
			save();
		}
		if (event.key === 'Tab' && !event.shiftKey) {
			event.preventDefault();
			const types: Array<typeof captureType> = voiceEnabled ? ['task', 'note', 'link', 'voice'] : ['task', 'note', 'link'];
			const idx = types.indexOf(captureType);
			captureType = types[(idx + 1) % types.length];
		}
	}
</script>

<svelte:window on:keydown={handleGlobalKeydown} />

{#if open}
	<div class="fixed inset-0 z-50 flex items-start justify-center pt-[20vh]" role="presentation">
		<div
			class="absolute inset-0 bg-black/50"
			on:click={close}
			on:keydown={(e) => e.key === 'Escape' && close()}
			role="button"
			tabindex="-1"
			aria-label="Close quick capture"
		></div>
		<div
			class="relative z-10 w-full max-w-lg rounded-2xl border border-slate-700 bg-slate-900 p-4 shadow-2xl"
			role="dialog"
			aria-modal="true"
			aria-label="Quick capture"
		>
			<div class="relative flex items-center gap-2">
				<div class="flex rounded-lg border border-slate-700 bg-slate-800 p-0.5">
					<button
						class="rounded-md px-2.5 py-1 text-[10px] font-medium transition {captureType === 'task'
							? 'bg-violet-500/20 text-violet-300'
							: 'text-slate-400 hover:text-white'}"
						on:click={() => { captureType = 'task'; }}
					>
						Task
					</button>
					<button
						class="relative rounded-md px-2.5 py-1 text-[10px] font-medium transition {captureType === 'note'
							? 'bg-sky-500/20 text-sky-300'
							: 'text-slate-400 hover:text-white'}"
						on:click={() => { captureType = 'note'; showKindPicker = !showKindPicker && captureType === 'note'; }}
					>
						{NOTE_KINDS.find(k => k.value === noteKind)?.label ?? 'Note'}
						<span class="ml-0.5 text-[8px]">▼</span>
					</button>
					<button
						class="rounded-md px-2.5 py-1 text-[10px] font-medium transition {captureType === 'link'
							? 'bg-emerald-500/20 text-emerald-300'
							: 'text-slate-400 hover:text-white'}"
						on:click={() => { captureType = 'link'; }}
					>
						Link
					</button>
					{#if voiceEnabled}
						<button
							class="rounded-md px-2.5 py-1 text-[10px] font-medium transition {captureType === 'voice'
								? 'bg-rose-500/20 text-rose-300'
								: 'text-slate-400 hover:text-white'}"
							on:click={() => { captureType = 'voice'; }}
						>
							Voice
						</button>
					{/if}
				</div>
				<span class="text-[10px] text-slate-500">Tab to switch</span>

				{#if showKindPicker && captureType === 'note'}
					<div class="absolute left-0 top-full z-20 mt-1 w-56 rounded-xl border border-slate-700 bg-slate-800 p-1 shadow-xl">
						{#each NOTE_KINDS as kind (kind.value)}
							<button
								class="flex w-full items-start gap-2 rounded-lg px-2.5 py-2 text-left transition {noteKind === kind.value
									? 'bg-sky-500/15 text-sky-300'
									: 'text-slate-300 hover:bg-slate-700/60'}"
								on:click={() => { noteKind = kind.value; showKindPicker = false; }}
							>
								<div>
									<div class="text-xs font-medium">{kind.label}</div>
									<div class="text-[10px] text-slate-500">{kind.description}</div>
								</div>
							</button>
						{/each}
					</div>
				{/if}
				<span class="ml-auto text-[10px] text-slate-500">
					<kbd class="rounded border border-slate-700 bg-slate-800 px-1 py-0.5 text-[9px]">Cmd+Enter</kbd> to save
				</span>
			</div>

			{#if captureType === 'voice'}
				<div class="mt-3 flex flex-col items-center gap-3 rounded-xl border border-slate-700/60 bg-slate-800/40 p-4">
					{#if !audioBlob}
						<button
							class="flex h-16 w-16 items-center justify-center rounded-full transition {isRecording
								? 'bg-red-500 text-white animate-pulse'
								: 'border-2 border-rose-500/40 text-rose-400 hover:bg-rose-500/10'}"
							on:click={() => { isRecording ? stopRecording() : startRecording(); }}
						>
							{#if isRecording}
								<svg class="h-6 w-6" fill="currentColor" viewBox="0 0 24 24">
									<rect x="6" y="6" width="12" height="12" rx="2" />
								</svg>
							{:else}
								<svg class="h-6 w-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
									<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11a7 7 0 01-7 7m0 0a7 7 0 01-7-7m7 7v4m0 0H8m4 0h4M12 15a3 3 0 003-3V5a3 3 0 00-6 0v7a3 3 0 003 3z" />
								</svg>
							{/if}
						</button>
						<p class="text-xs text-slate-400">
							{isRecording ? formatRecordingTime(recordingDuration) : 'Tap to record'}
						</p>
					{:else}
						<audio src={audioUrl} controls class="w-full rounded-lg">
							<track kind="captions" />
						</audio>
						<button
							class="rounded-lg border border-slate-700 px-3 py-1 text-[10px] text-slate-300 hover:bg-slate-800"
							on:click={discardRecording}
						>
							Discard & re-record
						</button>
					{/if}
				</div>
				<input
					class="mt-2 w-full rounded-lg border border-slate-700 bg-slate-800/60 px-3 py-2 text-xs text-white placeholder-slate-500 outline-none focus:border-sky-500"
					placeholder="Add a title or description (optional)"
					bind:value={text}
					bind:this={inputEl}
					on:keydown={handleKeydown}
					disabled={saving}
				/>
			{:else}
				<input
					class="mt-3 w-full rounded-lg border border-slate-700 bg-slate-800/60 px-3 py-2.5 text-sm text-white placeholder-slate-500 outline-none focus:border-sky-500"
					placeholder={captureType === 'task'
						? 'buy milk tomorrow 5pm p2 #home'
						: captureType === 'link'
							? 'https://example.com/article'
							: 'Quick note...'}
					bind:value={text}
					bind:this={inputEl}
					on:keydown={handleKeydown}
					disabled={saving}
				/>
			{/if}

			{#if captureType === 'task'}
				<p class="mt-2 text-[10px] text-slate-500">
					Supports: p1-p5 priority, #tags, @assignee, time estimates (30m/1h), dates, recurrence
				</p>
			{:else if captureType === 'link'}
				<p class="mt-2 text-[10px] text-slate-500">
					Paste a URL to save it as a reference. Tagged as web-clip for easy triage.
				</p>
			{:else if captureType === 'voice'}
				<p class="mt-2 text-[10px] text-slate-500">
					Record audio, then optionally add a title. Audio is saved as a note.
				</p>
			{/if}

			<div class="mt-3 flex justify-end gap-2">
				<button
					class="rounded-lg border border-slate-700 px-3 py-1.5 text-xs text-slate-300 hover:bg-slate-800"
					on:click={close}
				>
					Cancel
				</button>
				<button
					class="rounded-lg bg-sky-500 px-3 py-1.5 text-xs font-semibold text-white hover:bg-sky-400 disabled:opacity-50"
					on:click={save}
					disabled={saving || (captureType === 'voice' ? !audioBlob : !text.trim())}
				>
					{#if saving && uploadPhase === 'uploading'}
						Uploading...
					{:else if saving && uploadPhase === 'transcribing'}
						Transcribing...
					{:else if saving}
						Saving...
					{:else}
						Capture
					{/if}
				</button>
			</div>
		</div>
	</div>
{/if}
