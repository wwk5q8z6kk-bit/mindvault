/**
 * Accessibility utilities for the editor.
 * Provides ARIA labels, announcements, and keyboard navigation.
 */

// ============================================================================
// Live Region Announcements
// ============================================================================

let liveRegion: HTMLElement | null = null;

/**
 * Get or create the live region for announcements.
 */
function getLiveRegion(): HTMLElement {
	if (!liveRegion) {
		liveRegion = document.createElement('div');
		liveRegion.setAttribute('role', 'status');
		liveRegion.setAttribute('aria-live', 'polite');
		liveRegion.setAttribute('aria-atomic', 'true');
		liveRegion.className = 'sr-only';
		liveRegion.style.cssText = `
			position: absolute;
			width: 1px;
			height: 1px;
			padding: 0;
			margin: -1px;
			overflow: hidden;
			clip: rect(0, 0, 0, 0);
			white-space: nowrap;
			border: 0;
		`;
		document.body.appendChild(liveRegion);
	}
	return liveRegion;
}

/**
 * Announce a message to screen readers.
 */
export function announce(message: string, priority: 'polite' | 'assertive' = 'polite'): void {
	const region = getLiveRegion();
	region.setAttribute('aria-live', priority);

	// Clear and set to trigger announcement
	region.textContent = '';
	requestAnimationFrame(() => {
		region.textContent = message;
	});
}

/**
 * Announce a formatting change.
 */
export function announceFormat(format: string, active: boolean): void {
	announce(`${format} ${active ? 'applied' : 'removed'}`);
}

/**
 * Announce a block type change.
 */
export function announceBlockType(type: string): void {
	announce(`Changed to ${type}`);
}

/**
 * Announce an action completion.
 */
export function announceAction(action: string): void {
	announce(action);
}

// ============================================================================
// ARIA Attributes
// ============================================================================

/**
 * Get ARIA attributes for the editor container.
 */
export function getEditorAriaAttrs(options: {
	label?: string;
	placeholder?: string;
	readonly?: boolean;
	required?: boolean;
}): Record<string, string> {
	const attrs: Record<string, string> = {
		role: 'textbox',
		'aria-multiline': 'true'
	};

	if (options.label) {
		attrs['aria-label'] = options.label;
	}

	if (options.placeholder) {
		attrs['aria-placeholder'] = options.placeholder;
	}

	if (options.readonly) {
		attrs['aria-readonly'] = 'true';
	}

	if (options.required) {
		attrs['aria-required'] = 'true';
	}

	return attrs;
}

/**
 * Get ARIA attributes for a toolbar button.
 */
export function getToolbarButtonAriaAttrs(options: {
	label: string;
	pressed?: boolean;
	disabled?: boolean;
	keyboardShortcut?: string;
}): Record<string, string> {
	const attrs: Record<string, string> = {
		role: 'button',
		'aria-label': options.label
	};

	if (options.pressed !== undefined) {
		attrs['aria-pressed'] = String(options.pressed);
	}

	if (options.disabled) {
		attrs['aria-disabled'] = 'true';
	}

	if (options.keyboardShortcut) {
		attrs['aria-keyshortcuts'] = options.keyboardShortcut;
	}

	return attrs;
}

/**
 * Get ARIA attributes for a menu.
 */
export function getMenuAriaAttrs(options: {
	label?: string;
	orientation?: 'horizontal' | 'vertical';
}): Record<string, string> {
	return {
		role: 'menu',
		'aria-label': options.label || 'Menu',
		'aria-orientation': options.orientation || 'vertical'
	};
}

/**
 * Get ARIA attributes for a menu item.
 */
export function getMenuItemAriaAttrs(options: {
	label: string;
	selected?: boolean;
	disabled?: boolean;
}): Record<string, string> {
	const attrs: Record<string, string> = {
		role: 'menuitem',
		'aria-label': options.label
	};

	if (options.selected) {
		attrs['aria-selected'] = 'true';
	}

	if (options.disabled) {
		attrs['aria-disabled'] = 'true';
	}

	return attrs;
}

// ============================================================================
// Keyboard Navigation
// ============================================================================

/**
 * Handle arrow key navigation in a list.
 */
export function handleListNavigation(
	event: KeyboardEvent,
	currentIndex: number,
	itemCount: number,
	onSelect: (index: number) => void
): boolean {
	let newIndex = currentIndex;

	switch (event.key) {
		case 'ArrowDown':
			newIndex = (currentIndex + 1) % itemCount;
			break;
		case 'ArrowUp':
			newIndex = (currentIndex - 1 + itemCount) % itemCount;
			break;
		case 'Home':
			newIndex = 0;
			break;
		case 'End':
			newIndex = itemCount - 1;
			break;
		default:
			return false;
	}

	if (newIndex !== currentIndex) {
		event.preventDefault();
		onSelect(newIndex);
		return true;
	}

	return false;
}

/**
 * Handle type-ahead search in a list.
 */
export function createTypeAheadHandler(
	getItems: () => { label: string }[],
	onSelect: (index: number) => void
): (event: KeyboardEvent) => void {
	let searchBuffer = '';
	let searchTimer: ReturnType<typeof setTimeout> | null = null;

	return (event: KeyboardEvent) => {
		// Only handle printable characters
		if (event.key.length !== 1 || event.ctrlKey || event.metaKey || event.altKey) {
			return;
		}

		// Add to search buffer
		searchBuffer += event.key.toLowerCase();

		// Clear buffer after delay
		if (searchTimer) {
			clearTimeout(searchTimer);
		}
		searchTimer = setTimeout(() => {
			searchBuffer = '';
		}, 500);

		// Find matching item
		const items = getItems();
		const matchIndex = items.findIndex((item) =>
			item.label.toLowerCase().startsWith(searchBuffer)
		);

		if (matchIndex !== -1) {
			onSelect(matchIndex);
		}
	};
}

// ============================================================================
// Focus Management
// ============================================================================

/**
 * Trap focus within an element (for modals).
 */
export function trapFocus(container: HTMLElement): () => void {
	const focusableSelectors = [
		'button:not([disabled])',
		'input:not([disabled])',
		'select:not([disabled])',
		'textarea:not([disabled])',
		'a[href]',
		'[tabindex]:not([tabindex="-1"])'
	].join(', ');

	const getFocusableElements = () =>
		Array.from(container.querySelectorAll<HTMLElement>(focusableSelectors));

	const handleKeyDown = (event: KeyboardEvent) => {
		if (event.key !== 'Tab') return;

		const focusable = getFocusableElements();
		if (focusable.length === 0) return;

		const first = focusable[0];
		const last = focusable[focusable.length - 1];

		if (event.shiftKey && document.activeElement === first) {
			event.preventDefault();
			last.focus();
		} else if (!event.shiftKey && document.activeElement === last) {
			event.preventDefault();
			first.focus();
		}
	};

	container.addEventListener('keydown', handleKeyDown);

	// Focus first element
	const focusable = getFocusableElements();
	if (focusable.length > 0) {
		focusable[0].focus();
	}

	return () => {
		container.removeEventListener('keydown', handleKeyDown);
	};
}

/**
 * Restore focus to a previously focused element.
 */
export function createFocusRestore(): {
	save: () => void;
	restore: () => void;
} {
	let savedElement: HTMLElement | null = null;

	return {
		save: () => {
			savedElement = document.activeElement as HTMLElement | null;
		},
		restore: () => {
			if (savedElement && typeof savedElement.focus === 'function') {
				savedElement.focus();
			}
		}
	};
}

// ============================================================================
// High Contrast Mode
// ============================================================================

/**
 * Check if high contrast mode is enabled.
 */
export function isHighContrastMode(): boolean {
	if (typeof window === 'undefined') return false;

	const mediaQuery = window.matchMedia('(forced-colors: active)');
	return mediaQuery.matches;
}

/**
 * Check if reduced motion is preferred.
 */
export function prefersReducedMotion(): boolean {
	if (typeof window === 'undefined') return false;

	const mediaQuery = window.matchMedia('(prefers-reduced-motion: reduce)');
	return mediaQuery.matches;
}
