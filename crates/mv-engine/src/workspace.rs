use std::collections::{HashMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Take};
use std::path::Path;

use chrono::{DateTime, Utc};
use mv_core::{
    KnowledgeWorkspace, KnowledgeWorkspaceDocument, KnowledgeWorkspaceManifestStore,
    KnowledgeWorkspaceState, MvError, MvResult, PortableWorkspacePathPolicy,
    WorkspaceDescriptorPayloadV1, WorkspaceDocumentContentStatus, WorkspaceDocumentLifecycle,
    WorkspaceDocumentManifestUpdate, WorkspaceDocumentPayloadV1, WorkspaceFileIdentityHint,
    WorkspaceManifestPayloadFormat, WorkspaceManifestReconciliation, WorkspacePathAssessment,
    WorkspaceProjectionState,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceScanDiagnosticSeverity {
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceScanDiagnostic {
    pub severity: WorkspaceScanDiagnosticSeverity,
    pub code: String,
    pub relative_path: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct WorkspaceScanConfig {
    pub max_documents: usize,
    pub max_document_bytes: u64,
}

impl Default for WorkspaceScanConfig {
    fn default() -> Self {
        Self {
            max_documents: 10_000,
            max_document_bytes: 16 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScannedWorkspaceDocument {
    pub path_token: String,
    pub path_assessment: WorkspacePathAssessment,
    pub payload: WorkspaceDocumentPayloadV1,
    pub content: Option<String>,
    pub lifecycle: WorkspaceDocumentLifecycle,
}

#[derive(Debug, Clone)]
pub struct ScannedWorkspaceDirectory {
    pub path_assessment: WorkspacePathAssessment,
}

#[derive(Debug, Clone)]
pub struct WorkspaceScan {
    pub directories: Vec<ScannedWorkspaceDirectory>,
    pub documents: Vec<ScannedWorkspaceDocument>,
    pub diagnostics: Vec<WorkspaceScanDiagnostic>,
    pub completed_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct WorkspaceScanner {
    config: WorkspaceScanConfig,
    path_policy: PortableWorkspacePathPolicy,
}

impl Default for WorkspaceScanner {
    fn default() -> Self {
        Self::new(WorkspaceScanConfig::default())
    }
}

impl WorkspaceScanner {
    pub fn new(config: WorkspaceScanConfig) -> Self {
        Self {
            config,
            path_policy: PortableWorkspacePathPolicy::new(),
        }
    }

    /// Read a workspace without mutating canonical files or creating metadata
    /// inside the mounted root.
    pub fn scan(&self, workspace: &KnowledgeWorkspace, root: &Path) -> MvResult<WorkspaceScan> {
        if workspace.payload_format != WorkspaceManifestPayloadFormat::JsonV1 {
            return Err(MvError::InvalidInput(
                "read-only workspace scanning requires a decrypted json-v1 manifest view".into(),
            ));
        }

        let canonical_root = fs::canonicalize(root).map_err(|error| {
            MvError::InvalidInput(format!("workspace root unavailable: {error}"))
        })?;
        if !canonical_root.is_dir() {
            return Err(MvError::InvalidInput(
                "workspace root must be a directory".into(),
            ));
        }

        let mut directories = Vec::new();
        let mut documents = Vec::new();
        let mut diagnostics = Vec::new();
        let mut walker = WalkDir::new(&canonical_root)
            .follow_links(false)
            .into_iter();

        while let Some(entry) = walker.next() {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    diagnostics.push(diagnostic(
                        WorkspaceScanDiagnosticSeverity::Error,
                        "walk_error",
                        error
                            .path()
                            .and_then(|path| relative_display(&canonical_root, path)),
                        format!("could not inspect workspace entry: {error}"),
                    ));
                    continue;
                }
            };
            if entry.depth() == 0 {
                continue;
            }

            let relative = match entry.path().strip_prefix(&canonical_root) {
                Ok(relative) => relative,
                Err(_) => {
                    diagnostics.push(diagnostic(
                        WorkspaceScanDiagnosticSeverity::Error,
                        "root_escape",
                        None,
                        "workspace entry did not remain beneath the mounted root".into(),
                    ));
                    continue;
                }
            };
            let relative_text = match relative.to_str() {
                Some(_) => slash_separated(relative),
                None => {
                    diagnostics.push(diagnostic(
                        WorkspaceScanDiagnosticSeverity::Error,
                        "non_utf8_path",
                        Some(relative.to_string_lossy().into_owned()),
                        "non-UTF-8 paths are quarantined from content access".into(),
                    ));
                    if entry.file_type().is_dir() {
                        walker.skip_current_dir();
                    }
                    continue;
                }
            };

            if entry.file_type().is_dir() && should_skip_directory(&relative_text) {
                if is_mindvault_reserved_path(&relative_text) {
                    diagnostics.push(diagnostic(
                        WorkspaceScanDiagnosticSeverity::Warning,
                        "reserved_internal_path",
                        Some(relative_text),
                        "reserved MindVault paths are excluded from canonical content".into(),
                    ));
                }
                walker.skip_current_dir();
                continue;
            }

            if entry.file_type().is_symlink() {
                diagnostics.push(diagnostic(
                    WorkspaceScanDiagnosticSeverity::Warning,
                    "symlink_skipped",
                    Some(relative_text),
                    "symbolic links are observed but never traversed by the scanner".into(),
                ));
                continue;
            }
            if entry.file_type().is_dir() {
                directories.push(ScannedWorkspaceDirectory {
                    path_assessment: self.path_policy.assess_existing(&relative_text),
                });
                continue;
            }
            if !entry.file_type().is_file()
                || !self
                    .path_policy
                    .has_supported_markdown_extension(&relative_text)
            {
                continue;
            }

            if documents.len() >= self.config.max_documents {
                diagnostics.push(diagnostic(
                    WorkspaceScanDiagnosticSeverity::Error,
                    "document_limit_reached",
                    None,
                    format!(
                        "scan stopped after {} Markdown documents",
                        self.config.max_documents
                    ),
                ));
                break;
            }

            let assessment = self.path_policy.assess_existing(&relative_text);
            if !assessment.is_safe_to_read() {
                diagnostics.push(diagnostic(
                    WorkspaceScanDiagnosticSeverity::Error,
                    "unsafe_path",
                    Some(relative_text),
                    "unsafe path was quarantined from content access".into(),
                ));
                continue;
            }
            if !assessment.is_portable() {
                diagnostics.push(diagnostic(
                    WorkspaceScanDiagnosticSeverity::Warning,
                    "nonportable_path",
                    Some(relative_text.clone()),
                    "existing document is readable but cannot be created on every supported host"
                        .into(),
                ));
            }

            match self.scan_document(&canonical_root, entry.path(), assessment) {
                Ok(document) => documents.push(document),
                Err(error) => diagnostics.push(diagnostic(
                    WorkspaceScanDiagnosticSeverity::Error,
                    "content_read_failed",
                    Some(relative_text),
                    error.to_string(),
                )),
            }
        }

        directories.sort_by(|left, right| {
            left.path_assessment
                .normalized_relative_path
                .cmp(&right.path_assessment.normalized_relative_path)
        });
        documents
            .sort_by(|left, right| left.payload.relative_path.cmp(&right.payload.relative_path));
        detect_path_collisions(&mut documents, &mut diagnostics);

        Ok(WorkspaceScan {
            directories,
            documents,
            diagnostics,
            completed_at: Utc::now(),
        })
    }

    fn scan_document(
        &self,
        canonical_root: &Path,
        path: &Path,
        assessment: WorkspacePathAssessment,
    ) -> MvResult<ScannedWorkspaceDocument> {
        let metadata = fs::symlink_metadata(path)
            .map_err(|error| MvError::Storage(format!("read file metadata failed: {error}")))?;
        if !metadata.file_type().is_file() {
            return Err(MvError::InvalidInput(
                "workspace entry changed type during scanning".into(),
            ));
        }

        let observed_at = Utc::now();
        let mut payload = WorkspaceDocumentPayloadV1::new(
            assessment.normalized_relative_path.clone(),
            observed_at,
        );
        payload.byte_size = metadata.len();
        payload.modified_at = metadata.modified().ok().map(DateTime::<Utc>::from);
        payload.file_identity_hint = file_identity_hint(&metadata);
        payload.portability_issues = assessment.issues.clone();

        let mut content = None;
        let lifecycle;
        if metadata.len() > self.config.max_document_bytes {
            payload.content_status = WorkspaceDocumentContentStatus::TooLarge;
            payload.reconciliation_note = Some(format!(
                "document exceeds the configured {} byte indexing limit",
                self.config.max_document_bytes
            ));
            lifecycle = WorkspaceDocumentLifecycle::Unsupported;
        } else {
            let bytes = read_regular_file_without_following_links(
                canonical_root,
                path,
                self.config.max_document_bytes,
            )?;
            payload.byte_size = bytes.len() as u64;
            payload.content_hash = Some(sha256_content_hash(&bytes));
            match String::from_utf8(bytes) {
                Ok(value) => {
                    content = Some(value);
                    payload.content_status = WorkspaceDocumentContentStatus::Utf8;
                    lifecycle = WorkspaceDocumentLifecycle::Active;
                }
                Err(_) => {
                    payload.content_status = WorkspaceDocumentContentStatus::InvalidUtf8;
                    payload.reconciliation_note =
                        Some("document bytes are not valid UTF-8".to_string());
                    lifecycle = WorkspaceDocumentLifecycle::Unsupported;
                }
            }
        }

        Ok(ScannedWorkspaceDocument {
            path_token: plaintext_path_token(&assessment.normalized_relative_path),
            path_assessment: assessment,
            payload,
            content,
            lifecycle,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceTreeEntryKind {
    Directory,
    Document,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceManifestMatch {
    Current,
    Changed,
    Untracked,
}

/// One flat, parent-addressable tree row.
///
/// A flat representation avoids recursive response depth and lets clients
/// virtualize large workspaces while still rendering a conventional tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceTreeEntry {
    pub kind: WorkspaceTreeEntryKind,
    pub relative_path: String,
    pub parent_path: Option<String>,
    pub name: String,
    pub document_id: Option<uuid::Uuid>,
    pub lifecycle: Option<WorkspaceDocumentLifecycle>,
    pub projection_state: Option<WorkspaceProjectionState>,
    pub projected_node_id: Option<uuid::Uuid>,
    pub content_status: Option<WorkspaceDocumentContentStatus>,
    pub manifest_match: Option<WorkspaceManifestMatch>,
    pub content_hash: Option<String>,
    pub byte_size: Option<u64>,
    pub modified_at: Option<DateTime<Utc>>,
    pub portable: bool,
    pub portability_issues: Vec<mv_core::WorkspacePathIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceTree {
    pub workspace_id: uuid::Uuid,
    pub workspace_revision: u64,
    pub scanned_at: DateTime<Utc>,
    pub entries: Vec<WorkspaceTreeEntry>,
    pub diagnostics: Vec<WorkspaceScanDiagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceDocumentRead {
    pub workspace_id: uuid::Uuid,
    pub document_id: uuid::Uuid,
    pub relative_path: String,
    pub content: String,
    pub content_hash: String,
    pub byte_size: u64,
    pub modified_at: Option<DateTime<Utc>>,
    pub manifest_match: WorkspaceManifestMatch,
}

pub fn decode_workspace_descriptor(
    workspace: &KnowledgeWorkspace,
) -> MvResult<WorkspaceDescriptorPayloadV1> {
    if workspace.payload_format != WorkspaceManifestPayloadFormat::JsonV1 {
        return Err(MvError::InvalidInput(
            "workspace descriptor requires a decrypted json-v1 view".into(),
        ));
    }
    let descriptor =
        serde_json::from_slice::<WorkspaceDescriptorPayloadV1>(&workspace.descriptor_payload)
            .map_err(|error| {
                MvError::InvalidInput(format!("workspace descriptor is invalid: {error}"))
            })?;
    if !descriptor.has_supported_schema() {
        return Err(MvError::InvalidInput(
            "workspace descriptor schema is unsupported".into(),
        ));
    }
    Ok(descriptor)
}

pub fn build_workspace_tree(
    workspace: &KnowledgeWorkspace,
    manifest_documents: &[KnowledgeWorkspaceDocument],
    scan: WorkspaceScan,
) -> WorkspaceTree {
    let manifest_by_token = manifest_documents
        .iter()
        .map(|document| (document.path_token.as_str(), document))
        .collect::<HashMap<_, _>>();
    let mut entries = Vec::with_capacity(scan.directories.len() + scan.documents.len());

    for directory in scan.directories {
        let relative_path = directory.path_assessment.normalized_relative_path.clone();
        entries.push(WorkspaceTreeEntry {
            kind: WorkspaceTreeEntryKind::Directory,
            parent_path: parent_path(&relative_path),
            name: entry_name(&relative_path),
            relative_path,
            document_id: None,
            lifecycle: None,
            projection_state: None,
            projected_node_id: None,
            content_status: None,
            manifest_match: None,
            content_hash: None,
            byte_size: None,
            modified_at: None,
            portable: directory.path_assessment.is_portable(),
            portability_issues: directory.path_assessment.issues,
        });
    }

    for observation in scan.documents {
        let manifest = manifest_by_token
            .get(observation.path_token.as_str())
            .copied();
        let manifest_match = manifest
            .map(|document| {
                let stored_hash = serde_json::from_slice::<WorkspaceDocumentPayloadV1>(
                    &document.document_payload,
                )
                .ok()
                .and_then(|payload| payload.content_hash);
                if stored_hash == observation.payload.content_hash {
                    WorkspaceManifestMatch::Current
                } else {
                    WorkspaceManifestMatch::Changed
                }
            })
            .unwrap_or(WorkspaceManifestMatch::Untracked);
        let relative_path = observation.payload.relative_path.clone();
        entries.push(WorkspaceTreeEntry {
            kind: WorkspaceTreeEntryKind::Document,
            parent_path: parent_path(&relative_path),
            name: entry_name(&relative_path),
            relative_path,
            document_id: manifest.map(|document| document.id),
            lifecycle: Some(observation.lifecycle),
            projection_state: manifest.map(|document| document.projection_state),
            projected_node_id: manifest.and_then(|document| document.projected_node_id),
            content_status: Some(observation.payload.content_status),
            manifest_match: Some(manifest_match),
            content_hash: observation.payload.content_hash,
            byte_size: Some(observation.payload.byte_size),
            modified_at: observation.payload.modified_at,
            portable: observation.path_assessment.is_portable(),
            portability_issues: observation.path_assessment.issues,
        });
    }

    entries.sort_by(|left, right| {
        left.relative_path
            .cmp(&right.relative_path)
            .then_with(|| tree_kind_rank(left.kind).cmp(&tree_kind_rank(right.kind)))
    });

    WorkspaceTree {
        workspace_id: workspace.id,
        workspace_revision: workspace.revision,
        scanned_at: scan.completed_at,
        entries,
        diagnostics: scan.diagnostics,
    }
}

pub fn read_workspace_document_from_scan(
    workspace: &KnowledgeWorkspace,
    manifest_document: &KnowledgeWorkspaceDocument,
    scan: WorkspaceScan,
) -> MvResult<WorkspaceDocumentRead> {
    if manifest_document.workspace_id != workspace.id {
        return Err(MvError::InvalidInput(
            "document does not belong to the requested workspace".into(),
        ));
    }
    let observation = scan
        .documents
        .into_iter()
        .find(|document| document.path_token == manifest_document.path_token)
        .ok_or(MvError::NodeNotFound(manifest_document.id))?;
    if observation.lifecycle != WorkspaceDocumentLifecycle::Active {
        return Err(MvError::InvalidInput(
            "workspace document is not readable in its current lifecycle state".into(),
        ));
    }
    let content = observation.content.ok_or_else(|| {
        MvError::InvalidInput("workspace document has no readable UTF-8 content".into())
    })?;
    let content_hash = observation.payload.content_hash.clone().ok_or_else(|| {
        MvError::Internal("readable workspace document is missing its content hash".into())
    })?;
    let stored_hash =
        serde_json::from_slice::<WorkspaceDocumentPayloadV1>(&manifest_document.document_payload)
            .ok()
            .and_then(|payload| payload.content_hash);
    let manifest_match = if stored_hash.as_deref() == Some(content_hash.as_str()) {
        WorkspaceManifestMatch::Current
    } else {
        WorkspaceManifestMatch::Changed
    };

    Ok(WorkspaceDocumentRead {
        workspace_id: workspace.id,
        document_id: manifest_document.id,
        relative_path: observation.payload.relative_path,
        content,
        content_hash,
        byte_size: observation.payload.byte_size,
        modified_at: observation.payload.modified_at,
        manifest_match,
    })
}

fn parent_path(relative_path: &str) -> Option<String> {
    relative_path
        .rsplit_once('/')
        .map(|(parent, _)| parent.to_string())
}

fn entry_name(relative_path: &str) -> String {
    relative_path
        .rsplit('/')
        .next()
        .unwrap_or(relative_path)
        .to_string()
}

const fn tree_kind_rank(kind: WorkspaceTreeEntryKind) -> u8 {
    match kind {
        WorkspaceTreeEntryKind::Directory => 0,
        WorkspaceTreeEntryKind::Document => 1,
    }
}

#[derive(Debug, Clone)]
pub struct WorkspaceReconciliationPlan {
    pub manifest: WorkspaceManifestReconciliation,
    pub diagnostics: Vec<WorkspaceScanDiagnostic>,
    pub unchanged_documents: usize,
    pub renamed_documents: usize,
}

#[derive(Debug, Clone)]
pub struct WorkspaceReconciliationOutcome {
    pub applied: bool,
    pub diagnostics: Vec<WorkspaceScanDiagnostic>,
    pub inserted_documents: usize,
    pub updated_documents: usize,
    pub unchanged_documents: usize,
    pub renamed_documents: usize,
    pub projection: crate::engine::WorkspaceProjectionOutcome,
}

pub struct WorkspaceReconciler<'a, S: ?Sized> {
    store: &'a S,
    scanner: WorkspaceScanner,
}

impl<'a, S> WorkspaceReconciler<'a, S>
where
    S: KnowledgeWorkspaceManifestStore + ?Sized,
{
    pub fn new(store: &'a S, scanner: WorkspaceScanner) -> Self {
        Self { store, scanner }
    }

    pub async fn plan(
        &self,
        workspace: &KnowledgeWorkspace,
        root: &Path,
    ) -> MvResult<WorkspaceReconciliationPlan> {
        let scan = self.scanner.scan(workspace, root)?;
        let existing = self.store.list_workspace_documents(workspace.id).await?;
        build_reconciliation_plan(workspace, existing, scan)
    }

    pub async fn reconcile(
        &self,
        workspace: &KnowledgeWorkspace,
        root: &Path,
    ) -> MvResult<WorkspaceReconciliationOutcome> {
        let plan = self.plan(workspace, root).await?;
        let inserted_documents = plan.manifest.document_inserts.len();
        let updated_documents = plan.manifest.document_updates.len();
        let applied = self
            .store
            .apply_workspace_reconciliation(&plan.manifest)
            .await?;

        Ok(WorkspaceReconciliationOutcome {
            applied,
            diagnostics: plan.diagnostics,
            inserted_documents,
            updated_documents,
            unchanged_documents: plan.unchanged_documents,
            renamed_documents: plan.renamed_documents,
            projection: crate::engine::WorkspaceProjectionOutcome::default(),
        })
    }
}

pub fn build_reconciliation_plan(
    workspace: &KnowledgeWorkspace,
    existing_documents: Vec<KnowledgeWorkspaceDocument>,
    scan: WorkspaceScan,
) -> MvResult<WorkspaceReconciliationPlan> {
    if workspace.payload_format != WorkspaceManifestPayloadFormat::JsonV1 {
        return Err(MvError::InvalidInput(
            "reconciliation planning requires decrypted json-v1 manifest payloads".into(),
        ));
    }

    let WorkspaceScan {
        directories: _,
        documents,
        mut diagnostics,
        completed_at,
    } = scan;
    let mut inserts = Vec::new();
    let mut updates = Vec::new();
    let mut unchanged_documents = 0;
    let mut renamed_documents = 0;
    let mut matched_ids = HashSet::new();
    let observed_tokens = documents
        .iter()
        .map(|document| document.path_token.clone())
        .collect::<HashSet<_>>();
    if observed_tokens.len() != documents.len() {
        return Err(MvError::InvalidInput(
            "workspace reconciliation blocked by a normalized path collision".into(),
        ));
    }
    let by_token = existing_documents
        .iter()
        .map(|document| (document.path_token.as_str(), document))
        .collect::<HashMap<_, _>>();
    let parsed_existing = existing_documents
        .iter()
        .filter_map(|document| {
            serde_json::from_slice::<WorkspaceDocumentPayloadV1>(&document.document_payload)
                .ok()
                .filter(WorkspaceDocumentPayloadV1::has_supported_schema)
                .map(|payload| (document.id, payload))
        })
        .collect::<HashMap<_, _>>();
    let unmatched_observations = documents
        .iter()
        .filter(|document| !by_token.contains_key(document.path_token.as_str()))
        .collect::<Vec<_>>();
    let observed_identity_counts = unmatched_observations
        .iter()
        .filter_map(|document| {
            document
                .payload
                .file_identity_hint
                .as_ref()
                .map(file_identity_key)
        })
        .fold(HashMap::<String, usize>::new(), |mut counts, key| {
            *counts.entry(key).or_default() += 1;
            counts
        });
    let observed_hash_counts = unmatched_observations
        .iter()
        .filter_map(|document| document.payload.content_hash.clone())
        .fold(HashMap::<String, usize>::new(), |mut counts, hash| {
            *counts.entry(hash).or_default() += 1;
            counts
        });

    for observation in documents {
        if let Some(existing) = by_token.get(observation.path_token.as_str()) {
            matched_ids.insert(existing.id);
            if let Some(update) = replacement_if_changed(existing, &observation, false)? {
                updates.push(update);
            } else {
                unchanged_documents += 1;
            }
            continue;
        }

        let rename_match = rename_candidate(
            &observation,
            &existing_documents,
            &parsed_existing,
            &observed_tokens,
            &matched_ids,
            &observed_identity_counts,
            &observed_hash_counts,
        );
        match rename_match {
            RenameCandidateMatch::Unique(existing) => {
                matched_ids.insert(existing.id);
                updates.push(
                    replacement_if_changed(existing, &observation, true)?.ok_or_else(|| {
                        MvError::Internal("rename replacement unexpectedly unchanged".into())
                    })?,
                );
                renamed_documents += 1;
            }
            RenameCandidateMatch::Ambiguous {
                manifest_candidates,
                observations,
            } => {
                let mut observation = observation;
                observation.lifecycle = WorkspaceDocumentLifecycle::Conflict;
                observation.payload.reconciliation_note = Some(format!(
                    "ambiguous rename: {manifest_candidates} missing manifest documents and \
                     {observations} new paths share this identity or content hash"
                ));
                diagnostics.push(diagnostic(
                    WorkspaceScanDiagnosticSeverity::Error,
                    "ambiguous_rename",
                    Some(observation.payload.relative_path.clone()),
                    "stable document identity could not be selected without guessing".into(),
                ));
                inserts.push(document_from_observation(workspace.id, &observation)?);
            }
            RenameCandidateMatch::None => {
                inserts.push(document_from_observation(workspace.id, &observation)?);
            }
        }
    }

    for existing in &existing_documents {
        if matched_ids.contains(&existing.id) {
            continue;
        }
        if existing.lifecycle_state == WorkspaceDocumentLifecycle::Missing {
            unchanged_documents += 1;
            continue;
        }

        let mut replacement = existing.clone();
        replacement.lifecycle_state = WorkspaceDocumentLifecycle::Missing;
        replacement.projection_state = stale_projection_state(existing);
        replacement.revision = existing
            .revision
            .checked_add(1)
            .ok_or_else(|| MvError::InvalidInput("workspace document revision overflow".into()))?;
        replacement.updated_at = completed_at;
        if let Some(payload) = parsed_existing.get(&existing.id) {
            let mut payload = payload.clone();
            payload.observed_at = completed_at;
            payload.reconciliation_note =
                Some("canonical file was not observed during reconciliation".into());
            replacement.document_payload = serde_json::to_vec(&payload)?;
        }
        updates.push(WorkspaceDocumentManifestUpdate {
            expected_revision: existing.revision,
            replacement,
        });
    }

    let degraded = diagnostics.iter().any(|diagnostic| {
        matches!(
            diagnostic.severity,
            WorkspaceScanDiagnosticSeverity::Warning | WorkspaceScanDiagnosticSeverity::Error
        )
    }) || inserts
        .iter()
        .any(|document| document.lifecycle_state == WorkspaceDocumentLifecycle::Conflict);
    let mut workspace_replacement = workspace.clone();
    workspace_replacement.state = if degraded {
        KnowledgeWorkspaceState::Degraded
    } else {
        KnowledgeWorkspaceState::Ready
    };
    workspace_replacement.revision = workspace
        .revision
        .checked_add(1)
        .ok_or_else(|| MvError::InvalidInput("workspace revision overflow".into()))?;
    workspace_replacement.updated_at = completed_at;
    workspace_replacement.last_reconciled_at = Some(completed_at);

    Ok(WorkspaceReconciliationPlan {
        manifest: WorkspaceManifestReconciliation {
            expected_workspace_revision: workspace.revision,
            workspace_replacement,
            document_inserts: inserts,
            document_updates: updates,
        },
        diagnostics,
        unchanged_documents,
        renamed_documents,
    })
}

fn replacement_if_changed(
    existing: &KnowledgeWorkspaceDocument,
    observation: &ScannedWorkspaceDocument,
    renamed: bool,
) -> MvResult<Option<WorkspaceDocumentManifestUpdate>> {
    let old_payload =
        serde_json::from_slice::<WorkspaceDocumentPayloadV1>(&existing.document_payload).ok();
    let payload_changed = old_payload
        .as_ref()
        .map(|payload| !payload_semantically_equal(payload, &observation.payload))
        .unwrap_or(true);
    let lifecycle_changed = existing.lifecycle_state != observation.lifecycle;
    if !renamed && !payload_changed && !lifecycle_changed {
        return Ok(None);
    }

    let content_changed = old_payload
        .as_ref()
        .map(|payload| payload.content_hash != observation.payload.content_hash)
        .unwrap_or(true);
    let mut replacement = existing.clone();
    replacement.path_token = observation.path_token.clone();
    replacement.document_payload = serde_json::to_vec(&observation.payload)?;
    replacement.lifecycle_state = observation.lifecycle;
    if content_changed || lifecycle_changed || renamed {
        replacement.projection_state = stale_projection_state(existing);
    }
    replacement.revision = existing
        .revision
        .checked_add(1)
        .ok_or_else(|| MvError::InvalidInput("workspace document revision overflow".into()))?;
    replacement.updated_at = observation.payload.observed_at;

    Ok(Some(WorkspaceDocumentManifestUpdate {
        expected_revision: existing.revision,
        replacement,
    }))
}

fn payload_semantically_equal(
    left: &WorkspaceDocumentPayloadV1,
    right: &WorkspaceDocumentPayloadV1,
) -> bool {
    left.schema == right.schema
        && left.relative_path == right.relative_path
        && left.content_hash == right.content_hash
        && left.byte_size == right.byte_size
        && left.modified_at == right.modified_at
        && left.content_status == right.content_status
        && left.file_identity_hint == right.file_identity_hint
        && left.portability_issues == right.portability_issues
        && left.reconciliation_note == right.reconciliation_note
}

enum RenameCandidateMatch<'a> {
    Unique(&'a KnowledgeWorkspaceDocument),
    Ambiguous {
        manifest_candidates: usize,
        observations: usize,
    },
    None,
}

fn rename_candidate<'a>(
    observation: &ScannedWorkspaceDocument,
    existing_documents: &'a [KnowledgeWorkspaceDocument],
    parsed_existing: &HashMap<uuid::Uuid, WorkspaceDocumentPayloadV1>,
    observed_tokens: &HashSet<String>,
    matched_ids: &HashSet<uuid::Uuid>,
    observed_identity_counts: &HashMap<String, usize>,
    observed_hash_counts: &HashMap<String, usize>,
) -> RenameCandidateMatch<'a> {
    let available = existing_documents.iter().filter(|document| {
        !matched_ids.contains(&document.id)
            && !observed_tokens.contains(document.path_token.as_str())
            && parsed_existing.contains_key(&document.id)
    });
    if let Some(identity) = observation.payload.file_identity_hint.as_ref() {
        let candidates = available
            .clone()
            .filter(|document| {
                parsed_existing
                    .get(&document.id)
                    .and_then(|payload| payload.file_identity_hint.as_ref())
                    == Some(identity)
            })
            .collect::<Vec<_>>();
        if !candidates.is_empty() {
            let observations = observed_identity_counts
                .get(&file_identity_key(identity))
                .copied()
                .unwrap_or(1);
            return match (candidates.as_slice(), observations) {
                ([candidate], 1) => RenameCandidateMatch::Unique(candidate),
                _ => RenameCandidateMatch::Ambiguous {
                    manifest_candidates: candidates.len(),
                    observations,
                },
            };
        }
    }
    let Some(content_hash) = observation.payload.content_hash.as_ref() else {
        return RenameCandidateMatch::None;
    };
    let candidates = available
        .filter(|document| {
            parsed_existing
                .get(&document.id)
                .and_then(|payload| payload.content_hash.as_ref())
                == Some(content_hash)
        })
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        return RenameCandidateMatch::None;
    }
    let observations = observed_hash_counts.get(content_hash).copied().unwrap_or(1);
    match (candidates.as_slice(), observations) {
        ([candidate], 1) => RenameCandidateMatch::Unique(candidate),
        _ => RenameCandidateMatch::Ambiguous {
            manifest_candidates: candidates.len(),
            observations,
        },
    }
}

fn file_identity_key(identity: &WorkspaceFileIdentityHint) -> String {
    format!("{}\0{}", identity.scheme, identity.value)
}

fn document_from_observation(
    workspace_id: uuid::Uuid,
    observation: &ScannedWorkspaceDocument,
) -> MvResult<KnowledgeWorkspaceDocument> {
    let mut document = KnowledgeWorkspaceDocument::new(
        workspace_id,
        observation.path_token.clone(),
        serde_json::to_vec(&observation.payload)?,
        WorkspaceManifestPayloadFormat::JsonV1,
    );
    document.lifecycle_state = observation.lifecycle;
    Ok(document)
}

fn stale_projection_state(existing: &KnowledgeWorkspaceDocument) -> WorkspaceProjectionState {
    if existing.projected_node_id.is_some() {
        WorkspaceProjectionState::Stale
    } else {
        WorkspaceProjectionState::Pending
    }
}

fn detect_path_collisions(
    documents: &mut [ScannedWorkspaceDocument],
    diagnostics: &mut Vec<WorkspaceScanDiagnostic>,
) {
    let mut normalized = HashMap::<String, Vec<usize>>::new();
    let mut case_folded = HashMap::<String, Vec<usize>>::new();
    for (index, document) in documents.iter().enumerate() {
        normalized
            .entry(document.path_assessment.collision_keys.normalized.clone())
            .or_default()
            .push(index);
        case_folded
            .entry(document.path_assessment.collision_keys.case_folded.clone())
            .or_default()
            .push(index);
    }

    let mut collided = HashSet::new();
    for indexes in normalized.values().filter(|indexes| indexes.len() > 1) {
        for index in indexes {
            collided.insert(*index);
        }
    }
    for indexes in case_folded.values().filter(|indexes| indexes.len() > 1) {
        let distinct_normalized = indexes
            .iter()
            .map(|index| {
                documents[*index]
                    .path_assessment
                    .collision_keys
                    .normalized
                    .as_str()
            })
            .collect::<HashSet<_>>();
        if distinct_normalized.len() > 1 {
            for index in indexes {
                collided.insert(*index);
            }
        }
    }

    for index in collided {
        let document = &mut documents[index];
        document.lifecycle = WorkspaceDocumentLifecycle::Conflict;
        document.payload.reconciliation_note =
            Some("path collides under portable normalization or case folding".into());
        diagnostics.push(diagnostic(
            WorkspaceScanDiagnosticSeverity::Error,
            "portable_path_collision",
            Some(document.payload.relative_path.clone()),
            "document path collides on another supported filesystem".into(),
        ));
    }
}

fn plaintext_path_token(normalized_relative_path: &str) -> String {
    let digest = Sha256::digest(normalized_relative_path.as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn sha256_content_hash(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let hex = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("sha256:{hex}")
}

fn should_skip_directory(relative_path: &str) -> bool {
    let component = relative_path.rsplit('/').next().unwrap_or(relative_path);
    component == ".git" || is_mindvault_reserved_component(component)
}

fn is_mindvault_reserved_path(relative_path: &str) -> bool {
    relative_path
        .split('/')
        .any(is_mindvault_reserved_component)
}

fn is_mindvault_reserved_component(component: &str) -> bool {
    let lowercase = component.to_ascii_lowercase();
    lowercase == ".mindvault" || lowercase.starts_with(".mindvault-")
}

fn slash_separated(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn relative_display(root: &Path, path: &Path) -> Option<String> {
    path.strip_prefix(root).ok().map(slash_separated)
}

fn diagnostic(
    severity: WorkspaceScanDiagnosticSeverity,
    code: impl Into<String>,
    relative_path: Option<String>,
    message: String,
) -> WorkspaceScanDiagnostic {
    WorkspaceScanDiagnostic {
        severity,
        code: code.into(),
        relative_path,
        message,
    }
}

fn read_regular_file_without_following_links(
    canonical_root: &Path,
    path: &Path,
    maximum_bytes: u64,
) -> MvResult<Vec<u8>> {
    let canonical_path = fs::canonicalize(path)
        .map_err(|error| MvError::Storage(format!("resolve document path failed: {error}")))?;
    if !canonical_path.starts_with(canonical_root) {
        return Err(MvError::AccessDenied(
            "document resolved outside the mounted workspace root".into(),
        ));
    }

    let file = open_without_following_links(path)?;
    read_bounded(file.take(maximum_bytes.saturating_add(1)), maximum_bytes)
}

fn read_bounded(mut reader: Take<File>, maximum_bytes: u64) -> MvResult<Vec<u8>> {
    let mut bytes = Vec::new();
    reader
        .read_to_end(&mut bytes)
        .map_err(|error| MvError::Storage(format!("read document bytes failed: {error}")))?;
    if bytes.len() as u64 > maximum_bytes {
        return Err(MvError::InvalidInput(
            "document grew beyond the configured indexing limit during scanning".into(),
        ));
    }
    Ok(bytes)
}

#[cfg(unix)]
fn open_without_following_links(path: &Path) -> MvResult<File> {
    use std::os::unix::fs::OpenOptionsExt;

    OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
        .map_err(|error| {
            MvError::Storage(format!("open document without symlinks failed: {error}"))
        })
}

#[cfg(windows)]
fn open_without_following_links(path: &Path) -> MvResult<File> {
    use std::os::windows::fs::OpenOptionsExt;

    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    OpenOptions::new()
        .read(true)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)
        .map_err(|error| {
            MvError::Storage(format!(
                "open document without reparse points failed: {error}"
            ))
        })
}

#[cfg(not(any(unix, windows)))]
fn open_without_following_links(path: &Path) -> MvResult<File> {
    OpenOptions::new()
        .read(true)
        .open(path)
        .map_err(|error| MvError::Storage(format!("open document failed: {error}")))
}

#[cfg(unix)]
fn file_identity_hint(metadata: &fs::Metadata) -> Option<WorkspaceFileIdentityHint> {
    use std::os::unix::fs::MetadataExt;

    Some(WorkspaceFileIdentityHint {
        scheme: "unix-dev-inode".into(),
        value: format!("{}:{}", metadata.dev(), metadata.ino()),
    })
}

#[cfg(not(unix))]
fn file_identity_hint(_metadata: &fs::Metadata) -> Option<WorkspaceFileIdentityHint> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use mv_core::{KnowledgeWorkspaceMode, WorkspaceManifestPayloadFormat};
    use mv_storage::sqlite::SqliteNodeStore;
    use tempfile::tempdir;

    fn mounted_workspace() -> KnowledgeWorkspace {
        let descriptor = WorkspaceDescriptorPayloadV1::new("Test Vault", "/tmp/test-vault");
        KnowledgeWorkspace::new(
            "personal",
            KnowledgeWorkspaceMode::Mounted,
            serde_json::to_vec(&descriptor).unwrap(),
            WorkspaceManifestPayloadFormat::JsonV1,
        )
    }

    fn observed_document(relative_path: &str, content_hash: &str) -> ScannedWorkspaceDocument {
        let assessment = PortableWorkspacePathPolicy::new().assess_existing(relative_path);
        let mut payload = WorkspaceDocumentPayloadV1::new(
            assessment.normalized_relative_path.clone(),
            Utc::now(),
        );
        payload.content_hash = Some(content_hash.to_string());
        payload.content_status = WorkspaceDocumentContentStatus::Utf8;
        ScannedWorkspaceDocument {
            path_token: plaintext_path_token(&assessment.normalized_relative_path),
            path_assessment: assessment,
            payload,
            content: Some("same content".into()),
            lifecycle: WorkspaceDocumentLifecycle::Active,
        }
    }

    fn existing_document(
        workspace: &KnowledgeWorkspace,
        relative_path: &str,
        content_hash: &str,
    ) -> KnowledgeWorkspaceDocument {
        let observation = observed_document(relative_path, content_hash);
        document_from_observation(workspace.id, &observation).unwrap()
    }

    #[test]
    fn scanner_indexes_supported_files_without_mutating_the_root() {
        let directory = tempdir().unwrap();
        let root = directory.path();
        fs::create_dir(root.join("Notes")).unwrap();
        fs::create_dir_all(root.join("Empty/Nested")).unwrap();
        fs::create_dir(root.join(".mindvault")).unwrap();
        fs::write(root.join("Notes/Alpha.md"), b"# Alpha\r\n").unwrap();
        fs::write(root.join(".hidden.markdown"), b"hidden").unwrap();
        fs::write(root.join("binary.md"), [0xff, 0xfe]).unwrap();
        fs::write(root.join(".mindvault/internal.md"), b"internal").unwrap();
        let alpha_before = fs::read(root.join("Notes/Alpha.md")).unwrap();

        let scan = WorkspaceScanner::default()
            .scan(&mounted_workspace(), root)
            .unwrap();

        assert_eq!(scan.documents.len(), 3);
        assert_eq!(fs::read(root.join("Notes/Alpha.md")).unwrap(), alpha_before);
        assert!(root.join(".mindvault/internal.md").exists());
        assert!(!root.join(".mindvault-index").exists());
        assert!(scan.documents.iter().any(|document| {
            document.payload.relative_path == ".hidden.markdown"
                && document.lifecycle == WorkspaceDocumentLifecycle::Active
        }));
        assert!(scan.documents.iter().any(|document| {
            document.payload.relative_path == "binary.md"
                && document.lifecycle == WorkspaceDocumentLifecycle::Unsupported
                && document.payload.content_hash.is_some()
        }));
        assert!(scan
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "reserved_internal_path"));
        assert!(scan.directories.iter().any(|directory| {
            directory.path_assessment.normalized_relative_path == "Empty/Nested"
        }));
        assert!(!scan.directories.iter().any(|directory| {
            directory
                .path_assessment
                .normalized_relative_path
                .starts_with(".mindvault")
        }));
    }

    #[test]
    fn tree_contract_preserves_empty_directories_and_manifest_state() {
        let workspace = mounted_workspace();
        let current = existing_document(&workspace, "Notes/Current.md", "sha256:current");
        let directory_assessment =
            PortableWorkspacePathPolicy::new().assess_existing("Empty/Nested");
        let scan = WorkspaceScan {
            directories: vec![ScannedWorkspaceDirectory {
                path_assessment: directory_assessment,
            }],
            documents: vec![
                observed_document("Notes/Current.md", "sha256:current"),
                observed_document("Inbox.md", "sha256:new"),
            ],
            diagnostics: Vec::new(),
            completed_at: Utc::now(),
        };

        let tree = build_workspace_tree(&workspace, std::slice::from_ref(&current), scan.clone());

        assert_eq!(tree.workspace_id, workspace.id);
        assert!(tree.entries.iter().any(|entry| {
            entry.kind == WorkspaceTreeEntryKind::Directory
                && entry.relative_path == "Empty/Nested"
                && entry.parent_path.as_deref() == Some("Empty")
                && entry.document_id.is_none()
        }));
        assert!(tree.entries.iter().any(|entry| {
            entry.relative_path == "Notes/Current.md"
                && entry.document_id == Some(current.id)
                && entry.manifest_match == Some(WorkspaceManifestMatch::Current)
        }));
        assert!(tree.entries.iter().any(|entry| {
            entry.relative_path == "Inbox.md"
                && entry.document_id.is_none()
                && entry.manifest_match == Some(WorkspaceManifestMatch::Untracked)
        }));

        let mut changed_scan = scan;
        changed_scan.documents[0] = observed_document("Notes/Current.md", "sha256:changed-on-disk");
        let read = read_workspace_document_from_scan(&workspace, &current, changed_scan).unwrap();
        assert_eq!(read.relative_path, "Notes/Current.md");
        assert_eq!(read.content, "same content");
        assert_eq!(read.manifest_match, WorkspaceManifestMatch::Changed);
    }

    #[test]
    fn scanner_keeps_nonportable_existing_names_readable() {
        let directory = tempdir().unwrap();
        fs::write(directory.path().join("CON.notes.md"), b"legacy").unwrap();

        let scan = WorkspaceScanner::default()
            .scan(&mounted_workspace(), directory.path())
            .unwrap();

        assert_eq!(scan.documents.len(), 1);
        assert_eq!(
            scan.documents[0].lifecycle,
            WorkspaceDocumentLifecycle::Active
        );
        assert!(!scan.documents[0].path_assessment.is_portable());
        assert!(scan
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "nonportable_path"));
    }

    #[test]
    fn scanner_fails_closed_without_a_decrypted_manifest_view() {
        let directory = tempdir().unwrap();
        let mut workspace = mounted_workspace();
        workspace.payload_format = WorkspaceManifestPayloadFormat::MvencV1;

        assert!(matches!(
            WorkspaceScanner::default().scan(&workspace, directory.path()),
            Err(MvError::InvalidInput(message))
                if message.contains("decrypted json-v1")
        ));
    }

    #[tokio::test]
    async fn reconciliation_preserves_identity_across_an_unambiguous_rename() {
        let directory = tempdir().unwrap();
        fs::write(directory.path().join("Old.md"), b"stable content").unwrap();
        let store = SqliteNodeStore::open_in_memory().unwrap();
        let workspace = mounted_workspace();
        store.insert_knowledge_workspace(&workspace).await.unwrap();
        let reconciler = WorkspaceReconciler::new(&store, WorkspaceScanner::default());

        let first = reconciler
            .reconcile(&workspace, directory.path())
            .await
            .unwrap();
        assert!(first.applied);
        assert_eq!(first.inserted_documents, 1);
        let first_document = store
            .list_workspace_documents(workspace.id)
            .await
            .unwrap()
            .remove(0);

        fs::rename(
            directory.path().join("Old.md"),
            directory.path().join("New.md"),
        )
        .unwrap();
        let current_workspace = store
            .get_knowledge_workspace(workspace.id)
            .await
            .unwrap()
            .unwrap();
        let renamed = reconciler
            .reconcile(&current_workspace, directory.path())
            .await
            .unwrap();

        assert!(renamed.applied);
        assert_eq!(renamed.renamed_documents, 1);
        assert_eq!(renamed.inserted_documents, 0);
        let documents = store.list_workspace_documents(workspace.id).await.unwrap();
        assert_eq!(documents.len(), 1);
        assert_eq!(documents[0].id, first_document.id);
        let payload =
            serde_json::from_slice::<WorkspaceDocumentPayloadV1>(&documents[0].document_payload)
                .unwrap();
        assert_eq!(payload.relative_path, "New.md");
    }

    #[test]
    fn reconciliation_does_not_guess_when_two_new_paths_share_a_rename_hash() {
        let workspace = mounted_workspace();
        let existing = existing_document(&workspace, "Old.md", "sha256:same");
        let scan = WorkspaceScan {
            directories: Vec::new(),
            documents: vec![
                observed_document("Copy A.md", "sha256:same"),
                observed_document("Copy B.md", "sha256:same"),
            ],
            diagnostics: Vec::new(),
            completed_at: Utc::now(),
        };

        let plan = build_reconciliation_plan(&workspace, vec![existing.clone()], scan).unwrap();

        assert_eq!(plan.renamed_documents, 0);
        assert_eq!(plan.manifest.document_inserts.len(), 2);
        assert!(plan
            .manifest
            .document_inserts
            .iter()
            .all(|document| document.lifecycle_state == WorkspaceDocumentLifecycle::Conflict));
        assert!(plan.manifest.document_updates.iter().any(|update| {
            update.replacement.id == existing.id
                && update.replacement.lifecycle_state == WorkspaceDocumentLifecycle::Missing
        }));
        assert_eq!(
            plan.diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.code == "ambiguous_rename")
                .count(),
            2
        );
    }

    #[test]
    fn reconciliation_blocks_duplicate_normalized_path_tokens_before_storage() {
        let workspace = mounted_workspace();
        let scan = WorkspaceScan {
            directories: Vec::new(),
            documents: vec![
                observed_document("Cafe\u{301}.md", "sha256:first"),
                observed_document("Caf\u{e9}.md", "sha256:second"),
            ],
            diagnostics: Vec::new(),
            completed_at: Utc::now(),
        };

        assert!(matches!(
            build_reconciliation_plan(&workspace, Vec::new(), scan),
            Err(MvError::InvalidInput(message))
                if message.contains("normalized path collision")
        ));
    }
}
