export class AttachmentTriage {
    constructor(admin) {
        this.admin = admin;
        this.state = {
            query: '',
            status: 'all',
            sort: 'uploaded_at_desc',
            page: 1,
            pageSize: 25,
            total: 0,
            totalQueryMatched: 0,
            totalUnfiltered: 0,
            hasMore: false,
            supportsPagedEndpoint: true,
            statusFacets: null
        };
        this.available = true;
        this.elements = {
            toolbar: null,
            pagination: null,
            filterQueryInput: null,
            filterStatusSelect: null,
            sortSelect: null,
            filterApplyBtn: null,
            reindexFailedBtn: null,
            deleteFilteredBtn: null,
            statusFacets: null,
            listMeta: null,
            prevPageBtn: null,
            nextPageBtn: null,
            pageLabel: null
        };
    }

    init() {
        this.cacheElements();
        this.bindEvents();
        this.syncFilterInputs();
        this.renderSummary();
    }

    cacheElements() {
        this.elements.filterQueryInput = document.getElementById('attachment-filter-query');
        this.elements.filterStatusSelect = document.getElementById('attachment-filter-status');
        this.elements.sortSelect = document.getElementById('attachment-sort');
        this.elements.filterApplyBtn = document.getElementById('attachment-filter-apply-btn');
        this.elements.reindexFailedBtn = document.getElementById('attachment-reindex-failed-btn');
        this.elements.deleteFilteredBtn = document.getElementById('attachment-delete-filtered-btn');
        this.elements.statusFacets = document.getElementById('attachment-status-facets');
        this.elements.listMeta = document.getElementById('attachment-list-meta');
        this.elements.toolbar = document.querySelector('.attachment-toolbar');
        this.elements.pagination = document.querySelector('.attachment-pagination');
        this.elements.prevPageBtn = document.getElementById('attachment-prev-page-btn');
        this.elements.nextPageBtn = document.getElementById('attachment-next-page-btn');
        this.elements.pageLabel = document.getElementById('attachment-page-label');
    }

    bindEvents() {
        if (this.elements.filterApplyBtn) {
            this.elements.filterApplyBtn.addEventListener('click', () => {
                this.applyFilters({ resetPage: true });
            });
        }
        if (this.elements.filterQueryInput) {
            this.elements.filterQueryInput.addEventListener('keydown', (event) => {
                if (event.key === 'Enter') {
                    event.preventDefault();
                    this.applyFilters({ resetPage: true });
                }
            });
        }
        if (this.elements.filterStatusSelect) {
            this.elements.filterStatusSelect.addEventListener('change', () => {
                this.applyFilters({ resetPage: true });
            });
        }
        if (this.elements.sortSelect) {
            this.elements.sortSelect.addEventListener('change', () => {
                this.applyFilters({ resetPage: true });
            });
        }
        if (this.elements.prevPageBtn) {
            this.elements.prevPageBtn.addEventListener('click', () => {
                this.changePage(-1);
            });
        }
        if (this.elements.nextPageBtn) {
            this.elements.nextPageBtn.addEventListener('click', () => {
                this.changePage(1);
            });
        }
        if (this.elements.reindexFailedBtn) {
            this.elements.reindexFailedBtn.addEventListener('click', () => {
                this.reindexFailedAttachmentsForCurrentNode();
            });
        }
        if (this.elements.deleteFilteredBtn) {
            this.elements.deleteFilteredBtn.addEventListener('click', () => {
                this.deleteFilteredAttachmentsForNode();
            });
        }
    }

    setAvailability(available) {
        this.available = Boolean(available);
        if (this.elements.toolbar) {
            this.elements.toolbar.classList.toggle('is-disabled', !this.available);
        }
        if (this.elements.pagination) {
            this.elements.pagination.classList.toggle('is-disabled', !this.available);
        }
        const controls = [
            this.elements.filterQueryInput,
            this.elements.filterStatusSelect,
            this.elements.sortSelect,
            this.elements.filterApplyBtn,
            this.elements.reindexFailedBtn,
            this.elements.deleteFilteredBtn
        ];
        controls.forEach((control) => {
            if (control) {
                control.disabled = !this.available;
            }
        });
    }

    resetState() {
        this.state.query = '';
        this.state.status = 'all';
        this.state.sort = 'uploaded_at_desc';
        this.state.page = 1;
        this.state.total = 0;
        this.state.totalQueryMatched = 0;
        this.state.totalUnfiltered = 0;
        this.state.hasMore = false;
        this.state.statusFacets = null;
    }

    syncFilterInputs() {
        if (this.elements.filterQueryInput) {
            this.elements.filterQueryInput.value = this.state.query;
        }
        if (this.elements.filterStatusSelect) {
            this.elements.filterStatusSelect.value = this.state.status;
        }
        if (this.elements.sortSelect) {
            this.elements.sortSelect.value = this.state.sort;
        }
    }

    buildQueryParams({ includePaging = true } = {}) {
        const params = new URLSearchParams();
        const query = String(this.state.query || '').trim();
        if (query) {
            params.set('q', query);
        }
        const status = String(this.state.status || 'all').trim();
        if (status && status !== 'all') {
            params.set('status', status);
        }
        const sort = String(this.state.sort || 'uploaded_at_desc').trim();
        if (sort) {
            params.set('sort', sort);
        }
        if (includePaging) {
            const pageSize = Number(this.state.pageSize || 25);
            const page = Math.max(1, Number(this.state.page || 1));
            params.set('limit', String(pageSize));
            params.set('offset', String((page - 1) * pageSize));
        }
        return params;
    }

    renderSummary() {
        const page = Math.max(1, Number(this.state.page || 1));
        const total = Number(this.state.total || 0);
        const totalUnfiltered = Number(this.state.totalUnfiltered || total);
        const totalQueryMatched = Number(this.state.totalQueryMatched || total);
        const pageSize = Number(this.state.pageSize || 25);
        const hasPrev = page > 1;
        const hasNext = Boolean(this.state.hasMore);

        if (this.elements.pageLabel) {
            this.elements.pageLabel.textContent = `Page ${page}`;
        }
        if (this.elements.prevPageBtn) {
            this.elements.prevPageBtn.disabled = !hasPrev || !this.admin.currentEditingNode?.id;
        }
        if (this.elements.nextPageBtn) {
            this.elements.nextPageBtn.disabled = !hasNext || !this.admin.currentEditingNode?.id;
        }

        if (this.elements.listMeta) {
            if (!this.admin.currentEditingNode?.id) {
                this.elements.listMeta.textContent = 'Attachment triage is available when editing an existing node.';
            } else if (!this.available) {
                const shown = Array.isArray(this.admin.existingAttachments)
                    ? this.admin.existingAttachments.length
                    : 0;
                this.elements.listMeta.textContent =
                    `Attachment triage endpoints are unavailable on this server. Showing ${shown} item${shown === 1 ? '' : 's'}.`;
            } else {
                const start = total === 0 ? 0 : (page - 1) * pageSize + 1;
                const end = Math.min(page * pageSize, total);
                this.elements.listMeta.textContent = `${start}-${end} of ${total} shown • ${totalQueryMatched} matched • ${totalUnfiltered} total`;
            }
        }

        if (this.elements.statusFacets) {
            const facets = this.state.statusFacets || {
                all: totalQueryMatched || total,
                failed: 0,
                indexed: 0,
                transcribed: 0,
                tool_missing: 0
            };
            this.elements.statusFacets.innerHTML = `
                <span class="attachment-status-chip">all ${Number(facets.all || 0)}</span>
                <span class="attachment-status-chip">failed ${Number(facets.failed || 0)}</span>
                <span class="attachment-status-chip">indexed ${Number(facets.indexed || 0)}</span>
                <span class="attachment-status-chip">transcribed ${Number(facets.transcribed || 0)}</span>
                <span class="attachment-status-chip">tool_missing ${Number(facets.tool_missing || 0)}</span>
            `;
        }
    }

    async applyFilters({ resetPage = true } = {}) {
        if (!this.admin.currentEditingNode?.id) {
            return;
        }
        this.state.query = String(this.elements.filterQueryInput?.value || '').trim();
        this.state.status = String(this.elements.filterStatusSelect?.value || 'all');
        this.state.sort = String(this.elements.sortSelect?.value || 'uploaded_at_desc');
        if (resetPage) {
            this.state.page = 1;
        }
        await this.loadExistingAttachments(this.admin.currentEditingNode.id);
    }

    async changePage(delta) {
        if (!this.admin.currentEditingNode?.id) {
            return;
        }
        const nextPage = Math.max(1, Number(this.state.page || 1) + Number(delta || 0));
        if (nextPage === this.state.page) {
            return;
        }
        this.state.page = nextPage;
        await this.loadExistingAttachments(this.admin.currentEditingNode.id);
    }

    async loadExistingAttachments(nodeId, { resetPage = false } = {}) {
        if (!nodeId) {
            this.admin.existingAttachments = [];
            this.admin.attachmentChunksCache = new Map();
            this.resetState();
            this.syncFilterInputs();
            this.admin.renderAttachmentPreview();
            this.renderSummary();
            return;
        }

        if (resetPage) {
            this.state.page = 1;
        }

        try {
            const params = this.buildQueryParams({ includePaging: true });
            if (this.state.supportsPagedEndpoint) {
                try {
                    const page = await this.admin.apiCall(
                        `/api/v1/files/${encodeURIComponent(nodeId)}/paged?${params.toString()}`
                    );
                    this.setAvailability(true);
                    this.admin.existingAttachments = Array.isArray(page?.items) ? page.items : [];
                    this.state.total = Number(page?.total || 0);
                    this.state.totalQueryMatched = Number(page?.total_query_matched || this.state.total);
                    this.state.totalUnfiltered = Number(page?.total_unfiltered || this.state.total);
                    this.state.hasMore = Boolean(page?.has_more);
                    this.state.statusFacets = page?.status_facets || null;
                    this.state.sort = String(page?.sort || this.state.sort || 'uploaded_at_desc');
                    if (
                        this.admin.existingAttachments.length === 0
                        && this.state.total > 0
                        && Number(this.state.page || 1) > 1
                    ) {
                        this.state.page = Math.max(1, Number(this.state.page || 1) - 1);
                        await this.loadExistingAttachments(nodeId);
                        return;
                    }
                    this.syncFilterInputs();
                    this.admin.attachmentChunksCache = new Map();
                    this.admin.renderAttachmentPreview();
                    this.renderSummary();
                    return;
                } catch (pagedError) {
                    const unsupported = this.isUnsupportedEndpointError(pagedError);
                    if (unsupported) {
                        this.state.supportsPagedEndpoint = false;
                    }
                    // Fall back to legacy listing for any error; disable triage actions.
                    this.setAvailability(false);
                }
            }

            const fallbackParams = this.buildQueryParams({ includePaging: true });
            const items = await this.admin.apiCall(
                `/api/v1/files/${encodeURIComponent(nodeId)}?${fallbackParams.toString()}`
            );
            const pageSize = Number(this.state.pageSize || 25);
            const page = Math.max(1, Number(this.state.page || 1));
            const total = Array.isArray(items) ? items.length : 0;
            this.admin.existingAttachments = Array.isArray(items) ? items : [];
            this.state.total = total;
            this.state.totalQueryMatched = total;
            this.state.totalUnfiltered = total;
            this.state.hasMore = total >= pageSize && this.admin.existingAttachments.length >= pageSize;
            this.state.statusFacets = {
                all: total,
                failed: this.admin.existingAttachments.filter((item) =>
                    this.isFailedAttachmentStatus(item?.extraction_status)
                ).length,
                indexed: this.admin.existingAttachments.filter((item) =>
                    String(item?.extraction_status || '').toLowerCase().startsWith('indexed')
                ).length,
                transcribed: this.admin.existingAttachments.filter((item) =>
                    String(item?.extraction_status || '').toLowerCase() === 'transcribed'
                ).length,
                tool_missing: this.admin.existingAttachments.filter((item) =>
                    String(item?.extraction_status || '').toLowerCase() === 'tool_missing'
                ).length
            };
            if (this.admin.existingAttachments.length === 0 && page > 1) {
                this.state.page = Math.max(1, page - 1);
            }
            this.admin.attachmentChunksCache = new Map();
            this.admin.renderAttachmentPreview();
            this.renderSummary();
        } catch (error) {
            this.admin.existingAttachments = [];
            this.admin.attachmentChunksCache = new Map();
            this.state.total = 0;
            this.state.totalQueryMatched = 0;
            this.state.totalUnfiltered = 0;
            this.state.hasMore = false;
            this.state.statusFacets = null;
            this.admin.renderAttachmentPreview();
            this.renderSummary();
            console.error('Failed to load existing attachments:', error);
        }
    }

    isFailedAttachmentStatus(status) {
        const normalized = String(status || '').trim().toLowerCase();
        return normalized === 'tool_missing'
            || normalized === 'extraction_failed'
            || normalized === 'unsupported'
            || normalized === 'empty';
    }

    extractStatusCode(error) {
        const message = String(error?.message || '');
        const match = message.match(/HTTP\s+(\d{3})/i);
        if (match) {
            return Number(match[1]);
        }
        return null;
    }

    isUnsupportedEndpointError(error) {
        const status = this.extractStatusCode(error);
        if (status === 404 || status === 405 || status === 501) {
            return true;
        }
        const message = String(error?.message || '').toLowerCase();
        return message.includes('not found')
            || message.includes('method not allowed')
            || message.includes('not implemented');
    }

    buildBulkActionPayload() {
        const payload = {
            sort: String(this.state.sort || 'uploaded_at_desc')
        };
        const query = String(this.state.query || '').trim();
        if (query) {
            payload.q = query;
        }
        const status = String(this.state.status || 'all').trim();
        if (status && status !== 'all') {
            payload.status = status;
        }
        return payload;
    }

    async reindexFailedAttachmentsForCurrentNode() {
        const nodeId = this.admin.currentEditingNode?.id;
        if (!nodeId || !this.available) {
            return;
        }

        const trigger = this.elements.reindexFailedBtn;
        const originalLabel = trigger?.textContent || 'Reindex Failed';
        if (trigger) {
            trigger.disabled = true;
            trigger.textContent = 'Reindexing...';
        }

        try {
            const response = await this.admin.apiCall(
                `/api/v1/files/${encodeURIComponent(nodeId)}/reindex-failed`,
                { method: 'POST' }
            );
            await this.loadExistingAttachments(nodeId);
            this.admin.showNotification(
                `Batch reindex complete: ${Number(response?.reindexed || 0)} reindexed, ${Number(response?.failed || 0)} failed, ${Number(response?.skipped || 0)} skipped.`,
                response?.failed ? 'warning' : 'success'
            );
        } catch (error) {
            console.error('Failed to reindex failed attachments:', error);
            this.admin.showNotification('Unable to reindex failed attachments.', 'error');
        } finally {
            if (trigger) {
                trigger.disabled = false;
                trigger.textContent = originalLabel;
            }
        }
    }

    async deleteFilteredAttachmentsForNode() {
        const nodeId = this.admin.currentEditingNode?.id;
        if (!nodeId || !this.available) {
            return;
        }

        try {
            const basePayload = this.buildBulkActionPayload();
            const dryRun = await this.admin.apiCall(
                `/api/v1/files/${encodeURIComponent(nodeId)}/delete-filtered`,
                {
                    method: 'POST',
                    body: JSON.stringify({
                        ...basePayload,
                        dry_run: true
                    })
                }
            );
            const matchedCount = Number(dryRun?.matched_count || 0);
            if (matchedCount <= 0) {
                this.admin.showNotification('No attachments match the current filter.', 'warning');
                return;
            }

            const confirmationInput = window.prompt(
                `Delete ${matchedCount} attachment${matchedCount === 1 ? '' : 's'} matching the current filter.\nType ${matchedCount} to confirm.`
            );
            if (confirmationInput === null) {
                return;
            }
            if (Number(confirmationInput) !== matchedCount) {
                this.admin.showNotification('Confirmation count mismatch. Delete canceled.', 'warning');
                return;
            }

            const result = await this.admin.apiCall(
                `/api/v1/files/${encodeURIComponent(nodeId)}/delete-filtered`,
                {
                    method: 'POST',
                    body: JSON.stringify({
                        ...basePayload,
                        dry_run: false,
                        confirmed_count: matchedCount
                    })
                }
            );
            if (Number(this.state.page || 1) > 1 && Number(result?.remaining_attachments || 0) === 0) {
                this.state.page = 1;
            }
            await this.loadExistingAttachments(nodeId);
            const deletedCount = Number(result?.deleted_count || 0);
            const failedCount = Number(result?.failed_count || 0);
            this.admin.showNotification(
                `Deleted ${deletedCount} attachment${deletedCount === 1 ? '' : 's'}${failedCount ? `, ${failedCount} failed` : ''}.`,
                failedCount ? 'warning' : 'success'
            );
        } catch (error) {
            console.error('Failed to delete filtered attachments:', error);
            this.admin.showNotification('Unable to delete filtered attachments.', 'error');
        }
    }
}
