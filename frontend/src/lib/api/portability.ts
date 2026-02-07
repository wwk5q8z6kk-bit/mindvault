/**
 * Data portability API - export and import vaults.
 */

import { API_BASE_URL, ApiError } from './client';

export interface ExportOptions {
	/** Export format */
	format?: 'json' | 'markdown' | 'archive';
	/** Include specific namespaces only */
	namespaces?: string[];
	/** Include specific kinds only */
	kinds?: string[];
	/** Include node relationships */
	include_relationships?: boolean;
	/** Include attachments (for archive format) */
	include_attachments?: boolean;
	/** Include version history */
	include_versions?: boolean;
	/** Date range filter */
	created_after?: string;
	created_before?: string;
}

export interface ExportResponse {
	export_id: string;
	status: 'pending' | 'processing' | 'completed' | 'failed';
	download_url?: string;
	node_count: number;
	relationship_count: number;
	attachment_count: number;
	size_bytes: number;
	created_at: string;
	completed_at?: string;
	error?: string;
}

export interface ImportOptions {
	/** How to handle ID conflicts */
	conflict_strategy?: 'skip' | 'replace' | 'merge' | 'duplicate';
	/** Target namespace for imported nodes */
	target_namespace?: string;
	/** Whether to import relationships */
	import_relationships?: boolean;
	/** Whether to import attachments */
	import_attachments?: boolean;
	/** Tags to add to all imported nodes */
	add_tags?: string[];
}

export interface ImportResponse {
	import_id: string;
	status: 'pending' | 'processing' | 'completed' | 'failed';
	nodes_imported: number;
	nodes_skipped: number;
	nodes_failed: number;
	relationships_imported: number;
	attachments_imported: number;
	errors: ImportError[];
	created_at: string;
	completed_at?: string;
}

export interface ImportError {
	node_id?: string;
	error: string;
	details?: string;
}

export interface ImportProgress {
	phase: 'uploading' | 'parsing' | 'importing' | 'indexing';
	current: number;
	total: number;
	percentage: number;
}

/**
 * Start an export job.
 */
export async function startExport(options: ExportOptions = {}): Promise<ExportResponse> {
	const response = await fetch(`${API_BASE_URL}/api/v1/export`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify(options)
	});

	if (!response.ok) {
		let body: unknown;
		try {
			body = await response.json();
		} catch {
			body = await response.text();
		}
		throw new ApiError(`Export failed (${response.status})`, response.status, body);
	}

	return (await response.json()) as ExportResponse;
}

/**
 * Check export job status.
 */
export async function getExportStatus(exportId: string): Promise<ExportResponse> {
	const response = await fetch(`${API_BASE_URL}/api/v1/export/${exportId}`);

	if (!response.ok) {
		throw new ApiError(`Failed to get export status (${response.status})`, response.status);
	}

	return (await response.json()) as ExportResponse;
}

/**
 * Download an export once completed.
 */
export async function downloadExport(
	exportId: string,
	filename?: string
): Promise<void> {
	const status = await getExportStatus(exportId);

	if (status.status !== 'completed' || !status.download_url) {
		throw new ApiError('Export not ready for download', 400);
	}

	const response = await fetch(`${API_BASE_URL}${status.download_url}`);
	if (!response.ok) {
		throw new ApiError(`Download failed (${response.status})`, response.status);
	}

	const blob = await response.blob();
	const url = URL.createObjectURL(blob);

	const a = document.createElement('a');
	a.href = url;
	a.download = filename || `mindvault-export-${exportId}.zip`;
	document.body.appendChild(a);
	a.click();
	document.body.removeChild(a);
	URL.revokeObjectURL(url);
}

/**
 * Import data from a file.
 */
export async function importData(
	file: File,
	options: ImportOptions = {},
	onProgress?: (progress: ImportProgress) => void
): Promise<ImportResponse> {
	return new Promise((resolve, reject) => {
		const xhr = new XMLHttpRequest();
		const formData = new FormData();

		formData.append('file', file);
		if (options.conflict_strategy)
			formData.append('conflict_strategy', options.conflict_strategy);
		if (options.target_namespace)
			formData.append('target_namespace', options.target_namespace);
		if (options.import_relationships !== undefined)
			formData.append('import_relationships', String(options.import_relationships));
		if (options.import_attachments !== undefined)
			formData.append('import_attachments', String(options.import_attachments));
		if (options.add_tags?.length)
			formData.append('add_tags', JSON.stringify(options.add_tags));

		xhr.upload.addEventListener('progress', (e) => {
			if (e.lengthComputable && onProgress) {
				onProgress({
					phase: 'uploading',
					current: e.loaded,
					total: e.total,
					percentage: Math.round((e.loaded / e.total) * 100)
				});
			}
		});

		xhr.addEventListener('load', () => {
			if (xhr.status >= 200 && xhr.status < 300) {
				try {
					const response = JSON.parse(xhr.responseText) as ImportResponse;
					resolve(response);
				} catch {
					reject(new ApiError('Invalid import response', xhr.status, xhr.responseText));
				}
			} else {
				let body: unknown;
				try {
					body = JSON.parse(xhr.responseText);
				} catch {
					body = xhr.responseText;
				}
				reject(new ApiError(`Import failed (${xhr.status})`, xhr.status, body));
			}
		});

		xhr.addEventListener('error', () => {
			reject(new ApiError('Import network error', 0));
		});

		xhr.addEventListener('abort', () => {
			reject(new ApiError('Import aborted', 0));
		});

		xhr.open('POST', `${API_BASE_URL}/api/v1/import`);
		xhr.send(formData);
	});
}

/**
 * Check import job status.
 */
export async function getImportStatus(importId: string): Promise<ImportResponse> {
	const response = await fetch(`${API_BASE_URL}/api/v1/import/${importId}`);

	if (!response.ok) {
		throw new ApiError(`Failed to get import status (${response.status})`, response.status);
	}

	return (await response.json()) as ImportResponse;
}

/**
 * Poll for export/import completion.
 */
export async function pollJobCompletion<T extends { status: string }>(
	getStatus: () => Promise<T>,
	intervalMs = 1000,
	timeoutMs = 300000
): Promise<T> {
	const startTime = Date.now();

	while (Date.now() - startTime < timeoutMs) {
		const status = await getStatus();

		if (status.status === 'completed' || status.status === 'failed') {
			return status;
		}

		await new Promise((resolve) => setTimeout(resolve, intervalMs));
	}

	throw new ApiError('Job timed out', 408);
}
