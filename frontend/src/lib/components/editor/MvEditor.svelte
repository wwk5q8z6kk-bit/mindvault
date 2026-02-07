<script lang="ts">
	/**
	 * MvEditor - Modular Rich Text Editor
	 *
	 * A proprietary contenteditable-based markdown editor.
	 * Orchestrates sub-components: Toolbar, Content, Footer, Modals.
	 * Supports extensible plugin system.
	 */
	import { createEventDispatcher, onMount, onDestroy } from 'svelte';
	import EditorToolbar from './EditorToolbar.svelte';
	import EditorContent from './EditorContent.svelte';
	import EditorFooter from './EditorFooter.svelte';
	import LinkModal from './LinkModal.svelte';
	import ImageModal from './ImageModal.svelte';

	// Plugin system
	import {
		createPluginRegistry,
		builtinPlugins,
		type EditorPlugin,
		type SlashItem as PluginSlashItem
	} from '$lib/editor/plugins';

	// Props
	export let value = '';
	export let placeholder = 'Start writing...';
	export let readonly = false;
	/** Additional plugins to register */
	export let plugins: EditorPlugin[] = [];
	/** Whether to use built-in plugins (default: true) */
	export let useBuiltinPlugins = true;

	const dispatch = createEventDispatcher<{
		change: { markdown: string; html: string };
		save: { markdown: string };
	}>();

	// Plugin registry
	const pluginRegistry = createPluginRegistry();

	// State
	type EditorMode = 'rich' | 'source';
	let mode: EditorMode = 'rich';
	let contentComponent: EditorContent;
	let sourceEl: HTMLTextAreaElement;
	let toolbarState = {
		bold: false,
		italic: false,
		strikethrough: false,
		orderedList: false,
		unorderedList: false,
		heading: 0,
		blockquote: false,
		code: false
	};
	let undoStack: string[] = [];
	let redoStack: string[] = [];
	let snapshotTimer: ReturnType<typeof setTimeout> | null = null;
	let showLinkModal = false;
	let showImageModal = false;
	let linkUrl = '';
	let linkText = '';
	let imageUrl = '';
	let imageAlt = '';
	let savedSelection: Range | null = null;
	let isEmpty = true;
	let wordCount = 0;

	// Built-in slash items (fallback when plugins aren't ready)
	const builtinSlashItems = [
		{ id: 'h1', label: 'Heading 1', description: 'Large section heading', icon: 'H1', action: () => setHeading(1) },
		{ id: 'h2', label: 'Heading 2', description: 'Medium section heading', icon: 'H2', action: () => setHeading(2) },
		{ id: 'h3', label: 'Heading 3', description: 'Small section heading', icon: 'H3', action: () => setHeading(3) },
		{ id: 'bullet', label: 'Bullet List', description: 'Unordered list', icon: '•', action: () => toggleUnorderedList() },
		{ id: 'numbered', label: 'Numbered List', description: 'Ordered list', icon: '1.', action: () => toggleOrderedList() },
		{ id: 'task', label: 'Task List', description: 'Checklist items', icon: '☑', action: () => insertTaskList() },
		{ id: 'quote', label: 'Blockquote', description: 'Indented quote block', icon: '"', action: () => toggleBlockquote() },
		{ id: 'code', label: 'Code Block', description: 'Syntax-highlighted code', icon: '</>', action: () => toggleCodeBlock() },
		{ id: 'divider', label: 'Divider', description: 'Horizontal rule', icon: '—', action: () => insertHorizontalRule() },
		{ id: 'table', label: 'Table', description: 'Insert 3x3 table', icon: '⊞', action: () => insertTable(3, 3) },
		{ id: 'link', label: 'Link', description: 'Insert hyperlink', icon: '🔗', action: () => insertLink() },
		{ id: 'image', label: 'Image', description: 'Insert image from URL', icon: '🖼', action: () => insertImageFromUrl() }
	];

	// Combine built-in and plugin slash items
	$: allSlashItems = builtinSlashItems;

	// Handle plugin events
	function handlePluginEvent(event: string, data?: unknown) {
		switch (event) {
			case 'showLinkModal':
				insertLink();
				break;
			case 'showImageModal':
				insertImageFromUrl();
				break;
			default:
				// Forward to parent
				dispatch(event as any, data);
		}
	}

	onMount(() => {
		// Register plugins
		if (useBuiltinPlugins) {
			pluginRegistry.registerAll(builtinPlugins);
		}
		if (plugins.length > 0) {
			pluginRegistry.registerAll(plugins);
		}

		if (value && contentComponent) {
			contentComponent.setContent(markdownToHtml(value));
		}
		isEmpty = isEditorEmpty();
		pushSnapshot();
	});

	onDestroy(() => {
		if (snapshotTimer) clearTimeout(snapshotTimer);
		pluginRegistry.destroy();
	});

	// --- Markdown ↔ HTML Conversion ---

	function markdownToHtml(md: string): string {
		let html = '';
		const lines = md.split('\n');
		let i = 0;
		let inCodeBlock = false;
		let codeBlockContent = '';
		let codeBlockLang = '';
		let inList: 'ul' | 'ol' | null = null;

		while (i < lines.length) {
			const line = lines[i];

			// Fenced code blocks
			if (line.trimStart().startsWith('```')) {
				if (inCodeBlock) {
					html += `<pre><code class="language-${escapeHtml(codeBlockLang)}">${escapeHtml(codeBlockContent.replace(/\n$/, ''))}</code></pre>`;
					inCodeBlock = false;
					codeBlockContent = '';
					codeBlockLang = '';
				} else {
					if (inList) {
						html += inList === 'ul' ? '</ul>' : '</ol>';
						inList = null;
					}
					inCodeBlock = true;
					codeBlockLang = line.trimStart().slice(3).trim();
				}
				i++;
				continue;
			}

			if (inCodeBlock) {
				codeBlockContent += line + '\n';
				i++;
				continue;
			}

			// Blank line closes list
			if (line.trim() === '') {
				if (inList) {
					html += inList === 'ul' ? '</ul>' : '</ol>';
					inList = null;
				}
				html += '<p><br></p>';
				i++;
				continue;
			}

			// Markdown table
			if (line.trim().startsWith('|') && line.trim().endsWith('|')) {
				if (inList) {
					html += inList === 'ul' ? '</ul>' : '</ol>';
					inList = null;
				}
				const tableRows: string[][] = [];
				while (i < lines.length && lines[i].trim().startsWith('|') && lines[i].trim().endsWith('|')) {
					const row = lines[i].trim();
					if (/^\|[\s:-]+(?:\|[\s:-]+)+\|$/.test(row)) {
						i++;
						continue;
					}
					const cells = row.slice(1, -1).split('|').map((c) => c.trim());
					tableRows.push(cells);
					i++;
				}
				if (tableRows.length > 0) {
					html += '<table data-mv-table="true">';
					tableRows.forEach((cells, rowIdx) => {
						html += '<tr>';
						cells.forEach((cell) => {
							const tag = rowIdx === 0 ? 'th' : 'td';
							html += `<${tag}>${inlineMarkdown(cell)}</${tag}>`;
						});
						html += '</tr>';
					});
					html += '</table>';
				}
				continue;
			}

			// Image
			const imageMatch = line.match(/^!\[([^\]]*)\]\(([^)]+)\)\s*$/);
			if (imageMatch) {
				if (inList) {
					html += inList === 'ul' ? '</ul>' : '</ol>';
					inList = null;
				}
				html += `<p><img src="${escapeHtml(imageMatch[2])}" alt="${escapeHtml(imageMatch[1])}" style="max-width:100%;border-radius:8px;margin:0.5rem 0"></p>`;
				i++;
				continue;
			}

			// Horizontal rule
			if (/^-{3,}$/.test(line.trim()) || /^\*{3,}$/.test(line.trim())) {
				if (inList) {
					html += inList === 'ul' ? '</ul>' : '</ol>';
					inList = null;
				}
				html += '<hr>';
				i++;
				continue;
			}

			// Headings
			const headingMatch = line.match(/^(#{1,6})\s+(.*)$/);
			if (headingMatch) {
				if (inList) {
					html += inList === 'ul' ? '</ul>' : '</ol>';
					inList = null;
				}
				const level = headingMatch[1].length;
				html += `<h${level}>${inlineMarkdown(headingMatch[2])}</h${level}>`;
				i++;
				continue;
			}

			// Blockquote
			if (line.startsWith('> ')) {
				if (inList) {
					html += inList === 'ul' ? '</ul>' : '</ol>';
					inList = null;
				}
				let quoteContent = line.slice(2);
				while (i + 1 < lines.length && lines[i + 1].startsWith('> ')) {
					i++;
					quoteContent += '\n' + lines[i].slice(2);
				}
				html += `<blockquote><p>${inlineMarkdown(quoteContent)}</p></blockquote>`;
				i++;
				continue;
			}

			// Task lists
			const taskMatch = line.match(/^[-*]\s+\[([ xX])\]\s+(.*)$/);
			if (taskMatch) {
				if (inList !== 'ul') {
					if (inList) html += '</ol>';
					html += '<ul data-type="taskList">';
					inList = 'ul';
				}
				const checked = taskMatch[1] !== ' ';
				html += `<li data-type="taskItem" data-checked="${checked}"><label><input type="checkbox" ${checked ? 'checked' : ''}><span>${inlineMarkdown(taskMatch[2])}</span></label></li>`;
				i++;
				continue;
			}

			// Unordered list
			const ulMatch = line.match(/^[-*]\s+(.*)$/);
			if (ulMatch) {
				if (inList !== 'ul') {
					if (inList) html += '</ol>';
					html += '<ul>';
					inList = 'ul';
				}
				html += `<li>${inlineMarkdown(ulMatch[1])}</li>`;
				i++;
				continue;
			}

			// Ordered list
			const olMatch = line.match(/^\d+\.\s+(.*)$/);
			if (olMatch) {
				if (inList !== 'ol') {
					if (inList) html += '</ul>';
					html += '<ol>';
					inList = 'ol';
				}
				html += `<li>${inlineMarkdown(olMatch[1])}</li>`;
				i++;
				continue;
			}

			// Paragraph
			if (inList) {
				html += inList === 'ul' ? '</ul>' : '</ol>';
				inList = null;
			}
			html += `<p>${inlineMarkdown(line)}</p>`;
			i++;
		}

		if (inCodeBlock) {
			html += `<pre><code>${escapeHtml(codeBlockContent)}</code></pre>`;
		}
		if (inList) {
			html += inList === 'ul' ? '</ul>' : '</ol>';
		}

		return html || '<p><br></p>';
	}

	function inlineMarkdown(text: string): string {
		let result = escapeHtml(text);
		result = result.replace(/\*\*\*(.+?)\*\*\*/g, '<strong><em>$1</em></strong>');
		result = result.replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>');
		result = result.replace(/\*(.+?)\*/g, '<em>$1</em>');
		result = result.replace(/~~(.+?)~~/g, '<s>$1</s>');
		result = result.replace(/`([^`]+)`/g, '<code>$1</code>');
		result = result.replace(
			/\[\[([^\]|]+?)(?:\|([^\]]+?))?\]\]/g,
			(_, target, alias) =>
				`<a href="#" class="mv-wikilink" data-target="${target}">${alias || target}</a>`
		);
		result = result.replace(
			/\[([^\]]+)\]\(([^)]+)\)/g,
			'<a href="$2" target="_blank" rel="noopener">$1</a>'
		);
		return result;
	}

	function escapeHtml(s: string): string {
		return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
	}

	function htmlToMarkdown(html: string): string {
		const tmp = document.createElement('div');
		tmp.innerHTML = html;
		return nodeToMarkdown(tmp).trim();
	}

	function nodeToMarkdown(node: Node): string {
		if (node.nodeType === Node.TEXT_NODE) {
			return node.textContent ?? '';
		}
		if (node.nodeType !== Node.ELEMENT_NODE) return '';

		const el = node as HTMLElement;
		const tag = el.tagName.toLowerCase();
		const children = () => Array.from(el.childNodes).map(nodeToMarkdown).join('');

		switch (tag) {
			case 'h1': return `# ${children()}\n\n`;
			case 'h2': return `## ${children()}\n\n`;
			case 'h3': return `### ${children()}\n\n`;
			case 'h4': return `#### ${children()}\n\n`;
			case 'h5': return `##### ${children()}\n\n`;
			case 'h6': return `###### ${children()}\n\n`;
			case 'p': return `${children()}\n\n`;
			case 'br': return '\n';
			case 'strong':
			case 'b': return `**${children()}**`;
			case 'em':
			case 'i': return `*${children()}*`;
			case 's':
			case 'del':
			case 'strike': return `~~${children()}~~`;
			case 'code':
				if (el.parentElement?.tagName.toLowerCase() === 'pre') {
					return children();
				}
				return `\`${children()}\``;
			case 'pre': {
				const codeEl = el.querySelector('code');
				const lang = codeEl?.getAttribute('class')?.match(/language-(\S+)/)?.[1] ?? '';
				const content = codeEl ? codeEl.textContent ?? '' : el.textContent ?? '';
				return `\`\`\`${lang}\n${content}\n\`\`\`\n\n`;
			}
			case 'a': {
				const href = el.getAttribute('href') ?? '';
				const wikiTarget = el.getAttribute('data-target');
				if (wikiTarget) {
					const text = children();
					return text !== wikiTarget ? `[[${wikiTarget}|${text}]]` : `[[${wikiTarget}]]`;
				}
				return `[${children()}](${href})`;
			}
			case 'blockquote':
				return children().trim().split('\n').map((l: string) => `> ${l}`).join('\n') + '\n\n';
			case 'ul': {
				const isTaskList = el.getAttribute('data-type') === 'taskList';
				return Array.from(el.children)
					.map((li) => {
						if (isTaskList || li.getAttribute('data-type') === 'taskItem') {
							const checked = li.getAttribute('data-checked') === 'true';
							const input = li.querySelector('input');
							const isChecked = checked || input?.checked;
							const text = li.querySelector('span')?.textContent ?? li.textContent?.replace(/^\s*☐?\s*/, '') ?? '';
							return `- [${isChecked ? 'x' : ' '}] ${text}`;
						}
						return `- ${nodeToMarkdown(li).trim()}`;
					})
					.join('\n') + '\n\n';
			}
			case 'ol':
				return Array.from(el.children)
					.map((li, i) => `${i + 1}. ${nodeToMarkdown(li).trim()}`)
					.join('\n') + '\n\n';
			case 'li': return children();
			case 'table': {
				const rows = Array.from(el.querySelectorAll('tr'));
				if (rows.length === 0) return '';
				const headerRow = rows[0];
				const headerCells = Array.from(headerRow.children).map((c) => (c.textContent ?? '').trim());
				let md = '| ' + headerCells.join(' | ') + ' |\n';
				md += '| ' + headerCells.map(() => '---').join(' | ') + ' |\n';
				for (let r = 1; r < rows.length; r++) {
					const cells = Array.from(rows[r].children).map((c) => (c.textContent ?? '').trim());
					md += '| ' + cells.join(' | ') + ' |\n';
				}
				return md + '\n';
			}
			case 'thead':
			case 'tbody':
			case 'tfoot': return children();
			case 'tr':
			case 'th':
			case 'td': return '';
			case 'img': {
				const src = el.getAttribute('src') ?? '';
				const alt = el.getAttribute('alt') ?? '';
				return `![${alt}](${src})`;
			}
			case 'hr': return '---\n\n';
			case 'div': return `${children()}\n`;
			case 'input': return '';
			case 'label': return children();
			default: return children();
		}
	}

	// --- Editor Commands ---

	function exec(command: string, value?: string) {
		document.execCommand(command, false, value);
		contentComponent?.getElement()?.focus();
		updateToolbarState();
		scheduleSnapshot();
		emitChange();
	}

	function toggleBold() { exec('bold'); }
	function toggleItalic() { exec('italic'); }
	function toggleStrikethrough() { exec('strikethrough'); }
	function toggleUnorderedList() { exec('insertUnorderedList'); }
	function toggleOrderedList() { exec('insertOrderedList'); }

	function setHeading(level: number) {
		if (level === 0) {
			exec('formatBlock', 'p');
		} else {
			exec('formatBlock', `h${level}`);
		}
	}

	function toggleBlockquote() {
		exec('formatBlock', 'blockquote');
	}

	function toggleCodeBlock() {
		const sel = window.getSelection();
		if (!sel || sel.rangeCount === 0) return;
		const range = sel.getRangeAt(0);
		const parent = range.startContainer.parentElement;

		if (parent?.closest('pre')) {
			const pre = parent.closest('pre')!;
			const p = document.createElement('p');
			p.textContent = pre.textContent ?? '';
			pre.replaceWith(p);
		} else {
			const text = sel.toString() || 'code here';
			const pre = document.createElement('pre');
			const code = document.createElement('code');
			code.textContent = text;
			pre.appendChild(code);
			range.deleteContents();
			range.insertNode(pre);
		}
		updateToolbarState();
		scheduleSnapshot();
		emitChange();
	}

	function insertLink() {
		const sel = window.getSelection();
		if (!sel || sel.rangeCount === 0) return;
		savedSelection = sel.getRangeAt(0).cloneRange();
		linkText = sel.toString();
		linkUrl = '';
		showLinkModal = true;
	}

	function handleLinkSubmit(event: CustomEvent<{ url: string; text: string }>) {
		const { url, text } = event.detail;
		if (!savedSelection || !url) return;
		const sel = window.getSelection();
		if (sel) {
			sel.removeAllRanges();
			sel.addRange(savedSelection);
		}
		if (text && sel?.toString() !== text) {
			document.execCommand('insertText', false, text);
		}
		document.execCommand('createLink', false, url);
		showLinkModal = false;
		savedSelection = null;
		contentComponent?.getElement()?.focus();
		scheduleSnapshot();
		emitChange();
	}

	function removeFormatting() {
		exec('removeFormat');
	}

	function insertHorizontalRule() {
		exec('insertHorizontalRule');
	}

	function insertTable(rows: number, cols: number) {
		const sel = window.getSelection();
		if (!sel || sel.rangeCount === 0) return;
		const range = sel.getRangeAt(0);
		const table = document.createElement('table');
		table.setAttribute('data-mv-table', 'true');
		for (let r = 0; r < rows; r++) {
			const tr = document.createElement('tr');
			for (let c = 0; c < cols; c++) {
				const cell = document.createElement(r === 0 ? 'th' : 'td');
				cell.textContent = r === 0 ? `Header ${c + 1}` : '';
				cell.setAttribute('contenteditable', 'true');
				tr.appendChild(cell);
			}
			table.appendChild(tr);
		}
		range.deleteContents();
		range.insertNode(table);
		const firstTd = table.querySelector('td');
		if (firstTd) {
			const newRange = document.createRange();
			newRange.setStart(firstTd, 0);
			newRange.collapse(true);
			sel.removeAllRanges();
			sel.addRange(newRange);
		}
		scheduleSnapshot();
		emitChange();
	}

	function insertImageFromUrl() {
		showImageModal = true;
		imageUrl = '';
		imageAlt = '';
	}

	function handleImageSubmit(event: CustomEvent<{ url: string; alt: string }>) {
		const { url, alt } = event.detail;
		if (!url) return;
		const sel = window.getSelection();
		if (!sel || sel.rangeCount === 0) { showImageModal = false; return; }
		const range = sel.getRangeAt(0);
		const img = document.createElement('img');
		img.src = url;
		img.alt = alt || '';
		img.style.maxWidth = '100%';
		img.style.borderRadius = '8px';
		img.style.margin = '0.5rem 0';
		range.deleteContents();
		range.insertNode(img);
		showImageModal = false;
		contentComponent?.getElement()?.focus();
		scheduleSnapshot();
		emitChange();
	}

	function insertTaskList() {
		const sel = window.getSelection();
		if (!sel || sel.rangeCount === 0) return;
		const range = sel.getRangeAt(0);
		const ul = document.createElement('ul');
		ul.setAttribute('data-type', 'taskList');
		const li = document.createElement('li');
		li.setAttribute('data-type', 'taskItem');
		li.setAttribute('data-checked', 'false');
		const label = document.createElement('label');
		const checkbox = document.createElement('input');
		checkbox.type = 'checkbox';
		checkbox.addEventListener('change', () => {
			li.setAttribute('data-checked', String(checkbox.checked));
			scheduleSnapshot();
			emitChange();
		});
		const span = document.createElement('span');
		span.textContent = sel.toString() || 'Task item';
		label.appendChild(checkbox);
		label.appendChild(span);
		li.appendChild(label);
		ul.appendChild(li);
		range.deleteContents();
		range.insertNode(ul);
		contentComponent?.getElement()?.focus();
		scheduleSnapshot();
		emitChange();
	}

	// --- Toolbar State ---

	function updateToolbarState() {
		toolbarState = {
			bold: document.queryCommandState('bold'),
			italic: document.queryCommandState('italic'),
			strikethrough: document.queryCommandState('strikethrough'),
			orderedList: document.queryCommandState('insertOrderedList'),
			unorderedList: document.queryCommandState('insertUnorderedList'),
			heading: getHeadingLevel(),
			blockquote: isInsideTag('blockquote'),
			code: isInsideTag('pre')
		};
	}

	function getHeadingLevel(): number {
		const block = document.queryCommandValue('formatBlock');
		const match = block.match(/^h(\d)$/i);
		return match ? parseInt(match[1]) : 0;
	}

	function isInsideTag(tag: string): boolean {
		const sel = window.getSelection();
		if (!sel || sel.rangeCount === 0) return false;
		const editorEl = contentComponent?.getElement();
		let node: Node | null = sel.anchorNode;
		while (node && node !== editorEl) {
			if (node.nodeType === Node.ELEMENT_NODE && (node as HTMLElement).tagName.toLowerCase() === tag) {
				return true;
			}
			node = node.parentNode;
		}
		return false;
	}

	// --- History ---

	function pushSnapshot() {
		const editorEl = contentComponent?.getElement();
		if (!editorEl) return;
		const html = editorEl.innerHTML;
		if (undoStack.length > 0 && undoStack[undoStack.length - 1] === html) return;
		undoStack = [...undoStack.slice(-49), html];
		redoStack = [];
	}

	function scheduleSnapshot() {
		if (snapshotTimer) clearTimeout(snapshotTimer);
		snapshotTimer = setTimeout(() => pushSnapshot(), 500);
	}

	function undo() {
		if (undoStack.length <= 1) return;
		const editorEl = contentComponent?.getElement();
		if (!editorEl) return;
		redoStack = [...redoStack, undoStack[undoStack.length - 1]];
		undoStack = undoStack.slice(0, -1);
		editorEl.innerHTML = undoStack[undoStack.length - 1];
		emitChange();
	}

	function redo() {
		if (redoStack.length === 0) return;
		const editorEl = contentComponent?.getElement();
		if (!editorEl) return;
		const html = redoStack[redoStack.length - 1];
		redoStack = redoStack.slice(0, -1);
		undoStack = [...undoStack, html];
		editorEl.innerHTML = html;
		emitChange();
	}

	// --- Mode Switching ---

	function setMode(m: EditorMode) {
		if (m === mode) return;
		const editorEl = contentComponent?.getElement();
		if (m === 'source') {
			sourceEl.value = htmlToMarkdown(editorEl?.innerHTML ?? '');
		} else {
			if (editorEl) {
				editorEl.innerHTML = markdownToHtml(sourceEl.value);
			}
			pushSnapshot();
		}
		mode = m;
	}

	// --- Event Handlers ---

	function handleInput() {
		isEmpty = isEditorEmpty();
		updateToolbarState();
		updateWordCount();
		scheduleSnapshot();
		emitChange();
	}

	function handleKeyDown(event: CustomEvent<KeyboardEvent>) {
		const e = event.detail;
		const mod = e.metaKey || e.ctrlKey;

		// Let plugins handle the event first
		if (pluginRegistry.handleKeyDown(e)) {
			e.preventDefault();
			return;
		}

		if (mod && e.key === 'b') {
			e.preventDefault();
			toggleBold();
			return;
		}
		if (mod && e.key === 'i') {
			e.preventDefault();
			toggleItalic();
			return;
		}
		if (mod && e.key === 'k') {
			e.preventDefault();
			insertLink();
			return;
		}
		if (mod && e.key === 's') {
			e.preventDefault();
			dispatch('save', { markdown: getMarkdown() });
			return;
		}
		if (mod && e.key === 'z') {
			e.preventDefault();
			if (e.shiftKey) {
				redo();
			} else {
				undo();
			}
			return;
		}
		if (e.key === 'Tab' && isInsideTag('pre')) {
			e.preventDefault();
			document.execCommand('insertText', false, '  ');
			return;
		}
	}

	function handlePaste(event: CustomEvent<ClipboardEvent>) {
		const e = event.detail;

		// Let plugins handle the event first
		if (pluginRegistry.handlePaste(e)) {
			return;
		}

		const items = e.clipboardData?.items;
		if (!items) return;
		for (const item of items) {
			if (item.type.startsWith('image/')) {
				e.preventDefault();
				const file = item.getAsFile();
				if (!file) return;
				const reader = new FileReader();
				reader.onload = () => {
					const dataUrl = reader.result as string;
					document.execCommand('insertImage', false, dataUrl);
					scheduleSnapshot();
					emitChange();
				};
				reader.readAsDataURL(file);
				return;
			}
		}
	}

	function handleSourceInput() {
		emitChange();
	}

	function handleSourceKeyDown(event: KeyboardEvent) {
		const mod = event.metaKey || event.ctrlKey;
		if (mod && event.key === 's') {
			event.preventDefault();
			dispatch('save', { markdown: sourceEl.value });
			return;
		}
		if (event.key === 'Tab') {
			event.preventDefault();
			const start = sourceEl.selectionStart;
			const end = sourceEl.selectionEnd;
			sourceEl.value = sourceEl.value.substring(0, start) + '  ' + sourceEl.value.substring(end);
			sourceEl.selectionStart = sourceEl.selectionEnd = start + 2;
		}
	}

	// --- Helpers ---

	function isEditorEmpty(): boolean {
		const editorEl = contentComponent?.getElement();
		if (!editorEl) return true;
		const text = editorEl.textContent?.trim() ?? '';
		return text === '' || editorEl.innerHTML === '<p><br></p>';
	}

	function updateWordCount() {
		const editorEl = contentComponent?.getElement();
		const text = editorEl?.textContent ?? '';
		const words = text.trim().split(/\s+/).filter(Boolean);
		wordCount = words.length;
	}

	function emitChange() {
		if (mode === 'source') {
			dispatch('change', { markdown: sourceEl.value, html: markdownToHtml(sourceEl.value) });
		} else {
			const editorEl = contentComponent?.getElement();
			const md = htmlToMarkdown(editorEl?.innerHTML ?? '');
			dispatch('change', { markdown: md, html: editorEl?.innerHTML ?? '' });
		}
	}

	// --- Public API ---

	export function getMarkdown(): string {
		if (mode === 'source') return sourceEl.value;
		return htmlToMarkdown(contentComponent?.getElement()?.innerHTML ?? '');
	}

	export function setMarkdown(md: string) {
		value = md;
		if (mode === 'source') {
			sourceEl.value = md;
		} else {
			contentComponent?.setContent(markdownToHtml(md));
			pushSnapshot();
		}
		isEmpty = isEditorEmpty();
	}

	export function focus() {
		if (mode === 'source') {
			sourceEl.focus();
		} else {
			contentComponent?.focus();
		}
	}

	/**
	 * Register a plugin dynamically.
	 */
	export function registerPlugin(plugin: EditorPlugin) {
		pluginRegistry.register(plugin);
	}

	/**
	 * Unregister a plugin by ID.
	 */
	export function unregisterPlugin(pluginId: string) {
		pluginRegistry.unregister(pluginId);
	}

	/**
	 * Execute a plugin command by name.
	 */
	export function executeCommand(commandName: string, ...args: unknown[]) {
		return pluginRegistry.executeCommand(commandName, ...args);
	}

	/**
	 * Get the plugin registry for advanced use.
	 */
	export function getPluginRegistry() {
		return pluginRegistry;
	}
</script>

<div class="mv-editor rounded-xl border border-slate-800 bg-slate-950/80">
	<EditorToolbar
		{mode}
		{readonly}
		state={toolbarState}
		canUndo={undoStack.length > 1}
		canRedo={redoStack.length > 0}
		on:setMode={(e) => setMode(e.detail)}
		on:toggleBold={toggleBold}
		on:toggleItalic={toggleItalic}
		on:toggleStrikethrough={toggleStrikethrough}
		on:toggleUnorderedList={toggleUnorderedList}
		on:toggleOrderedList={toggleOrderedList}
		on:insertTaskList={insertTaskList}
		on:setHeading={(e) => setHeading(e.detail)}
		on:toggleBlockquote={toggleBlockquote}
		on:toggleCodeBlock={toggleCodeBlock}
		on:insertHorizontalRule={insertHorizontalRule}
		on:insertLink={insertLink}
		on:removeFormatting={removeFormatting}
		on:undo={undo}
		on:redo={redo}
	/>

	{#if mode === 'rich'}
		<EditorContent
			bind:this={contentComponent}
			{placeholder}
			{readonly}
			{isEmpty}
			slashItems={allSlashItems}
			content={value ? markdownToHtml(value) : ''}
			on:input={handleInput}
			on:keydown={handleKeyDown}
			on:paste={handlePaste}
			on:selectionchange={updateToolbarState}
		/>
	{:else}
		<textarea
			bind:this={sourceEl}
			class="min-h-[200px] w-full resize-y bg-transparent px-4 py-3 font-mono text-sm text-slate-200 outline-none"
			readonly={readonly}
			placeholder={placeholder}
			value={value}
			on:input={handleSourceInput}
			on:keydown={handleSourceKeyDown}
		></textarea>
	{/if}

	<EditorFooter {wordCount} {mode} {readonly} />
</div>

{#if showLinkModal}
	<LinkModal
		url={linkUrl}
		text={linkText}
		on:submit={handleLinkSubmit}
		on:cancel={() => { showLinkModal = false; savedSelection = null; contentComponent?.focus(); }}
	/>
{/if}

{#if showImageModal}
	<ImageModal
		url={imageUrl}
		alt={imageAlt}
		on:submit={handleImageSubmit}
		on:cancel={() => { showImageModal = false; contentComponent?.focus(); }}
	/>
{/if}
