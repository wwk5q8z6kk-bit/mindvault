<script lang="ts">
	import { LockKeyhole } from '@lucide/svelte';

	export let contextName = 'Personal Vault';
	export let accessLabel = 'Private';
	export let detail: string | null = null;
	export let compact = false;

	$: accessibleLabel = `Current context: ${contextName}. ${accessLabel}${detail ? `. ${detail}` : ''}`;
</script>

<div
	class="context-boundary"
	class:compact
	role="group"
	aria-label={accessibleLabel}
	title={accessibleLabel}
>
	<span class="context-icon" aria-hidden="true">
		<LockKeyhole size={14} strokeWidth={1.9} />
	</span>
	<span class="context-copy">
		<strong>{contextName}</strong>
		<span>{accessLabel}{detail ? ` · ${detail}` : ''}</span>
	</span>
</div>

<style>
	.context-boundary {
		display: inline-flex;
		min-width: 0;
		align-items: center;
		gap: 8px;
		padding: 7px 9px;
		border: 1px solid rgb(var(--mv-border) / 0.65);
		border-radius: 10px;
		background: rgb(var(--mv-panel-strong) / 0.42);
		color: rgb(var(--mv-text));
	}

	.context-icon {
		display: grid;
		width: 27px;
		height: 27px;
		flex: 0 0 27px;
		place-items: center;
		border-radius: 8px;
		background: rgb(var(--mv-accent) / 0.13);
		color: rgb(var(--mv-accent));
	}

	.context-copy {
		display: flex;
		min-width: 0;
		flex-direction: column;
		gap: 3px;
		line-height: 1;
	}

	.context-copy strong,
	.context-copy span {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.context-copy strong {
		color: rgb(var(--mv-text) / 0.82);
		font-size: 11px;
		font-weight: 650;
	}

	.context-copy span {
		color: rgb(var(--mv-muted) / 0.72);
		font-size: 9px;
		font-weight: 550;
		letter-spacing: 0.02em;
	}

	.context-boundary.compact {
		flex: 0 0 auto;
		gap: 7px;
		padding: 5px 9px 5px 6px;
		border-radius: 999px;
	}

	.compact .context-icon {
		width: 24px;
		height: 24px;
		flex-basis: 24px;
		border-radius: 999px;
	}

	.compact .context-copy {
		flex-direction: row;
		align-items: center;
		gap: 6px;
	}

	.compact .context-copy span {
		display: flex;
		align-items: center;
		gap: 6px;
	}

	.compact .context-copy span::before {
		width: 3px;
		height: 3px;
		border-radius: 50%;
		background: rgb(var(--mv-muted) / 0.55);
		content: '';
	}
</style>
