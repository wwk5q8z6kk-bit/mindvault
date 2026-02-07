/**
 * MindVault Editor - Main exports
 *
 * A modular, extensible rich text editor built from scratch.
 * No third-party editor dependencies (TipTap/ProseMirror/Slate/Quill).
 */

// Model
export * from './model';

// Selection
export * from './selection';

// History
export * from './history';

// Plugins
export * from './plugins';

// Markdown Parser
export * from './markdown';

// Collaboration (namespaced to avoid conflicts with model)
export {
	// Types
	type ClientInfo,
	type VectorClock,
	type Timestamp,
	type CollabOperation,
	type OperationType as CollabOperationType,
	type CursorPosition as CollabCursorPosition,
	type RemoteCursor,
	type SyncMessage,
	type SyncMessageType,
	type ConnectionState,
	type CollabState,
	type CollabEvent,
	type CollabEventType,
	type CollabEventHandler,
	// Functions
	generateClientId,
	generateOperationId,
	getClientColor,
	createClientInfo,
	createVectorClock,
	incrementClock,
	mergeClock,
	compareClock,
	createTimestamp,
	compareTimestamp,
	createOperation,
	serializeOperation,
	deserializeOperation,
	createCursorPosition as createCollabCursorPosition,
	compareCursorPosition as compareCollabCursorPosition,
	// Sync
	SyncManager,
	createSyncManager
} from './collab';
