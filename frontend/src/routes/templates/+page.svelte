<script lang="ts">
	import { goto } from '$app/navigation';
	import type { KnowledgeNode } from '$lib/api/types';
	import {
		createTemplate,
		deleteTemplate,
		getTemplateVersion,
		installTemplatePack,
		instantiateTemplate,
		listTemplateVersions,
		listTemplatePacks,
		listTemplates,
		readTemplateKey,
		readTemplateTargetKind,
		readTemplateVariables,
		restoreTemplateVersion,
		type TemplatePackSummary,
		type TemplateVersionDetail,
		type TemplateVersionSummary
	} from '$lib/api/templates';
	import { pushToast } from '$lib/stores/toast';
	import { onMount } from 'svelte';

	type TemplateKind = 'fact' | 'task' | 'event' | 'decision' | 'procedure' | 'bookmark';

	const kindOptions: Array<{ value: TemplateKind; label: string }> = [
		{ value: 'fact', label: 'Note' },
		{ value: 'task', label: 'Task' },
		{ value: 'event', label: 'Event' },
		{ value: 'decision', label: 'Decision' },
		{ value: 'procedure', label: 'Procedure' },
		{ value: 'bookmark', label: 'Bookmark' }
	];

	let loading = true;
	let templates: KnowledgeNode[] = [];
	let packs: TemplatePackSummary[] = [];

	let search = '';
	let namespaceFilter = 'all';
	let selectedTemplateId: string | null = null;

	let createTitle = '';
	let createKind: TemplateKind = 'fact';
	let createNamespace = 'default';
	let createTemplateKey = '';
	let createVariables = '';
	let createTags = '';
	let createContent = '';
	let creating = false;

	let instantiateNamespace = '';
	let instantiateTitle = '';
	let instantiateTags = '';
	let instantiateValues: Record<string, string> = {};
	let instantiating = false;
	let deletePending = false;
	let duplicating = false;
	let installingPackId: string | null = null;
	let overwriteExisting = false;
	let selectedTemplateMarker: string | null = null;
	let versionsTemplateMarker: string | null = null;
	let templateVersions: TemplateVersionSummary[] = [];
	let versionsLoading = false;
	let selectedVersionId: string | null = null;
	let selectedVersionDetail: TemplateVersionDetail | null = null;
	let versionDetailLoading = false;
	let restoringVersion = false;

	$: namespaces = Array.from(
		new Set(templates.map((template) => template.namespace || 'default'))
	).sort((left, right) => left.localeCompare(right));

	$: filteredTemplates = templates.filter((template) => {
		const matchesNamespace = namespaceFilter === 'all' || (template.namespace || 'default') === namespaceFilter;
		if (!matchesNamespace) return false;
		if (!search.trim()) return true;
		const term = search.trim().toLowerCase();
		return (
			(template.title || '').toLowerCase().includes(term) ||
			(template.content || '').toLowerCase().includes(term) ||
			(template.tags || []).some((tag) => tag.toLowerCase().includes(term))
		);
	});

	$: selectedTemplate = templates.find((template) => template.id === selectedTemplateId) ?? null;
	$: templateVariables = selectedTemplate ? deriveVariables(selectedTemplate) : [];

	$: if (filteredTemplates.length > 0 && !selectedTemplateId) {
		selectedTemplateId = filteredTemplates[0].id;
	}

	$: if (selectedTemplate && selectedTemplateMarker !== selectedTemplate.id) {
		selectedTemplateMarker = selectedTemplate.id;
		instantiateNamespace = selectedTemplate.namespace || 'default';
		instantiateTitle = '';
		instantiateTags = '';
		instantiateValues = Object.fromEntries(templateVariables.map((name) => [name, '']));
	}

	$: if (selectedTemplate && versionsTemplateMarker !== selectedTemplate.id) {
		versionsTemplateMarker = selectedTemplate.id;
		void refreshTemplateVersions(selectedTemplate.id);
	}

	onMount(async () => {
		await Promise.all([refreshTemplates(), refreshPacks()]);
	});

	function deriveVariables(template: KnowledgeNode): string[] {
		const fromMetadata = readTemplateVariables(template);
		if (fromMetadata.length > 0) return fromMetadata;
		const scan = `${template.title ?? ''}\n${template.content ?? ''}`;
		const matches = scan.matchAll(/\{\{\s*([a-zA-Z0-9_-]+)\s*\}\}/g);
		const values = new Set<string>();
		for (const match of matches) {
			const name = match[1]?.trim();
			if (!name) continue;
			values.add(name.toLowerCase());
		}
		return [...values];
	}

	function parseList(raw: string): string[] {
		return raw
			.split(',')
			.map((item) => item.trim())
			.filter((item) => item.length > 0);
	}

	function selectTemplate(templateId: string) {
		selectedTemplateId = templateId;
	}

	function formatDate(value: string): string {
		const parsed = new Date(value);
		if (Number.isNaN(parsed.getTime())) return value;
		return parsed.toLocaleString(undefined, {
			year: 'numeric',
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		});
	}

	async function refreshTemplates() {
		loading = true;
		try {
			templates = await listTemplates({ limit: 500 });
			if (!selectedTemplateId && templates[0]) {
				selectedTemplateId = templates[0].id;
			}
			if (selectedTemplateId && !templates.some((template) => template.id === selectedTemplateId)) {
				selectedTemplateId = templates[0]?.id ?? null;
			}
		} catch {
			pushToast('Failed to load templates', 'danger');
		} finally {
			loading = false;
		}
	}

	async function refreshPacks() {
		try {
			packs = await listTemplatePacks();
		} catch {
			pushToast('Failed to load template packs', 'warning');
		}
	}

	async function refreshTemplateVersions(templateId: string) {
		versionsLoading = true;
		selectedVersionDetail = null;
		try {
			templateVersions = await listTemplateVersions(templateId);
			if (templateVersions.length === 0) {
				selectedVersionId = null;
				return;
			}
			const nextVersionId =
				selectedVersionId && templateVersions.some((version) => version.version_id === selectedVersionId)
					? selectedVersionId
					: templateVersions[0].version_id;
			selectedVersionId = nextVersionId;
			await loadTemplateVersionDetail(templateId, nextVersionId);
		} catch {
			templateVersions = [];
			selectedVersionId = null;
			selectedVersionDetail = null;
			pushToast('Failed to load template history', 'warning');
		} finally {
			versionsLoading = false;
		}
	}

	async function loadTemplateVersionDetail(templateId: string, versionId: string) {
		versionDetailLoading = true;
		try {
			selectedVersionDetail = await getTemplateVersion(templateId, versionId);
		} catch {
			selectedVersionDetail = null;
			pushToast('Failed to load version detail', 'warning');
		} finally {
			versionDetailLoading = false;
		}
	}

	async function selectVersion(versionId: string) {
		if (!selectedTemplate) return;
		selectedVersionId = versionId;
		await loadTemplateVersionDetail(selectedTemplate.id, versionId);
	}

	async function restoreSelectedVersion() {
		if (!selectedTemplate || !selectedVersionId) return;
		if (!confirm('Restore this version to become the active template content?')) return;
		restoringVersion = true;
		try {
			await restoreTemplateVersion(selectedTemplate.id, selectedVersionId);
			pushToast('Template version restored', 'success');
			await refreshTemplates();
			await refreshTemplateVersions(selectedTemplate.id);
		} catch {
			pushToast('Failed to restore template version', 'danger');
		} finally {
			restoringVersion = false;
		}
	}

	async function submitCreateTemplate() {
		const content = createContent.trim();
		if (!content) {
			pushToast('Template content is required', 'warning');
			return;
		}

		creating = true;
		try {
			const created = await createTemplate({
				kind: createKind,
				content,
				title: createTitle.trim() || undefined,
				namespace: createNamespace.trim() || undefined,
				tags: parseList(createTags),
				template_key: createTemplateKey.trim() || undefined,
				template_variables: parseList(createVariables)
			});
			pushToast('Template created', 'success');
			createTitle = '';
			createTemplateKey = '';
			createVariables = '';
			createTags = '';
			createContent = '';
			await refreshTemplates();
			selectedTemplateId = created.id;
		} catch {
			pushToast('Failed to create template', 'danger');
		} finally {
			creating = false;
		}
	}

	async function submitInstantiateTemplate() {
		if (!selectedTemplate) return;
		instantiating = true;
		try {
			const values = Object.fromEntries(
				Object.entries(instantiateValues)
					.map(([key, value]) => [key.trim(), value.trim()])
					.filter(([key, value]) => key.length > 0 && value.length > 0)
			);
			const created = await instantiateTemplate(selectedTemplate.id, {
				namespace: instantiateNamespace.trim() || undefined,
				title: instantiateTitle.trim() || undefined,
				tags: parseList(instantiateTags),
				values: Object.keys(values).length > 0 ? values : undefined
			});
			pushToast('Template instantiated', 'success');
			if (created.kind === 'task') {
				await goto(`/tasks?task=${created.id}`);
			} else {
				await goto(`/notes?note=${created.id}`);
			}
		} catch {
			pushToast('Failed to instantiate template', 'danger');
		} finally {
			instantiating = false;
		}
	}

	async function removeSelectedTemplate() {
		if (!selectedTemplate) return;
		const title = selectedTemplate.title || selectedTemplate.id;
		if (!confirm(`Delete template "${title}"?`)) return;
		deletePending = true;
		try {
			await deleteTemplate(selectedTemplate.id);
			pushToast('Template deleted', 'success');
			await refreshTemplates();
		} catch {
			pushToast('Failed to delete template', 'danger');
		} finally {
			deletePending = false;
		}
	}

	async function duplicateSelectedTemplate() {
		if (!selectedTemplate) return;
		duplicating = true;
		try {
			const original = selectedTemplate;
			const variables = readTemplateVariables(original);
			const key = readTemplateKey(original);
			const created = await createTemplate({
				kind: original.kind,
				content: original.content ?? '',
				title: (original.title || 'Untitled template') + ' (Copy)',
				namespace: original.namespace || undefined,
				tags: original.tags ?? [],
				template_key: key ? key + '-copy' : undefined,
				template_variables: variables.length > 0 ? variables : undefined
			});
			pushToast('Template duplicated', 'success');
			await refreshTemplates();
			selectedTemplateId = created.id;
		} catch {
			pushToast('Failed to duplicate template', 'danger');
		} finally {
			duplicating = false;
		}
	}

	async function installPack(packId: string) {
		installingPackId = packId;
		try {
			const result = await installTemplatePack(packId, {
				namespace: instantiateNamespace.trim() || createNamespace.trim() || 'default',
				overwrite_existing: overwriteExisting
			});
			pushToast(
				`Pack installed: +${result.installed_templates} · updated ${result.updated_templates} · skipped ${result.skipped_templates}`,
				'success'
			);
			await refreshTemplates();
		} catch {
			pushToast('Template pack installation failed', 'danger');
		} finally {
			installingPackId = null;
		}
	}
</script>

<div class="grid gap-6 lg:grid-cols-12">
	<section class="lg:col-span-4">
		<div class="rounded-2xl border border-slate-900 bg-slate-900/40 p-4">
			<h2 class="text-sm font-semibold text-white">Templates</h2>
			<p class="mt-1 text-[11px] text-slate-400">
				Reusable blueprints for notes, tasks, and workflow checklists.
			</p>

			<div class="mt-4 flex flex-col gap-2">
				<input
					class="w-full rounded-lg border border-slate-800 bg-slate-900 px-3 py-2 text-xs text-white outline-none focus:border-sky-500"
					placeholder="Search templates"
					bind:value={search}
				/>
				<select
					class="w-full rounded-lg border border-slate-800 bg-slate-900 px-3 py-2 text-xs text-white outline-none focus:border-sky-500"
					bind:value={namespaceFilter}
				>
					<option value="all">All namespaces</option>
					{#each namespaces as namespace (namespace)}
						<option value={namespace}>{namespace}</option>
					{/each}
				</select>
			</div>

			<div class="mt-4 max-h-[50vh] space-y-2 overflow-y-auto pr-1">
				{#if loading}
					<p class="text-xs text-slate-500">Loading templates…</p>
				{:else if filteredTemplates.length === 0}
					<p class="text-xs text-slate-500">No templates found.</p>
				{:else}
					{#each filteredTemplates as template (template.id)}
						<button
							class={`w-full rounded-lg border px-3 py-2 text-left transition ${
								selectedTemplateId === template.id
									? 'border-sky-500/50 bg-sky-500/10'
									: 'border-slate-800 hover:border-slate-700 hover:bg-slate-800/60'
							}`}
							on:click={() => selectTemplate(template.id)}
						>
							<div class="flex items-center justify-between gap-2">
								<span class="truncate text-xs font-medium text-white">
									{template.title || 'Untitled template'}
								</span>
								<span class="rounded bg-slate-800 px-1.5 py-0.5 text-[9px] text-slate-400">
									{readTemplateTargetKind(template) ?? template.kind}
								</span>
							</div>
							<div class="mt-1 truncate text-[10px] text-slate-500">
								{template.namespace}
								{#if readTemplateKey(template)}
									· key: {readTemplateKey(template)}
								{/if}
							</div>
						</button>
					{/each}
				{/if}
			</div>
		</div>

		<div class="mt-4 rounded-2xl border border-slate-900 bg-slate-900/40 p-4">
			<div class="flex items-center justify-between gap-2">
				<h3 class="text-xs font-semibold uppercase tracking-wide text-slate-400">Template Packs</h3>
				<label class="flex items-center gap-2 text-[10px] text-slate-400">
					<input type="checkbox" bind:checked={overwriteExisting} />
					Overwrite
				</label>
			</div>
			<div class="mt-3 space-y-2">
				{#if packs.length === 0}
					<p class="text-xs text-slate-500">No packs available.</p>
				{:else}
					{#each packs as pack (pack.pack_id)}
						<div class="rounded-lg border border-slate-800 p-3">
							<div class="text-xs font-medium text-white">{pack.name}</div>
							<div class="mt-1 text-[10px] text-slate-500">{pack.description}</div>
							<div class="mt-2 flex items-center justify-between">
								<span class="text-[10px] text-slate-500">{pack.template_count} templates</span>
								<button
									class="rounded-lg bg-slate-800 px-2 py-1 text-[10px] text-slate-200 hover:bg-slate-700 disabled:opacity-60"
									disabled={installingPackId === pack.pack_id}
									on:click={() => installPack(pack.pack_id)}
								>
									{installingPackId === pack.pack_id ? 'Installing…' : 'Install'}
								</button>
							</div>
						</div>
					{/each}
				{/if}
			</div>
		</div>
	</section>

	<section class="space-y-4 lg:col-span-8">
		<div class="rounded-2xl border border-slate-900 bg-slate-900/40 p-5">
			<div class="flex items-center justify-between">
				<div>
					<h3 class="text-sm font-semibold text-white">Instantiate Template</h3>
					<p class="mt-1 text-[11px] text-slate-400">
						{#if selectedTemplate}
							Using: {selectedTemplate.title || selectedTemplate.id}
						{:else}
							Select a template to generate a new note or task.
						{/if}
					</p>
				</div>
				<div class="flex items-center gap-2">
					<button
						class="rounded-lg border border-slate-700 px-3 py-1.5 text-xs text-slate-200 hover:bg-slate-800 disabled:opacity-60"
						disabled={!selectedTemplate || duplicating}
						on:click={duplicateSelectedTemplate}
					>
						{duplicating ? 'Duplicating…' : 'Duplicate'}
					</button>
					<button
						class="rounded-lg border border-red-500/30 px-3 py-1.5 text-xs text-red-300 hover:bg-red-500/10 disabled:opacity-60"
						disabled={!selectedTemplate || deletePending}
						on:click={removeSelectedTemplate}
					>
						{deletePending ? 'Deleting…' : 'Delete template'}
					</button>
				</div>
			</div>

			{#if selectedTemplate}
				<div class="mt-4 grid gap-3 md:grid-cols-2">
					<label class="text-xs text-slate-300">
						Namespace
						<input
							class="mt-1 w-full rounded-lg border border-slate-800 bg-slate-900 px-3 py-2 text-xs text-white outline-none focus:border-sky-500"
							bind:value={instantiateNamespace}
						/>
					</label>
					<label class="text-xs text-slate-300">
						Title Override
						<input
							class="mt-1 w-full rounded-lg border border-slate-800 bg-slate-900 px-3 py-2 text-xs text-white outline-none focus:border-sky-500"
							placeholder="Optional custom title"
							bind:value={instantiateTitle}
						/>
					</label>
				</div>

				<label class="mt-3 block text-xs text-slate-300">
					Extra Tags (comma-separated)
					<input
						class="mt-1 w-full rounded-lg border border-slate-800 bg-slate-900 px-3 py-2 text-xs text-white outline-none focus:border-sky-500"
						placeholder="project-x, weekly"
						bind:value={instantiateTags}
					/>
				</label>

				{#if templateVariables.length > 0}
					<div class="mt-4">
						<h4 class="text-xs font-semibold uppercase tracking-wide text-slate-400">Variables</h4>
						<div class="mt-2 grid gap-2 md:grid-cols-2">
							{#each templateVariables as variable (variable)}
								<label class="text-xs text-slate-300">
									{variable}
									<input
										class="mt-1 w-full rounded-lg border border-slate-800 bg-slate-900 px-3 py-2 text-xs text-white outline-none focus:border-sky-500"
										bind:value={instantiateValues[variable]}
									/>
								</label>
							{/each}
						</div>
					</div>
				{/if}

				<div class="mt-4">
					<button
						class="rounded-lg bg-sky-500 px-4 py-2 text-xs font-semibold text-white hover:bg-sky-400 disabled:opacity-60"
						disabled={instantiating}
						on:click={submitInstantiateTemplate}
					>
						{instantiating ? 'Instantiating…' : 'Create Instance'}
					</button>
				</div>
			{/if}
		</div>

		<div class="rounded-2xl border border-slate-900 bg-slate-900/40 p-5">
			<div class="flex items-center justify-between gap-2">
				<div>
					<h3 class="text-sm font-semibold text-white">Template History</h3>
					<p class="mt-1 text-[11px] text-slate-400">
						Inspect prior revisions and restore older template states.
					</p>
				</div>
				<button
					class="rounded-lg border border-slate-700 px-3 py-1.5 text-[11px] text-slate-200 hover:bg-slate-800 disabled:opacity-60"
					disabled={!selectedTemplate || versionsLoading}
					on:click={() => selectedTemplate && void refreshTemplateVersions(selectedTemplate.id)}
				>
					Refresh
				</button>
			</div>

			{#if !selectedTemplate}
				<p class="mt-3 text-xs text-slate-500">Select a template to view version history.</p>
			{:else}
				<div class="mt-4 grid gap-4 md:grid-cols-2">
					<div class="max-h-64 space-y-2 overflow-y-auto pr-1">
						{#if versionsLoading}
							<p class="text-xs text-slate-500">Loading version history…</p>
						{:else if templateVersions.length === 0}
							<p class="text-xs text-slate-500">No saved versions yet.</p>
						{:else}
							{#each templateVersions as version (version.version_id)}
								<button
									class={`w-full rounded-lg border px-3 py-2 text-left transition ${
										selectedVersionId === version.version_id
											? 'border-sky-500/50 bg-sky-500/10'
											: 'border-slate-800 hover:border-slate-700 hover:bg-slate-800/60'
									}`}
									on:click={() => void selectVersion(version.version_id)}
								>
									<div class="flex items-center justify-between gap-2">
										<span class="truncate text-xs font-medium text-white">
											{version.title || 'Untitled'}
										</span>
										<span class="text-[10px] text-slate-500">{version.kind}</span>
									</div>
									<div class="mt-1 text-[10px] text-slate-500">{formatDate(version.captured_at)}</div>
									<div class="mt-1 line-clamp-2 text-[10px] text-slate-500">{version.content_preview}</div>
								</button>
							{/each}
						{/if}
					</div>

					<div class="rounded-lg border border-slate-800 bg-slate-900/60 p-3">
						{#if versionDetailLoading}
							<p class="text-xs text-slate-500">Loading version detail…</p>
						{:else if selectedVersionDetail}
							<div class="flex items-center justify-between gap-2">
								<h4 class="text-xs font-semibold text-white">
									Version {selectedVersionDetail.version.version_id.slice(0, 8)}
								</h4>
								<button
									class="rounded-lg border border-amber-500/30 px-2 py-1 text-[10px] text-amber-300 hover:bg-amber-500/10 disabled:opacity-60"
									disabled={restoringVersion}
									on:click={restoreSelectedVersion}
								>
									{restoringVersion ? 'Restoring…' : 'Restore'}
								</button>
							</div>
							<div class="mt-2 grid grid-cols-2 gap-2 text-[10px] text-slate-400">
								<div class="rounded border border-slate-800 px-2 py-1">
									Added lines: {selectedVersionDetail.diff.added_line_count}
								</div>
								<div class="rounded border border-slate-800 px-2 py-1">
									Removed lines: {selectedVersionDetail.diff.removed_line_count}
								</div>
							</div>
							<div class="mt-3 space-y-1">
								{#each selectedVersionDetail.field_changes.filter((change) => change.changed) as change (change.field)}
									<div class="rounded border border-slate-800 px-2 py-1 text-[10px] text-slate-300">
										<span class="font-semibold text-white">{change.field}</span>
										<div class="mt-0.5 text-slate-500">
											{change.current_value} → {change.version_value}
										</div>
									</div>
								{/each}
								{#if selectedVersionDetail.field_changes.every((change) => !change.changed)}
									<p class="text-[10px] text-slate-500">No field differences from current template.</p>
								{/if}
							</div>
						{:else}
							<p class="text-xs text-slate-500">Select a version to inspect details.</p>
						{/if}
					</div>
				</div>
			{/if}
		</div>

		<div class="rounded-2xl border border-slate-900 bg-slate-900/40 p-5">
			<h3 class="text-sm font-semibold text-white">Create Template</h3>
			<p class="mt-1 text-[11px] text-slate-400">
				Use placeholders like <code>{'{{project_name}}'}</code> in title or content.
			</p>

			<div class="mt-4 grid gap-3 md:grid-cols-2">
				<label class="text-xs text-slate-300">
					Kind
					<select
						class="mt-1 w-full rounded-lg border border-slate-800 bg-slate-900 px-3 py-2 text-xs text-white outline-none focus:border-sky-500"
						bind:value={createKind}
					>
						{#each kindOptions as option (option.value)}
							<option value={option.value}>{option.label}</option>
						{/each}
					</select>
				</label>
				<label class="text-xs text-slate-300">
					Namespace
					<input
						class="mt-1 w-full rounded-lg border border-slate-800 bg-slate-900 px-3 py-2 text-xs text-white outline-none focus:border-sky-500"
						bind:value={createNamespace}
					/>
				</label>
				<label class="text-xs text-slate-300 md:col-span-2">
					Title
					<input
						class="mt-1 w-full rounded-lg border border-slate-800 bg-slate-900 px-3 py-2 text-xs text-white outline-none focus:border-sky-500"
						placeholder="Weekly planning - &#123;&#123;week_label&#125;&#125;"
						bind:value={createTitle}
					/>
				</label>
				<label class="text-xs text-slate-300">
					Template Key
					<input
						class="mt-1 w-full rounded-lg border border-slate-800 bg-slate-900 px-3 py-2 text-xs text-white outline-none focus:border-sky-500"
						placeholder="planning.weekly"
						bind:value={createTemplateKey}
					/>
				</label>
				<label class="text-xs text-slate-300">
					Variables (comma-separated)
					<input
						class="mt-1 w-full rounded-lg border border-slate-800 bg-slate-900 px-3 py-2 text-xs text-white outline-none focus:border-sky-500"
						placeholder="week_label, focus_area"
						bind:value={createVariables}
					/>
				</label>
				<label class="text-xs text-slate-300 md:col-span-2">
					Tags (comma-separated)
					<input
						class="mt-1 w-full rounded-lg border border-slate-800 bg-slate-900 px-3 py-2 text-xs text-white outline-none focus:border-sky-500"
						placeholder="planning, template"
						bind:value={createTags}
					/>
				</label>
			</div>

			<label class="mt-3 block text-xs text-slate-300">
				Template Content
				<textarea
					class="mt-1 min-h-[180px] w-full rounded-lg border border-slate-800 bg-slate-900 px-3 py-2 text-xs text-white outline-none focus:border-sky-500"
					placeholder="# Weekly Plan&#10;&#10;Focus: &#123;&#123;focus_area&#125;&#125;&#10;&#10;- [ ] ..."
					bind:value={createContent}
				></textarea>
			</label>

			<div class="mt-4">
				<button
					class="rounded-lg bg-sky-500 px-4 py-2 text-xs font-semibold text-white hover:bg-sky-400 disabled:opacity-60"
					disabled={creating}
					on:click={submitCreateTemplate}
				>
					{creating ? 'Creating…' : 'Save Template'}
				</button>
			</div>
		</div>
	</section>
</div>
