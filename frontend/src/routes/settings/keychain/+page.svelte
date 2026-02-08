<script lang="ts">
	import { onDestroy } from 'svelte';
	import { pushToast } from '$lib/stores/toast';
	import { keychainStore, pollVaultStatus } from '$lib/stores/keychain';
	import {
		initVault,
		unsealVault,
		sealVault,
		rotateKey,
		listEpochs,
		createDomain,
		listDomains,
		revokeDomain,
		storeCredential,
		listCredentials,
		readCredential,
		updateCredential,
		archiveCredential,
		destroyCredential,
		createDelegation,
		listDelegations,
		revokeDelegation,
		subDelegate,
		listAudit,
		verifyAuditIntegrity,
		listAlerts,
		acknowledgeAlert,
		runLifecycle,
		backupVault,
		restoreVault,
		WELL_KNOWN_API_KEYS,
		type DomainKey,
		type StoredCredential,
		type Delegation,
		type AuditEntry,
		type BreachAlert,
		type KeyEpoch,
		type CredentialState
	} from '$lib/api/keychain';

	// ── Tab state ────────────────────────────────────────────────
	type Tab = 'vault' | 'credentials' | 'domains' | 'delegations' | 'audit' | 'alerts' | 'backup';
	let activeTab: Tab = 'vault';
	const tabs: Array<{ key: Tab; label: string }> = [
		{ key: 'vault', label: 'Vault' },
		{ key: 'credentials', label: 'Credentials' },
		{ key: 'domains', label: 'Domains' },
		{ key: 'delegations', label: 'Delegations' },
		{ key: 'audit', label: 'Audit' },
		{ key: 'alerts', label: 'Alerts' },
		{ key: 'backup', label: 'Backup' }
	];

	// ── Vault tab ────────────────────────────────────────────────
	let initPassword = '';
	let initPasswordConfirm = '';
	let initMacosBridge = false;
	let initBusy = false;
	let showInitPassword = false;

	let unsealPassword = '';
	let unsealBusy = false;
	let showUnsealPassword = false;

	let rotateNewPassword = '';
	let rotateNewPasswordConfirm = '';
	let rotateGraceHours = '24';
	let rotateBusy = false;
	let showRotatePassword = false;
	let showRotateSection = false;

	let epochs: KeyEpoch[] = [];
	let epochsLoading = false;

	// ── Credentials tab ──────────────────────────────────────────
	let credentials: StoredCredential[] = [];
	let credsLoading = false;
	let credsDomainFilter = '';
	let credsStateFilter = '';

	let newCredDomainId = '';
	let newCredName = '';
	let newCredKind = 'api_key';
	let newCredValue = '';
	let newCredTags = '';
	let newCredExpires = '';
	let newCredBusy = false;
	let showNewCredValue = false;
	let showNewCredForm = false;

	let viewCredId = '';
	let viewCredData: { credential: StoredCredential; value: string } | null = null;
	let viewCredLoading = false;
	let showViewCredValue = false;

	let credSearchQuery = '';
	let editCredValue = '';
	let editingCredValue = false;
	let editCredBusy = false;
	let lifecycleBusy = false;

	$: now = new Date().toISOString().slice(0, 16);
	$: initPasswordsMatch = initPassword === initPasswordConfirm;
	$: rotatePasswordsMatch = rotateNewPassword === rotateNewPasswordConfirm;
	$: backupPasswordsMatch = backupPassword === backupPasswordConfirm;
	$: filteredCredentials = credSearchQuery
		? credentials.filter((c) => c.name.toLowerCase().includes(credSearchQuery.toLowerCase()))
		: credentials;

	// ── Domains tab ──────────────────────────────────────────────
	let domains: DomainKey[] = [];
	let domainsLoading = false;
	let newDomainName = '';
	let newDomainDesc = '';
	let newDomainBusy = false;

	// ── Delegations tab ──────────────────────────────────────────
	let delegations: Delegation[] = [];
	let delegationsLoading = false;
	let delegationCredId = '';
	let newDelCredId = '';
	let newDelDelegatee = '';
	let newDelCanRead = true;
	let newDelCanUse = false;
	let newDelCanDelegate = false;
	let newDelExpires = '';
	let newDelMaxDepth = '3';
	let newDelBusy = false;

	// ── Audit tab ────────────────────────────────────────────────
	let auditEntries: AuditEntry[] = [];
	let auditLoading = false;
	let auditOffset = 0;
	const auditLimit = 50;

	// ── Alerts tab ───────────────────────────────────────────────
	let alerts: BreachAlert[] = [];
	let alertsLoading = false;

	// ── Backup tab ───────────────────────────────────────────────
	let backupPassword = '';
	let backupPasswordConfirm = '';
	let backupBusy = false;
	let showBackupPassword = false;
	let restoreFile: File | null = null;
	let restorePassword = '';
	let restoreBusy = false;
	let showRestorePassword = false;
	let restoreFileInput: HTMLInputElement | null = null;

	// ── Auto-seal countdown ──────────────────────────────────────
	let sealCountdown: number | null = null;
	let countdownInterval: ReturnType<typeof setInterval> | null = null;

	function startCountdown(secs: number) {
		stopCountdown();
		sealCountdown = secs;
		countdownInterval = setInterval(() => {
			if (sealCountdown != null && sealCountdown > 0) {
				sealCountdown--;
			} else {
				stopCountdown();
				pushToast('Vault auto-sealed due to inactivity', 'warning');
				pollVaultStatus();
			}
		}, 1000);
	}

	function stopCountdown() {
		if (countdownInterval) {
			clearInterval(countdownInterval);
			countdownInterval = null;
		}
	}

	onDestroy(() => {
		stopCountdown();
	});

	function formatCountdown(secs: number): string {
		const m = Math.floor(secs / 60);
		const s = secs % 60;
		return `${m}:${String(s).padStart(2, '0')}`;
	}

	// ── Reactive: start countdown when vault unseals ─────────────
	$: if ($keychainStore.state === 'unsealed' && $keychainStore.autoSealSecs != null) {
		startCountdown($keychainStore.autoSealSecs);
	} else {
		stopCountdown();
		sealCountdown = null;
	}

	// ── Actions: Vault ───────────────────────────────────────────
	async function handleInit() {
		if (initPassword !== initPasswordConfirm) {
			pushToast('Passwords do not match', 'danger');
			return;
		}
		if (initPassword.length < 8) {
			pushToast('Password must be at least 8 characters', 'danger');
			return;
		}
		initBusy = true;
		try {
			await initVault(initPassword, initMacosBridge);
			pushToast('Vault initialized', 'success');
			initPassword = '';
			initPasswordConfirm = '';
			await pollVaultStatus();
		} catch (e: any) {
			pushToast(e?.message ?? 'Failed to initialize vault', 'danger');
		} finally {
			initBusy = false;
		}
	}

	async function handleUnseal(opts?: { macos?: boolean; enclave?: boolean }) {
		unsealBusy = true;
		try {
			await unsealVault({
				password: opts ? undefined : unsealPassword,
				fromMacosKeychain: opts?.macos,
				fromSecureEnclave: opts?.enclave
			});
			pushToast('Vault unsealed', 'success');
			unsealPassword = '';
			await pollVaultStatus();
		} catch (e: any) {
			pushToast(e?.message ?? 'Failed to unseal vault', 'danger');
		} finally {
			unsealBusy = false;
		}
	}

	async function handleSeal() {
		try {
			await sealVault();
			pushToast('Vault sealed', 'success');
			await pollVaultStatus();
		} catch (e: any) {
			pushToast(e?.message ?? 'Failed to seal vault', 'danger');
		}
	}

	async function handleRotate() {
		if (rotateNewPassword !== rotateNewPasswordConfirm) {
			pushToast('Passwords do not match', 'danger');
			return;
		}
		if (rotateNewPassword.length < 8) {
			pushToast('Password must be at least 8 characters', 'danger');
			return;
		}
		rotateBusy = true;
		try {
			await rotateKey(rotateNewPassword, parseInt(rotateGraceHours, 10) || 24);
			pushToast('Master key rotated', 'success');
			rotateNewPassword = '';
			rotateNewPasswordConfirm = '';
			showRotateSection = false;
			await pollVaultStatus();
			await loadEpochs();
		} catch (e: any) {
			pushToast(e?.message ?? 'Failed to rotate key', 'danger');
		} finally {
			rotateBusy = false;
		}
	}

	async function loadEpochs() {
		epochsLoading = true;
		try {
			epochs = await listEpochs();
		} catch (e: any) {
			pushToast(e?.message ?? 'Failed to load key epochs', 'danger');
		} finally {
			epochsLoading = false;
		}
	}

	// ── Actions: Domains ─────────────────────────────────────────
	async function loadDomains() {
		domainsLoading = true;
		try {
			domains = await listDomains();
		} catch (e: any) {
			pushToast(e?.message ?? 'Failed to load domains', 'danger');
		} finally {
			domainsLoading = false;
		}
	}

	async function handleCreateDomain() {
		if (!newDomainName.trim()) return;
		newDomainBusy = true;
		try {
			await createDomain(newDomainName.trim(), newDomainDesc.trim() || undefined);
			pushToast(`Domain "${newDomainName}" created`, 'success');
			newDomainName = '';
			newDomainDesc = '';
			await loadDomains();
		} catch (e: any) {
			pushToast(e?.message ?? 'Failed to create domain', 'danger');
		} finally {
			newDomainBusy = false;
		}
	}

	async function handleRevokeDomain(id: string, name: string) {
		if (!confirm(`Revoke domain "${name}"? Credentials in this domain will no longer be accessible.`))
			return;
		try {
			await revokeDomain(id);
			pushToast(`Domain "${name}" revoked`, 'success');
			await loadDomains();
		} catch (e: any) {
			pushToast(e?.message ?? 'Failed to revoke domain', 'danger');
		}
	}

	// ── Actions: Credentials ─────────────────────────────────────
	async function loadCredentials() {
		credsLoading = true;
		try {
			credentials = await listCredentials({
				domainId: credsDomainFilter || undefined,
				state: (credsStateFilter as CredentialState) || undefined
			});
		} catch (e: any) {
			pushToast(e?.message ?? 'Failed to load credentials', 'danger');
		} finally {
			credsLoading = false;
		}
	}

	async function handleStoreCredential() {
		if (!newCredDomainId || !newCredName || !newCredValue) return;
		newCredBusy = true;
		try {
			await storeCredential({
				domainId: newCredDomainId,
				name: newCredName.trim(),
				kind: newCredKind,
				value: newCredValue,
				tags: newCredTags
					? newCredTags
							.split(',')
							.map((t) => t.trim())
							.filter(Boolean)
					: [],
				expiresAt: newCredExpires || undefined
			});
			pushToast(`Credential "${newCredName}" stored`, 'success');
			newCredName = '';
			newCredValue = '';
			newCredTags = '';
			newCredExpires = '';
			showNewCredForm = false;
			await loadCredentials();
		} catch (e: any) {
			pushToast(e?.message ?? 'Failed to store credential', 'danger');
		} finally {
			newCredBusy = false;
		}
	}

	async function handleViewCredential(id: string) {
		viewCredId = id;
		viewCredLoading = true;
		showViewCredValue = false;
		try {
			viewCredData = await readCredential(id);
		} catch (e: any) {
			pushToast(e?.message ?? 'Failed to read credential', 'danger');
			viewCredData = null;
		} finally {
			viewCredLoading = false;
		}
	}

	function closeViewModal() {
		viewCredData = null;
		viewCredId = '';
		editingCredValue = false;
		editCredValue = '';
	}

	async function handleArchiveCredential(id: string) {
		try {
			await archiveCredential(id);
			pushToast('Credential archived', 'success');
			await loadCredentials();
		} catch (e: any) {
			pushToast(e?.message ?? 'Failed to archive', 'danger');
		}
	}

	async function handleDestroyCredential(id: string) {
		if (!confirm('Permanently destroy this credential? This cannot be undone.')) return;
		try {
			await destroyCredential(id);
			pushToast('Credential destroyed', 'success');
			if (viewCredId === id) closeViewModal();
			await loadCredentials();
		} catch (e: any) {
			pushToast(e?.message ?? 'Failed to destroy', 'danger');
		}
	}

	async function handleRunLifecycle() {
		lifecycleBusy = true;
		try {
			await runLifecycle();
			pushToast('Lifecycle check completed', 'success');
			await loadCredentials();
		} catch (e: any) {
			pushToast(e?.message ?? 'Lifecycle check failed', 'danger');
		} finally {
			lifecycleBusy = false;
		}
	}

	async function handleEditCredentialValue() {
		if (!viewCredId || !editCredValue) return;
		editCredBusy = true;
		try {
			await updateCredential(viewCredId, editCredValue);
			pushToast('Credential value updated', 'success');
			editingCredValue = false;
			editCredValue = '';
			viewCredData = await readCredential(viewCredId);
		} catch (e: any) {
			pushToast(e?.message ?? 'Failed to update credential', 'danger');
		} finally {
			editCredBusy = false;
		}
	}

	function prefillCredential(name: string, kind: string) {
		newCredName = name;
		newCredKind = kind;
		showNewCredForm = true;
		// Auto-select api-keys domain if it exists
		const apiDomain = domains.find((d) => d.name === 'api-keys' && !d.revoked_at);
		if (apiDomain) newCredDomainId = apiDomain.id;
	}

	async function copyToClipboard(text: string) {
		try {
			await navigator.clipboard.writeText(text);
			pushToast('Copied to clipboard', 'success');
		} catch {
			pushToast('Failed to copy', 'danger');
		}
	}

	// ── Actions: Delegations ─────────────────────────────────────
	async function loadDelegationsFor(credId: string) {
		if (!credId) return;
		delegationsLoading = true;
		try {
			delegations = await listDelegations(credId);
		} catch (e: any) {
			pushToast(e?.message ?? 'Failed to load delegations', 'danger');
		} finally {
			delegationsLoading = false;
		}
	}

	async function handleCreateDelegation() {
		if (!newDelCredId || !newDelDelegatee.trim()) return;
		newDelBusy = true;
		try {
			await createDelegation({
				credentialId: newDelCredId,
				delegatee: newDelDelegatee.trim(),
				canRead: newDelCanRead,
				canUse: newDelCanUse,
				canDelegate: newDelCanDelegate,
				expiresAt: newDelExpires || undefined,
				maxDepth: parseInt(newDelMaxDepth, 10) || 3
			});
			pushToast('Delegation created', 'success');
			newDelDelegatee = '';
			newDelExpires = '';
			if (delegationCredId) await loadDelegationsFor(delegationCredId);
		} catch (e: any) {
			pushToast(e?.message ?? 'Failed to create delegation', 'danger');
		} finally {
			newDelBusy = false;
		}
	}

	async function handleRevokeDelegation(id: string) {
		try {
			await revokeDelegation(id);
			pushToast('Delegation revoked', 'success');
			if (delegationCredId) await loadDelegationsFor(delegationCredId);
		} catch (e: any) {
			pushToast(e?.message ?? 'Failed to revoke delegation', 'danger');
		}
	}

	// ── Actions: Audit ───────────────────────────────────────────
	async function loadAudit() {
		auditLoading = true;
		try {
			auditEntries = await listAudit({ limit: auditLimit, offset: auditOffset });
		} catch (e: any) {
			pushToast(e?.message ?? 'Failed to load audit log', 'danger');
		} finally {
			auditLoading = false;
		}
	}

	async function handleVerifyIntegrity() {
		try {
			const res = await verifyAuditIntegrity();
			if (res.valid) {
				pushToast('Audit integrity verified', 'success');
			} else {
				pushToast('Audit integrity check FAILED', 'danger');
			}
		} catch (e: any) {
			pushToast(e?.message ?? 'Verification failed', 'danger');
		}
	}

	// ── Actions: Alerts ──────────────────────────────────────────
	async function loadAlerts() {
		alertsLoading = true;
		try {
			alerts = await listAlerts({ limit: 100 });
		} catch (e: any) {
			pushToast(e?.message ?? 'Failed to load alerts', 'danger');
		} finally {
			alertsLoading = false;
		}
	}

	async function handleAcknowledgeAlert(id: string) {
		try {
			await acknowledgeAlert(id);
			pushToast('Alert acknowledged', 'success');
			await loadAlerts();
		} catch (e: any) {
			pushToast(e?.message ?? 'Failed to acknowledge', 'danger');
		}
	}

	// ── Actions: Backup ──────────────────────────────────────────
	async function handleBackup() {
		if (backupPassword !== backupPasswordConfirm) {
			pushToast('Passwords do not match', 'danger');
			return;
		}
		backupBusy = true;
		try {
			const blob = await backupVault(backupPassword);
			const url = URL.createObjectURL(blob);
			const a = document.createElement('a');
			a.href = url;
			a.download = `mindvault-keychain-backup-${new Date().toISOString().slice(0, 10)}.bin`;
			document.body.appendChild(a);
			a.click();
			document.body.removeChild(a);
			URL.revokeObjectURL(url);
			pushToast('Backup downloaded', 'success');
			backupPassword = '';
			backupPasswordConfirm = '';
		} catch (e: any) {
			pushToast(e?.message ?? 'Backup failed', 'danger');
		} finally {
			backupBusy = false;
		}
	}

	async function handleRestore() {
		if (!restoreFile || !restorePassword) return;
		if (
			!confirm(
				'Restoring will replace the current vault data. This is destructive. Continue?'
			)
		)
			return;
		restoreBusy = true;
		try {
			const buffer = await restoreFile.arrayBuffer();
			const bytes = new Uint8Array(buffer);
			let binary = '';
			for (let i = 0; i < bytes.byteLength; i++) {
				binary += String.fromCharCode(bytes[i]);
			}
			const b64 = btoa(binary);
			await restoreVault(b64, restorePassword);
			pushToast('Vault restored successfully', 'success');
			restoreFile = null;
			restorePassword = '';
			if (restoreFileInput) restoreFileInput.value = '';
			await pollVaultStatus();
		} catch (e: any) {
			pushToast(e?.message ?? 'Restore failed', 'danger');
		} finally {
			restoreBusy = false;
		}
	}

	// ── Tab change: load data ────────────────────────────────────
	function onTabChange(tab: Tab) {
		activeTab = tab;
		if (tab === 'vault' && $keychainStore.state === 'unsealed') loadEpochs();
		if (tab === 'credentials') {
			loadDomains();
			loadCredentials();
		}
		if (tab === 'domains') loadDomains();
		if (tab === 'delegations') loadDomains();
		if (tab === 'audit') {
			auditOffset = 0;
			loadAudit();
		}
		if (tab === 'alerts') loadAlerts();
	}

	// ── Helpers ──────────────────────────────────────────────────
	function stateColor(state: string): string {
		switch (state) {
			case 'active':
				return 'bg-emerald-400/20 text-emerald-300';
			case 'expiring':
				return 'bg-amber-400/20 text-amber-300';
			case 'expired':
				return 'bg-red-400/20 text-red-300';
			case 'archived':
				return 'bg-slate-400/20 text-slate-300';
			case 'destroyed':
				return 'bg-red-600/20 text-red-400';
			default:
				return 'bg-slate-400/20 text-slate-400';
		}
	}

	function severityColor(sev: string): string {
		switch (sev) {
			case 'low':
				return 'bg-emerald-400/20 text-emerald-300';
			case 'medium':
				return 'bg-amber-400/20 text-amber-300';
			case 'high':
				return 'bg-red-400/20 text-red-300';
			case 'critical':
				return 'bg-red-600/30 text-red-300 animate-pulse';
			default:
				return 'bg-slate-400/20 text-slate-400';
		}
	}

	function fmtDate(iso: string | null): string {
		if (!iso) return '—';
		try {
			return new Date(iso).toLocaleString();
		} catch {
			return iso;
		}
	}

	function shortId(id: string): string {
		return id.slice(0, 8) + '...';
	}

	// ── Mount ────────────────────────────────────────────────────
	pollVaultStatus();
</script>

<div class="mx-auto max-w-3xl">
	<div class="flex items-center justify-between">
		<div>
			<h2 class="text-lg font-semibold text-white">Sovereign Keychain</h2>
			<p class="mt-1 text-xs text-slate-400">
				Encrypted vault for credentials, delegations, and audit.
			</p>
		</div>
		<a
			href="/settings"
			class="rounded-lg border border-slate-700 px-2.5 py-1.5 text-[10px] text-slate-300 hover:bg-slate-800"
		>
			Back to Settings
		</a>
	</div>

	<!-- Tab nav -->
	<div class="mt-4 flex gap-1 overflow-x-auto border-b border-slate-800 pb-px">
		{#each tabs as tab (tab.key)}
			<button
				class={`whitespace-nowrap rounded-t-lg px-3 py-2 text-xs font-medium transition ${
					activeTab === tab.key
						? 'border-b-2 border-sky-500 text-white'
						: 'text-slate-400 hover:text-slate-200'
				}`}
				on:click={() => onTabChange(tab.key)}
			>
				{tab.label}
			</button>
		{/each}
	</div>

	<div class="mt-4 flex flex-col gap-4">
		<!-- ============================================================ -->
		<!-- TAB: Vault                                                   -->
		<!-- ============================================================ -->
		{#if activeTab === 'vault'}
			{#if $keychainStore.state === 'uninitialized'}
				<section class="rounded-xl border border-slate-800 bg-slate-900/40 p-5">
					<h3 class="text-sm font-semibold text-white">Initialize Vault</h3>
					<p class="mt-1 text-[11px] text-slate-400">
						Create a new encrypted vault protected by a master password. All credentials are
						encrypted at rest using AES-256-GCM with Argon2-derived keys.
					</p>
					<div class="mt-3 flex flex-col gap-3">
						<div>
							<label class="text-[10px] uppercase tracking-wider text-slate-500" for="init-pw"
								>Master Password</label
							>
							<div class="relative mt-1">
								<input
									id="init-pw"
									class="w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 pr-14 text-xs text-white outline-none focus:border-sky-500"
									type={showInitPassword ? 'text' : 'password'}
									placeholder="At least 8 characters"
									bind:value={initPassword}
								/>
								<button
									class="absolute right-2 top-1/2 -translate-y-1/2 text-[10px] text-slate-400 hover:text-white"
									aria-label={showInitPassword ? 'Hide password' : 'Show password'}
									on:click={() => (showInitPassword = !showInitPassword)}
								>
									{showInitPassword ? 'Hide' : 'Show'}
								</button>
							</div>
						</div>
						<div>
							<label class="text-[10px] uppercase tracking-wider text-slate-500" for="init-pw2"
								>Confirm Password</label
							>
							<input
								id="init-pw2"
								class="mt-1 w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white outline-none focus:border-sky-500"
								type="password"
								placeholder="Re-enter password"
								bind:value={initPasswordConfirm}
							/>
						</div>
						<label class="flex items-center gap-2 text-xs text-slate-300">
							<input type="checkbox" bind:checked={initMacosBridge} />
							Store password in macOS Keychain for auto-unseal
						</label>
						{#if initPassword && initPasswordConfirm && !initPasswordsMatch}
							<p class="text-[10px] text-red-400">Passwords do not match</p>
						{/if}
						<button
							class="self-start rounded-lg bg-sky-500 px-3 py-2 text-xs font-semibold text-white hover:bg-sky-400 disabled:opacity-50"
							disabled={!initPassword || !initPasswordConfirm || !initPasswordsMatch || initBusy}
							on:click={handleInit}
						>
							{initBusy ? 'Initializing...' : 'Initialize Vault'}
						</button>
					</div>
				</section>
			{:else if $keychainStore.state === 'sealed'}
				<section class="rounded-xl border border-amber-500/30 bg-amber-500/5 p-5">
					<div class="flex items-center gap-2">
						<span class="h-2 w-2 rounded-full bg-amber-400"></span>
						<h3 class="text-sm font-semibold text-amber-200">Vault Sealed</h3>
					</div>
					<p class="mt-1 text-[11px] text-slate-400">
						The vault is locked. Enter your master password to unseal.
					</p>
					<div class="mt-3 flex flex-col gap-3">
						<div>
							<label class="text-[10px] uppercase tracking-wider text-slate-500" for="unseal-pw"
								>Master Password</label
							>
							<div class="relative mt-1">
								<input
									id="unseal-pw"
									class="w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 pr-14 text-xs text-white outline-none focus:border-sky-500"
									type={showUnsealPassword ? 'text' : 'password'}
									placeholder="Enter password"
									bind:value={unsealPassword}
									on:keydown={(e) => e.key === 'Enter' && handleUnseal()}
								/>
								<button
									class="absolute right-2 top-1/2 -translate-y-1/2 text-[10px] text-slate-400 hover:text-white"
									aria-label={showUnsealPassword ? 'Hide password' : 'Show password'}
									on:click={() => (showUnsealPassword = !showUnsealPassword)}
								>
									{showUnsealPassword ? 'Hide' : 'Show'}
								</button>
							</div>
						</div>
						<div class="flex gap-2">
							<button
								class="rounded-lg bg-sky-500 px-3 py-2 text-xs font-semibold text-white hover:bg-sky-400 disabled:opacity-50"
								disabled={!unsealPassword || unsealBusy}
								on:click={() => handleUnseal()}
							>
								{unsealBusy ? 'Unsealing...' : 'Unseal'}
							</button>
							<button
								class="rounded-lg border border-slate-700 px-3 py-2 text-xs text-slate-300 hover:bg-slate-800"
								disabled={unsealBusy}
								on:click={() => handleUnseal({ macos: true })}
							>
								From macOS Keychain
							</button>
							<button
								class="rounded-lg border border-slate-700 px-3 py-2 text-xs text-slate-300 hover:bg-slate-800"
								disabled={unsealBusy}
								on:click={() => handleUnseal({ enclave: true })}
							>
								From Secure Enclave
							</button>
						</div>
					</div>
				</section>
			{:else}
				<!-- Unsealed -->
				<section class="rounded-xl border border-emerald-500/30 bg-emerald-500/5 p-5">
					<div class="flex items-center justify-between">
						<div class="flex items-center gap-2">
							<span class="h-2 w-2 rounded-full bg-emerald-400"></span>
							<h3 class="text-sm font-semibold text-emerald-200">Vault Unsealed</h3>
						</div>
						{#if sealCountdown != null}
							<span class="text-[10px] text-slate-400">
								Auto-seal in {formatCountdown(sealCountdown)}
							</span>
						{/if}
					</div>
					<div class="mt-3 grid grid-cols-3 gap-3">
						<div class="rounded-lg border border-slate-800/60 px-3 py-2.5 text-center">
							<div class="text-lg font-bold text-white">{$keychainStore.epoch ?? 0}</div>
							<div class="text-[10px] text-slate-500">Key Epoch</div>
						</div>
						<div class="rounded-lg border border-slate-800/60 px-3 py-2.5 text-center">
							<div class="text-lg font-bold text-white">
								{$keychainStore.credentialCount ?? 0}
							</div>
							<div class="text-[10px] text-slate-500">Credentials</div>
						</div>
						<div class="rounded-lg border border-slate-800/60 px-3 py-2.5 text-center">
							<div class="text-lg font-bold text-white">{$keychainStore.domainCount ?? 0}</div>
							<div class="text-[10px] text-slate-500">Domains</div>
						</div>
					</div>
					<div class="mt-3 flex gap-2">
						<button
							class="rounded-lg border border-slate-700 px-3 py-2 text-xs text-slate-300 hover:bg-slate-800"
							on:click={handleSeal}
						>
							Seal Now
						</button>
						<button
							class="rounded-lg border border-slate-700 px-3 py-2 text-xs text-slate-300 hover:bg-slate-800"
							on:click={() => (showRotateSection = !showRotateSection)}
						>
							{showRotateSection ? 'Cancel Rotation' : 'Rotate Master Key'}
						</button>
						<button
							class="rounded-lg border border-slate-700 px-3 py-2 text-xs text-slate-300 hover:bg-slate-800 disabled:opacity-50"
							disabled={lifecycleBusy}
							on:click={handleRunLifecycle}
						>
							{lifecycleBusy ? 'Running...' : 'Run Lifecycle'}
						</button>
					</div>

					{#if showRotateSection}
						<div class="mt-3 rounded-lg border border-slate-800/60 p-3">
							<h4 class="text-xs font-medium text-white">Rotate Master Key</h4>
							<p class="mt-1 text-[10px] text-slate-400">
								Re-encrypts all credentials with a new key. Old key remains valid during the
								grace period.
							</p>
							<div class="mt-2 flex flex-col gap-2">
								<div class="relative">
									<input
										class="w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 pr-14 text-xs text-white outline-none focus:border-sky-500"
										type={showRotatePassword ? 'text' : 'password'}
										placeholder="New master password"
										bind:value={rotateNewPassword}
									/>
									<button
										class="absolute right-2 top-1/2 -translate-y-1/2 text-[10px] text-slate-400 hover:text-white"
										on:click={() => (showRotatePassword = !showRotatePassword)}
									>
										{showRotatePassword ? 'Hide' : 'Show'}
									</button>
								</div>
								<input
									class="w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white outline-none focus:border-sky-500"
									type="password"
									placeholder="Confirm new password"
									bind:value={rotateNewPasswordConfirm}
								/>
								<div>
									<label
										class="text-[10px] uppercase tracking-wider text-slate-500"
										for="grace-hours">Grace period (hours)</label
									>
									<input
										id="grace-hours"
										class="mt-1 w-20 rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white outline-none focus:border-sky-500"
										type="number"
										min="1"
										max="168"
										bind:value={rotateGraceHours}
									/>
								</div>
								{#if rotateNewPassword && rotateNewPasswordConfirm && !rotatePasswordsMatch}
									<p class="text-[10px] text-red-400">Passwords do not match</p>
								{/if}
								<button
									class="self-start rounded-lg border border-red-500/30 bg-red-500/10 px-3 py-2 text-xs text-red-300 hover:bg-red-500/20 disabled:opacity-50"
									disabled={!rotateNewPassword || !rotateNewPasswordConfirm || !rotatePasswordsMatch || rotateBusy}
									on:click={handleRotate}
								>
									{rotateBusy ? 'Rotating...' : 'Rotate Key'}
								</button>
							</div>
						</div>
					{/if}

					<!-- Key Epochs table -->
					{#if epochs.length > 0}
						<div class="mt-4">
							<h4 class="text-[10px] uppercase tracking-wider text-slate-500">Key Epochs</h4>
							<div class="mt-2 overflow-x-auto">
								<table class="w-full text-left text-xs">
									<thead>
										<tr class="border-b border-slate-800 text-[10px] uppercase tracking-wider text-slate-500">
											<th class="py-2 pr-3">Epoch</th>
											<th class="py-2 pr-3">Created</th>
											<th class="py-2 pr-3">Grace Expires</th>
											<th class="py-2">Status</th>
										</tr>
									</thead>
									<tbody>
										{#each epochs as ep (ep.epoch)}
											<tr class="border-b border-slate-800/40">
												<td class="py-2 pr-3 text-white">{ep.epoch}</td>
												<td class="py-2 pr-3 text-slate-300">{fmtDate(ep.created_at)}</td>
												<td class="py-2 pr-3 text-slate-300">{fmtDate(ep.grace_expires_at)}</td>
												<td class="py-2">
													{#if ep.retired_at}
														<span class="rounded-full bg-slate-600/30 px-2 py-0.5 text-[10px] text-slate-400">retired</span>
													{:else}
														<span class="rounded-full bg-emerald-400/20 px-2 py-0.5 text-[10px] text-emerald-300">active</span>
													{/if}
												</td>
											</tr>
										{/each}
									</tbody>
								</table>
							</div>
						</div>
					{/if}
				</section>
			{/if}

		<!-- ============================================================ -->
		<!-- TAB: Credentials                                             -->
		<!-- ============================================================ -->
		{:else if activeTab === 'credentials'}
			<section class="rounded-xl border border-slate-800 bg-slate-900/40 p-5">
				<div class="flex items-center justify-between">
					<h3 class="text-sm font-semibold text-white">Credentials</h3>
					<div class="flex gap-2">
						<select
							class="rounded-lg border border-slate-700 bg-slate-800 px-2 py-1.5 text-xs text-white"
							bind:value={credsDomainFilter}
							on:change={loadCredentials}
						>
							<option value="">All domains</option>
							{#each domains as d (d.id)}
								<option value={d.id}>{d.name}</option>
							{/each}
						</select>
						<select
							class="rounded-lg border border-slate-700 bg-slate-800 px-2 py-1.5 text-xs text-white"
							bind:value={credsStateFilter}
							on:change={loadCredentials}
						>
							<option value="">All states</option>
							<option value="active">Active</option>
							<option value="expiring">Expiring</option>
							<option value="expired">Expired</option>
							<option value="archived">Archived</option>
						</select>
						<button
							class="rounded-lg bg-sky-500 px-3 py-1.5 text-xs font-semibold text-white hover:bg-sky-400"
							on:click={() => (showNewCredForm = !showNewCredForm)}
						>
							{showNewCredForm ? 'Cancel' : 'New Credential'}
						</button>
					</div>
				</div>

				<!-- Quick setup chips -->
				{#if $keychainStore.state === 'unsealed'}
					<div class="mt-3">
						<h4 class="text-[10px] uppercase tracking-wider text-slate-500">Quick Setup</h4>
						<div class="mt-1.5 flex flex-wrap gap-1.5">
							{#each WELL_KNOWN_API_KEYS as key}
								<button
									class="rounded-md border border-slate-700 px-2 py-1 text-[10px] text-slate-400 transition hover:border-sky-500/50 hover:text-sky-300"
									on:click={() => prefillCredential(key.name, 'api_key')}
									title={key.description}
								>
									{key.label}
									{#if 'required' in key && key.required}
										<span class="text-amber-400">*</span>
									{/if}
								</button>
							{/each}
						</div>
					</div>
				{/if}

				<!-- Search credentials -->
				<div class="mt-3">
					<input
						class="w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white outline-none focus:border-sky-500"
						placeholder="Search credentials by name..."
						bind:value={credSearchQuery}
					/>
				</div>

				<!-- New credential form -->
				{#if showNewCredForm}
					<div class="mt-3 rounded-lg border border-slate-800/60 p-3">
						<h4 class="text-xs font-medium text-white">Store New Credential</h4>
						<div class="mt-2 grid grid-cols-2 gap-2">
							<div>
								<label for="cred-domain" class="text-[10px] uppercase tracking-wider text-slate-500">Domain</label>
								<select
									id="cred-domain"
									class="mt-1 w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white"
									bind:value={newCredDomainId}
								>
									<option value="">Select domain...</option>
									{#each domains.filter((d) => !d.revoked_at) as d (d.id)}
										<option value={d.id}>{d.name}</option>
									{/each}
								</select>
							</div>
							<div>
								<label for="cred-kind" class="text-[10px] uppercase tracking-wider text-slate-500">Kind</label>
								<select
									id="cred-kind"
									class="mt-1 w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white"
									bind:value={newCredKind}
								>
									<option value="api_key">API Key</option>
									<option value="password">Password</option>
									<option value="token">Token</option>
									<option value="certificate">Certificate</option>
									<option value="ssh_key">SSH Key</option>
									<option value="other">Other</option>
								</select>
							</div>
						</div>
						<div class="mt-2">
							<label for="cred-name" class="text-[10px] uppercase tracking-wider text-slate-500">Name</label>
							<input
								id="cred-name"
								class="mt-1 w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white outline-none focus:border-sky-500"
								placeholder="e.g. OPENAI_API_KEY"
								bind:value={newCredName}
							/>
						</div>
						<div class="mt-2">
							<label for="cred-value" class="text-[10px] uppercase tracking-wider text-slate-500">Value</label>
							<div class="relative mt-1">
								<input
									id="cred-value"
									class="w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 pr-14 text-xs text-white outline-none focus:border-sky-500"
									type={showNewCredValue ? 'text' : 'password'}
									placeholder="Secret value"
									bind:value={newCredValue}
								/>
								<button
									class="absolute right-2 top-1/2 -translate-y-1/2 text-[10px] text-slate-400 hover:text-white"
									on:click={() => (showNewCredValue = !showNewCredValue)}
								>
									{showNewCredValue ? 'Hide' : 'Show'}
								</button>
							</div>
						</div>
						<div class="mt-2 grid grid-cols-2 gap-2">
							<div>
								<label for="cred-tags" class="text-[10px] uppercase tracking-wider text-slate-500">Tags</label>
								<input
									id="cred-tags"
									class="mt-1 w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white outline-none focus:border-sky-500"
									placeholder="comma-separated tags"
									bind:value={newCredTags}
								/>
							</div>
							<div>
								<label class="text-[10px] uppercase tracking-wider text-slate-500"
									 for="cred-expires">Expires At</label
								>
								<input
									id="cred-expires"
									class="mt-1 w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white outline-none focus:border-sky-500"
									type="datetime-local"
									min={now}
									bind:value={newCredExpires}
								/>
							</div>
						</div>
						<button
							class="mt-3 rounded-lg bg-sky-500 px-3 py-2 text-xs font-semibold text-white hover:bg-sky-400 disabled:opacity-50"
							disabled={!newCredDomainId || !newCredName || !newCredValue || newCredBusy}
							on:click={handleStoreCredential}
						>
							{newCredBusy ? 'Storing...' : 'Store Credential'}
						</button>
					</div>
				{/if}

				<!-- Credentials table -->
				{#if credsLoading}
					<p class="mt-3 text-xs text-slate-500">Loading...</p>
				{:else if filteredCredentials.length === 0}
					<p class="mt-3 text-xs text-slate-500">{credSearchQuery ? 'No credentials match your search.' : 'No credentials found.'}</p>
				{:else}
					<div class="mt-3 overflow-x-auto">
						<table class="w-full text-left text-xs">
							<thead>
								<tr class="border-b border-slate-800 text-[10px] uppercase tracking-wider text-slate-500">
									<th class="py-2 pr-3">Name</th>
									<th class="py-2 pr-3">Kind</th>
									<th class="py-2 pr-3">State</th>
									<th class="py-2 pr-3">Epoch</th>
									<th class="py-2 pr-3">Last Accessed</th>
									<th class="py-2">Actions</th>
								</tr>
							</thead>
							<tbody>
								{#each filteredCredentials as cred (cred.id)}
									<tr class="border-b border-slate-800/40">
										<td class="py-2 pr-3 font-medium text-white">{cred.name}</td>
										<td class="py-2 pr-3">
											<span class="rounded-md bg-slate-700/50 px-1.5 py-0.5 text-[10px] text-slate-300">{cred.kind}</span>
										</td>
										<td class="py-2 pr-3">
											<span class="rounded-full px-2 py-0.5 text-[10px] {stateColor(cred.state)}">{cred.state}</span>
										</td>
										<td class="py-2 pr-3 text-slate-400">{cred.epoch}</td>
										<td class="py-2 pr-3 text-slate-400">{fmtDate(cred.last_accessed_at)}</td>
										<td class="py-2">
											<div class="flex gap-1">
												<button
													class="rounded border border-slate-700 px-1.5 py-0.5 text-[10px] text-slate-300 hover:bg-slate-800"
													on:click={() => handleViewCredential(cred.id)}
												>
													View
												</button>
												{#if cred.state === 'active' || cred.state === 'expiring'}
													<button
														class="rounded border border-slate-700 px-1.5 py-0.5 text-[10px] text-slate-300 hover:bg-slate-800"
														on:click={() => handleArchiveCredential(cred.id)}
													>
														Archive
													</button>
												{/if}
												{#if cred.state !== 'destroyed'}
													<button
														class="rounded border border-red-500/30 px-1.5 py-0.5 text-[10px] text-red-300 hover:bg-red-500/10"
														on:click={() => handleDestroyCredential(cred.id)}
													>
														Destroy
													</button>
												{/if}
											</div>
										</td>
									</tr>
								{/each}
							</tbody>
						</table>
					</div>
				{/if}
			</section>

			<!-- View credential modal -->
			{#if viewCredData}
				<!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
				<div
					class="fixed inset-0 z-50 flex items-center justify-center bg-black/60"
					role="dialog"
					aria-modal="true"
					tabindex="-1"
					on:keydown={(e) => { if (e.key === 'Escape') closeViewModal(); }}
				>
					<div class="w-full max-w-lg rounded-xl border border-slate-800 bg-slate-900 p-5">
						<div class="flex items-center justify-between">
							<h3 class="text-sm font-semibold text-white">{viewCredData.credential.name}</h3>
							<button
								class="text-slate-400 hover:text-white"
								on:click={closeViewModal}
							>
								&times;
							</button>
						</div>
						<div class="mt-3 flex flex-col gap-2">
							<div>
								<label for="view-cred-value" class="text-[10px] uppercase tracking-wider text-slate-500">Value</label>
								<div class="relative mt-1">
									<input
										id="view-cred-value"
										class="w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 pr-24 text-xs text-white"
										type={showViewCredValue ? 'text' : 'password'}
										value={viewCredData.value}
										readonly
									/>
									<div class="absolute right-2 top-1/2 flex -translate-y-1/2 gap-1">
										<button
											class="text-[10px] text-slate-400 hover:text-white"
											aria-label={showViewCredValue ? 'Hide credential value' : 'Show credential value'}
											on:click={() => (showViewCredValue = !showViewCredValue)}
										>
											{showViewCredValue ? 'Hide' : 'Show'}
										</button>
										<button
											class="text-[10px] text-sky-400 hover:text-sky-300"
											on:click={() => copyToClipboard(viewCredData?.value ?? '')}
										>
											Copy
										</button>
									</div>
								</div>
							</div>
							<div class="grid grid-cols-2 gap-2 text-xs">
								<div>
									<span class="text-[10px] text-slate-500">Kind:</span>
									<span class="ml-1 text-slate-300">{viewCredData.credential.kind}</span>
								</div>
								<div>
									<span class="text-[10px] text-slate-500">State:</span>
									<span class="ml-1 rounded-full px-2 py-0.5 text-[10px] {stateColor(viewCredData.credential.state)}"
										>{viewCredData.credential.state}</span
									>
								</div>
								<div>
									<span class="text-[10px] text-slate-500">Epoch:</span>
									<span class="ml-1 text-slate-300">{viewCredData.credential.epoch}</span>
								</div>
								<div>
									<span class="text-[10px] text-slate-500">Access Count:</span>
									<span class="ml-1 text-slate-300">{viewCredData.credential.access_count}</span>
								</div>
								<div>
									<span class="text-[10px] text-slate-500">Created:</span>
									<span class="ml-1 text-slate-300">{fmtDate(viewCredData.credential.created_at)}</span>
								</div>
								<div>
									<span class="text-[10px] text-slate-500">Updated:</span>
									<span class="ml-1 text-slate-300">{fmtDate(viewCredData.credential.updated_at)}</span>
								</div>
							</div>
							{#if viewCredData.credential.tags.length > 0}
								<div>
									<span class="text-[10px] text-slate-500">Tags:</span>
									<div class="mt-1 flex flex-wrap gap-1">
										{#each viewCredData.credential.tags as tag}
											<span class="rounded-md bg-slate-700/50 px-1.5 py-0.5 text-[10px] text-slate-300">{tag}</span>
										{/each}
									</div>
								</div>
							{/if}
						</div>
						{#if editingCredValue}
							<div class="mt-3 flex flex-col gap-2">
								<label for="edit-cred-value" class="text-[10px] uppercase tracking-wider text-slate-500">New Value</label>
								<input
									id="edit-cred-value"
									class="w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white outline-none focus:border-sky-500"
									type="password"
									placeholder="Enter new secret value"
									bind:value={editCredValue}
								/>
								<div class="flex gap-2">
									<button
										class="rounded-lg bg-sky-500 px-3 py-1.5 text-xs font-semibold text-white hover:bg-sky-400 disabled:opacity-50"
										disabled={!editCredValue || editCredBusy}
										on:click={handleEditCredentialValue}
									>
										{editCredBusy ? 'Saving...' : 'Save'}
									</button>
									<button
										class="rounded-lg border border-slate-700 px-3 py-1.5 text-xs text-slate-300 hover:bg-slate-800"
										on:click={() => { editingCredValue = false; editCredValue = ''; }}
									>
										Cancel
									</button>
								</div>
							</div>
						{/if}
						<div class="mt-4 flex gap-2">
							{#if !editingCredValue && viewCredData.credential.state !== 'destroyed'}
								<button
									class="rounded-lg border border-slate-700 px-3 py-2 text-xs text-slate-300 hover:bg-slate-800"
									on:click={() => (editingCredValue = true)}
								>
									Edit Value
								</button>
							{/if}
							<button
								class="rounded-lg border border-slate-700 px-3 py-2 text-xs text-slate-300 hover:bg-slate-800"
								on:click={closeViewModal}
							>
								Close
							</button>
						</div>
					</div>
				</div>
			{/if}

		<!-- ============================================================ -->
		<!-- TAB: Domains                                                 -->
		<!-- ============================================================ -->
		{:else if activeTab === 'domains'}
			<section class="rounded-xl border border-slate-800 bg-slate-900/40 p-5">
				<h3 class="text-sm font-semibold text-white">Credential Domains</h3>
				<p class="mt-1 text-[11px] text-slate-400">
					Domains isolate credentials by purpose. Each domain has its own derived key.
				</p>

				<!-- Create domain form -->
				<div class="mt-3 flex gap-2">
					<input
						class="flex-1 rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white outline-none focus:border-sky-500"
						placeholder="Domain name (e.g. api-keys)"
						bind:value={newDomainName}
					/>
					<input
						class="flex-1 rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white outline-none focus:border-sky-500"
						placeholder="Description (optional)"
						bind:value={newDomainDesc}
					/>
					<button
						class="rounded-lg bg-sky-500 px-3 py-2 text-xs font-semibold text-white hover:bg-sky-400 disabled:opacity-50"
						disabled={!newDomainName.trim() || newDomainBusy}
						on:click={handleCreateDomain}
					>
						{newDomainBusy ? 'Creating...' : 'Create'}
					</button>
				</div>

				<!-- Domain cards -->
				{#if domainsLoading}
					<p class="mt-3 text-xs text-slate-500">Loading...</p>
				{:else if domains.length === 0}
					<p class="mt-3 text-xs text-slate-500">No domains yet. Create one to start storing credentials.</p>
				{:else}
					<div class="mt-3 grid grid-cols-1 gap-2 sm:grid-cols-2">
						{#each domains as domain (domain.id)}
							<div class="rounded-lg border border-slate-800/60 px-3 py-2.5">
								<div class="flex items-center justify-between">
									<div class="flex items-center gap-2">
										<span
											class={`h-2 w-2 rounded-full ${domain.revoked_at ? 'bg-red-400' : 'bg-emerald-400'}`}
										></span>
										<span class="text-xs font-medium text-white">{domain.name}</span>
									</div>
									{#if !domain.revoked_at}
										<button
											class="rounded border border-red-500/30 px-1.5 py-0.5 text-[10px] text-red-300 hover:bg-red-500/10"
											on:click={() => handleRevokeDomain(domain.id, domain.name)}
										>
											Revoke
										</button>
									{/if}
								</div>
								{#if domain.description}
									<p class="mt-1 text-[10px] text-slate-400">{domain.description}</p>
								{/if}
								<div class="mt-1.5 flex gap-3 text-[10px] text-slate-500">
									<span>{domain.credential_count} credentials</span>
									<span>Epoch {domain.epoch}</span>
									<span>{fmtDate(domain.created_at)}</span>
								</div>
								{#if domain.revoked_at}
									<div class="mt-1 text-[10px] text-red-400">
										Revoked {fmtDate(domain.revoked_at)}
									</div>
								{/if}
							</div>
						{/each}
					</div>
				{/if}
			</section>

		<!-- ============================================================ -->
		<!-- TAB: Delegations                                             -->
		<!-- ============================================================ -->
		{:else if activeTab === 'delegations'}
			<section class="rounded-xl border border-slate-800 bg-slate-900/40 p-5">
				<h3 class="text-sm font-semibold text-white">Delegations</h3>
				<p class="mt-1 text-[11px] text-slate-400">
					Grant third-party access to credentials with scoped permissions and depth limits.
				</p>

				<!-- Credential selector -->
				<div class="mt-3">
					<label for="del-cred-select" class="text-[10px] uppercase tracking-wider text-slate-500">Select Credential</label>
					<div class="mt-1 flex gap-2">
						<select
							id="del-cred-select"
							class="flex-1 rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white"
							bind:value={delegationCredId}
							on:change={() => loadDelegationsFor(delegationCredId)}
						>
							<option value="">Choose credential...</option>
							{#each credentials as c (c.id)}
								<option value={c.id}>{c.name}</option>
							{/each}
						</select>
					</div>
				</div>

				<!-- Delegation table -->
				{#if delegationCredId}
					{#if delegationsLoading}
						<p class="mt-3 text-xs text-slate-500">Loading...</p>
					{:else if delegations.length === 0}
						<p class="mt-3 text-xs text-slate-500">No delegations for this credential.</p>
					{:else}
						<div class="mt-3 overflow-x-auto">
							<table class="w-full text-left text-xs">
								<thead>
									<tr class="border-b border-slate-800 text-[10px] uppercase tracking-wider text-slate-500">
										<th class="py-2 pr-3">Delegatee</th>
										<th class="py-2 pr-3">Depth</th>
										<th class="py-2 pr-3">Permissions</th>
										<th class="py-2 pr-3">Expires</th>
										<th class="py-2 pr-3">Status</th>
										<th class="py-2">Actions</th>
									</tr>
								</thead>
								<tbody>
									{#each delegations as del (del.id)}
										<tr class="border-b border-slate-800/40">
											<td class="py-2 pr-3 text-white">{del.delegatee}</td>
											<td class="py-2 pr-3 text-slate-400">{del.depth}/{del.max_depth}</td>
											<td class="py-2 pr-3">
												<div class="flex gap-1">
													{#if del.permissions.can_read}
														<span class="rounded bg-emerald-400/20 px-1 py-0.5 text-[10px] text-emerald-300">R</span>
													{/if}
													{#if del.permissions.can_use}
														<span class="rounded bg-sky-400/20 px-1 py-0.5 text-[10px] text-sky-300">U</span>
													{/if}
													{#if del.permissions.can_delegate}
														<span class="rounded bg-amber-400/20 px-1 py-0.5 text-[10px] text-amber-300">D</span>
													{/if}
												</div>
											</td>
											<td class="py-2 pr-3 text-slate-400">{fmtDate(del.expires_at)}</td>
											<td class="py-2 pr-3">
												{#if del.revoked_at}
													<span class="rounded-full bg-red-400/20 px-2 py-0.5 text-[10px] text-red-300">revoked</span>
												{:else}
													<span class="rounded-full bg-emerald-400/20 px-2 py-0.5 text-[10px] text-emerald-300">active</span>
												{/if}
											</td>
											<td class="py-2">
												{#if !del.revoked_at}
													<button
														class="rounded border border-red-500/30 px-1.5 py-0.5 text-[10px] text-red-300 hover:bg-red-500/10"
														on:click={() => handleRevokeDelegation(del.id)}
													>
														Revoke
													</button>
												{/if}
											</td>
										</tr>
									{/each}
								</tbody>
							</table>
						</div>
					{/if}
				{/if}

				<!-- Create delegation form -->
				<div class="mt-4 rounded-lg border border-slate-800/60 p-3">
					<h4 class="text-xs font-medium text-white">Create Delegation</h4>
					<div class="mt-2 grid grid-cols-2 gap-2">
						<div>
							<label for="new-del-cred" class="text-[10px] uppercase tracking-wider text-slate-500">Credential</label>
							<select
								id="new-del-cred"
								class="mt-1 w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white"
								bind:value={newDelCredId}
							>
								<option value="">Choose...</option>
								{#each credentials as c (c.id)}
									<option value={c.id}>{c.name}</option>
								{/each}
							</select>
						</div>
						<div>
							<label for="new-del-delegatee" class="text-[10px] uppercase tracking-wider text-slate-500">Delegatee</label>
							<input
								id="new-del-delegatee"
								class="mt-1 w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white outline-none focus:border-sky-500"
								placeholder="Agent or user name"
								bind:value={newDelDelegatee}
							/>
						</div>
					</div>
					<div class="mt-2 flex flex-wrap items-center gap-4">
						<label class="flex items-center gap-1 text-xs text-slate-300">
							<input type="checkbox" bind:checked={newDelCanRead} />
							Read
						</label>
						<label class="flex items-center gap-1 text-xs text-slate-300">
							<input type="checkbox" bind:checked={newDelCanUse} />
							Use
						</label>
						<label class="flex items-center gap-1 text-xs text-slate-300">
							<input type="checkbox" bind:checked={newDelCanDelegate} />
							Delegate
						</label>
						<div>
							<label for="new-del-depth" class="text-[10px] uppercase tracking-wider text-slate-500">Max Depth</label>
							<input
								id="new-del-depth"
								class="ml-1 w-14 rounded-lg border border-slate-700 bg-slate-800 px-2 py-1 text-xs text-white"
								type="number"
								min="1"
								max="10"
								bind:value={newDelMaxDepth}
							/>
						</div>
						<div>
							<label for="new-del-expires" class="text-[10px] uppercase tracking-wider text-slate-500">Expires</label>
							<input
								id="new-del-expires"
								class="ml-1 rounded-lg border border-slate-700 bg-slate-800 px-2 py-1 text-xs text-white"
								type="datetime-local"
								min={now}
								bind:value={newDelExpires}
							/>
						</div>
					</div>
					<button
						class="mt-3 rounded-lg bg-sky-500 px-3 py-2 text-xs font-semibold text-white hover:bg-sky-400 disabled:opacity-50"
						disabled={!newDelCredId || !newDelDelegatee.trim() || newDelBusy}
						on:click={handleCreateDelegation}
					>
						{newDelBusy ? 'Creating...' : 'Create Delegation'}
					</button>
				</div>
			</section>

		<!-- ============================================================ -->
		<!-- TAB: Audit                                                   -->
		<!-- ============================================================ -->
		{:else if activeTab === 'audit'}
			<section class="rounded-xl border border-slate-800 bg-slate-900/40 p-5">
				<div class="flex items-center justify-between">
					<h3 class="text-sm font-semibold text-white">Audit Trail</h3>
					<button
						class="rounded-lg border border-slate-700 px-2.5 py-1.5 text-[10px] text-slate-300 hover:bg-slate-800"
						on:click={handleVerifyIntegrity}
					>
						Verify Integrity
					</button>
				</div>

				{#if auditLoading}
					<p class="mt-3 text-xs text-slate-500">Loading...</p>
				{:else if auditEntries.length === 0}
					<p class="mt-3 text-xs text-slate-500">No audit entries.</p>
				{:else}
					<div class="mt-3 overflow-x-auto">
						<table class="w-full text-left text-xs">
							<thead>
								<tr class="border-b border-slate-800 text-[10px] uppercase tracking-wider text-slate-500">
									<th class="py-2 pr-3">Seq</th>
									<th class="py-2 pr-3">Action</th>
									<th class="py-2 pr-3">Subject</th>
									<th class="py-2 pr-3">Resource</th>
									<th class="py-2 pr-3">Timestamp</th>
									<th class="py-2">Sig</th>
								</tr>
							</thead>
							<tbody>
								{#each auditEntries as entry (entry.id)}
									<tr class="border-b border-slate-800/40">
										<td class="py-2 pr-3 text-slate-400">{entry.sequence}</td>
										<td class="py-2 pr-3 text-white">{entry.action}</td>
										<td class="py-2 pr-3 text-slate-300">{entry.subject}</td>
										<td class="py-2 pr-3 text-slate-400">{entry.resource_id ? shortId(entry.resource_id) : '—'}</td>
										<td class="py-2 pr-3 text-slate-400">{fmtDate(entry.timestamp)}</td>
										<td class="py-2">
											{#if entry.signature}
												<span class="text-emerald-400">&#10003;</span>
											{:else}
												<span class="text-slate-600">—</span>
											{/if}
										</td>
									</tr>
								{/each}
							</tbody>
						</table>
					</div>
					<div class="mt-3 flex gap-2">
						<button
							class="rounded-lg border border-slate-700 px-2.5 py-1.5 text-[10px] text-slate-300 hover:bg-slate-800 disabled:opacity-50"
							disabled={auditOffset === 0}
							on:click={() => {
								auditOffset = Math.max(0, auditOffset - auditLimit);
								loadAudit();
							}}
						>
							Previous
						</button>
						<span class="flex items-center text-[10px] text-slate-500"
							>Offset: {auditOffset}</span
						>
						<button
							class="rounded-lg border border-slate-700 px-2.5 py-1.5 text-[10px] text-slate-300 hover:bg-slate-800 disabled:opacity-50"
							disabled={auditEntries.length < auditLimit}
							on:click={() => {
								auditOffset += auditLimit;
								loadAudit();
							}}
						>
							Next
						</button>
					</div>
				{/if}
			</section>

		<!-- ============================================================ -->
		<!-- TAB: Alerts                                                  -->
		<!-- ============================================================ -->
		{:else if activeTab === 'alerts'}
			<section class="rounded-xl border border-slate-800 bg-slate-900/40 p-5">
				<h3 class="text-sm font-semibold text-white">Breach Alerts</h3>
				<p class="mt-1 text-[11px] text-slate-400">
					Anomaly detection for credential access patterns.
				</p>

				{#if alertsLoading}
					<p class="mt-3 text-xs text-slate-500">Loading...</p>
				{:else if alerts.length === 0}
					<p class="mt-3 text-xs text-slate-500">No alerts. All clear.</p>
				{:else}
					<div class="mt-3 space-y-2">
						{#each alerts as alert (alert.id)}
							<div class="rounded-lg border border-slate-800/60 px-3 py-2.5">
								<div class="flex items-center justify-between">
									<div class="flex items-center gap-2">
										<span class="rounded-full px-2 py-0.5 text-[10px] {severityColor(alert.severity)}"
											>{alert.severity}</span
										>
										<span class="text-xs font-medium text-white">{alert.alert_type.replace(/_/g, ' ')}</span>
									</div>
									{#if !alert.acknowledged_at}
										<button
											class="rounded border border-slate-700 px-1.5 py-0.5 text-[10px] text-slate-300 hover:bg-slate-800"
											on:click={() => handleAcknowledgeAlert(alert.id)}
										>
											Acknowledge
										</button>
									{:else}
										<span class="text-[10px] text-slate-500">Acknowledged</span>
									{/if}
								</div>
								<p class="mt-1 text-[11px] text-slate-300">{alert.description}</p>
								<div class="mt-1 flex gap-3 text-[10px] text-slate-500">
									<span>Credential: {shortId(alert.credential_id)}</span>
									<span>{fmtDate(alert.timestamp)}</span>
								</div>
							</div>
						{/each}
					</div>
				{/if}
			</section>

		<!-- ============================================================ -->
		<!-- TAB: Backup                                                  -->
		<!-- ============================================================ -->
		{:else if activeTab === 'backup'}
			<section class="rounded-xl border border-slate-800 bg-slate-900/40 p-5">
				<h3 class="text-sm font-semibold text-white">Export Backup</h3>
				<p class="mt-1 text-[11px] text-slate-400">
					Download an encrypted backup of the entire vault.
				</p>
				<div class="mt-3 flex flex-col gap-2">
					<div class="relative">
						<input
							class="w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 pr-14 text-xs text-white outline-none focus:border-sky-500"
							type={showBackupPassword ? 'text' : 'password'}
							placeholder="Backup password"
							bind:value={backupPassword}
						/>
						<button
							class="absolute right-2 top-1/2 -translate-y-1/2 text-[10px] text-slate-400 hover:text-white"
							on:click={() => (showBackupPassword = !showBackupPassword)}
						>
							{showBackupPassword ? 'Hide' : 'Show'}
						</button>
					</div>
					<input
						class="w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white outline-none focus:border-sky-500"
						type="password"
						placeholder="Confirm backup password"
						bind:value={backupPasswordConfirm}
					/>
					<button
						class="self-start rounded-lg bg-sky-500 px-3 py-2 text-xs font-semibold text-white hover:bg-sky-400 disabled:opacity-50"
						disabled={!backupPassword || !backupPasswordConfirm || !backupPasswordsMatch || backupBusy}
						on:click={handleBackup}
					>
						{backupBusy ? 'Exporting...' : 'Download Backup'}
					</button>
				</div>
			</section>

			<section class="rounded-xl border border-slate-800 bg-slate-900/40 p-5">
				<h3 class="text-sm font-semibold text-white">Restore from Backup</h3>
				<p class="mt-1 text-[11px] text-slate-400">
					Import a previously exported vault backup. This replaces current vault data.
				</p>
				<div class="mt-3 flex flex-col gap-2">
					<label
						class="cursor-pointer rounded-lg border border-dashed border-slate-700 px-3 py-2 text-xs text-slate-400 hover:border-slate-500"
					>
						{restoreFile ? restoreFile.name : 'Choose backup file...'}
						<input
							type="file"
							accept=".bin,.backup"
							class="hidden"
							bind:this={restoreFileInput}
							on:change={(e) => {
								const input = e.currentTarget;
								restoreFile = input.files?.[0] ?? null;
							}}
						/>
					</label>
					<div class="relative">
						<input
							class="w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 pr-14 text-xs text-white outline-none focus:border-sky-500"
							type={showRestorePassword ? 'text' : 'password'}
							placeholder="Backup password"
							bind:value={restorePassword}
						/>
						<button
							class="absolute right-2 top-1/2 -translate-y-1/2 text-[10px] text-slate-400 hover:text-white"
							on:click={() => (showRestorePassword = !showRestorePassword)}
						>
							{showRestorePassword ? 'Hide' : 'Show'}
						</button>
					</div>
					<button
						class="self-start rounded-lg border border-red-500/30 bg-red-500/10 px-3 py-2 text-xs text-red-300 hover:bg-red-500/20 disabled:opacity-50"
						disabled={!restoreFile || !restorePassword || restoreBusy}
						on:click={handleRestore}
					>
						{restoreBusy ? 'Restoring...' : 'Restore Vault'}
					</button>
				</div>
			</section>
		{/if}
	</div>
</div>
