import { isTauri } from '@tauri-apps/api/core';

export function supportsNativeWorkspacePicker(): boolean {
	return isTauri();
}

export async function pickWorkspaceDirectory(): Promise<string | null> {
	if (!supportsNativeWorkspacePicker()) return null;

	const { open } = await import('@tauri-apps/plugin-dialog');
	const selected = await open({
		title: 'Choose a MindVault workspace',
		directory: true,
		multiple: false,
		canCreateDirectories: true
	});

	return typeof selected === 'string' ? selected : null;
}
