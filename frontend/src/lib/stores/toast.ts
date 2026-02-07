import { writable } from 'svelte/store';

export interface Toast {
	id: string;
	message: string;
	variant?: 'info' | 'success' | 'warning' | 'danger';
	undoId?: string;
}

export const toastStore = writable<Toast[]>([]);

export function pushToast(
	message: string,
	variant: Toast['variant'] = 'info',
	timeout = 3000,
	undoId?: string
) {
	const id = crypto.randomUUID();
	toastStore.update((items) => [...items, { id, message, variant, undoId }]);

	setTimeout(() => {
		toastStore.update((items) => items.filter((toast) => toast.id !== id));
	}, timeout);
}
