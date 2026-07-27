import { test, expect } from '@playwright/test';

test('command palette opens and navigates', async ({ page }) => {
	await page.goto('http://localhost:4173/');
	await page.click('body');
	await page.keyboard.press('Meta+K');
	await page.keyboard.press('Control+K');
	// The palette's own input, not "whichever combobox is first in the DOM".
	// `getByRole('combobox')` also matches the page's `<select>` elements, which
	// carry that role implicitly, so it began resolving to two elements and
	// failed strict mode.
	const input = page.getByPlaceholder('What do you need?');
	await expect(input).toBeVisible({ timeout: 10000 });
	await input.fill('open calendar');
	const option = page.getByRole('option', { name: /Open Calendar/i });
	await option.click();
	// The command navigates to `/calendar`, which is a redirect shim: it
	// `replaceState`s to the real calendar view (`src/routes/calendar/+page.svelte`).
	// Asserting the pre-redirect path made the test fail while the app worked.
	await expect(page).toHaveURL(/view=calendar/);
});
