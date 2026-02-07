/**
 * Built-in lists plugin.
 * Provides list commands, slash items, and toolbar items.
 */

import type { EditorPlugin, PluginContext, SlashItem, ToolbarItem } from '../types';
import { createList, createListItem, createTaskList, createTaskItem } from '../../model/document';

/**
 * Lists plugin provides bullet lists, numbered lists, and task lists.
 */
export const listsPlugin: EditorPlugin = {
	id: 'lists',
	name: 'Lists',
	description: 'Bullet, numbered, and task lists',
	version: '1.0.0',

	commands: {
		insertBulletList: (ctx: PluginContext) => {
			const list = createList(false);
			ctx.insertBlock(list);
		},
		insertNumberedList: (ctx: PluginContext) => {
			const list = createList(true);
			ctx.insertBlock(list);
		},
		insertTaskList: (ctx: PluginContext) => {
			const list = createTaskList();
			ctx.insertBlock(list);
		}
	},

	slashItems: [
		{
			id: 'bullet-list',
			label: 'Bullet List',
			description: 'Unordered list',
			icon: '•',
			keywords: ['ul', 'unordered', 'bullet'],
			group: 'lists',
			action: (ctx) => {
				const list = createList(false);
				ctx.insertBlock(list);
			}
		},
		{
			id: 'numbered-list',
			label: 'Numbered List',
			description: 'Ordered list',
			icon: '1.',
			keywords: ['ol', 'ordered', 'numbered'],
			group: 'lists',
			action: (ctx) => {
				const list = createList(true);
				ctx.insertBlock(list);
			}
		},
		{
			id: 'task-list',
			label: 'Task List',
			description: 'Checklist items',
			icon: '☑',
			keywords: ['todo', 'checkbox', 'checklist'],
			group: 'lists',
			action: (ctx) => {
				const list = createTaskList();
				ctx.insertBlock(list);
			}
		}
	],

	toolbarItems: [
		{
			id: 'bullet-list',
			label: '• List',
			title: 'Bullet list',
			group: 'lists',
			action: (ctx) => {
				const list = createList(false);
				ctx.insertBlock(list);
			},
			isActive: (ctx) => ctx.getCurrentBlockType() === 'list'
		},
		{
			id: 'numbered-list',
			label: '1. List',
			title: 'Numbered list',
			group: 'lists',
			action: (ctx) => {
				const list = createList(true);
				ctx.insertBlock(list);
			},
			isActive: (ctx) => ctx.getCurrentBlockType() === 'list'
		},
		{
			id: 'task-list',
			label: '☐ Tasks',
			title: 'Task list',
			group: 'lists',
			action: (ctx) => {
				const list = createTaskList();
				ctx.insertBlock(list);
			},
			isActive: (ctx) => ctx.getCurrentBlockType() === 'task-list'
		}
	],

	inputRules: [
		// Bullet list: "- " at start of line
		{
			id: 'bullet-list-dash',
			pattern: /^-\s$/,
			handler: (ctx, match) => {
				ctx.setBlockType('list', { ordered: false });
				return true;
			}
		},
		// Bullet list: "* " at start of line
		{
			id: 'bullet-list-asterisk',
			pattern: /^\*\s$/,
			handler: (ctx, match) => {
				ctx.setBlockType('list', { ordered: false });
				return true;
			}
		},
		// Numbered list: "1. " at start of line
		{
			id: 'numbered-list',
			pattern: /^\d+\.\s$/,
			handler: (ctx, match) => {
				ctx.setBlockType('list', { ordered: true });
				return true;
			}
		},
		// Task list: "- [ ] " at start of line
		{
			id: 'task-list-unchecked',
			pattern: /^-\s\[\s?\]\s$/,
			handler: (ctx, match) => {
				// Convert to task list with unchecked item
				return true;
			}
		},
		// Task list: "- [x] " at start of line
		{
			id: 'task-list-checked',
			pattern: /^-\s\[[xX]\]\s$/,
			handler: (ctx, match) => {
				// Convert to task list with checked item
				return true;
			}
		}
	]
};

export default listsPlugin;
