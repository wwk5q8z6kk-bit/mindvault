import { writable } from 'svelte/store';
import { STORAGE_KEYS } from '$lib/constants/storage-keys';

export type ThemeMode = 'system' | 'light' | 'dark';

const STORAGE_KEY = STORAGE_KEYS.THEME;

function applyTheme(mode: ThemeMode) {
	const root = document.documentElement;
	if (mode === 'system') {
		root.removeAttribute('data-theme');
		return;
	}
	root.setAttribute('data-theme', mode);
}

const initial = ((): ThemeMode => {
	if (typeof localStorage === 'undefined') return 'system';
	const stored = localStorage.getItem(STORAGE_KEY) as ThemeMode | null;
	return stored ?? 'system';
})();

export const themeMode = writable<ThemeMode>(initial);

themeMode.subscribe((mode) => {
	if (typeof document === 'undefined') return;
	applyTheme(mode);
	if (typeof localStorage !== 'undefined') {
		localStorage.setItem(STORAGE_KEY, mode);
	}
});

export function toggleTheme() {
	themeMode.update((mode) => {
		if (mode === 'dark') return 'light';
		if (mode === 'light') return 'dark';
		const prefersDark = window.matchMedia?.('(prefers-color-scheme: dark)').matches ?? true;
		return prefersDark ? 'light' : 'dark';
	});
}

export function setTheme(mode: ThemeMode) {
	themeMode.set(mode);
}
