import { writable } from 'svelte/store';
import { vaultStatus, type VaultState } from '$lib/api/keychain';

export interface KeychainState {
	state: VaultState;
	epoch: number | null;
	autoSealSecs: number | null;
	credentialCount: number | null;
	domainCount: number | null;
	degradedSecurity: boolean;
	loading: boolean;
}

const initial: KeychainState = {
	state: 'uninitialized',
	epoch: null,
	autoSealSecs: null,
	credentialCount: null,
	domainCount: null,
	degradedSecurity: false,
	loading: false
};

export const keychainStore = writable<KeychainState>(initial);

export async function pollVaultStatus(): Promise<void> {
	keychainStore.update((s) => ({ ...s, loading: true }));
	try {
		const res = await vaultStatus();
		keychainStore.set({
			state: (res.state as VaultState) ?? 'uninitialized',
			epoch: res.key_epoch,
			autoSealSecs: res.auto_seal_remaining_secs,
			credentialCount: res.credential_count,
			domainCount: res.domain_count,
			degradedSecurity: res.degraded_security ?? false,
			loading: false
		});
	} catch {
		keychainStore.update((s) => ({ ...s, loading: false }));
	}
}
