<script lang="ts">
	import { onMount } from 'svelte';
	import {
		getWorkspaceTree,
		listWorkspaces,
		mountWorkspace,
		readWorkspaceDocument,
		rebuildWorkspaceProjections,
		reconcileWorkspace,
		type WorkspaceDocumentRead,
		type WorkspaceSummary,
		type WorkspaceTree,
		type WorkspaceTreeEntry
	} from '$lib/api/workspaces';
	import { ApiError } from '$lib/api/client';
	import {
		pickWorkspaceDirectory,
		supportsNativeWorkspacePicker
	} from '$lib/desktop/workspace-picker';
	import { pushToast } from '$lib/stores/toast';
	import { workspaceReconciliation } from '$lib/stores/websocket';

	let workspaces: WorkspaceSummary[] = [];
	let selectedWorkspaceId = '';
	let tree: WorkspaceTree | null = null;
	let selectedDocument: WorkspaceDocumentRead | null = null;
	let loading = true;
	let refreshing = false;
	let rebuilding = false;
	let reading = false;
	let mounting = false;
	let choosingDirectory = false;
	let showMountForm = false;
	let nativePickerAvailable = false;
	let mountPath = '';
	let mountDisplayName = '';
	let mountError = '';
	let collapsedPaths = new Set<string>();
	let workspaceRefreshTimer: ReturnType<typeof setTimeout> | null = null;
	let filesystemRefreshSequence = 0;

	$: selectedWorkspace =
		workspaces.find((workspace) => workspace.id === selectedWorkspaceId) ?? null;
	$: visibleEntries = (tree?.entries ?? []).filter(isEntryVisible);
	$: documentCount = tree?.entries.filter((entry) => entry.kind === 'document').length ?? 0;
	$: folderCount = tree?.entries.filter((entry) => entry.kind === 'directory').length ?? 0;
	$: pendingProjectionCount =
		tree?.entries.filter(
			(entry) =>
				entry.kind === 'document' &&
				entry.projection_state !== null &&
				entry.projection_state !== 'ready'
		).length ?? 0;

	onMount(() => {
		nativePickerAvailable = supportsNativeWorkspacePicker();
		void loadWorkspaces();
		const unsubscribe = workspaceReconciliation.subscribe((event) => {
			if (!event || event.workspaceId !== selectedWorkspaceId) return;
			scheduleFilesystemRefresh(event.workspaceId);
		});
		return () => {
			unsubscribe();
			if (workspaceRefreshTimer) clearTimeout(workspaceRefreshTimer);
		};
	});

	function scheduleFilesystemRefresh(workspaceId: string) {
		if (workspaceRefreshTimer) clearTimeout(workspaceRefreshTimer);
		workspaceRefreshTimer = setTimeout(() => {
			workspaceRefreshTimer = null;
			void refreshFromFilesystem(workspaceId);
		}, 200);
	}

	async function refreshFromFilesystem(workspaceId: string) {
		if (workspaceId !== selectedWorkspaceId) return;
		const refreshSequence = ++filesystemRefreshSequence;
		const selectedDocumentId =
			selectedDocument?.workspace_id === workspaceId ? selectedDocument.document_id : null;
		refreshing = true;
		try {
			const [nextWorkspaces, nextTree] = await Promise.all([
				listWorkspaces(),
				getWorkspaceTree(workspaceId)
			]);
			if (workspaceId !== selectedWorkspaceId || refreshSequence !== filesystemRefreshSequence)
				return;
			workspaces = nextWorkspaces;
			tree = nextTree;

			if (selectedDocumentId) {
				const selectedEntry = nextTree.entries.find(
					(entry) => entry.document_id === selectedDocumentId
				);
				const nextDocument = selectedEntry?.document_id
					? await readWorkspaceDocument(workspaceId, selectedEntry.document_id)
					: null;
				if (
					workspaceId === selectedWorkspaceId &&
					refreshSequence === filesystemRefreshSequence &&
					selectedDocument?.document_id === selectedDocumentId
				) {
					selectedDocument = nextDocument;
				}
			}
		} catch (error) {
			console.warn('[workspace] automatic filesystem refresh failed', error);
		} finally {
			if (workspaceId === selectedWorkspaceId && refreshSequence === filesystemRefreshSequence)
				refreshing = false;
		}
	}

	async function loadWorkspaces() {
		loading = true;
		try {
			workspaces = await listWorkspaces();
			if (!workspaces.some((workspace) => workspace.id === selectedWorkspaceId)) {
				selectedWorkspaceId = workspaces[0]?.id ?? '';
			}
			if (selectedWorkspaceId) {
				await loadTree();
			} else {
				tree = null;
				selectedDocument = null;
			}
		} catch {
			pushToast('Unable to load mounted workspaces.', 'danger');
		} finally {
			loading = false;
		}
	}

	async function selectWorkspace(workspaceId: string) {
		selectedWorkspaceId = workspaceId;
		selectedDocument = null;
		collapsedPaths = new Set();
		await loadTree();
	}

	async function loadTree() {
		if (!selectedWorkspaceId) return;
		refreshing = true;
		try {
			tree = await getWorkspaceTree(selectedWorkspaceId);
		} catch {
			pushToast('Unable to scan the workspace tree.', 'danger');
		} finally {
			refreshing = false;
		}
	}

	async function reconcile() {
		if (!selectedWorkspaceId) return;
		refreshing = true;
		try {
			const result = await reconcileWorkspace(selectedWorkspaceId);
			workspaces = workspaces.map((workspace) =>
				workspace.id === result.workspace.id ? result.workspace : workspace
			);
			tree = await getWorkspaceTree(selectedWorkspaceId);
			const changes =
				result.reconciliation.inserted_documents +
				result.reconciliation.updated_documents +
				result.reconciliation.renamed_documents;
			pushToast(
				changes === 0
					? 'Workspace is already current.'
					: `Workspace reconciled: ${changes} change${changes === 1 ? '' : 's'}.`,
				'success'
			);
		} catch {
			pushToast('Workspace reconciliation failed.', 'danger');
		} finally {
			refreshing = false;
		}
	}

	async function rebuildProjections() {
		if (!selectedWorkspaceId) return;
		rebuilding = true;
		try {
			const result = await rebuildWorkspaceProjections(selectedWorkspaceId);
			tree = await getWorkspaceTree(selectedWorkspaceId);
			const projection = result.projection;
			pushToast(
				projection.failed_documents === 0
					? `Search index rebuilt for ${projection.projected_documents} document${projection.projected_documents === 1 ? '' : 's'}.`
					: `Index rebuild completed with ${projection.failed_documents} failed document${projection.failed_documents === 1 ? '' : 's'}.`,
				projection.failed_documents === 0 ? 'success' : 'danger'
			);
		} catch {
			pushToast('Workspace search index rebuild failed.', 'danger');
		} finally {
			rebuilding = false;
		}
	}

	async function chooseDirectory() {
		choosingDirectory = true;
		mountError = '';
		try {
			const selected = await pickWorkspaceDirectory();
			if (selected) mountPath = selected;
		} catch {
			mountError = 'MindVault could not open the system folder chooser. Enter the path below.';
		} finally {
			choosingDirectory = false;
		}
	}

	async function mountSelectedWorkspace() {
		const rootPath = mountPath.trim();
		if (!rootPath) {
			mountError = 'Choose a folder or enter its absolute path.';
			return;
		}

		mounting = true;
		mountError = '';
		try {
			const displayName = mountDisplayName.trim();
			const result = await mountWorkspace({
				root_path: rootPath,
				...(displayName ? { display_name: displayName } : {})
			});
			workspaces = [
				result.workspace,
				...workspaces.filter((workspace) => workspace.id !== result.workspace.id)
			];
			selectedWorkspaceId = result.workspace.id;
			selectedDocument = null;
			collapsedPaths = new Set();
			showMountForm = false;
			mountPath = '';
			mountDisplayName = '';
			await loadTree();

			const documents = result.reconciliation.inserted_documents;
			pushToast(
				`Workspace mounted with ${documents} document${documents === 1 ? '' : 's'}.`,
				'success'
			);
		} catch (error) {
			mountError = workspaceMountError(error);
		} finally {
			mounting = false;
		}
	}

	async function openDocument(entry: WorkspaceTreeEntry) {
		if (!selectedWorkspaceId || !entry.document_id) return;
		reading = true;
		try {
			selectedDocument = await readWorkspaceDocument(selectedWorkspaceId, entry.document_id);
		} catch {
			pushToast('Unable to read the canonical document.', 'danger');
		} finally {
			reading = false;
		}
	}

	function toggleDirectory(path: string) {
		const next = new Set(collapsedPaths);
		if (next.has(path)) next.delete(path);
		else next.add(path);
		collapsedPaths = next;
	}

	function isEntryVisible(entry: WorkspaceTreeEntry): boolean {
		let parent = entry.parent_path;
		while (parent) {
			if (collapsedPaths.has(parent)) return false;
			const separator = parent.lastIndexOf('/');
			parent = separator === -1 ? null : parent.slice(0, separator);
		}
		return true;
	}

	function depth(path: string): number {
		return path.split('/').length - 1;
	}

	function formatBytes(bytes: number | null): string {
		if (bytes === null) return '';
		if (bytes < 1024) return `${bytes} B`;
		if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
		return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
	}

	function workspaceMountError(error: unknown): string {
		if (!(error instanceof ApiError)) return 'Unable to mount this workspace.';
		if (error.status === 403) {
			return 'This folder is not authorized by the server. Add it or a parent folder to MINDVAULT_WORKSPACE_ALLOWED_ROOTS, restart MindVault, and try again.';
		}
		if (error.status === 409) return 'This folder is already mounted as a workspace.';
		if (typeof error.body === 'string' && error.body.trim()) return error.body;
		if (error.body && typeof error.body === 'object') {
			const body = error.body as { error?: unknown; message?: unknown };
			if (typeof body.message === 'string') return body.message;
			if (typeof body.error === 'string') return body.error;
		}
		return error.status === 0
			? 'MindVault could not reach the workspace service.'
			: 'Unable to mount this workspace.';
	}
</script>

<section
	class="mt-4 grid min-h-[34rem] gap-4 lg:grid-cols-12 lg:gap-6"
	aria-label="Knowledge files"
>
	<aside
		class="rounded-xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/50 p-3 lg:col-span-4"
	>
		<div class="flex items-center justify-between gap-3">
			<div>
				<p class="text-xs font-semibold uppercase tracking-[0.14em] text-sky-300">Workspaces</p>
				<p class="mt-1 text-[11px] text-[rgb(var(--mv-muted))]">Canonical Markdown files</p>
			</div>
			<div class="flex items-center gap-2">
				<button
					class="rounded-lg border border-sky-400/40 bg-sky-500/10 px-2.5 py-1.5 text-[11px] text-sky-200 hover:bg-sky-500/20"
					on:click={() => {
						showMountForm = !showMountForm;
						mountError = '';
					}}
					aria-expanded={showMountForm}
				>
					{showMountForm ? 'Cancel' : 'Add'}
				</button>
				<button
					class="rounded-lg border border-[rgb(var(--mv-border))] px-2.5 py-1.5 text-[11px] text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-panel-strong))]"
					on:click={reconcile}
					disabled={!selectedWorkspaceId || refreshing}
				>
					{refreshing ? 'Scanning…' : 'Reconcile'}
				</button>
				<button
					class="rounded-lg border border-[rgb(var(--mv-border))] px-2.5 py-1.5 text-[11px] text-[rgb(var(--mv-muted))] hover:bg-[rgb(var(--mv-panel-strong))]"
					on:click={rebuildProjections}
					disabled={!selectedWorkspaceId || rebuilding}
					title="Rebuild disposable search and graph indexes from canonical Markdown"
				>
					{rebuilding ? 'Indexing…' : 'Rebuild index'}
				</button>
			</div>
		</div>

		{#if showMountForm}
			<form
				class="mt-4 rounded-lg border border-sky-400/25 bg-sky-500/5 p-3"
				on:submit|preventDefault={mountSelectedWorkspace}
			>
				<p class="text-xs font-semibold text-[rgb(var(--mv-text))]">Mount a Markdown folder</p>
				<p class="mt-1 text-[11px] leading-4 text-[rgb(var(--mv-muted))]">
					MindVault indexes Markdown without moving or rewriting your files.
				</p>

				{#if nativePickerAvailable}
					<button
						type="button"
						class="mt-3 w-full rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))] px-3 py-2 text-xs text-[rgb(var(--mv-text))] hover:border-sky-400/50"
						on:click={chooseDirectory}
						disabled={choosingDirectory || mounting}
					>
						{choosingDirectory ? 'Opening chooser…' : 'Choose folder…'}
					</button>
				{/if}

				<label class="mt-3 block text-[11px] text-[rgb(var(--mv-muted))]" for="workspace-path">
					Absolute folder path
				</label>
				<input
					id="workspace-path"
					class="mt-1 w-full rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))] px-3 py-2 text-xs text-[rgb(var(--mv-text))] placeholder:text-[rgb(var(--mv-muted))]/60"
					bind:value={mountPath}
					placeholder="/Users/me/Knowledge"
					autocomplete="off"
					disabled={mounting}
				/>

				<label class="mt-3 block text-[11px] text-[rgb(var(--mv-muted))]" for="workspace-name">
					Display name <span class="opacity-70">(optional)</span>
				</label>
				<input
					id="workspace-name"
					class="mt-1 w-full rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))] px-3 py-2 text-xs text-[rgb(var(--mv-text))] placeholder:text-[rgb(var(--mv-muted))]/60"
					bind:value={mountDisplayName}
					placeholder="Uses the folder name"
					autocomplete="off"
					maxlength="120"
					disabled={mounting}
				/>

				{#if mountError}
					<p class="mt-3 text-[11px] leading-4 text-rose-300" role="alert">{mountError}</p>
				{/if}

				<button
					type="submit"
					class="mt-3 w-full rounded-lg bg-sky-500 px-3 py-2 text-xs font-semibold text-slate-950 hover:bg-sky-400 disabled:cursor-not-allowed disabled:opacity-60"
					disabled={mounting || !mountPath.trim()}
				>
					{mounting ? 'Mounting and indexing…' : 'Mount workspace'}
				</button>
			</form>
		{/if}

		{#if loading}
			<p class="mt-6 text-xs text-[rgb(var(--mv-muted))]">Loading workspaces…</p>
		{:else if workspaces.length === 0}
			<div class="mt-5 rounded-lg border border-dashed border-[rgb(var(--mv-border))] p-4">
				<p class="text-sm font-medium text-[rgb(var(--mv-text))]">No workspace mounted</p>
				<p class="mt-2 text-xs leading-5 text-[rgb(var(--mv-muted))]">
					Add a Markdown folder after authorizing it in the server’s workspace allowlist.
				</p>
				{#if !showMountForm}
					<button
						class="mt-3 text-xs font-medium text-sky-300 hover:text-sky-200"
						on:click={() => (showMountForm = true)}
					>
						Add your first workspace →
					</button>
				{/if}
			</div>
		{:else}
			<label class="mt-4 block text-[11px] text-[rgb(var(--mv-muted))]" for="workspace-select">
				Workspace
			</label>
			<select
				id="workspace-select"
				class="mt-1 w-full rounded-lg border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel-strong))] px-3 py-2 text-sm text-[rgb(var(--mv-text))]"
				value={selectedWorkspaceId}
				on:change={(event) => void selectWorkspace(event.currentTarget.value)}
			>
				{#each workspaces as workspace}
					<option value={workspace.id}>{workspace.display_name} · {workspace.root_name}</option>
				{/each}
			</select>

			<div class="mt-4 flex gap-3 text-[10px] text-[rgb(var(--mv-muted))]">
				<span>{folderCount} folders</span>
				<span>{documentCount} documents</span>
				{#if selectedWorkspace}<span>rev {selectedWorkspace.revision}</span>{/if}
				{#if pendingProjectionCount > 0}
					<span class="text-amber-300">{pendingProjectionCount} index pending</span>
				{/if}
			</div>

			<div class="mt-3 max-h-[31rem] overflow-y-auto pr-1" role="tree">
				{#each visibleEntries as entry (entry.kind + entry.relative_path)}
					<button
						class={`flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-xs transition hover:bg-[rgb(var(--mv-panel-strong))] ${
							selectedDocument?.document_id === entry.document_id
								? 'bg-sky-500/15 text-sky-200'
								: 'text-[rgb(var(--mv-text))]/90'
						}`}
						style={`padding-left: ${0.5 + depth(entry.relative_path) * 0.9}rem`}
						role="treeitem"
						aria-selected={selectedDocument?.document_id === entry.document_id}
						aria-expanded={entry.kind === 'directory'
							? !collapsedPaths.has(entry.relative_path)
							: undefined}
						on:click={() =>
							entry.kind === 'directory'
								? toggleDirectory(entry.relative_path)
								: void openDocument(entry)}
					>
						<span class="w-4 shrink-0 text-center text-[rgb(var(--mv-muted))]">
							{#if entry.kind === 'directory'}
								{collapsedPaths.has(entry.relative_path) ? '▸' : '▾'}
							{:else}
								◇
							{/if}
						</span>
						<span class="min-w-0 flex-1 truncate">{entry.name}</span>
						{#if entry.manifest_match === 'changed'}
							<span class="h-1.5 w-1.5 rounded-full bg-amber-400" title="Changed on disk"></span>
						{:else if entry.manifest_match === 'untracked'}
							<span class="h-1.5 w-1.5 rounded-full bg-violet-400" title="Not reconciled"></span>
						{/if}
						{#if entry.kind === 'document' && entry.projection_state === 'failed'}
							<span class="text-[10px] text-rose-300" title="Search and graph projection failed"
								>index error</span
							>
						{:else if entry.kind === 'document' && entry.projection_state !== 'ready'}
							<span class="text-[10px] text-amber-300" title="Search and graph projection pending"
								>indexing</span
							>
						{/if}
					</button>
				{/each}
			</div>
		{/if}
	</aside>

	<div
		class="rounded-xl border border-[rgb(var(--mv-border))] bg-[rgb(var(--mv-panel))]/40 p-5 lg:col-span-8"
	>
		{#if reading}
			<p class="text-sm text-[rgb(var(--mv-muted))]">Reading canonical file…</p>
		{:else if selectedDocument}
			<div
				class="flex flex-wrap items-start justify-between gap-3 border-b border-[rgb(var(--mv-border))] pb-4"
			>
				<div class="min-w-0">
					<p class="truncate text-lg font-semibold text-[rgb(var(--mv-text))]">
						{selectedDocument.relative_path.split('/').at(-1)}
					</p>
					<p class="mt-1 truncate text-xs text-[rgb(var(--mv-muted))]">
						{selectedWorkspace?.display_name} / {selectedDocument.relative_path}
					</p>
				</div>
				<div class="flex items-center gap-2 text-[10px]">
					<span
						class={`rounded-full px-2 py-1 ${
							selectedDocument.manifest_match === 'current'
								? 'bg-emerald-500/15 text-emerald-300'
								: 'bg-amber-500/15 text-amber-300'
						}`}
					>
						{selectedDocument.manifest_match}
					</span>
					<span class="text-[rgb(var(--mv-muted))]">{formatBytes(selectedDocument.byte_size)}</span>
				</div>
			</div>
			<pre
				class="mt-5 max-h-[29rem] overflow-auto whitespace-pre-wrap break-words font-mono text-sm leading-6 text-[rgb(var(--mv-text))]/90">{selectedDocument.content}</pre>
		{:else}
			<div class="flex h-full min-h-[28rem] items-center justify-center text-center">
				<div class="max-w-sm">
					<p class="text-lg font-semibold text-[rgb(var(--mv-text))]">Your files stay yours</p>
					<p class="mt-2 text-sm leading-6 text-[rgb(var(--mv-muted))]">
						Choose a Markdown document from the real folder tree. This view reads the canonical file
						directly and shows when it has changed since reconciliation.
					</p>
				</div>
			</div>
		{/if}
	</div>
</section>
