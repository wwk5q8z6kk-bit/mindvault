<script lang="ts">
	import { onDestroy, onMount, tick } from 'svelte';
	import { Editor, Extension } from '@tiptap/core';
	import { StarterKit } from '@tiptap/starter-kit';
	import { Image } from '@tiptap/extension-image';
	import { Link } from '@tiptap/extension-link';
	import { TaskList } from '@tiptap/extension-task-list';
	import { TaskItem } from '@tiptap/extension-task-item';
	import { Table } from '@tiptap/extension-table';
	import { TableRow } from '@tiptap/extension-table-row';
	import { TableCell } from '@tiptap/extension-table-cell';
	import { TableHeader } from '@tiptap/extension-table-header';
	import { Placeholder } from '@tiptap/extension-placeholder';
	import { Markdown } from '@tiptap/markdown';
	import Suggestion from '@tiptap/suggestion';
	import { createEventDispatcher } from 'svelte';
	import { pushToast } from '$lib/stores/toast';
	import {
		assistAutocomplete,
		assistCompletion,
		assistLinks,
		assistTransform,
		type AssistLinkSuggestion,
		type AssistSuggestionSource
	} from '$lib/api/assist';
	import { searchFullTextNodes } from '$lib/api/search';
	import { markdownToHTML } from '$lib/editor';
	import { renderMermaid } from '$lib/utils/mermaid';

	export let markdown = '';
	export let placeholder = 'Write your note…';
	export let noteTitle: string | undefined = undefined;
	export let namespace: string | undefined = undefined;
	export let excludeNodeId: string | undefined = undefined;

	const dispatch = createEventDispatcher<{ change: { markdown: string } }>();

	let editor: Editor | null = null;
	let editorElement: HTMLDivElement | null = null;
	let markdownTextarea: HTMLTextAreaElement | null = null;
	let mode: 'wysiwyg' | 'markdown' | 'split' = 'wysiwyg';
	let currentMarkdown = markdown;
	let settingContent = false;

	let autoCompleteSuggestion = '';
	let autoCompleteTimer: ReturnType<typeof setTimeout> | null = null;
	let aiSuggestions: string[] = [];
	let aiGroundingSources: AssistSuggestionSource[] = [];
	let semanticLinkSuggestions: AssistLinkSuggestion[] = [];
	let suggestionsStrategy = '';
	let suggestionsSourceNodes = 0;
	let linksStrategy = '';
	let linksSourceNodes = 0;
	let isLoadingSuggestions = false;
	let transformMode: 'summarize' | 'action_items' | 'refine' | 'meeting' = 'summarize';
	let transformTarget: 'append_section' | 'replace_selection' = 'append_section';
	let isTransforming = false;
	let showMermaidPreview = false;
	let mermaidContainer: HTMLDivElement | null = null;
	let mermaidHtml = '';
	let hasMermaidBlock = false;

	let wysiwygLinkPanel: HTMLDivElement | null = null;
	type WysiwygLinkSuggestion = {
		id?: string;
		title: string;
		heading?: string;
		preview?: string;
		source: 'fts' | 'semantic' | 'create';
		create?: boolean;
		score?: number;
	};
	let wysiwygLinkSuggestions: WysiwygLinkSuggestion[] = [];
	let wysiwygLinkActiveIndex = -1;
	let wysiwygLinkQuery = '';
	let wysiwygLinkAlias = '';
	let wysiwygLinkCommand: ((item: unknown) => void) | null = null;

	const AUTOCOMPLETE_MIN_CHARS = 3;
	const AUTOCOMPLETE_DELAY_MS = 520;
	const TRANSFORM_MIN_CHARS = 16;
	const TRANSFORM_SELECTION_MIN_CHARS = 8;
	const CONTEXT_WINDOW_CHARS = 360;

	function escapeHtml(value: string): string {
		return value
			.replace(/&/g, '&amp;')
			.replace(/</g, '&lt;')
			.replace(/>/g, '&gt;')
			.replace(/"/g, '&quot;')
			.replace(/'/g, '&#39;');
	}

	const mermaidHtmlOptions = {
		renderers: {
			html: (node: { value?: string }) => {
				const raw = String(node?.value ?? '');
				if (!raw) {
					return '';
				}
				return `<pre class="mv-html-block">${escapeHtml(raw)}</pre>\n`;
			}
		}
	};

	$: hasMermaidBlock = /```mermaid[\s\S]*?```/i.test(markdown);
	$: if (showMermaidPreview && hasMermaidBlock) {
		mermaidHtml = markdownToHTML(markdown || '', undefined, mermaidHtmlOptions);
	} else {
		mermaidHtml = '';
	}

	$: if (showMermaidPreview && hasMermaidBlock && mermaidContainer && mermaidHtml) {
		void (async () => {
			await tick();
			await renderMermaid(mermaidContainer);
		})();
	}

	function syncFromEditor() {
		if (!editor) return;
		const next = editor.getMarkdown().trim();
		if (next === currentMarkdown) return;
		currentMarkdown = next;
		markdown = next;
		dispatch('change', { markdown: next });
	}

	function setEditorContent(next: string) {
		if (!editor) return;
		settingContent = true;
		editor.commands.setContent(next || '', { contentType: 'markdown', emitUpdate: false });
		settingContent = false;
		currentMarkdown = next || '';
	}

	function applyCommand(command: string) {
		if (!editor || mode === 'markdown') return;
		editor.chain().focus();
		switch (command) {
			case 'bold':
				editor.chain().toggleBold().run();
				break;
			case 'italic':
				editor.chain().toggleItalic().run();
				break;
			case 'bullet':
				editor.chain().toggleBulletList().run();
				break;
			case 'ordered':
				editor.chain().toggleOrderedList().run();
				break;
			case 'h2':
				editor.chain().toggleHeading({ level: 2 }).run();
				break;
			case 'quote':
				editor.chain().toggleBlockquote().run();
				break;
			case 'link': {
				const href = window.prompt('Enter URL');
				if (href && href.trim()) {
					editor.chain().extendMarkRange('link').setLink({ href: href.trim() }).run();
				} else {
					editor.chain().unsetLink().run();
				}
				break;
			}
			case 'clear':
				editor.chain().unsetAllMarks().clearNodes().run();
				break;
			default:
				break;
		}
		syncFromEditor();
	}

	function setMode(next: typeof mode) {
		mode = next;
		if (next === 'markdown') {
			syncFromEditor();
			autoCompleteSuggestion = '';
			hideWysiwygWikiLinkSuggestions();
		} else if (editor && currentMarkdown !== markdown) {
			setEditorContent(markdown);
		}
	}

	function normalizeSuggestions(candidates: string[], prompt: string) {
		const promptNormalized = prompt.trim().toLowerCase();
		const seen = new Set<string>();
		return candidates
			.map((item) => String(item || '').trim())
			.filter((item) => item.length > 0 && item.toLowerCase() !== promptNormalized)
			.filter((item) => {
				const normalized = item.toLowerCase();
				if (seen.has(normalized)) return false;
				seen.add(normalized);
				return true;
			});
	}

	function buildAutocompleteContext(prompt: string) {
		const contextChunks: string[] = [];
		const cleanTitle = noteTitle?.trim();
		if (cleanTitle) {
			contextChunks.push(cleanTitle);
		}
		const tail = markdown.trim().slice(-CONTEXT_WINDOW_CHARS);
		if (tail) {
			contextChunks.push(tail);
		}
		if (prompt.trim()) {
			contextChunks.push(prompt.trim());
		}
		return contextChunks.join('\n').slice(0, 1000);
	}

	function dedupeSemanticLinks(items: AssistLinkSuggestion[]) {
		const seen = new Set<string>();
		const results: AssistLinkSuggestion[] = [];
		for (const item of items) {
			const title = String(item.title || '').trim();
			if (!title) continue;
			const heading = String(item.heading || '').trim();
			const key = `${String(item.node_id || '').toLowerCase()}::${title.toLowerCase()}::${heading.toLowerCase()}`;
			if (seen.has(key)) continue;
			seen.add(key);
			results.push(item);
		}
		return results;
	}

	function scheduleAutocomplete() {
		if (!editor || mode === 'markdown') return;
		if (autoCompleteTimer) clearTimeout(autoCompleteTimer);
		autoCompleteTimer = setTimeout(async () => {
			if (!editor) return;
			const { $from } = editor.state.selection;
			const lineText = $from.parent?.textContent ?? '';
			const prompt = lineText.slice(0, $from.parentOffset).trim();
			if (!prompt || prompt.length < AUTOCOMPLETE_MIN_CHARS) {
				autoCompleteSuggestion = '';
				return;
			}
			try {
				const response = await assistAutocomplete({ text: prompt, limit: 5, namespace });
				let unique = normalizeSuggestions(response.completions || [], prompt);
				if (unique.length === 0) {
					const contextualPrompt = buildAutocompleteContext(prompt);
					if (contextualPrompt.length >= AUTOCOMPLETE_MIN_CHARS) {
						const semanticResponse = await assistCompletion({
							text: contextualPrompt,
							limit: 2,
							namespace
						});
						unique = normalizeSuggestions(semanticResponse.suggestions || [], prompt);
						if (semanticResponse.sources?.length) {
							aiGroundingSources = semanticResponse.sources.slice(0, 4);
						}
					}
				}
				autoCompleteSuggestion = unique[0] || '';
			} catch {
				autoCompleteSuggestion = '';
			}
		}, AUTOCOMPLETE_DELAY_MS);
	}

	function acceptAutocomplete() {
		if (!editor || !autoCompleteSuggestion) return;
		editor.chain().focus().insertContent(autoCompleteSuggestion).run();
		autoCompleteSuggestion = '';
		syncFromEditor();
	}

	function insertAtMarkdownSelection(text: string) {
		if (!markdownTextarea) return false;
		const start = markdownTextarea.selectionStart ?? markdown.length;
		const end = markdownTextarea.selectionEnd ?? start;
		const current = markdown;
		const next = `${current.slice(0, start)}${text}${current.slice(end)}`;
		markdown = next;
		setEditorContent(next);
		syncFromEditor();
		const cursor = start + text.length;
		markdownTextarea.focus();
		markdownTextarea.setSelectionRange(cursor, cursor);
		return true;
	}

	function insertWikiLinkToken(title: string, heading?: string) {
		const cleanTitle = String(title || '').trim();
		if (!cleanTitle) return;
		const cleanHeading = String(heading || '').trim();
		const target = cleanHeading ? `${cleanTitle}#${cleanHeading}` : cleanTitle;
		const token = `[[${target}]]`;
		if (mode === 'markdown' && insertAtMarkdownSelection(token)) {
			return;
		}
		if (editor) {
			editor.chain().focus().insertContent(token).run();
			syncFromEditor();
			return;
		}
		const current = markdown.trim();
		const separator = current.length === 0 ? '' : '\n\n';
		const next = `${current}${separator}${token}`;
		markdown = next;
		setEditorContent(next);
		syncFromEditor();
	}

	async function requestSuggestions() {
		if (isLoadingSuggestions) return;
		const text = markdown.trim();
		if (text.length < TRANSFORM_MIN_CHARS) {
			pushToast('Add a bit more context before requesting suggestions.', 'warning');
			return;
		}
		isLoadingSuggestions = true;
		try {
			const query = buildAutocompleteContext(text);
			const [suggestionResponse, linkResponse] = await Promise.all([
				assistCompletion({ text: query, limit: 4, namespace }),
				assistLinks({
					text: query,
					limit: 6,
					namespace,
					exclude_node_id: excludeNodeId
				})
			]);
			aiSuggestions = suggestionResponse.suggestions || [];
			aiGroundingSources = (suggestionResponse.sources || []).slice(0, 5);
			suggestionsStrategy = suggestionResponse.strategy || '';
			suggestionsSourceNodes = suggestionResponse.source_nodes || 0;
			semanticLinkSuggestions = dedupeSemanticLinks(linkResponse.suggestions || []);
			linksStrategy = linkResponse.strategy || '';
			linksSourceNodes = linkResponse.source_nodes || 0;
			if (aiSuggestions.length === 0) {
				pushToast('No suggestions returned.', 'info');
			}
		} catch {
			pushToast('AI suggestions unavailable.', 'warning');
		} finally {
			isLoadingSuggestions = false;
		}
	}

	function insertSuggestion(suggestion: string) {
		if (mode === 'markdown' && insertAtMarkdownSelection(suggestion)) {
			pushToast('Suggestion inserted.', 'success');
			return;
		}
		if (editor) {
			editor.chain().focus().insertContent(suggestion).run();
			syncFromEditor();
			pushToast('Suggestion inserted.', 'success');
			return;
		}
		const current = markdown.trim();
		const separator = current.length === 0 ? '' : '\n\n';
		const next = `${current}${separator}${suggestion}`;
		markdown = next;
		setEditorContent(next);
		syncFromEditor();
		pushToast('Suggestion inserted.', 'success');
	}

	function insertGroundingSource(source: AssistSuggestionSource) {
		insertWikiLinkToken(source.title);
		pushToast(`Inserted [[${source.title}]]`, 'success');
	}

	function insertSemanticLinkSuggestion(link: AssistLinkSuggestion) {
		insertWikiLinkToken(link.title, link.heading);
		const target = link.heading ? `${link.title}#${link.heading}` : link.title;
		pushToast(`Inserted [[${target}]]`, 'success');
	}

	function getWysiwygSelectionMarkdown() {
		if (!editor) return null;
		const selection = editor.state.selection;
		if (selection.empty) return null;
		const fragment = selection.content().content;
		if (!fragment || fragment.size === 0) return null;
		const jsonContent = typeof fragment.toJSON === 'function' ? fragment.toJSON() : null;
		if (!jsonContent) {
			return editor.state.doc.textBetween(selection.from, selection.to, '\n');
		}
		const docJson = { type: 'doc', content: jsonContent };
		if (editor.markdown && typeof editor.markdown.serialize === 'function') {
			return editor.markdown.serialize(docJson);
		}
		return editor.state.doc.textBetween(selection.from, selection.to, '\n');
	}

	function replaceMarkdownSelection(replacement: string) {
		if (!markdownTextarea) return false;
		const start = markdownTextarea.selectionStart ?? 0;
		const end = markdownTextarea.selectionEnd ?? start;
		if (end <= start) return false;
		const current = markdown;
		const next = `${current.slice(0, start)}${replacement}${current.slice(end)}`;
		markdown = next;
		setEditorContent(next);
		syncFromEditor();
		return true;
	}

	function replaceWysiwygSelection(replacement: string) {
		if (!editor) return false;
		if (editor.state.selection.empty) return false;
		editor.chain().focus().deleteSelection().insertContent(replacement, { contentType: 'markdown' }).run();
		syncFromEditor();
		return true;
	}

	async function requestTransform() {
		if (isTransforming) return;
		let text = '';
		let replaced = false;
		if (transformTarget === 'replace_selection') {
			if (mode === 'markdown') {
				const selected = markdownTextarea?.value?.slice(
					markdownTextarea.selectionStart ?? 0,
					markdownTextarea.selectionEnd ?? 0
				) ?? '';
				if (selected.trim().length < TRANSFORM_SELECTION_MIN_CHARS) {
					pushToast('Select at least 8 characters to replace.', 'warning');
					return;
				}
				text = selected.trim();
			} else {
				const selection = getWysiwygSelectionMarkdown();
				if (!selection || selection.trim().length < TRANSFORM_SELECTION_MIN_CHARS) {
					pushToast('Select at least 8 characters to replace.', 'warning');
					return;
				}
				text = selection.trim();
			}
		} else {
			text = markdown.trim();
			if (text.length < TRANSFORM_MIN_CHARS) {
				pushToast('Add more context before running transform.', 'warning');
				return;
			}
		}

		isTransforming = true;
		try {
			const response = await assistTransform({ text, mode: transformMode, limit: 4, namespace });
			const transformed = String(response.transformed_text || '').trim();
			if (!transformed) {
				pushToast('Transform returned no content.', 'warning');
				return;
			}
			if (transformTarget === 'replace_selection') {
				if (mode === 'markdown') {
					replaced = replaceMarkdownSelection(transformed);
				} else {
					replaced = replaceWysiwygSelection(transformed);
				}
				if (replaced) {
					pushToast('Selection replaced with AI transform.', 'success');
					return;
				}
			}

				const sectionTitle =
					transformMode === 'summarize'
						? 'AI Summary'
						: transformMode === 'action_items'
							? 'AI Action Items'
							: transformMode === 'meeting'
								? 'AI Meeting Notes'
								: 'AI Refined Draft';
			const current = markdown.trim();
			const separator = current.length === 0 ? '' : '\n\n';
			const next = `${current}${separator}## ${sectionTitle}\n${transformed}`;
			markdown = next;
			setEditorContent(next);
			syncFromEditor();
			pushToast('AI transform inserted.', 'success');
		} catch {
			pushToast('AI transform failed.', 'warning');
		} finally {
			isTransforming = false;
		}
	}

	function createWysiwygLinkPanel() {
		if (wysiwygLinkPanel) return;
		const panel = document.createElement('div');
		panel.className = 'mv-link-suggestions';
		panel.style.display = 'none';
		panel.setAttribute('role', 'listbox');
		panel.setAttribute('aria-label', 'Wiki link suggestions');
		document.body.appendChild(panel);
		wysiwygLinkPanel = panel;
	}

	function hideWysiwygWikiLinkSuggestions() {
		if (!wysiwygLinkPanel) return;
		wysiwygLinkPanel.style.display = 'none';
		wysiwygLinkPanel.innerHTML = '';
		wysiwygLinkSuggestions = [];
		wysiwygLinkActiveIndex = -1;
		wysiwygLinkCommand = null;
	}

	function positionWysiwygLinkPanel(clientRect?: (() => DOMRect | null) | null) {
		if (!wysiwygLinkPanel || !clientRect) return;
		const rect = clientRect();
		if (!rect) return;
		wysiwygLinkPanel.style.top = `${rect.bottom + window.scrollY + 6}px`;
		wysiwygLinkPanel.style.left = `${rect.left + window.scrollX}px`;
	}

	function renderWysiwygLinkSuggestions(
		items: typeof wysiwygLinkSuggestions,
		query: string,
		clientRect?: (() => DOMRect | null) | null
	) {
		if (!wysiwygLinkPanel) return;
		wysiwygLinkPanel.innerHTML = '';
		wysiwygLinkSuggestions = Array.isArray(items) ? items : [];
		wysiwygLinkActiveIndex = wysiwygLinkSuggestions.length > 0 ? 0 : -1;
		wysiwygLinkQuery = query;

		const header = document.createElement('div');
		header.className = 'mv-link-header';
		header.textContent = query ? `Wiki links for "${query}"` : 'Type to search notes…';
		wysiwygLinkPanel.appendChild(header);

		if (wysiwygLinkSuggestions.length === 0) {
			const empty = document.createElement('div');
			empty.className = 'mv-link-empty';
			empty.textContent = query ? 'No matches found.' : 'Keep typing to search.';
			wysiwygLinkPanel.appendChild(empty);
		} else {
			wysiwygLinkSuggestions.forEach((item, index) => {
				const entry = document.createElement('button');
				entry.type = 'button';
				entry.className = `mv-link-item ${index === wysiwygLinkActiveIndex ? 'active' : ''}`;
				entry.dataset.index = String(index);
				if (item.create) {
					entry.innerHTML = `<span class=\"mv-link-create\">Create new note: \"${item.title}\"</span>`;
				} else {
					const preview = item.preview ? `<div class=\"mv-link-preview\">${item.preview}</div>` : '';
					entry.innerHTML = `\n<div class=\"mv-link-title\">${item.title}</div>\n<div class=\"mv-link-meta\">${item.source.toUpperCase()}</div>\n${preview}`;
				}
				entry.addEventListener('click', () => selectWysiwygLinkSuggestion(index));
				wysiwygLinkPanel?.appendChild(entry);
			});
		}

		wysiwygLinkPanel.style.display = 'block';
		positionWysiwygLinkPanel(clientRect);
	}

	function moveWysiwygLinkSelection(direction: number) {
		if (!wysiwygLinkSuggestions.length) return;
		const size = wysiwygLinkSuggestions.length;
		wysiwygLinkActiveIndex = (wysiwygLinkActiveIndex + direction + size) % size;
		const items = wysiwygLinkPanel?.querySelectorAll('.mv-link-item') || [];
		items.forEach((item, index) => {
			item.classList.toggle('active', index === wysiwygLinkActiveIndex);
		});
	}

	function selectWysiwygLinkSuggestion(index: number) {
		if (!wysiwygLinkCommand) return;
		const item = wysiwygLinkSuggestions[index];
		if (!item) return;
		wysiwygLinkCommand(item);
		hideWysiwygWikiLinkSuggestions();
	}

	function handleWysiwygLinkKeyDown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			hideWysiwygWikiLinkSuggestions();
			return true;
		}
		if (event.key === 'ArrowDown') {
			event.preventDefault();
			moveWysiwygLinkSelection(1);
			return true;
		}
		if (event.key === 'ArrowUp') {
			event.preventDefault();
			moveWysiwygLinkSelection(-1);
			return true;
		}
		if (event.key === 'Enter' || event.key === 'Tab') {
			event.preventDefault();
			selectWysiwygLinkSuggestion(wysiwygLinkActiveIndex >= 0 ? wysiwygLinkActiveIndex : 0);
			return true;
		}
		return false;
	}

	function insertWysiwygWikiLink(editorRef: Editor, range: { from: number; to: number }, item: any) {
		const title = String(item?.title || wysiwygLinkQuery || '').trim();
		if (!title) return;
		const heading = String(item?.heading || '').trim();
		const alias = String(wysiwygLinkAlias || '').trim();
		const target = heading ? `${title}#${heading}` : title;
		const linkText = alias ? `[[${target}|${alias}]]` : `[[${target}]]`;
		editorRef.chain().focus().insertContentAt(range, linkText).run();
		syncFromEditor();
	}

	async function fetchWysiwygWikiLinkSuggestions(rawQuery: string) {
		let query = String(rawQuery || '');
		if (query.startsWith('[')) query = query.slice(1);
		const [searchPart, aliasPart] = query.split('|');
		wysiwygLinkAlias = String(aliasPart || '').trim();
		const searchQuery = String(searchPart || '').trim();
		if (!searchQuery) return [];

		const limit = 8;
		let fts: typeof wysiwygLinkSuggestions = [];
		try {
			const results = await searchFullTextNodes(searchQuery, limit);
			fts = results
				.map((result) => ({
					id: result.node?.id,
					title: result.node?.title || 'Untitled',
					preview: (result.node?.content || '').slice(0, 140),
					source: 'fts' as const,
					score: result.score || 0
				}))
				.filter((item) => item.title);
		} catch {
			fts = [];
		}

		const wordCount = searchQuery.split(/\s+/).filter(Boolean).length;
		const needsSemantic = fts.length < 3 || searchQuery.length > 25 || wordCount > 3;
		let semantic: typeof wysiwygLinkSuggestions = [];
		if (needsSemantic) {
			try {
				const response = await assistLinks({
					text: searchQuery,
					limit,
					namespace,
					exclude_node_id: excludeNodeId
				});
				semantic = (response.suggestions || []).map((item) => ({
					id: item.node_id,
					title: item.title,
					heading: item.heading,
					preview: item.preview || '',
					source: 'semantic' as const,
					score: item.score || 0
				}));
			} catch {
				semantic = [];
			}
		}

		const normalized = searchQuery.toLowerCase();
		const merged: typeof wysiwygLinkSuggestions = [];
		const seen = new Set<string>();
		const add = (item: typeof merged[number]) => {
			const key = item.id || `${item.title.toLowerCase()}`;
			if (seen.has(key)) return;
			seen.add(key);
			let rank = item.source === 'fts' ? 1 : 0.4;
			const titleLower = item.title.toLowerCase();
			if (titleLower === normalized) rank += 1.5;
			else if (titleLower.startsWith(normalized)) rank += 0.8;
			else if (titleLower.includes(normalized)) rank += 0.4;
			rank += item.score || 0;
			merged.push({ ...item, score: rank });
		};
		fts.forEach(add);
		semantic.forEach(add);

		const results = merged.sort((a, b) => (b.score || 0) - (a.score || 0)).slice(0, limit);
		if (results.length === 0) {
			return [{ title: searchQuery, source: 'create' as const, create: true }];
		}
		return results;
	}

	function handleGlobalKeydown(event: KeyboardEvent) {
		if (!(event.metaKey || event.ctrlKey)) return;
		const key = event.key.toLowerCase();
		if (event.shiftKey && key === 'm') {
			event.preventDefault();
			setMode(mode === 'markdown' ? 'wysiwyg' : 'markdown');
			return;
		}
		if (event.shiftKey && key === 'k') {
			event.preventDefault();
			void requestSuggestions();
			return;
		}
		if (key === 'j') {
			event.preventDefault();
			void requestSuggestions();
		}
	}

	onMount(() => {
		if (!editorElement) return;
		createWysiwygLinkPanel();
		window.addEventListener('keydown', handleGlobalKeydown);

		const wikiLinkSuggestion = Extension.create({
			name: 'wikiLinkSuggestion',
			addProseMirrorPlugins() {
				return [
					Suggestion({
						editor: this.editor,
						char: '[',
						allowSpaces: true,
						allow: ({ state, range }) => {
							const before = state.doc.textBetween(
								Math.max(range.from - 1, 0),
								range.from + 1,
								'\0',
								'\0'
							);
							const at = state.doc.textBetween(range.from, range.from + 2, '\0', '\0');
							return before === '[[' || at === '[[';
						},
						items: async ({ query }) => await fetchWysiwygWikiLinkSuggestions(query),
						command: ({ editor, range, props }) => insertWysiwygWikiLink(editor, range, props),
						render: () => ({
							onStart: (props) => {
								wysiwygLinkCommand = props.command;
								renderWysiwygLinkSuggestions(props.items, props.query, props.clientRect ?? null);
							},
							onUpdate: (props) => {
								wysiwygLinkCommand = props.command;
								renderWysiwygLinkSuggestions(props.items, props.query, props.clientRect ?? null);
							},
							onKeyDown: ({ event }) => handleWysiwygLinkKeyDown(event),
							onExit: () => hideWysiwygWikiLinkSuggestions()
						})
					})
				];
			}
		});

		editor = new Editor({
			element: editorElement,
			extensions: [
				StarterKit,
				Image.configure({ inline: false, allowBase64: true }),
				Link.configure({ openOnClick: false }),
				TaskList,
				TaskItem.configure({ nested: true }),
				Table.configure({ resizable: true }),
				TableRow,
				TableHeader,
				TableCell,
				Placeholder.configure({ placeholder }),
				wikiLinkSuggestion,
				Markdown
			],
			content: markdown || '',
			contentType: 'markdown',
			editorProps: {
				handleKeyDown: (_view, event) => {
					if (autoCompleteSuggestion && event.key === 'Tab') {
						event.preventDefault();
						acceptAutocomplete();
						return true;
					}
					if (autoCompleteSuggestion && event.key === 'Escape') {
						autoCompleteSuggestion = '';
						return true;
					}
					return false;
				}
			},
			onUpdate: () => {
				if (settingContent) return;
				syncFromEditor();
				scheduleAutocomplete();
			}
		});
		currentMarkdown = markdown;
	});

	$: if (editor && !settingContent && markdown !== currentMarkdown) {
		setEditorContent(markdown);
	}

	onDestroy(() => {
		if (autoCompleteTimer) clearTimeout(autoCompleteTimer);
		editor?.destroy();
		wysiwygLinkPanel?.remove();
		window.removeEventListener('keydown', handleGlobalKeydown);
	});
</script>

<div class="mv-editor">
	<div class="mv-editor-toolbar">
		<div class="mv-editor-modes">
			<button type="button" class:active={mode === 'wysiwyg'} on:click={() => setMode('wysiwyg')}>
				WYSIWYG
			</button>
			<button type="button" class:active={mode === 'markdown'} on:click={() => setMode('markdown')}>
				Markdown
			</button>
			<button type="button" class:active={mode === 'split'} on:click={() => setMode('split')}>
				Split
			</button>
		</div>
		<div class="mv-editor-actions">
			<button type="button" on:click={() => applyCommand('bold')}>B</button>
			<button type="button" on:click={() => applyCommand('italic')}>I</button>
			<button type="button" on:click={() => applyCommand('bullet')}>• List</button>
			<button type="button" on:click={() => applyCommand('ordered')}>1. List</button>
			<button type="button" on:click={() => applyCommand('h2')}>H2</button>
			<button type="button" on:click={() => applyCommand('quote')}>Quote</button>
			<button type="button" on:click={() => applyCommand('link')}>Link</button>
			<button type="button" on:click={() => applyCommand('clear')}>Clear</button>
			<button type="button" on:click={requestSuggestions} disabled={isLoadingSuggestions}>
				AI Suggest
			</button>
		</div>
	</div>

		<div class="mv-editor-transform">
			<select bind:value={transformMode}>
				<option value="summarize">Summarize</option>
				<option value="action_items">Action Items</option>
				<option value="refine">Refine</option>
				<option value="meeting">Meeting Notes</option>
			</select>
			<select bind:value={transformTarget}>
				<option value="append_section">Insert Section</option>
				<option value="replace_selection">Replace Selection</option>
			</select>
			<button type="button" on:click={requestTransform} disabled={isTransforming}>
				{isTransforming ? 'Transforming…' : 'AI Transform'}
			</button>
			<button
				type="button"
				class:active={showMermaidPreview}
				on:click={() => (showMermaidPreview = !showMermaidPreview)}
				disabled={!hasMermaidBlock}
				title={hasMermaidBlock ? 'Toggle Mermaid preview' : 'No Mermaid blocks found'}
			>
				Mermaid Preview
			</button>
		</div>

		<div class={`mv-editor-surface mode-${mode}`}>
			<div class="mv-editor-rich" bind:this={editorElement}></div>
			<textarea
			class="mv-editor-markdown"
			bind:this={markdownTextarea}
			bind:value={markdown}
			on:input={() => {
				if (mode !== 'markdown') return;
				setEditorContent(markdown);
				syncFromEditor();
			}}
			placeholder="Markdown"
			></textarea>
		</div>

		{#if showMermaidPreview && hasMermaidBlock}
			<div class="mv-editor-mermaid-preview" bind:this={mermaidContainer}>
				{@html mermaidHtml}
			</div>
		{/if}

	{#if autoCompleteSuggestion}
		<div class="mv-editor-autocomplete">
			<span>AI autocomplete:</span>
			<strong>{autoCompleteSuggestion}</strong>
			<button type="button" on:click={acceptAutocomplete}>Tab to accept</button>
		</div>
	{/if}

	{#if aiSuggestions.length}
		<div class="mv-editor-suggestions">
			{#each aiSuggestions as suggestion (suggestion)}
				<button type="button" on:click={() => insertSuggestion(suggestion)}>
					{suggestion}
				</button>
			{/each}
		</div>
	{/if}

	{#if aiGroundingSources.length}
		<div class="mv-editor-grounding">
			<div class="mv-editor-grounding-header">
				<span>Grounded context</span>
				{#if suggestionsSourceNodes > 0}
					<small>{suggestionsSourceNodes} source nodes {suggestionsStrategy ? `· ${suggestionsStrategy}` : ''}</small>
				{/if}
			</div>
			<div class="mv-editor-grounding-chips">
				{#each aiGroundingSources as source (source.node_id)}
					<button type="button" on:click={() => insertGroundingSource(source)}>
						<span>{source.title}</span>
						<small>{source.score.toFixed(2)}</small>
					</button>
				{/each}
			</div>
		</div>
	{/if}

	{#if semanticLinkSuggestions.length}
		<div class="mv-editor-semantic-links">
			<div class="mv-editor-grounding-header">
				<span>Semantic link suggestions</span>
				{#if linksSourceNodes > 0}
					<small>{linksSourceNodes} source nodes {linksStrategy ? `· ${linksStrategy}` : ''}</small>
				{/if}
			</div>
			<div class="mv-editor-semantic-list">
				{#each semanticLinkSuggestions as link (`${link.node_id || link.title}#${link.heading || ''}`)}
					<button type="button" on:click={() => insertSemanticLinkSuggestion(link)}>
						<span>{link.title}{link.heading ? `#${link.heading}` : ''}</span>
						<small>{typeof link.score === 'number' ? link.score.toFixed(2) : ''}</small>
					</button>
				{/each}
			</div>
		</div>
	{/if}
</div>

<style>
	.mv-editor {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.mv-editor-toolbar {
		display: flex;
		flex-wrap: wrap;
		gap: 0.75rem;
		justify-content: space-between;
	}

	.mv-editor-modes,
	.mv-editor-actions {
		display: flex;
		gap: 0.5rem;
		flex-wrap: wrap;
	}

	.mv-editor-modes button,
	.mv-editor-actions button,
	.mv-editor-transform button,
	.mv-editor-transform select {
		border: 1px solid rgba(148, 163, 184, 0.3);
		background: rgba(15, 23, 42, 0.8);
		color: #e2e8f0;
		font-size: 0.75rem;
		padding: 0.35rem 0.6rem;
		border-radius: 0.6rem;
		cursor: pointer;
	}

	.mv-editor-modes button.active {
		background: rgba(56, 189, 248, 0.2);
		border-color: rgba(56, 189, 248, 0.6);
		color: #e0f2fe;
	}

	.mv-editor-transform {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
		align-items: center;
	}

	.mv-editor-transform button.active {
		background: rgba(34, 197, 94, 0.2);
		border-color: rgba(34, 197, 94, 0.5);
		color: #dcfce7;
	}

	.mv-editor-mermaid-preview {
		border: 1px solid rgba(148, 163, 184, 0.2);
		background: rgba(15, 23, 42, 0.4);
		border-radius: 0.75rem;
		padding: 0.75rem;
	}

	.mv-editor-mermaid-preview .mv-mermaid,
	.mv-editor-mermaid-preview .mermaid {
		background: rgba(15, 23, 42, 0.6);
		border-radius: 0.5rem;
		padding: 0.5rem;
		overflow-x: auto;
	}

	.mv-editor-mermaid-preview .mv-html-block {
		background: rgba(15, 23, 42, 0.6);
		border-radius: 0.5rem;
		padding: 0.5rem;
		overflow-x: auto;
		color: #e2e8f0;
	}

	.mv-editor-surface {
		display: grid;
		gap: 0;
		border: 1px solid rgba(148, 163, 184, 0.25);
		border-radius: 0.75rem;
		overflow: hidden;
		min-height: 220px;
		background: rgba(15, 23, 42, 0.6);
	}

	.mv-editor-surface.mode-wysiwyg,
	.mv-editor-surface.mode-markdown {
		grid-template-columns: 1fr;
	}

	.mv-editor-surface.mode-split {
		grid-template-columns: 1fr 1fr;
	}

	.mv-editor-rich {
		padding: 0.75rem;
		min-height: 220px;
		background: rgba(15, 23, 42, 0.4);
	}

	.mv-editor-rich :global(.ProseMirror) {
		outline: none;
		min-height: 220px;
		color: #e2e8f0;
	}

	.mv-editor-rich :global(.ProseMirror p.is-editor-empty::before) {
		content: attr(data-placeholder);
		float: left;
		color: rgba(148, 163, 184, 0.8);
		pointer-events: none;
		height: 0;
	}

	.mv-editor-markdown {
		border: none;
		background: rgba(15, 23, 42, 0.6);
		color: #e2e8f0;
		padding: 0.75rem;
		font-size: 0.8rem;
		min-height: 220px;
		resize: vertical;
		font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono',
			'Courier New', monospace;
	}

	.mv-editor-surface.mode-wysiwyg .mv-editor-markdown {
		display: none;
	}

	.mv-editor-surface.mode-markdown .mv-editor-rich {
		display: none;
	}

	.mv-editor-autocomplete {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 0.75rem;
		color: rgba(226, 232, 240, 0.8);
	}

	.mv-editor-autocomplete button {
		border: 1px solid rgba(148, 163, 184, 0.3);
		background: rgba(15, 23, 42, 0.8);
		color: #e2e8f0;
		font-size: 0.7rem;
		padding: 0.2rem 0.5rem;
		border-radius: 0.5rem;
	}

	.mv-editor-suggestions {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
	}

	.mv-editor-suggestions button {
		border: 1px solid rgba(148, 163, 184, 0.3);
		background: rgba(56, 189, 248, 0.15);
		color: #e0f2fe;
		font-size: 0.7rem;
		padding: 0.3rem 0.6rem;
		border-radius: 999px;
		cursor: pointer;
	}

	.mv-editor-grounding,
	.mv-editor-semantic-links {
		display: flex;
		flex-direction: column;
		gap: 0.45rem;
		border: 1px solid rgba(56, 189, 248, 0.22);
		background: rgba(15, 23, 42, 0.5);
		border-radius: 0.7rem;
		padding: 0.6rem;
	}

	.mv-editor-grounding-header {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: 0.6rem;
		color: rgba(226, 232, 240, 0.9);
		font-size: 0.72rem;
	}

	.mv-editor-grounding-header small {
		color: rgba(148, 163, 184, 0.9);
		font-size: 0.66rem;
	}

	.mv-editor-grounding-chips,
	.mv-editor-semantic-list {
		display: flex;
		flex-wrap: wrap;
		gap: 0.45rem;
	}

	.mv-editor-grounding-chips button,
	.mv-editor-semantic-list button {
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		border: 1px solid rgba(148, 163, 184, 0.35);
		border-radius: 999px;
		background: rgba(15, 23, 42, 0.85);
		color: #e2e8f0;
		padding: 0.26rem 0.55rem;
		font-size: 0.7rem;
		cursor: pointer;
	}

	.mv-editor-grounding-chips button:hover,
	.mv-editor-semantic-list button:hover {
		border-color: rgba(56, 189, 248, 0.55);
		background: rgba(56, 189, 248, 0.12);
	}

	.mv-editor-grounding-chips button small,
	.mv-editor-semantic-list button small {
		color: rgba(148, 163, 184, 0.92);
		font-size: 0.62rem;
	}

	.mv-link-suggestions {
		position: absolute;
		z-index: 50;
		min-width: 220px;
		max-width: 360px;
		border: 1px solid rgba(56, 189, 248, 0.4);
		border-radius: 0.75rem;
		background: rgba(15, 23, 42, 0.96);
		box-shadow: 0 12px 28px rgba(15, 23, 42, 0.4);
		overflow: hidden;
	}

	.mv-link-header {
		padding: 0.5rem 0.75rem;
		font-size: 0.7rem;
		color: #bae6fd;
		background: rgba(56, 189, 248, 0.15);
		border-bottom: 1px solid rgba(56, 189, 248, 0.25);
	}

	.mv-link-empty {
		padding: 0.6rem 0.75rem;
		font-size: 0.7rem;
		color: rgba(226, 232, 240, 0.7);
	}

	.mv-link-item {
		width: 100%;
		border: none;
		background: transparent;
		text-align: left;
		padding: 0.6rem 0.75rem;
		cursor: pointer;
	}

	.mv-link-item:hover,
	.mv-link-item:focus,
	.mv-link-item.active {
		background: rgba(56, 189, 248, 0.18);
		outline: none;
	}

	.mv-link-title {
		font-size: 0.8rem;
		font-weight: 600;
		color: #f8fafc;
	}

	.mv-link-meta {
		font-size: 0.6rem;
		color: rgba(148, 163, 184, 0.8);
		letter-spacing: 0.04em;
		text-transform: uppercase;
	}

	.mv-link-preview {
		margin-top: 0.25rem;
		font-size: 0.7rem;
		color: rgba(226, 232, 240, 0.8);
	}

	.mv-link-create {
		font-size: 0.75rem;
		color: #e0f2fe;
		font-weight: 600;
	}
</style>
