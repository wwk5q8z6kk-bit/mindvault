import { describe, expect, it } from 'vitest';
import { applyJsonCanvasLayout, exportToJsonCanvas, extractTitleFromText } from './jsonCanvas';

describe('json canvas helpers', () => {
	it('extracts title from markdown text', () => {
		expect(extractTitleFromText('# Project Alpha\n\nDetails')).toBe('Project Alpha');
		expect(extractTitleFromText('First line title')).toBe('First line title');
		expect(extractTitleFromText('')).toBeNull();
	});

	it('exports cards to json canvas format', () => {
		const doc = exportToJsonCanvas([
			{ id: 'a', title: 'Alpha', content: 'Body', x: 10, y: 20, width: 200, height: 100 }
		]);
		expect(doc.nodes).toHaveLength(1);
		expect(doc.nodes[0].id).toBe('a');
		expect(doc.nodes[0].type).toBe('text');
		expect(doc.nodes[0].text).toContain('# Alpha');
		expect(doc.edges).toHaveLength(0);
	});

	it('applies layout using id match and title fallback', () => {
		const cards = [
			{ id: 'one', title: 'Alpha', content: '', x: 0, y: 0, width: 100, height: 80 },
			{ id: 'two', title: 'Beta', content: '', x: 5, y: 5, width: 100, height: 80 }
		];
		const doc = {
			nodes: [
				{ id: 'one', type: 'text' as const, x: 44, y: 55, width: 120, height: 90 },
				{ id: 'missing', type: 'text' as const, x: 8, y: 9, width: 110, height: 70, text: '# Beta' }
			],
			edges: []
		};
		const result = applyJsonCanvasLayout(cards, doc);
		expect(result.matched).toBe(2);
		expect(result.skipped).toBe(0);
		const updatedAlpha = result.cards.find((c) => c.id === 'one');
		const updatedBeta = result.cards.find((c) => c.id === 'two');
		expect(updatedAlpha?.x).toBe(44);
		expect(updatedBeta?.x).toBe(8);
	});
});
