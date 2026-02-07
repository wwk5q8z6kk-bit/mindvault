<script lang="ts">
	import { createEventDispatcher, onMount, tick } from 'svelte';
	import { renderMath, ensureKatexLoaded, isKatexLoaded } from './index';

	/** Initial LaTeX */
	export let latex = '';
	/** Whether this is a block (display) equation */
	export let displayMode = true;

	const dispatch = createEventDispatcher<{
		submit: { latex: string };
		cancel: void;
	}>();

	let currentLatex = latex;
	let latexInput: HTMLTextAreaElement | null = null;
	let previewHtml = '';
	let error = '';
	let katexReady = false;

	onMount(async () => {
	tick().then(() => latexInput?.focus());
		katexReady = isKatexLoaded();
		if (!katexReady) {
			katexReady = await ensureKatexLoaded();
		}
		updatePreview();
	});

	function updatePreview() {
		if (!katexReady) {
			previewHtml = '<span class="text-slate-500">Loading KaTeX...</span>';
			return;
		}

		try {
			previewHtml = renderMath(currentLatex, displayMode);
			error = '';
		} catch (e) {
			error = String(e);
			previewHtml = '';
		}
	}

	function handleInput() {
		updatePreview();
	}

	function handleSubmit() {
		if (currentLatex.trim()) {
			dispatch('submit', { latex: currentLatex });
		}
	}

	function handleCancel() {
		dispatch('cancel');
	}

	function handleKeyDown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			handleCancel();
		}
		if (event.key === 'Enter' && event.metaKey) {
			handleSubmit();
		}
	}

	// Common LaTeX snippets
	const snippets = [
		{ label: 'Fraction', latex: '\\frac{a}{b}' },
		{ label: 'Square root', latex: '\\sqrt{x}' },
		{ label: 'Power', latex: 'x^{n}' },
		{ label: 'Subscript', latex: 'x_{i}' },
		{ label: 'Sum', latex: '\\sum_{i=0}^{n}' },
		{ label: 'Integral', latex: '\\int_{a}^{b}' },
		{ label: 'Limit', latex: '\\lim_{x \\to \\infty}' },
		{ label: 'Matrix', latex: '\\begin{pmatrix} a & b \\\\ c & d \\end{pmatrix}' }
	];

	function insertSnippet(snippet: string) {
		currentLatex += snippet;
		updatePreview();
	}
</script>

<div
	class="fixed inset-0 z-50 flex items-center justify-center bg-black/60"
	on:click|self={handleCancel}
	on:keydown={handleKeyDown}
	role="dialog"
	aria-modal="true"
	aria-labelledby="math-modal-title" tabindex="0"
>
	<div class="w-[600px] rounded-xl border border-slate-700 bg-slate-900 p-5 shadow-2xl">
		<h3 id="math-modal-title" class="mb-3 text-sm font-semibold text-white flex items-center gap-2">
			<span class="text-lg">∑</span>
			{displayMode ? 'Block Equation' : 'Inline Equation'}
		</h3>

		<div class="flex flex-col gap-4">
			<!-- LaTeX input -->
			<div>
				<label for="latex-input" class="mb-1 block text-[10px] uppercase tracking-wide text-slate-500">
					LaTeX
				</label>
				<textarea
					id="latex-input"
					class="h-24 w-full resize-none rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 font-mono text-xs text-white outline-none focus:border-sky-500"
					placeholder="Enter LaTeX expression..."
					bind:value={currentLatex}
					bind:this={latexInput}
					on:input={handleInput}
				></textarea>
			</div>

			<!-- Snippets -->
			<div>
				<span class="mb-1 block text-[10px] uppercase tracking-wide text-slate-500">Insert snippet</span>
				<div class="flex flex-wrap gap-1">
					{#each snippets as snippet}
						<button
							class="rounded border border-slate-700 bg-slate-800 px-2 py-1 text-[10px] text-slate-400 hover:bg-slate-700 hover:text-white"
							on:click={() => insertSnippet(snippet.latex)}
						>
							{snippet.label}
						</button>
					{/each}
				</div>
			</div>

			<!-- Preview -->
			<div>
				<span class="mb-1 block text-[10px] uppercase tracking-wide text-slate-500">Preview</span>
				<div
					class="min-h-[60px] rounded-lg border border-slate-700 bg-slate-950 p-4 flex items-center justify-center"
				>
					{#if error}
						<span class="text-xs text-red-400">{error}</span>
					{:else if previewHtml}
						<div class="math-preview" class:block-mode={displayMode}>
							{@html previewHtml}
						</div>
					{:else}
						<span class="text-xs text-slate-500">Enter LaTeX to see preview</span>
					{/if}
				</div>
			</div>

			<!-- Actions -->
			<div class="flex justify-end gap-2">
				<button
					class="rounded-lg px-3 py-2 text-xs text-slate-400 hover:bg-slate-800 hover:text-white"
					on:click={handleCancel}
				>
					Cancel
				</button>
				<button
					class="rounded-lg bg-sky-500 px-3 py-2 text-xs font-semibold text-white hover:bg-sky-400 disabled:opacity-40"
					disabled={!currentLatex.trim()}
					on:click={handleSubmit}
				>
					Insert
				</button>
			</div>
		</div>
	</div>
</div>

<style>
	.math-preview {
		color: white;
		font-size: 1.25rem;
	}

	.math-preview.block-mode {
		font-size: 1.5rem;
	}

	/* KaTeX styles - basic overrides for dark theme */
	.math-preview :global(.katex) {
		color: white;
	}

	.math-preview :global(.katex-error) {
		color: #f87171;
	}
</style>
