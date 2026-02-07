import { test, expect } from '@playwright/test';

test('command palette opens and navigates', async ({ page }) => {
	await page.goto('http://localhost:4173/');
	await page.click('body');
	await page.keyboard.press('Meta+K');
	await page.keyboard.press('Control+K');
	const input = page.getByRole('combobox');
	await expect(input).toBeVisible({ timeout: 10000 });
	await input.fill('open calendar');
	const option = page.getByRole('option', { name: /Open Calendar/i });
	await option.click();
	await expect(page).toHaveURL(/\/calendar/);
});
