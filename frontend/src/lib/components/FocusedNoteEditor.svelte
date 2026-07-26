<script lang="ts">
	import { createEventDispatcher, onDestroy, onMount } from 'svelte';
	import { Editor } from '@tiptap/core';
	import { StarterKit } from '@tiptap/starter-kit';
	import { Markdown } from '@tiptap/markdown';
	import { Placeholder } from '@tiptap/extension-placeholder';

	export let markdown = '';
	export let placeholder = "Type '/' for commands";

	const dispatch = createEventDispatcher<{ change: { markdown: string } }>();

	let editorElement: HTMLDivElement;
	let editor: Editor | null = null;

	onMount(() => {
		editor = new Editor({
			element: editorElement,
			extensions: [
				StarterKit,
				Markdown,
				Placeholder.configure({
					placeholder
				})
			],
			content: markdown,
			contentType: 'markdown',
			editorProps: {
				attributes: {
					class: 'mv-focused-editor',
					'aria-label': 'Note content'
				}
			},
			onUpdate: ({ editor: activeEditor }) => {
				markdown = activeEditor.getMarkdown();
				dispatch('change', { markdown });
			}
		});
	});

	onDestroy(() => {
		editor?.destroy();
		editor = null;
	});
</script>

<div class="focused-editor-shell">
	<div bind:this={editorElement}></div>
</div>

<style>
	.focused-editor-shell {
		min-height: 430px;
	}

	:global(.mv-focused-editor) {
		min-height: 430px;
		max-width: 760px;
		padding: 0;
		color: rgb(193 195 204);
		font-family: 'Plus Jakarta Sans', system-ui, sans-serif;
		font-size: 17px;
		font-weight: 400;
		line-height: 1.9;
		outline: none;
		caret-color: rgb(125 92 255);
	}

	:global(.mv-focused-editor > *:first-child) {
		margin-top: 0;
	}

	:global(.mv-focused-editor p) {
		margin: 0 0 28px;
	}

	:global(.mv-focused-editor ul),
	:global(.mv-focused-editor ol) {
		margin: -2px 0 30px;
		padding-left: 30px;
	}

	:global(.mv-focused-editor ul) {
		list-style: disc;
	}

	:global(.mv-focused-editor ol) {
		list-style: decimal;
	}

	:global(.mv-focused-editor li) {
		margin: 5px 0;
		padding-left: 2px;
	}

	:global(.mv-focused-editor li p) {
		margin: 0;
	}

	:global(.mv-focused-editor li::marker) {
		color: rgb(112 79 255);
	}

	:global(.mv-focused-editor h2),
	:global(.mv-focused-editor h3) {
		margin: 30px 0 12px;
		color: rgb(236 236 241);
		font-weight: 600;
		line-height: 1.35;
	}

	:global(.mv-focused-editor h2) {
		font-size: 21px;
	}

	:global(.mv-focused-editor h3) {
		font-size: 18px;
	}

	:global(.mv-focused-editor blockquote) {
		margin: 24px 0;
		border-left: 2px solid rgb(107 76 255);
		padding-left: 20px;
		color: rgb(213 214 221);
	}

	:global(.mv-focused-editor code) {
		border-radius: 5px;
		background: rgb(31 31 38);
		padding: 2px 5px;
		color: rgb(202 190 255);
		font-size: 0.88em;
	}

	:global(.mv-focused-editor pre) {
		overflow-x: auto;
		border: 1px solid rgb(43 43 52);
		border-radius: 10px;
		background: rgb(17 17 22);
		padding: 16px;
	}

	:global(.mv-focused-editor a) {
		color: rgb(174 156 255);
		text-decoration: underline;
		text-underline-offset: 3px;
	}

	:global(.mv-focused-editor p.is-editor-empty:first-child::before) {
		float: left;
		height: 0;
		color: rgb(95 96 107);
		content: attr(data-placeholder);
		pointer-events: none;
	}

	@media (max-width: 760px) {
		:global(.mv-focused-editor) {
			min-height: 360px;
			font-size: 16px;
			line-height: 1.8;
		}
	}
</style>
