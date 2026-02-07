/**
 * Collaboration module.
 *
 * Foundation for real-time collaboration:
 * - Unique client IDs
 * - Logical timestamps (vector clocks)
 * - Operation serialization
 * - WebSocket sync protocol
 *
 * Note: This is a foundation only, not a full CRDT implementation.
 * Full collaboration requires server-side support and conflict resolution.
 */

// Types
export type {
	ClientInfo,
	VectorClock,
	Timestamp,
	CollabOperation,
	OperationType,
	CursorPosition,
	RemoteCursor,
	SyncMessage,
	SyncMessageType,
	ConnectionState,
	CollabState,
	CollabEvent,
	CollabEventType,
	CollabEventHandler,
	JoinPayload,
	LeavePayload,
	SyncRequestPayload,
	SyncResponsePayload,
	OperationPayload,
	AckPayload,
	CursorUpdatePayload,
	AwarenessPayload,
	ErrorPayload
} from './types';

// Client utilities
export {
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
	createCursorPosition,
	compareCursorPosition
} from './client';

// Sync manager
export { SyncManager, createSyncManager } from './sync';
