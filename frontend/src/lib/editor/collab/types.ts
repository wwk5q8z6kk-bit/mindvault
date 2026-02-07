/**
 * Collaboration types.
 * Foundation for real-time collaboration (not full CRDT).
 */

// ============================================================================
// Client Identity
// ============================================================================

export interface ClientInfo {
	/** Unique client identifier */
	clientId: string;
	/** User identifier (if authenticated) */
	userId?: string;
	/** Display name */
	name: string;
	/** Client color for cursors/selections */
	color: string;
	/** When this client joined */
	joinedAt: number;
	/** Last activity timestamp */
	lastActiveAt: number;
}

// ============================================================================
// Logical Timestamps
// ============================================================================

export interface VectorClock {
	[clientId: string]: number;
}

export interface Timestamp {
	/** The client that created this timestamp */
	clientId: string;
	/** Local sequence number */
	seq: number;
	/** Vector clock at time of creation */
	clock: VectorClock;
}

// ============================================================================
// Operations
// ============================================================================

export type OperationType =
	| 'insert'
	| 'delete'
	| 'format'
	| 'split'
	| 'merge'
	| 'setAttr'
	| 'cursor'
	| 'selection';

export interface CollabOperation {
	/** Unique operation ID */
	id: string;
	/** Operation type */
	type: OperationType;
	/** Logical timestamp */
	timestamp: Timestamp;
	/** Path to the target node */
	path: number[];
	/** Operation-specific data */
	data: unknown;
	/** Dependencies (operation IDs that must be applied first) */
	dependencies?: string[];
}

// ============================================================================
// Cursor/Selection
// ============================================================================

export interface CursorPosition {
	/** Path to the node */
	path: number[];
	/** Offset within the node */
	offset: number;
}

export interface RemoteCursor {
	clientId: string;
	clientInfo: ClientInfo;
	position: CursorPosition;
	selection?: {
		anchor: CursorPosition;
		focus: CursorPosition;
	};
}

// ============================================================================
// Sync Protocol
// ============================================================================

export type SyncMessageType =
	| 'join'
	| 'leave'
	| 'sync_request'
	| 'sync_response'
	| 'operation'
	| 'ack'
	| 'cursor_update'
	| 'awareness'
	| 'error';

export interface SyncMessage {
	type: SyncMessageType;
	clientId: string;
	documentId: string;
	timestamp: number;
	payload: unknown;
}

export interface JoinPayload {
	clientInfo: ClientInfo;
	documentVersion?: number;
}

export interface LeavePayload {
	reason?: string;
}

export interface SyncRequestPayload {
	fromVersion: number;
}

export interface SyncResponsePayload {
	version: number;
	operations: CollabOperation[];
	clients: ClientInfo[];
}

export interface OperationPayload {
	operation: CollabOperation;
}

export interface AckPayload {
	operationId: string;
	accepted: boolean;
	error?: string;
}

export interface CursorUpdatePayload {
	cursor: RemoteCursor;
}

export interface AwarenessPayload {
	clients: ClientInfo[];
}

export interface ErrorPayload {
	code: string;
	message: string;
}

// ============================================================================
// Connection State
// ============================================================================

export type ConnectionState =
	| 'disconnected'
	| 'connecting'
	| 'connected'
	| 'syncing'
	| 'ready'
	| 'error';

export interface CollabState {
	/** Current connection state */
	connectionState: ConnectionState;
	/** Local client info */
	localClient: ClientInfo | null;
	/** Connected remote clients */
	remoteClients: Map<string, ClientInfo>;
	/** Remote cursors */
	remoteCursors: Map<string, RemoteCursor>;
	/** Document version */
	documentVersion: number;
	/** Pending operations (not yet acknowledged) */
	pendingOperations: CollabOperation[];
	/** Error message if any */
	error?: string;
}

// ============================================================================
// Event Types
// ============================================================================

export type CollabEventType =
	| 'connection_change'
	| 'client_join'
	| 'client_leave'
	| 'operation_received'
	| 'operation_acknowledged'
	| 'cursor_update'
	| 'sync_complete'
	| 'error';

export interface CollabEvent {
	type: CollabEventType;
	data: unknown;
}

export type CollabEventHandler = (event: CollabEvent) => void;
