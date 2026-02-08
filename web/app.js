// MindVault Admin UI JavaScript
import { Editor, Extension } from 'https://unpkg.com/@tiptap/core@3.19.0/dist/index.js';
import { StarterKit } from 'https://unpkg.com/@tiptap/starter-kit@3.19.0/dist/index.js';
import { Link } from 'https://unpkg.com/@tiptap/extension-link@3.19.0/dist/index.js';
import { Table } from 'https://unpkg.com/@tiptap/extension-table@3.19.0/dist/index.js';
import { TableRow } from 'https://unpkg.com/@tiptap/extension-table-row@3.19.0/dist/index.js';
import { TableCell } from 'https://unpkg.com/@tiptap/extension-table-cell@3.19.0/dist/index.js';
import { TableHeader } from 'https://unpkg.com/@tiptap/extension-table-header@3.19.0/dist/index.js';
import { TaskList } from 'https://unpkg.com/@tiptap/extension-task-list@3.19.0/dist/index.js';
import { TaskItem } from 'https://unpkg.com/@tiptap/extension-task-item@3.19.0/dist/index.js';
import { Placeholder } from 'https://unpkg.com/@tiptap/extension-placeholder@3.19.0/dist/index.js';
import { Markdown } from 'https://unpkg.com/@tiptap/markdown@3.19.0/dist/index.js';
import Suggestion from 'https://unpkg.com/@tiptap/suggestion@3.19.0/dist/index.js';
import { AttachmentTriage } from './attachments.js';

const AI_TRANSFORM_SELECTION_CHAR_LIMIT = 5000;


class MindVaultAdmin {
    constructor() {
        this.apiBase = this.getApiBase();
        this.currentPage = 1;
        this.pageSize = 20;
        this.currentFilters = {};
        this.editorMode = 'wysiwyg';
        this.tiptapEditor = null;
        this.suppressEditorSync = false;
        this.editorSyncTimer = null;
        this.editorSuggestionsTimer = null;
        this.autoCompleteTimer = null;
        this.linkSuggestionTimer = null;
        this.autoSuggestEnabled = true;
        this.lastSuggestionSignature = '';
        this.lastSuggestionFetchedAt = 0;
        this.autoCompleteSuggestions = [];
        this.autoCompleteActiveIndex = -1;
        this.autoCompleteContext = null;
        this.wikiLinkSuggestions = [];
        this.wikiLinkContext = null;
        this.wikiLinkActiveIndex = -1;
        this.wysiwygWikiLinkSuggestions = [];
        this.wysiwygWikiLinkActiveIndex = -1;
        this.wysiwygWikiLinkPanel = null;
        this.wysiwygWikiLinkAlias = '';
        this.wysiwygWikiLinkQuery = '';
        this.wysiwygLinkMode = 'wiki';
        this.wysiwygWikiLinkCommand = null;
        this.dailyNotesCache = [];
        this.dueTasksCache = [];
        this.focusTasksCache = [];
        this.calendarItemsCache = [];
        this.templatesCache = [];
        this.templateHistoryCache = [];
        this.templatePacksCache = [];
        this.savedSearchesCache = [];
        this.permissionTemplatesCache = [];
        this.accessKeysCache = [];
        this.editingPermissionTemplateId = null;
        this.activeSavedSearchId = null;
        this.selectedVersionPreview = null;
        this.selectedDailyNoteId = null;
        this.selectedTemplateId = null;
        this.currentEditingNode = null;
        this.auditOffset = 0;
        this.auditHasMore = false;
        this.auditLoadingMore = false;
        this.auditQuerySignature = '';

        this.richEditor = document.getElementById('rich-editor');
        this.markdownEditor = document.getElementById('markdown-editor');
        this.editorSurface = document.getElementById('editor-surface');
        this.aiSuggestions = document.getElementById('ai-suggestions');
        this.aiSuggestionsMeta = document.getElementById('ai-suggestions-meta');
        this.autoCompleteSuggestionsPanel = document.getElementById('autocomplete-suggestions');
        this.linkSuggestionsPanel = document.getElementById('link-suggestions');
        this.aiTransformSelectionBtn = document.getElementById('ai-transform-selection-btn');
        this.createWysiwygWikiLinkPanel();
        this.dailyLinkedItems = document.getElementById('daily-linked-items');
        this.dueTasksList = document.getElementById('due-tasks-list');
        this.focusTasksList = document.getElementById('focus-tasks-list');
        this.focusGeneratedAt = document.getElementById('focus-generated-at');
        this.calendarItemsList = document.getElementById('calendar-items-list');
        this.calendarRangeSummary = document.getElementById('calendar-range-summary');
        this.templatesList = document.getElementById('templates-list');
        this.savedSearchesList = document.getElementById('saved-searches-list');
        this.permissionTemplatesList = document.getElementById('permission-templates-list');
        this.accessKeysList = document.getElementById('access-keys-list');
        this.accessKeyOutput = document.getElementById('access-key-output');
        this.accessKeyToken = document.getElementById('access-key-token');
        this.attachmentPreview = document.getElementById('attachment-preview');
        this.fileAttachments = document.getElementById('file-attachments');
        this.nodeVersionHistory = document.getElementById('node-version-history');
        this.nodeRelationshipOverview = document.getElementById('node-relationship-overview');
        this.auditContent = document.getElementById('audit-content');
        this.loadMoreAuditBtn = document.getElementById('load-more-audit-btn');

        this.attachedFiles = [];
        this.existingAttachments = [];
        this.attachmentChunksCache = new Map();
        this.attachmentTriage = new AttachmentTriage(this);
        this.init();
    }

    getApiBase() {
        if (window.location.protocol === 'http:' || window.location.protocol === 'https:') {
            return window.location.origin;
        }
        return 'http://127.0.0.1:9470';
    }

    init() {
        this.bindEvents();
        this.attachmentTriage.init();
        this.initRichTextEditor();
        this.setEditorContentFromMarkdown('');
        this.setEditorMode(this.editorMode);
        this.initializeDailyNotesControls();
        this.initializeCalendarControls();
        this.initializeTemplateControls();
        this.loadNodes();
        this.loadStats();
        this.loadSavedSearches();
        this.loadPermissionTemplates();
        this.loadAccessKeys();
        this.resetPermissionTemplateForm();
    }

    initRichTextEditor() {
        if (!this.richEditor) {
            return;
        }

        const self = this;
        const WikiLinkSuggestion = Extension.create({
            name: 'wikiLinkSuggestion',
            addProseMirrorPlugins() {
                return [
                    Suggestion({
                        editor: this.editor,
                        char: '[',
                        allowSpaces: true,
                        startOfLine: false,
                        allow: ({ state, range }) => {
                            if (range.from < 0) {
                                return false;
                            }
                            const before = state.doc.textBetween(
                                Math.max(range.from - 1, 0),
                                range.from + 1,
                                '\0',
                                '\0'
                            );
                            const at = state.doc.textBetween(range.from, range.from + 2, '\0', '\0');
                            return before === '[[' || at === '[[';
                        },
                        items: async ({ query }) => {
                            return await self.fetchWysiwygWikiLinkSuggestions(query);
                        },
                        command: ({ editor, range, props }) => {
                            self.insertWysiwygWikiLink(editor, range, props);
                        },
                        render: () => {
                            return {
                                onStart: (props) => {
                                    self.wysiwygLinkMode = 'wiki';
                                    self.wysiwygWikiLinkCommand = props.command;
                                    self.renderWysiwygWikiLinkSuggestions(props.items, props.query, props.clientRect);
                                },
                                onUpdate: (props) => {
                                    self.wysiwygLinkMode = 'wiki';
                                    self.wysiwygWikiLinkCommand = props.command;
                                    self.renderWysiwygWikiLinkSuggestions(props.items, props.query, props.clientRect);
                                },
                                onKeyDown: (props) => {
                                    return self.handleWysiwygWikiLinkKeyDown(props);
                                },
                                onExit: () => {
                                    self.hideWysiwygWikiLinkSuggestions();
                                }
                            };
                        }
                    })
                ];
            }
        });
        const MentionSuggestion = Extension.create({
            name: 'mentionSuggestion',
            addProseMirrorPlugins() {
                return [
                    Suggestion({
                        editor: this.editor,
                        char: '@',
                        allowSpaces: true,
                        startOfLine: false,
                        allow: ({ state, range }) => {
                            if (range.from < 0) {
                                return false;
                            }
                            const before = state.doc.textBetween(
                                Math.max(range.from - 1, 0),
                                range.from,
                                '\0',
                                '\0'
                            );
                            return !/[A-Za-z0-9._-]/.test(before);
                        },
                        items: async ({ query }) => {
                            return await self.fetchWysiwygWikiLinkSuggestions(query, 'mention');
                        },
                        command: ({ editor, range, props }) => {
                            self.insertWysiwygMention(editor, range, props);
                        },
                        render: () => {
                            return {
                                onStart: (props) => {
                                    self.wysiwygLinkMode = 'mention';
                                    self.wysiwygWikiLinkCommand = props.command;
                                    self.renderWysiwygWikiLinkSuggestions(props.items, props.query, props.clientRect);
                                },
                                onUpdate: (props) => {
                                    self.wysiwygLinkMode = 'mention';
                                    self.wysiwygWikiLinkCommand = props.command;
                                    self.renderWysiwygWikiLinkSuggestions(props.items, props.query, props.clientRect);
                                },
                                onKeyDown: (props) => {
                                    return self.handleWysiwygWikiLinkKeyDown(props);
                                },
                                onExit: () => {
                                    self.hideWysiwygWikiLinkSuggestions();
                                }
                            };
                        }
                    })
                ];
            }
        });

        this.tiptapEditor = new Editor({
            element: this.richEditor,
            extensions: [
                StarterKit,
                Link.configure({ openOnClick: false }),
                TaskList,
                TaskItem.configure({ nested: true }),
                Table.configure({ resizable: true }),
                TableRow,
                TableHeader,
                TableCell,
                Placeholder.configure({ placeholder: 'Write your note…' }),
                WikiLinkSuggestion,
                MentionSuggestion,
                Markdown
            ],
            content: '',
            contentType: 'markdown',
            editorProps: {
                attributes: {
                    class: 'tiptap-content'
                },
                handleKeyDown: (_view, event) => {
                    const isTransformHotkey = (event.metaKey || event.ctrlKey)
                        && event.shiftKey
                        && event.key.toLowerCase() === 't';
                    if (isTransformHotkey) {
                        event.preventDefault();
                        this.requestEditorTransform(true);
                        return true;
                    }
                    if (event.key === 'Escape') {
                        this.clearAutoComplete();
                        this.clearWikiLinkSuggestions();
                        return false;
                    }

                    if (this.autoCompleteSuggestions.length > 0) {
                        if (event.key === 'ArrowDown') {
                            event.preventDefault();
                            this.moveAutoCompleteSelection(1);
                            return true;
                        }
                        if (event.key === 'ArrowUp') {
                            event.preventDefault();
                            this.moveAutoCompleteSelection(-1);
                            return true;
                        }
                        if (event.key === 'Tab' || event.key === 'Enter') {
                            event.preventDefault();
                            this.acceptActiveAutoComplete();
                            return true;
                        }
                    }

                    return false;
                }
            },
            onSelectionUpdate: () => {
                this.updateTransformSelectionButton();
            },
            onUpdate: () => {
                if (this.suppressEditorSync || this.editorMode === 'markdown') {
                    return;
                }
                this.debounceEditorSync('rich');
                this.debounceEditorSuggestions();
                this.debounceAutoComplete();
                this.clearWikiLinkSuggestions();
            }
        });
    }

    initializeDailyNotesControls() {
        const dailyDateInput = document.getElementById('daily-date');
        if (dailyDateInput) {
            dailyDateInput.value = this.todayDateString();
        }
        const dueBeforeInput = document.getElementById('due-before');
        if (dueBeforeInput) {
            dueBeforeInput.value = this.nowDatetimeLocalString();
        }
        const focusLimitInput = document.getElementById('focus-limit');
        if (focusLimitInput) {
            focusLimitInput.value = focusLimitInput.value || '8';
        }
    }

    initializeCalendarControls() {
        const anchorInput = document.getElementById('calendar-anchor');
        if (anchorInput) {
            anchorInput.value = this.todayDateString();
        }
        const namespaceInput = document.getElementById('calendar-namespace');
        if (namespaceInput && !namespaceInput.value.trim()) {
            namespaceInput.value = 'default';
        }
    }

    initializeTemplateControls() {
        const templateNamespaceFilter = document.getElementById('template-namespace-filter');
        if (templateNamespaceFilter) {
            templateNamespaceFilter.value = '';
        }
        const templateCreateNamespace = document.getElementById('template-create-namespace');
        if (templateCreateNamespace) {
            templateCreateNamespace.value = 'default';
        }
        const templateInstanceNamespace = document.getElementById('template-instance-namespace');
        if (templateInstanceNamespace) {
            templateInstanceNamespace.value = 'default';
        }
        const templatePackNamespace = document.getElementById('template-pack-namespace');
        if (templatePackNamespace) {
            templatePackNamespace.value = 'default';
        }
    }

    bindEvents() {
        // Tab switching
        document.querySelectorAll('.tab').forEach((tab) => {
            tab.addEventListener('click', (event) => this.switchTab(event.target.id));
        });

        // Nodes panel
        document.getElementById('add-node-btn').addEventListener('click', () => this.showAddNodeModal());
        document.getElementById('apply-filters-btn').addEventListener('click', () => this.applyFilters());
        document.getElementById('prev-page-btn').addEventListener('click', () => this.changePage(-1));
        document.getElementById('next-page-btn').addEventListener('click', () => this.changePage(1));

        // Search panel
        document.getElementById('search-btn').addEventListener('click', () => this.performSearch());
        document.getElementById('search-query').addEventListener('keydown', (event) => {
            if (event.key === 'Enter') {
                this.performSearch();
            }
        });
        document.getElementById('save-search-btn').addEventListener('click', () => this.saveCurrentSearch());
        document.getElementById('update-search-btn').addEventListener('click', () => this.updateActiveSavedSearch());
        document.getElementById('refresh-saved-searches-btn').addEventListener('click', () => this.loadSavedSearches());

        // Graph panel
        document.getElementById('load-graph-btn').addEventListener('click', () => this.loadGraph());

        // Daily notes panel
        document.getElementById('ensure-today-btn').addEventListener('click', () => this.ensureTodayDailyNote());
        document.getElementById('ensure-selected-day-btn').addEventListener('click', () => this.ensureSelectedDailyNote());
        document.getElementById('refresh-daily-btn').addEventListener('click', () => this.loadDailyNotes());
        document.getElementById('refresh-due-tasks-btn').addEventListener('click', () => this.loadDueTasks());
        document.getElementById('due-include-completed').addEventListener('change', () => this.loadDueTasks());
        document.getElementById('refresh-focus-tasks-btn').addEventListener('click', () => this.loadFocusTasks());
        document.getElementById('focus-include-completed').addEventListener('change', () => this.loadFocusTasks());
        document.getElementById('focus-include-no-due').addEventListener('change', () => this.loadFocusTasks());
        document.getElementById('focus-limit').addEventListener('change', () => this.loadFocusTasks());
        document.getElementById('refresh-calendar-btn').addEventListener('click', () => this.loadCalendarItems());
        document.getElementById('calendar-view').addEventListener('change', () => this.loadCalendarItems());
        document.getElementById('calendar-anchor').addEventListener('change', () => this.loadCalendarItems());
        document.getElementById('calendar-include-completed').addEventListener('change', () => this.loadCalendarItems());
        document.getElementById('calendar-prev-btn').addEventListener('click', () => this.shiftCalendarAnchor(-1));
        document.getElementById('calendar-next-btn').addEventListener('click', () => this.shiftCalendarAnchor(1));
        document.getElementById('calendar-today-btn').addEventListener('click', () => this.resetCalendarAnchorToToday());
        document.getElementById('calendar-new-event-btn').addEventListener('click', () => this.startCalendarEventCapture());
        document.getElementById('export-calendar-ical-btn').addEventListener('click', () => this.exportCalendarIcal());
        document.getElementById('import-calendar-ical-file').addEventListener('change', (event) => {
            this.importCalendarIcal(event.target.files);
        });
        document.getElementById('calendar-namespace').addEventListener('keydown', (event) => {
            if (event.key === 'Enter') {
                this.loadCalendarItems();
            }
        });

        // Templates panel
        document.getElementById('refresh-templates-btn').addEventListener('click', () => this.loadTemplates());
        document.getElementById('refresh-template-packs-btn').addEventListener('click', () => this.loadTemplatePacks());
        document.getElementById('apply-template-filters-btn').addEventListener('click', () => this.loadTemplates());
        document.getElementById('template-create-form').addEventListener('submit', (event) => this.createTemplate(event));
        document.getElementById('template-instance-form').addEventListener('submit', (event) => this.instantiateSelectedTemplate(event));

        // Access panel
        document.getElementById('refresh-permissions-btn').addEventListener('click', () => {
            this.loadPermissionTemplates();
            this.loadAccessKeys();
        });
        document.getElementById('permission-template-form').addEventListener('submit', (event) => this.submitPermissionTemplate(event));
        document.getElementById('permission-template-cancel').addEventListener('click', () => this.resetPermissionTemplateForm());
        document.getElementById('access-key-form').addEventListener('submit', (event) => this.createAccessKey(event));

        document.getElementById('export-data-btn').addEventListener('click', () => this.exportVaultData());
        document.getElementById('import-data-file').addEventListener('change', (event) => {
            this.importVaultData(event.target.files);
        });
        document.getElementById('refresh-audit-btn').addEventListener('click', () => this.loadAuditLogs(true));
        document.getElementById('clear-audit-filters-btn').addEventListener('click', () => {
            const subjectInput = document.getElementById('audit-subject');
            const actionInput = document.getElementById('audit-action');
            const sinceInput = document.getElementById('audit-since');
            const limitInput = document.getElementById('audit-limit');
            if (subjectInput) subjectInput.value = '';
            if (actionInput) actionInput.value = '';
            if (sinceInput) sinceInput.value = '';
            if (limitInput) limitInput.value = '50';
            this.loadAuditLogs(false);
        });
        if (this.loadMoreAuditBtn) {
            this.loadMoreAuditBtn.addEventListener('click', () => this.loadAuditLogs(true, true));
        }

        // Modal
        document.getElementById('close-modal-btn').addEventListener('click', () => this.hideModal());
        document.getElementById('cancel-node-btn').addEventListener('click', () => this.hideModal());
        document.getElementById('node-form').addEventListener('submit', (event) => this.saveNode(event));
        document.getElementById('close-template-version-modal-btn').addEventListener('click', () => {
            this.hideTemplateVersionModal();
        });
        document.getElementById('template-version-modal-cancel-btn').addEventListener('click', () => {
            this.hideTemplateVersionModal();
        });
        document.getElementById('template-version-modal-restore-btn').addEventListener('click', () => {
            if (!this.selectedVersionPreview) {
                return;
            }
            const { entityType, entityId, versionId } = this.selectedVersionPreview;
            this.hideTemplateVersionModal();
            if (entityType === 'node') {
                this.restoreNodeVersion(entityId, versionId);
            } else {
                this.restoreTemplateVersion(entityId, versionId);
            }
        });

        // Close modal on outside click
        document.getElementById('node-modal').addEventListener('click', (event) => {
            if (event.target.id === 'node-modal') {
                this.hideModal();
            }
        });
        document.getElementById('template-version-modal').addEventListener('click', (event) => {
            if (event.target.id === 'template-version-modal') {
                this.hideTemplateVersionModal();
            }
        });

        // Rich editor controls
        document.querySelectorAll('.editor-mode-btn').forEach((button) => {
            button.addEventListener('click', () => this.setEditorMode(button.dataset.editorMode));
        });

        document.querySelectorAll('.editor-action-btn').forEach((button) => {
            button.addEventListener('click', () => this.handleEditorCommand(button.dataset.editorCommand));
        });
        this.markdownEditor.addEventListener('input', () => {
            this.debounceEditorSync('markdown');
            this.debounceEditorSuggestions();
            this.debounceAutoComplete();
            this.debounceWikiLinkSuggestions();
        });

        this.markdownEditor.addEventListener('keydown', (event) => {
            if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 's') {
                event.preventDefault();
                document.getElementById('node-form').requestSubmit();
                return;
            }

            if (event.key === 'Escape') {
                event.preventDefault();
                this.clearAutoComplete();
                this.clearWikiLinkSuggestions();
                return;
            }

            if (this.wikiLinkSuggestions.length > 0) {
                if (event.key === 'ArrowDown') {
                    event.preventDefault();
                    this.moveWikiLinkSelection(1);
                    return;
                }
                if (event.key === 'ArrowUp') {
                    event.preventDefault();
                    this.moveWikiLinkSelection(-1);
                    return;
                }
                if (event.key === 'Tab' || event.key === 'Enter') {
                    event.preventDefault();
                    this.acceptActiveWikiLinkSuggestion();
                    return;
                }
            }

            if (this.autoCompleteSuggestions.length === 0) {
                return;
            }

            if (event.key === 'ArrowDown') {
                event.preventDefault();
                this.moveAutoCompleteSelection(1);
                return;
            }
            if (event.key === 'ArrowUp') {
                event.preventDefault();
                this.moveAutoCompleteSelection(-1);
                return;
            }
            if (event.key === 'Tab' || event.key === 'Enter') {
                event.preventDefault();
                this.acceptActiveAutoComplete();
                return;
            }
        });

        document.getElementById('ai-suggest-btn').addEventListener('click', () => {
            this.requestEditorSuggestions(false, { force: true });
        });
        const aiSuggestAuto = document.getElementById('ai-suggest-auto');
        if (aiSuggestAuto) {
            aiSuggestAuto.checked = this.autoSuggestEnabled;
            aiSuggestAuto.addEventListener('change', () => {
                this.autoSuggestEnabled = Boolean(aiSuggestAuto.checked);
                if (!this.autoSuggestEnabled) {
                    this.renderEditorSuggestions([], null);
                } else {
                    this.requestEditorSuggestions(true, { force: true });
                }
            });
        });
        document.getElementById('ai-link-btn').addEventListener('click', () => {
            this.requestWikiLinkSuggestions(false, this.resolveExplicitWikiQuery());
        });
        document.getElementById('ai-transform-btn').addEventListener('click', () => {
            this.requestEditorTransform();
        });
        if (this.aiTransformSelectionBtn) {
            this.aiTransformSelectionBtn.addEventListener('click', () => {
                this.requestEditorTransform(true);
            });
        }

        // File attachments
        this.fileAttachments.addEventListener('change', (event) => {
            this.handleFileSelection(event.target.files);
        });
    }

    setEditorMode(mode) {
        this.editorMode = mode;
        document.querySelectorAll('.editor-mode-btn').forEach((button) => {
            button.classList.toggle('active', button.dataset.editorMode === mode);
        });

        this.editorSurface.classList.remove('mode-wysiwyg', 'mode-markdown', 'mode-split');
        this.editorSurface.classList.add(`mode-${mode}`);

        if (mode === 'markdown') {
            this.syncMarkdownFromRichEditor();
            this.hideWysiwygWikiLinkSuggestions();
            if (this.tiptapEditor) {
                this.tiptapEditor.setEditable(false);
            }
            this.markdownEditor.focus();
            return;
        }

        this.clearAutoComplete();
        this.clearWikiLinkSuggestions();
        this.syncRichEditorFromMarkdown();
        if (this.tiptapEditor) {
            this.tiptapEditor.setEditable(true);
            this.tiptapEditor.commands.focus();
        }
        this.updateTransformSelectionButton();
    }

    handleEditorCommand(rawCommand) {
        if (!rawCommand || !this.tiptapEditor) {
            return;
        }

        if (this.editorMode === 'markdown') {
            this.showNotification('Switch to WYSIWYG or Split mode to use toolbar actions.', 'warning');
            return;
        }

        const editor = this.tiptapEditor;
        editor.commands.focus();

        const [command, value] = rawCommand.split(':');
        switch (command) {
            case 'bold':
                editor.chain().toggleBold().run();
                break;
            case 'italic':
                editor.chain().toggleItalic().run();
                break;
            case 'insertUnorderedList':
                editor.chain().toggleBulletList().run();
                break;
            case 'insertOrderedList':
                editor.chain().toggleOrderedList().run();
                break;
            case 'formatBlock':
                if (value === 'h2') {
                    editor.chain().toggleHeading({ level: 2 }).run();
                } else if (value === 'blockquote') {
                    editor.chain().toggleBlockquote().run();
                }
                break;
            case 'createLink': {
                const link = window.prompt('Enter URL:');
                if (link && link.trim().length > 0) {
                    editor.chain().extendMarkRange('link').setLink({ href: link.trim() }).run();
                } else {
                    editor.chain().unsetLink().run();
                }
                break;
            }
            case 'removeFormat':
                editor.chain().unsetAllMarks().clearNodes().run();
                break;
            default:
                break;
        }

        this.syncMarkdownFromRichEditor();
    }

    debounceEditorSync(source) {
        window.clearTimeout(this.editorSyncTimer);
        this.editorSyncTimer = window.setTimeout(() => {
            if (source === 'rich') {
                this.syncMarkdownFromRichEditor();
            } else {
                this.syncRichEditorFromMarkdown();
            }
        }, 180);
    }

    debounceEditorSuggestions() {
        window.clearTimeout(this.editorSuggestionsTimer);
        this.editorSuggestionsTimer = window.setTimeout(() => {
            if (!this.autoSuggestEnabled) {
                return;
            }
            this.requestEditorSuggestions(true);
        }, 700);
    }

    debounceAutoComplete() {
        window.clearTimeout(this.autoCompleteTimer);
        this.autoCompleteTimer = window.setTimeout(() => {
            this.requestAutoComplete(true);
        }, 500);
    }

    debounceWikiLinkSuggestions() {
        window.clearTimeout(this.linkSuggestionTimer);
        this.linkSuggestionTimer = window.setTimeout(() => {
            this.requestWikiLinkSuggestions(true);
        }, 420);
    }

    createWysiwygWikiLinkPanel() {
        if (this.wysiwygWikiLinkPanel) {
            return;
        }
        const panel = document.createElement('div');
        panel.id = 'wysiwyg-link-suggestions';
        panel.className = 'wysiwyg-link-suggestions';
        panel.setAttribute('role', 'listbox');
        panel.setAttribute('aria-label', 'Link suggestions');
        panel.style.display = 'none';
        document.body.appendChild(panel);
        this.wysiwygWikiLinkPanel = panel;
    }

    updateTransformSelectionButton() {
        if (!this.aiTransformSelectionBtn) {
            return;
        }
        const hasSelection = Boolean(
            this.tiptapEditor
            && this.editorMode !== 'markdown'
            && !this.tiptapEditor.state.selection.empty
        );
        this.aiTransformSelectionBtn.classList.toggle('is-hidden', !hasSelection);
    }

    getWysiwygSelectionMarkdown() {
        if (!this.tiptapEditor || this.editorMode === 'markdown') {
            return null;
        }
        const selection = this.tiptapEditor.state.selection;
        if (!selection || selection.empty) {
            return null;
        }
        const fragment = selection.content().content;
        if (!fragment || fragment.size === 0) {
            return null;
        }
        const jsonContent = typeof fragment.toJSON === 'function' ? fragment.toJSON() : null;
        if (!jsonContent) {
            return this.tiptapEditor.state.doc.textBetween(selection.from, selection.to, '\n');
        }
        const docJson = { type: 'doc', content: jsonContent };
        if (this.tiptapEditor.markdown && typeof this.tiptapEditor.markdown.serialize === 'function') {
            return this.tiptapEditor.markdown.serialize(docJson);
        }
        return this.tiptapEditor.state.doc.textBetween(selection.from, selection.to, '\n');
    }

    truncateSelectionMarkdown(markdown, limit = AI_TRANSFORM_SELECTION_CHAR_LIMIT) {
        const normalized = String(markdown || '');
        if (normalized.length <= limit) {
            return { text: normalized, truncated: false };
        }
        const slice = normalized.slice(0, limit);
        const boundary = Math.max(
            slice.lastIndexOf('\n\n'),
            slice.lastIndexOf('\n'),
            slice.lastIndexOf('. '),
            slice.lastIndexOf(' ')
        );
        const cutoff = boundary > 120 ? boundary : limit;
        return { text: slice.slice(0, cutoff).trim(), truncated: true };
    }

    replaceWysiwygSelection(replacement) {
        if (!this.tiptapEditor || this.tiptapEditor.state.selection.empty) {
            return false;
        }
        try {
            this.tiptapEditor.chain().focus().deleteSelection().insertContent(replacement, {
                contentType: 'markdown'
            }).run();
            this.syncMarkdownFromRichEditor();
            this.clearAutoComplete();
            this.clearWikiLinkSuggestions();
            return true;
        } catch (error) {
            return false;
        }
    }

    hideWysiwygWikiLinkSuggestions() {
        if (!this.wysiwygWikiLinkPanel) {
            return;
        }
        this.wysiwygWikiLinkPanel.style.display = 'none';
        this.wysiwygWikiLinkPanel.innerHTML = '';
        this.wysiwygWikiLinkSuggestions = [];
        this.wysiwygWikiLinkActiveIndex = -1;
        this.wysiwygWikiLinkCommand = null;
        this.wysiwygLinkMode = 'wiki';
    }

    positionWysiwygWikiLinkPanel(clientRect) {
        if (!this.wysiwygWikiLinkPanel || !clientRect) {
            return;
        }
        const rect = clientRect();
        if (!rect) {
            return;
        }
        const top = rect.bottom + window.scrollY + 6;
        const left = rect.left + window.scrollX;
        this.wysiwygWikiLinkPanel.style.top = `${top}px`;
        this.wysiwygWikiLinkPanel.style.left = `${left}px`;
    }

    renderWysiwygWikiLinkSuggestions(items, query, clientRect) {
        if (!this.wysiwygWikiLinkPanel) {
            return;
        }
        this.wysiwygWikiLinkPanel.innerHTML = '';
        this.wysiwygWikiLinkSuggestions = Array.isArray(items) ? items : [];
        this.wysiwygWikiLinkActiveIndex = this.wysiwygWikiLinkSuggestions.length > 0 ? 0 : -1;
        this.wysiwygWikiLinkQuery = String(query || '').trim();

        const header = document.createElement('div');
        header.className = 'wysiwyg-link-suggestions-header';
        const modeLabel = this.wysiwygLinkMode === 'mention' ? 'mention' : 'note';
        if (!this.wysiwygWikiLinkQuery) {
            header.textContent = `Type to search ${modeLabel}s…`;
        } else {
            const titleLabel = this.wysiwygLinkMode === 'mention'
                ? `Mentions for "${this.wysiwygWikiLinkQuery}"`
                : `Wiki links for "${this.wysiwygWikiLinkQuery}"`;
            header.textContent = titleLabel;
        }
        this.wysiwygWikiLinkPanel.appendChild(header);

        if (this.wysiwygWikiLinkSuggestions.length === 0) {
            const empty = document.createElement('div');
            empty.className = 'wysiwyg-link-empty';
            empty.textContent = this.wysiwygWikiLinkQuery
                ? 'No matches found.'
                : 'Keep typing to search.';
            this.wysiwygWikiLinkPanel.appendChild(empty);
        } else {
            this.wysiwygWikiLinkSuggestions.forEach((item, index) => {
                const entry = document.createElement('button');
                entry.type = 'button';
                entry.className = `wysiwyg-link-item ${index === this.wysiwygWikiLinkActiveIndex ? 'active' : ''}`;
                entry.dataset.suggestionIndex = String(index);

                if (item.create) {
                    entry.innerHTML = `<span class=\"wysiwyg-link-create\">Create new note: \"${this.escapeHtml(item.title || this.wysiwygWikiLinkQuery)}\"</span>`;
                } else {
                    const title = this.escapeHtml(item.title || 'Untitled');
                    const preview = item.preview ? this.escapeHtml(item.preview) : '';
                    const source = this.escapeHtml(item.source || 'fts');
                    entry.innerHTML = `\n                        <div class=\"wysiwyg-link-title\">${title}</div>\n                        <div class=\"wysiwyg-link-meta\">${source.toUpperCase()}</div>\n                        ${preview ? `<div class=\"wysiwyg-link-preview\">${preview}</div>` : ''}\n                    `;
                }

                entry.addEventListener('click', () => this.selectWysiwygWikiLinkSuggestion(index));
                this.wysiwygWikiLinkPanel.appendChild(entry);
            });
        }

        this.wysiwygWikiLinkPanel.style.display = 'block';
        this.positionWysiwygWikiLinkPanel(clientRect);
    }

    moveWysiwygWikiLinkSelection(direction) {
        if (this.wysiwygWikiLinkSuggestions.length === 0) {
            return;
        }
        const size = this.wysiwygWikiLinkSuggestions.length;
        this.wysiwygWikiLinkActiveIndex = (this.wysiwygWikiLinkActiveIndex + direction + size) % size;
        const items = this.wysiwygWikiLinkPanel?.querySelectorAll('.wysiwyg-link-item') || [];
        items.forEach((item, index) => {
            item.classList.toggle('active', index === this.wysiwygWikiLinkActiveIndex);
        });
    }

    selectWysiwygWikiLinkSuggestion(index) {
        if (!this.wysiwygWikiLinkCommand) {
            return;
        }
        const selection = this.wysiwygWikiLinkSuggestions[index];
        if (!selection) {
            return;
        }
        this.wysiwygWikiLinkCommand(selection);
        this.hideWysiwygWikiLinkSuggestions();
    }

    handleWysiwygWikiLinkKeyDown(props) {
        const { event } = props;
        if (event.key === 'Escape') {
            this.hideWysiwygWikiLinkSuggestions();
            return true;
        }
        if (event.key === 'ArrowDown') {
            event.preventDefault();
            this.moveWysiwygWikiLinkSelection(1);
            return true;
        }
        if (event.key === 'ArrowUp') {
            event.preventDefault();
            this.moveWysiwygWikiLinkSelection(-1);
            return true;
        }
        if (event.key === 'Enter' || event.key === 'Tab') {
            event.preventDefault();
            this.selectWysiwygWikiLinkSuggestion(
                this.wysiwygWikiLinkActiveIndex >= 0 ? this.wysiwygWikiLinkActiveIndex : 0
            );
            return true;
        }
        return false;
    }

    insertWysiwygWikiLink(editor, range, suggestion) {
        const title = String(suggestion?.title || this.wysiwygWikiLinkQuery || '').trim();
        if (!title) {
            return;
        }
        const heading = String(suggestion?.heading || '').trim();
        const alias = String(this.wysiwygWikiLinkAlias || '').trim();
        const target = heading ? `${title}#${heading}` : title;
        const linkText = alias ? `[[${target}|${alias}]]` : `[[${target}]]`;
        editor.chain().focus().insertContentAt(range, linkText).run();
        this.hideWysiwygWikiLinkSuggestions();
        this.syncMarkdownFromRichEditor();
    }

    insertWysiwygMention(editor, range, suggestion) {
        const title = String(suggestion?.title || this.wysiwygWikiLinkQuery || '').trim();
        if (!title) {
            return;
        }
        const mentionText = this.buildMentionSuggestionToken(title, {});
        if (!mentionText) {
            return;
        }
        editor.chain().focus().insertContentAt(range, mentionText).run();
        this.hideWysiwygWikiLinkSuggestions();
        this.syncMarkdownFromRichEditor();
    }

    async fetchWysiwygWikiLinkSuggestions(rawQuery, mode = 'wiki') {
        this.wysiwygLinkMode = mode === 'mention' ? 'mention' : 'wiki';
        let query = String(rawQuery || '');
        if (this.wysiwygLinkMode === 'wiki' && query.startsWith('[')) {
            query = query.slice(1);
        }
        let searchQuery = '';
        if (this.wysiwygLinkMode === 'wiki') {
            const [searchPart, aliasPart] = query.split('|');
            this.wysiwygWikiLinkAlias = String(aliasPart || '').trim();
            searchQuery = String(searchPart || '').trim();
        } else {
            this.wysiwygWikiLinkAlias = '';
            searchQuery = query.trim();
            if (searchQuery.startsWith('"') || searchQuery.startsWith('\'')) {
                searchQuery = searchQuery.slice(1);
            }
            if (searchQuery.endsWith('"') || searchQuery.endsWith('\'')) {
                searchQuery = searchQuery.slice(0, -1);
            }
            searchQuery = searchQuery.trim();
        }
        if (!searchQuery) {
            return [];
        }

        const namespace = document.getElementById('node-namespace').value.trim() || undefined;
        const limit = 8;
        const ftsResults = await this.fetchWikiLinkFtsResults(searchQuery, limit);
        const wordCount = searchQuery.split(/\s+/).filter(Boolean).length;
        const needsSemantic = ftsResults.length < 3 || searchQuery.length > 25 || wordCount > 3;
        let semanticResults = [];
        if (needsSemantic) {
            semanticResults = await this.fetchWikiLinkSemanticResults(searchQuery, namespace, limit);
        }
        const merged = this.mergeWysiwygWikiLinkSuggestions(ftsResults, semanticResults, searchQuery, limit);
        if (merged.length === 0 && this.wysiwygLinkMode === 'wiki') {
            return [{ id: 'create', title: searchQuery, create: true }];
        }
        return merged;
    }

    async fetchWikiLinkFtsResults(query, limit) {
        const params = new URLSearchParams({
            q: query,
            type: 'fulltext',
            limit: String(limit)
        });
        try {
            const data = await this.apiCall(`/api/v1/search?${params.toString()}`);
            if (!Array.isArray(data)) {
                return [];
            }
            return data
                .map((result) => ({
                    id: result?.node?.id || result?.node?.node_id || result?.node?.uuid || '',
                    title: String(result?.node?.title || '').trim(),
                    preview: String((result?.node?.content || '').slice(0, 140)).trim(),
                    namespace: String(result?.node?.namespace || '').trim() || 'default',
                    score: Number(result?.score || 0),
                    source: 'fts'
                }))
                .filter((item) => item.title.length > 0 && item.id !== this.currentEditingNode?.id);
        } catch (error) {
            return [];
        }
    }

    async fetchWikiLinkSemanticResults(query, namespace, limit) {
        const payload = {
            text: query,
            limit,
            namespace,
            exclude_node_id: this.currentEditingNode?.id || undefined
        };
        try {
            const response = await this.apiCall('/api/v1/assist/links', {
                method: 'POST',
                body: JSON.stringify(payload)
            });
            const suggestions = Array.isArray(response?.suggestions) ? response.suggestions : [];
            return suggestions
                .map((item) => ({
                    id: item.node_id || item.id || '',
                    title: String(item.title || '').trim(),
                    heading: String(item.heading || '').trim(),
                    preview: String(item.preview || '').trim(),
                    namespace: String(item.namespace || '').trim() || 'default',
                    score: Number(item.score || 0),
                    source: 'semantic'
                }))
                .filter((item) => item.title.length > 0);
        } catch (error) {
            return [];
        }
    }

    mergeWysiwygWikiLinkSuggestions(ftsResults, semanticResults, query, limit) {
        const normalizedQuery = String(query || '').toLowerCase();
        const results = [];
        const seen = new Set();
        const add = (item) => {
            const key = item.id || `${item.title.toLowerCase()}::${item.namespace}`;
            if (seen.has(key)) {
                return;
            }
            seen.add(key);
            const titleLower = String(item.title || '').toLowerCase();
            let rank = item.source === 'fts' ? 1 : 0.4;
            if (titleLower === normalizedQuery) {
                rank += 1.5;
            } else if (titleLower.startsWith(normalizedQuery)) {
                rank += 0.8;
            } else if (titleLower.includes(normalizedQuery)) {
                rank += 0.4;
            }
            rank += Number(item.score || 0);
            results.push({ ...item, rank });
        };
        (ftsResults || []).forEach(add);
        (semanticResults || []).forEach(add);
        return results
            .sort((a, b) => b.rank - a.rank)
            .slice(0, limit);
    }

    syncMarkdownFromRichEditor() {
        if (!this.tiptapEditor) {
            return;
        }
        if (typeof this.tiptapEditor.getMarkdown === 'function') {
            this.markdownEditor.value = this.tiptapEditor.getMarkdown().trim();
            return;
        }
        this.markdownEditor.value = this.tiptapEditor.getText().trim();
    }

    syncRichEditorFromMarkdown() {
        if (!this.tiptapEditor) {
            return;
        }
        const markdown = this.markdownEditor.value || '';
        this.suppressEditorSync = true;
        this.tiptapEditor.commands.setContent(markdown, {
            contentType: 'markdown',
            emitUpdate: false
        });
        this.suppressEditorSync = false;
    }

    setEditorContentFromMarkdown(markdown) {
        const safeMarkdown = markdown || '';
        this.markdownEditor.value = safeMarkdown;
        this.syncRichEditorFromMarkdown();
        this.clearAutoComplete();
        this.clearWikiLinkSuggestions();
    }

    getEditorMarkdown() {
        if (this.editorMode !== 'markdown') {
            this.syncMarkdownFromRichEditor();
        }
        return this.markdownEditor.value.trim();
    }

    async requestEditorSuggestions(silent = false, options = {}) {
        const text = this.getEditorMarkdown();
        if (text.length < 16) {
            this.renderEditorSuggestions([], null);
            if (!silent) {
                this.showNotification('Add a bit more context before requesting suggestions.', 'warning');
            }
            return;
        }

        const namespace = document.getElementById('node-namespace').value.trim() || undefined;
        const signature = `${namespace || '*'}:${text.slice(-1000)}`;
        const now = Date.now();
        if (
            !options.force
            && signature === this.lastSuggestionSignature
            && (now - this.lastSuggestionFetchedAt) < 8_000
        ) {
            return;
        }
        try {
            const response = await this.apiCall('/api/v1/assist/completion', {
                method: 'POST',
                body: JSON.stringify({
                    text,
                    limit: 4,
                    namespace
                })
            });
            const normalizedSources = Array.isArray(response.sources)
                ? response.sources
                    .map((item) => ({
                        nodeId: String(item?.node_id || item?.nodeId || '').trim(),
                        title: String(item?.title || '').trim(),
                        namespace: String(item?.namespace || '').trim(),
                        score: Number(item?.score || 0)
                    }))
                    .filter((item) => item.title.length > 0 || item.nodeId.length > 0)
                    .slice(0, 5)
                : [];
            this.lastSuggestionSignature = signature;
            this.lastSuggestionFetchedAt = now;
            this.renderEditorSuggestions(response.suggestions || [], {
                sourceNodes: Number(response.source_nodes || 0),
                strategy: String(response.strategy || '').trim(),
                sources: normalizedSources
            });
        } catch (error) {
            this.lastSuggestionFetchedAt = now;
            if (!silent) {
                this.showNotification('AI suggestions are currently unavailable.', 'warning');
            }
        }
    }

    renderEditorSuggestions(suggestions, context = null) {
        this.aiSuggestions.innerHTML = '';
        if (this.aiSuggestionsMeta) {
            this.aiSuggestionsMeta.textContent = '';
            this.aiSuggestionsMeta.classList.remove('active');
        }
        if (!Array.isArray(suggestions) || suggestions.length === 0) {
            this.aiSuggestions.classList.remove('active');
            return;
        }

        suggestions.forEach((suggestion) => {
            const chip = document.createElement('button');
            chip.type = 'button';
            chip.className = 'ai-suggestion-chip';
            chip.textContent = suggestion;
            chip.addEventListener('click', () => this.insertSuggestion(suggestion));
            this.aiSuggestions.appendChild(chip);
        });

        if (this.aiSuggestionsMeta) {
            const sourceNodes = Number(context?.sourceNodes || 0);
            const strategy = String(context?.strategy || '').trim();
            const strategyLabel = strategy ? strategy.replaceAll('_', ' ') : 'retrieval context';
            this.aiSuggestionsMeta.innerHTML = '';

            const summary = document.createElement('span');
            summary.className = 'ai-suggestions-summary';
            summary.textContent = sourceNodes > 0
                ? `Grounded on ${sourceNodes} vault node${sourceNodes === 1 ? '' : 's'} via ${strategyLabel}.`
                : `Generated via ${strategyLabel}.`;
            this.aiSuggestionsMeta.appendChild(summary);

            const sources = Array.isArray(context?.sources) ? context.sources : [];
            if (sources.length > 0) {
                const sourceContainer = document.createElement('div');
                sourceContainer.className = 'ai-suggestion-sources';
                sources.forEach((source) => {
                    const sourceButton = document.createElement('button');
                    sourceButton.type = 'button';
                    sourceButton.className = 'ai-suggestion-source-chip';
                    const sourceTitle = source.title || `Node ${String(source.nodeId || '').slice(0, 8)}`;
                    sourceButton.textContent = sourceTitle;
                    const scoreLabel = Number.isFinite(source.score) ? ` • score ${source.score.toFixed(2)}` : '';
                    sourceButton.title = `${sourceTitle}${source.namespace ? ` • ${source.namespace}` : ''}${scoreLabel}`;
                    sourceButton.addEventListener('click', () => this.insertSuggestionSourceCitation(source));
                    sourceContainer.appendChild(sourceButton);
                });
                this.aiSuggestionsMeta.appendChild(sourceContainer);
            }
            this.aiSuggestionsMeta.classList.add('active');
        }
        this.aiSuggestions.classList.add('active');
    }

    insertSuggestion(suggestion) {
        const current = this.getEditorMarkdown();
        const separator = current.length === 0 ? '' : '\n\n';
        this.setEditorContentFromMarkdown(`${current}${separator}${String(suggestion || '').trim()}`);
        this.showNotification('Suggestion inserted into note.', 'success');
    }

    insertSuggestionSourceCitation(source) {
        const nodeId = String(source?.nodeId || source?.node_id || '').trim();
        const rawTitle = String(source?.title || '').trim();
        const sanitizedTitle = rawTitle
            .replace(/[\[\]\r\n]+/g, ' ')
            .replace(/\s+/g, ' ')
            .trim();
        const fallbackTitle = nodeId ? `node-${nodeId.slice(0, 8)}` : 'reference';
        const citationTarget = sanitizedTitle || fallbackTitle;
        const citation = `[[${citationTarget}]]`;
        const current = this.getEditorMarkdown();
        const separator = current.length === 0 ? '' : '\n';
        this.setEditorContentFromMarkdown(`${current}${separator}${citation}`);
        this.showNotification(`Source citation inserted: ${citationTarget}`, 'success');
    }

    async requestEditorTransform(forceReplaceSelection = false) {
        const mode = (document.getElementById('ai-transform-mode')?.value || 'summarize').trim();
        const targetMode = forceReplaceSelection
            ? 'replace_selection'
            : this.resolveTransformTargetMode(
                document.getElementById('ai-transform-target')?.value
            );
        const selectionSnapshot = this.getMarkdownSelectionSnapshot();
        let selectionContext = null;

        let text = '';
        if (targetMode === 'replace_selection') {
            if (this.editorMode === 'markdown') {
                if (!selectionSnapshot || selectionSnapshot.selectedText.trim().length < 8) {
                    this.showNotification(
                        'Select Markdown text first (at least 8 characters) to replace with AI transform.',
                        'warning'
                    );
                    return;
                }
                text = selectionSnapshot.selectedText.trim();
                selectionContext = { mode: 'markdown', snapshot: selectionSnapshot };
            } else if (this.tiptapEditor) {
                const selectionMarkdown = this.getWysiwygSelectionMarkdown();
                if (!selectionMarkdown || selectionMarkdown.trim().length < 8) {
                    this.showNotification(
                        'Select text first (at least 8 characters) to replace with AI transform.',
                        'warning'
                    );
                    return;
                }
                const truncated = this.truncateSelectionMarkdown(selectionMarkdown.trim());
                text = truncated.text;
                selectionContext = { mode: 'wysiwyg' };
                if (truncated.truncated) {
                    this.showNotification(
                        `Selection truncated to ${AI_TRANSFORM_SELECTION_CHAR_LIMIT.toLocaleString()} characters for faster AI processing.`,
                        'info'
                    );
                }
            } else {
                this.showNotification('Switch to Markdown or WYSIWYG mode to replace selected text.', 'warning');
                return;
            }
        } else {
            text = this.getEditorMarkdown();
            if (text.length < 16) {
                this.showNotification('Add more context before running AI transform.', 'warning');
                return;
            }
        }

        const namespace = document.getElementById('node-namespace').value.trim() || undefined;
        try {
            const response = await this.apiCall('/api/v1/assist/transform', {
                method: 'POST',
                body: JSON.stringify({
                    text,
                    mode,
                    limit: 4,
                    namespace
                })
            });
            const transformed = String(response?.transformed_text || '').trim();
            if (!transformed) {
                this.showNotification('AI transform returned no content.', 'warning');
                return;
            }
            this.insertTransformResult(mode, transformed, targetMode, selectionSnapshot, selectionContext);
        } catch (error) {
            this.showNotification('AI transform is currently unavailable.', 'warning');
        }
    }

    resolveTransformTargetMode(rawMode) {
        const normalized = String(rawMode || '').trim().toLowerCase();
        if (normalized === 'replace_selection') {
            return 'replace_selection';
        }
        return 'append_section';
    }

    getMarkdownSelectionSnapshot() {
        const text = this.markdownEditor.value || '';
        const start = Number.isInteger(this.markdownEditor.selectionStart)
            ? this.markdownEditor.selectionStart
            : 0;
        const end = Number.isInteger(this.markdownEditor.selectionEnd)
            ? this.markdownEditor.selectionEnd
            : start;
        if (end <= start) {
            return null;
        }
        return {
            start,
            end,
            selectedText: text.slice(start, end)
        };
    }

    replaceMarkdownSelection(snapshot, replacement) {
        if (!snapshot) {
            return false;
        }
        const current = this.markdownEditor.value || '';
        if (
            snapshot.start < 0
            || snapshot.end < snapshot.start
            || snapshot.end > current.length
        ) {
            return false;
        }
        const currentSlice = current.slice(snapshot.start, snapshot.end);
        if (currentSlice !== snapshot.selectedText) {
            return false;
        }

        this.markdownEditor.focus();
        this.markdownEditor.setSelectionRange(snapshot.start, snapshot.end);
        this.markdownEditor.setRangeText(replacement, snapshot.start, snapshot.end, 'end');
        this.syncRichEditorFromMarkdown();
        this.clearAutoComplete();
        this.clearWikiLinkSuggestions();
        return true;
    }

    insertTransformResult(
        mode,
        transformedMarkdown,
        targetMode = 'append_section',
        selectionSnapshot = null,
        selectionContext = null
    ) {
        const normalizedMode = String(mode || 'summarize').toLowerCase();
        let sectionTitle = 'AI Transform';
        if (normalizedMode === 'summarize') {
            sectionTitle = 'AI Summary';
        } else if (normalizedMode === 'action_items') {
            sectionTitle = 'AI Action Items';
        } else if (normalizedMode === 'refine') {
            sectionTitle = 'AI Refined Draft';
        }

        if (targetMode === 'replace_selection') {
            let replaced = false;
            if (selectionContext?.mode === 'wysiwyg') {
                replaced = this.replaceWysiwygSelection(transformedMarkdown);
            } else {
                replaced = this.replaceMarkdownSelection(selectionSnapshot, transformedMarkdown);
            }
            if (replaced) {
                this.showNotification('Selected text replaced with AI transform.', 'success');
                return;
            }
            this.showNotification(
                'Selection changed during transform. Result inserted as a new section instead.',
                'info'
            );
        }

        const current = this.getEditorMarkdown();
        const separator = current.length === 0 ? '' : '\n\n';
        const section = `## ${sectionTitle}\n${transformedMarkdown}`;
        this.setEditorContentFromMarkdown(`${current}${separator}${section}`);
        this.showNotification(`${sectionTitle} inserted into note.`, 'success');
    }

    async requestAutoComplete(silent = false) {
        const context = this.getAutoCompleteContext();
        if (!context || context.prompt.length < 3) {
            this.clearAutoComplete();
            return;
        }

        this.autoCompleteContext = context;
        const namespace = document.getElementById('node-namespace').value.trim() || undefined;
        try {
            const response = await this.apiCall('/api/v1/assist/autocomplete', {
                method: 'POST',
                body: JSON.stringify({
                    text: context.prompt,
                    limit: 5,
                    namespace
                })
            });
            this.renderAutoComplete(response.completions || [], context.prompt);
        } catch (error) {
            if (!silent) {
                this.clearAutoComplete();
            }
        }
    }

    getAutoCompleteContext() {
        if (this.editorMode !== 'markdown' && this.tiptapEditor && document.activeElement !== this.markdownEditor) {
            const { state } = this.tiptapEditor;
            const { $from } = state.selection;
            const parentText = $from.parent?.textContent || '';
            const prompt = parentText.slice(0, $from.parentOffset).trim();
            if (!prompt) {
                return null;
            }
            return {
                mode: 'tiptap',
                prompt
            };
        }

        const text = this.markdownEditor.value || '';
        const cursor = typeof this.markdownEditor.selectionStart === 'number'
            ? this.markdownEditor.selectionStart
            : text.length;
        const beforeCursor = text.slice(0, cursor);
        const lineStart = beforeCursor.lastIndexOf('\n') + 1;
        const rawSegment = beforeCursor.slice(lineStart);
        const leadingWhitespaceMatch = rawSegment.match(/^\s*/);
        const leadingWhitespace = leadingWhitespaceMatch ? leadingWhitespaceMatch[0].length : 0;
        const prompt = rawSegment.slice(leadingWhitespace).trim();

        if (prompt.length === 0) {
            return null;
        }

        return {
            mode: 'markdown',
            text,
            cursor,
            promptStart: lineStart + leadingWhitespace,
            prompt
        };
    }

    renderAutoComplete(completions, prompt) {
        const normalizedPrompt = String(prompt || '').toLowerCase().trim();
        const unique = [];
        const seen = new Set();

        (Array.isArray(completions) ? completions : []).forEach((item) => {
            const value = String(item || '').trim();
            if (!value) {
                return;
            }
            const normalized = value.toLowerCase();
            if (normalized === normalizedPrompt || seen.has(normalized)) {
                return;
            }
            seen.add(normalized);
            unique.push(value);
        });

        this.autoCompleteSuggestions = unique.slice(0, 5);
        if (this.autoCompleteSuggestions.length === 0) {
            this.clearAutoComplete();
            return;
        }

        this.autoCompleteActiveIndex = 0;
        this.autoCompleteSuggestionsPanel.innerHTML = '';
        this.autoCompleteSuggestions.forEach((suggestion, index) => {
            const button = document.createElement('button');
            button.type = 'button';
            button.className = `autocomplete-item ${index === this.autoCompleteActiveIndex ? 'active' : ''}`;
            button.textContent = suggestion;
            button.addEventListener('click', () => this.acceptAutoCompleteByIndex(index));
            this.autoCompleteSuggestionsPanel.appendChild(button);
        });
        this.autoCompleteSuggestionsPanel.classList.add('active');
    }

    moveAutoCompleteSelection(direction) {
        if (this.autoCompleteSuggestions.length === 0) {
            return;
        }
        const size = this.autoCompleteSuggestions.length;
        this.autoCompleteActiveIndex = (this.autoCompleteActiveIndex + direction + size) % size;
        this.refreshAutoCompleteSelection();
    }

    refreshAutoCompleteSelection() {
        const buttons = this.autoCompleteSuggestionsPanel.querySelectorAll('.autocomplete-item');
        buttons.forEach((button, index) => {
            button.classList.toggle('active', index === this.autoCompleteActiveIndex);
        });
    }

    acceptActiveAutoComplete() {
        this.acceptAutoCompleteByIndex(this.autoCompleteActiveIndex >= 0 ? this.autoCompleteActiveIndex : 0);
    }

    acceptAutoCompleteByIndex(index) {
        if (!Array.isArray(this.autoCompleteSuggestions) || index < 0 || index >= this.autoCompleteSuggestions.length) {
            return;
        }
        const suggestion = this.autoCompleteSuggestions[index];
        this.insertAutoCompleteSuggestion(suggestion);
        this.clearAutoComplete();
    }

    insertAutoCompleteSuggestion(suggestion) {
        const context = this.getAutoCompleteContext() || this.autoCompleteContext;
        if (!context) {
            return;
        }

        if (context.mode === 'tiptap' && this.tiptapEditor) {
            this.tiptapEditor.chain().focus().insertContent(suggestion).run();
            this.syncMarkdownFromRichEditor();
            return;
        }

        const beforePrompt = context.text.slice(0, context.promptStart);
        const afterCursor = context.text.slice(context.cursor);
        const nextValue = `${beforePrompt}${suggestion}${afterCursor}`;
        this.markdownEditor.value = nextValue;
        const nextCursor = beforePrompt.length + suggestion.length;
        this.markdownEditor.focus();
        this.markdownEditor.setSelectionRange(nextCursor, nextCursor);
        this.syncRichEditorFromMarkdown();
    }

    clearAutoComplete() {
        this.autoCompleteSuggestions = [];
        this.autoCompleteActiveIndex = -1;
        this.autoCompleteContext = null;
        if (this.autoCompleteSuggestionsPanel) {
            this.autoCompleteSuggestionsPanel.classList.remove('active');
            this.autoCompleteSuggestionsPanel.innerHTML = '';
        }
    }

    resolveExplicitWikiQuery() {
        const selectionStart = this.markdownEditor.selectionStart;
        const selectionEnd = this.markdownEditor.selectionEnd;
        if (
            Number.isInteger(selectionStart)
            && Number.isInteger(selectionEnd)
            && selectionEnd > selectionStart
        ) {
            const selected = this.markdownEditor.value.slice(selectionStart, selectionEnd).trim();
            if (selected.length >= 2) {
                return selected.slice(0, 180);
            }
        }

        const context = this.getLinkSuggestionContext();
        if (context?.prompt?.length >= 2) {
            return context.prompt;
        }

        const title = (document.getElementById('node-title')?.value || '').trim();
        if (title.length >= 2) {
            return title.slice(0, 180);
        }

        const body = (this.markdownEditor.value || '').trim().slice(0, 180);
        return body.length >= 2 ? body : null;
    }

    getLinkSuggestionContext() {
        const wikiContext = this.getWikiLinkContext();
        if (wikiContext) {
            return wikiContext;
        }
        return this.getMentionLinkContext();
    }

    getWikiLinkContext() {
        if (this.editorMode !== 'markdown') {
            return null;
        }
        const text = this.markdownEditor.value || '';
        const cursor = Number.isInteger(this.markdownEditor.selectionStart)
            ? this.markdownEditor.selectionStart
            : text.length;
        const beforeCursor = text.slice(0, cursor);
        const wikiStart = beforeCursor.lastIndexOf('[[');
        if (wikiStart < 0) {
            return null;
        }

        const afterOpen = beforeCursor.slice(wikiStart + 2);
        if (afterOpen.includes(']]')) {
            return null;
        }
        if (/\n/.test(afterOpen)) {
            return null;
        }

        const pipeIndex = afterOpen.indexOf('|');
        const querySegment = pipeIndex >= 0 ? afterOpen.slice(0, pipeIndex) : afterOpen;
        const aliasSegment = pipeIndex >= 0 ? afterOpen.slice(pipeIndex + 1) : '';
        const prompt = querySegment.trim();
        return {
            text,
            cursor,
            wikiStart,
            promptStart: wikiStart + 2,
            prompt,
            alias: aliasSegment.trim(),
            hasAlias: pipeIndex >= 0
        };
    }

    getMentionLinkContext() {
        if (this.editorMode !== 'markdown') {
            return null;
        }
        const text = this.markdownEditor.value || '';
        const cursor = Number.isInteger(this.markdownEditor.selectionStart)
            ? this.markdownEditor.selectionStart
            : text.length;
        const beforeCursor = text.slice(0, cursor);
        const mentionStart = beforeCursor.lastIndexOf('@');
        if (mentionStart < 0) {
            return null;
        }

        const boundary = mentionStart > 0 ? beforeCursor.charAt(mentionStart - 1) : '';
        if (boundary && /[A-Za-z0-9._-]/.test(boundary)) {
            return null;
        }

        const afterMarker = beforeCursor.slice(mentionStart + 1);
        if (/\n/.test(afterMarker)) {
            return null;
        }

        if (afterMarker.startsWith('"') || afterMarker.startsWith('\'')) {
            const quote = afterMarker.charAt(0);
            const quoted = afterMarker.slice(1);
            if (quoted.includes(quote)) {
                return null;
            }
            return {
                kind: 'mention',
                text,
                cursor,
                mentionStart,
                promptStart: mentionStart + 2,
                prompt: quoted.trim(),
                quoted: true,
                quote
            };
        }

        if (/[^A-Za-z0-9._-]/.test(afterMarker)) {
            return null;
        }

        return {
            kind: 'mention',
            text,
            cursor,
            mentionStart,
            promptStart: mentionStart + 1,
            prompt: afterMarker.trim(),
            quoted: false,
            quote: null
        };
    }

    slugifyMentionToken(value) {
        return String(value || '')
            .toLowerCase()
            .replace(/['"]/g, '')
            .replace(/[^a-z0-9._-]+/g, '-')
            .replace(/^-+/, '')
            .replace(/-+$/, '')
            .slice(0, 120);
    }

    buildMentionSuggestionToken(title, context) {
        const normalizedTitle = String(title || '').trim();
        if (!normalizedTitle) {
            return '';
        }

        if (!context?.quoted && /^[A-Za-z0-9._-]+$/.test(normalizedTitle)) {
            return `@${normalizedTitle}`;
        }

        const preferredQuote = context?.quote === '\'' ? '\'' : '"';
        const alternateQuote = preferredQuote === '"' ? '\'' : '"';
        if (!normalizedTitle.includes(preferredQuote)) {
            return `@${preferredQuote}${normalizedTitle}${preferredQuote}`;
        }
        if (!normalizedTitle.includes(alternateQuote)) {
            return `@${alternateQuote}${normalizedTitle}${alternateQuote}`;
        }

        const slug = this.slugifyMentionToken(normalizedTitle);
        return slug ? `@${slug}` : '';
    }

    async requestWikiLinkSuggestions(silent = false, explicitQuery = null) {
        let context = this.getLinkSuggestionContext();
        const query = String(explicitQuery || context?.prompt || '').trim();

        if (query.length < 2) {
            this.clearWikiLinkSuggestions();
            return;
        }

        if (!context) {
            context = {
                kind: 'manual',
                text: this.markdownEditor.value || '',
                cursor: this.markdownEditor.selectionStart || 0,
                wikiStart: -1,
                mentionStart: -1,
                promptStart: -1,
                prompt: query
            };
        }
        this.wikiLinkContext = context;
        if (context.wikiStart >= 0 || context.mentionStart >= 0) {
            this.clearAutoComplete();
        }

        const namespace = document.getElementById('node-namespace').value.trim() || undefined;
        const payload = {
            text: query,
            limit: 6,
            namespace,
            exclude_node_id: this.currentEditingNode?.id || undefined
        };

        try {
            const response = await this.apiCall('/api/v1/assist/links', {
                method: 'POST',
                body: JSON.stringify(payload)
            });
            this.renderWikiLinkSuggestions(response.suggestions || [], query, context.kind || 'wiki');
        } catch (error) {
            if (!silent) {
                this.showNotification('AI link suggestions are currently unavailable.', 'warning');
            }
            this.clearWikiLinkSuggestions();
        }
    }

    renderWikiLinkSuggestions(suggestions, prompt, contextKind = 'wiki') {
        if (!this.linkSuggestionsPanel) {
            return;
        }

        this.linkSuggestionsPanel.innerHTML = '';
        if (!Array.isArray(suggestions) || suggestions.length === 0) {
            this.linkSuggestionsPanel.classList.remove('active');
            this.wikiLinkSuggestions = [];
            this.wikiLinkActiveIndex = -1;
            return;
        }

        this.wikiLinkSuggestions = suggestions
            .map((item) => ({
                node_id: item.node_id,
                title: String(item.title || '').trim(),
                heading: String(item.heading || '').trim(),
                preview: String(item.preview || '').trim(),
                namespace: String(item.namespace || '').trim() || 'default',
                score: Number(item.score || 0),
                reason: String(item.reason || '').trim() || 'semantic_match'
            }))
            .filter((item) => item.title.length > 0);

        if (this.wikiLinkSuggestions.length === 0) {
            this.linkSuggestionsPanel.classList.remove('active');
            this.wikiLinkActiveIndex = -1;
            return;
        }

        this.wikiLinkActiveIndex = 0;
        const header = document.createElement('div');
        header.className = 'link-suggestions-header';
        const headerLabel = contextKind === 'mention' ? 'Mention suggestions' : 'Link suggestions';
        header.textContent = `${headerLabel} for "${prompt}"`;
        this.linkSuggestionsPanel.appendChild(header);

        this.wikiLinkSuggestions.forEach((suggestion, index) => {
            const item = document.createElement('button');
            item.type = 'button';
            item.className = `link-suggestion-item ${index === this.wikiLinkActiveIndex ? 'active' : ''}`;
            item.dataset.suggestionIndex = String(index);

            const title = document.createElement('div');
            title.className = 'link-suggestion-title';
            const titleLabel = suggestion.heading
                ? `${suggestion.title} # ${suggestion.heading}`
                : suggestion.title;
            title.textContent = titleLabel;

            const meta = document.createElement('div');
            meta.className = 'link-suggestion-meta';
            const confidence = this.linkScoreConfidence(suggestion.score);
            const confidenceClass = confidence.toLowerCase();
            const reasonLabel = suggestion.reason
                .replaceAll('_', ' ')
                .replace(/\s+/g, ' ')
                .trim();
            meta.innerHTML = `
                <span class="link-suggestion-confidence ${confidenceClass}">${this.escapeHtml(confidence)}</span>
                <span>${this.escapeHtml(suggestion.namespace)} • ${this.escapeHtml(reasonLabel)}</span>
            `;

            const preview = document.createElement('div');
            preview.className = 'link-suggestion-preview';
            preview.textContent = suggestion.preview || '';

            item.appendChild(title);
            item.appendChild(meta);
            if (suggestion.preview) {
                item.appendChild(preview);
            }
            item.addEventListener('click', () => this.acceptWikiLinkByIndex(index));

            this.linkSuggestionsPanel.appendChild(item);
        });

        this.linkSuggestionsPanel.classList.add('active');
    }

    linkScoreConfidence(scoreRaw) {
        const score = Number(scoreRaw);
        if (!Number.isFinite(score)) {
            return 'Low';
        }
        if (score >= 2.5) {
            return 'High';
        }
        if (score >= 1.4) {
            return 'Medium';
        }
        return 'Low';
    }

    moveWikiLinkSelection(direction) {
        if (this.wikiLinkSuggestions.length === 0) {
            return;
        }
        const size = this.wikiLinkSuggestions.length;
        this.wikiLinkActiveIndex = (this.wikiLinkActiveIndex + direction + size) % size;
        this.refreshWikiLinkSelection();
    }

    refreshWikiLinkSelection() {
        const items = this.linkSuggestionsPanel?.querySelectorAll('.link-suggestion-item') || [];
        items.forEach((item, index) => {
            item.classList.toggle('active', index === this.wikiLinkActiveIndex);
        });
    }

    acceptActiveWikiLinkSuggestion() {
        this.acceptWikiLinkByIndex(this.wikiLinkActiveIndex >= 0 ? this.wikiLinkActiveIndex : 0);
    }

    acceptWikiLinkByIndex(index) {
        if (!Array.isArray(this.wikiLinkSuggestions) || index < 0 || index >= this.wikiLinkSuggestions.length) {
            return;
        }
        const suggestion = this.wikiLinkSuggestions[index];
        this.insertWikiLinkSuggestion(suggestion);
    }

    insertWikiLinkSuggestion(suggestion) {
        const title = String(suggestion?.title || '').trim();
        if (!title) {
            return;
        }
        const context = this.getLinkSuggestionContext() || this.wikiLinkContext;
        if (context?.kind === 'mention') {
            const mention = this.buildMentionSuggestionToken(title, context);
            if (!mention) {
                this.showNotification('Unable to build mention token for this suggestion.', 'warning');
                return;
            }

            if (context.mentionStart >= 0) {
                const beforeStart = context.text.slice(0, context.mentionStart);
                const afterCursor = context.text.slice(context.cursor);
                const nextValue = `${beforeStart}${mention}${afterCursor}`;
                const nextCursor = beforeStart.length + mention.length;
                this.markdownEditor.value = nextValue;
                this.markdownEditor.focus();
                this.markdownEditor.setSelectionRange(nextCursor, nextCursor);
            } else {
                const current = (this.markdownEditor.value || '').trimEnd();
                const separator = current.length > 0 ? '\n\n' : '';
                const nextValue = `${current}${separator}${mention}`;
                this.markdownEditor.value = nextValue;
                const nextCursor = current.length + separator.length + mention.length;
                this.markdownEditor.focus();
                this.markdownEditor.setSelectionRange(nextCursor, nextCursor);
            }

            this.syncRichEditorFromMarkdown();
            this.clearAutoComplete();
            this.clearWikiLinkSuggestions();
            this.showNotification('Mention inserted.', 'success');
            return;
        }

        const heading = String(suggestion?.heading || '').trim();
        const target = heading.length > 0 ? `${title}#${heading}` : title;
        let insertedLink = `[[${target}]]`;
        let cursorOffset = insertedLink.length;
        if (context?.hasAlias) {
            const alias = String(context.alias || '').trim();
            if (alias.length > 0) {
                insertedLink = `[[${target}|${alias}]]`;
                cursorOffset = insertedLink.length;
            } else {
                insertedLink = `[[${target}|]]`;
                cursorOffset = insertedLink.length - 2;
            }
        }

        if (context && context.wikiStart >= 0) {
            const beforeStart = context.text.slice(0, context.wikiStart);
            const afterCursor = context.text.slice(context.cursor);
            const nextValue = `${beforeStart}${insertedLink}${afterCursor}`;
            const nextCursor = beforeStart.length + cursorOffset;
            this.markdownEditor.value = nextValue;
            this.markdownEditor.focus();
            this.markdownEditor.setSelectionRange(nextCursor, nextCursor);
        } else {
            const current = (this.markdownEditor.value || '').trimEnd();
            const separator = current.length > 0 ? '\n\n' : '';
            const nextValue = `${current}${separator}${insertedLink}`;
            this.markdownEditor.value = nextValue;
            const nextCursor = current.length + separator.length + cursorOffset;
            this.markdownEditor.focus();
            this.markdownEditor.setSelectionRange(nextCursor, nextCursor);
        }

        this.syncRichEditorFromMarkdown();
        this.clearAutoComplete();
        this.clearWikiLinkSuggestions();
        this.showNotification('Wiki link inserted.', 'success');
    }

    clearWikiLinkSuggestions() {
        this.wikiLinkSuggestions = [];
        this.wikiLinkContext = null;
        this.wikiLinkActiveIndex = -1;
        this.hideWysiwygWikiLinkSuggestions();
        if (this.linkSuggestionsPanel) {
            this.linkSuggestionsPanel.classList.remove('active');
            this.linkSuggestionsPanel.innerHTML = '';
        }
    }

    switchTab(tabId) {
        document.querySelectorAll('.tab').forEach((tab) => tab.classList.remove('active'));
        document.getElementById(tabId).classList.add('active');

        document.querySelectorAll('.panel').forEach((panel) => panel.classList.remove('active'));
        const panelId = tabId.replace('-tab', '-panel');
        document.getElementById(panelId).classList.add('active');

        if (tabId === 'stats-tab') {
            this.loadStats();
        }

        if (tabId === 'daily-tab') {
            this.loadDailyNotes();
            this.loadDueTasks();
            this.loadFocusTasks();
        }

        if (tabId === 'calendar-tab') {
            this.loadCalendarItems();
        }

        if (tabId === 'templates-tab') {
            this.loadTemplatePacks();
            this.loadTemplates();
        }

        if (tabId === 'access-tab') {
            this.loadPermissionTemplates();
            this.loadAccessKeys();
        }
    }

    todayDateString() {
        const now = new Date();
        const month = String(now.getMonth() + 1).padStart(2, '0');
        const day = String(now.getDate()).padStart(2, '0');
        return `${now.getFullYear()}-${month}-${day}`;
    }

    nowDatetimeLocalString() {
        const now = new Date();
        const year = now.getFullYear();
        const month = String(now.getMonth() + 1).padStart(2, '0');
        const day = String(now.getDate()).padStart(2, '0');
        const hours = String(now.getHours()).padStart(2, '0');
        const minutes = String(now.getMinutes()).padStart(2, '0');
        return `${year}-${month}-${day}T${hours}:${minutes}`;
    }

    timestampForFilename() {
        const now = new Date();
        const year = String(now.getFullYear());
        const month = String(now.getMonth() + 1).padStart(2, '0');
        const day = String(now.getDate()).padStart(2, '0');
        const hours = String(now.getHours()).padStart(2, '0');
        const minutes = String(now.getMinutes()).padStart(2, '0');
        const seconds = String(now.getSeconds()).padStart(2, '0');
        return `${year}${month}${day}-${hours}${minutes}${seconds}`;
    }

    activeTabId() {
        return document.querySelector('.tab.active')?.id || '';
    }

    setSelectValueWithFallback(selectId, value, fallback) {
        const select = document.getElementById(selectId);
        if (!select) {
            return;
        }

        const normalized = (value || '').toString().toLowerCase();
        const optionValues = Array.from(select.options).map((option) => option.value);
        if (normalized && !optionValues.includes(normalized)) {
            const custom = document.createElement('option');
            custom.value = normalized;
            custom.textContent = `${normalized} (custom)`;
            select.appendChild(custom);
        }

        select.value = normalized || fallback;
    }

    toDatetimeLocalFromIso(isoValue) {
        if (!isoValue) {
            return '';
        }
        const date = new Date(isoValue);
        if (Number.isNaN(date.getTime())) {
            return '';
        }

        const year = date.getFullYear();
        const month = String(date.getMonth() + 1).padStart(2, '0');
        const day = String(date.getDate()).padStart(2, '0');
        const hours = String(date.getHours()).padStart(2, '0');
        const minutes = String(date.getMinutes()).padStart(2, '0');
        return `${year}-${month}-${day}T${hours}:${minutes}`;
    }

    toIsoFromDatetimeLocal(localValue) {
        if (!localValue) {
            return null;
        }
        const parsed = new Date(localValue);
        if (Number.isNaN(parsed.getTime())) {
            return null;
        }
        return parsed.toISOString();
    }

    buildSchedulingMetadata(kind, baseMetadata = {}) {
        const metadata = { ...(baseMetadata || {}) };
        const dueAtInput = document.getElementById('node-due-at').value.trim();
        const recurrenceFrequency = document.getElementById('node-recurrence-frequency').value;
        const recurrenceIntervalRaw = parseInt(
            document.getElementById('node-recurrence-interval').value,
            10
        );
        const recurrenceInterval = Number.isFinite(recurrenceIntervalRaw)
            ? Math.min(Math.max(recurrenceIntervalRaw, 1), 365)
            : 1;
        const dueAtIso = this.toIsoFromDatetimeLocal(dueAtInput);

        if (dueAtIso) {
            metadata.task_due_at = dueAtIso;
        } else {
            delete metadata.task_due_at;
        }

        const normalizedKind = (kind || '').toLowerCase();
        if (normalizedKind === 'task' && recurrenceFrequency !== 'none') {
            metadata.task_recurrence = {
                frequency: recurrenceFrequency,
                interval: recurrenceInterval,
                enabled: true
            };
        } else {
            delete metadata.task_recurrence;
            delete metadata.task_recurrence_last_generated_at;
            delete metadata.task_recurrence_generated_count;
        }

        return metadata;
    }

    applySchedulingInputsFromMetadata(node) {
        const metadata = node?.metadata || {};
        const dueAt = this.toDatetimeLocalFromIso(metadata.task_due_at);
        document.getElementById('node-due-at').value = dueAt;

        const recurrence = metadata.task_recurrence || {};
        const frequency = recurrence.frequency || 'none';
        document.getElementById('node-recurrence-frequency').value = frequency;
        document.getElementById('node-recurrence-interval').value = String(
            Number.isFinite(recurrence.interval) ? Math.min(Math.max(recurrence.interval, 1), 365) : 1
        );
    }

    selectedDailyDate() {
        const value = document.getElementById('daily-date').value.trim();
        return value || this.todayDateString();
    }

    selectedDailyNamespace() {
        return document.getElementById('daily-namespace').value.trim() || 'journal';
    }

    selectedDueBeforeIso() {
        const dueBeforeInput = document.getElementById('due-before');
        const value = dueBeforeInput ? dueBeforeInput.value.trim() : '';
        return this.toIsoFromDatetimeLocal(value);
    }

    includeCompletedDueTasks() {
        const includeCompleted = document.getElementById('due-include-completed');
        return Boolean(includeCompleted && includeCompleted.checked);
    }

    focusLimit() {
        const limitInput = document.getElementById('focus-limit');
        if (!limitInput) {
            return 8;
        }
        const parsed = Number.parseInt(limitInput.value || '8', 10);
        if (!Number.isFinite(parsed)) {
            return 8;
        }
        return Math.min(Math.max(parsed, 1), 50);
    }

    includeCompletedFocusTasks() {
        const includeCompleted = document.getElementById('focus-include-completed');
        return Boolean(includeCompleted && includeCompleted.checked);
    }

    includeWithoutDueFocusTasks() {
        const includeNoDue = document.getElementById('focus-include-no-due');
        return Boolean(includeNoDue && includeNoDue.checked);
    }

    parseCommaSeparatedInput(rawValue) {
        return String(rawValue || '')
            .split(',')
            .map((item) => item.trim())
            .filter((item) => item.length > 0);
    }

    templateVariablesFromNode(template) {
        const metadataVariables = template?.metadata?.template_variables;
        if (Array.isArray(metadataVariables)) {
            return metadataVariables
                .map((value) => String(value || '').trim())
                .filter((value) => value.length > 0);
        }
        return [];
    }

    sortNodesByCreatedDesc(nodes) {
        return [...nodes].sort((left, right) => {
            const leftTime = Date.parse(left?.temporal?.created_at || left?.created_at || 0) || 0;
            const rightTime = Date.parse(right?.temporal?.created_at || right?.created_at || 0) || 0;
            return rightTime - leftTime;
        });
    }

    isDailyNoteNode(node) {
        const tags = Array.isArray(node?.tags) ? node.tags : [];
        return tags.some((tag) => String(tag).toLowerCase() === 'daily-note');
    }

    isTemplateNode(node) {
        const tags = Array.isArray(node?.tags) ? node.tags : [];
        const metadataFlag = Boolean(node?.metadata?.template);
        return metadataFlag || tags.some((tag) => String(tag).toLowerCase() === 'template');
    }

    isTaskCompleted(node) {
        return Boolean(node?.metadata?.task_completed);
    }

    formatDateTime(isoValue) {
        if (!isoValue) {
            return 'n/a';
        }
        const date = new Date(isoValue);
        if (Number.isNaN(date.getTime())) {
            return String(isoValue);
        }
        return date.toLocaleString();
    }

    setDailyLinkedItemsPlaceholder(message) {
        if (!this.dailyLinkedItems) {
            return;
        }
        this.dailyLinkedItems.innerHTML = `<p>${this.escapeHtml(message)}</p>`;
    }

    async apiCall(endpoint, options = {}) {
        const { silent = false, ...requestOptions } = options;
        const url = `${this.apiBase}${endpoint}`;
        const config = {
            headers: {
                'Content-Type': 'application/json',
                ...(requestOptions.headers || {})
            },
            ...requestOptions
        };

        try {
            const response = await fetch(url, config);
            if (!response.ok) {
                throw new Error(`HTTP ${response.status}: ${response.statusText}`);
            }

            if (response.status === 204) {
                return null;
            }
            return await response.json();
        } catch (error) {
            if (!silent) {
                this.showNotification(`API Error: ${error.message}`, 'error');
            }
            throw error;
        }
    }

    formatNodeDate(node) {
        const timestamp = node.temporal?.created_at || node.created_at;
        if (!timestamp) {
            return 'Unknown date';
        }
        return new Date(timestamp).toLocaleDateString();
    }

    async loadNodes() {
        try {
            const params = new URLSearchParams({
                limit: this.pageSize,
                offset: (this.currentPage - 1) * this.pageSize,
                ...this.currentFilters
            });

            const data = await this.apiCall(`/api/v1/nodes?${params.toString()}`);
            this.renderNodes(data);
            this.updatePagination(data.length === this.pageSize);
        } catch (error) {
            console.error('Failed to load nodes:', error);
        }
    }

    renderNodes(nodes) {
        const container = document.getElementById('nodes-list');
        container.innerHTML = '';

        if (!Array.isArray(nodes) || nodes.length === 0) {
            container.innerHTML = '<div class="node-item"><p>No nodes found.</p></div>';
            return;
        }

        nodes.forEach((node) => {
            const nodeElement = this.createNodeElement(node);
            container.appendChild(nodeElement);
        });
    }

    createNodeElement(node) {
        const div = document.createElement('div');
        div.className = 'node-item';
        div.onclick = () => this.editNode(node);

        const tagsHtml = (node.tags || []).map((tag) => `<span class="node-tag">${this.escapeHtml(tag)}</span>`).join('');
        const preview = this.escapeHtml((node.content || '').slice(0, 420));

        div.innerHTML = `
            <div class="node-header">
                <div>
                    <div class="node-title">${this.escapeHtml(node.title || 'Untitled')}</div>
                    <div class="node-meta">
                        <span>${this.escapeHtml(node.kind || 'unknown')}</span>
                        <span>${this.escapeHtml(node.namespace || 'default')}</span>
                        <span>${this.formatNodeDate(node)}</span>
                        <span>Importance: ${Number(node.importance || 0).toFixed(2)}</span>
                    </div>
                </div>
            </div>
            <div class="node-content">${preview}</div>
            <div class="node-tags">${tagsHtml}</div>
        `;

        return div;
    }

    updatePagination(hasMore) {
        const prevBtn = document.getElementById('prev-page-btn');
        const nextBtn = document.getElementById('next-page-btn');
        const pageInfo = document.getElementById('page-info');

        prevBtn.disabled = this.currentPage === 1;
        nextBtn.disabled = !hasMore;
        pageInfo.textContent = `Page ${this.currentPage}`;
    }

    changePage(delta) {
        this.currentPage += delta;
        if (this.currentPage < 1) {
            this.currentPage = 1;
        }
        this.loadNodes();
    }

    applyFilters() {
        const kind = document.getElementById('node-kind-filter').value;
        const namespace = document.getElementById('node-namespace-filter').value.trim();

        this.currentFilters = {};
        if (kind) {
            this.currentFilters.kind = kind;
        }
        if (namespace) {
            this.currentFilters.namespace = namespace;
        }

        this.currentPage = 1;
        this.loadNodes();
    }

    async performSearch(options = {}) {
        const query = document.getElementById('search-query').value.trim();
        if (!query) {
            this.showNotification('Please enter a search query.', 'warning');
            return;
        }

        const strategy = document.getElementById('search-strategy').value;
        const limit = parseInt(document.getElementById('search-limit').value, 10) || 10;
        const params = new URLSearchParams({
            q: query,
            type: strategy,
            limit: String(limit)
        });

        try {
            const data = await this.apiCall(`/api/v1/search?${params.toString()}`);
            this.renderSearchResults(data);
            if (!options.fromSavedSearch && this.activeSavedSearchId) {
                this.activeSavedSearchId = null;
                this.renderSavedSearches();
            }
        } catch (error) {
            console.error('Search failed:', error);
        }
    }

    renderSearchResults(results) {
        const container = document.getElementById('search-results');
        container.innerHTML = '';

        if (!Array.isArray(results) || results.length === 0) {
            container.innerHTML = '<p>No results found.</p>';
            return;
        }

        results.forEach((result) => {
            const div = document.createElement('div');
            div.className = 'search-result-item';

            const tagsHtml = (result.node.tags || []).map((tag) => `<span class="node-tag">${this.escapeHtml(tag)}</span>`).join('');
            const preview = this.escapeHtml((result.node.content || '').slice(0, 420));

            div.innerHTML = `
                <div class="search-result-score">Score: ${(Number(result.score || 0) * 100).toFixed(1)}%</div>
                <div class="search-result-match">${this.escapeHtml(result.match_source || 'unknown')}</div>
                <div class="node-title">${this.escapeHtml(result.node.title || 'Untitled')}</div>
                <div class="node-meta">
                    <span>${this.escapeHtml(result.node.kind || 'unknown')}</span>
                    <span>${this.escapeHtml(result.node.namespace || 'default')}</span>
                    <span>${this.formatNodeDate(result.node)}</span>
                </div>
                <div class="node-content">${preview}</div>
                <div class="node-tags">${tagsHtml}</div>
            `;

            container.appendChild(div);
        });
    }

    currentSearchPayload() {
        const query = document.getElementById('search-query').value.trim();
        const searchType = document.getElementById('search-strategy').value;
        const parsedLimit = parseInt(document.getElementById('search-limit').value, 10);
        const limit = Number.isFinite(parsedLimit)
            ? Math.min(Math.max(parsedLimit, 1), 200)
            : 10;
        return { query, searchType, limit };
    }

    parseOptionalUnitIntervalInput(inputId) {
        const input = document.getElementById(inputId);
        const raw = input ? input.value.trim() : '';
        if (!raw) {
            return null;
        }

        const parsed = Number.parseFloat(raw);
        if (!Number.isFinite(parsed)) {
            return null;
        }
        return Math.min(Math.max(parsed, 0), 1);
    }

    normalizeSavedSearchKinds(items) {
        const seen = new Set();
        const normalized = [];
        items.forEach((item) => {
            const value = String(item || '').trim().toLowerCase();
            if (!value || seen.has(value)) {
                return;
            }
            seen.add(value);
            normalized.push(value);
        });
        return normalized;
    }

    normalizeSavedSearchTags(items) {
        const seen = new Set();
        const normalized = [];
        items.forEach((item) => {
            const value = String(item || '').trim();
            const key = value.toLowerCase();
            if (!value || seen.has(key)) {
                return;
            }
            seen.add(key);
            normalized.push(value);
        });
        return normalized;
    }

    savedSearchFiltersFromForm(includeNulls = false) {
        const targetNamespaceInput = document.getElementById('saved-search-target-namespace');
        const rawTargetNamespace = targetNamespaceInput ? targetNamespaceInput.value.trim() : '';
        const kinds = this.normalizeSavedSearchKinds(
            this.parseCommaSeparatedInput(document.getElementById('saved-search-kinds')?.value || '')
        );
        const tags = this.normalizeSavedSearchTags(
            this.parseCommaSeparatedInput(document.getElementById('saved-search-tags')?.value || '')
        );
        const minScore = this.parseOptionalUnitIntervalInput('saved-search-min-score');
        const minImportance = this.parseOptionalUnitIntervalInput('saved-search-min-importance');

        const payload = {};
        if (includeNulls || rawTargetNamespace.length > 0) {
            payload.target_namespace = rawTargetNamespace || null;
        }
        if (includeNulls || kinds.length > 0) {
            payload.kinds = kinds;
        }
        if (includeNulls || tags.length > 0) {
            payload.tags = tags;
        }
        if (includeNulls || minScore !== null) {
            payload.min_score = minScore;
        }
        if (includeNulls || minImportance !== null) {
            payload.min_importance = minImportance;
        }
        return payload;
    }

    savedSearchDescriptionFromForm(includeNulls = false) {
        const descriptionInput = document.getElementById('saved-search-description');
        const description = descriptionInput ? descriptionInput.value.trim() : '';
        if (description.length > 0) {
            return description;
        }
        return includeNulls ? null : undefined;
    }

    defaultSavedSearchName(query) {
        const cleaned = query.trim();
        if (!cleaned) {
            return 'Saved Search';
        }
        return cleaned.length > 56 ? `${cleaned.slice(0, 56)}...` : cleaned;
    }

    async saveCurrentSearch() {
        const payload = this.currentSearchPayload();
        if (!payload.query) {
            this.showNotification('Enter a search query before saving.', 'warning');
            return;
        }

        const nameInput = document.getElementById('saved-search-name');
        const rawName = nameInput.value.trim();
        const request = {
            name: rawName || this.defaultSavedSearchName(payload.query),
            query: payload.query,
            search_type: payload.searchType,
            limit: payload.limit,
            description: this.savedSearchDescriptionFromForm(false),
            ...this.savedSearchFiltersFromForm(false)
        };

        try {
            const created = await this.apiCall('/api/v1/search/saved', {
                method: 'POST',
                body: JSON.stringify(request)
            });
            this.activeSavedSearchId = created.id;
            await this.loadSavedSearches();
            nameInput.value = '';
            this.showNotification('Saved search created.', 'success');
        } catch (error) {
            console.error('Failed to save search:', error);
            this.showNotification(`Could not save search: ${error.message}`, 'error');
        }
    }

    async updateActiveSavedSearch() {
        if (!this.activeSavedSearchId) {
            this.showNotification('Load or run a saved search first.', 'warning');
            return;
        }

        const payload = this.currentSearchPayload();
        if (!payload.query) {
            this.showNotification('Saved search query cannot be empty.', 'warning');
            return;
        }

        const rawName = document.getElementById('saved-search-name')?.value.trim();
        const request = {
            name: rawName || this.defaultSavedSearchName(payload.query),
            query: payload.query,
            search_type: payload.searchType,
            limit: payload.limit,
            description: this.savedSearchDescriptionFromForm(true),
            ...this.savedSearchFiltersFromForm(true)
        };

        try {
            const updated = await this.apiCall(`/api/v1/search/saved/${encodeURIComponent(this.activeSavedSearchId)}`, {
                method: 'PUT',
                body: JSON.stringify(request)
            });
            this.activeSavedSearchId = updated?.id || this.activeSavedSearchId;
            await this.loadSavedSearches();
            if (updated) {
                this.loadSavedSearchIntoForm(updated);
            }
            this.showNotification('Saved search updated.', 'success');
        } catch (error) {
            console.error('Failed to update saved search:', error);
            this.showNotification(`Could not update saved search: ${error.message}`, 'error');
        }
    }

    async loadSavedSearches() {
        if (!this.savedSearchesList) {
            return;
        }

        try {
            const items = await this.apiCall('/api/v1/search/saved?limit=100');
            this.savedSearchesCache = Array.isArray(items) ? items : [];
        } catch (error) {
            console.error('Failed to load saved searches:', error);
            this.savedSearchesCache = [];
            this.showNotification(`Could not load saved searches: ${error.message}`, 'error');
        }

        this.renderSavedSearches();
    }

    renderSavedSearches() {
        if (!this.savedSearchesList) {
            return;
        }

        if (!Array.isArray(this.savedSearchesCache) || this.savedSearchesCache.length === 0) {
            this.savedSearchesList.innerHTML = '<p class="saved-search-empty">No saved searches yet.</p>';
            return;
        }

        this.savedSearchesList.innerHTML = '';
        this.savedSearchesCache.forEach((item) => {
            const wrapper = document.createElement('div');
            wrapper.className = 'saved-search-item';
            if (this.activeSavedSearchId && item.id === this.activeSavedSearchId) {
                wrapper.classList.add('active');
            }

            const kinds = Array.isArray(item.kinds) && item.kinds.length > 0
                ? item.kinds.join(', ')
                : 'all kinds';
            const tags = Array.isArray(item.tags) && item.tags.length > 0
                ? item.tags.join(', ')
                : 'all tags';
            const scope = item.target_namespace || item.namespace || 'default';
            const minScore = Number.isFinite(Number(item.min_score)) ? Number(item.min_score).toFixed(2) : 'none';
            const minImportance = Number.isFinite(Number(item.min_importance)) ? Number(item.min_importance).toFixed(2) : 'none';
            const description = String(item.description || '').trim();

            const info = document.createElement('div');
            info.innerHTML = `
                <div class="node-title">${this.escapeHtml(item.name || 'Saved Search')}</div>
                ${description ? `<div class="saved-search-description">${this.escapeHtml(description)}</div>` : ''}
                <div class="saved-search-meta">
                    <span>strategy: ${this.escapeHtml(item.search_type || 'hybrid')}</span>
                    <span>limit: ${this.escapeHtml(String(item.limit || 10))}</span>
                    <span>scope: ${this.escapeHtml(scope)}</span>
                    <span>kinds: ${this.escapeHtml(kinds)}</span>
                    <span>tags: ${this.escapeHtml(tags)}</span>
                    <span>min score: ${this.escapeHtml(minScore)}</span>
                    <span>min importance: ${this.escapeHtml(minImportance)}</span>
                </div>
            `;

            const actions = document.createElement('div');
            actions.className = 'saved-search-actions';
            const runBtn = document.createElement('button');
            runBtn.type = 'button';
            runBtn.className = 'btn primary';
            runBtn.textContent = 'Run';
            runBtn.addEventListener('click', () => this.runSavedSearch(item.id));

            const loadBtn = document.createElement('button');
            loadBtn.type = 'button';
            loadBtn.className = 'btn';
            loadBtn.textContent = 'Load';
            loadBtn.addEventListener('click', () => this.loadSavedSearchIntoForm(item));

            const deleteBtn = document.createElement('button');
            deleteBtn.type = 'button';
            deleteBtn.className = 'btn danger';
            deleteBtn.textContent = 'Delete';
            deleteBtn.addEventListener('click', () => this.deleteSavedSearch(item.id));

            actions.appendChild(runBtn);
            actions.appendChild(loadBtn);
            actions.appendChild(deleteBtn);

            wrapper.appendChild(info);
            wrapper.appendChild(actions);
            this.savedSearchesList.appendChild(wrapper);
        });
    }

    loadSavedSearchIntoForm(savedSearch) {
        if (!savedSearch || !savedSearch.id) {
            return;
        }
        document.getElementById('search-query').value = savedSearch.query || '';
        document.getElementById('search-strategy').value = savedSearch.search_type || 'hybrid';
        document.getElementById('search-limit').value = String(savedSearch.limit || 10);
        const nameInput = document.getElementById('saved-search-name');
        if (nameInput) {
            nameInput.value = savedSearch.name || '';
        }
        const descriptionInput = document.getElementById('saved-search-description');
        if (descriptionInput) {
            descriptionInput.value = savedSearch.description || '';
        }
        const targetNamespaceInput = document.getElementById('saved-search-target-namespace');
        if (targetNamespaceInput) {
            targetNamespaceInput.value = savedSearch.target_namespace || '';
        }
        const kindsInput = document.getElementById('saved-search-kinds');
        if (kindsInput) {
            kindsInput.value = Array.isArray(savedSearch.kinds) ? savedSearch.kinds.join(', ') : '';
        }
        const tagsInput = document.getElementById('saved-search-tags');
        if (tagsInput) {
            tagsInput.value = Array.isArray(savedSearch.tags) ? savedSearch.tags.join(', ') : '';
        }
        const minScoreInput = document.getElementById('saved-search-min-score');
        if (minScoreInput) {
            minScoreInput.value = Number.isFinite(Number(savedSearch.min_score))
                ? String(Number(savedSearch.min_score))
                : '';
        }
        const minImportanceInput = document.getElementById('saved-search-min-importance');
        if (minImportanceInput) {
            minImportanceInput.value = Number.isFinite(Number(savedSearch.min_importance))
                ? String(Number(savedSearch.min_importance))
                : '';
        }
        this.activeSavedSearchId = savedSearch.id;
        this.renderSavedSearches();
    }

    async runSavedSearch(savedSearchId) {
        if (!savedSearchId) {
            return;
        }

        try {
            const response = await this.apiCall(`/api/v1/search/saved/${encodeURIComponent(savedSearchId)}/run`, {
                method: 'POST'
            });
            if (response?.saved_search) {
                this.loadSavedSearchIntoForm(response.saved_search);
            } else {
                this.activeSavedSearchId = savedSearchId;
                this.renderSavedSearches();
            }
            this.renderSearchResults(Array.isArray(response?.results) ? response.results : []);
        } catch (error) {
            console.error('Failed to run saved search:', error);
            this.showNotification(`Could not run saved search: ${error.message}`, 'error');
        }
    }

    async deleteSavedSearch(savedSearchId) {
        if (!savedSearchId) {
            return;
        }
        if (!window.confirm('Delete this saved search?')) {
            return;
        }

        try {
            await this.apiCall(`/api/v1/search/saved/${encodeURIComponent(savedSearchId)}`, {
                method: 'DELETE'
            });
            this.savedSearchesCache = this.savedSearchesCache.filter((item) => item.id !== savedSearchId);
            if (this.activeSavedSearchId === savedSearchId) {
                this.activeSavedSearchId = null;
            }
            this.renderSavedSearches();
            this.showNotification('Saved search deleted.', 'success');
        } catch (error) {
            console.error('Failed to delete saved search:', error);
            this.showNotification(`Could not delete saved search: ${error.message}`, 'error');
        }
    }

    async loadGraph() {
        const nodeId = document.getElementById('graph-node-id').value.trim();
        const depth = parseInt(document.getElementById('graph-depth').value, 10) || 2;

        if (!nodeId) {
            this.showNotification('Please enter a node ID.', 'warning');
            return;
        }

        try {
            const data = await this.apiCall(`/api/v1/graph/neighbors/${nodeId}?depth=${depth}`);
            this.renderGraph(data, nodeId);
        } catch (error) {
            console.error('Failed to load graph:', error);
        }
    }

    renderGraph(neighbors, centerNodeId) {
        const container = document.getElementById('graph-container');
        container.innerHTML = '';

        if (!Array.isArray(neighbors) || neighbors.length === 0) {
            container.innerHTML = '<p>No relationships found.</p>';
            return;
        }

        const graphHtml = `
            <div class="graph-center">
                <strong>Center Node:</strong> ${this.escapeHtml(centerNodeId)}
            </div>
            <div class="graph-neighbors">
                <strong>Connected Nodes (${neighbors.length}):</strong>
                <ul>
                    ${neighbors.map((id) => `<li>${this.escapeHtml(id)}</li>`).join('')}
                </ul>
            </div>
        `;

        container.innerHTML = graphHtml;
    }

    selectedCalendarNamespace() {
        return document.getElementById('calendar-namespace').value.trim() || 'default';
    }

    selectedCalendarView() {
        return document.getElementById('calendar-view').value || 'week';
    }

    selectedCalendarAnchorDate() {
        return document.getElementById('calendar-anchor').value.trim() || this.todayDateString();
    }

    selectedCalendarAnchorIso() {
        return `${this.selectedCalendarAnchorDate()}T12:00:00Z`;
    }

    includeCompletedCalendarTasks() {
        const includeCompleted = document.getElementById('calendar-include-completed');
        return Boolean(includeCompleted && includeCompleted.checked);
    }

    shiftCalendarAnchor(direction) {
        const anchorInput = document.getElementById('calendar-anchor');
        if (!anchorInput) {
            return;
        }
        const currentRaw = this.selectedCalendarAnchorDate();
        const anchor = new Date(`${currentRaw}T00:00:00Z`);
        if (Number.isNaN(anchor.getTime())) {
            anchorInput.value = this.todayDateString();
            this.loadCalendarItems();
            return;
        }

        const view = this.selectedCalendarView();
        if (view === 'month') {
            anchor.setUTCMonth(anchor.getUTCMonth() + direction);
        } else if (view === 'day') {
            anchor.setUTCDate(anchor.getUTCDate() + direction);
        } else {
            anchor.setUTCDate(anchor.getUTCDate() + (7 * direction));
        }

        anchorInput.value = anchor.toISOString().slice(0, 10);
        this.loadCalendarItems();
    }

    resetCalendarAnchorToToday() {
        const anchorInput = document.getElementById('calendar-anchor');
        if (!anchorInput) {
            return;
        }
        anchorInput.value = this.todayDateString();
        this.loadCalendarItems();
    }

    setCalendarRangeSummaryPlaceholder(message) {
        if (!this.calendarRangeSummary) {
            return;
        }
        this.calendarRangeSummary.innerHTML = `<p>${this.escapeHtml(message)}</p>`;
    }

    setCalendarItemsPlaceholder(message) {
        if (!this.calendarItemsList) {
            return;
        }
        this.calendarItemsList.innerHTML = `<p>${this.escapeHtml(message)}</p>`;
    }

    buildCalendarQueryParams(limit = 250) {
        return new URLSearchParams({
            namespace: this.selectedCalendarNamespace(),
            view: this.selectedCalendarView(),
            anchor: this.selectedCalendarAnchorIso(),
            limit: String(limit),
            include_completed: this.includeCompletedCalendarTasks() ? 'true' : 'false'
        });
    }

    async loadCalendarItems() {
        this.setCalendarRangeSummaryPlaceholder('Loading calendar range...');
        this.setCalendarItemsPlaceholder('Loading scheduled items...');

        const params = this.buildCalendarQueryParams(250);

        try {
            const response = await this.apiCall(`/api/v1/calendar/items?${params.toString()}`);
            const items = Array.isArray(response?.items) ? response.items : [];
            this.calendarItemsCache = items;
            this.renderCalendarRangeSummary(response);
            this.renderCalendarItems(items);
        } catch (error) {
            console.error('Failed to load calendar items:', error);
            this.setCalendarRangeSummaryPlaceholder('Unable to load calendar range right now.');
            this.setCalendarItemsPlaceholder('Unable to load scheduled items right now.');
        }
    }

    async exportCalendarIcal() {
        const params = this.buildCalendarQueryParams(500);
        try {
            const response = await fetch(`${this.apiBase}/api/v1/calendar/ical?${params.toString()}`);
            if (!response.ok) {
                throw new Error(`HTTP ${response.status}: ${response.statusText}`);
            }

            const blob = await response.blob();
            const rawDisposition = response.headers.get('content-disposition') || '';
            const filenameMatch = rawDisposition.match(/filename="([^"]+)"/i);
            const fallback = `mindvault-calendar-${this.timestampForFilename()}.ics`;
            const filename = filenameMatch?.[1] || fallback;

            const objectUrl = window.URL.createObjectURL(blob);
            const anchor = document.createElement('a');
            anchor.href = objectUrl;
            anchor.download = filename;
            anchor.click();
            window.URL.revokeObjectURL(objectUrl);

            this.showNotification('Calendar iCal export downloaded.', 'success');
        } catch (error) {
            console.error('Failed to export calendar iCal:', error);
            this.showNotification(`Calendar export failed: ${error.message}`, 'error');
        }
    }

    async importCalendarIcal(fileList) {
        const input = document.getElementById('import-calendar-ical-file');
        const file = fileList && fileList[0];
        if (!file) {
            return;
        }

        const overwriteExisting = Boolean(
            document.getElementById('calendar-import-overwrite')?.checked
        );
        const maxBytes = 2 * 1024 * 1024;
        if (file.size > maxBytes) {
            this.showNotification('iCal file too large (max 2MB).', 'error');
            if (input) {
                input.value = '';
            }
            return;
        }

        const formData = new FormData();
        formData.append('file', file, file.name || 'calendar.ics');
        formData.append('namespace', this.selectedCalendarNamespace());
        formData.append('overwrite_existing', overwriteExisting ? 'true' : 'false');
        formData.append('default_kind', 'event');

        try {
            const response = await fetch(`${this.apiBase}/api/v1/calendar/ical/import`, {
                method: 'POST',
                body: formData
            });
            if (!response.ok) {
                throw new Error(`HTTP ${response.status}: ${response.statusText}`);
            }

            const result = await response.json();
            this.showNotification(
                `iCal import complete: ${Number(result.imported_nodes || 0)} created, ${Number(result.updated_nodes || 0)} updated, ${Number(result.skipped_events || 0)} skipped.`,
                'success'
            );
            this.loadCalendarItems();
            this.loadNodes();
            this.loadDueTasks();
            this.loadFocusTasks();
            this.loadStats();
        } catch (error) {
            console.error('Failed to import calendar iCal:', error);
            this.showNotification(`Calendar import failed: ${error.message}`, 'error');
        } finally {
            if (input) {
                input.value = '';
            }
        }
    }

    renderCalendarRangeSummary(response) {
        if (!this.calendarRangeSummary) {
            return;
        }
        const view = this.escapeHtml(String(response?.view || this.selectedCalendarView()));
        const rangeStart = this.escapeHtml(this.formatDateTime(response?.range_start));
        const rangeEnd = this.escapeHtml(this.formatDateTime(response?.range_end));
        const shownCount = Number(response?.returned_items || 0);
        const totalCount = Number(response?.total_items || 0);

        this.calendarRangeSummary.innerHTML = `
            <div class="calendar-range-meta">
                <span><strong>${view.toUpperCase()}</strong> window</span>
                <span>${rangeStart} → ${rangeEnd}</span>
                <span>Showing ${shownCount} of ${totalCount} item(s)</span>
            </div>
        `;
    }

    renderCalendarItems(items) {
        if (!this.calendarItemsList) {
            return;
        }
        this.calendarItemsList.innerHTML = '';

        if (!Array.isArray(items) || items.length === 0) {
            this.setCalendarItemsPlaceholder('No scheduled tasks or events in this range.');
            return;
        }

        items.forEach((item) => {
            const node = item?.node || {};
            const isCompleted = Boolean(item?.completed);
            const scheduleStart = this.formatDateTime(item?.scheduled_at);
            const scheduleEnd = this.formatDateTime(item?.scheduled_end_at);
            const timeLabel = item?.scheduled_end_at
                ? `${scheduleStart} → ${scheduleEnd}`
                : scheduleStart;
            const recurrence = node?.metadata?.task_recurrence;
            const recurrenceLabel = recurrence
                ? `Recurring ${String(recurrence.frequency || '').toLowerCase() || 'custom'}`
                : 'One-time';
            const card = document.createElement('div');
            card.className = `calendar-item ${isCompleted ? 'is-completed' : ''}`;

            card.innerHTML = `
                <div class="calendar-item-head">
                    <div class="calendar-item-title">${this.escapeHtml(node.title || node.content || 'Untitled item')}</div>
                    <span class="calendar-item-kind">${this.escapeHtml(node.kind || 'unknown')}</span>
                </div>
                <div class="calendar-item-meta">
                    <span>${this.escapeHtml(timeLabel)}</span>
                    <span>${this.escapeHtml(node.namespace || 'default')}</span>
                    <span>${this.escapeHtml(item?.schedule_source || 'schedule')}</span>
                    ${node.kind === 'task' ? `<span>${this.escapeHtml(recurrenceLabel)}</span>` : ''}
                </div>
                <div class="calendar-item-actions">
                    <span>Importance ${(Number(node.importance || 0)).toFixed(2)}</span>
                    <button type="button" class="btn calendar-item-open-btn">Open</button>
                </div>
            `;

            const openButton = card.querySelector('.calendar-item-open-btn');
            if (openButton) {
                openButton.addEventListener('click', (event) => {
                    event.stopPropagation();
                    this.editNode(node);
                });
            }

            card.addEventListener('click', () => this.editNode(node));
            this.calendarItemsList.appendChild(card);
        });
    }

    startCalendarEventCapture() {
        this.showAddNodeModal();
        this.setSelectValueWithFallback('node-kind', 'event', 'event');
        document.getElementById('node-namespace').value = this.selectedCalendarNamespace();
        document.getElementById('node-due-at').value = `${this.selectedCalendarAnchorDate()}T09:00`;
        this.showNotification('New event draft opened from calendar context.', 'success');
    }

    async loadDailyNotes() {
        const namespace = this.selectedDailyNamespace();
        const params = new URLSearchParams({
            namespace,
            limit: '30',
            offset: '0'
        });

        try {
            const notes = await this.apiCall(`/api/v1/daily-notes?${params.toString()}`);
            this.dailyNotesCache = Array.isArray(notes) ? notes : [];
            this.renderDailyNotes(this.dailyNotesCache);

            if (this.dailyNotesCache.length === 0) {
                this.selectedDailyNoteId = null;
                this.setDailyLinkedItemsPlaceholder('No linked items to show until a daily note exists.');
                return;
            }

            const selected = this.dailyNotesCache.find((note) => note.id === this.selectedDailyNoteId) || this.dailyNotesCache[0];
            this.selectedDailyNoteId = selected.id;
            this.renderDailyNotes(this.dailyNotesCache);
            this.loadLinkedItemsForDailyNote(selected);
        } catch (error) {
            console.error('Failed to load daily notes:', error);
        }
    }

    setDueTasksPlaceholder(message) {
        if (!this.dueTasksList) {
            return;
        }
        this.dueTasksList.innerHTML = `<p>${this.escapeHtml(message)}</p>`;
    }

    async loadDueTasks() {
        this.setDueTasksPlaceholder('Loading due tasks...');
        const params = new URLSearchParams({
            namespace: this.selectedDailyNamespace(),
            limit: '80',
            include_completed: this.includeCompletedDueTasks() ? 'true' : 'false'
        });
        const beforeIso = this.selectedDueBeforeIso();
        if (beforeIso) {
            params.set('before', beforeIso);
        }

        try {
            const tasks = await this.apiCall(`/api/v1/tasks/due?${params.toString()}`);
            this.dueTasksCache = Array.isArray(tasks) ? tasks : [];
            this.renderDueTasks(this.dueTasksCache);
        } catch (error) {
            console.error('Failed to load due tasks:', error);
            this.setDueTasksPlaceholder('Unable to load due tasks right now.');
        }
    }

    renderDueTasks(tasks) {
        if (!this.dueTasksList) {
            return;
        }
        this.dueTasksList.innerHTML = '';

        if (!Array.isArray(tasks) || tasks.length === 0) {
            this.setDueTasksPlaceholder('No due tasks for the selected filters.');
            return;
        }

        tasks.forEach((task) => {
            const isCompleted = this.isTaskCompleted(task);
            const dueAt = task?.metadata?.task_due_at || null;
            const recurrence = task?.metadata?.task_recurrence;
            const recurrenceLabel = recurrence
                ? `Recurring ${String(recurrence.frequency || '').toLowerCase() || 'custom'}`
                : 'One-time';

            const card = document.createElement('div');
            card.className = `due-task-item ${isCompleted ? 'is-completed' : ''}`;

            card.innerHTML = `
                <div class="due-task-head">
                    <div class="due-task-title">${this.escapeHtml(task.title || task.content || 'Untitled task')}</div>
                    <label>
                        <input type="checkbox" data-task-complete-id="${this.escapeHtml(task.id)}" ${isCompleted ? 'checked' : ''}>
                        Done
                    </label>
                </div>
                <div class="due-task-meta">
                    <span>Due ${this.escapeHtml(this.formatDateTime(dueAt))}</span>
                    <span>${this.escapeHtml(task.namespace || 'default')}</span>
                    <span>${this.escapeHtml(recurrenceLabel)}</span>
                </div>
                <div class="due-task-actions">
                    <span>Importance ${(Number(task.importance || 0)).toFixed(2)}</span>
                    <button type="button" class="btn due-task-open-btn" data-task-open-id="${this.escapeHtml(task.id)}">Open</button>
                </div>
            `;

            const completeToggle = card.querySelector('[data-task-complete-id]');
            if (completeToggle) {
                completeToggle.addEventListener('click', (event) => event.stopPropagation());
                completeToggle.addEventListener('change', (event) => {
                    const target = event.target;
                    this.toggleTaskCompletion(task.id, target.checked);
                });
            }

            const openButton = card.querySelector('[data-task-open-id]');
            if (openButton) {
                openButton.addEventListener('click', (event) => {
                    event.stopPropagation();
                    this.editNode(task);
                });
            }

            card.addEventListener('click', () => this.editNode(task));
            this.dueTasksList.appendChild(card);
        });
    }

    setFocusTasksPlaceholder(message) {
        if (!this.focusTasksList) {
            return;
        }
        this.focusTasksList.innerHTML = `<p>${this.escapeHtml(message)}</p>`;
    }

    async loadFocusTasks() {
        if (!this.focusTasksList) {
            return;
        }
        this.setFocusTasksPlaceholder('Loading focus list...');
        const payload = {
            namespace: this.selectedDailyNamespace(),
            limit: this.focusLimit(),
            include_completed: this.includeCompletedFocusTasks(),
            include_without_due: this.includeWithoutDueFocusTasks()
        };

        try {
            const response = await this.apiCall('/api/v1/tasks/prioritize', {
                method: 'POST',
                body: JSON.stringify(payload)
            });
            this.focusTasksCache = Array.isArray(response?.items) ? response.items : [];
            if (this.focusGeneratedAt) {
                if (response?.generated_at) {
                    this.focusGeneratedAt.textContent = `Generated ${this.formatDateTime(response.generated_at)}`;
                } else {
                    this.focusGeneratedAt.textContent = 'Not generated yet.';
                }
            }
            this.renderFocusTasks(this.focusTasksCache);
        } catch (error) {
            console.error('Failed to load focus tasks:', error);
            this.setFocusTasksPlaceholder('Unable to generate focus list right now.');
        }
    }

    renderFocusTasks(items) {
        if (!this.focusTasksList) {
            return;
        }
        this.focusTasksList.innerHTML = '';

        if (!Array.isArray(items) || items.length === 0) {
            this.setFocusTasksPlaceholder('No focus tasks found for the selected filters.');
            return;
        }

        items.forEach((item) => {
            const task = item.task || {};
            const dueAt = task?.metadata?.task_due_at || null;
            const reason = item.reason || 'Balanced priority';

            const card = document.createElement('div');
            card.className = 'focus-task-item';
            card.innerHTML = `
                <div class="focus-task-head">
                    <div>
                        <div class="focus-task-rank">#${this.escapeHtml(String(item.rank || 0))}</div>
                        <div class="focus-task-score">Score ${(Number(item.score || 0)).toFixed(2)}</div>
                    </div>
                    <button type="button" class="btn focus-task-open-btn" data-focus-open-id="${this.escapeHtml(task.id)}">Open</button>
                </div>
                <div class="focus-task-title">${this.escapeHtml(task.title || task.content || 'Untitled task')}</div>
                <div class="focus-task-reason">${this.escapeHtml(reason)}</div>
                <div class="focus-task-meta">
                    <span>${this.escapeHtml(dueAt ? `Due ${this.formatDateTime(dueAt)}` : 'No due date')}</span>
                    <span>${this.escapeHtml(task.namespace || 'default')}</span>
                    <span>Importance ${(Number(task.importance || 0)).toFixed(2)}</span>
                </div>
            `;

            const openButton = card.querySelector('[data-focus-open-id]');
            if (openButton) {
                openButton.addEventListener('click', (event) => {
                    event.stopPropagation();
                    this.editNode(task);
                });
            }

            card.addEventListener('click', () => this.editNode(task));
            this.focusTasksList.appendChild(card);
        });
    }

    async toggleTaskCompletion(taskId, completed) {
        try {
            const action = completed ? 'complete' : 'reopen';
            await this.apiCall(`/api/v1/tasks/${encodeURIComponent(taskId)}/${action}`, {
                method: 'POST'
            });

            this.showNotification(
                completed ? 'Task marked completed.' : 'Task marked active.',
                'success'
            );
            this.loadDueTasks();
            this.loadFocusTasks();
            this.loadNodes();
        } catch (error) {
            console.error('Failed to toggle task completion:', error);
            this.showNotification('Unable to update task completion right now.', 'error');
            this.loadDueTasks();
            this.loadFocusTasks();
        }
    }

    renderDailyNotes(notes) {
        const container = document.getElementById('daily-notes-list');
        container.innerHTML = '';

        if (!Array.isArray(notes) || notes.length === 0) {
            container.innerHTML = '<div class="node-item"><p>No daily notes found for this namespace.</p></div>';
            return;
        }

        notes.forEach((note) => {
            const card = document.createElement('div');
            card.className = 'daily-note-card';
            if (note.id === this.selectedDailyNoteId) {
                card.classList.add('active');
            }
            card.addEventListener('click', () => this.selectDailyNote(note));

            const dayTag = (note.tags || []).find((tag) => tag.startsWith('day:')) || '';
            const preview = this.escapeHtml((note.content || '').slice(0, 240));
            card.innerHTML = `
                <div class="daily-note-card-header">
                    <div class="daily-note-card-title">${this.escapeHtml(note.title || 'Daily Note')}</div>
                    <div class="daily-note-card-date">${this.escapeHtml(dayTag.replace('day:', '') || this.formatNodeDate(note))}</div>
                </div>
                <div class="daily-note-card-content">${preview}</div>
            `;
            container.appendChild(card);
        });
    }

    selectDailyNote(note) {
        this.selectedDailyNoteId = note.id;
        this.renderDailyNotes(this.dailyNotesCache);
        this.loadLinkedItemsForDailyNote(note);
        this.editNode(note);
    }

    async loadLinkedItemsForDailyNote(note) {
        if (!note?.id) {
            this.setDailyLinkedItemsPlaceholder('Select a daily note card to inspect linked tasks/events.');
            return;
        }

        this.setDailyLinkedItemsPlaceholder('Loading linked items...');
        try {
            const neighbors = await this.apiCall(`/api/v1/graph/neighbors/${encodeURIComponent(note.id)}?depth=1`);
            if (!Array.isArray(neighbors) || neighbors.length === 0) {
                this.setDailyLinkedItemsPlaceholder('No linked items found for this daily note yet.');
                return;
            }

            const nodeLookups = await Promise.all(
                neighbors.slice(0, 80).map(async (id) => {
                    try {
                        return await this.apiCall(`/api/v1/nodes/${encodeURIComponent(id)}`);
                    } catch (_error) {
                        return null;
                    }
                })
            );

            const nodes = nodeLookups
                .filter((node) => node && node.id && !this.isDailyNoteNode(node))
                .sort((left, right) => Number(right.importance || 0) - Number(left.importance || 0));

            this.renderLinkedItems(nodes);
        } catch (error) {
            console.error('Failed to load linked items:', error);
            this.setDailyLinkedItemsPlaceholder('Unable to load linked items right now.');
        }
    }

    renderLinkedItems(nodes) {
        if (!this.dailyLinkedItems) {
            return;
        }
        this.dailyLinkedItems.innerHTML = '';

        if (!Array.isArray(nodes) || nodes.length === 0) {
            this.setDailyLinkedItemsPlaceholder('No linked items found for this daily note yet.');
            return;
        }

        nodes.forEach((node) => {
            const item = document.createElement('div');
            item.className = 'daily-linked-item';
            item.addEventListener('click', () => this.editNode(node));

            item.innerHTML = `
                <div class="daily-linked-item-title">${this.escapeHtml(node.title || 'Untitled')}</div>
                <div class="daily-linked-item-meta">
                    <span>${this.escapeHtml(node.kind || 'unknown')}</span>
                    <span>${this.escapeHtml(node.namespace || 'default')}</span>
                    <span>${this.formatNodeDate(node)}</span>
                    <span>Importance ${(Number(node.importance || 0)).toFixed(2)}</span>
                </div>
            `;
            this.dailyLinkedItems.appendChild(item);
        });
    }

    async ensureDailyNoteForDate(date) {
        const namespace = this.selectedDailyNamespace();
        const payload = { namespace, date };

        try {
            const response = await this.apiCall('/api/v1/daily-notes/ensure', {
                method: 'POST',
                body: JSON.stringify(payload)
            });
            if (response.created) {
                this.showNotification(`Daily note created for ${date}.`, 'success');
            } else {
                this.showNotification(`Daily note already exists for ${date}.`, 'info');
            }
            this.loadDailyNotes();
            this.loadDueTasks();
            this.loadFocusTasks();
            this.editNode(response.node);
        } catch (error) {
            console.error('Failed to ensure daily note:', error);
        }
    }

    async ensureTodayDailyNote() {
        await this.ensureDailyNoteForDate(this.todayDateString());
    }

    async ensureSelectedDailyNote() {
        await this.ensureDailyNoteForDate(this.selectedDailyDate());
    }

    async loadTemplatePacks() {
        try {
            const packs = await this.apiCall('/api/v1/template-packs');
            this.templatePacksCache = Array.isArray(packs) ? packs : [];
            this.renderTemplatePacks(this.templatePacksCache);
        } catch (error) {
            console.error('Failed to load template packs:', error);
            this.templatePacksCache = [];
            this.renderTemplatePacks([]);
            this.showNotification('Unable to load template packs right now.', 'error');
        }
    }

    renderTemplatePacks(packs) {
        const container = document.getElementById('template-packs-list');
        if (!container) {
            return;
        }
        container.innerHTML = '';

        const packItems = Array.isArray(packs) ? packs : [];
        if (packItems.length === 0) {
            container.innerHTML = '<p class="template-empty">No curated packs available.</p>';
            return;
        }

        packItems.forEach((pack) => {
            const card = document.createElement('div');
            card.className = 'template-pack-card';
            card.innerHTML = `
                <div class="template-pack-head">
                    <div class="template-pack-name">${this.escapeHtml(pack.name || pack.pack_id || 'Untitled Pack')}</div>
                    <div class="template-pack-id">${this.escapeHtml(pack.pack_id || 'unknown')}</div>
                </div>
                <div class="template-pack-description">${this.escapeHtml(pack.description || '')}</div>
                <div class="template-pack-meta">
                    <span>Templates: ${Number(pack.template_count || 0)}</span>
                </div>
                <div class="template-pack-actions">
                    <button type="button" class="btn primary template-pack-install-btn" data-template-pack-id="${this.escapeHtml(pack.pack_id)}">Install</button>
                </div>
            `;
            const installButton = card.querySelector('[data-template-pack-id]');
            if (installButton) {
                installButton.addEventListener('click', () => this.installTemplatePack(pack.pack_id));
            }
            container.appendChild(card);
        });
    }

    async installTemplatePack(packId) {
        if (!packId) {
            return;
        }
        const namespace = document.getElementById('template-pack-namespace')?.value?.trim() || 'default';
        const overwriteExisting = Boolean(document.getElementById('template-pack-overwrite')?.checked);
        const additionalTags = this.parseCommaSeparatedInput(
            document.getElementById('template-pack-tags')?.value || ''
        );

        const payload = {
            namespace,
            overwrite_existing: overwriteExisting
        };
        if (additionalTags.length > 0) {
            payload.additional_tags = additionalTags;
        }

        try {
            const result = await this.apiCall(
                `/api/v1/template-packs/${encodeURIComponent(packId)}/install`,
                {
                    method: 'POST',
                    body: JSON.stringify(payload)
                }
            );
            this.showNotification(
                `Pack installed: +${result.installed_templates} new, ${result.updated_templates} updated, ${result.skipped_templates} skipped.`,
                'success'
            );
            await this.loadTemplates();
        } catch (error) {
            console.error('Failed to install template pack:', error);
            this.showNotification('Unable to install template pack.', 'error');
        }
    }

    async loadTemplates() {
        const namespace = document.getElementById('template-namespace-filter').value.trim();
        const kind = document.getElementById('template-kind-filter').value.trim();
        const params = new URLSearchParams({
            limit: '100',
            offset: '0'
        });
        if (namespace) {
            params.set('namespace', namespace);
        }
        if (kind) {
            params.set('kind', kind);
        }

        try {
            const templates = await this.apiCall(`/api/v1/templates?${params.toString()}`);
            this.templatesCache = Array.isArray(templates) ? this.sortNodesByCreatedDesc(templates) : [];
            if (this.templatesCache.length === 0) {
                this.selectedTemplateId = null;
            } else if (!this.templatesCache.some((template) => template.id === this.selectedTemplateId)) {
                this.selectedTemplateId = this.templatesCache[0].id;
            }

            this.renderTemplates(this.templatesCache);
            this.renderSelectedTemplateDetails();
        } catch (error) {
            console.error('Failed to load templates:', error);
            this.showNotification('Unable to load templates right now.', 'error');
        }
    }

    renderTemplates(templates) {
        if (!this.templatesList) {
            return;
        }
        this.templatesList.innerHTML = '';

        if (!Array.isArray(templates) || templates.length === 0) {
            this.templatesList.innerHTML = '<p class="template-empty">No templates found for current filters.</p>';
            return;
        }

        templates.forEach((template) => {
            const card = document.createElement('div');
            card.className = 'template-card';
            if (template.id === this.selectedTemplateId) {
                card.classList.add('active');
            }

            const variables = this.templateVariablesFromNode(template);
            const key = String(template?.metadata?.template_key || '').trim();
            const preview = this.escapeHtml((template.content || '').slice(0, 180));
            const variablePreview = variables.length > 0 ? variables.join(', ') : 'none';

            card.innerHTML = `
                <div class="template-card-head">
                    <div class="template-card-title">${this.escapeHtml(template.title || 'Untitled Template')}</div>
                    <span class="template-card-kind">${this.escapeHtml(template.kind || 'unknown')}</span>
                </div>
                <div class="template-card-meta">
                    <span>${this.escapeHtml(template.namespace || 'default')}</span>
                    <span>vars: ${this.escapeHtml(String(variables.length))}</span>
                    ${key ? `<span>key: ${this.escapeHtml(key)}</span>` : ''}
                </div>
                <div class="template-card-content">${preview}</div>
                <div class="template-card-vars">Variables: ${this.escapeHtml(variablePreview)}</div>
                <div class="template-card-actions">
                    <button type="button" class="btn template-open-btn" data-template-open-id="${this.escapeHtml(template.id)}">Open</button>
                    <button type="button" class="btn primary template-use-btn" data-template-use-id="${this.escapeHtml(template.id)}">Use</button>
                    <button type="button" class="btn template-duplicate-btn" data-template-duplicate-id="${this.escapeHtml(template.id)}">Duplicate</button>
                    <button type="button" class="btn danger template-delete-btn" data-template-delete-id="${this.escapeHtml(template.id)}">Delete</button>
                </div>
            `;

            card.addEventListener('click', () => this.selectTemplate(template.id));
            const openButton = card.querySelector('[data-template-open-id]');
            if (openButton) {
                openButton.addEventListener('click', (event) => {
                    event.stopPropagation();
                    this.openTemplateInEditor(template.id);
                });
            }
            const useButton = card.querySelector('[data-template-use-id]');
            if (useButton) {
                useButton.addEventListener('click', (event) => {
                    event.stopPropagation();
                    this.selectTemplate(template.id);
                    const instantiateButton = document.getElementById('instantiate-template-btn');
                    if (instantiateButton) {
                        instantiateButton.focus();
                    }
                });
            }
            const duplicateButton = card.querySelector('[data-template-duplicate-id]');
            if (duplicateButton) {
                duplicateButton.addEventListener('click', (event) => {
                    event.stopPropagation();
                    this.duplicateTemplate(template.id);
                });
            }
            const deleteButton = card.querySelector('[data-template-delete-id]');
            if (deleteButton) {
                deleteButton.addEventListener('click', (event) => {
                    event.stopPropagation();
                    this.deleteTemplate(template.id);
                });
            }
            this.templatesList.appendChild(card);
        });
    }

    async loadPermissionTemplates() {
        try {
            const templates = await this.apiCall('/api/v1/permission-templates?limit=200&offset=0');
            this.permissionTemplatesCache = Array.isArray(templates) ? templates : [];
            this.renderPermissionTemplates(this.permissionTemplatesCache);
            this.refreshAccessKeyTemplateOptions();
        } catch (error) {
            console.error('Failed to load permission templates:', error);
            this.permissionTemplatesCache = [];
            this.renderPermissionTemplates([]);
            this.showNotification('Unable to load access templates.', 'error');
        }
    }

    renderPermissionTemplates(templates) {
        if (!this.permissionTemplatesList) {
            return;
        }
        this.permissionTemplatesList.innerHTML = '';

        if (!Array.isArray(templates) || templates.length === 0) {
            this.permissionTemplatesList.innerHTML = '<p class="template-empty">No permission templates yet.</p>';
            return;
        }

        templates.forEach((template) => {
            const card = document.createElement('div');
            card.className = 'access-card';

            card.innerHTML = `
                <div class="access-card-head">
                    <div>
                        <div class="access-card-title">${this.escapeHtml(template.name || 'Untitled')}</div>
                        <div class="access-card-meta">Tier: ${this.escapeHtml(template.tier || 'unknown')}</div>
                    </div>
                    <div class="access-card-meta">${this.escapeHtml(template.scope_namespace || 'all namespaces')}</div>
                </div>
                <div class="access-card-body">
                    <div class="access-card-line">Tags: ${this.escapeHtml((template.scope_tags || []).join(', ') || 'none')}</div>
                    <div class="access-card-line">Actions: ${this.escapeHtml((template.allow_actions || []).join(', ') || 'none')}</div>
                </div>
                <div class="access-card-actions">
                    <button type="button" class="btn" data-template-edit-id="${this.escapeHtml(template.id)}">Edit</button>
                    <button type="button" class="btn danger" data-template-delete-id="${this.escapeHtml(template.id)}">Delete</button>
                </div>
            `;

            const editButton = card.querySelector('[data-template-edit-id]');
            if (editButton) {
                editButton.addEventListener('click', (event) => {
                    event.stopPropagation();
                    this.populatePermissionTemplateForm(template);
                });
            }

            const deleteButton = card.querySelector('[data-template-delete-id]');
            if (deleteButton) {
                deleteButton.addEventListener('click', (event) => {
                    event.stopPropagation();
                    this.deletePermissionTemplate(template.id);
                });
            }

            this.permissionTemplatesList.appendChild(card);
        });
    }

    populatePermissionTemplateForm(template) {
        this.editingPermissionTemplateId = template.id;
        document.getElementById('permission-template-name').value = template.name || '';
        document.getElementById('permission-template-description').value = template.description || '';
        document.getElementById('permission-template-tier').value = template.tier || 'view';
        document.getElementById('permission-template-namespace').value = template.scope_namespace || '';
        document.getElementById('permission-template-tags').value = (template.scope_tags || []).join(', ');
        document.getElementById('permission-template-actions').value = (template.allow_actions || []).join(', ');
        document.getElementById('permission-template-submit').textContent = 'Update Template';
        document.getElementById('permission-template-cancel').classList.remove('is-hidden');
    }

    resetPermissionTemplateForm() {
        this.editingPermissionTemplateId = null;
        document.getElementById('permission-template-form').reset();
        document.getElementById('permission-template-submit').textContent = 'Create Template';
        document.getElementById('permission-template-cancel').classList.add('is-hidden');
    }

    async submitPermissionTemplate(event) {
        event.preventDefault();
        const name = document.getElementById('permission-template-name').value.trim();
        const description = document.getElementById('permission-template-description').value.trim();
        const tier = document.getElementById('permission-template-tier').value;
        const scopeNamespace = document.getElementById('permission-template-namespace').value.trim();
        const scopeTags = this.parseCommaSeparatedInput(document.getElementById('permission-template-tags').value);
        const allowActions = this.parseCommaSeparatedInput(document.getElementById('permission-template-actions').value);

        const payload = {
            name,
            description: description || null,
            tier,
            scope_namespace: scopeNamespace || null,
            scope_tags: scopeTags,
            allow_actions: allowActions
        };

        try {
            if (this.editingPermissionTemplateId) {
                await this.apiCall(`/api/v1/permission-templates/${encodeURIComponent(this.editingPermissionTemplateId)}`, {
                    method: 'PUT',
                    body: JSON.stringify(payload)
                });
                this.showNotification('Template updated.', 'success');
            } else {
                await this.apiCall('/api/v1/permission-templates', {
                    method: 'POST',
                    body: JSON.stringify(payload)
                });
                this.showNotification('Template created.', 'success');
            }
            this.resetPermissionTemplateForm();
            await this.loadPermissionTemplates();
        } catch (error) {
            console.error('Failed to save permission template:', error);
            this.showNotification('Unable to save template.', 'error');
        }
    }

    async deletePermissionTemplate(templateId) {
        if (!templateId) {
            return;
        }
        const confirmed = window.confirm('Delete this permission template?');
        if (!confirmed) {
            return;
        }

        try {
            await this.apiCall(`/api/v1/permission-templates/${encodeURIComponent(templateId)}`, {
                method: 'DELETE'
            });
            this.showNotification('Template deleted.', 'success');
            await this.loadPermissionTemplates();
        } catch (error) {
            console.error('Failed to delete permission template:', error);
            this.showNotification('Unable to delete template.', 'error');
        }
    }

    async loadAccessKeys() {
        try {
            const keys = await this.apiCall('/api/v1/access-keys');
            this.accessKeysCache = Array.isArray(keys) ? keys : [];
            this.renderAccessKeys(this.accessKeysCache);
        } catch (error) {
            console.error('Failed to load access keys:', error);
            this.accessKeysCache = [];
            this.renderAccessKeys([]);
            this.showNotification('Unable to load access keys.', 'error');
        }
    }

    renderAccessKeys(keys) {
        if (!this.accessKeysList) {
            return;
        }
        this.accessKeysList.innerHTML = '';
        if (!Array.isArray(keys) || keys.length === 0) {
            this.accessKeysList.innerHTML = '<p class="template-empty">No access keys yet.</p>';
            return;
        }

        keys.forEach((key) => {
            const card = document.createElement('div');
            card.className = 'access-card';
            const templateName = key.template_name || key.template_id;
            card.innerHTML = `
                <div class="access-card-head">
                    <div>
                        <div class="access-card-title">${this.escapeHtml(key.name || 'Untitled Key')}</div>
                        <div class="access-card-meta">Template: ${this.escapeHtml(templateName)}</div>
                    </div>
                    <div class="access-card-meta">${this.escapeHtml(key.revoked_at ? 'revoked' : 'active')}</div>
                </div>
                <div class="access-card-body">
                    <div class="access-card-line">Created: ${this.escapeHtml(this.formatDateTime(key.created_at))}</div>
                    <div class="access-card-line">Last used: ${this.escapeHtml(key.last_used_at ? this.formatDateTime(key.last_used_at) : 'never')}</div>
                    <div class="access-card-line">Expires: ${this.escapeHtml(key.expires_at || 'never')}</div>
                </div>
                <div class="access-card-actions">
                    <button type="button" class="btn danger" data-access-revoke-id="${this.escapeHtml(key.id)}">Revoke</button>
                </div>
            `;

            const revokeButton = card.querySelector('[data-access-revoke-id]');
            if (revokeButton) {
                revokeButton.addEventListener('click', (event) => {
                    event.stopPropagation();
                    this.revokeAccessKey(key.id);
                });
            }

            this.accessKeysList.appendChild(card);
        });
    }

    refreshAccessKeyTemplateOptions() {
        const select = document.getElementById('access-key-template');
        if (!select) {
            return;
        }
        select.innerHTML = '';
        if (!Array.isArray(this.permissionTemplatesCache) || this.permissionTemplatesCache.length === 0) {
            const option = document.createElement('option');
            option.value = '';
            option.textContent = 'No templates available';
            select.appendChild(option);
            return;
        }

        this.permissionTemplatesCache.forEach((template) => {
            const option = document.createElement('option');
            option.value = template.id;
            option.textContent = `${template.name} (${template.tier})`;
            select.appendChild(option);
        });
    }

    async createAccessKey(event) {
        event.preventDefault();
        const templateId = document.getElementById('access-key-template').value;
        const name = document.getElementById('access-key-name').value.trim();
        const expiresAt = document.getElementById('access-key-expires').value.trim();

        if (!templateId) {
            this.showNotification('Select a template first.', 'warning');
            return;
        }

        const payload = {
            template_id: templateId,
            name: name || null,
            expires_at: expiresAt || null
        };

        try {
            const response = await this.apiCall('/api/v1/access-keys', {
                method: 'POST',
                body: JSON.stringify(payload)
            });
            if (response?.token && this.accessKeyToken && this.accessKeyOutput) {
                this.accessKeyToken.textContent = response.token;
                this.accessKeyOutput.classList.remove('is-hidden');
            }
            this.showNotification('Access key created.', 'success');
            await this.loadAccessKeys();
        } catch (error) {
            console.error('Failed to create access key:', error);
            this.showNotification('Unable to create access key.', 'error');
        }
    }

    async revokeAccessKey(keyId) {
        const confirmed = window.confirm('Revoke this access key?');
        if (!confirmed) {
            return;
        }

        try {
            await this.apiCall(`/api/v1/access-keys/${encodeURIComponent(keyId)}`, {
                method: 'DELETE'
            });
            this.showNotification('Access key revoked.', 'success');
            await this.loadAccessKeys();
        } catch (error) {
            console.error('Failed to revoke access key:', error);
            this.showNotification('Unable to revoke access key.', 'error');
        }
    }

    selectTemplate(templateId) {
        if (!templateId) {
            return;
        }
        this.selectedTemplateId = templateId;
        this.renderTemplates(this.templatesCache);
        this.renderSelectedTemplateDetails();
    }

    selectedTemplate() {
        if (!this.selectedTemplateId) {
            return null;
        }
        return this.templatesCache.find((template) => template.id === this.selectedTemplateId) || null;
    }

    renderSelectedTemplateDetails() {
        const summary = document.getElementById('template-selection-summary');
        const instantiateButton = document.getElementById('instantiate-template-btn');
        const valuesContainer = document.getElementById('template-values-fields');
        const template = this.selectedTemplate();

        if (!template) {
            if (summary) {
                summary.innerHTML = '<p>Select a template card to configure instantiation.</p>';
            }
            if (instantiateButton) {
                instantiateButton.disabled = true;
            }
            if (valuesContainer) {
                valuesContainer.innerHTML = '';
            }
            this.templateHistoryCache = [];
            this.renderTemplateHistory([]);
            return;
        }

        const variables = this.templateVariablesFromNode(template);
        const key = String(template?.metadata?.template_key || '').trim();
        if (summary) {
            summary.innerHTML = `
                <div class="template-selection-title">${this.escapeHtml(template.title || 'Untitled Template')}</div>
                <div class="template-selection-meta">
                    <span>${this.escapeHtml(template.kind || 'unknown')}</span>
                    <span>${this.escapeHtml(template.namespace || 'default')}</span>
                    ${key ? `<span>key: ${this.escapeHtml(key)}</span>` : ''}
                    <span>variables: ${this.escapeHtml(String(variables.length))}</span>
                </div>
            `;
        }
        if (instantiateButton) {
            instantiateButton.disabled = false;
        }
        this.renderTemplateValueInputs(variables);
        this.loadTemplateHistory(template.id);
        const namespaceInput = document.getElementById('template-instance-namespace');
        if (namespaceInput && !namespaceInput.value.trim()) {
            namespaceInput.value = template.namespace || 'default';
        }
    }

    renderTemplateValueInputs(variables) {
        const valuesContainer = document.getElementById('template-values-fields');
        if (!valuesContainer) {
            return;
        }
        valuesContainer.innerHTML = '';

        const normalized = Array.isArray(variables) ? variables : [];
        if (normalized.length === 0) {
            valuesContainer.innerHTML = '<p class="template-empty">This template has no required placeholders.</p>';
            return;
        }

        const fragment = document.createDocumentFragment();
        normalized.forEach((name) => {
            const wrapper = document.createElement('div');
            wrapper.className = 'form-group';

            const label = document.createElement('label');
            label.textContent = `Value for {{${name}}}:`;

            const input = document.createElement('input');
            input.type = 'text';
            input.maxLength = 512;
            input.placeholder = `Enter ${name}`;
            input.setAttribute('data-template-variable', name);

            wrapper.appendChild(label);
            wrapper.appendChild(input);
            fragment.appendChild(wrapper);
        });
        valuesContainer.appendChild(fragment);
    }

    templateValuesPayload() {
        const valuesContainer = document.getElementById('template-values-fields');
        if (!valuesContainer) {
            return {};
        }
        const values = {};
        valuesContainer.querySelectorAll('input[data-template-variable]').forEach((input) => {
            const key = String(input.getAttribute('data-template-variable') || '').trim();
            if (!key) {
                return;
            }
            const value = input.value;
            if (value.length > 0) {
                values[key] = value;
            }
        });
        return values;
    }

    async loadTemplateHistory(templateId) {
        if (!templateId) {
            this.templateHistoryCache = [];
            this.renderTemplateHistory([]);
            return;
        }
        try {
            const history = await this.apiCall(`/api/v1/templates/${encodeURIComponent(templateId)}/versions`);
            this.templateHistoryCache = Array.isArray(history) ? history : [];
            this.renderTemplateHistory(this.templateHistoryCache);
        } catch (error) {
            console.error('Failed to load template history:', error);
            this.templateHistoryCache = [];
            this.renderTemplateHistory([]);
        }
    }

    renderTemplateHistory(historyItems) {
        const container = document.getElementById('template-history-list');
        const template = this.selectedTemplate();
        if (!container) {
            return;
        }
        container.innerHTML = '';

        const items = Array.isArray(historyItems) ? historyItems : [];
        if (!template) {
            container.innerHTML = '<p class="template-empty">Version history appears when a template is selected.</p>';
            return;
        }
        if (items.length === 0) {
            container.innerHTML = '<p class="template-empty">No saved versions yet. Edit and save the template to start history.</p>';
            return;
        }

        items.forEach((version) => {
            const item = document.createElement('div');
            item.className = 'template-history-item';
            const capturedAt = this.formatDateTime(version.captured_at);
            const title = version.title || 'Untitled';
            const preview = this.escapeHtml(version.content_preview || '');
            item.innerHTML = `
                <div class="template-history-head">
                    <div class="template-history-title">${this.escapeHtml(title)}</div>
                    <div class="template-history-time">${this.escapeHtml(capturedAt)}</div>
                </div>
                <div class="template-history-meta">
                    <span>${this.escapeHtml(version.kind || 'unknown')}</span>
                    <span>${this.escapeHtml(version.namespace || 'default')}</span>
                    <span>tags: ${this.escapeHtml(String(version.tag_count || 0))}</span>
                    <span>importance: ${Number(version.importance || 0).toFixed(2)}</span>
                </div>
                <div class="template-history-preview">${preview}</div>
                <div class="template-history-actions">
                    <button type="button" class="btn template-history-preview-btn" data-template-version-preview-id="${this.escapeHtml(version.version_id)}">Preview Diff</button>
                    <button type="button" class="btn template-history-restore-btn" data-template-version-id="${this.escapeHtml(version.version_id)}">Restore</button>
                </div>
            `;
            const previewButton = item.querySelector('[data-template-version-preview-id]');
            if (previewButton) {
                previewButton.addEventListener('click', () => {
                    this.previewTemplateVersion(template.id, version.version_id);
                });
            }
            const restoreButton = item.querySelector('[data-template-version-id]');
            if (restoreButton) {
                restoreButton.addEventListener('click', () => {
                    this.restoreTemplateVersion(template.id, version.version_id);
                });
            }
            container.appendChild(item);
        });
    }

    async previewTemplateVersion(templateId, versionId) {
        if (!templateId || !versionId) {
            return;
        }
        try {
            const detail = await this.apiCall(
                `/api/v1/templates/${encodeURIComponent(templateId)}/versions/${encodeURIComponent(versionId)}`
            );
            this.selectedVersionPreview = { entityType: 'template', entityId: templateId, versionId };
            this.renderTemplateVersionPreviewModal(detail);
            this.showTemplateVersionModal();
        } catch (error) {
            console.error('Failed to load template version preview:', error);
            this.showNotification('Unable to load version preview.', 'error');
        }
    }

    renderTemplateVersionPreviewModal(detail) {
        const modalTitle = document.getElementById('template-version-modal-title');
        const modalMeta = document.getElementById('template-version-modal-meta');
        const fieldChangesContainer = document.getElementById('template-version-field-changes');
        const removedLines = document.getElementById('template-version-removed-lines');
        const addedLines = document.getElementById('template-version-added-lines');
        const restoreButton = document.getElementById('template-version-modal-restore-btn');
        const entityLabel = this.selectedVersionPreview?.entityType === 'node' ? 'Node' : 'Template';

        const versionTitle = detail?.version?.title || `${entityLabel} Version`;
        if (modalTitle) {
            modalTitle.textContent = `${entityLabel} Version Preview: ${versionTitle}`;
        }

        const diff = detail?.diff || {};
        const capturedAt = this.formatDateTime(detail?.version?.captured_at);
        if (modalMeta) {
            modalMeta.innerHTML = `
                <span>Captured: ${this.escapeHtml(capturedAt)}</span>
                <span>Removed: ${Number(diff.removed_line_count || 0)}</span>
                <span>Added: ${Number(diff.added_line_count || 0)}</span>
                <span>Version Lines: ${Number(diff.version_line_count || 0)}</span>
                <span>Current Lines: ${Number(diff.current_line_count || 0)}</span>
            `;
        }

        if (fieldChangesContainer) {
            fieldChangesContainer.innerHTML = '';
            const fieldChanges = Array.isArray(detail?.field_changes) ? detail.field_changes : [];
            if (fieldChanges.length === 0) {
                fieldChangesContainer.innerHTML = '<p class="template-empty">No field-level changes detected.</p>';
            } else {
                const sorted = [...fieldChanges].sort((left, right) => Number(right.changed) - Number(left.changed));
                sorted.forEach((change) => {
                    const row = document.createElement('div');
                    row.className = `template-version-field-change-row ${change.changed ? 'changed' : 'unchanged'}`;
                    row.innerHTML = `
                        <div class="template-version-field-name">${this.escapeHtml(change.field || 'unknown')}</div>
                        <div class="template-version-field-values">
                            <div><strong>Version</strong>: ${this.escapeHtml(String(change.version_value || ''))}</div>
                            <div><strong>Current</strong>: ${this.escapeHtml(String(change.current_value || ''))}</div>
                        </div>
                    `;
                    fieldChangesContainer.appendChild(row);
                });
            }
        }

        const renderLines = (container, lines, emptyMessage, lineClass) => {
            if (!container) {
                return;
            }
            container.innerHTML = '';
            const normalized = Array.isArray(lines) ? lines : [];
            if (normalized.length === 0) {
                container.innerHTML = `<p class="template-empty">${this.escapeHtml(emptyMessage)}</p>`;
                return;
            }
            normalized.forEach((line) => {
                const row = document.createElement('pre');
                row.className = `template-version-diff-line ${lineClass}`;
                row.textContent = line;
                container.appendChild(row);
            });
        };

        renderLines(
            removedLines,
            diff.removed_line_samples,
            'No removed-line samples for this version.',
            'removed'
        );
        renderLines(
            addedLines,
            diff.added_line_samples,
            'No added-line samples for this version.',
            'added'
        );

        if (restoreButton) {
            restoreButton.disabled = !this.selectedVersionPreview;
            restoreButton.textContent = `Restore ${entityLabel} Version`;
        }
    }

    showTemplateVersionModal() {
        const modal = document.getElementById('template-version-modal');
        if (modal) {
            modal.classList.add('active');
        }
    }

    hideTemplateVersionModal() {
        const modal = document.getElementById('template-version-modal');
        if (modal) {
            modal.classList.remove('active');
        }
        this.selectedVersionPreview = null;
    }

    async restoreTemplateVersion(templateId, versionId) {
        if (!templateId || !versionId) {
            return;
        }
        const confirmed = window.confirm('Restore this template version? Current template content will be versioned before restore.');
        if (!confirmed) {
            return;
        }
        try {
            const restored = await this.apiCall(
                `/api/v1/templates/${encodeURIComponent(templateId)}/versions/${encodeURIComponent(versionId)}/restore`,
                { method: 'POST' }
            );
            this.showNotification('Template version restored.', 'success');
            this.selectedTemplateId = restored.id;
            await this.loadTemplates();
            this.editNode(restored);
        } catch (error) {
            console.error('Failed to restore template version:', error);
            this.showNotification('Unable to restore template version.', 'error');
        }
    }

    openTemplateInEditor(templateId) {
        const template = this.templatesCache.find((item) => item.id === templateId);
        if (!template) {
            this.showNotification('Template is no longer available in the current list.', 'warning');
            return;
        }
        this.editNode(template);
    }

    async duplicateTemplate(templateId) {
        const template = this.templatesCache.find((item) => item.id === templateId);
        if (!template) {
            this.showNotification('Template not found in current list.', 'warning');
            return;
        }

        const defaultTitle = template.title ? `${template.title} (copy)` : '';
        const titleOverride = window.prompt('Duplicate template title (optional):', defaultTitle);
        if (titleOverride === null) {
            return;
        }
        const namespaceInput = document.getElementById('template-instance-namespace');
        const targetNamespace = namespaceInput?.value?.trim() || template.namespace || 'default';
        const payload = { namespace: targetNamespace };
        const trimmedTitle = String(titleOverride || '').trim();
        if (trimmedTitle.length > 0) {
            payload.title = trimmedTitle;
        }

        try {
            const duplicate = await this.apiCall(
                `/api/v1/templates/${encodeURIComponent(templateId)}/duplicate`,
                {
                    method: 'POST',
                    body: JSON.stringify(payload)
                }
            );
            this.selectedTemplateId = duplicate.id;
            this.showNotification('Template duplicated successfully.', 'success');
            await this.loadTemplates();
        } catch (error) {
            console.error('Failed to duplicate template:', error);
        }
    }

    async deleteTemplate(templateId) {
        const template = this.templatesCache.find((item) => item.id === templateId);
        if (!template) {
            this.showNotification('Template not found in current list.', 'warning');
            return;
        }

        const confirmed = window.confirm(`Delete template "${template.title || template.id}"?`);
        if (!confirmed) {
            return;
        }

        try {
            const response = await this.apiCall(
                `/api/v1/templates/${encodeURIComponent(templateId)}`,
                { method: 'DELETE' }
            );
            if (response?.deleted) {
                this.showNotification('Template deleted.', 'success');
                this.templatesCache = this.templatesCache.filter((item) => item.id !== templateId);
                if (this.selectedTemplateId === templateId) {
                    this.selectedTemplateId = this.templatesCache.length > 0
                        ? this.templatesCache[0].id
                        : null;
                }
                this.renderTemplates(this.templatesCache);
                this.renderSelectedTemplateDetails();
                this.loadNodes();
                this.loadStats();
            } else {
                this.showNotification('Template was not deleted.', 'warning');
            }
        } catch (error) {
            console.error('Failed to delete template:', error);
        }
    }

    async createTemplate(event) {
        event.preventDefault();

        const kind = document.getElementById('template-create-kind').value;
        const content = document.getElementById('template-create-content').value.trim();
        if (!content) {
            this.showNotification('Template content cannot be empty.', 'warning');
            return;
        }

        const namespace = document.getElementById('template-create-namespace').value.trim() || 'default';
        const importanceRaw = parseFloat(document.getElementById('template-create-importance').value);
        const importance = Number.isFinite(importanceRaw)
            ? Math.min(Math.max(importanceRaw, 0), 1)
            : 0.5;

        const payload = {
            kind,
            content,
            namespace,
            importance,
            tags: this.parseCommaSeparatedInput(document.getElementById('template-create-tags').value)
        };

        const title = document.getElementById('template-create-title').value.trim();
        if (title) {
            payload.title = title;
        }
        const source = document.getElementById('template-create-source').value.trim();
        if (source) {
            payload.source = source;
        }
        const templateKey = document.getElementById('template-create-template-key').value.trim();
        if (templateKey) {
            payload.template_key = templateKey;
        }
        const templateVariables = this.parseCommaSeparatedInput(
            document.getElementById('template-create-template-variables').value
        );
        if (templateVariables.length > 0) {
            payload.template_variables = templateVariables;
        }

        try {
            const template = await this.apiCall('/api/v1/templates', {
                method: 'POST',
                body: JSON.stringify(payload)
            });
            this.showNotification('Template created successfully.', 'success');

            const form = document.getElementById('template-create-form');
            if (form) {
                form.reset();
            }
            document.getElementById('template-create-namespace').value = namespace;
            document.getElementById('template-create-importance').value = String(importance.toFixed(1));

            this.selectedTemplateId = template.id;
            document.getElementById('template-instance-namespace').value = namespace;
            await this.loadTemplates();
        } catch (error) {
            console.error('Failed to create template:', error);
        }
    }

    async instantiateSelectedTemplate(event) {
        event.preventDefault();
        const template = this.selectedTemplate();
        if (!template) {
            this.showNotification('Select a template first.', 'warning');
            return;
        }

        const namespace = document.getElementById('template-instance-namespace').value.trim();
        const title = document.getElementById('template-instance-title').value.trim();
        const extraTags = this.parseCommaSeparatedInput(document.getElementById('template-instance-tags').value);
        const values = this.templateValuesPayload();

        const payload = {};
        if (namespace) {
            payload.namespace = namespace;
        }
        if (title) {
            payload.title = title;
        }
        if (extraTags.length > 0) {
            payload.tags = extraTags;
        }
        if (Object.keys(values).length > 0) {
            payload.values = values;
        }

        try {
            const instance = await this.apiCall(
                `/api/v1/templates/${encodeURIComponent(template.id)}/instantiate`,
                {
                    method: 'POST',
                    body: JSON.stringify(payload)
                }
            );
            this.showNotification('Template instantiated successfully.', 'success');
            this.loadNodes();
            this.loadDueTasks();
            this.loadFocusTasks();
            this.loadDailyNotes();
            this.loadStats();
            this.editNode(instance);
        } catch (error) {
            console.error('Failed to instantiate template:', error);
        }
    }

    async loadStats() {
        try {
            const [healthResult, embeddingResult] = await Promise.allSettled([
                this.apiCall('/api/v1/health'),
                this.apiCall('/api/v1/diagnostics/embedding')
            ]);

            const health = healthResult.status === 'fulfilled' ? healthResult.value : {};
            const embedding = embeddingResult.status === 'fulfilled' ? embeddingResult.value : null;
            this.renderStats(health, embedding);
            if (this.activeTabId() === 'stats-tab') {
                this.loadAuditLogs(false);
            }
        } catch (error) {
            console.error('Failed to load stats:', error);
        }
    }

    auditQueryParams() {
        const limitInput = document.getElementById('audit-limit');
        const subjectInput = document.getElementById('audit-subject');
        const actionInput = document.getElementById('audit-action');
        const sinceInput = document.getElementById('audit-since');

        const params = new URLSearchParams();
        const rawLimit = Number.parseInt(limitInput?.value || '50', 10);
        const normalizedLimit = Number.isFinite(rawLimit)
            ? Math.min(Math.max(rawLimit, 1), 200)
            : 50;
        params.set('limit', String(normalizedLimit));
        if (limitInput) {
            limitInput.value = String(normalizedLimit);
        }

        const subject = subjectInput?.value?.trim();
        if (subject) {
            params.set('subject', subject);
        }

        const action = actionInput?.value?.trim();
        if (action) {
            params.set('action', action);
        }

        const sinceRaw = sinceInput?.value?.trim();
        if (sinceRaw) {
            const sinceIso = this.toIsoFromDatetimeLocal(sinceRaw);
            if (sinceIso) {
                params.set('since', sinceIso);
            } else {
                return {
                    params,
                    querySignature: '',
                    limit: normalizedLimit,
                    error: 'Invalid "since" timestamp. Use a valid date/time.'
                };
            }
        }

        return {
            params,
            querySignature: params.toString(),
            limit: normalizedLimit,
            error: null
        };
    }

    updateAuditLoadMoreButton() {
        if (!this.loadMoreAuditBtn) {
            return;
        }
        if (this.auditLoadingMore) {
            this.loadMoreAuditBtn.disabled = true;
            this.loadMoreAuditBtn.textContent = 'Loading...';
            return;
        }
        this.loadMoreAuditBtn.disabled = !this.auditHasMore;
        this.loadMoreAuditBtn.textContent = this.auditHasMore ? 'Load More' : 'No More Entries';
    }

    async loadAuditLogs(showErrors = false, append = false) {
        if (!this.auditContent) {
            return [];
        }

        const { params, querySignature, limit, error } = this.auditQueryParams();
        if (error) {
            this.auditContent.innerHTML = `<p class="template-empty">${this.escapeHtml(error)}</p>`;
            if (showErrors) {
                this.showNotification(error, 'warning');
            }
            this.auditHasMore = false;
            this.updateAuditLoadMoreButton();
            return [];
        }

        const queryChanged = querySignature !== this.auditQuerySignature;
        const useAppend = append && !queryChanged;
        if (queryChanged || !useAppend) {
            this.auditOffset = 0;
            this.auditQuerySignature = querySignature;
        }
        params.set('offset', String(this.auditOffset));

        if (!useAppend) {
            this.auditContent.innerHTML = '<p class="template-empty">Loading audit entries...</p>';
        }

        this.auditLoadingMore = useAppend;
        this.updateAuditLoadMoreButton();

        try {
            const entries = await this.apiCall(`/api/v1/audit?${params.toString()}`, {
                silent: true
            });
            const items = Array.isArray(entries) ? entries : [];
            this.renderAuditLogs(items, useAppend);
            if (useAppend) {
                this.auditOffset += items.length;
            } else {
                this.auditOffset = items.length;
            }
            this.auditHasMore = items.length >= limit;
            return items;
        } catch (fetchError) {
            console.error('Failed to load audit logs:', fetchError);
            const isForbidden = String(fetchError?.message || '').includes('403');
            const friendlyMessage = isForbidden
                ? 'Audit log access requires an admin-authenticated session.'
                : 'Unable to load audit entries right now.';
            if (!useAppend) {
                this.auditContent.innerHTML = `<p class="template-empty">${this.escapeHtml(friendlyMessage)}</p>`;
            }
            this.auditHasMore = false;
            if (showErrors) {
                this.showNotification(friendlyMessage, 'error');
            }
            return [];
        } finally {
            this.auditLoadingMore = false;
            this.updateAuditLoadMoreButton();
        }
    }

    renderAuditLogs(entries, append = false) {
        if (!this.auditContent) {
            return;
        }

        if (!append) {
            this.auditContent.innerHTML = '';
        }

        const items = Array.isArray(entries) ? entries : [];
        if (items.length === 0 && !append) {
            this.auditContent.innerHTML = '<p class="template-empty">No audit entries match the current filters.</p>';
            return;
        }

        items.forEach((entry) => {
            const container = document.createElement('div');
            container.className = 'audit-item';
            const queryParams = entry?.query_params && Object.keys(entry.query_params).length > 0
                ? Object.entries(entry.query_params)
                    .map(([key, value]) => `${key}=${value}`)
                    .join(', ')
                : 'none';

            const successLabel = entry?.success ? 'success' : 'failure';
            const statusCode = Number(entry?.status_code || 0);
            const latency = Number(entry?.latency_ms || 0);
            const timestamp = this.formatDateTime(entry?.timestamp);

            container.innerHTML = `
                <div class="audit-item-header">
                    <span class="audit-item-action">${this.escapeHtml(entry?.action || 'unknown_action')}</span>
                    <span class="audit-item-time">${this.escapeHtml(timestamp)}</span>
                </div>
                <div class="audit-item-meta">
                    <span>${this.escapeHtml((entry?.method || 'GET').toUpperCase())}</span>
                    <span>${this.escapeHtml(entry?.path || '/')}</span>
                    <span>${this.escapeHtml(`status:${statusCode}`)}</span>
                    <span>${this.escapeHtml(successLabel)}</span>
                    <span>${this.escapeHtml(`latency:${latency}ms`)}</span>
                    <span>${this.escapeHtml(`role:${entry?.role || 'none'}`)}</span>
                    <span>${this.escapeHtml(`subject:${entry?.subject || 'anonymous'}`)}</span>
                    <span>${this.escapeHtml(`namespace:${entry?.namespace || 'global'}`)}</span>
                    <span>${this.escapeHtml(`request:${entry?.request_id || 'n/a'}`)}</span>
                    <span>${this.escapeHtml(`resource:${entry?.resource_id || 'n/a'}`)}</span>
                    <span>${this.escapeHtml(`query:${queryParams}`)}</span>
                </div>
            `;
            this.auditContent.appendChild(container);
        });
    }

    async exportVaultData() {
        const namespace = document.getElementById('node-namespace-filter').value.trim();
        const params = new URLSearchParams({
            include_relationships: 'true'
        });
        if (namespace) {
            params.set('namespace', namespace);
        }

        try {
            const bundle = await this.apiCall(`/api/v1/export?${params.toString()}`);
            const blob = new Blob([JSON.stringify(bundle, null, 2)], { type: 'application/json' });
            const filename = `mindvault-export-${this.timestampForFilename()}.json`;
            const url = URL.createObjectURL(blob);

            const anchor = document.createElement('a');
            anchor.href = url;
            anchor.download = filename;
            document.body.appendChild(anchor);
            anchor.click();
            anchor.remove();
            URL.revokeObjectURL(url);

            this.showNotification(`Exported ${bundle.nodes?.length || 0} nodes.`, 'success');
        } catch (error) {
            console.error('Failed to export vault data:', error);
        }
    }

    async importVaultData(fileList) {
        const input = document.getElementById('import-data-file');
        const file = fileList && fileList[0];
        if (!file) {
            return;
        }

        try {
            const raw = await file.text();
            const parsed = JSON.parse(raw);
            const payload = {
                namespace_override: null,
                overwrite_existing: false,
                include_relationships: true,
                nodes: Array.isArray(parsed.nodes) ? parsed.nodes : [],
                relationships: Array.isArray(parsed.relationships) ? parsed.relationships : []
            };

            if (payload.nodes.length === 0) {
                this.showNotification('Import file contains no nodes.', 'warning');
                return;
            }

            const result = await this.apiCall('/api/v1/import', {
                method: 'POST',
                body: JSON.stringify(payload)
            });

            this.showNotification(
                `Imported ${result.imported_nodes} nodes and ${result.imported_relationships} relationships.`,
                'success'
            );
            this.loadNodes();
            this.loadStats();
            this.loadDailyNotes();
            this.loadDueTasks();
            this.loadFocusTasks();
        } catch (error) {
            console.error('Failed to import vault data:', error);
            this.showNotification('Import failed. Ensure the file is valid MindVault export JSON.', 'error');
        } finally {
            if (input) {
                input.value = '';
            }
        }
    }

    renderStats(stats, embedding) {
        const container = document.getElementById('stats-content');
        const diagnostics = embedding
            ? `
            <div class="stat-item">
                <span class="stat-label">Embedding Provider</span>
                <span class="stat-value">${this.escapeHtml(embedding.effective_provider || 'unknown')}</span>
            </div>
            <div class="stat-item">
                <span class="stat-label">Embedding Model</span>
                <span class="stat-value">${this.escapeHtml(embedding.effective_model || 'unknown')}</span>
            </div>
            <div class="stat-item">
                <span class="stat-label">Fallback To Noop</span>
                <span class="stat-value">${embedding.fallback_to_noop ? 'true' : 'false'}</span>
            </div>
            `
            : '';

        container.innerHTML = `
            <div class="stat-item">
                <span class="stat-label">Status</span>
                <span class="stat-value">${this.escapeHtml(stats.status || 'unknown')}</span>
            </div>
            <div class="stat-item">
                <span class="stat-label">Total Nodes</span>
                <span class="stat-value">${Number(stats.node_count || 0)}</span>
            </div>
            <div class="stat-item">
                <span class="stat-label">Version</span>
                <span class="stat-value">${this.escapeHtml(stats.version || 'unknown')}</span>
            </div>
            ${diagnostics}
        `;
    }

    showAddNodeModal() {
        document.getElementById('modal-title').textContent = 'Add Knowledge Node';
        document.getElementById('node-form').reset();
        document.getElementById('node-form').dataset.nodeId = '';
        this.currentEditingNode = null;
        this.attachedFiles = [];
        this.existingAttachments = [];
        this.setEditorContentFromMarkdown('');
        this.setEditorMode('wysiwyg');
        this.applySchedulingInputsFromMetadata({});
        this.aiSuggestions.classList.remove('active');
        this.aiSuggestions.innerHTML = '';
        this.clearWikiLinkSuggestions();
        this.renderAttachmentPreview();
        if (this.attachmentTriage) {
            this.attachmentTriage.loadExistingAttachments(null);
        }
        this.setNodeVersionHistoryPlaceholder('Version history appears after the first edit save.');
        this.setNodeRelationshipOverviewPlaceholder('Relationship context appears when editing an existing node.');
        document.getElementById('node-modal').classList.add('active');
    }

    editNode(node) {
        document.getElementById('modal-title').textContent = 'Edit Knowledge Node';
        this.setSelectValueWithFallback('node-kind', node.kind || 'fact', 'fact');
        document.getElementById('node-title').value = node.title || '';
        this.setEditorContentFromMarkdown(node.content || '');
        document.getElementById('node-source').value = node.source || '';
        document.getElementById('node-namespace').value = node.namespace || 'default';
        document.getElementById('node-tags').value = (node.tags || []).join(', ');
        document.getElementById('node-importance').value = node.importance ?? 0.5;
        this.currentEditingNode = node;
        this.applySchedulingInputsFromMetadata(node);

        document.getElementById('node-form').dataset.nodeId = node.id;
        this.aiSuggestions.classList.remove('active');
        this.aiSuggestions.innerHTML = '';
        this.clearWikiLinkSuggestions();
        this.attachedFiles = [];
        this.existingAttachments = [];
        this.renderAttachmentPreview();
        this.loadExistingAttachments(node.id, { resetPage: true });
        if (this.isTemplateNode(node)) {
            this.setNodeVersionHistoryPlaceholder('Template nodes use the Templates tab history workflow.');
        } else {
            this.loadNodeVersionHistory(node.id);
        }
        this.loadNodeRelationshipOverview(node.id);
        document.getElementById('node-modal').classList.add('active');
    }

    hideModal() {
        document.getElementById('node-modal').classList.remove('active');
        document.getElementById('node-form').dataset.nodeId = '';
        this.currentEditingNode = null;
        this.aiSuggestions.classList.remove('active');
        this.aiSuggestions.innerHTML = '';
        this.attachedFiles = [];
        this.existingAttachments = [];
        this.renderAttachmentPreview();
        if (this.attachmentTriage) {
            this.attachmentTriage.loadExistingAttachments(null);
        }
        this.setNodeVersionHistoryPlaceholder('Version history appears after the first edit save.');
        this.setNodeRelationshipOverviewPlaceholder('Relationship context appears when editing an existing node.');
        this.clearAutoComplete();
        this.clearWikiLinkSuggestions();
    }

    setNodeVersionHistoryPlaceholder(message) {
        if (!this.nodeVersionHistory) {
            return;
        }
        this.nodeVersionHistory.innerHTML = `<p class="template-empty">${this.escapeHtml(message)}</p>`;
    }

    setNodeRelationshipOverviewPlaceholder(message) {
        if (!this.nodeRelationshipOverview) {
            return;
        }
        this.nodeRelationshipOverview.innerHTML = `<p class="template-empty">${this.escapeHtml(message)}</p>`;
    }

    async loadNodeRelationshipOverview(nodeId) {
        if (!nodeId || !this.nodeRelationshipOverview) {
            this.setNodeRelationshipOverviewPlaceholder('Relationship context appears when editing an existing node.');
            return;
        }

        this.setNodeRelationshipOverviewPlaceholder('Loading relationship context...');
        try {
            const overview = await this.apiCall(
                `/api/v1/graph/relationships/${encodeURIComponent(nodeId)}`,
                { silent: true }
            );
            this.renderNodeRelationshipOverview(overview);
        } catch (error) {
            console.error('Failed to load node relationship overview:', error);
            this.setNodeRelationshipOverviewPlaceholder('Unable to load relationship context right now.');
        }
    }

    renderNodeRelationshipOverview(overview) {
        if (!this.nodeRelationshipOverview) {
            return;
        }

        const outgoing = Array.isArray(overview?.outgoing) ? overview.outgoing : [];
        const incoming = Array.isArray(overview?.incoming) ? overview.incoming : [];
        if (outgoing.length === 0 && incoming.length === 0) {
            this.setNodeRelationshipOverviewPlaceholder('No related nodes yet. Add wiki-links, @mentions, or graph relationships to build context.');
            return;
        }

        this.nodeRelationshipOverview.innerHTML = '';
        this.nodeRelationshipOverview.appendChild(
            this.renderNodeRelationshipGroup('Outgoing', outgoing, 'Links from this node')
        );
        this.nodeRelationshipOverview.appendChild(
            this.renderNodeRelationshipGroup('Incoming', incoming, 'Links to this node')
        );
    }

    renderNodeRelationshipGroup(title, entries, emptyMessage) {
        const section = document.createElement('section');
        section.className = 'relationship-group';
        const heading = document.createElement('h5');
        heading.textContent = title;
        section.appendChild(heading);

        const items = Array.isArray(entries) ? entries : [];
        if (items.length === 0) {
            const empty = document.createElement('p');
            empty.className = 'template-empty';
            empty.textContent = emptyMessage;
            section.appendChild(empty);
            return section;
        }

        items.forEach((entry) => {
            const autoSource = String(entry?.auto_source || '').trim();
            const autoSourceBadge = entry.auto_managed && autoSource
                ? `<span class="relationship-auto-source-badge">${this.escapeHtml(`auto:${autoSource.replaceAll('_', ' ')}`)}</span>`
                : '';
            const row = document.createElement('div');
            row.className = 'relationship-entry';
            row.innerHTML = `
                <div class="relationship-entry-head">
                    <button type="button" class="relationship-entry-target">${this.escapeHtml(entry.related_node_title || 'Untitled')}</button>
                    <span class="relationship-entry-kind">${this.escapeHtml(entry.relation_kind || 'relates_to')}</span>
                </div>
                <div class="relationship-entry-meta">
                    <span>${this.escapeHtml(entry.related_node_kind || 'unknown')}</span>
                    <span>${this.escapeHtml(entry.related_node_namespace || 'default')}</span>
                    <span>${this.escapeHtml(`weight:${Number(entry.weight || 0).toFixed(2)}`)}</span>
                    <span>${this.escapeHtml(this.formatDateTime(entry.created_at))}</span>
                    <span class="relationship-entry-id">${this.escapeHtml(entry.related_node_id || '')}</span>
                    ${entry.auto_managed ? '<span class="relationship-auto-badge">auto</span>' : ''}
                    ${autoSourceBadge}
                </div>
            `;
            const targetButton = row.querySelector('.relationship-entry-target');
            if (targetButton) {
                targetButton.addEventListener('click', () => {
                    this.openNodeById(entry.related_node_id);
                });
            }
            section.appendChild(row);
        });

        return section;
    }

    async openNodeById(nodeId) {
        const normalizedNodeId = String(nodeId || '').trim();
        if (!normalizedNodeId) {
            return;
        }
        try {
            const node = await this.apiCall(`/api/v1/nodes/${encodeURIComponent(normalizedNodeId)}`, {
                silent: true
            });
            if (!node) {
                this.showNotification('Related node not found.', 'warning');
                return;
            }
            this.editNode(node);
        } catch (error) {
            console.error('Failed to open related node:', error);
            this.showNotification('Unable to open related node.', 'error');
        }
    }

    async loadNodeVersionHistory(nodeId) {
        if (!nodeId || !this.nodeVersionHistory) {
            this.setNodeVersionHistoryPlaceholder('Version history appears after the first edit save.');
            return;
        }
        try {
            const versions = await this.apiCall(`/api/v1/nodes/${encodeURIComponent(nodeId)}/versions`);
            this.renderNodeVersionHistory(nodeId, Array.isArray(versions) ? versions : []);
        } catch (error) {
            console.error('Failed to load node history:', error);
            this.setNodeVersionHistoryPlaceholder('Unable to load node history right now.');
        }
    }

    renderNodeVersionHistory(nodeId, versions) {
        if (!this.nodeVersionHistory) {
            return;
        }
        this.nodeVersionHistory.innerHTML = '';

        const items = Array.isArray(versions) ? versions : [];
        if (items.length === 0) {
            this.setNodeVersionHistoryPlaceholder('No saved versions yet. Edit and save this node to start history.');
            return;
        }

        items.forEach((version) => {
            const entry = document.createElement('div');
            entry.className = 'template-history-item';
            entry.innerHTML = `
                <div class="template-history-head">
                    <div class="template-history-title">${this.escapeHtml(version.title || 'Untitled')}</div>
                    <div class="template-history-time">${this.escapeHtml(this.formatDateTime(version.captured_at))}</div>
                </div>
                <div class="template-history-meta">
                    <span>${this.escapeHtml(version.kind || 'unknown')}</span>
                    <span>${this.escapeHtml(version.namespace || 'default')}</span>
                    <span>tags: ${this.escapeHtml(String(version.tag_count || 0))}</span>
                    <span>importance: ${Number(version.importance || 0).toFixed(2)}</span>
                </div>
                <div class="template-history-preview">${this.escapeHtml(version.content_preview || '')}</div>
                <div class="template-history-actions">
                    <button type="button" class="btn node-history-preview-btn">Preview Diff</button>
                    <button type="button" class="btn node-history-restore-btn">Restore</button>
                </div>
            `;
            const previewButton = entry.querySelector('.node-history-preview-btn');
            if (previewButton) {
                previewButton.addEventListener('click', () => {
                    this.previewNodeVersion(nodeId, version.version_id);
                });
            }
            const restoreButton = entry.querySelector('.node-history-restore-btn');
            if (restoreButton) {
                restoreButton.addEventListener('click', () => {
                    this.restoreNodeVersion(nodeId, version.version_id);
                });
            }
            this.nodeVersionHistory.appendChild(entry);
        });
    }

    async previewNodeVersion(nodeId, versionId) {
        if (!nodeId || !versionId) {
            return;
        }
        try {
            const detail = await this.apiCall(
                `/api/v1/nodes/${encodeURIComponent(nodeId)}/versions/${encodeURIComponent(versionId)}`
            );
            this.selectedVersionPreview = { entityType: 'node', entityId: nodeId, versionId };
            this.renderTemplateVersionPreviewModal(detail);
            this.showTemplateVersionModal();
        } catch (error) {
            console.error('Failed to load node version preview:', error);
            this.showNotification('Unable to load node version preview.', 'error');
        }
    }

    async restoreNodeVersion(nodeId, versionId) {
        if (!nodeId || !versionId) {
            return;
        }
        const confirmed = window.confirm('Restore this node version? Current content will be captured in history first.');
        if (!confirmed) {
            return;
        }
        try {
            const restored = await this.apiCall(
                `/api/v1/nodes/${encodeURIComponent(nodeId)}/versions/${encodeURIComponent(versionId)}/restore`,
                { method: 'POST' }
            );
            this.showNotification('Node version restored.', 'success');
            this.currentEditingNode = restored;
            this.editNode(restored);
            this.loadNodes();
            this.loadStats();
        } catch (error) {
            console.error('Failed to restore node version:', error);
            this.showNotification('Unable to restore node version.', 'error');
        }
    }

    async saveNode(event) {
        event.preventDefault();

        const markdownContent = this.getEditorMarkdown();
        if (!markdownContent) {
            this.showNotification('Content cannot be empty.', 'warning');
            return;
        }

        const nodeData = {
            kind: document.getElementById('node-kind').value,
            title: document.getElementById('node-title').value || null,
            content: markdownContent,
            source: document.getElementById('node-source').value || null,
            namespace: document.getElementById('node-namespace').value,
            tags: document.getElementById('node-tags').value
                .split(',')
                .map((tag) => tag.trim())
                .filter((tag) => tag.length > 0),
            importance: parseFloat(document.getElementById('node-importance').value)
        };
        nodeData.metadata = this.buildSchedulingMetadata(nodeData.kind, this.currentEditingNode?.metadata || {});

        const nodeId = event.target.dataset.nodeId;

        try {
            let savedNode;
            if (nodeId) {
                const existingNode = (this.currentEditingNode && this.currentEditingNode.id === nodeId)
                    ? this.currentEditingNode
                    : await this.apiCall(`/api/v1/nodes/${encodeURIComponent(nodeId)}`);
                if (!existingNode) {
                    throw new Error('Existing node not found for update.');
                }
                const payload = {
                    ...existingNode,
                    ...nodeData,
                    id: nodeId,
                    temporal: existingNode.temporal,
                    metadata: nodeData.metadata
                };
                savedNode = await this.apiCall(`/api/v1/nodes/${nodeId}`, {
                    method: 'PUT',
                    body: JSON.stringify(payload)
                });
                this.showNotification('Node updated successfully.', 'success');
            } else {
                savedNode = await this.apiCall('/api/v1/nodes', {
                    method: 'POST',
                    body: JSON.stringify(nodeData)
                });
                this.showNotification('Node created successfully.', 'success');
            }

            // Upload attachments if any
            if (this.attachedFiles.length > 0) {
                const nodeId = savedNode.id;
                const uploadedFiles = await this.uploadAttachments(nodeId);
                if (uploadedFiles.length > 0) {
                    this.showNotification(`Uploaded ${uploadedFiles.length} file(s).`, 'success');
                }
                // Clear attachments after successful upload
                this.attachedFiles = [];
                this.renderAttachmentPreview();
            }

            this.hideModal();
            this.loadNodes();
            this.loadDueTasks();
            this.loadFocusTasks();
            this.loadStats();
            const activeTab = this.activeTabId();
            if (
                activeTab === 'calendar-tab'
                || nodeData.kind === 'task'
                || nodeData.kind === 'event'
            ) {
                this.loadCalendarItems();
            }
            if (this.isTemplateNode(savedNode) || this.templatesCache.length > 0) {
                this.loadTemplates();
            }
        } catch (error) {
            console.error('Failed to save node:', error);
        }
    }

    escapeHtml(value) {
        return String(value)
            .replaceAll('&', '&amp;')
            .replaceAll('<', '&lt;')
            .replaceAll('>', '&gt;')
            .replaceAll('"', '&quot;')
            .replaceAll("'", '&#39;');
    }

    showNotification(message, type = 'info') {
        const notification = document.createElement('div');
        notification.className = `notification ${type}`;
        notification.textContent = message;

        document.getElementById('notifications').appendChild(notification);

        window.setTimeout(() => {
            notification.remove();
        }, 5000);
    }

    // File attachment methods
    handleFileSelection(files) {
        Array.from(files).forEach(file => {
            if (this.validateFile(file)) {
                this.addFileToAttachments(file);
            }
        });
    }

    validateFile(file) {
        const maxSize = 10 * 1024 * 1024; // 10MB
        const allowedTypes = [
            'image/', 'audio/', 'video/',
            'application/pdf',
            'application/msword',
            'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
            'text/plain', 'text/markdown'
        ];

        if (file.size > maxSize) {
            this.showNotification(`File "${file.name}" is too large (max 10MB)`, 'error');
            return false;
        }

        if (!allowedTypes.some(type => file.type.startsWith(type))) {
            this.showNotification(`File type "${file.type}" not supported`, 'error');
            return false;
        }

        return true;
    }

    addFileToAttachments(file) {
        const fileId = Date.now() + '_' + Math.random().toString(36).substr(2, 9);
        const attachment = {
            id: fileId,
            file: file,
            name: file.name,
            size: file.size,
            type: file.type
        };

        this.attachedFiles.push(attachment);
        this.renderAttachmentPreview();
    }

    renderAttachmentPreview() {
        this.attachmentPreview.innerHTML = '';

        if (this.attachedFiles.length === 0 && this.existingAttachments.length === 0) {
            this.attachmentPreview.innerHTML = '<p class="attachment-empty">No attachments yet.</p>';
            return;
        }

        this.existingAttachments.forEach((attachment) => {
            const item = document.createElement('div');
            item.className = 'attachment-item existing';
            const searchChunkMeta = this.formatAttachmentChunkStatus(attachment);
            const searchPreview = String(attachment?.search_preview || '').trim();
            const chunkCount = Number(attachment?.search_chunk_count || 0);
            const hasChunkPreview = Number.isFinite(chunkCount) && chunkCount > 0;
            const extractionStatus = String(attachment?.extraction_status || '').toLowerCase();
            const canReindex = extractionStatus !== 'transcribed';
            item.innerHTML = `
                <div style="width:32px;height:32px;background:#dbeafe;border-radius:4px;display:flex;align-items:center;justify-content:center;font-size:12px;">📂</div>
                <div class="attachment-info">
                    <div class="attachment-name">${this.escapeHtml(attachment.file_name || 'attachment')}</div>
                    <div class="attachment-size">
                        ${this.formatFileSize(Number(attachment.size_bytes || 0))}
                        ${this.escapeHtml(this.formatAttachmentExtractionStatus(attachment))}
                        ${this.escapeHtml(searchChunkMeta)}
                    </div>
                    ${searchPreview ? `<div class="attachment-search-preview">${this.escapeHtml(searchPreview)}</div>` : ''}
                    ${hasChunkPreview ? '<div class="attachment-chunks-panel is-hidden"></div>' : ''}
                </div>
                <div class="attachment-actions">
                    <button type="button" class="attachment-download" data-attachment-id="${this.escapeHtml(attachment.attachment_id)}">Download</button>
                    ${canReindex ? `<button type="button" class="attachment-reindex" data-attachment-id="${this.escapeHtml(attachment.attachment_id)}">Reindex</button>` : ''}
                    ${hasChunkPreview ? `<button type="button" class="attachment-view-text" data-attachment-id="${this.escapeHtml(attachment.attachment_id)}">View Text</button>` : ''}
                    <button type="button" class="attachment-remove-existing" data-attachment-id="${this.escapeHtml(attachment.attachment_id)}">Remove</button>
                </div>
            `;

            item.querySelector('.attachment-download').addEventListener('click', () => {
                this.downloadExistingAttachment(attachment);
            });
            const reindexButton = item.querySelector('.attachment-reindex');
            if (reindexButton) {
                reindexButton.addEventListener('click', () => {
                    this.reindexExistingAttachment(attachment, reindexButton);
                });
            }
            const viewTextButton = item.querySelector('.attachment-view-text');
            if (viewTextButton) {
                viewTextButton.addEventListener('click', () => {
                    this.toggleAttachmentChunks(attachment, item, viewTextButton);
                });
            }
            item.querySelector('.attachment-remove-existing').addEventListener('click', () => {
                this.deleteExistingAttachment(attachment);
            });

            this.attachmentPreview.appendChild(item);
        });

        this.attachedFiles.forEach(attachment => {
            const item = document.createElement('div');
            item.className = 'attachment-item';

            const isImage = attachment.type.startsWith('image/');
            const preview = isImage ?
                `<img src="${URL.createObjectURL(attachment.file)}" alt="${attachment.name}">` :
                '<div style="width:32px;height:32px;background:#e2e8f0;border-radius:4px;display:flex;align-items:center;justify-content:center;font-size:12px;">📎</div>';

            item.innerHTML = `
                ${preview}
                <div class="attachment-info">
                    <div class="attachment-name">${this.escapeHtml(attachment.name)}</div>
                    <div class="attachment-size">${this.formatFileSize(attachment.size)}</div>
                </div>
                <button type="button" class="attachment-remove" data-attachment-id="${attachment.id}">×</button>
            `;

            item.querySelector('.attachment-remove').addEventListener('click', () => {
                this.removeAttachment(attachment.id);
            });

            this.attachmentPreview.appendChild(item);
        });
    }

    async loadExistingAttachments(nodeId, options = {}) {
        if (this.attachmentTriage?.loadExistingAttachments) {
            await this.attachmentTriage.loadExistingAttachments(nodeId, options);
            return;
        }

        if (!nodeId) {
            this.existingAttachments = [];
            this.attachmentChunksCache = new Map();
            this.renderAttachmentPreview();
            return;
        }

        try {
            const attachments = await this.apiCall(`/api/v1/files/${encodeURIComponent(nodeId)}`);
            this.existingAttachments = Array.isArray(attachments) ? attachments : [];
            this.attachmentChunksCache = new Map();
            this.renderAttachmentPreview();
        } catch (error) {
            this.existingAttachments = [];
            this.attachmentChunksCache = new Map();
            this.renderAttachmentPreview();
            console.error('Failed to load existing attachments:', error);
        }
    }

    async downloadExistingAttachment(attachment) {
        if (!attachment || !attachment.download_url) {
            return;
        }

        try {
            const response = await fetch(`${this.apiBase}${attachment.download_url}`);
            if (!response.ok) {
                throw new Error(`Download failed: ${response.statusText}`);
            }
            const blob = await response.blob();
            const url = URL.createObjectURL(blob);
            const anchor = document.createElement('a');
            anchor.href = url;
            anchor.download = attachment.file_name || 'attachment.bin';
            document.body.appendChild(anchor);
            anchor.click();
            anchor.remove();
            URL.revokeObjectURL(url);
        } catch (error) {
            console.error('Failed to download attachment:', error);
            this.showNotification('Unable to download attachment.', 'error');
        }
    }

    async deleteExistingAttachment(attachment) {
        if (!attachment || !this.currentEditingNode?.id) {
            return;
        }

        const confirmed = window.confirm(`Remove attachment "${attachment.file_name || attachment.attachment_id}"?`);
        if (!confirmed) {
            return;
        }

        try {
            await this.apiCall(
                `/api/v1/files/${encodeURIComponent(this.currentEditingNode.id)}/${encodeURIComponent(attachment.attachment_id)}`,
                { method: 'DELETE' }
            );
            await this.loadExistingAttachments(this.currentEditingNode.id);
            this.showNotification('Attachment removed.', 'success');
        } catch (error) {
            console.error('Failed to delete attachment:', error);
            this.showNotification('Unable to delete attachment.', 'error');
        }
    }

    async reindexExistingAttachment(attachment, triggerButton) {
        if (!attachment || !this.currentEditingNode?.id) {
            return;
        }
        const attachmentId = String(attachment.attachment_id || '').trim();
        if (!attachmentId) {
            return;
        }

        const originalLabel = triggerButton?.textContent || 'Reindex';
        if (triggerButton) {
            triggerButton.disabled = true;
            triggerButton.textContent = 'Reindexing...';
        }

        try {
            const response = await this.apiCall(
                `/api/v1/files/${encodeURIComponent(this.currentEditingNode.id)}/${encodeURIComponent(attachmentId)}/reindex`,
                { method: 'POST' }
            );
            this.attachmentChunksCache.delete(attachmentId);
            await this.loadExistingAttachments(this.currentEditingNode.id);
            const status = String(response?.extraction_status || '').replaceAll('_', ' ');
            this.showNotification(`Attachment reindexed (${status || 'updated'}).`, 'success');
        } catch (error) {
            console.error('Failed to reindex attachment:', error);
            this.showNotification('Unable to reindex attachment.', 'error');
        } finally {
            if (triggerButton?.isConnected) {
                triggerButton.disabled = false;
                triggerButton.textContent = originalLabel;
            }
        }
    }

    removeAttachment(attachmentId) {
        this.attachedFiles = this.attachedFiles.filter(att => att.id !== attachmentId);
        this.renderAttachmentPreview();
    }

    formatFileSize(bytes) {
        if (bytes === 0) return '0 Bytes';
        const k = 1024;
        const sizes = ['Bytes', 'KB', 'MB', 'GB'];
        const i = Math.floor(Math.log(bytes) / Math.log(k));
        return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
    }

    formatAttachmentExtractionStatus(attachment) {
        const status = String(attachment?.extraction_status || '').toLowerCase();
        const extractedChars = Number(attachment?.extracted_chars || 0);

        if (status.startsWith('indexed') && extractedChars > 0) {
            return `• indexed ${extractedChars} chars`;
        }
        if (status === 'unsupported') {
            return '• not indexed';
        }
        if (status === 'tool_missing') {
            return '• tool missing';
        }
        if (status === 'extraction_failed') {
            return '• extraction failed';
        }
        if (status === 'empty') {
            return '• empty text';
        }
        return '';
    }

    formatAttachmentChunkStatus(attachment) {
        const chunkCount = Number(attachment?.search_chunk_count || 0);
        if (!Number.isFinite(chunkCount) || chunkCount <= 0) {
            return '';
        }
        return `• ${chunkCount} chunk${chunkCount === 1 ? '' : 's'}`;
    }

    async fetchAttachmentChunks(attachmentId, limit = 12, offset = 0) {
        if (!this.currentEditingNode?.id || !attachmentId) {
            return null;
        }
        const params = new URLSearchParams({
            limit: String(limit),
            offset: String(offset)
        });
        return this.apiCall(
            `/api/v1/files/${encodeURIComponent(this.currentEditingNode.id)}/${encodeURIComponent(attachmentId)}/chunks?${params.toString()}`
        );
    }

    renderAttachmentChunksPanel(panel, payload) {
        if (!panel) {
            return;
        }

        const chunks = Array.isArray(payload?.chunks) ? payload.chunks : [];
        const total = Number(payload?.total_chunks || chunks.length || 0);
        const returned = Number(payload?.returned_chunks || chunks.length || 0);
        if (chunks.length === 0) {
            panel.innerHTML = '<p class="attachment-chunks-empty">No indexed text chunks available yet.</p>';
            return;
        }

        const chunkHtml = chunks
            .map((chunk) => {
                const index = Number(chunk?.index ?? 0);
                const text = String(chunk?.text || '').trim();
                const charCount = Number(chunk?.char_count || text.length || 0);
                return `
                    <div class="attachment-chunk-item">
                        <div class="attachment-chunk-meta">Chunk ${this.escapeHtml(String(index + 1))} • ${this.escapeHtml(String(charCount))} chars</div>
                        <div class="attachment-chunk-text">${this.escapeHtml(text)}</div>
                    </div>
                `;
            })
            .join('');

        panel.innerHTML = `
            <div class="attachment-chunks-header">Indexed text chunks ${this.escapeHtml(String(returned))}/${this.escapeHtml(String(total))}</div>
            ${chunkHtml}
        `;
    }

    async toggleAttachmentChunks(attachment, itemElement, toggleButton) {
        if (!attachment || !itemElement || !toggleButton) {
            return;
        }
        const panel = itemElement.querySelector('.attachment-chunks-panel');
        if (!panel) {
            return;
        }

        const isVisible = !panel.classList.contains('is-hidden');
        if (isVisible) {
            panel.classList.add('is-hidden');
            toggleButton.textContent = 'View Text';
            return;
        }

        const attachmentId = String(attachment.attachment_id || '').trim();
        if (!attachmentId) {
            return;
        }

        try {
            toggleButton.disabled = true;
            toggleButton.textContent = 'Loading...';
            let payload = this.attachmentChunksCache.get(attachmentId);
            if (!payload) {
                payload = await this.fetchAttachmentChunks(attachmentId, 12, 0);
                if (payload) {
                    this.attachmentChunksCache.set(attachmentId, payload);
                }
            }

            this.renderAttachmentChunksPanel(panel, payload || {});
            panel.classList.remove('is-hidden');
            toggleButton.textContent = 'Hide Text';
        } catch (error) {
            console.error('Failed to load attachment chunks:', error);
            this.showNotification('Unable to load indexed attachment text.', 'warning');
            toggleButton.textContent = 'View Text';
        } finally {
            toggleButton.disabled = false;
        }
    }

    async uploadAttachments(nodeId) {
        if (this.attachedFiles.length === 0) return [];

        const uploadedFiles = [];

        for (const attachment of this.attachedFiles) {
            try {
                const formData = new FormData();
                formData.append('file', attachment.file);
                formData.append('node_id', nodeId);

                const response = await fetch(`${this.apiBase}/api/v1/files/upload`, {
                    method: 'POST',
                    body: formData
                });

                if (!response.ok) {
                    throw new Error(`Upload failed: ${response.statusText}`);
                }

                const result = await response.json();
                uploadedFiles.push(result);
            } catch (error) {
                console.error('Failed to upload file:', attachment.name, error);
                this.showNotification(`Failed to upload ${attachment.name}`, 'error');
            }
        }

        return uploadedFiles;
    }
}

document.addEventListener('DOMContentLoaded', () => {
    window.mindVaultAdmin = new MindVaultAdmin();
});
