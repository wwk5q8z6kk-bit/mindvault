import { describe, expect, it } from 'vitest';
import { normalizeViewPreferences } from './view-preferences';

describe('normalizeViewPreferences', () => {
	it('falls back to default sidebar groups when stored value is not an array', () => {
		const prefs = normalizeViewPreferences({
			tasks: 'kanban',
			sidebarCollapsed: { bad: true }
		});

		expect(prefs.tasks).toBe('kanban');
		expect(prefs.sidebarCollapsed).toEqual(['System']);
	});

	it('normalizes sidebar groups to non-empty strings', () => {
		const prefs = normalizeViewPreferences({
			sidebarCollapsed: ['System', '', null, 42, 'Tasks']
		});

		expect(prefs.sidebarCollapsed).toEqual(['System', 'Tasks']);
	});
});

	it('defaults notes list sidebar prefs', () => {
		const prefs = normalizeViewPreferences({});
		expect(prefs.notesListCollapsed).toBe(false);
		expect(prefs.notesListTab).toBe('all');
	});

	it('preserves notes list collapse and tab', () => {
		const prefs = normalizeViewPreferences({
			notesListCollapsed: true,
			notesListTab: 'pinned'
		});
		expect(prefs.notesListCollapsed).toBe(true);
		expect(prefs.notesListTab).toBe('pinned');
	});

