import { describe, it, expect } from 'vitest';
import { page } from 'vitest/browser';
import { render } from 'vitest-browser-svelte';
import CommandPalette from './CommandPalette.svelte';
import { closePalette, paletteOpen, paletteQuery } from '$lib/command-palette/store';

describe('CommandPalette', () => {
	it('opens and closes with Escape', async () => {
		render(CommandPalette);
		paletteQuery.set('');
		paletteOpen.set(true);

		const input = page.getByRole('combobox');
		await expect.element(input).toBeInTheDocument();

		closePalette();
		await expect.element(input).not.toBeInTheDocument();
	});
});
