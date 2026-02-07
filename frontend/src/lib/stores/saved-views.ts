import { derived, get, writable } from 'svelte/store';
import type { SavedView } from '$lib/api/saved-views';
import { listSavedViews } from '$lib/api/saved-views';
import { activeNamespace } from '$lib/stores/namespace';

export const savedViewsStore = writable<SavedView[]>([]);
export const savedViewsLoading = writable(false);
export const activeSavedViewId = writable<string | null>(null);

export const activeSavedView = derived(
	[savedViewsStore, activeSavedViewId],
	([$views, $activeId]) => $views.find((view) => view.id === $activeId) ?? null
);

let loaded = false;

export async function loadSavedViews(force = false): Promise<void> {
	if (loaded && !force) return;
	savedViewsLoading.set(true);
	try {
		const ns = get(activeNamespace) ?? undefined;
		const views = await listSavedViews({ namespace: ns ?? undefined, limit: 200 });
		savedViewsStore.set(views);
		loaded = true;
	} finally {
		savedViewsLoading.set(false);
	}
}

export function setActiveSavedView(view: SavedView | null): void {
	activeSavedViewId.set(view?.id ?? null);
}
