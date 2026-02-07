/**
 * Error boundary utilities for the plugin system.
 * Provides graceful error handling and recovery.
 */

import type { EditorPlugin, PluginContext } from './types';

// ============================================================================
// Error Types
// ============================================================================

export class PluginError extends Error {
	constructor(
		public readonly pluginId: string,
		public readonly phase: 'init' | 'destroy' | 'event' | 'command',
		message: string,
		public readonly cause?: unknown
	) {
		super(`[Plugin:${pluginId}] ${phase}: ${message}`);
		this.name = 'PluginError';
	}
}

// ============================================================================
// Error Handler
// ============================================================================

export type ErrorHandler = (error: PluginError) => void;

let globalErrorHandler: ErrorHandler = (error) => {
	console.error(error.message, error.cause);
};

/**
 * Set the global error handler for plugin errors.
 */
export function setPluginErrorHandler(handler: ErrorHandler): void {
	globalErrorHandler = handler;
}

/**
 * Report a plugin error.
 */
export function reportPluginError(error: PluginError): void {
	try {
		globalErrorHandler(error);
	} catch (handlerError) {
		console.error('Error in plugin error handler:', handlerError);
	}
}

// ============================================================================
// Safe Plugin Wrapper
// ============================================================================

/**
 * Wrap a plugin to catch and handle errors gracefully.
 */
export function createSafePlugin(plugin: EditorPlugin): EditorPlugin {
	const wrapHandler = <T extends (...args: never[]) => unknown>(
		fn: T | undefined,
		phase: PluginError['phase']
	): T | undefined => {
		if (!fn) return undefined;

		const wrapped = (...args: Parameters<T>): ReturnType<T> | undefined => {
			try {
				const result = fn(...args);
				if (result instanceof Promise) {
					return result.catch((error) => {
						reportPluginError(new PluginError(plugin.id, phase, String(error), error));
					}) as ReturnType<T>;
				}
				return result as ReturnType<T>;
			} catch (error) {
				reportPluginError(new PluginError(plugin.id, phase, String(error), error));
				return undefined;
			}
		};

		return wrapped as T;
	};

	const safePlugin: EditorPlugin = {
		...plugin
	};

	if (plugin.onInit) {
		safePlugin.onInit = wrapHandler(plugin.onInit.bind(plugin), 'init');
	}
	if (plugin.onDestroy) {
		safePlugin.onDestroy = wrapHandler(plugin.onDestroy.bind(plugin), 'destroy');
	}
	if (plugin.onKeyDown) {
		safePlugin.onKeyDown = wrapHandler(plugin.onKeyDown.bind(plugin), 'event');
	}
	if (plugin.onKeyUp) {
		safePlugin.onKeyUp = wrapHandler(plugin.onKeyUp.bind(plugin), 'event');
	}
	if (plugin.onPaste) {
		safePlugin.onPaste = wrapHandler(plugin.onPaste.bind(plugin), 'event');
	}
	if (plugin.onDrop) {
		safePlugin.onDrop = wrapHandler(plugin.onDrop.bind(plugin), 'event');
	}
	if (plugin.onClick) {
		safePlugin.onClick = wrapHandler(plugin.onClick.bind(plugin), 'event');
	}
	if (plugin.onFocus) {
		safePlugin.onFocus = wrapHandler(plugin.onFocus.bind(plugin), 'event');
	}
	if (plugin.onBlur) {
		safePlugin.onBlur = wrapHandler(plugin.onBlur.bind(plugin), 'event');
	}
	if (plugin.onDocumentChange) {
		safePlugin.onDocumentChange = wrapHandler(plugin.onDocumentChange.bind(plugin), 'event');
	}
	if (plugin.onSelectionChange) {
		safePlugin.onSelectionChange = wrapHandler(plugin.onSelectionChange.bind(plugin), 'event');
	}

	return safePlugin;
}

// ============================================================================
// Recovery Utilities
// ============================================================================

/**
 * State for tracking plugin health.
 */
interface PluginHealth {
	pluginId: string;
	errorCount: number;
	lastError?: PluginError;
	disabled: boolean;
}

const pluginHealth: Map<string, PluginHealth> = new Map();

/**
 * Get the health status of a plugin.
 */
export function getPluginHealth(pluginId: string): PluginHealth | undefined {
	return pluginHealth.get(pluginId);
}

/**
 * Get all unhealthy plugins.
 */
export function getUnhealthyPlugins(): PluginHealth[] {
	return Array.from(pluginHealth.values()).filter((h) => h.errorCount > 0);
}

/**
 * Record an error for a plugin.
 */
export function recordPluginError(pluginId: string, error: PluginError): void {
	const health = pluginHealth.get(pluginId) || {
		pluginId,
		errorCount: 0,
		disabled: false
	};

	health.errorCount++;
	health.lastError = error;

	// Auto-disable after too many errors
	if (health.errorCount >= 5) {
		health.disabled = true;
		console.warn(`Plugin ${pluginId} disabled due to repeated errors`);
	}

	pluginHealth.set(pluginId, health);
}

/**
 * Reset the health status of a plugin.
 */
export function resetPluginHealth(pluginId: string): void {
	pluginHealth.delete(pluginId);
}

/**
 * Check if a plugin is healthy.
 */
export function isPluginHealthy(pluginId: string): boolean {
	const health = pluginHealth.get(pluginId);
	return !health || (!health.disabled && health.errorCount < 3);
}

// ============================================================================
// Debug Utilities
// ============================================================================

/**
 * Create a debug wrapper that logs all plugin method calls.
 * Note: This uses any types internally for simplicity, but maintains type safety at the API level.
 */
export function createDebugPlugin(plugin: EditorPlugin, verbose = false): EditorPlugin {
	const log = (method: string, args: unknown[]) => {
		if (verbose) {
			console.debug(`[Plugin:${plugin.id}] ${method}`, args);
		}
	};

	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	const wrapMethod = (fn: ((...args: any[]) => any) | undefined, name: string) => {
		if (!fn) return undefined;

		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		return (...args: any[]) => {
			log(name, args);
			const start = performance.now();
			const result = fn(...args);
			const duration = performance.now() - start;

			if (duration > 16) {
				console.warn(`[Plugin:${plugin.id}] ${name} took ${duration.toFixed(2)}ms`);
			}

			return result;
		};
	};

	return {
		...plugin,
		onInit: wrapMethod(plugin.onInit?.bind(plugin), 'onInit'),
		onDestroy: wrapMethod(plugin.onDestroy?.bind(plugin), 'onDestroy'),
		onKeyDown: wrapMethod(plugin.onKeyDown?.bind(plugin), 'onKeyDown'),
		onKeyUp: wrapMethod(plugin.onKeyUp?.bind(plugin), 'onKeyUp'),
		onPaste: wrapMethod(plugin.onPaste?.bind(plugin), 'onPaste'),
		onDrop: wrapMethod(plugin.onDrop?.bind(plugin), 'onDrop'),
		onClick: wrapMethod(plugin.onClick?.bind(plugin), 'onClick'),
		onFocus: wrapMethod(plugin.onFocus?.bind(plugin), 'onFocus'),
		onBlur: wrapMethod(plugin.onBlur?.bind(plugin), 'onBlur'),
		onDocumentChange: wrapMethod(plugin.onDocumentChange?.bind(plugin), 'onDocumentChange'),
		onSelectionChange: wrapMethod(plugin.onSelectionChange?.bind(plugin), 'onSelectionChange')
	} as EditorPlugin;
}
