/**
 * Built-in links plugin.
 * Provides link insertion and wiki-link support.
 */

import type { EditorPlugin, PluginContext, SlashItem, KeyboardShortcut } from '../types';
import { createImage } from '../../model/document';

/**
 * Links plugin provides regular links and wiki-links.
 */
export const linksPlugin: EditorPlugin = {
	id: 'links',
	name: 'Links',
	description: 'Links and wiki-links',
	version: '1.0.0',

	commands: {
		insertLink: (ctx: PluginContext, ...args: unknown[]) => {
			const [url, text] = args as [string | undefined, string | undefined];
			if (url) {
				ctx.toggleMark('link', { href: url });
			} else {
				ctx.emit('showLinkModal');
			}
		},
		insertWikiLink: (ctx: PluginContext, ...args: unknown[]) => {
			const [target, alias] = args as [string | undefined, string | undefined];
			if (target) {
				ctx.toggleMark('wikilink', { target, alias });
			} else {
				ctx.emit('showWikiLinkSearch');
			}
		},
		insertImage: (ctx: PluginContext, ...args: unknown[]) => {
			const [src, alt] = args as [string | undefined, string | undefined];
			if (src) {
				const img = createImage(src, alt ?? '');
				ctx.insertBlock(img);
			} else {
				ctx.emit('showImageModal');
			}
		}
	},

	keyboardShortcuts: [
		{
			key: 'k',
			mod: true,
			handler: (ctx) => {
				ctx.emit('showLinkModal');
				return true;
			}
		}
	],

	slashItems: [
		{
			id: 'link',
			label: 'Link',
			description: 'Insert hyperlink',
			icon: '🔗',
			keywords: ['link', 'url', 'href'],
			group: 'media',
			action: (ctx) => {
				ctx.emit('showLinkModal');
			}
		},
		{
			id: 'image',
			label: 'Image',
			description: 'Insert image from URL',
			icon: '🖼',
			keywords: ['image', 'picture', 'img'],
			group: 'media',
			action: (ctx) => {
				ctx.emit('showImageModal');
			}
		}
	],

	toolbarItems: [
		{
			id: 'link',
			label: 'Link',
			title: 'Insert link (Ctrl+K)',
			group: 'media',
			action: (ctx) => {
				ctx.emit('showLinkModal');
			},
			isActive: (ctx) => ctx.isMarkActive('link')
		}
	],

	inputRules: [
		// Wiki-link: [[target]] or [[target|alias]]
		{
			id: 'wiki-link',
			pattern: /\[\[([^\]|]+)(?:\|([^\]]+))?\]\]$/,
			handler: (ctx, match) => {
				const target = match[1];
				const alias = match[2];
				// Would replace the [[...]] with a wiki-link mark
				return false;
			}
		},
		// Link: [text](url)
		{
			id: 'markdown-link',
			pattern: /\[([^\]]+)\]\(([^)]+)\)$/,
			handler: (ctx, match) => {
				const text = match[1];
				const url = match[2];
				// Would replace with a link mark
				return false;
			}
		},
		// Image: ![alt](url)
		{
			id: 'markdown-image',
			pattern: /!\[([^\]]*)\]\(([^)]+)\)$/,
			handler: (ctx, match) => {
				const alt = match[1];
				const src = match[2];
				const img = createImage(src, alt);
				ctx.insertBlock(img);
				return true;
			}
		}
	],

	// Handle paste events for URLs
	onPaste(event: ClipboardEvent, ctx: PluginContext): boolean | void {
		const text = event.clipboardData?.getData('text/plain');
		if (!text) return;

		// Check if the pasted text is a URL
		const urlPattern = /^https?:\/\/[^\s]+$/i;
		if (urlPattern.test(text.trim())) {
			// If there's a text selection, convert it to a link
			if (ctx.hasTextSelection()) {
				event.preventDefault();
				ctx.toggleMark('link', { href: text.trim() });
				return true;
			}
		}

		// Check for image URLs
		const imagePattern = /^https?:\/\/[^\s]+\.(jpg|jpeg|png|gif|webp|svg)(\?[^\s]*)?$/i;
		if (imagePattern.test(text.trim())) {
			event.preventDefault();
			const img = createImage(text.trim(), '');
			ctx.insertBlock(img);
			return true;
		}
	}
};

export default linksPlugin;
