export interface SidebarViewPreferences {
	tasks: string;
	review: string;
	resources: string;
}

type ViewPreferenceKey = keyof SidebarViewPreferences;

export interface SidebarNavigationItem {
	label: string;
	href: string;
	viewPreferenceKey?: ViewPreferenceKey;
	match?: {
		aliases?: string[];
		excludeViews?: string[];
	};
}

export interface SidebarNavigationGroup {
	label?: string;
	preferenceKey?: string;
	collapsible: boolean;
	items: SidebarNavigationItem[];
}

export const sidebarNavigationGroups: SidebarNavigationGroup[] = [
	{
		collapsible: false,
		items: [
			{ label: 'Dashboard', href: '/' },
			{ label: 'Inbox', href: '/inbox' },
			{ label: 'Daily', href: '/focus' },
			{ label: 'Search', href: '/search' }
		]
	},
	{
		label: 'Vault',
		preferenceKey: 'Knowledge Base',
		collapsible: true,
		items: [
			{
				label: 'Notes',
				href: '/notes',
				match: { excludeViews: ['graph'] }
			},
			{ label: 'Resources', href: '/bookmarks', viewPreferenceKey: 'resources' }
		]
	},
	{
		label: 'Knowledge',
		collapsible: true,
		items: [
			{
				label: 'Graph',
				href: '/notes?view=graph',
				match: { aliases: ['/graph'] }
			},
			{ label: 'Review', href: '/review', viewPreferenceKey: 'review' }
		]
	},
	{
		label: 'Work',
		preferenceKey: 'Productivity',
		collapsible: true,
		items: [
			{ label: 'Tasks', href: '/tasks', viewPreferenceKey: 'tasks' },
			{ label: 'Goals', href: '/goals' }
		]
	},
	{
		label: 'Relay',
		preferenceKey: 'Connections',
		collapsible: true,
		items: [
			{ label: 'Chat', href: '/chat' },
			{ label: 'Relay', href: '/relay' }
		]
	},
	{
		label: 'Control',
		preferenceKey: 'System',
		collapsible: true,
		items: [
			{ label: 'Sync', href: '/sync' },
			{ label: 'Plugins', href: '/plugins' },
			{ label: 'Work Orders', href: '/work-orders' },
			{ label: 'Autonomy', href: '/autonomy' },
			{ label: 'Settings', href: '/settings' }
		]
	}
];

function splitHref(href: string): { pathname: string; view: string | null } {
	const [pathname, query = ''] = href.split('?');
	return {
		pathname,
		view: new URLSearchParams(query).get('view')
	};
}

export function isSidebarNavigationItemActive(
	item: SidebarNavigationItem,
	url: Pick<URL, 'pathname' | 'searchParams'>
): boolean {
	const target = splitHref(item.href);
	const pathMatches =
		url.pathname === target.pathname ||
		(target.pathname !== '/' && url.pathname.startsWith(`${target.pathname}/`)) ||
		item.match?.aliases?.includes(url.pathname) === true;

	if (!pathMatches) return false;

	const activeView = url.searchParams.get('view');
	if (target.view && activeView !== target.view && !item.match?.aliases?.includes(url.pathname)) {
		return false;
	}

	return !item.match?.excludeViews?.includes(activeView ?? '');
}

export function sidebarNavigationHref(
	item: SidebarNavigationItem,
	preferences: SidebarViewPreferences
): string {
	if (!item.viewPreferenceKey) return item.href;

	const view = preferences[item.viewPreferenceKey];
	const defaultView: Record<ViewPreferenceKey, string> = {
		tasks: 'list',
		review: 'digest',
		resources: 'bookmarks'
	};

	return view !== defaultView[item.viewPreferenceKey]
		? `${item.href}?view=${encodeURIComponent(view)}`
		: item.href;
}
