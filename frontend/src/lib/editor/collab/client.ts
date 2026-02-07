/**
 * Collaboration client utilities.
 * Handles client identity, timestamps, and operation creation.
 */

import type {
	ClientInfo,
	VectorClock,
	Timestamp,
	CollabOperation,
	OperationType,
	CursorPosition
} from './types';

export type { VectorClock };

// ============================================================================
// Client ID Generation
// ============================================================================

/**
 * Generate a unique client ID.
 */
export function generateClientId(): string {
	const timestamp = Date.now().toString(36);
	const random = Math.random().toString(36).slice(2, 10);
	return `${timestamp}-${random}`;
}

/**
 * Generate a unique operation ID.
 */
export function generateOperationId(clientId: string, seq: number): string {
	return `${clientId}:${seq}`;
}

// ============================================================================
// Color Generation
// ============================================================================

const CURSOR_COLORS = [
	'#f87171', // red-400
	'#fb923c', // orange-400
	'#fbbf24', // amber-400
	'#a3e635', // lime-400
	'#4ade80', // green-400
	'#2dd4bf', // teal-400
	'#22d3ee', // cyan-400
	'#38bdf8', // sky-400
	'#818cf8', // indigo-400
	'#c084fc', // purple-400
	'#f472b6', // pink-400
	'#fb7185' // rose-400
];

/**
 * Get a consistent color for a client ID.
 */
export function getClientColor(clientId: string): string {
	// Simple hash function
	let hash = 0;
	for (let i = 0; i < clientId.length; i++) {
		hash = (hash << 5) - hash + clientId.charCodeAt(i);
		hash |= 0;
	}
	return CURSOR_COLORS[Math.abs(hash) % CURSOR_COLORS.length];
}

// ============================================================================
// Client Info
// ============================================================================

/**
 * Create client info for the local client.
 */
export function createClientInfo(
	clientId: string,
	name?: string,
	userId?: string
): ClientInfo {
	return {
		clientId,
		userId,
		name: name || `User ${clientId.slice(0, 4)}`,
		color: getClientColor(clientId),
		joinedAt: Date.now(),
		lastActiveAt: Date.now()
	};
}

// ============================================================================
// Vector Clock
// ============================================================================

/**
 * Create an empty vector clock.
 */
export function createVectorClock(): VectorClock {
	return {};
}

/**
 * Increment the clock for a client.
 */
export function incrementClock(clock: VectorClock, clientId: string): VectorClock {
	return {
		...clock,
		[clientId]: (clock[clientId] || 0) + 1
	};
}

/**
 * Merge two vector clocks (take max of each component).
 */
export function mergeClock(clock1: VectorClock, clock2: VectorClock): VectorClock {
	const merged: VectorClock = { ...clock1 };

	for (const [clientId, seq] of Object.entries(clock2)) {
		merged[clientId] = Math.max(merged[clientId] || 0, seq);
	}

	return merged;
}

/**
 * Compare two vector clocks.
 * Returns:
 *  -1 if clock1 < clock2 (clock1 happened before clock2)
 *   1 if clock1 > clock2 (clock1 happened after clock2)
 *   0 if concurrent (neither happened before the other)
 */
export function compareClock(clock1: VectorClock, clock2: VectorClock): -1 | 0 | 1 {
	let lessThan = false;
	let greaterThan = false;

	const allClients = new Set([...Object.keys(clock1), ...Object.keys(clock2)]);

	for (const clientId of allClients) {
		const seq1 = clock1[clientId] || 0;
		const seq2 = clock2[clientId] || 0;

		if (seq1 < seq2) lessThan = true;
		if (seq1 > seq2) greaterThan = true;
	}

	if (lessThan && !greaterThan) return -1;
	if (greaterThan && !lessThan) return 1;
	return 0;
}

// ============================================================================
// Timestamp
// ============================================================================

/**
 * Create a new timestamp.
 */
export function createTimestamp(clientId: string, clock: VectorClock): Timestamp {
	const seq = (clock[clientId] || 0) + 1;
	return {
		clientId,
		seq,
		clock: incrementClock(clock, clientId)
	};
}

/**
 * Compare two timestamps.
 */
export function compareTimestamp(ts1: Timestamp, ts2: Timestamp): -1 | 0 | 1 {
	// First compare by vector clock
	const clockComparison = compareClock(ts1.clock, ts2.clock);
	if (clockComparison !== 0) return clockComparison;

	// If concurrent, use client ID for deterministic ordering
	if (ts1.clientId < ts2.clientId) return -1;
	if (ts1.clientId > ts2.clientId) return 1;

	// Same client, compare sequence numbers
	if (ts1.seq < ts2.seq) return -1;
	if (ts1.seq > ts2.seq) return 1;

	return 0;
}

// ============================================================================
// Operation Creation
// ============================================================================

/**
 * Create a collaboration operation.
 */
export function createOperation(
	type: OperationType,
	clientId: string,
	clock: VectorClock,
	path: number[],
	data: unknown,
	dependencies?: string[]
): { operation: CollabOperation; newClock: VectorClock } {
	const timestamp = createTimestamp(clientId, clock);
	const operation: CollabOperation = {
		id: generateOperationId(clientId, timestamp.seq),
		type,
		timestamp,
		path,
		data,
		dependencies
	};

	return {
		operation,
		newClock: timestamp.clock
	};
}

// ============================================================================
// Operation Serialization
// ============================================================================

/**
 * Serialize an operation for transmission.
 */
export function serializeOperation(operation: CollabOperation): string {
	return JSON.stringify(operation);
}

/**
 * Deserialize an operation from transmission.
 */
export function deserializeOperation(data: string): CollabOperation {
	return JSON.parse(data) as CollabOperation;
}

// ============================================================================
// Cursor Position
// ============================================================================

/**
 * Create a cursor position.
 */
export function createCursorPosition(path: number[], offset: number): CursorPosition {
	return { path, offset };
}

/**
 * Compare two cursor positions.
 */
export function compareCursorPosition(pos1: CursorPosition, pos2: CursorPosition): -1 | 0 | 1 {
	// Compare paths first
	const minLen = Math.min(pos1.path.length, pos2.path.length);
	for (let i = 0; i < minLen; i++) {
		if (pos1.path[i] < pos2.path[i]) return -1;
		if (pos1.path[i] > pos2.path[i]) return 1;
	}

	// If paths are equal up to the shorter one, longer path comes after
	if (pos1.path.length < pos2.path.length) return -1;
	if (pos1.path.length > pos2.path.length) return 1;

	// Same path, compare offsets
	if (pos1.offset < pos2.offset) return -1;
	if (pos1.offset > pos2.offset) return 1;

	return 0;
}
