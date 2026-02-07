/* @vitest-environment jsdom */
import { describe, it, expect } from 'vitest';
import { Editor } from '@tiptap/core';
import { StarterKit } from '@tiptap/starter-kit';
import { Markdown } from '@tiptap/markdown';

describe('RichNoteEditor markdown round-trip', () => {
	it('preserves basic markdown formatting', () => {
		const input = '# Title\n\nHello **world**\n\n- Item 1\n- Item 2';
		const editor = new Editor({
			extensions: [StarterKit, Markdown],
			content: input,
			contentType: 'markdown'
		});
		const output = editor.getMarkdown().trim();
		expect(output).toContain('# Title');
		expect(output).toContain('**world**');
		expect(output).toContain('- Item 1');
		editor.destroy();
	});
});
