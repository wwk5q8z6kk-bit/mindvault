import { describe, expect, it } from 'vitest';
import { buildTaskPayloadsFromActionItems, parseActionItems } from './action-items';

describe('parseActionItems', () => {
	it('parses structured task lines with priority, due date, estimate, and tags', () => {
		const now = new Date('2026-02-06T00:00:00Z');
		const text = [
			'Action Items:',
			'- [ ] Draft release notes p2 due:2026-02-12 #release #docs est:35m',
			'- [ ] Run smoke tests !1 due:tomorrow #qa ~20m'
		].join('\n');

		const result = parseActionItems(text, { now });

		expect(result).toEqual([
			{
				title: 'Draft release notes',
				priority: 2,
				due_at: '2026-02-12',
				estimate_min: 35,
				labels: ['release', 'docs']
			},
			{
				title: 'Run smoke tests',
				priority: 1,
				due_at: '2026-02-07',
				estimate_min: 20,
				labels: ['qa']
			}
		]);
	});

	it('deduplicates similar task titles and skips headings', () => {
		const text = [
			'Tasks:',
			'- Ship planner panel',
			'- ship planner panel',
			'Next steps:',
			'Finalize onboarding copy'
		].join('\n');

		const result = parseActionItems(text);
		expect(result.map((item) => item.title)).toEqual([
			'Ship planner panel',
			'Finalize onboarding copy'
		]);
	});
});

describe('buildTaskPayloadsFromActionItems', () => {
	it('applies defaults and metadata to generated payloads', () => {
		const payloads = buildTaskPayloadsFromActionItems(
			[
				{
					title: 'Prepare roadmap review',
					priority: 2,
					due_at: null,
					estimate_min: null,
					labels: ['roadmap']
				}
			],
			{
				status: 'planned',
				defaultLabels: ['from-note'],
				baseMetadata: { source_note_id: 'note-1' }
			}
		);

		expect(payloads).toEqual([
			{
				title: 'Prepare roadmap review',
				status: 'planned',
				priority: 2,
				due_at: null,
				estimate_min: null,
				labels: ['from-note', 'roadmap'],
				metadata: {
					source_note_id: 'note-1',
					ai_extracted_action_item: true
				}
			}
		]);
	});
});
