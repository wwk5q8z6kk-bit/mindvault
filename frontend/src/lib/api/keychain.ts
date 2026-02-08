import { fetchJson, API_BASE_URL } from './client';

// ---------------------------------------------------------------------------
// Enums (match Rust #[serde(rename_all = "snake_case")])
// ---------------------------------------------------------------------------

export type VaultState = 'uninitialized' | 'sealed' | 'unsealed';
export type CredentialState = 'active' | 'expiring' | 'expired' | 'archived' | 'destroyed';
export type BreachAlertType =
	| 'unusual_frequency'
	| 'new_accessor'
	| 'off_hours_access'
	| 'rapid_sequential_access'
	| 'failed_access_spike';
export type BreachSeverity = 'low' | 'medium' | 'high' | 'critical';

// ---------------------------------------------------------------------------
// Response types (match Rust DTOs — snake_case fields, RFC 3339 dates, UUIDs as strings)
// ---------------------------------------------------------------------------

export interface VaultStatusResponse {
	state: string;
	key_epoch: number | null;
	credential_count: number | null;
	domain_count: number | null;
	created_at: string | null;
	last_rotated_at: string | null;
	auto_seal_remaining_secs: number | null;
}

export interface DomainKey {
	id: string;
	name: string;
	description: string | null;
	derivation_info: string;
	epoch: number;
	created_at: string;
	revoked_at: string | null;
	credential_count: number;
}

export interface StoredCredential {
	id: string;
	domain_id: string;
	name: string;
	description: string | null;
	kind: string;
	encrypted_value: string;
	derivation_info: string;
	epoch: number;
	state: CredentialState;
	tags: string[];
	metadata: Record<string, unknown>;
	created_at: string;
	updated_at: string;
	last_accessed_at: string;
	access_count: number;
	expires_at: string | null;
	archived_at: string | null;
	destroyed_at: string | null;
	delegation_id: string | null;
	version: number;
}

export interface Delegation {
	id: string;
	credential_id: string;
	delegatee: string;
	parent_id: string | null;
	permissions: DelegationPermissions;
	chain_hash: string;
	created_at: string;
	expires_at: string | null;
	revoked_at: string | null;
	max_depth: number;
	depth: number;
}

export interface DelegationPermissions {
	can_read: boolean;
	can_use: boolean;
	can_delegate: boolean;
}

export interface AccessProof {
	credential_id: string;
	challenge_nonce: string;
	proof: string;
	generated_at: string;
	expires_at: string;
}

export interface KeyEpoch {
	epoch: number;
	wrapped_key: string | null;
	created_at: string;
	grace_expires_at: string | null;
	retired_at: string | null;
}

export interface AuditEntry {
	id: string;
	sequence: number;
	action: string;
	subject: string;
	resource_id: string | null;
	details: unknown | null;
	entry_hash: string;
	previous_hash: string | null;
	timestamp: string;
	source_ip: string | null;
	signature: string | null;
}

export interface BreachAlert {
	id: string;
	credential_id: string;
	alert_type: BreachAlertType;
	severity: BreachSeverity;
	description: string;
	details: unknown | null;
	timestamp: string;
	acknowledged_at: string | null;
}

export interface ReadCredentialResponse {
	credential: StoredCredential;
	value: string;
}

// ---------------------------------------------------------------------------
// Well-known API keys (for quick-add chips)
// ---------------------------------------------------------------------------

export const WELL_KNOWN_API_KEYS = [
	{ name: 'OPENAI_API_KEY', label: 'OpenAI', description: 'Embeddings & LLM', required: true },
	{ name: 'ANTHROPIC_API_KEY', label: 'Anthropic', description: 'Claude LLM provider' },
	{ name: 'MINDVAULT_LLM_API_KEY', label: 'LLM Override', description: 'Custom LLM endpoint' },
	{
		name: 'MINDVAULT_EMBEDDING_API_KEY',
		label: 'Embedding Override',
		description: 'Custom embeddings'
	}
] as const;

// ---------------------------------------------------------------------------
// Vault
// ---------------------------------------------------------------------------

const KC = '/api/v1/keychain';

export async function initVault(
	password: string,
	macosBridge = false
): Promise<{ status: string }> {
	return fetchJson(`${KC}/init`, {
		method: 'POST',
		body: JSON.stringify({ password, macos_bridge: macosBridge })
	});
}

export async function unsealVault(opts: {
	password?: string;
	fromMacosKeychain?: boolean;
	fromSecureEnclave?: boolean;
}): Promise<{ status: string }> {
	return fetchJson(`${KC}/unseal`, {
		method: 'POST',
		body: JSON.stringify({
			password: opts.password,
			from_macos_keychain: opts.fromMacosKeychain ?? false,
			from_secure_enclave: opts.fromSecureEnclave ?? false
		})
	});
}

export async function sealVault(): Promise<{ status: string }> {
	return fetchJson(`${KC}/seal`, { method: 'POST' });
}

export async function vaultStatus(): Promise<VaultStatusResponse> {
	return fetchJson(`${KC}/status`);
}

export async function rotateKey(
	newPassword: string,
	graceHours = 24
): Promise<{ status: string }> {
	return fetchJson(`${KC}/rotate`, {
		method: 'POST',
		body: JSON.stringify({ new_password: newPassword, grace_hours: graceHours })
	});
}

export async function listEpochs(): Promise<KeyEpoch[]> {
	return fetchJson(`${KC}/epochs`);
}

// ---------------------------------------------------------------------------
// Domains
// ---------------------------------------------------------------------------

export async function createDomain(
	name: string,
	description?: string
): Promise<DomainKey> {
	return fetchJson(`${KC}/domains`, {
		method: 'POST',
		body: JSON.stringify({ name, description: description ?? null })
	});
}

export async function listDomains(): Promise<DomainKey[]> {
	return fetchJson(`${KC}/domains`);
}

export async function revokeDomain(id: string): Promise<{ status: string }> {
	return fetchJson(`${KC}/domains/${encodeURIComponent(id)}`, { method: 'DELETE' });
}

// ---------------------------------------------------------------------------
// Credentials
// ---------------------------------------------------------------------------

export async function storeCredential(opts: {
	domainId: string;
	name: string;
	kind: string;
	value: string;
	tags?: string[];
	expiresAt?: string;
}): Promise<StoredCredential> {
	return fetchJson(`${KC}/credentials`, {
		method: 'POST',
		body: JSON.stringify({
			domain_id: opts.domainId,
			name: opts.name,
			kind: opts.kind,
			value: opts.value,
			tags: opts.tags ?? [],
			expires_at: opts.expiresAt ?? null
		})
	});
}

export async function listCredentials(opts?: {
	domainId?: string;
	state?: CredentialState;
	limit?: number;
	offset?: number;
}): Promise<StoredCredential[]> {
	const params = new URLSearchParams();
	if (opts?.domainId) params.set('domain_id', opts.domainId);
	if (opts?.state) params.set('state', opts.state);
	if (opts?.limit != null) params.set('limit', String(opts.limit));
	if (opts?.offset != null) params.set('offset', String(opts.offset));
	const qs = params.toString();
	return fetchJson(`${KC}/credentials${qs ? `?${qs}` : ''}`);
}

export async function readCredential(id: string): Promise<ReadCredentialResponse> {
	return fetchJson(`${KC}/credentials/${encodeURIComponent(id)}`);
}

export async function updateCredential(
	id: string,
	value: string
): Promise<StoredCredential> {
	return fetchJson(`${KC}/credentials/${encodeURIComponent(id)}`, {
		method: 'PUT',
		body: JSON.stringify({ value })
	});
}

export async function archiveCredential(id: string): Promise<{ status: string }> {
	return fetchJson(`${KC}/credentials/${encodeURIComponent(id)}/archive`, {
		method: 'POST'
	});
}

export async function destroyCredential(id: string): Promise<{ status: string }> {
	return fetchJson(`${KC}/credentials/${encodeURIComponent(id)}`, {
		method: 'DELETE'
	});
}

// ---------------------------------------------------------------------------
// Delegations
// ---------------------------------------------------------------------------

export async function createDelegation(opts: {
	credentialId: string;
	delegatee: string;
	canRead?: boolean;
	canUse?: boolean;
	canDelegate?: boolean;
	expiresAt?: string;
	maxDepth?: number;
}): Promise<Delegation> {
	return fetchJson(`${KC}/delegations`, {
		method: 'POST',
		body: JSON.stringify({
			credential_id: opts.credentialId,
			delegatee: opts.delegatee,
			can_read: opts.canRead ?? true,
			can_use: opts.canUse ?? false,
			can_delegate: opts.canDelegate ?? false,
			expires_at: opts.expiresAt ?? null,
			max_depth: opts.maxDepth ?? 3
		})
	});
}

export async function listDelegations(credentialId: string): Promise<Delegation[]> {
	return fetchJson(
		`${KC}/delegations?credential_id=${encodeURIComponent(credentialId)}`
	);
}

export async function revokeDelegation(id: string): Promise<{ status: string }> {
	return fetchJson(`${KC}/delegations/${encodeURIComponent(id)}`, {
		method: 'DELETE'
	});
}

export async function subDelegate(
	parentId: string,
	opts: {
		delegatee: string;
		canRead?: boolean;
		canUse?: boolean;
		canDelegate?: boolean;
		expiresAt?: string;
	}
): Promise<Delegation> {
	return fetchJson(
		`${KC}/delegations/${encodeURIComponent(parentId)}/sub-delegate`,
		{
			method: 'POST',
			body: JSON.stringify({
				delegatee: opts.delegatee,
				can_read: opts.canRead ?? true,
				can_use: opts.canUse ?? false,
				can_delegate: opts.canDelegate ?? false,
				expires_at: opts.expiresAt ?? null
			})
		}
	);
}

// ---------------------------------------------------------------------------
// Proofs
// ---------------------------------------------------------------------------

export async function generateProof(
	credentialId: string,
	challengeNonce: string
): Promise<AccessProof> {
	return fetchJson(`${KC}/proof/generate`, {
		method: 'POST',
		body: JSON.stringify({
			credential_id: credentialId,
			challenge_nonce: challengeNonce
		})
	});
}

export async function verifyProof(opts: {
	credentialId: string;
	challengeNonce: string;
	proof: string;
	generatedAt: string;
	expiresAt: string;
}): Promise<{ valid: boolean }> {
	return fetchJson(`${KC}/proof/verify`, {
		method: 'POST',
		body: JSON.stringify({
			credential_id: opts.credentialId,
			challenge_nonce: opts.challengeNonce,
			proof: opts.proof,
			generated_at: opts.generatedAt,
			expires_at: opts.expiresAt
		})
	});
}

// ---------------------------------------------------------------------------
// Audit
// ---------------------------------------------------------------------------

export async function listAudit(opts?: {
	limit?: number;
	offset?: number;
}): Promise<AuditEntry[]> {
	const params = new URLSearchParams();
	if (opts?.limit != null) params.set('limit', String(opts.limit));
	if (opts?.offset != null) params.set('offset', String(opts.offset));
	const qs = params.toString();
	return fetchJson(`${KC}/audit${qs ? `?${qs}` : ''}`);
}

export async function verifyAuditIntegrity(): Promise<{ valid: boolean }> {
	return fetchJson(`${KC}/audit/verify`, { method: 'POST' });
}

// ---------------------------------------------------------------------------
// Alerts
// ---------------------------------------------------------------------------

export async function listAlerts(opts?: {
	limit?: number;
	offset?: number;
}): Promise<BreachAlert[]> {
	const params = new URLSearchParams();
	if (opts?.limit != null) params.set('limit', String(opts.limit));
	if (opts?.offset != null) params.set('offset', String(opts.offset));
	const qs = params.toString();
	return fetchJson(`${KC}/alerts${qs ? `?${qs}` : ''}`);
}

export async function acknowledgeAlert(id: string): Promise<{ status: string }> {
	return fetchJson(`${KC}/alerts/${encodeURIComponent(id)}/acknowledge`, {
		method: 'POST'
	});
}

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------

export async function runLifecycle(): Promise<{ transitioned: number }> {
	return fetchJson(`${KC}/lifecycle/run`, { method: 'POST' });
}

// ---------------------------------------------------------------------------
// Backup / Restore
// ---------------------------------------------------------------------------

export async function backupVault(password: string): Promise<Blob> {
	const controller = new AbortController();
	const timer = setTimeout(() => controller.abort(), 30000);
	try {
		const res = await fetch(`${API_BASE_URL}${KC}/backup`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ password }),
			signal: controller.signal
		});
		if (!res.ok) {
			const text = await res.text();
			throw new Error(`Backup failed (${res.status}): ${text}`);
		}
		return await res.blob();
	} finally {
		clearTimeout(timer);
	}
}

export async function restoreVault(
	data: string,
	password: string
): Promise<{ status: string }> {
	return fetchJson(`${KC}/restore`, {
		method: 'POST',
		body: JSON.stringify({ password, data })
	});
}
