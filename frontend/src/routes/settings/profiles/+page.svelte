<script lang="ts">
	import { onMount } from 'svelte';
	import { pushToast } from '$lib/stores/toast';
	import {
		listAccessKeys,
		createAccessKey,
		revokeAccessKey,
		type AccessKey,
		type CreateAccessKeyPayload
	} from '$lib/api/access-keys';
	import {
		listPermissionTemplates,
		createPermissionTemplate,
		deletePermissionTemplate,
		type PermissionTemplate,
		type CreatePermissionTemplatePayload
	} from '$lib/api/permission-templates';

	let accessKeys: AccessKey[] = [];
	let templates: PermissionTemplate[] = [];
	let loading = true;

	// Create key form
	let newKeyName = '';
	let newKeyDescription = '';
	let newKeyTemplateId = '';
	let newKeyExpires = '';
	let isCreatingKey = false;

	// Revealed raw key (shown once after creation)
	let revealedKey: { id: string; raw: string } | null = null;

	// Create template form
	let showTemplateForm = false;
	let newTemplateName = '';
	let newTemplateDescription = '';
	let newTemplateTier: 'view' | 'edit' | 'action' | 'admin' = 'view';
	let newTemplateNamespaces = '';
	let newTemplateTags = '';
	let newTemplateKinds = '';
	let isCreatingTemplate = false;

	onMount(async () => {
		await loadData();
	});

	async function loadData() {
		loading = true;
		try {
			const [keys, tmpls] = await Promise.all([
				listAccessKeys(),
				listPermissionTemplates()
			]);
			accessKeys = keys;
			templates = tmpls;
		} catch {
			pushToast('Failed to load profiles data', 'danger');
		} finally {
			loading = false;
		}
	}

	async function handleCreateKey() {
		if (!newKeyName.trim()) {
			pushToast('Key name is required', 'warning');
			return;
		}
		isCreatingKey = true;
		try {
			const payload: CreateAccessKeyPayload = {
				name: newKeyName.trim(),
				description: newKeyDescription.trim() || undefined,
				permission_template_id: newKeyTemplateId || undefined,
				expires_at: newKeyExpires || undefined
			};
			const result = await createAccessKey(payload);
			revealedKey = { id: result.key_id, raw: result.raw_key };
			newKeyName = '';
			newKeyDescription = '';
			newKeyTemplateId = '';
			newKeyExpires = '';
			await loadData();
			pushToast('Access key created', 'success');
		} catch {
			pushToast('Failed to create access key', 'danger');
		} finally {
			isCreatingKey = false;
		}
	}

	async function handleRevokeKey(id: string) {
		try {
			await revokeAccessKey(id);
			accessKeys = accessKeys.filter(k => k.id !== id);
			pushToast('Access key revoked', 'success');
		} catch {
			pushToast('Failed to revoke key', 'danger');
		}
	}

	async function handleCreateTemplate() {
		if (!newTemplateName.trim()) {
			pushToast('Template name is required', 'warning');
			return;
		}
		isCreatingTemplate = true;
		try {
			const payload: CreatePermissionTemplatePayload = {
				name: newTemplateName.trim(),
				description: newTemplateDescription.trim() || undefined,
				tier: newTemplateTier,
				allowed_namespaces: newTemplateNamespaces.trim()
					? newTemplateNamespaces.split(',').map(s => s.trim())
					: undefined,
				allowed_tags: newTemplateTags.trim()
					? newTemplateTags.split(',').map(s => s.trim())
					: undefined,
				allowed_kinds: newTemplateKinds.trim()
					? newTemplateKinds.split(',').map(s => s.trim())
					: undefined
			};
			await createPermissionTemplate(payload);
			newTemplateName = '';
			newTemplateDescription = '';
			newTemplateTier = 'view';
			newTemplateNamespaces = '';
			newTemplateTags = '';
			newTemplateKinds = '';
			showTemplateForm = false;
			await loadData();
			pushToast('Permission template created', 'success');
		} catch {
			pushToast('Failed to create template', 'danger');
		} finally {
			isCreatingTemplate = false;
		}
	}

	async function handleDeleteTemplate(id: string) {
		try {
			await deletePermissionTemplate(id);
			templates = templates.filter(t => t.id !== id);
			pushToast('Template deleted', 'success');
		} catch {
			pushToast('Failed to delete template', 'danger');
		}
	}

	function copyToClipboard(text: string) {
		navigator.clipboard.writeText(text);
		pushToast('Copied to clipboard', 'success');
	}

	function formatDate(iso: string | null | undefined): string {
		if (!iso) return '--';
		return new Date(iso).toLocaleDateString(undefined, {
			year: 'numeric',
			month: 'short',
			day: 'numeric'
		});
	}

	const TIER_COLORS: Record<string, string> = {
		view: 'bg-emerald-500/20 text-emerald-300',
		edit: 'bg-sky-500/20 text-sky-300',
		action: 'bg-amber-500/20 text-amber-300',
		admin: 'bg-red-500/20 text-red-300'
	};
</script>

<div class="mx-auto max-w-3xl">
	<div class="flex items-center justify-between">
		<div>
			<h2 class="text-lg font-semibold text-white">Profiles &amp; Access Keys</h2>
			<p class="mt-1 text-xs text-slate-400">Manage API access keys and permission templates.</p>
		</div>
		<a
			href="/settings"
			class="rounded-lg border border-slate-700 px-3 py-1.5 text-xs text-slate-300 transition hover:bg-slate-800"
		>
			Back to Settings
		</a>
	</div>

	{#if loading}
		<div class="mt-12 text-center text-sm text-slate-400">Loading...</div>
	{:else}
		<div class="mt-6 flex flex-col gap-6">
			<!-- Revealed key banner -->
			{#if revealedKey}
				<section class="rounded-xl border border-emerald-500/40 bg-emerald-500/10 p-5">
					<h3 class="text-sm font-semibold text-emerald-200">New Access Key Created</h3>
					<p class="mt-1 text-[11px] text-emerald-300/70">
						Copy this key now. It will not be shown again.
					</p>
					<div class="mt-3 flex items-center gap-2">
						<code class="flex-1 rounded-lg border border-emerald-500/30 bg-slate-900 px-3 py-2 text-xs text-emerald-200 break-all">
							{revealedKey.raw}
						</code>
						<button
							class="rounded-lg bg-emerald-500/20 px-3 py-2 text-xs text-emerald-200 hover:bg-emerald-500/30"
							on:click={() => revealedKey && copyToClipboard(revealedKey.raw)}
						>
							Copy
						</button>
					</div>
					<button
						class="mt-2 text-[10px] text-emerald-400 hover:underline"
						on:click={() => { revealedKey = null; }}
					>
						Dismiss
					</button>
				</section>
			{/if}

			<!-- Access Keys -->
			<section class="rounded-xl border border-slate-800 bg-slate-900/40 p-5">
				<h3 class="text-sm font-semibold text-white">Access Keys</h3>
				<p class="mt-1 text-[11px] text-slate-400">
					Generate API keys for external integrations and automation.
				</p>

				{#if accessKeys.length > 0}
					<div class="mt-4 overflow-x-auto">
						<table class="w-full text-xs">
							<thead>
								<tr class="border-b border-slate-800 text-left text-[10px] uppercase tracking-wider text-slate-500">
									<th class="pb-2 pr-4">Name</th>
									<th class="pb-2 pr-4">Template</th>
									<th class="pb-2 pr-4">Created</th>
									<th class="pb-2 pr-4">Last Used</th>
									<th class="pb-2 pr-4">Expires</th>
									<th class="pb-2"></th>
								</tr>
							</thead>
							<tbody>
								{#each accessKeys as key (key.id)}
									<tr class="border-b border-slate-800/50">
										<td class="py-2.5 pr-4">
											<div class="font-medium text-white">{key.name}</div>
											{#if key.description}
												<div class="text-[10px] text-slate-500">{key.description}</div>
											{/if}
										</td>
										<td class="py-2.5 pr-4 text-slate-400">
											{templates.find(t => t.id === key.permission_template_id)?.name ?? '--'}
										</td>
										<td class="py-2.5 pr-4 text-slate-400">{formatDate(key.created_at)}</td>
										<td class="py-2.5 pr-4 text-slate-400">{formatDate(key.last_used_at)}</td>
										<td class="py-2.5 pr-4 text-slate-400">{formatDate(key.expires_at)}</td>
										<td class="py-2.5">
											<button
												class="rounded-lg border border-red-500/30 bg-red-500/10 px-2 py-1 text-[10px] text-red-300 hover:bg-red-500/20"
												on:click={() => handleRevokeKey(key.id)}
											>
												Revoke
											</button>
										</td>
									</tr>
								{/each}
							</tbody>
						</table>
					</div>
				{:else}
					<div class="mt-4 rounded-lg border border-dashed border-slate-800 p-4 text-center text-xs text-slate-500">
						No access keys yet
					</div>
				{/if}

				<!-- Create key form -->
				<div class="mt-4 rounded-lg border border-slate-800 bg-slate-800/30 p-4">
					<h4 class="text-xs font-semibold text-white">Create New Key</h4>
					<div class="mt-3 grid grid-cols-2 gap-3">
						<div>
							<label class="text-[10px] uppercase tracking-wider text-slate-500" for="key-name">Name</label>
							<input
								id="key-name"
								class="mt-1 w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white outline-none focus:border-sky-500"
								placeholder="e.g. CI Pipeline"
								bind:value={newKeyName}
							/>
						</div>
						<div>
							<label class="text-[10px] uppercase tracking-wider text-slate-500" for="key-template">Permission Template</label>
							<select
								id="key-template"
								class="mt-1 w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white"
								bind:value={newKeyTemplateId}
							>
								<option value="">None</option>
								{#each templates as tmpl (tmpl.id)}
									<option value={tmpl.id}>{tmpl.name} ({tmpl.tier})</option>
								{/each}
							</select>
						</div>
						<div>
							<label class="text-[10px] uppercase tracking-wider text-slate-500" for="key-desc">Description</label>
							<input
								id="key-desc"
								class="mt-1 w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white outline-none focus:border-sky-500"
								placeholder="Optional"
								bind:value={newKeyDescription}
							/>
						</div>
						<div>
							<label class="text-[10px] uppercase tracking-wider text-slate-500" for="key-expires">Expires</label>
							<input
								id="key-expires"
								type="date"
								class="mt-1 w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white outline-none focus:border-sky-500"
								bind:value={newKeyExpires}
							/>
						</div>
					</div>
					<button
						class="mt-3 rounded-lg bg-sky-500 px-3 py-2 text-xs font-semibold text-white hover:bg-sky-400 disabled:opacity-50"
						on:click={handleCreateKey}
						disabled={isCreatingKey}
					>
						{isCreatingKey ? 'Creating...' : 'Create Key'}
					</button>
				</div>
			</section>

			<!-- Permission Templates -->
			<section class="rounded-xl border border-slate-800 bg-slate-900/40 p-5">
				<div class="flex items-center justify-between">
					<div>
						<h3 class="text-sm font-semibold text-white">Permission Templates</h3>
						<p class="mt-1 text-[11px] text-slate-400">
							Define reusable permission sets for access keys.
						</p>
					</div>
					<button
						class="rounded-lg border border-sky-500/30 bg-sky-500/10 px-3 py-1.5 text-xs text-sky-300 hover:bg-sky-500/20"
						on:click={() => { showTemplateForm = !showTemplateForm; }}
					>
						{showTemplateForm ? 'Cancel' : 'New Template'}
					</button>
				</div>

				{#if showTemplateForm}
					<div class="mt-4 rounded-lg border border-slate-800 bg-slate-800/30 p-4">
						<div class="grid grid-cols-2 gap-3">
							<div>
								<label class="text-[10px] uppercase tracking-wider text-slate-500" for="tmpl-name">Name</label>
								<input
									id="tmpl-name"
									class="mt-1 w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white outline-none focus:border-sky-500"
									placeholder="e.g. Read Only"
									bind:value={newTemplateName}
								/>
							</div>
							<div>
								<label class="text-[10px] uppercase tracking-wider text-slate-500" for="tmpl-tier">Tier</label>
								<select
									id="tmpl-tier"
									class="mt-1 w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white"
									bind:value={newTemplateTier}
								>
									<option value="view">View</option>
									<option value="edit">Edit</option>
									<option value="action">Action</option>
									<option value="admin">Admin</option>
								</select>
							</div>
							<div class="col-span-2">
								<label class="text-[10px] uppercase tracking-wider text-slate-500" for="tmpl-desc">Description</label>
								<input
									id="tmpl-desc"
									class="mt-1 w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white outline-none focus:border-sky-500"
									placeholder="Optional"
									bind:value={newTemplateDescription}
								/>
							</div>
							<div>
								<label class="text-[10px] uppercase tracking-wider text-slate-500" for="tmpl-ns">Allowed Namespaces</label>
								<input
									id="tmpl-ns"
									class="mt-1 w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white outline-none focus:border-sky-500"
									placeholder="Comma-separated"
									bind:value={newTemplateNamespaces}
								/>
							</div>
							<div>
								<label class="text-[10px] uppercase tracking-wider text-slate-500" for="tmpl-tags">Allowed Tags</label>
								<input
									id="tmpl-tags"
									class="mt-1 w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white outline-none focus:border-sky-500"
									placeholder="Comma-separated"
									bind:value={newTemplateTags}
								/>
							</div>
							<div>
								<label class="text-[10px] uppercase tracking-wider text-slate-500" for="tmpl-kinds">Allowed Kinds</label>
								<input
									id="tmpl-kinds"
									class="mt-1 w-full rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white outline-none focus:border-sky-500"
									placeholder="e.g. task, fact"
									bind:value={newTemplateKinds}
								/>
							</div>
						</div>
						<button
							class="mt-3 rounded-lg bg-sky-500 px-3 py-2 text-xs font-semibold text-white hover:bg-sky-400 disabled:opacity-50"
							on:click={handleCreateTemplate}
							disabled={isCreatingTemplate}
						>
							{isCreatingTemplate ? 'Creating...' : 'Create Template'}
						</button>
					</div>
				{/if}

				{#if templates.length > 0}
					<div class="mt-4 flex flex-col gap-2">
						{#each templates as tmpl (tmpl.id)}
							<div class="flex items-center justify-between rounded-lg border border-slate-800/60 px-4 py-3">
								<div>
									<div class="flex items-center gap-2">
										<span class="text-xs font-medium text-white">{tmpl.name}</span>
										<span class={`rounded-full px-2 py-0.5 text-[10px] ${TIER_COLORS[tmpl.tier] ?? 'bg-slate-500/20 text-slate-300'}`}>
											{tmpl.tier}
										</span>
									</div>
									{#if tmpl.description}
										<div class="mt-0.5 text-[10px] text-slate-500">{tmpl.description}</div>
									{/if}
									<div class="mt-1 flex flex-wrap gap-1 text-[10px] text-slate-500">
										{#if tmpl.allowed_namespaces?.length}
											<span>Namespaces: {tmpl.allowed_namespaces.join(', ')}</span>
										{/if}
										{#if tmpl.allowed_tags?.length}
											<span>Tags: {tmpl.allowed_tags.join(', ')}</span>
										{/if}
										{#if tmpl.allowed_kinds?.length}
											<span>Kinds: {tmpl.allowed_kinds.join(', ')}</span>
										{/if}
									</div>
								</div>
								<button
									class="rounded-lg border border-red-500/30 bg-red-500/10 px-2 py-1 text-[10px] text-red-300 hover:bg-red-500/20"
									on:click={() => handleDeleteTemplate(tmpl.id)}
								>
									Delete
								</button>
							</div>
						{/each}
					</div>
				{:else if !showTemplateForm}
					<div class="mt-4 rounded-lg border border-dashed border-slate-800 p-4 text-center text-xs text-slate-500">
						No permission templates yet
					</div>
				{/if}
			</section>
		</div>
	{/if}
</div>
