import { describe, expect, it } from 'vitest';
import { mergeTaskLabels, parseQuickAddPreview } from './tasks';

describe('tasks api helpers', () => {
	it('merges and deduplicates default labels case-insensitively', () => {
		expect(mergeTaskLabels(['focus', 'Inbox'], ['inbox', 'daily-capture'])).toEqual([
			'focus',
			'Inbox',
			'daily-capture'
		]);
	});

	it('quick-add parser still extracts labels and metadata markers', () => {
		const preview = parseQuickAddPreview('Ship docs p2 #ops #docs tomorrow');
		expect(preview.priority).toBe(2);
		expect(preview.labels).toEqual(['ops', 'docs']);
		expect(preview.due_at).toBeTruthy();
	});
});
