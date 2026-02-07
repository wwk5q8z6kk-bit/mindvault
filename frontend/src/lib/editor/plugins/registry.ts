/**
 * Plugin registry for managing editor plugins.
 * Handles registration, lifecycle, and event routing.
 */

import type {
	EditorPlugin,
	PluginContext,
	RegisteredPlugin,
	PluginRegistrationOptions,
	SlashItem,
	ToolbarItem,
	KeyboardShortcut,
	InputRule,
	CommandHandler
} from './types';
import type { EditorSelection } from '../model/types';

// ============================================================================
// Plugin Registry Class
// ============================================================================

/**
 * Registry that manages all editor plugins.
 */
export class PluginRegistry {
	private plugins: Map<string, RegisteredPlugin> = new Map();
	private context: PluginContext | null = null;
	private eventHandlers: Map<string, Set<string>> = new Map();

	/**
	 * Register a plugin.
	 */
	register(plugin: EditorPlugin, options: PluginRegistrationOptions = {}): void {
		if (this.plugins.has(plugin.id)) {
			console.warn(`Plugin "${plugin.id}" is already registered. Replacing.`);
			this.unregister(plugin.id);
		}

		const registered: RegisteredPlugin = {
			plugin,
			options: {
				priority: options.priority ?? 0,
				enabled: options.enabled ?? true
			},
			initialized: false
		};

		this.plugins.set(plugin.id, registered);

		// If context is already set, initialize immediately
		if (this.context && registered.options.enabled) {
			this.initializePlugin(registered);
		}
	}

	/**
	 * Register multiple plugins.
	 */
	registerAll(plugins: EditorPlugin[], options: PluginRegistrationOptions = {}): void {
		for (const plugin of plugins) {
			this.register(plugin, options);
		}
	}

	/**
	 * Unregister a plugin.
	 */
	unregister(pluginId: string): boolean {
		const registered = this.plugins.get(pluginId);
		if (!registered) return false;

		// Call destroy if initialized
		if (registered.initialized && registered.plugin.onDestroy) {
			try {
				registered.plugin.onDestroy();
			} catch (error) {
				console.error(`Error destroying plugin "${pluginId}":`, error);
			}
		}

		this.plugins.delete(pluginId);
		return true;
	}

	/**
	 * Get a registered plugin by ID.
	 */
	get(pluginId: string): EditorPlugin | null {
		return this.plugins.get(pluginId)?.plugin ?? null;
	}

	/**
	 * Check if a plugin is registered.
	 */
	has(pluginId: string): boolean {
		return this.plugins.has(pluginId);
	}

	/**
	 * Get all registered plugins.
	 */
	getAll(): EditorPlugin[] {
		return Array.from(this.plugins.values())
			.filter((r) => r.options.enabled)
			.sort((a, b) => (b.options.priority ?? 0) - (a.options.priority ?? 0))
			.map((r) => r.plugin);
	}

	/**
	 * Enable a plugin.
	 */
	enable(pluginId: string): boolean {
		const registered = this.plugins.get(pluginId);
		if (!registered) return false;

		registered.options.enabled = true;

		if (this.context && !registered.initialized) {
			this.initializePlugin(registered);
		}

		return true;
	}

	/**
	 * Disable a plugin.
	 */
	disable(pluginId: string): boolean {
		const registered = this.plugins.get(pluginId);
		if (!registered) return false;

		if (registered.initialized && registered.plugin.onDestroy) {
			try {
				registered.plugin.onDestroy();
			} catch (error) {
				console.error(`Error destroying plugin "${pluginId}":`, error);
			}
		}

		registered.options.enabled = false;
		registered.initialized = false;

		return true;
	}

	// --- Initialization ---

	/**
	 * Set the plugin context and initialize all enabled plugins.
	 */
	setContext(context: PluginContext): void {
		this.context = context;

		for (const registered of this.getEnabledPlugins()) {
			if (!registered.initialized) {
				this.initializePlugin(registered);
			}
		}
	}

	/**
	 * Initialize a single plugin.
	 */
	private initializePlugin(registered: RegisteredPlugin): void {
		if (!this.context) return;
		if (registered.initialized) return;

		// Check dependencies
		if (registered.plugin.dependencies) {
			for (const depId of registered.plugin.dependencies) {
				if (!this.has(depId)) {
					console.warn(
						`Plugin "${registered.plugin.id}" depends on "${depId}" which is not registered.`
					);
				}
			}
		}

		try {
			if (registered.plugin.onInit) {
				const result = registered.plugin.onInit(this.context);
				if (result instanceof Promise) {
					result.catch((error) => {
						console.error(`Error initializing plugin "${registered.plugin.id}":`, error);
					});
				}
			}
			registered.initialized = true;
		} catch (error) {
			console.error(`Error initializing plugin "${registered.plugin.id}":`, error);
		}
	}

	/**
	 * Get enabled plugins sorted by priority.
	 */
	private getEnabledPlugins(): RegisteredPlugin[] {
		return Array.from(this.plugins.values())
			.filter((r) => r.options.enabled)
			.sort((a, b) => (b.options.priority ?? 0) - (a.options.priority ?? 0));
	}

	// --- Event Routing ---

	/**
	 * Route a keydown event to plugins.
	 * Returns true if any plugin handled the event.
	 */
	handleKeyDown(event: KeyboardEvent): boolean {
		if (!this.context) return false;

		for (const registered of this.getEnabledPlugins()) {
			if (!registered.initialized) continue;

			// Check keyboard shortcuts first
			if (registered.plugin.keyboardShortcuts) {
				for (const shortcut of registered.plugin.keyboardShortcuts) {
					if (this.matchesShortcut(event, shortcut)) {
						const result = shortcut.handler(this.context, event);
						if (result === true) return true;
					}
				}
			}

			// Then check onKeyDown handler
			if (registered.plugin.onKeyDown) {
				try {
					const result = registered.plugin.onKeyDown(event, this.context);
					if (result === true) return true;
				} catch (error) {
					console.error(`Error in plugin "${registered.plugin.id}" keydown handler:`, error);
				}
			}
		}

		return false;
	}

	/**
	 * Check if an event matches a keyboard shortcut.
	 */
	private matchesShortcut(event: KeyboardEvent, shortcut: KeyboardShortcut): boolean {
		const isMod = event.metaKey || event.ctrlKey;

		if (shortcut.mod && !isMod) return false;
		if (shortcut.shift && !event.shiftKey) return false;
		if (shortcut.alt && !event.altKey) return false;

		return event.key.toLowerCase() === shortcut.key.toLowerCase();
	}

	/**
	 * Route a paste event to plugins.
	 */
	handlePaste(event: ClipboardEvent): boolean {
		if (!this.context) return false;

		for (const registered of this.getEnabledPlugins()) {
			if (!registered.initialized || !registered.plugin.onPaste) continue;

			try {
				const result = registered.plugin.onPaste(event, this.context);
				if (result === true) return true;
			} catch (error) {
				console.error(`Error in plugin "${registered.plugin.id}" paste handler:`, error);
			}
		}

		return false;
	}

	/**
	 * Route a drop event to plugins.
	 */
	handleDrop(event: DragEvent): boolean {
		if (!this.context) return false;

		for (const registered of this.getEnabledPlugins()) {
			if (!registered.initialized || !registered.plugin.onDrop) continue;

			try {
				const result = registered.plugin.onDrop(event, this.context);
				if (result === true) return true;
			} catch (error) {
				console.error(`Error in plugin "${registered.plugin.id}" drop handler:`, error);
			}
		}

		return false;
	}

	/**
	 * Notify plugins of document change.
	 */
	notifyDocumentChange(): void {
		if (!this.context) return;

		for (const registered of this.getEnabledPlugins()) {
			if (!registered.initialized || !registered.plugin.onDocumentChange) continue;

			try {
				registered.plugin.onDocumentChange(this.context);
			} catch (error) {
				console.error(`Error in plugin "${registered.plugin.id}" document change handler:`, error);
			}
		}
	}

	/**
	 * Notify plugins of selection change.
	 */
	notifySelectionChange(selection: EditorSelection | null): void {
		if (!this.context) return;

		for (const registered of this.getEnabledPlugins()) {
			if (!registered.initialized || !registered.plugin.onSelectionChange) continue;

			try {
				registered.plugin.onSelectionChange(this.context, selection);
			} catch (error) {
				console.error(`Error in plugin "${registered.plugin.id}" selection change handler:`, error);
			}
		}
	}

	// --- Aggregated Items ---

	/**
	 * Get all slash items from all plugins.
	 */
	getAllSlashItems(): SlashItem[] {
		const items: SlashItem[] = [];

		for (const registered of this.getEnabledPlugins()) {
			if (!registered.initialized || !registered.plugin.slashItems) continue;
			items.push(...registered.plugin.slashItems);
		}

		return items;
	}

	/**
	 * Get all toolbar items from all plugins.
	 */
	getAllToolbarItems(): ToolbarItem[] {
		const items: ToolbarItem[] = [];

		for (const registered of this.getEnabledPlugins()) {
			if (!registered.initialized || !registered.plugin.toolbarItems) continue;
			items.push(...registered.plugin.toolbarItems);
		}

		return items;
	}

	/**
	 * Get all input rules from all plugins.
	 */
	getAllInputRules(): InputRule[] {
		const rules: InputRule[] = [];

		for (const registered of this.getEnabledPlugins()) {
			if (!registered.initialized || !registered.plugin.inputRules) continue;
			rules.push(...registered.plugin.inputRules);
		}

		return rules;
	}

	/**
	 * Get all commands from all plugins.
	 */
	getAllCommands(): Map<string, CommandHandler> {
		const commands = new Map<string, CommandHandler>();

		for (const registered of this.getEnabledPlugins()) {
			if (!registered.initialized || !registered.plugin.commands) continue;

			for (const [name, handler] of Object.entries(registered.plugin.commands)) {
				const fullName = `${registered.plugin.id}:${name}`;
				commands.set(fullName, handler);
				// Also register without prefix for convenience
				if (!commands.has(name)) {
					commands.set(name, handler);
				}
			}
		}

		return commands;
	}

	/**
	 * Execute a command by name.
	 */
	executeCommand(name: string, ...args: unknown[]): boolean {
		if (!this.context) return false;

		const commands = this.getAllCommands();
		const handler = commands.get(name);

		if (!handler) {
			console.warn(`Command "${name}" not found.`);
			return false;
		}

		try {
			handler(this.context, ...args);
			return true;
		} catch (error) {
			console.error(`Error executing command "${name}":`, error);
			return false;
		}
	}

	/**
	 * Destroy all plugins and clean up.
	 */
	destroy(): void {
		for (const registered of this.plugins.values()) {
			if (registered.initialized && registered.plugin.onDestroy) {
				try {
					registered.plugin.onDestroy();
				} catch (error) {
					console.error(`Error destroying plugin "${registered.plugin.id}":`, error);
				}
			}
		}

		this.plugins.clear();
		this.context = null;
	}
}

// ============================================================================
// Factory Function
// ============================================================================

/**
 * Create a new plugin registry.
 */
export function createPluginRegistry(): PluginRegistry {
	return new PluginRegistry();
}
