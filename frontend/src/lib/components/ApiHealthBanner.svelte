<script lang="ts">
	import { apiHealth } from '$lib/stores/api-health';
	import { onDestroy } from 'svelte';

	export let online = true;
	let dismissed = false;
	let bannerVisible = false;
	let bannerTimer: ReturnType<typeof setTimeout> | null = null;

	$: status = $apiHealth.status;
	$: severityClass = status === 'offline' ? 'border-rose-500/40 bg-rose-500/10 text-rose-100' : 'border-amber-500/40 bg-amber-500/10 text-amber-100';
	$: title = status === 'offline' ? 'Backend unavailable' : 'Backend degraded';
	$: body =
		status === 'offline'
			? 'The API is unreachable. Local data remains available, but sync and AI features may be unavailable.'
			: 'The API is reachable but returning server errors. Some actions may fail.';

	$: {
		if (!online || dismissed || (status !== 'offline' && status !== 'degraded')) {
			if (bannerTimer) {
				clearTimeout(bannerTimer);
				bannerTimer = null;
			}
			bannerVisible = false;
		} else if (!bannerVisible && !bannerTimer) {
			// Delay avoids flashing a banner during initial in-flight requests.
			bannerTimer = setTimeout(() => {
				bannerVisible = true;
				bannerTimer = null;
			}, 900);
		}
	}

	$: if (status === 'healthy') {
		dismissed = false;
		bannerVisible = false;
		if (bannerTimer) {
			clearTimeout(bannerTimer);
			bannerTimer = null;
		}
	}

	function dismissBanner() {
		dismissed = true;
		bannerVisible = false;
		if (bannerTimer) {
			clearTimeout(bannerTimer);
			bannerTimer = null;
		}
	}

	onDestroy(() => {
		if (bannerTimer) {
			clearTimeout(bannerTimer);
			bannerTimer = null;
		}
	});
</script>

{#if bannerVisible}
	<div
		class={`mx-4 mt-3 flex items-start justify-between gap-3 rounded-xl border px-4 py-3 text-sm shadow-sm md:mx-6 ${severityClass}`}
		role="status"
		aria-live="polite"
	>
		<div class="min-w-0">
			<p class="font-semibold">{title}</p>
			<p class="text-xs opacity-90">{body}</p>
		</div>
		<button
			type="button"
			class="shrink-0 rounded-md px-2 py-1 text-xs font-medium transition hover:bg-white/10"
			on:click={dismissBanner}
			aria-label="Dismiss backend status notice"
		>
			Dismiss
		</button>
	</div>
{/if}
