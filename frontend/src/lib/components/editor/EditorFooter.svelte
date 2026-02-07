<script lang="ts">
	/** Current word count */
	export let wordCount = 0;
	/** Current editor mode */
	export let mode: 'rich' | 'source' = 'rich';
	/** Whether the editor is read-only */
	export let readonly = false;
</script>

{#if !readonly}
	<div class="flex items-center justify-between border-t border-slate-800/60 px-4 py-1.5 text-[10px] text-slate-500">
		<span>{wordCount} words</span>
		<span>
			{#if mode === 'rich'}
				Type <kbd class="rounded border border-slate-700 bg-slate-800 px-1">/</kbd> for commands
			{:else}
				Markdown source
			{/if}
		</span>
	</div>
{/if}
