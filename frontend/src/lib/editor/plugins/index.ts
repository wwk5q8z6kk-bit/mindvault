/**
 * Plugin system exports.
 */

// Types
export type {
	EditorPlugin,
	PluginContext,
	RegisteredPlugin,
	PluginRegistrationOptions,
	SlashItem,
	ToolbarItem,
	KeyboardShortcut,
	InputRule,
	CommandHandler,
	MarkdownExtension
} from './types';

// Registry
export { PluginRegistry, createPluginRegistry } from './registry';

// Context
export { createPluginContext, createNullContext } from './context';
export type { PluginContextOptions } from './context';

// Built-in plugins
export { formattingPlugin } from './builtin/formatting';
export { listsPlugin } from './builtin/lists';
export { blocksPlugin } from './builtin/blocks';
export { linksPlugin } from './builtin/links';
export { tablesPlugin } from './builtin/tables';

// Feature plugins
export { codeHighlightPlugin } from './code-highlight';
export { mentionsPlugin, onMentionStateChange, getMentionState, handleMentionSelect, handleMentionCreateNew, handleMentionClose } from './mentions';
export { mathPlugin, isKatexLoaded, ensureKatexLoaded, renderMath } from './math';
export { attachmentsPlugin, onAttachmentStateChange, getUploadState } from './attachments';
export { searchReplacePlugin, onSearchStateChange, getSearchState, openSearch, closeSearch, setSearchQuery, nextMatch, prevMatch } from './search-replace';
export { blockDndPlugin, onDragStateChange, getDragState, startDrag, endDrag, cancelDrag } from './block-dnd';
export { aiAssistPlugin, onAIAssistStateChange, getAIAssistState, acceptGhostCompletion, dismissGhostCompletion, showTransformMenu, hideTransformMenu, transformText, applyTransformResult } from './ai-assist';

// All built-in plugins as an array
import { formattingPlugin } from './builtin/formatting';
import { listsPlugin } from './builtin/lists';
import { blocksPlugin } from './builtin/blocks';
import { linksPlugin } from './builtin/links';
import { tablesPlugin } from './builtin/tables';
import { codeHighlightPlugin } from './code-highlight';
import { mentionsPlugin } from './mentions';
import { mathPlugin } from './math';
import { attachmentsPlugin } from './attachments';
import { searchReplacePlugin } from './search-replace';
import { blockDndPlugin } from './block-dnd';
import { aiAssistPlugin } from './ai-assist';

export const builtinPlugins = [
	formattingPlugin,
	listsPlugin,
	blocksPlugin,
	linksPlugin,
	tablesPlugin,
	codeHighlightPlugin,
	mentionsPlugin,
	mathPlugin,
	attachmentsPlugin,
	searchReplacePlugin,
	blockDndPlugin,
	aiAssistPlugin
];
