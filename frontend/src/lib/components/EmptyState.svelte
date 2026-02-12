<script lang="ts">
	export let title = 'Nothing here yet';
	export let description = '';
	export let icon = '';
</script>

<div class="flex flex-col items-center justify-center py-16 text-center">
	{#if icon}
		<div
			class="mb-4 flex h-14 w-14 items-center justify-center rounded-2xl bg-[rgb(var(--mv-accent))]/10 text-2xl text-[rgb(var(--mv-accent))]"
		>
			{icon}
		</div>
	{/if}
	<h3 class="text-base font-medium text-[rgb(var(--mv-text))]">{title}</h3>
	{#if description}
		<p class="mt-1 max-w-sm text-sm text-[rgb(var(--mv-muted))]">{description}</p>
	{/if}
	{#if $$slots.actions}
		<div class="mt-4">
			<slot name="actions" />
		</div>
	{/if}
</div>
