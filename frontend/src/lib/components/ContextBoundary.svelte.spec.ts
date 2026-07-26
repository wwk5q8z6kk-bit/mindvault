import { describe, expect, it } from 'vitest';
import { page } from 'vitest/browser';
import { render } from 'vitest-browser-svelte';
import ContextBoundary from './ContextBoundary.svelte';

describe('ContextBoundary', () => {
	it('identifies the current Personal Vault as private', async () => {
		render(ContextBoundary);

		const boundary = page.getByRole('group', {
			name: 'Current context: Personal Vault. Private'
		});
		await expect.element(boundary).toBeInTheDocument();
		await expect.element(page.getByText('Personal Vault')).toBeInTheDocument();
		await expect.element(page.getByText('Private')).toBeInTheDocument();
	});
});
