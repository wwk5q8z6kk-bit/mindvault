import { fetchJson } from './client';

// --- Types (mirror Rust models) ---

export type TrustLevel = 'relay_only' | 'context_inject' | 'full';
export type ChannelType = 'direct' | 'group';
export type MessageDirection = 'inbound' | 'outbound';
export type MessageStatus = 'pending' | 'delivered' | 'read' | 'deferred' | 'auto_replied' | 'failed';
export type ContentType = 'text' | 'voice' | 'attachment';

export interface RelayContact {
	id: string;
	display_name: string;
	public_key: string;
	vault_address?: string;
	trust_level: TrustLevel;
	autonomy_rule_id?: string;
	notes?: string;
	created_at: string;
	updated_at?: string;
}

export interface RelayChannel {
	id: string;
	name?: string;
	channel_type: ChannelType;
	member_contact_ids: string[];
	created_at: string;
	updated_at?: string;
}

export interface RelayMessage {
	id: string;
	channel_id: string;
	thread_id?: string;
	sender_contact_id?: string;
	recipient_contact_id?: string;
	direction: MessageDirection;
	content: string;
	content_type: ContentType;
	status: MessageStatus;
	vault_node_id?: string | null;
	metadata: Record<string, unknown>;
	created_at: string;
	updated_at?: string;
}

export interface UnreadCountResponse {
	count: number;
}

// --- Contacts ---

export async function listContacts(): Promise<RelayContact[]> {
	return fetchJson<RelayContact[]>('/api/v1/relay/contacts');
}

export async function createContact(data: {
	display_name: string;
	public_key: string;
	vault_address?: string;
	trust_level?: TrustLevel;
	notes?: string;
}): Promise<RelayContact> {
	return fetchJson<RelayContact>('/api/v1/relay/contacts', {
		method: 'POST',
		body: JSON.stringify(data)
	});
}

export async function getContact(id: string): Promise<RelayContact> {
	return fetchJson<RelayContact>(`/api/v1/relay/contacts/${id}`);
}

export async function updateContact(
	id: string,
	data: {
		display_name?: string;
		vault_address?: string;
		trust_level?: TrustLevel;
		notes?: string;
	}
): Promise<RelayContact> {
	return fetchJson<RelayContact>(`/api/v1/relay/contacts/${id}`, {
		method: 'PUT',
		body: JSON.stringify(data)
	});
}

export async function deleteContact(id: string): Promise<void> {
	await fetchJson<void>(`/api/v1/relay/contacts/${id}`, { method: 'DELETE' });
}

// --- Channels ---

export async function listChannels(): Promise<RelayChannel[]> {
	return fetchJson<RelayChannel[]>('/api/v1/relay/channels');
}

export async function createChannel(data: {
	name?: string;
	channel_type?: ChannelType;
	member_contact_ids: string[];
}): Promise<RelayChannel> {
	return fetchJson<RelayChannel>('/api/v1/relay/channels', {
		method: 'POST',
		body: JSON.stringify(data)
	});
}

export async function deleteChannel(id: string): Promise<void> {
	await fetchJson<void>(`/api/v1/relay/channels/${id}`, { method: 'DELETE' });
}

// --- Messages ---

export async function listMessages(
	channelId: string,
	limit = 50,
	offset = 0
): Promise<RelayMessage[]> {
	return fetchJson<RelayMessage[]>(
		`/api/v1/relay/channels/${channelId}/messages?limit=${limit}&offset=${offset}`
	);
}

export async function sendMessage(
	channelId: string,
	content: string,
	opts?: { content_type?: ContentType; thread_id?: string }
): Promise<RelayMessage> {
	return fetchJson<RelayMessage>(`/api/v1/relay/channels/${channelId}/messages`, {
		method: 'POST',
		body: JSON.stringify({ content, ...opts })
	});
}

export async function markRead(messageId: string): Promise<void> {
	await fetchJson<void>(`/api/v1/relay/messages/${messageId}/read`, { method: 'POST' });
}

export async function getUnreadCount(channelId?: string): Promise<number> {
	const params = channelId ? `?channel_id=${channelId}` : '';
	const res = await fetchJson<UnreadCountResponse>(`/api/v1/relay/unread${params}`);
	return res.count;
}
