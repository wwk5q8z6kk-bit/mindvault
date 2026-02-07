import { describe, expect, it } from 'vitest';
import type { NodeAttachment } from '$lib/api/files';
import {
	badgeClass,
	buildAttachmentEmbedMarkdown,
	classifyAttachment,
	extractionBadge,
	filterAttachmentChunks,
	formatExtractionStatus
} from './attachments';

const baseAttachment: NodeAttachment = {
	attachment_id: 'att-1',
	file_name: 'file.pdf',
	content_type: 'application/pdf',
	size_bytes: 1200,
	download_url: '/api/v1/files/node-1/att-1'
};

describe('attachment utils', () => {
	it('classifies attachments by type', () => {
		expect(classifyAttachment(baseAttachment).isPdf).toBe(true);
		expect(classifyAttachment({ ...baseAttachment, file_name: 'photo.png', content_type: 'image/png' }).isImage).toBe(true);
		expect(classifyAttachment({ ...baseAttachment, file_name: 'track.mp3', content_type: 'audio/mpeg' }).isAudio).toBe(true);
		expect(classifyAttachment({ ...baseAttachment, file_name: 'clip.mp4', content_type: 'video/mp4' }).isVideo).toBe(true);
	});

	it('formats extraction status with chars', () => {
		expect(formatExtractionStatus('indexed_text', 120)).toBe('indexed text · 120 chars');
		expect(formatExtractionStatus(null, null)).toBe('pending');
	});

	it('maps extraction badges and styles', () => {
		const badge = extractionBadge('tool_missing');
		expect(badge.label).toBe('Tool missing');
		expect(badgeClass(badge.tone)).toContain('amber');
	});

	it('filters chunks by query', () => {
		const chunks = [
			{ index: 0, text: 'alpha beta', char_count: 10 },
			{ index: 1, text: 'gamma delta', char_count: 11 }
		];
		expect(filterAttachmentChunks(chunks, 'alpha')).toHaveLength(1);
		expect(filterAttachmentChunks(chunks, '')).toHaveLength(2);
	});

	it('builds embed markdown by type', () => {
		const url = 'http://localhost/file';
		const image = { ...baseAttachment, file_name: 'photo.png', content_type: 'image/png' };
		const audio = { ...baseAttachment, file_name: 'song.mp3', content_type: 'audio/mpeg' };
		const video = { ...baseAttachment, file_name: 'clip.mp4', content_type: 'video/mp4' };

		expect(buildAttachmentEmbedMarkdown('', image, url)).toContain('![');
		expect(buildAttachmentEmbedMarkdown('', audio, url)).toContain('<audio');
		expect(buildAttachmentEmbedMarkdown('', video, url)).toContain('<video');
	});
});
