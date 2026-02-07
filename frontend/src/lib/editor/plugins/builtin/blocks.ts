/**
 * Built-in blocks plugin.
 * Provides block-level elements: headings, code blocks, blockquotes, etc.
 */

import type { EditorPlugin, PluginContext, SlashItem, ToolbarItem } from '../types';
import {
	createParagraph,
	createHeading,
	createCodeBlock,
	createBlockquote,
	createHorizontalRule,
	createTable,
	createImage
} from '../../model/document';

/**
 * Blocks plugin provides headings, code blocks, blockquotes, tables, and more.
 */
export const blocksPlugin: EditorPlugin = {
	id: 'blocks',
	name: 'Blocks',
	description: 'Block-level elements (headings, code, quotes, tables)',
	version: '1.0.0',

	commands: {
		setHeading1: (ctx: PluginContext) => ctx.setBlockType('heading', { level: 1 }),
		setHeading2: (ctx: PluginContext) => ctx.setBlockType('heading', { level: 2 }),
		setHeading3: (ctx: PluginContext) => ctx.setBlockType('heading', { level: 3 }),
		setHeading4: (ctx: PluginContext) => ctx.setBlockType('heading', { level: 4 }),
		setParagraph: (ctx: PluginContext) => ctx.setBlockType('paragraph'),
		insertCodeBlock: (ctx: PluginContext) => {
			const block = createCodeBlock();
			ctx.insertBlock(block);
		},
		insertBlockquote: (ctx: PluginContext) => {
			const block = createBlockquote();
			ctx.insertBlock(block);
		},
		insertHorizontalRule: (ctx: PluginContext) => {
			const block = createHorizontalRule();
			ctx.insertBlock(block);
		},
		insertTable: (ctx: PluginContext, rows = 3, cols = 3) => {
			const table = createTable(rows as number, cols as number);
			ctx.insertBlock(table);
		}
	},

	slashItems: [
		{
			id: 'h1',
			label: 'Heading 1',
			description: 'Large section heading',
			icon: 'H1',
			keywords: ['title', 'heading', 'h1'],
			group: 'headings',
			action: (ctx) => ctx.setBlockType('heading', { level: 1 })
		},
		{
			id: 'h2',
			label: 'Heading 2',
			description: 'Medium section heading',
			icon: 'H2',
			keywords: ['subtitle', 'heading', 'h2'],
			group: 'headings',
			action: (ctx) => ctx.setBlockType('heading', { level: 2 })
		},
		{
			id: 'h3',
			label: 'Heading 3',
			description: 'Small section heading',
			icon: 'H3',
			keywords: ['heading', 'h3'],
			group: 'headings',
			action: (ctx) => ctx.setBlockType('heading', { level: 3 })
		},
		{
			id: 'code',
			label: 'Code Block',
			description: 'Syntax-highlighted code',
			icon: '</>',
			keywords: ['code', 'pre', 'syntax'],
			group: 'blocks',
			action: (ctx) => {
				const block = createCodeBlock();
				ctx.insertBlock(block);
			}
		},
		{
			id: 'quote',
			label: 'Blockquote',
			description: 'Indented quote block',
			icon: '"',
			keywords: ['quote', 'blockquote', 'citation'],
			group: 'blocks',
			action: (ctx) => {
				const block = createBlockquote();
				ctx.insertBlock(block);
			}
		},
		{
			id: 'divider',
			label: 'Divider',
			description: 'Horizontal rule',
			icon: '—',
			keywords: ['hr', 'divider', 'separator', 'line'],
			group: 'blocks',
			action: (ctx) => {
				const block = createHorizontalRule();
				ctx.insertBlock(block);
			}
		},
		{
			id: 'table',
			label: 'Table',
			description: 'Insert 3x3 table',
			icon: '⊞',
			keywords: ['table', 'grid'],
			group: 'blocks',
			action: (ctx) => {
				const table = createTable(3, 3);
				ctx.insertBlock(table);
			}
		}
	],

	toolbarItems: [
		{
			id: 'blockquote',
			label: '" Quote',
			title: 'Blockquote',
			group: 'blocks',
			action: (ctx) => {
				const block = createBlockquote();
				ctx.insertBlock(block);
			},
			isActive: (ctx) => ctx.getCurrentBlockType() === 'blockquote'
		},
		{
			id: 'code-block',
			label: '</> Code',
			title: 'Code block',
			group: 'blocks',
			action: (ctx) => {
				const block = createCodeBlock();
				ctx.insertBlock(block);
			},
			isActive: (ctx) => ctx.getCurrentBlockType() === 'code-block'
		},
		{
			id: 'divider',
			label: '—',
			title: 'Horizontal rule',
			group: 'blocks',
			action: (ctx) => {
				const block = createHorizontalRule();
				ctx.insertBlock(block);
			}
		}
	],

	inputRules: [
		// Heading 1: "# " at start
		{
			id: 'heading-1',
			pattern: /^#\s$/,
			handler: (ctx) => {
				ctx.setBlockType('heading', { level: 1 });
				return true;
			}
		},
		// Heading 2: "## " at start
		{
			id: 'heading-2',
			pattern: /^##\s$/,
			handler: (ctx) => {
				ctx.setBlockType('heading', { level: 2 });
				return true;
			}
		},
		// Heading 3: "### " at start
		{
			id: 'heading-3',
			pattern: /^###\s$/,
			handler: (ctx) => {
				ctx.setBlockType('heading', { level: 3 });
				return true;
			}
		},
		// Blockquote: "> " at start
		{
			id: 'blockquote',
			pattern: /^>\s$/,
			handler: (ctx) => {
				ctx.setBlockType('blockquote');
				return true;
			}
		},
		// Code block: "```" at start
		{
			id: 'code-block',
			pattern: /^```$/,
			handler: (ctx) => {
				const block = createCodeBlock();
				ctx.insertBlock(block);
				return true;
			}
		},
		// Horizontal rule: "---" at start
		{
			id: 'horizontal-rule',
			pattern: /^---$/,
			handler: (ctx) => {
				const block = createHorizontalRule();
				ctx.insertBlock(block);
				return true;
			}
		}
	]
};

export default blocksPlugin;
