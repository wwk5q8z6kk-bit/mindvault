# MindVault Web Admin Interface

A modern web-based administrative interface for MindVault, providing an intuitive way to manage knowledge nodes, perform searches, visualize relationships, and monitor system statistics.

## Features

### Node Management
- **Browse Nodes**: Paginated list of all knowledge nodes with filtering by kind and namespace
- **Add Nodes**: Create new knowledge nodes with rich metadata
- **Edit Nodes**: Update existing nodes inline
- **Attachment Lifecycle**: Upload, inspect, and download node attachments directly from the editor
- **Attachment Text Indexing**: Text-like attachments are auto-indexed for improved search recall
- **Optional OCR/PDF Extraction**: Attachment indexing can use local `tesseract`/`pdftotext` when installed
- **Attachment Search Chunks**: Existing attachments now show chunk counts and indexed text preview snippets when extraction succeeds
- **Attachment Chunk Inspector**: “View Text” in attachment cards fetches paginated chunked extraction text for quick in-editor review
- **Attachment Reindex Action**: “Reindex” action lets you retry extraction/OCR without re-uploading files
- **Native Work Kinds**: Built-in `task` and `event` kinds for planning workflows
- **Task Scheduling Fields**: Due-at and recurrence controls in the node form (`daily|weekly|monthly`)
- **Filter & Search**: Real-time filtering and pagination
- **Rich Writing Workspace**: Markdown + WYSIWYG + split editing modes with formatting toolbar
- **AI Writing Assist**: Retrieval-assisted "suggest next" completions based on existing vault knowledge
- **Grounded Auto-Suggest Mode**: Optional background AI suggestions while typing with source-node provenance
- **AI Source Citations**: Suggestion provenance chips can be inserted as `[[...]]` citations directly into notes
- **AI Wiki-Link Assist**: Semantic `[[...]]` link suggestions from vault context with keyboard navigation, heading deep links, and inline preview snippets
- **AI Mention Assist**: Semantic `@...` / `@"Title"` mention suggestions in both Markdown and WYSIWYG modes with the same keyboard-driven picker
- **AI Transform Actions**: One-click summarize/action-items/refine transforms inserted as sections or used to replace selected Markdown
- **Quick Pause Controls**: Temporarily pause auto-suggest with one click and resume later
- **Template Workflows UI**: Browse, filter, create, and instantiate reusable templates with variable substitution
- **Template Version History**: Inspect historical versions and restore previous template revisions
- **Field-Level Diff Preview**: Compare historical version fields (title/tags/source/importance/etc.) against current template before restore
- **Curated Template Packs**: Install built-in packs (daily flow, project execution, meeting system) into any namespace
- **Node Version History**: Non-template node edits now keep restorable history with inline diff preview directly in the editor modal
- **Relationship Context Inspector**: Editor modal now surfaces incoming/outgoing graph links (kind, namespace, weight, auto-managed markers, and auto-link source provenance)
- **Relationship Jump Navigation**: Click a related-node title in relationship context to jump directly into that node
- **Public Share Links**: Create read-only share links per node with optional expiry and revoke controls

### Daily Notes
- **Daily Notes Workspace**: Dedicated tab for daily planning/journaling notes
- **Ensure-on-Demand**: Create today's note or a selected date if missing (idempotent API behavior)
- **Template-Backed Notes**: Uses engine-level template and namespace configuration
- **Fast Edit Loop**: Click a daily note card to open and edit in the rich writing modal
- **Auto-Linking Hooks**: Nodes tagged as task/event (`task`, `todo`, `event`, `meeting`, etc.) are auto-linked to the day's note by the engine
- **Kind-Based Auto-Linking**: Native `task`/`event` nodes are auto-linked to the day's note even without tags
- **Linked Item Inspector**: Daily Notes tab surfaces graph-linked work items for the selected day
- **Due Task Dashboard**: Daily Notes tab includes due-task listing with due cutoff, completion filtering, and one-click completion toggles

### Calendar Workspace
- **Day/Week/Month Views**: Timeline-style views backed by scheduled task/event metadata
- **Range Navigation**: Previous/next/today controls with date anchoring
- **Completion-Aware Filters**: Hide or include completed tasks without affecting events
- **Context Capture**: “Add Event” action opens the editor with calendar namespace/date prefilled
- **iCal Export**: Download current calendar window as `.ics` for external calendar tools
- **iCal Import**: Upload `.ics` files into a namespace with optional overwrite behavior

### Search Interface
- **Hybrid Search**: Combine vector, full-text, and graph-based search strategies
- **Result Scoring**: View relevance scores and match sources
- **Configurable Limits**: Control result set sizes
- **Saved Search Workbench**: Save, rerun, load, and delete recurring queries from the Search tab

### Graph Visualization
- **Relationship Explorer**: Load and visualize node relationships
- **Depth Control**: Configure traversal depth for relationship discovery
- **Interactive Display**: Click-to-explore connected nodes

### System Monitoring
- **Health Status**: Real-time system health checks
- **Statistics**: Node counts, version information, and system metrics
- **Performance Insights**: Monitor system performance and usage
- **Vault Transfer Controls**: One-click JSON export/import from the Stats tab
- **Audit Activity Feed**: Filterable audit trail (subject/action/since/limit) in the Stats tab for admin sessions
- **Audit Pagination**: Incremental "Load More" audit retrieval for large activity streams
- **Runtime Settings**: Configure API base, suggestion toggles, and auto-suggest cooldown from the Stats tab
- **Panel Status Indicators**: Each tab shows loading state and last refresh timestamp
- **Reset Confirmation Toggle**: Optional confirm dialog for per-tab reset actions

## Getting Started

### Prerequisites
- MindVault server running on `http://127.0.0.1:9470` (default)
- Modern web browser with JavaScript enabled

### Installation (Bundled)
1. Ensure the MindVault server is running
2. From `web/`, install dependencies: `npm install`
3. Run the dev server: `npm run dev`
4. Open the printed local URL (Vite default is `http://localhost:5173`)

### Installation (Static Build)
1. Ensure the MindVault server is running
2. From `web/`, install dependencies: `npm install`
3. Build: `npm run build`
4. Open `web/dist/index.html` or serve `web/dist/` via a static server

### CORS Configuration
If running the web interface from a different origin, configure CORS in your MindVault server:

```toml
[server]
cors_allowed_origins = ["http://localhost:3000", "http://localhost:8080"]
```

## Usage

### Managing Nodes
1. **View Nodes**: Click the "Nodes" tab to browse existing knowledge nodes
2. **Add Node**: Click "Add Node" to create a new knowledge entry
3. **Edit Node**: Click any node in the list to edit its content
4. **Filter**: Use the dropdown and text filters to narrow down the node list (filters persist across reloads)
5. **Link Faster**: In Markdown or WYSIWYG mode type `[[` (or `[[target#heading|alias`) for wiki-links, or `@` / `@"Title"` for mentions, then use `Up`/`Down` + `Enter`/`Tab` (or click "AI Suggest Links")
6. **Transform Drafts**: Pick AI transform mode + target (`Insert Section` or `Replace Selection`) and click "AI Transform"
7. **Control Suggestion Flow**: Toggle `Auto Suggest` on/off for background completions with grounding metadata

### Searching Knowledge
1. **Basic Search**: Enter a query in the search box
2. **Advanced Options**: Select search strategy and result limit
3. **Configure Saved Filters**: Optionally set namespace/kinds/tags/min-score/min-importance
4. **Save Reusable Queries**: Provide a name and click "Save Current Search"
5. **Run Saved Queries**: Use "Run" to execute stored filters instantly
6. **Persistent Inputs**: Search form and filter inputs are remembered between sessions
7. **Reset Tab Inputs**: Each tab now has a “Reset Tab” action to clear only that panel’s saved inputs
8. **Quick Reset Shortcut**: Use `Ctrl/Cmd + Alt + R` to reset the active tab
9. **Update Active Presets**: Load a saved query, edit fields, then click "Update Active"
10. **View Results**: Browse scored results with match source information

### Working with Templates
1. **Open Templates tab**: Review template cards by namespace/kind filters
2. **Create Template**: Define content with `{{variable}}` placeholders
3. **Instantiate**: Select a template, fill variable values, and create a concrete node instance

### Exploring Relationships
1. **Load Graph**: Enter a node ID and desired depth
2. **Visualize**: View connected nodes and their relationships
3. **Navigate**: Click on connected nodes to explore further

### Monitoring System
1. **Health Check**: View real-time system status
2. **Statistics**: Monitor node counts and system metrics
3. **Performance**: Track system performance indicators

## API Integration

The web interface communicates with MindVault's REST API endpoints:

- `GET /api/v1/nodes` - List and filter nodes
- `POST /api/v1/nodes` - Create new nodes
- `PUT /api/v1/nodes/{id}` - Update existing nodes
- `GET /api/v1/nodes/{id}/versions` - List saved versions for a non-template node
- `GET /api/v1/nodes/{id}/versions/{version_id}` - Inspect a saved node version with diff summary vs current node
- `POST /api/v1/nodes/{id}/versions/{version_id}/restore` - Restore a non-template node from version history
- `GET /api/v1/search` - Perform knowledge searches
- `GET /api/v1/search/saved` - List saved search definitions
- `POST /api/v1/search/saved` - Create a saved search definition
- `PUT /api/v1/search/saved/{id}` - Update a saved search definition
- `DELETE /api/v1/search/saved/{id}` - Delete a saved search definition
- `POST /api/v1/search/saved/{id}/run` - Execute a saved search
- `GET /api/v1/graph/relationships/{id}` - Retrieve incoming/outgoing relationship context for a node
- `GET /api/v1/graph/neighbors/{id}` - Load node relationships
- `GET /api/v1/health` - System health and statistics
- `POST /api/v1/assist/completion` - Retrieval-assisted writing completion suggestions (includes top grounding sources for citation chips)
- `POST /api/v1/assist/autocomplete` - Retrieval-assisted inline autocomplete completions
- `POST /api/v1/assist/links` - Semantic wiki-link suggestions for editor insertion (includes optional heading previews)
- `POST /api/v1/assist/transform` - Retrieval-assisted markdown transforms (`summarize`, `action_items`, `refine`)
- `GET /api/v1/diagnostics/embedding` - Active embedding provider diagnostics
- `GET /api/v1/daily-notes` - List daily notes by namespace/date
- `POST /api/v1/daily-notes/ensure` - Ensure a daily note exists for a date/namespace
- `GET /api/v1/calendar/items` - List scheduled task/event items for day/week/month or custom ranges
- `GET /api/v1/calendar/ical` - Export scheduled task/event items as iCal (`.ics`)
- `POST /api/v1/calendar/ical/import` - Import `.ics` events into task/event nodes
- `GET /api/v1/tasks/due` - List due tasks with due-time and completion filters
- `POST /api/v1/tasks/prioritize` - Generate ranked focus list using deterministic AI-priority heuristics
- `POST /api/v1/tasks/{id}/complete` - Mark a task complete
- `POST /api/v1/tasks/{id}/reopen` - Mark a task active again
- `GET /api/v1/template-packs` - List built-in curated template packs
- `POST /api/v1/template-packs/{pack_id}/install` - Install a curated template pack into a namespace
- `GET /api/v1/templates` - List reusable templates
- `POST /api/v1/templates` - Create a reusable template
- `DELETE /api/v1/templates/{id}` - Delete a reusable template
- `POST /api/v1/templates/{id}/duplicate` - Duplicate a template into a new template node
- `GET /api/v1/templates/{id}/versions` - List saved template versions
- `GET /api/v1/templates/{id}/versions/{version_id}` - Inspect a template version with diff summary vs current template
- `POST /api/v1/templates/{id}/versions/{version_id}/restore` - Restore a template from version history
- `POST /api/v1/templates/{id}/instantiate` - Create a new node instance from a template
- `GET /api/v1/export` - Export nodes/relationships to portable JSON
- `POST /api/v1/import` - Import nodes/relationships from portable JSON
- `GET /api/v1/audit` - List audit entries with filters (`limit`, `offset`, `subject`, `action`, `since`)
- `POST /api/v1/files/upload` - Upload a file attachment to a node (multipart)
- `GET /api/v1/files/{node_id}` - List existing attachments for a node (includes extraction status/chars and optional search chunk preview metadata)
- `GET /api/v1/files/{node_id}/{attachment_id}/chunks` - Retrieve paginated indexed text chunks for an attachment
- `POST /api/v1/files/{node_id}/{attachment_id}/reindex` - Re-run attachment extraction/OCR and refresh indexed metadata
- `GET /api/v1/files/{node_id}/{attachment_id}` - Download an attachment (`?inline=true` for preview)
- `DELETE /api/v1/files/{node_id}/{attachment_id}` - Remove an attachment from a node
- `POST /api/v1/shares` - Create a public share link for a node
- `GET /api/v1/shares` - List public share links (filter by `node_id`)
- `DELETE /api/v1/shares/{id}` - Revoke a share link
- `GET /public/shares/{token}` - Read-only public share content

## Browser Support

- Chrome 90+
- Firefox 88+
- Safari 14+
- Edge 90+

## Development

### File Structure
```
web/
├── index.html        # Main HTML shell
├── styles.css        # Global styling
├── src/
│   ├── main.js       # App entrypoint (bootstraps admin)
│   ├── admin.js      # MindVault admin UI logic
│   └── attachments.js# Attachment triage module
├── dist/             # Bundled output (generated)
└── README.md         # This documentation
```

### Customization
- **Styling**: Modify `styles.css` for visual customization
- **Functionality**: Extend `src/admin.js` for additional features
- **API Endpoints**: Update `apiBase` in `src/admin.js` for different server locations

### Adding New Features
1. Add HTML elements to `index.html`
2. Style components in `styles.css`
3. Implement functionality in `src/admin.js` (or split into new modules under `src/`)
4. Bind events in the `bindEvents()` method

## Development Notes

- The admin UI uses a Vite build so dependencies are pinned locally (no CDN runtime).
- For offline-first use, deploy from `web/dist/` or serve `web/dist/` via a static server.

## Security Considerations

- The web interface assumes a trusted local environment
- For production use, implement proper authentication and HTTPS
- Configure CORS appropriately for your deployment scenario
- Consider implementing CSRF protection for state-changing operations

## Troubleshooting

### Connection Issues
- Verify MindVault server is running on the expected port
- Check CORS configuration if accessing from different origins
- Review browser console for JavaScript errors

### Performance Issues
- Large node lists may impact browser performance
- Consider implementing virtual scrolling for extensive datasets
- Monitor network requests in browser developer tools

### Authentication Errors
- Ensure proper authentication headers are configured
- Check JWT token validity and expiration
- Verify role-based access permissions

## Contributing

When contributing to the web interface:
1. Follow the existing code style and structure
2. Test functionality across supported browsers
3. Ensure responsive design works on mobile devices
4. Add appropriate error handling and user feedback
5. Update this documentation for new features
