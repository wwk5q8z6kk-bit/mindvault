import { expect, test } from '@playwright/test';

test('home page has expected h1', async ({ page }) => {
	await page.goto('/');
	// Scoped to the app header, which renders the current route's title.
	// A bare `locator('h1')` matched three headings once page content grew, and
	// Playwright's strict mode fails on multiple matches — so the test broke
	// without the app breaking. Naming the one heading it means keeps the
	// assertion specific rather than loosening it with `.first()`.
	await expect(page.locator('header h1')).toBeVisible();
});
