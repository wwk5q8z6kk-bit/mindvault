import { beforeEach, describe, expect, it, vi } from 'vitest';

const fetchJson = vi.fn();

vi.mock('./client', () => ({
	fetchJson: (...args: unknown[]) => fetchJson(...args)
}));

import {
	getWorkspaceTree,
	listWorkspaces,
	mountWorkspace,
	readWorkspaceDocument,
	rebuildWorkspaceProjections,
	reconcileWorkspace
} from './workspaces';

describe('workspace api client', () => {
	beforeEach(() => {
		fetchJson.mockReset();
		fetchJson.mockResolvedValue({});
	});

	it('lists all workspaces or scopes the request by namespace', async () => {
		await listWorkspaces();
		await listWorkspaces('research & notes');

		expect(fetchJson).toHaveBeenNthCalledWith(1, '/api/v1/workspaces');
		expect(fetchJson).toHaveBeenNthCalledWith(2, '/api/v1/workspaces?namespace=research+%26+notes');
	});

	it('mounts a selected filesystem root with optional metadata', async () => {
		await mountWorkspace({
			root_path: '/Users/example/Knowledge',
			namespace: 'research',
			display_name: 'Knowledge Base'
		});

		expect(fetchJson).toHaveBeenCalledWith('/api/v1/workspaces', {
			method: 'POST',
			body: JSON.stringify({
				root_path: '/Users/example/Knowledge',
				namespace: 'research',
				display_name: 'Knowledge Base'
			})
		});
	});

	it('encodes workspace and document identifiers in read routes', async () => {
		await getWorkspaceTree('workspace/one');
		await readWorkspaceDocument('workspace/one', 'document?two');

		expect(fetchJson).toHaveBeenNthCalledWith(1, '/api/v1/workspaces/workspace%2Fone/tree');
		expect(fetchJson).toHaveBeenNthCalledWith(
			2,
			'/api/v1/workspaces/workspace%2Fone/documents/document%3Ftwo'
		);
	});

	it('uses POST for explicit reconciliation', async () => {
		await reconcileWorkspace('workspace-one');

		expect(fetchJson).toHaveBeenCalledWith('/api/v1/workspaces/workspace-one/reconcile', {
			method: 'POST'
		});
	});

	it('uses POST for an explicit projection rebuild', async () => {
		await rebuildWorkspaceProjections('workspace/one');

		expect(fetchJson).toHaveBeenCalledWith(
			'/api/v1/workspaces/workspace%2Fone/projections/rebuild',
			{ method: 'POST' }
		);
	});
});
