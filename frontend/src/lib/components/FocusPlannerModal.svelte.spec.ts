import { describe, it, expect } from 'vitest';
import { page } from 'vitest/browser';
import { render } from 'vitest-browser-svelte';
import FocusPlannerModal from './FocusPlannerModal.svelte';
import { focusPlannerState } from '$lib/stores/ui';

describe('FocusPlannerModal', () => {
	it('renders focus list items when open', async () => {
		render(FocusPlannerModal);
		focusPlannerState.set({
			open: true,
			generatedAt: '2026-02-05T12:00:00Z',
			items: [
				{
					task: {
						id: 'task-1',
						title: 'Ship focus planner',
						status: 'inbox',
						priority: 1,
						due_at: null
					},
					score: 0.9,
					rank: 1,
					reason: 'High priority'
				}
			]
		});

		const heading = page.getByRole('heading', { name: 'Focus list' });
		await expect.element(heading).toBeInTheDocument();
		await expect.element(page.getByText('Ship focus planner')).toBeInTheDocument();
	});
});
