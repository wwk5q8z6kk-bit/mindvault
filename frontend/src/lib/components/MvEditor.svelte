<script lang="ts">
	import { createEventDispatcher, onMount, onDestroy, tick } from 'svelte';
	import hljs from 'highlight.js/lib/core';
	import javascript from 'highlight.js/lib/languages/javascript';
	import typescript from 'highlight.js/lib/languages/typescript';
	import python from 'highlight.js/lib/languages/python';
	import rust from 'highlight.js/lib/languages/rust';
	import css from 'highlight.js/lib/languages/css';
	import xml from 'highlight.js/lib/languages/xml';
	import bash from 'highlight.js/lib/languages/bash';
	import json from 'highlight.js/lib/languages/json';
	import sql from 'highlight.js/lib/languages/sql';
	import go from 'highlight.js/lib/languages/go';

	hljs.registerLanguage('javascript', javascript);
	hljs.registerLanguage('js', javascript);
	hljs.registerLanguage('typescript', typescript);
	hljs.registerLanguage('ts', typescript);
	hljs.registerLanguage('python', python);
	hljs.registerLanguage('py', python);
	hljs.registerLanguage('rust', rust);
	hljs.registerLanguage('rs', rust);
	hljs.registerLanguage('css', css);
	hljs.registerLanguage('html', xml);
	hljs.registerLanguage('xml', xml);
	hljs.registerLanguage('bash', bash);
	hljs.registerLanguage('sh', bash);
	hljs.registerLanguage('json', json);
	hljs.registerLanguage('sql', sql);
	hljs.registerLanguage('go', go);

	/** Initial content in markdown */
	export let value = '';
	/** Placeholder text */
	export let placeholder = 'Start writing...';
	/** Whether the editor is read-only */
	export let readonly = false;

	const dispatch = createEventDispatcher<{
		change: { markdown: string; html: string };
		save: { markdown: string };
	}>();

	type EditorMode = 'rich' | 'source';
	let mode: EditorMode = 'rich';
	let editorEl: HTMLDivElement;
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
	let linkUrl = '';
	let linkText = '';
	let linkUrlInput: HTMLInputElement | null = null;
	let savedSelection: Range | null = null;
	let isEmpty = true;
	let wordCount = 0;

	// --- Slash command state ---
	interface SlashItem {
		id: string;
		label: string;
		description: string;
		action: () => void;
	}
	let showSlashMenu = false;
	let slashMenuItems: SlashItem[] = [];
	let slashActiveIndex = 0;
	let slashQuery = '';
	let slashMenuX = 0;
	let slashMenuY = 0;
	let slashRange: Range | null = null;

	const allSlashItems: SlashItem[] = [
		{ id: 'h1', label: 'Heading 1', description: 'Large section heading', action: () => setHeading(1) },
		{ id: 'h2', label: 'Heading 2', description: 'Medium section heading', action: () => setHeading(2) },
		{ id: 'h3', label: 'Heading 3', description: 'Small section heading', action: () => setHeading(3) },
		{ id: 'bullet', label: 'Bullet List', description: 'Unordered list', action: () => toggleUnorderedList() },
		{ id: 'numbered', label: 'Numbered List', description: 'Ordered list', action: () => toggleOrderedList() },
		{ id: 'task', label: 'Task List', description: 'Checklist items', action: () => insertTaskList() },
		{ id: 'quote', label: 'Blockquote', description: 'Indented quote block', action: () => toggleBlockquote() },
		{ id: 'code', label: 'Code Block', description: 'Syntax-highlighted code', action: () => toggleCodeBlock() },
		{ id: 'divider', label: 'Divider', description: 'Horizontal rule', action: () => insertHorizontalRule() },
		{ id: 'table', label: 'Table', description: 'Insert 3x3 table', action: () => insertTable(3, 3) },
		{ id: 'link', label: 'Link', description: 'Insert hyperlink', action: () => insertLink() },
		{ id: 'image', label: 'Image', description: 'Insert image from URL', action: () => insertImageFromUrl() },
	];

	onMount(() => {
		if (value) {
			editorEl.innerHTML = markdownToHtml(value);
		}
		isEmpty = isEditorEmpty();
		pushSnapshot();
		highlightCodeBlocks();
	});

	onDestroy(() => {
		if (snapshotTimer) clearTimeout(snapshotTimer);
	});

	// --- Markdown → HTML ---
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

			// Markdown table (detect pipe-delimited rows)
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

			// Image: ![alt](url)
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
		// Bold + italic
		result = result.replace(/\*\*\*(.+?)\*\*\*/g, '<strong><em>$1</em></strong>');
		// Bold
		result = result.replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>');
		// Italic
		result = result.replace(/\*(.+?)\*/g, '<em>$1</em>');
		// Strikethrough
		result = result.replace(/~~(.+?)~~/g, '<s>$1</s>');
		// Inline code
		result = result.replace(/`([^`]+)`/g, '<code>$1</code>');
		// Wiki-links [[target|alias]] or [[target]]
		result = result.replace(
			/\[\[([^\]|]+?)(?:\|([^\]]+?))?\]\]/g,
			(_, target, alias) =>
				`<a href="#" class="mv-wikilink" data-target="${target}">${alias || target}</a>`
		);
		// Links [text](url)
		result = result.replace(
			/\[([^\]]+)\]\(([^)]+)\)/g,
			'<a href="$2" target="_blank" rel="noopener">$1</a>'
		);
		return result;
	}

	function escapeHtml(s: string): string {
		return s
			.replace(/&/g, '&amp;')
			.replace(/</g, '&lt;')
			.replace(/>/g, '&gt;')
			.replace(/"/g, '&quot;');
	}

	// --- HTML → Markdown ---
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
			case 'h1':
				return `# ${children()}\n\n`;
			case 'h2':
				return `## ${children()}\n\n`;
			case 'h3':
				return `### ${children()}\n\n`;
			case 'h4':
				return `#### ${children()}\n\n`;
			case 'h5':
				return `##### ${children()}\n\n`;
			case 'h6':
				return `###### ${children()}\n\n`;
			case 'p':
				return `${children()}\n\n`;
			case 'br':
				return '\n';
			case 'strong':
			case 'b':
				return `**${children()}**`;
			case 'em':
			case 'i':
				return `*${children()}*`;
			case 's':
			case 'del':
			case 'strike':
				return `~~${children()}~~`;
			case 'code':
				if (el.parentElement?.tagName.toLowerCase() === 'pre') {
					return children();
				}
				return `\`${children()}\``;
			case 'pre': {
				const codeEl = el.querySelector('code');
				const lang =
					codeEl
						?.getAttribute('class')
						?.match(/language-(\S+)/)?.[1] ?? '';
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
				return (
					children()
						.trim()
						.split('\n')
						.map((l: string) => `> ${l}`)
						.join('\n') + '\n\n'
				);
			case 'ul': {
				const isTaskList = el.getAttribute('data-type') === 'taskList';
				return (
					Array.from(el.children)
						.map((li) => {
							if (isTaskList || li.getAttribute('data-type') === 'taskItem') {
								const checked = li.getAttribute('data-checked') === 'true';
								const input = li.querySelector('input');
								const isChecked = checked || input?.checked;
								const text =
									li.querySelector('span')?.textContent ??
									li.textContent?.replace(/^\s*☐?\s*/, '') ??
									'';
								return `- [${isChecked ? 'x' : ' '}] ${text}`;
							}
							return `- ${nodeToMarkdown(li).trim()}`;
						})
						.join('\n') + '\n\n'
				);
			}
			case 'ol':
				return (
					Array.from(el.children)
						.map((li, i) => `${i + 1}. ${nodeToMarkdown(li).trim()}`)
						.join('\n') + '\n\n'
				);
			case 'li':
				return children();
			case 'table': {
				const rows = Array.from(el.querySelectorAll('tr'));
				if (rows.length === 0) return '';
				const headerRow = rows[0];
				const headerCells = Array.from(headerRow.children).map(
					(c) => (c.textContent ?? '').trim()
				);
				let md = '| ' + headerCells.join(' | ') + ' |\n';
				md += '| ' + headerCells.map(() => '---').join(' | ') + ' |\n';
				for (let r = 1; r < rows.length; r++) {
					const cells = Array.from(rows[r].children).map(
						(c) => (c.textContent ?? '').trim()
					);
					md += '| ' + cells.join(' | ') + ' |\n';
				}
				return md + '\n';
			}
			case 'thead':
			case 'tbody':
			case 'tfoot':
				return children();
			case 'tr':
			case 'th':
			case 'td':
				return '';
			case 'img': {
				const src = el.getAttribute('src') ?? '';
				const alt = el.getAttribute('alt') ?? '';
				return `![${alt}](${src})`;
			}
			case 'hr':
				return '---\n\n';
			case 'div':
				return `${children()}\n`;
			case 'input':
				return '';
			case 'label':
				return children();
			default:
				return children();
		}
	}

	// --- Editor commands ---
	function exec(command: string, value?: string) {
		document.execCommand(command, false, value);
		editorEl.focus();
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
			// Exit code block: unwrap to paragraph
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
			tick().then(() => linkUrlInput?.focus());
	}

	function applyLink() {
		if (!savedSelection || !linkUrl) return;
		const sel = window.getSelection();
		if (sel) {
			sel.removeAllRanges();
			sel.addRange(savedSelection);
		}
		if (linkText && sel?.toString() !== linkText) {
			document.execCommand('insertText', false, linkText);
		}
		document.execCommand('createLink', false, linkUrl);
		showLinkModal = false;
		savedSelection = null;
		editorEl.focus();
		scheduleSnapshot();
		emitChange();
	}

	function cancelLink() {
		showLinkModal = false;
		savedSelection = null;
		editorEl.focus();
	}

	function removeFormatting() {
		exec('removeFormat');
	}

	function insertHorizontalRule() {
		exec('insertHorizontalRule');
	}

	// --- Table insertion ---
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
		// Place cursor in first data cell
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

	// --- Image insertion ---
	let showImageModal = false;
	let imageUrlInput: HTMLInputElement | null = null;
	let imageUrl = '';
	let imageAlt = '';

	function insertImageFromUrl() {
		showImageModal = true;
			tick().then(() => imageUrlInput?.focus());
		imageUrl = '';
		imageAlt = '';
	}

	function applyImage() {
		if (!imageUrl) return;
		const sel = window.getSelection();
		if (!sel || sel.rangeCount === 0) { showImageModal = false; return; }
		const range = sel.getRangeAt(0);
		const img = document.createElement('img');
		img.src = imageUrl;
		img.alt = imageAlt || '';
		img.style.maxWidth = '100%';
		img.style.borderRadius = '8px';
		img.style.margin = '0.5rem 0';
		range.deleteContents();
		range.insertNode(img);
		showImageModal = false;
		editorEl.focus();
		scheduleSnapshot();
		emitChange();
	}

	function cancelImage() {
		showImageModal = false;
		editorEl.focus();
	}

	// --- Image paste handling ---
	function onPaste(event: ClipboardEvent) {
		const items = event.clipboardData?.items;
		if (!items) return;
		for (const item of items) {
			if (item.type.startsWith('image/')) {
				event.preventDefault();
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

	// --- Slash command logic ---
	function openSlashMenu() {
		const sel = window.getSelection();
		if (!sel || sel.rangeCount === 0) return;
		const range = sel.getRangeAt(0);
		slashRange = range.cloneRange();
		const rect = range.getBoundingClientRect();
		const editorRect = editorEl.getBoundingClientRect();
		slashMenuX = rect.left - editorRect.left;
		slashMenuY = rect.bottom - editorRect.top + 4;
		slashQuery = '';
		slashMenuItems = allSlashItems;
		slashActiveIndex = 0;
		showSlashMenu = true;
	}

	function filterSlashMenu(query: string) {
		slashQuery = query;
		const q = query.toLowerCase();
		slashMenuItems = allSlashItems.filter(
			(item) => item.label.toLowerCase().includes(q) || item.description.toLowerCase().includes(q)
		);
		slashActiveIndex = 0;
	}

	function executeSlashItem(item: SlashItem) {
		// Remove the "/" and query text from the editor
		if (slashRange) {
			const sel = window.getSelection();
			if (sel) {
				const range = slashRange.cloneRange();
				// Expand range backwards to capture "/" + query
				const startOffset = Math.max(0, range.startOffset - 1 - slashQuery.length);
				range.setStart(range.startContainer, startOffset);
				sel.removeAllRanges();
				sel.addRange(range);
				document.execCommand('delete');
			}
		}
		showSlashMenu = false;
		slashRange = null;
		item.action();
	}

	function closeSlashMenu() {
		showSlashMenu = false;
		slashRange = null;
	}

	// --- Word count ---
	function updateWordCount() {
		const text = editorEl?.textContent ?? '';
		const words = text.trim().split(/\s+/).filter(Boolean);
		wordCount = words.length;
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
		editorEl.focus();
		scheduleSnapshot();
		emitChange();
	}

	// --- Syntax highlighting ---
	function highlightCodeBlocks() {
		if (!editorEl) return;
		const blocks = editorEl.querySelectorAll('pre > code');
		blocks.forEach((block) => {
			if (block.getAttribute('data-highlighted') === 'yes') return;
			hljs.highlightElement(block as HTMLElement);
		});
	}

	// --- Table context menu ---
	let showTableMenu = false;
	let tableMenuX = 0;
	let tableMenuY = 0;
	let contextTable: HTMLTableElement | null = null;
	let contextRow: HTMLTableRowElement | null = null;
	let contextCell: HTMLTableCellElement | null = null;

	function onContextMenu(event: MouseEvent) {
		const target = event.target as HTMLElement;
		const cell = target.closest('th, td') as HTMLTableCellElement | null;
		const table = target.closest('table') as HTMLTableElement | null;
		const row = target.closest('tr') as HTMLTableRowElement | null;
		if (!cell || !table || !row) {
			showTableMenu = false;
			return;
		}
		event.preventDefault();
		contextTable = table;
		contextRow = row;
		contextCell = cell;
		const editorRect = editorEl.getBoundingClientRect();
		tableMenuX = event.clientX - editorRect.left;
		tableMenuY = event.clientY - editorRect.top;
		showTableMenu = true;
	}

	function closeTableMenu() {
		showTableMenu = false;
	}

	function addRowAbove() {
		if (!contextRow || !contextTable) return;
		const cols = contextRow.children.length;
		const newRow = document.createElement('tr');
		for (let i = 0; i < cols; i++) {
			const td = document.createElement('td');
			td.textContent = '';
			td.setAttribute('contenteditable', 'true');
			newRow.appendChild(td);
		}
		contextRow.parentNode?.insertBefore(newRow, contextRow);
		closeTableMenu();
		scheduleSnapshot();
		emitChange();
	}

	function addRowBelow() {
		if (!contextRow || !contextTable) return;
		const cols = contextRow.children.length;
		const newRow = document.createElement('tr');
		for (let i = 0; i < cols; i++) {
			const td = document.createElement('td');
			td.textContent = '';
			td.setAttribute('contenteditable', 'true');
			newRow.appendChild(td);
		}
		contextRow.parentNode?.insertBefore(newRow, contextRow.nextSibling);
		closeTableMenu();
		scheduleSnapshot();
		emitChange();
	}

	function addColLeft() {
		if (!contextTable || !contextCell) return;
		const colIdx = Array.from(contextCell.parentElement!.children).indexOf(contextCell);
		const rows = contextTable.querySelectorAll('tr');
		rows.forEach((row, rIdx) => {
			const tag = rIdx === 0 ? 'th' : 'td';
			const cell = document.createElement(tag);
			cell.textContent = '';
			cell.setAttribute('contenteditable', 'true');
			const ref = row.children[colIdx];
			row.insertBefore(cell, ref);
		});
		closeTableMenu();
		scheduleSnapshot();
		emitChange();
	}

	function addColRight() {
		if (!contextTable || !contextCell) return;
		const colIdx = Array.from(contextCell.parentElement!.children).indexOf(contextCell);
		const rows = contextTable.querySelectorAll('tr');
		rows.forEach((row, rIdx) => {
			const tag = rIdx === 0 ? 'th' : 'td';
			const cell = document.createElement(tag);
			cell.textContent = '';
			cell.setAttribute('contenteditable', 'true');
			const ref = row.children[colIdx + 1];
			if (ref) {
				row.insertBefore(cell, ref);
			} else {
				row.appendChild(cell);
			}
		});
		closeTableMenu();
		scheduleSnapshot();
		emitChange();
	}

	function deleteRow() {
		if (!contextRow || !contextTable) return;
		if (contextTable.querySelectorAll('tr').length <= 1) return;
		contextRow.remove();
		closeTableMenu();
		scheduleSnapshot();
		emitChange();
	}

	function deleteCol() {
		if (!contextTable || !contextCell) return;
		const colIdx = Array.from(contextCell.parentElement!.children).indexOf(contextCell);
		const rows = contextTable.querySelectorAll('tr');
		if (rows[0]?.children.length <= 1) return;
		rows.forEach((row) => {
			row.children[colIdx]?.remove();
		});
		closeTableMenu();
		scheduleSnapshot();
		emitChange();
	}

	// --- Tab navigation in tables ---
	function handleTableTab(event: KeyboardEvent) {
		const target = event.target as HTMLElement;
		const cell = target.closest?.('th, td');
		if (!cell) return false;
		const row = cell.parentElement as HTMLTableRowElement;
		const table = row?.closest('table');
		if (!table) return false;

		event.preventDefault();
		const cells = Array.from(table.querySelectorAll('th, td'));
		const currentIdx = cells.indexOf(cell as HTMLTableCellElement);
		const nextIdx = event.shiftKey ? currentIdx - 1 : currentIdx + 1;
		if (nextIdx >= 0 && nextIdx < cells.length) {
			(cells[nextIdx] as HTMLElement).focus();
		}
		return true;
	}

	// --- Toolbar state tracking ---
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
		redoStack = [...redoStack, undoStack[undoStack.length - 1]];
		undoStack = undoStack.slice(0, -1);
		editorEl.innerHTML = undoStack[undoStack.length - 1];
		emitChange();
	}

	function redo() {
		if (redoStack.length === 0) return;
		const html = redoStack[redoStack.length - 1];
		redoStack = redoStack.slice(0, -1);
		undoStack = [...undoStack, html];
		editorEl.innerHTML = html;
		emitChange();
	}

	// --- Mode switching ---
	function setMode(m: EditorMode) {
		if (m === mode) return;
		if (m === 'source') {
			sourceEl.value = htmlToMarkdown(editorEl.innerHTML);
		} else {
			editorEl.innerHTML = markdownToHtml(sourceEl.value);
			pushSnapshot();
		}
		mode = m;
	}

	// --- Events ---
	function onInput() {
		isEmpty = isEditorEmpty();
		updateToolbarState();
		updateWordCount();
		scheduleSnapshot();
		emitChange();
		highlightCodeBlocks();

		// Slash command detection
		if (showSlashMenu) {
			const sel = window.getSelection();
			if (sel && sel.rangeCount > 0) {
				const range = sel.getRangeAt(0);
				const textNode = range.startContainer;
				if (textNode.nodeType === Node.TEXT_NODE) {
					const text = textNode.textContent ?? '';
					const cursorPos = range.startOffset;
					const slashIdx = text.lastIndexOf('/', cursorPos);
					if (slashIdx >= 0) {
						const query = text.substring(slashIdx + 1, cursorPos);
						filterSlashMenu(query);
					} else {
						closeSlashMenu();
					}
				}
			}
		}
	}

	function onSelectionChange() {
		updateToolbarState();
	}

	function onKeyDown(event: KeyboardEvent) {
		const mod = event.metaKey || event.ctrlKey;

		// Slash menu navigation
		if (showSlashMenu) {
			if (event.key === 'ArrowDown') {
				event.preventDefault();
				slashActiveIndex = (slashActiveIndex + 1) % slashMenuItems.length;
				return;
			}
			if (event.key === 'ArrowUp') {
				event.preventDefault();
				slashActiveIndex = (slashActiveIndex - 1 + slashMenuItems.length) % slashMenuItems.length;
				return;
			}
			if (event.key === 'Enter' || event.key === 'Tab') {
				event.preventDefault();
				if (slashMenuItems[slashActiveIndex]) {
					executeSlashItem(slashMenuItems[slashActiveIndex]);
				}
				return;
			}
			if (event.key === 'Escape') {
				event.preventDefault();
				closeSlashMenu();
				return;
			}
		}

		// Open slash menu on "/" at start of line or after space
		if (event.key === '/' && !mod && !showSlashMenu) {
			const sel = window.getSelection();
			if (sel && sel.rangeCount > 0) {
				const range = sel.getRangeAt(0);
				const textNode = range.startContainer;
				const offset = range.startOffset;
				const text = textNode.textContent ?? '';
				const charBefore = offset > 0 ? text[offset - 1] : '';
				if (offset === 0 || charBefore === ' ' || charBefore === '\n') {
					// Delay to let the "/" be inserted first
					setTimeout(() => openSlashMenu(), 10);
				}
			}
		}

		if (mod && event.key === 'b') {
			event.preventDefault();
			toggleBold();
			return;
		}
		if (mod && event.key === 'i') {
			event.preventDefault();
			toggleItalic();
			return;
		}
		if (mod && event.key === 'k') {
			event.preventDefault();
			insertLink();
			return;
		}
		if (mod && event.key === 's') {
			event.preventDefault();
			dispatch('save', { markdown: getMarkdown() });
			return;
		}
		if (mod && event.key === 'z') {
			event.preventDefault();
			if (event.shiftKey) {
				redo();
			} else {
				undo();
			}
			return;
		}
		// Tab for table cell navigation
		if (event.key === 'Tab' && handleTableTab(event)) {
			return;
		}
		// Tab for indentation in code blocks
		if (event.key === 'Tab' && isInsideTag('pre')) {
			event.preventDefault();
			document.execCommand('insertText', false, '  ');
			return;
		}
	}

	function onSourceInput() {
		emitChange();
	}

	function onSourceKeyDown(event: KeyboardEvent) {
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

	function isEditorEmpty(): boolean {
		if (!editorEl) return true;
		const text = editorEl.textContent?.trim() ?? '';
		return text === '' || editorEl.innerHTML === '<p><br></p>';
	}

	function emitChange() {
		if (mode === 'source') {
			dispatch('change', { markdown: sourceEl.value, html: markdownToHtml(sourceEl.value) });
		} else {
			const md = htmlToMarkdown(editorEl.innerHTML);
			dispatch('change', { markdown: md, html: editorEl.innerHTML });
		}
	}

	// --- Public API ---
	export function getMarkdown(): string {
		if (mode === 'source') return sourceEl.value;
		return htmlToMarkdown(editorEl.innerHTML);
	}

	export function setMarkdown(md: string) {
		value = md;
		if (mode === 'source') {
			sourceEl.value = md;
		} else {
			editorEl.innerHTML = markdownToHtml(md);
			pushSnapshot();
			highlightCodeBlocks();
		}
		isEmpty = isEditorEmpty();
	}

	export function focus() {
		if (mode === 'source') {
			sourceEl.focus();
		} else {
			editorEl.focus();
		}
	}
</script>

<svelte:document on:selectionchange={onSelectionChange} on:click={() => { showTableMenu = false; }} />

<div class="mv-editor rounded-xl border border-slate-800 bg-slate-950/80">
	<!-- Toolbar -->
	{#if !readonly}
		<div class="flex flex-wrap items-center gap-1 border-b border-slate-800 px-3 py-2">
			<!-- Mode toggle -->
			<div class="mr-2 flex rounded-lg border border-slate-700 text-[10px]">
				<button
					class={`px-2 py-1 transition ${mode === 'rich' ? 'bg-slate-700 text-white' : 'text-slate-400 hover:text-white'}`}
					on:click={() => setMode('rich')}
				>
					Rich
				</button>
				<button
					class={`px-2 py-1 transition ${mode === 'source' ? 'bg-slate-700 text-white' : 'text-slate-400 hover:text-white'}`}
					on:click={() => setMode('source')}
				>
					Source
				</button>
			</div>

			{#if mode === 'rich'}
				<span class="mx-1 h-4 w-px bg-slate-700"></span>

				<!-- Text formatting -->
				<button
					class={`rounded px-2 py-1 text-xs transition ${toolbarState.bold ? 'bg-sky-500/20 text-sky-300' : 'text-slate-400 hover:bg-slate-800 hover:text-white'}`}
					on:click={toggleBold}
					title="Bold (Ctrl+B)"
				>
					<strong>B</strong>
				</button>
				<button
					class={`rounded px-2 py-1 text-xs transition ${toolbarState.italic ? 'bg-sky-500/20 text-sky-300' : 'text-slate-400 hover:bg-slate-800 hover:text-white'}`}
					on:click={toggleItalic}
					title="Italic (Ctrl+I)"
				>
					<em>I</em>
				</button>
				<button
					class={`rounded px-2 py-1 text-xs transition ${toolbarState.strikethrough ? 'bg-sky-500/20 text-sky-300' : 'text-slate-400 hover:bg-slate-800 hover:text-white'}`}
					on:click={toggleStrikethrough}
					title="Strikethrough"
				>
					<s>S</s>
				</button>

				<span class="mx-1 h-4 w-px bg-slate-700"></span>

				<!-- Headings -->
				<select
					class="rounded border border-slate-700 bg-slate-900 px-2 py-1 text-[10px] text-slate-300"
					value={toolbarState.heading}
					on:change={(e) => setHeading(parseInt(e.currentTarget.value))}
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
					class={`rounded px-2 py-1 text-xs transition ${toolbarState.unorderedList ? 'bg-sky-500/20 text-sky-300' : 'text-slate-400 hover:bg-slate-800 hover:text-white'}`}
					on:click={toggleUnorderedList}
					title="Bullet list"
				>
					&bull; List
				</button>
				<button
					class={`rounded px-2 py-1 text-xs transition ${toolbarState.orderedList ? 'bg-sky-500/20 text-sky-300' : 'text-slate-400 hover:bg-slate-800 hover:text-white'}`}
					on:click={toggleOrderedList}
					title="Numbered list"
				>
					1. List
				</button>
				<button
					class="rounded px-2 py-1 text-xs text-slate-400 transition hover:bg-slate-800 hover:text-white"
					on:click={insertTaskList}
					title="Task list"
				>
					&#9744; Tasks
				</button>

				<span class="mx-1 h-4 w-px bg-slate-700"></span>

				<!-- Block formatting -->
				<button
					class={`rounded px-2 py-1 text-xs transition ${toolbarState.blockquote ? 'bg-sky-500/20 text-sky-300' : 'text-slate-400 hover:bg-slate-800 hover:text-white'}`}
					on:click={toggleBlockquote}
					title="Blockquote"
				>
					&ldquo; Quote
				</button>
				<button
					class={`rounded px-2 py-1 text-xs transition ${toolbarState.code ? 'bg-sky-500/20 text-sky-300' : 'text-slate-400 hover:bg-slate-800 hover:text-white'}`}
					on:click={toggleCodeBlock}
					title="Code block"
				>
					&lt;/&gt;
				</button>
				<button
					class="rounded px-2 py-1 text-xs text-slate-400 transition hover:bg-slate-800 hover:text-white"
					on:click={insertHorizontalRule}
					title="Horizontal rule"
				>
					&mdash;
				</button>

				<span class="mx-1 h-4 w-px bg-slate-700"></span>

				<!-- Link -->
				<button
					class="rounded px-2 py-1 text-xs text-slate-400 transition hover:bg-slate-800 hover:text-white"
					on:click={insertLink}
					title="Insert link (Ctrl+K)"
				>
					Link
				</button>

				<!-- Clear -->
				<button
					class="rounded px-2 py-1 text-xs text-slate-400 transition hover:bg-slate-800 hover:text-white"
					on:click={removeFormatting}
					title="Remove formatting"
				>
					Clear
				</button>

				<span class="mx-1 h-4 w-px bg-slate-700"></span>

				<!-- Undo/Redo -->
				<button
					class="rounded px-2 py-1 text-xs text-slate-400 transition hover:bg-slate-800 hover:text-white disabled:opacity-30"
					on:click={undo}
					disabled={undoStack.length <= 1}
					title="Undo (Ctrl+Z)"
				>
					Undo
				</button>
				<button
					class="rounded px-2 py-1 text-xs text-slate-400 transition hover:bg-slate-800 hover:text-white disabled:opacity-30"
					on:click={redo}
					disabled={redoStack.length === 0}
					title="Redo (Ctrl+Shift+Z)"
				>
					Redo
				</button>
			{/if}
		</div>
	{/if}

	<!-- Editor surface -->
	<div class="relative">
		{#if mode === 'rich'}
			<div
				bind:this={editorEl}
				class="mv-editor-content prose prose-invert prose-sm min-h-[200px] max-w-none px-4 py-3 text-sm text-slate-200 outline-none"
				contenteditable={!readonly}
				role="textbox" tabindex="0"
				aria-multiline="true"
				aria-placeholder={placeholder}
				on:input={onInput}
				on:keydown={onKeyDown}
				on:paste={onPaste}
				on:contextmenu={onContextMenu}
			></div>
			{#if isEmpty && !readonly}
				<div class="pointer-events-none absolute left-4 top-3 text-sm text-slate-600">
					{placeholder}
				</div>
			{/if}

			<!-- Slash command menu -->
			{#if showSlashMenu && slashMenuItems.length > 0}
				<div
					class="absolute z-40 w-64 max-h-64 overflow-y-auto rounded-xl border border-slate-700 bg-slate-900 py-1 shadow-2xl"
					style="left: {slashMenuX}px; top: {slashMenuY}px;"
				>
					<div class="px-3 py-1 text-[10px] uppercase tracking-wide text-slate-500">Commands</div>
					{#each slashMenuItems as item, idx (item.id)}
						<button
							class={`flex w-full items-center gap-3 px-3 py-2 text-left text-xs transition ${
								idx === slashActiveIndex
									? 'bg-sky-500/10 text-white'
									: 'text-slate-300 hover:bg-slate-800'
							}`}
							on:mousedown|preventDefault={() => executeSlashItem(item)}
							on:mouseenter={() => (slashActiveIndex = idx)}
						>
							<span class="font-medium">{item.label}</span>
							<span class="text-[10px] text-slate-500">{item.description}</span>
						</button>
					{/each}
				</div>
			{/if}

			<!-- Table context menu -->
			{#if showTableMenu}
				<div
					class="absolute z-50 w-48 rounded-xl border border-slate-700 bg-slate-900 py-1 shadow-2xl"
					style="left: {tableMenuX}px; top: {tableMenuY}px;"
				>
					<div class="px-3 py-1 text-[10px] uppercase tracking-wide text-slate-500">Table</div>
					<button class="flex w-full px-3 py-1.5 text-left text-xs text-slate-300 hover:bg-slate-800" on:mousedown|preventDefault={addRowAbove}>Add row above</button>
					<button class="flex w-full px-3 py-1.5 text-left text-xs text-slate-300 hover:bg-slate-800" on:mousedown|preventDefault={addRowBelow}>Add row below</button>
					<button class="flex w-full px-3 py-1.5 text-left text-xs text-slate-300 hover:bg-slate-800" on:mousedown|preventDefault={addColLeft}>Add column left</button>
					<button class="flex w-full px-3 py-1.5 text-left text-xs text-slate-300 hover:bg-slate-800" on:mousedown|preventDefault={addColRight}>Add column right</button>
					<div class="my-1 border-t border-slate-800"></div>
					<button class="flex w-full px-3 py-1.5 text-left text-xs text-red-400 hover:bg-slate-800" on:mousedown|preventDefault={deleteRow}>Delete row</button>
					<button class="flex w-full px-3 py-1.5 text-left text-xs text-red-400 hover:bg-slate-800" on:mousedown|preventDefault={deleteCol}>Delete column</button>
				</div>
			{/if}
		{:else}
			<textarea
				bind:this={sourceEl}
				class="min-h-[200px] w-full resize-y bg-transparent px-4 py-3 font-mono text-sm text-slate-200 outline-none"
				readonly={readonly}
				placeholder={placeholder}
				value={value}
				on:input={onSourceInput}
				on:keydown={onSourceKeyDown}
			></textarea>
		{/if}
	</div>

	<!-- Footer with word count -->
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
</div>

<!-- Link modal -->
{#if showLinkModal}
	<div class="fixed inset-0 z-50 flex items-center justify-center bg-black/60" role="button" aria-label="Close link modal" tabindex="0" on:click|self={cancelLink} on:keydown={(e) => { if (e.key === 'Escape') cancelLink(); }}>
		<div class="w-96 rounded-xl border border-slate-700 bg-slate-900 p-5 shadow-2xl">
			<h3 class="mb-3 text-sm font-semibold text-white">Insert Link</h3>
			<div class="flex flex-col gap-3">
				<input
					class="rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white outline-none focus:border-sky-500"
					placeholder="URL"
					bind:value={linkUrl}
				bind:this={linkUrlInput}
					
					on:keydown={(e) => { if (e.key === 'Enter') applyLink(); if (e.key === 'Escape') cancelLink(); }}
				/>
				<input
					class="rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white outline-none focus:border-sky-500"
					placeholder="Text (optional)"
					bind:value={linkText}
					on:keydown={(e) => { if (e.key === 'Enter') applyLink(); if (e.key === 'Escape') cancelLink(); }}
				/>
				<div class="flex justify-end gap-2">
					<button
						class="rounded-lg px-3 py-2 text-xs text-slate-400 hover:bg-slate-800 hover:text-white"
						on:click={cancelLink}
					>
						Cancel
					</button>
					<button
						class="rounded-lg bg-sky-500 px-3 py-2 text-xs font-semibold text-white hover:bg-sky-400 disabled:opacity-40"
						disabled={!linkUrl}
						on:click={applyLink}
					>
						Insert
					</button>
				</div>
			</div>
		</div>
	</div>
{/if}

<!-- Image modal -->
{#if showImageModal}
	<div class="fixed inset-0 z-50 flex items-center justify-center bg-black/60" role="button" aria-label="Close image modal" tabindex="0" on:click|self={cancelImage} on:keydown={(e) => { if (e.key === 'Escape') cancelImage(); }}>
		<div class="w-96 rounded-xl border border-slate-700 bg-slate-900 p-5 shadow-2xl">
			<h3 class="mb-3 text-sm font-semibold text-white">Insert Image</h3>
			<div class="flex flex-col gap-3">
				<input
					class="rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white outline-none focus:border-sky-500"
					placeholder="Image URL"
					bind:value={imageUrl}
				bind:this={imageUrlInput}
					
					on:keydown={(e) => { if (e.key === 'Enter') applyImage(); if (e.key === 'Escape') cancelImage(); }}
				/>
				<input
					class="rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-xs text-white outline-none focus:border-sky-500"
					placeholder="Alt text (optional)"
					bind:value={imageAlt}
					on:keydown={(e) => { if (e.key === 'Enter') applyImage(); if (e.key === 'Escape') cancelImage(); }}
				/>
				{#if imageUrl}
					<div class="rounded-lg border border-slate-700 bg-slate-800 p-2">
						<img src={imageUrl} alt={imageAlt || 'Preview'} class="max-h-32 w-full rounded object-contain" />
					</div>
				{/if}
				<div class="flex justify-end gap-2">
					<button
						class="rounded-lg px-3 py-2 text-xs text-slate-400 hover:bg-slate-800 hover:text-white"
						on:click={cancelImage}
					>
						Cancel
					</button>
					<button
						class="rounded-lg bg-sky-500 px-3 py-2 text-xs font-semibold text-white hover:bg-sky-400 disabled:opacity-40"
						disabled={!imageUrl}
						on:click={applyImage}
					>
						Insert
					</button>
				</div>
			</div>
		</div>
	</div>
{/if}

<style>
	.mv-editor-content :global(h1) {
		font-size: 1.5rem;
		font-weight: 700;
		margin: 0.75rem 0 0.5rem;
		color: white;
	}
	.mv-editor-content :global(h2) {
		font-size: 1.25rem;
		font-weight: 600;
		margin: 0.75rem 0 0.5rem;
		color: white;
	}
	.mv-editor-content :global(h3) {
		font-size: 1.1rem;
		font-weight: 600;
		margin: 0.5rem 0 0.25rem;
		color: white;
	}
	.mv-editor-content :global(h4) {
		font-size: 1rem;
		font-weight: 600;
		margin: 0.5rem 0 0.25rem;
		color: rgb(203 213 225);
	}
	.mv-editor-content :global(p) {
		margin: 0.25rem 0;
	}
	.mv-editor-content :global(blockquote) {
		border-left: 3px solid rgb(56 189 248);
		padding-left: 1rem;
		margin: 0.5rem 0;
		color: rgb(148 163 184);
	}
	.mv-editor-content :global(pre) {
		background: rgb(15 23 42);
		border: 1px solid rgb(30 41 59);
		border-radius: 8px;
		padding: 0.75rem 1rem;
		margin: 0.5rem 0;
		overflow-x: auto;
		font-size: 0.8rem;
	}
	.mv-editor-content :global(code) {
		font-family: 'SF Mono', 'Fira Code', 'Cascadia Code', monospace;
		font-size: 0.85em;
	}
	.mv-editor-content :global(:not(pre) > code) {
		background: rgb(30 41 59);
		padding: 0.15rem 0.4rem;
		border-radius: 4px;
		color: rgb(248 113 113);
	}
	.mv-editor-content :global(ul),
	.mv-editor-content :global(ol) {
		padding-left: 1.5rem;
		margin: 0.25rem 0;
	}
	.mv-editor-content :global(li) {
		margin: 0.15rem 0;
	}
	.mv-editor-content :global(ul[data-type='taskList']) {
		list-style: none;
		padding-left: 0;
	}
	.mv-editor-content :global(li[data-type='taskItem']) {
		display: flex;
		align-items: flex-start;
		gap: 0.5rem;
	}
	.mv-editor-content :global(li[data-type='taskItem'] label) {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		cursor: pointer;
	}
	.mv-editor-content :global(li[data-type='taskItem'] input[type='checkbox']) {
		accent-color: rgb(56 189 248);
		cursor: pointer;
	}
	.mv-editor-content :global(li[data-type='taskItem'][data-checked='true'] span) {
		text-decoration: line-through;
		color: rgb(100 116 139);
	}
	.mv-editor-content :global(a) {
		color: rgb(56 189 248);
		text-decoration: underline;
		text-decoration-color: rgb(56 189 248 / 0.3);
	}
	.mv-editor-content :global(a:hover) {
		text-decoration-color: rgb(56 189 248);
	}
	.mv-editor-content :global(a.mv-wikilink) {
		color: rgb(192 132 252);
		text-decoration-color: rgb(192 132 252 / 0.3);
	}
	.mv-editor-content :global(hr) {
		border: none;
		border-top: 1px solid rgb(51 65 85);
		margin: 1rem 0;
	}
	.mv-editor-content :global(s),
	.mv-editor-content :global(del) {
		color: rgb(100 116 139);
	}
	.mv-editor-content :global(table) {
		width: 100%;
		border-collapse: collapse;
		margin: 0.5rem 0;
		font-size: 0.85rem;
	}
	.mv-editor-content :global(th),
	.mv-editor-content :global(td) {
		border: 1px solid rgb(51 65 85);
		padding: 0.4rem 0.75rem;
		text-align: left;
		min-width: 80px;
	}
	.mv-editor-content :global(th) {
		background: rgb(30 41 59);
		font-weight: 600;
		color: rgb(203 213 225);
	}
	.mv-editor-content :global(td) {
		background: rgb(15 23 42 / 0.5);
	}
	.mv-editor-content :global(img) {
		max-width: 100%;
		border-radius: 8px;
		margin: 0.5rem 0;
	}
	/* highlight.js token colours (dark theme) */
	.mv-editor-content :global(.hljs-keyword) { color: #c678dd; }
	.mv-editor-content :global(.hljs-string) { color: #98c379; }
	.mv-editor-content :global(.hljs-number) { color: #d19a66; }
	.mv-editor-content :global(.hljs-comment) { color: #5c6370; font-style: italic; }
	.mv-editor-content :global(.hljs-function) { color: #61afef; }
	.mv-editor-content :global(.hljs-title) { color: #61afef; }
	.mv-editor-content :global(.hljs-params) { color: #abb2bf; }
	.mv-editor-content :global(.hljs-built_in) { color: #e6c07b; }
	.mv-editor-content :global(.hljs-type) { color: #e6c07b; }
	.mv-editor-content :global(.hljs-literal) { color: #56b6c2; }
	.mv-editor-content :global(.hljs-attr) { color: #d19a66; }
	.mv-editor-content :global(.hljs-variable) { color: #e06c75; }
	.mv-editor-content :global(.hljs-template-variable) { color: #e06c75; }
	.mv-editor-content :global(.hljs-tag) { color: #e06c75; }
	.mv-editor-content :global(.hljs-name) { color: #e06c75; }
	.mv-editor-content :global(.hljs-selector-class) { color: #d19a66; }
	.mv-editor-content :global(.hljs-selector-id) { color: #61afef; }
	.mv-editor-content :global(.hljs-meta) { color: #56b6c2; }
	.mv-editor-content :global(.hljs-punctuation) { color: #abb2bf; }
</style>
