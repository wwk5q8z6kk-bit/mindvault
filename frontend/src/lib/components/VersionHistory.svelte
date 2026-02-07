<script lang="ts">
	import { createEventDispatcher, onMount } from 'svelte';
	import {
		listNodeVersions,
		getNodeVersion,
		restoreNodeVersion,
		compareVersions,
		formatVersionDate,
		formatVersionSize
	} from '$lib/api/versions';
	import type { NodeVersionSummary, NodeVersionDetail, VersionDiff } from '$lib/api/versions';
	import { pushToast } from '$lib/stores/toast';

	export let nodeId: string;
	export let open = false;

	const dispatch = createEventDispatcher<{
		close: void;
		restored: { nodeId: string; versionId: string };
	}>();

	let versions: NodeVersionSummary[] = [];
	let loading = true;
	let selectedVersion: NodeVersionDetail | null = null;
	let loadingVersion = false;
	let restoring = false;
	let totalCount = 0;

	// Diff comparison state
	let diffResult: VersionDiff | null = null;
	let loadingDiff = false;
	let comparingVersionId: string | null = null;

	onMount(() => {
		if (open) loadVersions();
	});

	$: if (open && nodeId) {
		loadVersions();
	}

	async function loadVersions() {
		loading = true;
		diffResult = null;
		comparingVersionId = null;
		try {
			const response = await listNodeVersions(nodeId, { limit: 50 });
			versions = response.versions;
			totalCount = response.total_count;
		} catch (err) {
			pushToast('Failed to load version history', 'danger');
		} finally {
			loading = false;
		}
	}

	async function selectVersion(version: NodeVersionSummary) {
		loadingVersion = true;
		diffResult = null;
		comparingVersionId = null;
		try {
			selectedVersion = await getNodeVersion(nodeId, version.version_id);
		} catch (err) {
			pushToast('Failed to load version details', 'danger');
		} finally {
			loadingVersion = false;
		}
	}

	async function compareWithPrevious(version: NodeVersionSummary, previousVersion: NodeVersionSummary) {
		loadingDiff = true;
		comparingVersionId = version.version_id;
		diffResult = null;
		try {
			diffResult = await compareVersions(nodeId, previousVersion.version_id, version.version_id);
		} catch (err) {
			pushToast('Failed to compare versions', 'danger');
		} finally {
			loadingDiff = false;
		}
	}

	function getPreviousVersion(version: NodeVersionSummary): NodeVersionSummary | null {
		const idx = versions.indexOf(version);
		// versions are ordered newest-first, so previous is at idx + 1
		if (idx >= 0 && idx < versions.length - 1) {
			return versions[idx + 1];
		}
		return null;
	}

	async function restore() {
		if (!selectedVersion) return;
		restoring = true;
		try {
			await restoreNodeVersion(nodeId, selectedVersion.version_id);
			pushToast('Version restored successfully', 'success');
			dispatch('restored', { nodeId, versionId: selectedVersion.version_id });
			close();
		} catch (err) {
			pushToast('Failed to restore version', 'danger');
		} finally {
			restoring = false;
		}
	}

	function close() {
		open = false;
		selectedVersion = null;
		diffResult = null;
		comparingVersionId = null;
		dispatch('close');
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') close();
	}
</script>

<svelte:window on:keydown={handleKeydown} />

{#if open}
	<div class="modal-overlay" on:click={close} on:keydown={handleKeydown} role="button" tabindex="0">
		<!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
		<div class="modal-content" on:click|stopPropagation on:keydown|stopPropagation role="dialog" aria-modal="true" tabindex="-1">
			<header class="modal-header">
				<h2>Version History</h2>
				<button class="close-btn" on:click={close} aria-label="Close">
					<svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="2">
						<path d="M18 6L6 18M6 6l12 12"/>
					</svg>
				</button>
			</header>

			<div class="modal-body">
				<div class="versions-list">
					{#if loading}
						<div class="loading">Loading versions...</div>
					{:else if versions.length === 0}
						<div class="empty">No version history available</div>
					{:else}
						{#each versions as version, idx (version.version_id)}
							<div class="version-entry">
								<button
									class="version-item"
									class:selected={selectedVersion?.version_id === version.version_id}
									on:click={() => selectVersion(version)}
								>
									<div class="version-number">v{version.version_number}</div>
									<div class="version-info">
										<span class="version-date">{formatVersionDate(version.created_at)}</span>
										<span class="version-size">{formatVersionSize(version.size_bytes)}</span>
									</div>
									{#if version.change_summary}
										<div class="version-summary">{version.change_summary}</div>
									{/if}
								</button>
								{#if idx < versions.length - 1}
									<button
										class="compare-btn"
										class:active={comparingVersionId === version.version_id}
										on:click={() => compareWithPrevious(version, versions[idx + 1])}
										disabled={loadingDiff && comparingVersionId === version.version_id}
									>
										{#if loadingDiff && comparingVersionId === version.version_id}
											Comparing...
										{:else if comparingVersionId === version.version_id}
											Comparing v{versions[idx + 1].version_number} → v{version.version_number}
										{:else}
											Compare with previous
										{/if}
									</button>
								{/if}
							</div>
						{/each}
						{#if totalCount > versions.length}
							<div class="more-versions">+ {totalCount - versions.length} more versions</div>
						{/if}
					{/if}
				</div>

				<div class="version-preview">
					{#if loadingDiff}
						<div class="loading">Loading diff...</div>
					{:else if diffResult && comparingVersionId}
						{@const currentVer = versions.find((v) => v.version_id === comparingVersionId)}
						{@const prevVer = getPreviousVersion(currentVer ?? versions[0])}
						<div class="preview-header">
							<h3>Diff: v{prevVer?.version_number ?? '?'} → v{currentVer?.version_number ?? '?'}</h3>
						</div>

						<div class="diff-section">
							{#if diffResult.title}
								<div class="diff-item">
									<span class="diff-label">Title:</span>
									<span class="diff-old">{diffResult.title.old}</span>
									<span class="diff-arrow">→</span>
									<span class="diff-new">{diffResult.title.new}</span>
								</div>
							{/if}

							{#if diffResult.content}
								<div class="diff-content-block">
									<h4>Content changes</h4>
									<div class="diff-content-removed">
										<span class="diff-content-label">Removed</span>
										<pre>{diffResult.content.old || '(empty)'}</pre>
									</div>
									<div class="diff-content-added">
										<span class="diff-content-label">Added</span>
										<pre>{diffResult.content.new || '(empty)'}</pre>
									</div>
								</div>
							{/if}

							{#if diffResult.tags}
								<div class="diff-item">
									<span class="diff-label">Tags:</span>
									{#if diffResult.tags.added.length}
										<span class="tags-added">+{diffResult.tags.added.join(', ')}</span>
									{/if}
									{#if diffResult.tags.removed.length}
										<span class="tags-removed">-{diffResult.tags.removed.join(', ')}</span>
									{/if}
								</div>
							{/if}

							{#if diffResult.metadata}
								{#each Object.entries(diffResult.metadata) as [key, change]}
									<div class="diff-item">
										<span class="diff-label">{key}:</span>
										<span class="diff-old">{JSON.stringify(change.old)}</span>
										<span class="diff-arrow">→</span>
										<span class="diff-new">{JSON.stringify(change.new)}</span>
									</div>
								{/each}
							{/if}

							{#if !diffResult.title && !diffResult.content && !diffResult.tags && !diffResult.metadata}
								<div class="empty-diff">No changes detected between these versions.</div>
							{/if}
						</div>
					{:else if loadingVersion}
						<div class="loading">Loading version...</div>
					{:else if selectedVersion}
						<div class="preview-header">
							<h3>Version {selectedVersion.version_number}</h3>
							<span class="preview-date">{formatVersionDate(selectedVersion.created_at)}</span>
						</div>

						{#if selectedVersion.diff_from_previous}
							<div class="diff-section">
								<h4>Changes</h4>
								{#if selectedVersion.diff_from_previous.title}
									<div class="diff-item">
										<span class="diff-label">Title:</span>
										<span class="diff-old">{selectedVersion.diff_from_previous.title.old}</span>
										<span class="diff-arrow">→</span>
										<span class="diff-new">{selectedVersion.diff_from_previous.title.new}</span>
									</div>
								{/if}
								{#if selectedVersion.diff_from_previous.tags}
									<div class="diff-item">
										<span class="diff-label">Tags:</span>
										{#if selectedVersion.diff_from_previous.tags.added.length}
											<span class="tags-added">+{selectedVersion.diff_from_previous.tags.added.join(', ')}</span>
										{/if}
										{#if selectedVersion.diff_from_previous.tags.removed.length}
											<span class="tags-removed">-{selectedVersion.diff_from_previous.tags.removed.join(', ')}</span>
										{/if}
									</div>
								{/if}
							</div>
						{/if}

						<div class="content-preview">
							<h4>Content</h4>
							<pre>{selectedVersion.snapshot.content || '(empty)'}</pre>
						</div>

						<div class="preview-actions">
							<button class="btn-restore" on:click={restore} disabled={restoring}>
								{restoring ? 'Restoring...' : 'Restore This Version'}
							</button>
						</div>
					{:else}
						<div class="empty">Select a version to preview</div>
					{/if}
				</div>
			</div>
		</div>
	</div>
{/if}

<style>
	.modal-overlay {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.5);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 1000;
	}

	.modal-content {
		background: var(--surface-color, #fff);
		border-radius: 12px;
		width: 90vw;
		max-width: 900px;
		max-height: 80vh;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.modal-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 1rem 1.5rem;
		border-bottom: 1px solid var(--border-color, #e0e0e0);
	}

	.modal-header h2 {
		margin: 0;
		font-size: 1.25rem;
	}

	.close-btn {
		background: none;
		border: none;
		cursor: pointer;
		padding: 0.25rem;
		color: var(--text-muted, #6b7280);
	}

	.modal-body {
		display: grid;
		grid-template-columns: 280px 1fr;
		flex: 1;
		overflow: hidden;
	}

	.versions-list {
		border-right: 1px solid var(--border-color, #e0e0e0);
		overflow-y: auto;
		padding: 0.5rem;
	}

	.version-entry {
		display: flex;
		flex-direction: column;
	}

	.version-item {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		width: 100%;
		padding: 0.75rem;
		border: none;
		background: transparent;
		border-radius: 8px;
		text-align: left;
		cursor: pointer;
		transition: background 0.15s;
	}

	.version-item:hover {
		background: var(--hover-bg, #f3f4f6);
	}

	.version-item.selected {
		background: var(--primary-light, #dbeafe);
	}

	.version-number {
		font-weight: 600;
		font-size: 0.875rem;
	}

	.version-info {
		display: flex;
		gap: 0.5rem;
		font-size: 0.75rem;
		color: var(--text-muted, #6b7280);
	}

	.version-summary {
		font-size: 0.75rem;
		color: var(--text-secondary, #4b5563);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.compare-btn {
		margin: 0.125rem 0.75rem 0.25rem;
		padding: 0.25rem 0.5rem;
		background: transparent;
		border: 1px solid var(--border-color, #e0e0e0);
		border-radius: 4px;
		font-size: 0.625rem;
		color: var(--text-muted, #6b7280);
		cursor: pointer;
		transition: all 0.15s;
	}

	.compare-btn:hover {
		background: var(--hover-bg, #f3f4f6);
		color: var(--text-primary, #1f2937);
	}

	.compare-btn.active {
		background: var(--primary-light, #dbeafe);
		border-color: var(--primary-color, #3b82f6);
		color: var(--primary-color, #3b82f6);
	}

	.compare-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.more-versions {
		text-align: center;
		padding: 0.5rem;
		color: var(--text-muted, #6b7280);
		font-size: 0.75rem;
	}

	.version-preview {
		overflow-y: auto;
		padding: 1rem 1.5rem;
	}

	.preview-header {
		display: flex;
		align-items: baseline;
		gap: 1rem;
		margin-bottom: 1rem;
	}

	.preview-header h3 {
		margin: 0;
	}

	.preview-date {
		font-size: 0.875rem;
		color: var(--text-muted, #6b7280);
	}

	.diff-section, .content-preview {
		margin-bottom: 1rem;
	}

	.diff-section h4, .content-preview h4 {
		font-size: 0.875rem;
		margin: 0 0 0.5rem;
		color: var(--text-muted, #6b7280);
	}

	.diff-item {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 0.875rem;
		padding: 0.25rem 0;
	}

	.diff-label {
		font-weight: 500;
	}

	.diff-old {
		text-decoration: line-through;
		color: #ef4444;
		background: rgba(239, 68, 68, 0.1);
		padding: 0.125rem 0.375rem;
		border-radius: 4px;
	}

	.diff-new {
		color: #22c55e;
		background: rgba(34, 197, 94, 0.1);
		padding: 0.125rem 0.375rem;
		border-radius: 4px;
	}

	.diff-arrow {
		color: var(--text-muted, #6b7280);
	}

	.tags-added {
		color: #22c55e;
		background: rgba(34, 197, 94, 0.1);
		padding: 0.125rem 0.375rem;
		border-radius: 4px;
	}

	.tags-removed {
		color: #ef4444;
		background: rgba(239, 68, 68, 0.1);
		padding: 0.125rem 0.375rem;
		border-radius: 4px;
	}

	.diff-content-block {
		margin: 0.75rem 0;
	}

	.diff-content-block h4 {
		font-size: 0.875rem;
		margin: 0 0 0.5rem;
		color: var(--text-muted, #6b7280);
	}

	.diff-content-removed {
		margin-bottom: 0.5rem;
	}

	.diff-content-added {
		margin-bottom: 0.5rem;
	}

	.diff-content-label {
		display: inline-block;
		font-size: 0.625rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		margin-bottom: 0.25rem;
	}

	.diff-content-removed .diff-content-label {
		color: #ef4444;
	}

	.diff-content-added .diff-content-label {
		color: #22c55e;
	}

	.diff-content-removed pre {
		background: rgba(239, 68, 68, 0.05);
		border: 1px solid rgba(239, 68, 68, 0.15);
		border-left: 3px solid #ef4444;
		padding: 0.75rem;
		border-radius: 6px;
		font-size: 0.8rem;
		overflow-x: auto;
		white-space: pre-wrap;
		max-height: 200px;
		overflow-y: auto;
		color: #fca5a5;
	}

	.diff-content-added pre {
		background: rgba(34, 197, 94, 0.05);
		border: 1px solid rgba(34, 197, 94, 0.15);
		border-left: 3px solid #22c55e;
		padding: 0.75rem;
		border-radius: 6px;
		font-size: 0.8rem;
		overflow-x: auto;
		white-space: pre-wrap;
		max-height: 200px;
		overflow-y: auto;
		color: #86efac;
	}

	.empty-diff {
		text-align: center;
		padding: 1rem;
		color: var(--text-muted, #6b7280);
		font-size: 0.875rem;
	}

	.content-preview pre {
		background: var(--code-bg, #f3f4f6);
		padding: 1rem;
		border-radius: 8px;
		font-size: 0.875rem;
		overflow-x: auto;
		white-space: pre-wrap;
		max-height: 300px;
		overflow-y: auto;
	}

	.preview-actions {
		display: flex;
		justify-content: flex-end;
	}

	.btn-restore {
		padding: 0.5rem 1rem;
		background: var(--primary-color, #3b82f6);
		color: white;
		border: none;
		border-radius: 6px;
		font-size: 0.875rem;
		cursor: pointer;
	}

	.btn-restore:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.loading, .empty {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100%;
		color: var(--text-muted, #6b7280);
	}
</style>
