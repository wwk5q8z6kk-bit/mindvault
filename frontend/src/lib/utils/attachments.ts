import type { AttachmentChunkItem, NodeAttachment } from '$lib/api/files';

export type AttachmentKind = {
	isImage: boolean;
	isPdf: boolean;
	isAudio: boolean;
	isVideo: boolean;
};

export function classifyAttachment(
	attachment: Pick<NodeAttachment, 'file_name' | 'content_type'>
): AttachmentKind {
	const contentType = attachment.content_type?.toLowerCase() ?? '';
	const fileName = attachment.file_name.toLowerCase();
	const ext = fileName.includes('.') ? fileName.split('.').pop() ?? '' : '';
	return {
		isImage:
			contentType.startsWith('image/') || ['png', 'jpg', 'jpeg', 'gif', 'webp', 'bmp'].includes(ext),
		isPdf: contentType === 'application/pdf' || ext === 'pdf',
		isAudio:
			contentType.startsWith('audio/') ||
			['mp3', 'wav', 'm4a', 'webm', 'ogg', 'flac', 'aac', 'wma'].includes(ext),
		isVideo:
			contentType.startsWith('video/') || ['mp4', 'webm', 'mov', 'mkv'].includes(ext)
	};
}

export function formatExtractionStatus(status?: string | null, extracted?: number | null): string {
	if (!status) return 'pending';
	const normalized = status.replace(/_/g, ' ');
	if (extracted && extracted > 0) {
		return `${normalized} · ${extracted} chars`;
	}
	return normalized;
}

export function extractionBadge(status?: string | null): { label: string; tone: string } {
	const normalized = status?.toLowerCase() ?? 'pending';
	if (normalized === 'transcribed') return { label: 'Transcribed', tone: 'emerald' };
	if (normalized.startsWith('indexed')) return { label: 'Indexed', tone: 'emerald' };
	if (normalized === 'tool_missing') return { label: 'Tool missing', tone: 'amber' };
	if (normalized === 'extraction_failed') return { label: 'Extraction failed', tone: 'red' };
	if (normalized === 'unsupported') return { label: 'Unsupported', tone: 'slate' };
	if (normalized === 'empty') return { label: 'Empty', tone: 'slate' };
	return { label: 'Pending', tone: 'slate' };
}

export function badgeClass(tone: string): string {
	switch (tone) {
		case 'emerald':
			return 'border-emerald-500/30 bg-emerald-500/10 text-emerald-300';
		case 'amber':
			return 'border-amber-500/30 bg-amber-500/10 text-amber-300';
		case 'red':
			return 'border-red-500/30 bg-red-500/10 text-red-300';
		default:
			return 'border-slate-600/40 bg-slate-700/20 text-slate-300';
	}
}

export function filterAttachmentChunks(
	chunks: AttachmentChunkItem[],
	query: string
): AttachmentChunkItem[] {
	const needle = query.trim().toLowerCase();
	if (!needle) return chunks;
	return chunks.filter((item) => item.text.toLowerCase().includes(needle));
}

export function buildAttachmentEmbedMarkdown(
	markdown: string,
	attachment: NodeAttachment,
	inlineUrl: string
): string {
	const kind = classifyAttachment(attachment);
	const name = attachment.file_name || 'attachment';
	let snippet = `[${name}](${inlineUrl})`;
	if (kind.isImage) {
		snippet = `![${name}](${inlineUrl})`;
	} else if (kind.isAudio) {
		snippet = `<audio controls src="${inlineUrl}"></audio>\n\n[Audio: ${name}](${inlineUrl})`;
	} else if (kind.isVideo) {
		snippet = `<video controls src="${inlineUrl}"></video>\n\n[Video: ${name}](${inlineUrl})`;
	} else if (kind.isPdf) {
		snippet = `[PDF: ${name}](${inlineUrl})`;
	}
	const prefix = markdown.trim();
	const header = prefix ? `${prefix}\n\n` : '';
	return `${header}${snippet}\n`;
}
