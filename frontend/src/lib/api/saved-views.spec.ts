import { describe, expect, it, vi } from 'vitest';
import {
	createSavedView,
	deleteSavedView,
	listSavedViews,
	updateSavedView,
	type SavedView
} from './saved-views';

vi.mock('./client', () => ({
	API_BASE_URL: 'http://localhost',
	fetchJson: vi.fn(async (path: string, options?: RequestInit) => {
		if (path.startsWith('/api/v1/saved_views?')) {
			return [];
		}
		if (path === '/api/v1/saved_views') {
			return {
				id: 'view-1',
				name: 'Today',
				namespace: 'default',
				view_type: 'list',
				filters: {},
				updated_at: 'now'
			} satisfies SavedView;
		}
		if (path === '/api/v1/saved_views/view-1') {
			if (options?.method === 'DELETE') {
				return { deleted: true };
			}
			return {
				id: 'view-1',
				name: 'Today Updated',
				namespace: 'default',
				view_type: 'kanban',
				filters: {},
				updated_at: 'now'
			} satisfies SavedView;
		}
		return { deleted: true };
	})
}));

describe('saved views api client', () => {
	it('lists saved views with query params', async () => {
		await listSavedViews({ namespace: 'ops', limit: 25, offset: 5 });
		const { fetchJson } = await import('./client');
		expect(fetchJson).toHaveBeenCalledWith('/api/v1/saved_views?namespace=ops&limit=25&offset=5');
	});

	it('creates, updates, deletes saved views', async () => {
		const created = await createSavedView({ name: 'Today', view_type: 'list' });
		expect(created.view_type).toBe('list');
		const updated = await updateSavedView('view-1', { name: 'Today Updated' });
		expect(updated.name).toBe('Today Updated');
		const deleted = await deleteSavedView('view-1');
		expect(deleted.deleted).toBe(true);
	});
});
