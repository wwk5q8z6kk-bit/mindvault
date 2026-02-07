/**
 * Built-in formatting plugin.
 * Provides basic text formatting commands and toolbar items.
 */

import type { EditorPlugin, PluginContext, ToolbarItem, KeyboardShortcut } from '../types';

/**
 * Formatting plugin provides bold, italic, strikethrough, and code formatting.
 */
export const formattingPlugin: EditorPlugin = {
	id: 'formatting',
	name: 'Formatting',
	description: 'Basic text formatting (bold, italic, strikethrough, code)',
	version: '1.0.0',

	commands: {
		toggleBold: (ctx: PluginContext) => {
			ctx.toggleMark('bold');
		},
		toggleItalic: (ctx: PluginContext) => {
			ctx.toggleMark('italic');
		},
		toggleStrikethrough: (ctx: PluginContext) => {
			ctx.toggleMark('strikethrough');
		},
		toggleCode: (ctx: PluginContext) => {
			ctx.toggleMark('code');
		}
	},

	keyboardShortcuts: [
		{
			key: 'b',
			mod: true,
			handler: (ctx: PluginContext) => {
				ctx.toggleMark('bold');
				return true;
			}
		},
		{
			key: 'i',
			mod: true,
			handler: (ctx: PluginContext) => {
				ctx.toggleMark('italic');
				return true;
			}
		}
	],

	toolbarItems: [
		{
			id: 'bold',
			label: 'B',
			title: 'Bold (Ctrl+B)',
			group: 'formatting',
			action: (ctx) => ctx.toggleMark('bold'),
			isActive: (ctx) => ctx.isMarkActive('bold')
		},
		{
			id: 'italic',
			label: 'I',
			title: 'Italic (Ctrl+I)',
			group: 'formatting',
			action: (ctx) => ctx.toggleMark('italic'),
			isActive: (ctx) => ctx.isMarkActive('italic')
		},
		{
			id: 'strikethrough',
			label: 'S',
			title: 'Strikethrough',
			group: 'formatting',
			action: (ctx) => ctx.toggleMark('strikethrough'),
			isActive: (ctx) => ctx.isMarkActive('strikethrough')
		}
	],

	inputRules: [
		// Bold: **text**
		{
			id: 'bold-asterisk',
			pattern: /\*\*([^*]+)\*\*$/,
			handler: (ctx, match) => {
				// Would replace **text** with bold formatted text
				// Implementation depends on document model
				return false;
			}
		},
		// Italic: *text*
		{
			id: 'italic-asterisk',
			pattern: /(?<!\*)\*([^*]+)\*$/,
			handler: (ctx, match) => {
				return false;
			}
		},
		// Strikethrough: ~~text~~
		{
			id: 'strikethrough',
			pattern: /~~([^~]+)~~$/,
			handler: (ctx, match) => {
				return false;
			}
		},
		// Inline code: `text`
		{
			id: 'inline-code',
			pattern: /`([^`]+)`$/,
			handler: (ctx, match) => {
				return false;
			}
		}
	]
};

export default formattingPlugin;
