<script lang="ts">
	import { onMount } from 'svelte';
	import { pushToast } from '$lib/stores/toast';
	import { getProfile, updateProfile } from '$lib/api/profile';

	let loading = true;
	let saving = false;

	let displayName = '';
	let email = '';
	let avatarUrl = '';
	let bio = '';
	let timezone = 'UTC';
	let preferredNamespace = 'default';
	let defaultNodeKind = 'fact';
	let preferredLlmProvider = '';
	let signatureName = '';
	let signaturePublicKey = '';

	const TIMEZONES = [
		'UTC',
		'America/New_York',
		'America/Chicago',
		'America/Denver',
		'America/Los_Angeles',
		'America/Anchorage',
		'Pacific/Honolulu',
		'America/Toronto',
		'America/Vancouver',
		'America/Sao_Paulo',
		'America/Argentina/Buenos_Aires',
		'Europe/London',
		'Europe/Paris',
		'Europe/Berlin',
		'Europe/Amsterdam',
		'Europe/Rome',
		'Europe/Madrid',
		'Europe/Moscow',
		'Europe/Istanbul',
		'Asia/Dubai',
		'Asia/Kolkata',
		'Asia/Bangkok',
		'Asia/Shanghai',
		'Asia/Hong_Kong',
		'Asia/Tokyo',
		'Asia/Seoul',
		'Asia/Singapore',
		'Australia/Sydney',
		'Australia/Melbourne',
		'Pacific/Auckland'
	];

	const NODE_KINDS = [
		'fact',
		'task',
		'event',
		'decision',
		'preference',
		'entity',
		'code_snippet',
		'project',
		'conversation',
		'procedure',
		'observation',
		'bookmark',
		'template',
		'saved_view'
	];

	const LLM_PROVIDERS = [
		{ value: '', label: 'Server Default' },
		{ value: 'openai', label: 'OpenAI' },
		{ value: 'anthropic', label: 'Anthropic' },
		{ value: 'ollama', label: 'Ollama (Local)' }
	];

	onMount(async () => {
		try {
			const p = await getProfile();
			displayName = p.display_name;
			email = p.email ?? '';
			avatarUrl = p.avatar_url ?? '';
			bio = p.bio ?? '';
			timezone = p.timezone || 'UTC';
			preferredNamespace = p.preferred_namespace || 'default';
			defaultNodeKind = p.default_node_kind || 'fact';
			preferredLlmProvider = p.preferred_llm_provider ?? '';
			signatureName = p.signature_name ?? '';
			signaturePublicKey = p.signature_public_key ?? '';
		} catch (e: any) {
			pushToast(e?.message ?? 'Failed to load profile', 'danger');
		} finally {
			loading = false;
		}
	});

	async function handleSave() {
		if (!displayName.trim()) {
			pushToast('Display name is required', 'danger');
			return;
		}
		saving = true;
		try {
			const updated = await updateProfile({
				display_name: displayName.trim(),
				email: email.trim() || undefined,
				avatar_url: avatarUrl.trim() || undefined,
				bio: bio.trim() || undefined,
				timezone,
				preferred_namespace: preferredNamespace.trim() || 'default',
				default_node_kind: defaultNodeKind,
				preferred_llm_provider: preferredLlmProvider || undefined,
				signature_name: signatureName.trim() || undefined,
				signature_public_key: signaturePublicKey.trim() || undefined
			});
			displayName = updated.display_name;
			email = updated.email ?? '';
			avatarUrl = updated.avatar_url ?? '';
			bio = updated.bio ?? '';
			timezone = updated.timezone || 'UTC';
			preferredNamespace = updated.preferred_namespace || 'default';
			defaultNodeKind = updated.default_node_kind || 'fact';
			preferredLlmProvider = updated.preferred_llm_provider ?? '';
			signatureName = updated.signature_name ?? '';
			signaturePublicKey = updated.signature_public_key ?? '';
			pushToast('Profile saved', 'success');
		} catch (e: any) {
			pushToast(e?.message ?? 'Failed to update profile', 'danger');
		} finally {
			saving = false;
		}
	}
</script>

<div class="mx-auto max-w-2xl">
	<div class="flex items-center justify-between">
		<div>
			<h2 class="text-lg font-semibold text-white">Owner Profile</h2>
			<p class="mt-1 text-xs text-slate-400">
				Your identity for signing nodes, email headers, and federation.
			</p>
		</div>
		<a
			href="/settings"
			class="rounded-lg border border-slate-700 px-2.5 py-1.5 text-[10px] text-slate-300 hover:bg-slate-800"
		>
			Back to Settings
		</a>
	</div>

	{#if loading}
		<div class="mt-6 text-xs text-slate-500">Loading profile...</div>
	{:else}
		<section class="mt-6 rounded-xl border border-slate-800 bg-slate-900/40 p-5">
			<div class="flex flex-col gap-4">
				<!-- Identity -->
				<h4 class="text-[10px] font-semibold uppercase tracking-wider text-slate-400">
					Identity
				</h4>

				<div>
					<label class="text-[10px] uppercase tracking-wider text-slate-500" for="profile-name">
						Display Name
					</label>
					<input
						id="profile-name"
						class="mt-1 w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white outline-none focus:border-sky-500"
						placeholder="Your name"
						bind:value={displayName}
					/>
				</div>

				<div>
					<label class="text-[10px] uppercase tracking-wider text-slate-500" for="profile-email">
						Email
					</label>
					<input
						id="profile-email"
						class="mt-1 w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white outline-none focus:border-sky-500"
						type="email"
						placeholder="you@example.com"
						bind:value={email}
					/>
					<p class="mt-1 text-[10px] text-slate-500">
						Default sender for email adapters and federation identity.
					</p>
				</div>

				<div>
					<label class="text-[10px] uppercase tracking-wider text-slate-500" for="profile-avatar">
						Avatar URL
					</label>
					<input
						id="profile-avatar"
						class="mt-1 w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white outline-none focus:border-sky-500"
						placeholder="https://"
						bind:value={avatarUrl}
					/>
				</div>

				<div>
					<label class="text-[10px] uppercase tracking-wider text-slate-500" for="profile-bio">
						Bio
					</label>
					<textarea
						id="profile-bio"
						class="mt-1 w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white outline-none focus:border-sky-500"
						rows="3"
						placeholder="Short bio for federation profile"
						bind:value={bio}
					></textarea>
				</div>

				<div>
					<label class="text-[10px] uppercase tracking-wider text-slate-500" for="profile-sig">
						Signature Name
					</label>
					<input
						id="profile-sig"
						class="mt-1 w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white outline-none focus:border-sky-500"
						placeholder="Name used when signing outgoing messages"
						bind:value={signatureName}
					/>
				</div>

				<div>
					<label class="text-[10px] uppercase tracking-wider text-slate-500" for="profile-sig-key">
						Signature Public Key
					</label>
					<textarea
						id="profile-sig-key"
						class="mt-1 w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white outline-none focus:border-sky-500"
						rows="2"
						placeholder="Optional public key for verifying signatures"
						bind:value={signaturePublicKey}
					></textarea>
				</div>

				<!-- Preferences -->
				<h4 class="mt-2 text-[10px] font-semibold uppercase tracking-wider text-slate-400">
					Preferences
				</h4>

				<div>
					<label class="text-[10px] uppercase tracking-wider text-slate-500" for="profile-tz">
						Timezone
					</label>
					<select
						id="profile-tz"
						class="mt-1 w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white"
						bind:value={timezone}
					>
						{#each TIMEZONES as tz}
							<option value={tz}>{tz}</option>
						{/each}
					</select>
					<p class="mt-1 text-[10px] text-slate-500">
						Controls daily note scheduling, quiet hours, and date formatting.
					</p>
				</div>

				<div>
					<label class="text-[10px] uppercase tracking-wider text-slate-500" for="profile-ns">
						Preferred Namespace
					</label>
					<input
						id="profile-ns"
						class="mt-1 w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white outline-none focus:border-sky-500"
						placeholder="default"
						bind:value={preferredNamespace}
					/>
					<p class="mt-1 text-[10px] text-slate-500">
						Default namespace for new nodes and searches.
					</p>
				</div>

				<div>
					<label class="text-[10px] uppercase tracking-wider text-slate-500" for="profile-kind">
						Default Node Kind
					</label>
					<select
						id="profile-kind"
						class="mt-1 w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white"
						bind:value={defaultNodeKind}
					>
						{#each NODE_KINDS as kind}
							<option value={kind}>{kind}</option>
						{/each}
					</select>
					<p class="mt-1 text-[10px] text-slate-500">
						Kind assigned to new nodes when not specified.
					</p>
				</div>

				<div>
					<label class="text-[10px] uppercase tracking-wider text-slate-500" for="profile-llm">
						Preferred LLM Provider
					</label>
					<select
						id="profile-llm"
						class="mt-1 w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white"
						bind:value={preferredLlmProvider}
					>
						{#each LLM_PROVIDERS as provider}
							<option value={provider.value}>{provider.label}</option>
						{/each}
					</select>
					<p class="mt-1 text-[10px] text-slate-500">
						Default LLM provider for AI-assisted features.
					</p>
				</div>

				<div class="mt-2 flex items-center gap-3">
					<button
						class="rounded-lg bg-sky-500 px-4 py-2 text-xs font-semibold text-white hover:bg-sky-400 disabled:opacity-50"
						disabled={!displayName.trim() || saving}
						on:click={handleSave}
					>
						{saving ? 'Saving...' : 'Save Profile'}
					</button>
				</div>
			</div>
		</section>
	{/if}
</div>
