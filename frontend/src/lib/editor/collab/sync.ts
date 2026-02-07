/**
 * Collaboration sync protocol.
 * WebSocket-based synchronization stubs.
 */

import type {
	SyncMessage,
	SyncMessageType,
	ClientInfo,
	CollabOperation,
	CollabState,
	ConnectionState,
	CollabEvent,
	CollabEventHandler,
	RemoteCursor,
	JoinPayload,
	SyncRequestPayload,
	SyncResponsePayload,
	OperationPayload,
	AckPayload,
	CursorUpdatePayload,
	VectorClock
} from './types';
import {
	generateClientId,
	createClientInfo,
	createVectorClock,
	mergeClock
} from './client';

// ============================================================================
// Sync Manager
// ============================================================================

export class SyncManager {
	private ws: WebSocket | null = null;
	private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
	private reconnectAttempts = 0;
	private maxReconnectAttempts = 5;
	private reconnectDelay = 1000;

	private documentId: string;
	private serverUrl: string;
	private localClientId: string;
	private localClientInfo: ClientInfo;

	private clock: VectorClock = createVectorClock();
	private state: CollabState;
	private eventHandlers: Set<CollabEventHandler> = new Set();

	constructor(documentId: string, serverUrl: string, userName?: string, userId?: string) {
		this.documentId = documentId;
		this.serverUrl = serverUrl;
		this.localClientId = generateClientId();
		this.localClientInfo = createClientInfo(this.localClientId, userName, userId);

		this.state = {
			connectionState: 'disconnected',
			localClient: this.localClientInfo,
			remoteClients: new Map(),
			remoteCursors: new Map(),
			documentVersion: 0,
			pendingOperations: []
		};
	}

	/**
	 * Get the current state.
	 */
	getState(): CollabState {
		return { ...this.state };
	}

	/**
	 * Get the local client ID.
	 */
	getClientId(): string {
		return this.localClientId;
	}

	/**
	 * Get the current vector clock.
	 */
	getClock(): VectorClock {
		return { ...this.clock };
	}

	/**
	 * Subscribe to events.
	 */
	subscribe(handler: CollabEventHandler): () => void {
		this.eventHandlers.add(handler);
		return () => {
			this.eventHandlers.delete(handler);
		};
	}

	/**
	 * Emit an event to all handlers.
	 */
	private emit(type: CollabEvent['type'], data: unknown): void {
		const event: CollabEvent = { type, data };
		for (const handler of this.eventHandlers) {
			try {
				handler(event);
			} catch (error) {
				console.error('Collab event handler error:', error);
			}
		}
	}

	/**
	 * Update state and emit change event.
	 */
	private updateState(updates: Partial<CollabState>): void {
		this.state = { ...this.state, ...updates };
	}

	/**
	 * Set connection state.
	 */
	private setConnectionState(state: ConnectionState): void {
		this.updateState({ connectionState: state });
		this.emit('connection_change', { state });
	}

	/**
	 * Connect to the sync server.
	 */
	connect(): void {
		if (this.ws) {
			this.disconnect();
		}

		this.setConnectionState('connecting');

		try {
			const url = `${this.serverUrl}/collab/${this.documentId}`;
			this.ws = new WebSocket(url);

			this.ws.onopen = () => this.handleOpen();
			this.ws.onclose = (event) => this.handleClose(event);
			this.ws.onerror = (event) => this.handleError(event);
			this.ws.onmessage = (event) => this.handleMessage(event);
		} catch (error) {
			console.error('WebSocket connection error:', error);
			this.setConnectionState('error');
			this.scheduleReconnect();
		}
	}

	/**
	 * Disconnect from the sync server.
	 */
	disconnect(): void {
		if (this.reconnectTimer) {
			clearTimeout(this.reconnectTimer);
			this.reconnectTimer = null;
		}

		if (this.ws) {
			this.ws.close();
			this.ws = null;
		}

		this.setConnectionState('disconnected');
		this.reconnectAttempts = 0;
	}

	/**
	 * Handle WebSocket open.
	 */
	private handleOpen(): void {
		this.setConnectionState('connected');
		this.reconnectAttempts = 0;

		// Send join message
		this.sendMessage('join', {
			clientInfo: this.localClientInfo,
			documentVersion: this.state.documentVersion
		} as JoinPayload);

		// Request sync
		this.setConnectionState('syncing');
		this.sendMessage('sync_request', {
			fromVersion: this.state.documentVersion
		} as SyncRequestPayload);
	}

	/**
	 * Handle WebSocket close.
	 */
	private handleClose(event: CloseEvent): void {
		this.ws = null;

		if (event.wasClean) {
			this.setConnectionState('disconnected');
		} else {
			this.setConnectionState('error');
			this.scheduleReconnect();
		}
	}

	/**
	 * Handle WebSocket error.
	 */
	private handleError(event: Event): void {
		console.error('WebSocket error:', event);
		this.updateState({ error: 'Connection error' });
	}

	/**
	 * Handle incoming message.
	 */
	private handleMessage(event: MessageEvent): void {
		try {
			const message = JSON.parse(event.data) as SyncMessage;
			this.processMessage(message);
		} catch (error) {
			console.error('Failed to parse sync message:', error);
		}
	}

	/**
	 * Process a sync message.
	 */
	private processMessage(message: SyncMessage): void {
		switch (message.type) {
			case 'sync_response':
				this.handleSyncResponse(message.payload as SyncResponsePayload);
				break;

			case 'operation':
				this.handleRemoteOperation(message.payload as OperationPayload);
				break;

			case 'ack':
				this.handleAck(message.payload as AckPayload);
				break;

			case 'cursor_update':
				this.handleCursorUpdate(message.payload as CursorUpdatePayload);
				break;

			case 'join':
				this.handleClientJoin(message.payload as JoinPayload);
				break;

			case 'leave':
				this.handleClientLeave(message.clientId);
				break;

			case 'error':
				console.error('Sync error:', message.payload);
				this.updateState({ error: String((message.payload as { message: string }).message) });
				break;
		}
	}

	/**
	 * Handle sync response.
	 */
	private handleSyncResponse(payload: SyncResponsePayload): void {
		// Update version
		this.updateState({ documentVersion: payload.version });

		// Update remote clients
		for (const client of payload.clients) {
			if (client.clientId !== this.localClientId) {
				this.state.remoteClients.set(client.clientId, client);
			}
		}

		// Merge operations into local clock
		for (const op of payload.operations) {
			this.clock = mergeClock(this.clock, op.timestamp.clock);
		}

		this.setConnectionState('ready');
		this.emit('sync_complete', { version: payload.version, operations: payload.operations });
	}

	/**
	 * Handle remote operation.
	 */
	private handleRemoteOperation(payload: OperationPayload): void {
		const { operation } = payload;

		// Skip our own operations
		if (operation.timestamp.clientId === this.localClientId) {
			return;
		}

		// Merge clock
		this.clock = mergeClock(this.clock, operation.timestamp.clock);

		// Emit for local handling
		this.emit('operation_received', { operation });
	}

	/**
	 * Handle operation acknowledgment.
	 */
	private handleAck(payload: AckPayload): void {
		const { operationId, accepted, error } = payload;

		// Remove from pending
		this.updateState({
			pendingOperations: this.state.pendingOperations.filter((op) => op.id !== operationId)
		});

		this.emit('operation_acknowledged', { operationId, accepted, error });
	}

	/**
	 * Handle cursor update.
	 */
	private handleCursorUpdate(payload: CursorUpdatePayload): void {
		const { cursor } = payload;

		if (cursor.clientId !== this.localClientId) {
			this.state.remoteCursors.set(cursor.clientId, cursor);
			this.emit('cursor_update', { cursor });
		}
	}

	/**
	 * Handle client join.
	 */
	private handleClientJoin(payload: JoinPayload): void {
		const { clientInfo } = payload;

		if (clientInfo.clientId !== this.localClientId) {
			this.state.remoteClients.set(clientInfo.clientId, clientInfo);
			this.emit('client_join', { client: clientInfo });
		}
	}

	/**
	 * Handle client leave.
	 */
	private handleClientLeave(clientId: string): void {
		this.state.remoteClients.delete(clientId);
		this.state.remoteCursors.delete(clientId);
		this.emit('client_leave', { clientId });
	}

	/**
	 * Send a message to the server.
	 */
	private sendMessage(type: SyncMessageType, payload: unknown): void {
		if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
			console.warn('Cannot send message: WebSocket not connected');
			return;
		}

		const message: SyncMessage = {
			type,
			clientId: this.localClientId,
			documentId: this.documentId,
			timestamp: Date.now(),
			payload
		};

		this.ws.send(JSON.stringify(message));
	}

	/**
	 * Send a local operation.
	 */
	sendOperation(operation: CollabOperation): void {
		// Add to pending
		this.state.pendingOperations.push(operation);

		// Update local clock
		this.clock = mergeClock(this.clock, operation.timestamp.clock);

		// Send to server
		this.sendMessage('operation', { operation } as OperationPayload);
	}

	/**
	 * Send cursor update.
	 */
	sendCursorUpdate(cursor: RemoteCursor): void {
		this.sendMessage('cursor_update', { cursor } as CursorUpdatePayload);
	}

	/**
	 * Schedule a reconnection attempt.
	 */
	private scheduleReconnect(): void {
		if (this.reconnectAttempts >= this.maxReconnectAttempts) {
			console.error('Max reconnection attempts reached');
			return;
		}

		const delay = this.reconnectDelay * Math.pow(2, this.reconnectAttempts);
		this.reconnectAttempts++;

		this.reconnectTimer = setTimeout(() => {
			this.connect();
		}, delay);
	}
}

/**
 * Create a sync manager.
 */
export function createSyncManager(
	documentId: string,
	serverUrl: string,
	userName?: string,
	userId?: string
): SyncManager {
	return new SyncManager(documentId, serverUrl, userName, userId);
}
