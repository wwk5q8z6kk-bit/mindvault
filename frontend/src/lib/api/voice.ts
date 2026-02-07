/**
 * Voice note API - upload and transcription.
 * Backend uses Whisper (local) or OpenAI API for transcription.
 */

import { API_BASE_URL, ApiError } from './client';
import type { KnowledgeNode } from './types';

export interface VoiceUploadResponse {
	node_id: string;
	transcript: string;
	duration_seconds: number;
	language?: string;
	provider: 'whisper' | 'openai';
}

export interface VoiceUploadProgress {
	phase: 'uploading' | 'transcribing';
	loaded?: number;
	total?: number;
	percentage?: number;
}

export interface VoiceUploadOptions {
	/** Optional title for the created note */
	title?: string;
	/** Tags to apply to the created note */
	tags?: string[];
	/** Namespace for organization */
	namespace?: string;
	/** Language hint for transcription (e.g., 'en', 'es', 'zh') */
	language?: string;
	/** Progress callback */
	onProgress?: (progress: VoiceUploadProgress) => void;
}

/**
 * Upload an audio file for transcription.
 * Creates a new note with the transcript as content.
 */
export async function uploadVoiceNote(
	file: File,
	options: VoiceUploadOptions = {}
): Promise<VoiceUploadResponse> {
	const { title, tags, namespace, language, onProgress } = options;

	return new Promise((resolve, reject) => {
		const xhr = new XMLHttpRequest();
		const formData = new FormData();

		formData.append('file', file);
		if (title) formData.append('title', title);
		if (tags?.length) formData.append('tags', JSON.stringify(tags));
		if (namespace) formData.append('namespace', namespace);
		if (language) formData.append('language', language);

		xhr.upload.addEventListener('progress', (e) => {
			if (e.lengthComputable && onProgress) {
				onProgress({
					phase: 'uploading',
					loaded: e.loaded,
					total: e.total,
					percentage: Math.round((e.loaded / e.total) * 100)
				});
			}
		});

		xhr.addEventListener('load', () => {
			if (xhr.status >= 200 && xhr.status < 300) {
				try {
					const response = JSON.parse(xhr.responseText) as VoiceUploadResponse;
					resolve(response);
				} catch {
					reject(new ApiError('Invalid response format', xhr.status, xhr.responseText));
				}
			} else {
				let body: unknown;
				try {
					body = JSON.parse(xhr.responseText);
				} catch {
					body = xhr.responseText;
				}
				reject(new ApiError(`Voice upload failed (${xhr.status})`, xhr.status, body));
			}
		});

		xhr.addEventListener('error', () => {
			reject(new ApiError('Voice upload network error', 0));
		});

		xhr.addEventListener('abort', () => {
			reject(new ApiError('Voice upload aborted', 0));
		});

		// Signal transcription phase after upload completes
		xhr.addEventListener('loadend', () => {
			if (xhr.status >= 200 && xhr.status < 300 && onProgress) {
				onProgress({ phase: 'transcribing' });
			}
		});

		xhr.open('POST', `${API_BASE_URL}/api/v1/voice/upload`);
		xhr.send(formData);
	});
}

/**
 * Get the voice note node after upload.
 */
export async function getVoiceNote(nodeId: string): Promise<KnowledgeNode> {
	const response = await fetch(`${API_BASE_URL}/api/v1/nodes/${nodeId}`);
	if (!response.ok) {
		throw new ApiError(`Failed to get voice note (${response.status})`, response.status);
	}
	return (await response.json()) as KnowledgeNode;
}

/**
 * Check if a file is a supported audio format.
 */
export function isSupportedAudioFormat(file: File): boolean {
	const supportedTypes = [
		'audio/wav',
		'audio/wave',
		'audio/x-wav',
		'audio/mp3',
		'audio/mpeg',
		'audio/mp4',
		'audio/m4a',
		'audio/x-m4a',
		'audio/ogg',
		'audio/webm',
		'audio/flac'
	];
	return supportedTypes.includes(file.type) || /\.(wav|mp3|m4a|ogg|webm|flac)$/i.test(file.name);
}

/**
 * Format audio duration for display.
 */
export function formatDuration(seconds: number): string {
	const mins = Math.floor(seconds / 60);
	const secs = Math.floor(seconds % 60);
	return `${mins}:${secs.toString().padStart(2, '0')}`;
}
