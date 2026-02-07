import { writable } from 'svelte/store';
import { listNodes } from '$lib/api/nodes';

const STORAGE_KEY = 'mv_active_namespace';

function getStored(): string | null {
	if (typeof localStorage === 'undefined') return null;
	return localStorage.getItem(STORAGE_KEY);
}

export const activeNamespace = writable<string | null>(getStored());
export const availableNamespaces = writable<string[]>([]);

activeNamespace.subscribe(val => {
	if (typeof localStorage === 'undefined') return;
	if (val) localStorage.setItem(STORAGE_KEY, val);
	else localStorage.removeItem(STORAGE_KEY);
});

export async function loadAvailableNamespaces(): Promise<void> {
	try {
		const nodes = await listNodes({ limit: 200 });
		const ns = new Set<string>();
		for (const node of nodes) {
			if (node.namespace) ns.add(node.namespace);
		}
		availableNamespaces.set([...ns].sort());
	} catch {
		// silently fail — namespace selector will just show "All"
	}
}
