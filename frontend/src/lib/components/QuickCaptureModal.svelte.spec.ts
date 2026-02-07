import { describe, expect, it } from 'vitest';
import { page } from 'vitest/browser';
import { render } from 'vitest-browser-svelte';
import QuickCaptureModal from './QuickCaptureModal.svelte';

describe('QuickCaptureModal', () => {
	it('opens when the desktop quick-capture event is dispatched', async () => {
		render(QuickCaptureModal);
		window.dispatchEvent(new CustomEvent('mindvault:quick-capture'));
		const dialog = page.getByRole('dialog', { name: 'Quick capture' });
		await expect.element(dialog).toBeInTheDocument();
	});

	it('switches to link capture mode when shortcut detail requests link mode', async () => {
		render(QuickCaptureModal);
		window.dispatchEvent(new CustomEvent('mindvault:quick-capture', { detail: { mode: 'link' } }));
		const linkInput = page.getByPlaceholder('https://example.com/article');
		await expect.element(linkInput).toBeInTheDocument();
	});

	it('applies shortcut target routing when event detail includes target', async () => {
		render(QuickCaptureModal);
		window.dispatchEvent(
			new CustomEvent('mindvault:quick-capture', { detail: { mode: 'note', target: 'daily' } })
		);
		const targetSelect = page.getByLabelText('Capture target');
		await expect.element(targetSelect).toHaveValue('daily');
	});

	it('supports keyboard shortcut capture target routing in browser context', async () => {
		render(QuickCaptureModal);
		window.dispatchEvent(new KeyboardEvent('keydown', { key: 'd', ctrlKey: true, shiftKey: true }));
		const targetSelect = page.getByLabelText('Capture target');
		await expect.element(targetSelect).toHaveValue('daily');
	});

	it('supports planned target routing via shortcut detail', async () => {
		render(QuickCaptureModal);
		window.dispatchEvent(
			new CustomEvent('mindvault:quick-capture', { detail: { mode: 'task', target: 'planned' } })
		);
		const targetSelect = page.getByLabelText('Capture target');
		await expect.element(targetSelect).toHaveValue('planned');
	});

	it('prefills capture text when event detail includes prefill', async () => {
		render(QuickCaptureModal);
		window.dispatchEvent(
			new CustomEvent('mindvault:quick-capture', {
				detail: { mode: 'task', target: 'inbox', prefill: 'Follow up: Review roadmap' }
			})
		);
		const taskInput = page.getByPlaceholder('buy milk tomorrow 5pm p2 #home');
		await expect.element(taskInput).toHaveValue('Follow up: Review roadmap');
	});
});
