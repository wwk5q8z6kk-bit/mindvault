<script lang="ts">
	import { apiHealth } from '$lib/stores/api-health';
	import { onDestroy } from 'svelte';

	export let online = true;
	let dismissed = false;
	let bannerVisible = false;
	let bannerTimer: ReturnType<typeof setTimeout> | null = null;

	$: status = $apiHealth.status;
	$: severityClass =
		status === 'offline'
			? 'border-rose-400/70 bg-rose-50 text-rose-900'
			: 'border-amber-400/70 bg-amber-50 text-amber-900';
	$: title = status === 'offline' ? 'Backend unavailable' : 'Backend degraded';
	$: body =
		status === 'offline'
			? 'The API is unreachable. Local data remains available, but sync and AI features are temporarily paused.'
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
			<p class="text-xs">{body}</p>
		</div>
		<button
			type="button"
			class="shrink-0 rounded-md border border-current/30 px-2 py-1 text-xs font-medium transition hover:bg-black/5"
			on:click={dismissBanner}
			aria-label="Dismiss backend status notice"
		>
			Dismiss
		</button>
	</div>
{/if}
