/**
 * Plugin system types for the MindVault editor.
 * Defines the EditorPlugin interface and related types.
 */

import type { EditorSelection, EditorNode, Mark, MarkType, BlockType } from '../model/types';
import type { Document } from '../model/document';
import type { HistoryStack } from '../history/stack';
import type { SelectionManager } from '../selection/manager';

// ============================================================================
// Plugin Context
// ============================================================================

/**
 * Context object passed to plugins, providing access to editor functionality.
 */
export interface PluginContext {
	/** The document model */
	document: Document;
	/** History stack for undo/redo */
	history: HistoryStack;
	/** Selection manager */
	selection: SelectionManager;
	/** The contenteditable element */
	editorElement: HTMLElement;

	// --- Selection Operations ---

	/** Get the current selection */
	getSelection(): EditorSelection | null;
	/** Set the selection */
	setSelection(selection: EditorSelection | null): void;
	/** Check if there's a non-collapsed selection */
	hasTextSelection(): boolean;

	// --- Document Operations ---

	/** Get a node by path */
	getNodeAtPath(path: number[]): EditorNode | null;
	/** Insert text at the current cursor */
	insertText(text: string): void;
	/** Delete the selection or character before cursor */
	deleteSelection(): void;
	/** Insert a block node at the current position */
	insertBlock(node: EditorNode): void;
	/** Replace the current block with another block type */
	setBlockType(type: BlockType, attrs?: Record<string, unknown>): void;
	/** Toggle a mark on the selection */
	toggleMark(type: MarkType, attrs?: Record<string, unknown>): void;
	/** Check if a mark is active at the current selection */
	isMarkActive(type: MarkType): boolean;
	/** Get the current block type at cursor */
	getCurrentBlockType(): BlockType | null;

	// --- History Operations ---

	/** Undo the last operation */
	undo(): boolean;
	/** Redo the last undone operation */
	redo(): boolean;

	// --- View Operations ---

	/** Focus the editor */
	focus(): void;
	/** Blur the editor */
	blur(): void;
	/** Scroll to a position */
	scrollToSelection(): void;

	// --- State Access ---

	/** Check if the editor is empty */
	isEmpty(): boolean;
	/** Get the markdown content */
	getMarkdown(): string;
	/** Set the markdown content */
	setMarkdown(markdown: string): void;
	/** Get the word count */
	getWordCount(): number;

	// --- Events ---

	/** Emit a custom event */
	emit(event: string, data?: unknown): void;
	/** Trigger a document change event */
	triggerChange(): void;
}

// ============================================================================
// Slash Command
// ============================================================================

/**
 * A slash command item that appears in the slash menu.
 */
export interface SlashItem {
	/** Unique identifier */
	id: string;
	/** Display label */
	label: string;
	/** Description shown in the menu */
	description: string;
	/** Optional icon (emoji or icon class) */
	icon?: string;
	/** Keywords for filtering */
	keywords?: string[];
	/** Group for organization */
	group?: string;
	/** Handler when selected */
	action: (ctx: PluginContext) => void | Promise<void>;
}

// ============================================================================
// Toolbar
// ============================================================================

/**
 * A toolbar item configuration.
 */
export interface ToolbarItem {
	/** Unique identifier */
	id: string;
	/** Display label or icon */
	label: string;
	/** Tooltip text */
	title?: string;
	/** Icon component or class */
	icon?: string;
	/** Group for organization */
	group?: string;
	/** Handler when clicked */
	action: (ctx: PluginContext) => void;
	/** Check if the item should appear active */
	isActive?: (ctx: PluginContext) => boolean;
	/** Check if the item should be disabled */
	isDisabled?: (ctx: PluginContext) => boolean;
}

// ============================================================================
// Keyboard Shortcuts
// ============================================================================

/**
 * Keyboard shortcut definition.
 */
export interface KeyboardShortcut {
	/** Key code (e.g., 'b', 'Enter', 'Tab') */
	key: string;
	/** Require Ctrl/Cmd modifier */
	mod?: boolean;
	/** Require Shift modifier */
	shift?: boolean;
	/** Require Alt modifier */
	alt?: boolean;
	/** Handler - return true to prevent default */
	handler: (ctx: PluginContext, event: KeyboardEvent) => boolean | void;
}

// ============================================================================
// Commands
// ============================================================================

/**
 * A command that can be executed by name.
 */
export type CommandHandler = (ctx: PluginContext, ...args: unknown[]) => void | Promise<void>;

// ============================================================================
// Markdown Extension
// ============================================================================

/**
 * Extension for custom markdown syntax.
 */
export interface MarkdownExtension {
	/** Name of the extension */
	name: string;
	/** Parse markdown to nodes */
	parse?: (text: string) => EditorNode[] | null;
	/** Serialize nodes to markdown */
	serialize?: (node: EditorNode) => string | null;
}

// ============================================================================
// Input Rules
// ============================================================================

/**
 * An input rule that transforms text as you type.
 */
export interface InputRule {
	/** Unique identifier */
	id: string;
	/** Regex pattern to match (must end with $) */
	pattern: RegExp;
	/** Handler when pattern matches */
	handler: (ctx: PluginContext, match: RegExpMatchArray) => boolean | void;
}

// ============================================================================
// Plugin Interface
// ============================================================================

/**
 * The main plugin interface.
 * Plugins extend the editor with new functionality.
 */
export interface EditorPlugin {
	/** Unique plugin identifier */
	id: string;
	/** Human-readable name */
	name: string;
	/** Plugin description */
	description?: string;
	/** Plugin version */
	version?: string;
	/** Dependencies on other plugins */
	dependencies?: string[];

	// --- Lifecycle ---

	/** Called when the plugin is initialized */
	onInit?(ctx: PluginContext): void | Promise<void>;
	/** Called when the plugin is destroyed */
	onDestroy?(): void;
	/** Called when the document changes */
	onDocumentChange?(ctx: PluginContext): void;
	/** Called when selection changes */
	onSelectionChange?(ctx: PluginContext, selection: EditorSelection | null): void;

	// --- Event Handlers ---

	/** Handle keydown events - return true to prevent default */
	onKeyDown?(event: KeyboardEvent, ctx: PluginContext): boolean | void;
	/** Handle keyup events */
	onKeyUp?(event: KeyboardEvent, ctx: PluginContext): void;
	/** Handle paste events - return true to prevent default */
	onPaste?(event: ClipboardEvent, ctx: PluginContext): boolean | void;
	/** Handle drop events - return true to prevent default */
	onDrop?(event: DragEvent, ctx: PluginContext): boolean | void;
	/** Handle click events */
	onClick?(event: MouseEvent, ctx: PluginContext): void;
	/** Handle focus events */
	onFocus?(event: FocusEvent, ctx: PluginContext): void;
	/** Handle blur events */
	onBlur?(event: FocusEvent, ctx: PluginContext): void;

	// --- Extensions ---

	/** Commands provided by this plugin */
	commands?: Record<string, CommandHandler>;
	/** Slash menu items */
	slashItems?: SlashItem[];
	/** Toolbar items */
	toolbarItems?: ToolbarItem[];
	/** Keyboard shortcuts */
	keyboardShortcuts?: KeyboardShortcut[];
	/** Input rules for auto-formatting */
	inputRules?: InputRule[];
	/** Markdown parsing/serialization extension */
	markdownExtension?: MarkdownExtension;
}

// ============================================================================
// Plugin Registration
// ============================================================================

/**
 * Options for plugin registration.
 */
export interface PluginRegistrationOptions {
	/** Priority for event handling (higher = earlier) */
	priority?: number;
	/** Whether the plugin is enabled */
	enabled?: boolean;
}

/**
 * A registered plugin with metadata.
 */
export interface RegisteredPlugin {
	plugin: EditorPlugin;
	options: PluginRegistrationOptions;
	initialized: boolean;
}
