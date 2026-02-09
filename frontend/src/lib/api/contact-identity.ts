import { fetchJson } from './client';

export type IdentityType = 'email' | 'public_key' | 'oauth' | 'phone';

export interface ContactIdentity {
	id: string;
	contact_id: string;
	identity_type: IdentityType;
	identity_value: string;
	verified: boolean;
	verified_at: string | null;
	created_at: string;
}

export interface TrustModel {
	contact_id: string;
	can_query: boolean;
	can_inject_context: boolean;
	can_auto_reply: boolean;
	allowed_namespaces: string[];
	max_confidence_override: number | null;
	updated_at: string;
}

export interface AddIdentityRequest {
	identity_type: IdentityType;
	identity_value: string;
}

export interface SetTrustModelRequest {
	can_query?: boolean;
	can_inject_context?: boolean;
	can_auto_reply?: boolean;
	allowed_namespaces?: string[];
	max_confidence_override?: number | null;
}

/** List identities for a contact. */
export async function listContactIdentities(contactId: string): Promise<ContactIdentity[]> {
	return fetchJson<ContactIdentity[]>(`/api/v1/relay/contacts/${contactId}/identities`);
}

/** Add an identity to a contact. */
export async function addContactIdentity(
	contactId: string,
	req: AddIdentityRequest
): Promise<ContactIdentity> {
	return fetchJson<ContactIdentity>(`/api/v1/relay/contacts/${contactId}/identities`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify(req)
	});
}

/** Delete a contact identity. */
export async function deleteContactIdentity(id: string): Promise<{ deleted: boolean }> {
	return fetchJson<{ deleted: boolean }>(`/api/v1/relay/contacts/identities/${id}`, {
		method: 'DELETE'
	});
}

/** Verify a contact identity. */
export async function verifyContactIdentity(id: string): Promise<{ verified: boolean }> {
	return fetchJson<{ verified: boolean }>(`/api/v1/relay/contacts/identities/${id}/verify`, {
		method: 'POST'
	});
}

/** Get trust model for a contact. */
export async function getTrustModel(contactId: string): Promise<TrustModel> {
	return fetchJson<TrustModel>(`/api/v1/relay/contacts/${contactId}/trust`);
}

/** Update trust model for a contact. */
export async function setTrustModel(
	contactId: string,
	req: SetTrustModelRequest
): Promise<TrustModel> {
	return fetchJson<TrustModel>(`/api/v1/relay/contacts/${contactId}/trust`, {
		method: 'PUT',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify(req)
	});
}
