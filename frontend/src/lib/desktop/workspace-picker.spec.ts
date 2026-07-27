import { beforeEach, describe, expect, it, vi } from 'vitest';

const isTauri = vi.fn();
const open = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({
	isTauri: () => isTauri()
}));

vi.mock('@tauri-apps/plugin-dialog', () => ({
	open: (...args: unknown[]) => open(...args)
}));

import { pickWorkspaceDirectory, supportsNativeWorkspacePicker } from './workspace-picker';

describe('workspace directory picker', () => {
	beforeEach(() => {
		isTauri.mockReset();
		open.mockReset();
	});

	it('does not invoke a native dialog in the browser', async () => {
		isTauri.mockReturnValue(false);

		expect(supportsNativeWorkspacePicker()).toBe(false);
		await expect(pickWorkspaceDirectory()).resolves.toBeNull();
		expect(open).not.toHaveBeenCalled();
	});

	it('selects one recursive directory in the desktop shell', async () => {
		isTauri.mockReturnValue(true);
		open.mockResolvedValue('/Users/example/Knowledge');

		await expect(pickWorkspaceDirectory()).resolves.toBe('/Users/example/Knowledge');
		expect(open).toHaveBeenCalledWith({
			title: 'Choose a MindVault workspace',
			directory: true,
			multiple: false,
			canCreateDirectories: true
		});
	});

	it('returns null when the desktop picker is cancelled', async () => {
		isTauri.mockReturnValue(true);
		open.mockResolvedValue(null);

		await expect(pickWorkspaceDirectory()).resolves.toBeNull();
	});
});
