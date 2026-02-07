/**
 * Built-in tables plugin.
 * Provides table creation and manipulation.
 */

import type { EditorPlugin, PluginContext, SlashItem } from '../types';
import { createTable, createTableRow, createTableCell } from '../../model/document';

/**
 * Tables plugin provides table creation and basic table operations.
 */
export const tablesPlugin: EditorPlugin = {
	id: 'tables',
	name: 'Tables',
	description: 'Table creation and manipulation',
	version: '1.0.0',

	commands: {
		insertTable: (ctx: PluginContext, rows = 3, cols = 3) => {
			const table = createTable(rows as number, cols as number);
			ctx.insertBlock(table);
		},
		insertRow: (ctx: PluginContext) => {
			// Would insert a row at the current position
			ctx.emit('tableOperation', { type: 'insertRow' });
		},
		insertColumn: (ctx: PluginContext) => {
			// Would insert a column at the current position
			ctx.emit('tableOperation', { type: 'insertColumn' });
		},
		deleteRow: (ctx: PluginContext) => {
			// Would delete the current row
			ctx.emit('tableOperation', { type: 'deleteRow' });
		},
		deleteColumn: (ctx: PluginContext) => {
			// Would delete the current column
			ctx.emit('tableOperation', { type: 'deleteColumn' });
		},
		deleteTable: (ctx: PluginContext) => {
			// Would delete the entire table
			ctx.emit('tableOperation', { type: 'deleteTable' });
		}
	},

	slashItems: [
		{
			id: 'table-3x3',
			label: 'Table (3x3)',
			description: 'Insert a 3x3 table',
			icon: '⊞',
			keywords: ['table', 'grid', '3x3'],
			group: 'tables',
			action: (ctx) => {
				const table = createTable(3, 3);
				ctx.insertBlock(table);
			}
		},
		{
			id: 'table-2x2',
			label: 'Table (2x2)',
			description: 'Insert a 2x2 table',
			icon: '⊞',
			keywords: ['table', 'grid', '2x2'],
			group: 'tables',
			action: (ctx) => {
				const table = createTable(2, 2);
				ctx.insertBlock(table);
			}
		},
		{
			id: 'table-4x4',
			label: 'Table (4x4)',
			description: 'Insert a 4x4 table',
			icon: '⊞',
			keywords: ['table', 'grid', '4x4'],
			group: 'tables',
			action: (ctx) => {
				const table = createTable(4, 4);
				ctx.insertBlock(table);
			}
		}
	],

	// Handle keyboard navigation within tables
	onKeyDown(event: KeyboardEvent, ctx: PluginContext): boolean | void {
		const blockType = ctx.getCurrentBlockType();

		// Only handle events when in a table context
		if (blockType !== 'table' && blockType !== 'table-row' && blockType !== 'table-cell') {
			return;
		}

		// Tab to move to next cell
		if (event.key === 'Tab') {
			event.preventDefault();
			if (event.shiftKey) {
				ctx.emit('tableOperation', { type: 'moveToPrevCell' });
			} else {
				ctx.emit('tableOperation', { type: 'moveToNextCell' });
			}
			return true;
		}

		// Enter to move to next row
		if (event.key === 'Enter' && !event.shiftKey) {
			event.preventDefault();
			ctx.emit('tableOperation', { type: 'moveToNextRow' });
			return true;
		}

		// Arrow keys for cell navigation
		if (event.key === 'ArrowUp' || event.key === 'ArrowDown') {
			// Only intercept if at start/end of cell
			const sel = ctx.getSelection();
			if (!sel) return;

			event.preventDefault();
			if (event.key === 'ArrowUp') {
				ctx.emit('tableOperation', { type: 'moveToPrevRow' });
			} else {
				ctx.emit('tableOperation', { type: 'moveToNextRow' });
			}
			return true;
		}
	},

	inputRules: [
		// Markdown table header: | Header 1 | Header 2 |
		{
			id: 'table-start',
			pattern: /^\|([^|]+\|)+\s*$/,
			handler: (ctx, match) => {
				// Would start table creation
				return false;
			}
		}
	]
};

export default tablesPlugin;
