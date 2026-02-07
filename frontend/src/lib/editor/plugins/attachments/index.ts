/**
 * File attachments plugin.
 *
 * Features:
 * - Drag-drop files onto editor
 * - Paste images from clipboard
 * - Upload progress tracking
 * - Insert image/file links on success
 */

import type { EditorPlugin, PluginContext } from '../types';
import {
	uploadFile,
	isImageFile,
	formatFileSize,
	type UploadedFile,
	type UploadProgress
} from '$lib/api/files';

// State for tracking uploads
interface UploadState {
	isUploading: boolean;
	uploads: Map<string, { file: File; progress: UploadProgress }>;
}

let state: UploadState = {
	isUploading: false,
	uploads: new Map()
};

// Callback for UI updates
let onUploadStateChange: ((state: UploadState) => void) | null = null;

/**
 * Register a callback for upload state changes.
 */
export function onAttachmentStateChange(
	callback: (state: UploadState) => void
): () => void {
	onUploadStateChange = callback;
	return () => {
		onUploadStateChange = null;
	};
}

/**
 * Get the current upload state.
 */
export function getUploadState(): UploadState {
	return {
		isUploading: state.isUploading,
		uploads: new Map(state.uploads)
	};
}

/**
 * Update state and notify listeners.
 */
function updateState(updates: Partial<UploadState>): void {
	state = { ...state, ...updates };
	onUploadStateChange?.(state);
}

/**
 * Generate a unique upload ID.
 */
function generateUploadId(): string {
	return `upload-${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;
}

/**
 * Handle file upload and insertion.
 */
async function handleFileUpload(
	file: File,
	ctx: PluginContext,
	insertPosition?: Range
): Promise<void> {
	const uploadId = generateUploadId();

	// Track the upload
	state.uploads.set(uploadId, {
		file,
		progress: { loaded: 0, total: file.size, percentage: 0 }
	});
	updateState({ isUploading: true });

	// Emit upload start event
	ctx.emit('uploadStart', { uploadId, file });

	try {
		const uploaded = await uploadFile(file, (progress) => {
			const upload = state.uploads.get(uploadId);
			if (upload) {
				upload.progress = progress;
				updateState({ uploads: state.uploads });
				ctx.emit('uploadProgress', { uploadId, progress });
			}
		});

		// Remove from tracking
		state.uploads.delete(uploadId);
		updateState({ isUploading: state.uploads.size > 0 });

		// Insert the file link
		insertFileMarkdown(uploaded, file, ctx, insertPosition);

		// Emit upload complete event
		ctx.emit('uploadComplete', { uploadId, file: uploaded });
	} catch (error) {
		// Remove from tracking
		state.uploads.delete(uploadId);
		updateState({ isUploading: state.uploads.size > 0 });

		// Emit upload error event
		ctx.emit('uploadError', { uploadId, error: String(error) });
		console.error('File upload failed:', error);
	}
}

/**
 * Insert markdown for an uploaded file.
 */
function insertFileMarkdown(
	uploaded: UploadedFile,
	originalFile: File,
	ctx: PluginContext,
	insertPosition?: Range
): void {
	const sel = window.getSelection();
	if (!sel) return;

	// Restore position if provided
	if (insertPosition) {
		sel.removeAllRanges();
		sel.addRange(insertPosition);
	}

	let markdown: string;

	if (isImageFile(originalFile)) {
		// Insert as image
		markdown = `![${uploaded.filename}](${uploaded.url})`;
	} else {
		// Insert as link with file info
		const sizeStr = formatFileSize(uploaded.size);
		markdown = `[${uploaded.filename}](${uploaded.url}) (${sizeStr})`;
	}

	// Insert the markdown
	document.execCommand('insertText', false, markdown);

	ctx.triggerChange();
}

/**
 * Handle pasted files (images).
 */
function handlePastedFiles(
	items: DataTransferItemList,
	ctx: PluginContext
): boolean {
	let handled = false;

	for (let i = 0; i < items.length; i++) {
		const item = items[i];
		if (item.kind === 'file') {
			const file = item.getAsFile();
			if (file) {
				// Save current position
				const sel = window.getSelection();
				const position = sel?.rangeCount ? sel.getRangeAt(0).cloneRange() : undefined;

				handleFileUpload(file, ctx, position);
				handled = true;
			}
		}
	}

	return handled;
}

/**
 * Handle dropped files.
 */
function handleDroppedFiles(
	files: FileList,
	ctx: PluginContext,
	dropPosition?: Range
): boolean {
	if (files.length === 0) return false;

	for (let i = 0; i < files.length; i++) {
		const file = files[i];
		handleFileUpload(file, ctx, dropPosition);
	}

	return true;
}

/**
 * Get the caret position from a drop event.
 */
function getCaretPositionFromPoint(x: number, y: number): Range | undefined {
	// Use caretPositionFromPoint if available (Firefox)
	if ('caretPositionFromPoint' in document) {
		const pos = (document as Document & {
			caretPositionFromPoint(x: number, y: number): { offsetNode: Node; offset: number } | null;
		}).caretPositionFromPoint(x, y);
		if (pos) {
			const range = document.createRange();
			range.setStart(pos.offsetNode, pos.offset);
			range.collapse(true);
			return range;
		}
	}

	// Use caretRangeFromPoint (Chrome, Safari)
	if ('caretRangeFromPoint' in document) {
		const range = document.caretRangeFromPoint(x, y);
		if (range) {
			return range;
		}
	}

	return undefined;
}

/**
 * Attachments plugin.
 */
export const attachmentsPlugin: EditorPlugin = {
	id: 'attachments',
	name: 'File Attachments',
	description: 'Drag-drop and paste file uploads',
	version: '1.0.0',

	onInit(ctx: PluginContext) {
		// Reset state
		state = {
			isUploading: false,
			uploads: new Map()
		};
	},

	onDestroy() {
		onUploadStateChange = null;
	},

	onPaste(event: ClipboardEvent, ctx: PluginContext): boolean | void {
		const items = event.clipboardData?.items;
		if (!items) return;

		// Check for files in clipboard
		let hasFiles = false;
		for (let i = 0; i < items.length; i++) {
			if (items[i].kind === 'file') {
				hasFiles = true;
				break;
			}
		}

		if (hasFiles) {
			event.preventDefault();
			handlePastedFiles(items, ctx);
			return true;
		}
	},

	onDrop(event: DragEvent, ctx: PluginContext): boolean | void {
		const files = event.dataTransfer?.files;
		if (!files || files.length === 0) return;

		event.preventDefault();

		// Get drop position
		const dropPosition = getCaretPositionFromPoint(event.clientX, event.clientY);

		handleDroppedFiles(files, ctx, dropPosition);
		return true;
	},

	slashItems: [
		{
			id: 'upload-image',
			label: 'Upload Image',
			description: 'Upload an image file',
			icon: '📷',
			keywords: ['image', 'photo', 'picture', 'upload', 'attachment'],
			group: 'media',
			action: (ctx) => {
				// Create a hidden file input
				const input = document.createElement('input');
				input.type = 'file';
				input.accept = 'image/*';
				input.style.display = 'none';

				input.addEventListener('change', () => {
					const file = input.files?.[0];
					if (file) {
						const sel = window.getSelection();
						const position = sel?.rangeCount ? sel.getRangeAt(0).cloneRange() : undefined;
						handleFileUpload(file, ctx, position);
					}
					input.remove();
				});

				document.body.appendChild(input);
				input.click();
			}
		},
		{
			id: 'upload-file',
			label: 'Upload File',
			description: 'Upload any file',
			icon: '📎',
			keywords: ['file', 'attachment', 'upload', 'document'],
			group: 'media',
			action: (ctx) => {
				// Create a hidden file input
				const input = document.createElement('input');
				input.type = 'file';
				input.style.display = 'none';

				input.addEventListener('change', () => {
					const file = input.files?.[0];
					if (file) {
						const sel = window.getSelection();
						const position = sel?.rangeCount ? sel.getRangeAt(0).cloneRange() : undefined;
						handleFileUpload(file, ctx, position);
					}
					input.remove();
				});

				document.body.appendChild(input);
				input.click();
			}
		}
	],

	commands: {
		uploadFile: (ctx: PluginContext, ...args: unknown[]) => {
			const [file] = args as [File | undefined];
			if (file) {
				handleFileUpload(file, ctx);
			}
		},
		uploadFiles: (ctx: PluginContext, ...args: unknown[]) => {
			const [files] = args as [File[] | undefined];
			if (files && files.length > 0) {
				for (const file of files) {
					handleFileUpload(file, ctx);
				}
			}
		}
	}
};

export default attachmentsPlugin;
