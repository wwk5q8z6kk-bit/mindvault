import { derived } from 'svelte/store';
import { page } from '$app/stores';
import { goto } from '$app/navigation';

/**
 * Create a derived store that reads the current view mode from the URL query param `?view=`.
 * Falls back to `defaultView` if the param is missing or not in `allowedViews`.
 */
export function createViewMode(defaultView: string, allowedViews: string[]) {
	return derived(page, ($page) => {
		const param = $page.url.searchParams.get('view');
		if (param && allowedViews.includes(param)) return param;
		return defaultView;
	});
}

/**
 * Navigate to the given view mode by updating the `?view=` query param.
 * If `view` equals `defaultView`, removes the param for cleaner URLs.
 * Uses `replaceState` to avoid polluting browser history.
 */
export function setViewMode(view: string, defaultView: string) {
	const url = new URL(window.location.href);
	if (view === defaultView) {
		url.searchParams.delete('view');
	} else {
		url.searchParams.set('view', view);
	}
	goto(url.pathname + url.search, {
		replaceState: true,
		keepFocus: true,
		noScroll: true
	});
}
