<script lang="ts">
	import { createEventDispatcher } from 'svelte';

	/** Current editor mode */
	export let mode: 'rich' | 'source' = 'rich';
	/** Whether the editor is read-only */
	export let readonly = false;
	/** Current toolbar state */
	export let state = {
		bold: false,
		italic: false,
		strikethrough: false,
		orderedList: false,
		unorderedList: false,
		heading: 0,
		blockquote: false,
		code: false
	};
	/** Undo available */
	export let canUndo = false;
	/** Redo available */
	export let canRedo = false;

	const dispatch = createEventDispatcher<{
		setMode: 'rich' | 'source';
		toggleBold: void;
		toggleItalic: void;
		toggleStrikethrough: void;
		toggleUnorderedList: void;
		toggleOrderedList: void;
		insertTaskList: void;
		setHeading: number;
		toggleBlockquote: void;
		toggleCodeBlock: void;
		insertHorizontalRule: void;
		insertLink: void;
		removeFormatting: void;
		undo: void;
		redo: void;
	}>();
</script>

{#if !readonly}
	<div class="flex flex-wrap items-center gap-1 border-b border-slate-800 px-3 py-2">
		<!-- Mode toggle -->
		<div class="mr-2 flex rounded-lg border border-slate-700 text-[10px]">
			<button
				class={`px-2 py-1 transition ${mode === 'rich' ? 'bg-slate-700 text-white' : 'text-slate-400 hover:text-white'}`}
				on:click={() => dispatch('setMode', 'rich')}
			>
				Rich
			</button>
			<button
				class={`px-2 py-1 transition ${mode === 'source' ? 'bg-slate-700 text-white' : 'text-slate-400 hover:text-white'}`}
				on:click={() => dispatch('setMode', 'source')}
			>
				Source
			</button>
		</div>

		{#if mode === 'rich'}
			<span class="mx-1 h-4 w-px bg-slate-700"></span>

			<!-- Text formatting -->
			<button
				class={`rounded px-2 py-1 text-xs transition ${state.bold ? 'bg-sky-500/20 text-sky-300' : 'text-slate-400 hover:bg-slate-800 hover:text-white'}`}
				on:click={() => dispatch('toggleBold')}
				title="Bold (Ctrl+B)"
			>
				<strong>B</strong>
			</button>
			<button
				class={`rounded px-2 py-1 text-xs transition ${state.italic ? 'bg-sky-500/20 text-sky-300' : 'text-slate-400 hover:bg-slate-800 hover:text-white'}`}
				on:click={() => dispatch('toggleItalic')}
				title="Italic (Ctrl+I)"
			>
				<em>I</em>
			</button>
			<button
				class={`rounded px-2 py-1 text-xs transition ${state.strikethrough ? 'bg-sky-500/20 text-sky-300' : 'text-slate-400 hover:bg-slate-800 hover:text-white'}`}
				on:click={() => dispatch('toggleStrikethrough')}
				title="Strikethrough"
			>
				<s>S</s>
			</button>

			<span class="mx-1 h-4 w-px bg-slate-700"></span>

			<!-- Headings -->
			<select
				class="rounded border border-slate-700 bg-slate-900 px-2 py-1 text-[10px] text-slate-300"
				value={state.heading}
				on:change={(e) => dispatch('setHeading', parseInt(e.currentTarget.value))}
			>
				<option value={0}>Paragraph</option>
				<option value={1}>Heading 1</option>
				<option value={2}>Heading 2</option>
				<option value={3}>Heading 3</option>
				<option value={4}>Heading 4</option>
			</select>

			<span class="mx-1 h-4 w-px bg-slate-700"></span>

			<!-- Lists -->
			<button
				class={`rounded px-2 py-1 text-xs transition ${state.unorderedList ? 'bg-sky-500/20 text-sky-300' : 'text-slate-400 hover:bg-slate-800 hover:text-white'}`}
				on:click={() => dispatch('toggleUnorderedList')}
				title="Bullet list"
			>
				&bull; List
			</button>
			<button
				class={`rounded px-2 py-1 text-xs transition ${state.orderedList ? 'bg-sky-500/20 text-sky-300' : 'text-slate-400 hover:bg-slate-800 hover:text-white'}`}
				on:click={() => dispatch('toggleOrderedList')}
				title="Numbered list"
			>
				1. List
			</button>
			<button
				class="rounded px-2 py-1 text-xs text-slate-400 transition hover:bg-slate-800 hover:text-white"
				on:click={() => dispatch('insertTaskList')}
				title="Task list"
			>
				&#9744; Tasks
			</button>

			<span class="mx-1 h-4 w-px bg-slate-700"></span>

			<!-- Block formatting -->
			<button
				class={`rounded px-2 py-1 text-xs transition ${state.blockquote ? 'bg-sky-500/20 text-sky-300' : 'text-slate-400 hover:bg-slate-800 hover:text-white'}`}
				on:click={() => dispatch('toggleBlockquote')}
				title="Blockquote"
			>
				&ldquo; Quote
			</button>
			<button
				class={`rounded px-2 py-1 text-xs transition ${state.code ? 'bg-sky-500/20 text-sky-300' : 'text-slate-400 hover:bg-slate-800 hover:text-white'}`}
				on:click={() => dispatch('toggleCodeBlock')}
				title="Code block"
			>
				&lt;/&gt;
			</button>
			<button
				class="rounded px-2 py-1 text-xs text-slate-400 transition hover:bg-slate-800 hover:text-white"
				on:click={() => dispatch('insertHorizontalRule')}
				title="Horizontal rule"
			>
				&mdash;
			</button>

			<span class="mx-1 h-4 w-px bg-slate-700"></span>

			<!-- Link -->
			<button
				class="rounded px-2 py-1 text-xs text-slate-400 transition hover:bg-slate-800 hover:text-white"
				on:click={() => dispatch('insertLink')}
				title="Insert link (Ctrl+K)"
			>
				Link
			</button>

			<!-- Clear -->
			<button
				class="rounded px-2 py-1 text-xs text-slate-400 transition hover:bg-slate-800 hover:text-white"
				on:click={() => dispatch('removeFormatting')}
				title="Remove formatting"
			>
				Clear
			</button>

			<span class="mx-1 h-4 w-px bg-slate-700"></span>

			<!-- Undo/Redo -->
			<button
				class="rounded px-2 py-1 text-xs text-slate-400 transition hover:bg-slate-800 hover:text-white disabled:opacity-30"
				on:click={() => dispatch('undo')}
				disabled={!canUndo}
				title="Undo (Ctrl+Z)"
			>
				Undo
			</button>
			<button
				class="rounded px-2 py-1 text-xs text-slate-400 transition hover:bg-slate-800 hover:text-white disabled:opacity-30"
				on:click={() => dispatch('redo')}
				disabled={!canRedo}
				title="Redo (Ctrl+Shift+Z)"
			>
				Redo
			</button>
		{/if}
	</div>
{/if}
