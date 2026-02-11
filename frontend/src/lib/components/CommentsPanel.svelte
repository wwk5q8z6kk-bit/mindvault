<script lang="ts">
	import { onMount } from 'svelte';
	import { pushToast } from '$lib/stores/toast';
	import {
		createNodeComment,
		deleteNodeComment,
		listNodeComments,
		resolveNodeComment,
		type NodeComment
	} from '$lib/api/comments';

	export let nodeId: string | null = null;
	export let nodeTitle: string | null = null;

	let comments: NodeComment[] = [];
	let loading = false;
	let saving = false;
	let resolvingId: string | null = null;
	let deletingId: string | null = null;
	let includeResolved = false;
	let body = '';
	let loadedForNode: string | null = null;

	onMount(async () => {
		if (nodeId) {
			await refreshComments();
		}
	});

	$: if (nodeId && nodeId !== loadedForNode) {
		loadedForNode = nodeId;
		body = '';
		void refreshComments();
	}

	$: if (!nodeId) {
		comments = [];
		body = '';
		loadedForNode = null;
	}

	async function refreshComments() {
		if (!nodeId) return;
		loading = true;
		try {
			comments = await listNodeComments(nodeId, includeResolved);
		} catch {
			pushToast('Failed to load comments', 'danger');
		} finally {
			loading = false;
		}
	}

	function formatDateTime(iso?: string | null): string {
		if (!iso) return 'unknown';
		return new Date(iso).toLocaleString(undefined, {
			year: 'numeric',
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		});
	}

	async function handleCreate() {
		if (!nodeId) {
			pushToast('Select a note or task first', 'warning');
			return;
		}
		const trimmed = body.trim();
		if (!trimmed) {
			pushToast('Comment cannot be empty', 'warning');
			return;
		}
		saving = true;
		try {
			await createNodeComment({ node_id: nodeId, body: trimmed });
			body = '';
			pushToast('Comment added', 'success');
			await refreshComments();
		} catch (e: any) {
			pushToast(e?.message ?? 'Failed to add comment', 'danger');
		} finally {
			saving = false;
		}
	}

	async function handleResolve(commentId: string) {
		if (!nodeId) return;
		resolvingId = commentId;
		try {
			await resolveNodeComment(nodeId, commentId);
			pushToast('Comment resolved', 'success');
			await refreshComments();
		} catch (e: any) {
			pushToast(e?.message ?? 'Failed to resolve comment', 'danger');
		} finally {
			resolvingId = null;
		}
	}

	async function handleDelete(commentId: string) {
		if (!nodeId) return;
		if (!confirm('Delete this comment?')) return;
		deletingId = commentId;
		try {
			await deleteNodeComment(nodeId, commentId);
			pushToast('Comment deleted', 'success');
			await refreshComments();
		} catch (e: any) {
			pushToast(e?.message ?? 'Failed to delete comment', 'danger');
		} finally {
			deletingId = null;
		}
	}

	function commentStatus(comment: NodeComment): { label: string; className: string } {
		if (comment.resolved_at) {
			return { label: 'resolved', className: 'bg-emerald-500/10 text-emerald-300' };
		}
		return { label: 'open', className: 'bg-amber-500/10 text-amber-300' };
	}
</script>

<div class="rounded-2xl border border-slate-800 bg-slate-900/50 p-4">
	<div class="flex items-start justify-between gap-3">
		<div>
			<h3 class="text-xs font-semibold text-white">Comments</h3>
			<p class="mt-1 text-[10px] text-slate-500">
				Annotations for <span class="text-slate-300">{nodeTitle || 'this item'}</span>.
			</p>
		</div>
		<button
			class="rounded-lg border border-slate-700 px-2 py-1 text-[10px] text-slate-300 hover:bg-slate-800 disabled:opacity-50"
			on:click={refreshComments}
			disabled={loading || !nodeId}
			title="Refresh comments"
		>
			{loading ? 'Refreshing…' : 'Refresh'}
		</button>
	</div>

	<div class="mt-3 flex items-center gap-2 text-[10px] text-slate-400">
		<input
			id="include-resolved-comments"
			type="checkbox"
			class="h-3 w-3 rounded border-slate-600 bg-slate-900"
			bind:checked={includeResolved}
			on:change={() => refreshComments()}
		/>
		<label for="include-resolved-comments">Include resolved</label>
	</div>

	{#if !nodeId}
		<div class="mt-3 rounded-xl border border-dashed border-slate-800 px-3 py-4 text-center text-[10px] text-slate-500">
			Select a note or task to manage comments.
		</div>
	{:else}
		<div class="mt-3 grid gap-2">
			<label class="text-[10px] uppercase tracking-wide text-slate-500" for="comment-body">
				New comment
			</label>
			<textarea
				id="comment-body"
				rows="3"
				bind:value={body}
				class="w-full resize-none rounded-lg border border-slate-700 bg-slate-950/40 px-3 py-2 text-xs text-slate-200 focus:border-sky-500 focus:outline-none"
				placeholder="Add context, decisions, or follow-ups…"
			></textarea>
			<div class="flex justify-end">
				<button
					class="rounded-lg bg-sky-500/20 px-3 py-2 text-xs font-semibold text-sky-200 hover:bg-sky-500/30 disabled:opacity-50"
					on:click={handleCreate}
					disabled={saving}
				>
					{saving ? 'Saving…' : 'Add comment'}
				</button>
			</div>
		</div>

		<div class="mt-4 space-y-2">
			<p class="text-[10px] uppercase tracking-wide text-slate-500">History</p>
			{#if loading && comments.length === 0}
				<p class="text-[10px] text-slate-500">Loading comments…</p>
			{:else if comments.length === 0}
				<p class="text-[10px] text-slate-500">No comments yet.</p>
			{:else}
				{#each comments as comment (comment.id)}
					<div class="rounded-xl border border-slate-800 bg-slate-950/30 p-3">
						<div class="flex items-start justify-between gap-2">
							<div>
								<p class="text-[10px] uppercase tracking-wide text-slate-500">Comment</p>
								<p class="mt-1 text-xs text-slate-200 whitespace-pre-wrap">{comment.body}</p>
							</div>
							<span
								class={`rounded-full px-2 py-0.5 text-[9px] font-semibold uppercase ${commentStatus(comment).className}`}
							>
								{commentStatus(comment).label}
							</span>
						</div>
						<p class="mt-2 text-[10px] text-slate-500">
							{comment.author || 'system'} · {formatDateTime(comment.created_at)}
						</p>
						<div class="mt-2 flex flex-wrap gap-2">
							{#if !comment.resolved_at}
								<button
									class="rounded-lg border border-slate-700 px-2 py-1 text-[10px] text-slate-300 hover:bg-slate-800 disabled:opacity-50"
									on:click={() => handleResolve(comment.id)}
									disabled={resolvingId === comment.id}
								>
									{resolvingId === comment.id ? 'Resolving…' : 'Resolve'}
								</button>
							{/if}
							<button
								class="rounded-lg border border-red-500/30 px-2 py-1 text-[10px] text-red-200 hover:bg-red-500/10 disabled:opacity-50"
								on:click={() => handleDelete(comment.id)}
								disabled={deletingId === comment.id}
							>
								{deletingId === comment.id ? 'Deleting…' : 'Delete'}
							</button>
						</div>
					</div>
				{/each}
			{/if}
		</div>
	{/if}
</div>
