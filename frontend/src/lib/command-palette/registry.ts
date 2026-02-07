import { writable } from 'svelte/store';
import type { CommandAction, CommandContext } from './types';

const actionsStore = writable<CommandAction[]>([]);

function upsert(actions: CommandAction[], next: CommandAction): CommandAction[] {
	const filtered = actions.filter((action) => action.id !== next.id);
	return [...filtered, next];
}

export function registerAction(action: CommandAction) {
	actionsStore.update((actions) => upsert(actions, action));
}

export function registerActions(actions: CommandAction[]) {
	actionsStore.update((current) => {
		let next = [...current];
		for (const action of actions) {
			next = upsert(next, action);
		}
		return next;
	});
}

export function removeAction(id: string) {
	actionsStore.update((actions) => actions.filter((action) => action.id !== id));
}

export function filterAvailable(actions: CommandAction[], ctx: CommandContext) {
	return actions.filter((action) => (action.isAvailable ? action.isAvailable(ctx) : true));
}

export { actionsStore };
