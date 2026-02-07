<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { uploadVoiceNote, isSupportedAudioFormat, formatDuration } from '$lib/api/voice';
	import type { VoiceUploadProgress, VoiceUploadResponse } from '$lib/api/voice';
	import { pushToast } from '$lib/stores/toast';

	export let namespace: string | undefined = undefined;
	export let tags: string[] = [];
	export let compact = false;

	const dispatch = createEventDispatcher<{
		success: VoiceUploadResponse;
		error: Error;
	}>();

	// Recording state
	let isRecording = false;
	let isPaused = false;
	let recorder: MediaRecorder | null = null;
	let audioChunks: Blob[] = [];
	let recordingDuration = 0;
	let recordingTimer: ReturnType<typeof setInterval> | null = null;

	// Preview state
	let audioBlob: Blob | null = null;
	let audioUrl: string | null = null;

	// Upload state
	let isUploading = false;
	let uploadProgress: VoiceUploadProgress | null = null;
	let title = '';

	async function startRecording() {
		try {
			const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
			recorder = new MediaRecorder(stream, { mimeType: 'audio/webm' });
			audioChunks = [];
			recordingDuration = 0;

			recorder.ondataavailable = (e) => {
				if (e.data.size > 0) audioChunks.push(e.data);
			};

			recorder.onstop = () => {
				audioBlob = new Blob(audioChunks, { type: 'audio/webm' });
				audioUrl = URL.createObjectURL(audioBlob);
				stream.getTracks().forEach((t) => t.stop());
				if (recordingTimer) {
					clearInterval(recordingTimer);
					recordingTimer = null;
				}
			};

			recorder.start(1000); // Collect data every second
			isRecording = true;
			isPaused = false;
			recordingTimer = setInterval(() => {
				if (!isPaused) recordingDuration++;
			}, 1000);
		} catch (err) {
			pushToast('Microphone access denied', 'danger');
			dispatch('error', err instanceof Error ? err : new Error(String(err)));
		}
	}

	function pauseRecording() {
		if (recorder && recorder.state === 'recording') {
			recorder.pause();
			isPaused = true;
		}
	}

	function resumeRecording() {
		if (recorder && recorder.state === 'paused') {
			recorder.resume();
			isPaused = false;
		}
	}

	function stopRecording() {
		if (recorder && (recorder.state === 'recording' || recorder.state === 'paused')) {
			recorder.stop();
			isRecording = false;
			isPaused = false;
		}
	}

	function discardRecording() {
		if (audioUrl) URL.revokeObjectURL(audioUrl);
		audioBlob = null;
		audioUrl = null;
		recordingDuration = 0;
		title = '';
	}

	async function uploadRecording() {
		if (!audioBlob) return;

		isUploading = true;
		uploadProgress = { phase: 'uploading' };

		try {
			const file = new File([audioBlob], 'voice-note.webm', { type: 'audio/webm' });
			const response = await uploadVoiceNote(file, {
				title: title || undefined,
				tags,
				namespace,
				onProgress: (p) => {
					uploadProgress = p;
				}
			});

			pushToast('Voice note created', 'success');
			dispatch('success', response);
			discardRecording();
		} catch (err) {
			pushToast('Failed to upload voice note', 'danger');
			dispatch('error', err instanceof Error ? err : new Error(String(err)));
		} finally {
			isUploading = false;
			uploadProgress = null;
		}
	}

	function handleFileDrop(event: DragEvent) {
		event.preventDefault();
		const files = event.dataTransfer?.files;
		if (files?.length) {
			handleFileSelect(files[0]);
		}
	}

	function handleFileSelect(file: File) {
		if (!isSupportedAudioFormat(file)) {
			pushToast('Unsupported audio format', 'danger');
			return;
		}
		audioBlob = file;
		audioUrl = URL.createObjectURL(file);
		title = file.name.replace(/\.[^.]+$/, '');
	}
</script>

<svelte:window
	on:dragover|preventDefault
	on:drop|preventDefault={handleFileDrop}
/>

<div class="voice-recorder" class:compact>
	{#if !audioBlob}
		<!-- Recording Controls -->
		<div class="recording-controls">
			{#if isRecording}
				<div class="recording-indicator">
					<span class="pulse" class:paused={isPaused}></span>
					<span class="duration">{formatDuration(recordingDuration)}</span>
				</div>
				<div class="control-buttons">
					{#if isPaused}
						<button class="btn-icon" on:click={resumeRecording} title="Resume">
							<svg viewBox="0 0 24 24" width="20" height="20" fill="currentColor">
								<path d="M8 5v14l11-7z"/>
							</svg>
						</button>
					{:else}
						<button class="btn-icon" on:click={pauseRecording} title="Pause">
							<svg viewBox="0 0 24 24" width="20" height="20" fill="currentColor">
								<path d="M6 19h4V5H6v14zm8-14v14h4V5h-4z"/>
							</svg>
						</button>
					{/if}
					<button class="btn-stop" on:click={stopRecording} title="Stop">
						<svg viewBox="0 0 24 24" width="20" height="20" fill="currentColor">
							<path d="M6 6h12v12H6z"/>
						</svg>
					</button>
				</div>
			{:else}
				<button class="btn-record" on:click={startRecording}>
					<svg viewBox="0 0 24 24" width="24" height="24" fill="currentColor">
						<circle cx="12" cy="12" r="6"/>
					</svg>
					{#if !compact}
						<span>Start Recording</span>
					{/if}
				</button>
				{#if !compact}
					<span class="or-text">or drop audio file</span>
				{/if}
			{/if}
		</div>
	{:else}
		<!-- Preview and Upload -->
		<div class="preview-section">
			<audio src={audioUrl} controls class="audio-preview"></audio>
			<input
				type="text"
				bind:value={title}
				placeholder="Title (optional)"
				class="title-input"
			/>
			{#if uploadProgress}
				<div class="upload-progress">
					<span class="phase">{uploadProgress.phase === 'uploading' ? 'Uploading...' : 'Transcribing...'}</span>
					{#if uploadProgress.percentage !== undefined}
						<div class="progress-bar">
							<div class="progress-fill" style="width: {uploadProgress.percentage}%"></div>
						</div>
					{/if}
				</div>
			{/if}
			<div class="preview-actions">
				<button class="btn-discard" on:click={discardRecording} disabled={isUploading}>
					Discard
				</button>
				<button class="btn-upload" on:click={uploadRecording} disabled={isUploading}>
					{isUploading ? 'Processing...' : 'Create Note'}
				</button>
			</div>
		</div>
	{/if}
</div>

<style>
	.voice-recorder {
		display: flex;
		flex-direction: column;
		gap: 1rem;
		padding: 1rem;
		border: 1px solid var(--border-color, #e0e0e0);
		border-radius: 8px;
		background: var(--surface-color, #fff);
	}

	.voice-recorder.compact {
		padding: 0.5rem;
	}

	.recording-controls {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.75rem;
	}

	.recording-indicator {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.pulse {
		width: 12px;
		height: 12px;
		background: #ef4444;
		border-radius: 50%;
		animation: pulse 1s infinite;
	}

	.pulse.paused {
		animation: none;
		opacity: 0.5;
	}

	@keyframes pulse {
		0%, 100% { opacity: 1; transform: scale(1); }
		50% { opacity: 0.5; transform: scale(1.1); }
	}

	.duration {
		font-family: monospace;
		font-size: 1.25rem;
	}

	.control-buttons {
		display: flex;
		gap: 0.5rem;
	}

	.btn-icon, .btn-stop, .btn-record {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.5rem;
		padding: 0.5rem 1rem;
		border: none;
		border-radius: 8px;
		cursor: pointer;
		transition: all 0.15s;
	}

	.btn-icon {
		background: var(--btn-secondary-bg, #f3f4f6);
		color: var(--text-color, #374151);
	}

	.btn-stop {
		background: #ef4444;
		color: white;
	}

	.btn-record {
		background: #ef4444;
		color: white;
		font-size: 0.875rem;
	}

	.btn-record:hover, .btn-stop:hover {
		background: #dc2626;
	}

	.or-text {
		color: var(--text-muted, #6b7280);
		font-size: 0.875rem;
	}

	.preview-section {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.audio-preview {
		width: 100%;
	}

	.title-input {
		padding: 0.5rem;
		border: 1px solid var(--border-color, #e0e0e0);
		border-radius: 6px;
		font-size: 0.875rem;
	}

	.upload-progress {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}

	.phase {
		font-size: 0.75rem;
		color: var(--text-muted, #6b7280);
	}

	.progress-bar {
		height: 4px;
		background: var(--border-color, #e0e0e0);
		border-radius: 2px;
		overflow: hidden;
	}

	.progress-fill {
		height: 100%;
		background: var(--primary-color, #3b82f6);
		transition: width 0.2s;
	}

	.preview-actions {
		display: flex;
		gap: 0.5rem;
		justify-content: flex-end;
	}

	.btn-discard, .btn-upload {
		padding: 0.5rem 1rem;
		border: none;
		border-radius: 6px;
		font-size: 0.875rem;
		cursor: pointer;
	}

	.btn-discard {
		background: transparent;
		color: var(--text-muted, #6b7280);
	}

	.btn-upload {
		background: var(--primary-color, #3b82f6);
		color: white;
	}

	.btn-upload:disabled, .btn-discard:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
</style>
