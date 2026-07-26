import { beforeEach, describe, expect, it } from 'vitest';
import { page } from 'vitest/browser';
import { render } from 'vitest-browser-svelte';
import { activeNamespace, availableNamespaces } from '$lib/stores/namespace';
import NamespaceSelector from './NamespaceSelector.svelte';

describe('NamespaceSelector', () => {
	beforeEach(() => {
		activeNamespace.set(null);
		availableNamespaces.set([]);
	});

	it('identifies itself as a record filter inside the current vault', async () => {
		render(NamespaceSelector);

		const selector = page.getByRole('combobox', {
			name: 'Filter records by namespace'
		});
		await expect.element(selector).toBeDisabled();
		await expect.element(page.getByText('Records')).toBeInTheDocument();
		await expect.element(page.getByRole('option', { name: 'All records' })).toBeInTheDocument();
	});

	it('keeps a selected namespace available when discovery is temporarily unavailable', async () => {
		activeNamespace.set('research');
		render(NamespaceSelector);

		const selector = page.getByRole('combobox', {
			name: 'Filter records by namespace'
		});
		await expect.element(selector).toBeEnabled();
		await expect.element(page.getByRole('option', { name: 'research' })).toBeInTheDocument();
		await expect.element(selector).toHaveValue('research');
	});
});
