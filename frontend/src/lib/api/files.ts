/**
 * File upload API client.
 */

import { API_BASE_URL, ApiError } from './client';

export interface UploadedFile {
	file_id: string;
	url: string;
	filename: string;
	mime_type: string;
	size: number;
}

export interface UploadProgress {
	loaded: number;
	total: number;
	percentage: number;
}

export interface NodeAttachment {
	attachment_id: string;
	file_name: string;
	content_type?: string | null;
	size_bytes: number;
	uploaded_at?: string | null;
	extraction_status?: string | null;
	extracted_chars?: number | null;
	search_chunk_count?: number | null;
	search_preview?: string | null;
	download_url: string;
}

/**
 * Upload a file to the server.
 *
 * @param file - The file to upload
 * @param onProgress - Optional progress callback
 * @returns The uploaded file metadata
 */
export async function uploadFile(
	file: File,
	onProgress?: (progress: UploadProgress) => void
): Promise<UploadedFile> {
	return new Promise((resolve, reject) => {
		const xhr = new XMLHttpRequest();
		const formData = new FormData();
		formData.append('file', file);

		xhr.open('POST', `${API_BASE_URL}/api/v1/files/upload`);

		// Track upload progress
		if (onProgress) {
			xhr.upload.addEventListener('progress', (event) => {
				if (event.lengthComputable) {
					onProgress({
						loaded: event.loaded,
						total: event.total,
						percentage: Math.round((event.loaded / event.total) * 100)
					});
				}
			});
		}

		xhr.addEventListener('load', () => {
			if (xhr.status >= 200 && xhr.status < 300) {
				try {
					const response = JSON.parse(xhr.responseText) as UploadedFile;
					resolve(response);
				} catch {
					reject(new ApiError('Invalid response from server', xhr.status, xhr.responseText));
				}
			} else {
				let body: unknown = xhr.responseText;
				try {
					body = JSON.parse(xhr.responseText);
				} catch {
					// Keep as text
				}
				reject(new ApiError(`Upload failed (${xhr.status})`, xhr.status, body));
			}
		});

		xhr.addEventListener('error', () => {
			reject(new ApiError('Network error during upload', 0));
		});

		xhr.addEventListener('abort', () => {
			reject(new ApiError('Upload aborted', 0));
		});

		xhr.send(formData);
	});
}

/**
 * Upload multiple files.
 *
 * @param files - The files to upload
 * @param onProgress - Optional progress callback (called for each file)
 * @returns Array of uploaded file metadata
 */
export async function uploadFiles(
	files: File[],
	onProgress?: (fileIndex: number, progress: UploadProgress) => void
): Promise<UploadedFile[]> {
	const results: UploadedFile[] = [];

	for (let i = 0; i < files.length; i++) {
		const file = files[i];
		const uploaded = await uploadFile(file, (progress) => {
			onProgress?.(i, progress);
		});
		results.push(uploaded);
	}

	return results;
}

export async function uploadNodeAttachment(
	nodeId: string,
	file: File,
	onProgress?: (progress: UploadProgress) => void
): Promise<{
	attachment_id: string;
	node_id: string;
	file_name: string;
	content_type?: string | null;
	size_bytes: number;
	stored_path: string;
	extraction_status: string;
	extracted_chars: number;
}> {
	return new Promise((resolve, reject) => {
		const xhr = new XMLHttpRequest();
		const formData = new FormData();
		formData.append('node_id', nodeId);
		formData.append('file', file);
		xhr.open('POST', `${API_BASE_URL}/api/v1/files/upload`);

		if (onProgress) {
			xhr.upload.addEventListener('progress', (event) => {
				if (!event.lengthComputable) return;
				onProgress({
					loaded: event.loaded,
					total: event.total,
					percentage: Math.round((event.loaded / event.total) * 100)
				});
			});
		}

		xhr.addEventListener('load', () => {
			const text = xhr.responseText || '';
			if (xhr.status >= 200 && xhr.status < 300) {
				try {
					resolve(JSON.parse(text));
				} catch {
					reject(new ApiError('Invalid response from server', xhr.status, text));
				}
				return;
			}
			try {
				reject(new ApiError(`Upload failed (${xhr.status})`, xhr.status, JSON.parse(text)));
			} catch {
				reject(new ApiError(`Upload failed (${xhr.status})`, xhr.status, text));
			}
		});

		xhr.addEventListener('error', () => reject(new ApiError('Network error during upload', 0)));
		xhr.addEventListener('abort', () => reject(new ApiError('Upload aborted', 0)));
		xhr.send(formData);
	});
}

export async function listNodeAttachments(nodeId: string): Promise<NodeAttachment[]> {
	const res = await fetch(`${API_BASE_URL}/api/v1/files/${encodeURIComponent(nodeId)}`);
	if (!res.ok) {
		const raw = await res.text();
		let body: unknown = raw;
		try {
			body = JSON.parse(raw);
		} catch {
			// Keep raw text body
		}
		throw new ApiError(`Request failed (${res.status})`, res.status, body);
	}
	return (await res.json()) as NodeAttachment[];
}

export async function deleteNodeAttachment(
	nodeId: string,
	attachmentId: string
): Promise<{ attachment_id: string; file_deleted: boolean; remaining_attachments: number }> {
	const res = await fetch(
		`${API_BASE_URL}/api/v1/files/${encodeURIComponent(nodeId)}/${encodeURIComponent(attachmentId)}`,
		{
			method: 'DELETE'
		}
	);
	if (!res.ok) {
		const raw = await res.text();
		let body: unknown = raw;
		try {
			body = JSON.parse(raw);
		} catch {
			// Keep raw text body
		}
		throw new ApiError(`Request failed (${res.status})`, res.status, body);
	}
	return (await res.json()) as {
		attachment_id: string;
		file_deleted: boolean;
		remaining_attachments: number;
	};
}

export function attachmentDownloadUrl(nodeId: string, attachmentId: string): string {
	return `${API_BASE_URL}/api/v1/files/${encodeURIComponent(nodeId)}/${encodeURIComponent(attachmentId)}`;
}

export function attachmentInlineUrl(nodeId: string, attachmentId: string): string {
	return `${attachmentDownloadUrl(nodeId, attachmentId)}?inline=true`;
}

export interface AttachmentChunkItem {
	index: number;
	text: string;
	char_count: number;
}

export interface AttachmentChunkListResponse {
	node_id: string;
	attachment_id: string;
	extraction_status?: string | null;
	extracted_chars?: number | null;
	total_chunks: number;
	offset: number;
	limit: number;
	returned_chunks: number;
	chunks: AttachmentChunkItem[];
}

export async function getAttachmentChunks(
	nodeId: string,
	attachmentId: string,
	options: { limit?: number; offset?: number } = {}
): Promise<AttachmentChunkListResponse> {
	const params = new URLSearchParams();
	if (options.limit !== undefined) {
		params.set('limit', String(options.limit));
	}
	if (options.offset !== undefined) {
		params.set('offset', String(options.offset));
	}
	const suffix = params.toString();
	const res = await fetch(
		`${API_BASE_URL}/api/v1/files/${encodeURIComponent(nodeId)}/${encodeURIComponent(attachmentId)}/chunks${
			suffix ? `?${suffix}` : ''
		}`
	);
	if (!res.ok) {
		const raw = await res.text();
		let body: unknown = raw;
		try {
			body = JSON.parse(raw);
		} catch {
			// Keep raw text body
		}
		throw new ApiError(`Request failed (${res.status})`, res.status, body);
	}
	return (await res.json()) as AttachmentChunkListResponse;
}

/**
 * Check if a file type is an image.
 */
export function isImageFile(file: File): boolean {
	return file.type.startsWith('image/');
}

/**
 * Check if a file type is a video.
 */
export function isVideoFile(file: File): boolean {
	return file.type.startsWith('video/');
}

/**
 * Check if a file type is audio.
 */
export function isAudioFile(file: File): boolean {
	return file.type.startsWith('audio/');
}

/**
 * Check if a file type is a document (PDF, Word, etc.)
 */
export function isDocumentFile(file: File): boolean {
	const docTypes = [
		'application/pdf',
		'application/msword',
		'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
		'application/vnd.ms-excel',
		'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet',
		'application/vnd.ms-powerpoint',
		'application/vnd.openxmlformats-officedocument.presentationml.presentation',
		'text/plain',
		'text/markdown',
		'text/csv'
	];
	return docTypes.includes(file.type);
}

/**
 * Format file size for display.
 */
export function formatFileSize(bytes: number): string {
	if (bytes === 0) return '0 B';
	const k = 1024;
	const sizes = ['B', 'KB', 'MB', 'GB'];
	const i = Math.floor(Math.log(bytes) / Math.log(k));
	return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
}
