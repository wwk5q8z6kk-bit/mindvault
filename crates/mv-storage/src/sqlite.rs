use std::path::Path;
use std::str::FromStr;
use std::sync::Mutex;

use async_trait::async_trait;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use chrono::Utc;
use hmac::{Hmac, Mac};
use rusqlite::types::Type;
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;
use zeroize::Zeroizing;

use crate::sealed_runtime::{runtime_root_key_for_scope, runtime_scope_from_parent};
use crate::vault_crypto::VaultCrypto;
use mv_core::*;

/// Default number of connections in the pool.
/// SQLite WAL mode supports 1 writer + N readers, so even a small pool
/// eliminates head-of-line blocking for concurrent read queries.
const DEFAULT_POOL_SIZE: usize = 4;

type StoredNodeProjection = (
    Option<String>,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SealedNodePayload {
    title: Option<String>,
    content: String,
    source: Option<String>,
    metadata: std::collections::HashMap<String, serde_json::Value>,
}

const INTEROPERABILITY_REGISTRY_NAMESPACE: &str = "interoperability-registry";

pub struct SqliteNodeStore {
    /// Connection pool — round-robin across `DEFAULT_POOL_SIZE` connections.
    /// Each connection is independently protected by a Mutex so callers can
    /// run synchronous rusqlite operations without holding an async lock.
    pool: Vec<Mutex<Connection>>,
    /// Atomic counter for round-robin slot selection.
    next_slot: std::sync::atomic::AtomicUsize,
    sealed_mode: bool,
    runtime_scope: String,
}

impl SqliteNodeStore {
    /// Execute a synchronous closure with a pooled database connection.
    ///
    /// Picks the next connection via round-robin, locks it, runs the
    /// closure, then releases. Because the closure is `FnOnce` (not async),
    /// the `MutexGuard` is guaranteed to drop before any `.await` — making
    /// the enclosing future `Send`.
    fn with_conn<F, T>(&self, f: F) -> MvResult<T>
    where
        F: FnOnce(&Connection) -> MvResult<T>,
    {
        let idx = self
            .next_slot
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            % self.pool.len();
        let conn = self.pool[idx]
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        f(&conn)
    }

    /// Access a pooled connection via round-robin, matching `with_conn`
    /// distribution. Callers must `.lock()` the returned mutex.
    #[inline]
    fn conn(&self) -> &Mutex<Connection> {
        let idx = self
            .next_slot
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            % self.pool.len();
        &self.pool[idx]
    }

    #[inline]
    fn sealed_mode(&self) -> bool {
        self.sealed_mode
    }

    #[inline]
    fn runtime_scope(&self) -> &str {
        &self.runtime_scope
    }
}

impl SqliteNodeStore {
    fn open_connection(path: &Path) -> MvResult<Connection> {
        let conn = Connection::open(path)
            .map_err(|e| MvError::Storage(format!("failed to open sqlite: {e}")))?;

        conn.execute_batch(
            "PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000;",
        )
        .map_err(|e| MvError::Storage(format!("pragma error: {e}")))?;

        Ok(conn)
    }

    pub fn open(path: &Path) -> MvResult<Self> {
        Self::open_with_mode(path, false)
    }

    pub fn open_with_mode(path: &Path, sealed_mode: bool) -> MvResult<Self> {
        let mut pool = Vec::with_capacity(DEFAULT_POOL_SIZE);
        for _ in 0..DEFAULT_POOL_SIZE {
            pool.push(Mutex::new(Self::open_connection(path)?));
        }

        let store = Self {
            pool,
            next_slot: std::sync::atomic::AtomicUsize::new(0),
            sealed_mode,
            runtime_scope: runtime_scope_from_parent(path),
        };
        store.run_migrations()?;
        Ok(store)
    }

    pub fn open_in_memory() -> MvResult<Self> {
        Self::open_in_memory_with_mode(false)
    }

    pub fn open_in_memory_with_mode(sealed_mode: bool) -> MvResult<Self> {
        // In-memory DBs: use a shared cache URI so all pool connections see
        // the same data. Without this, each Connection::open_in_memory()
        // gets its own isolated database.
        //
        // SQLITE_OPEN_URI is required for rusqlite to parse the URI; the
        // default OpenFlags do NOT include it.
        let uri = format!(
            "file:memdb{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4()
        );
        let flags = rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE
            | rusqlite::OpenFlags::SQLITE_OPEN_CREATE
            | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX
            | rusqlite::OpenFlags::SQLITE_OPEN_URI;
        let mut pool = Vec::with_capacity(DEFAULT_POOL_SIZE);
        for _ in 0..DEFAULT_POOL_SIZE {
            let conn = Connection::open_with_flags(&uri, flags)
                .map_err(|e| MvError::Storage(format!("failed to open in-memory sqlite: {e}")))?;
            conn.execute_batch("PRAGMA foreign_keys=ON;")
                .map_err(|e| MvError::Storage(format!("pragma error: {e}")))?;
            pool.push(Mutex::new(conn));
        }

        let store = Self {
            pool,
            next_slot: std::sync::atomic::AtomicUsize::new(0),
            sealed_mode,
            runtime_scope: String::new(),
        };
        store.run_migrations()?;
        Ok(store)
    }

    pub fn open_read_only(path: &Path) -> MvResult<Self> {
        Self::open_read_only_with_mode(path, false)
    }

    pub fn open_read_only_with_mode(path: &Path, sealed_mode: bool) -> MvResult<Self> {
        let conn = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|e| MvError::Storage(format!("failed to open sqlite (read-only): {e}")))?;

        conn.execute_batch("PRAGMA foreign_keys=ON; PRAGMA query_only=ON;")
            .map_err(|e| MvError::Storage(format!("pragma error: {e}")))?;

        // Read-only mode: single connection is fine (no write contention)
        Ok(Self {
            pool: vec![Mutex::new(conn)],
            next_slot: std::sync::atomic::AtomicUsize::new(0),
            sealed_mode,
            runtime_scope: runtime_scope_from_parent(path),
        })
    }

    fn run_migrations(&self) -> MvResult<()> {
        // Migrations run on slot 0 only — they need exclusive access.
        let conn = self.pool[0]
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        // Table-driven migration registry.
        // Versions 002, 009, 017-021 are keychain-only and applied in
        // SqliteKeychainStore — they are intentionally excluded here.
        const MIGRATIONS: &[(i64, &str)] = &[
            (1, include_str!("../../../migrations/001_initial.sql")),
            (3, include_str!("../../../migrations/003_agentic.sql")),
            (4, include_str!("../../../migrations/004_exchange.sql")),
            (
                5,
                include_str!("../../../migrations/005_relay_safeguards.sql"),
            ),
            (6, include_str!("../../../migrations/006_feedback.sql")),
            (7, include_str!("../../../migrations/007_autonomy.sql")),
            (8, include_str!("../../../migrations/008_relay.sql")),
            (10, include_str!("../../../migrations/010_profile.sql")),
            (
                11,
                include_str!("../../../migrations/011_consumer_profiles.sql"),
            ),
            (
                12,
                include_str!("../../../migrations/012_access_policies.sql"),
            ),
            (13, include_str!("../../../migrations/013_proxy_audit.sql")),
            (14, include_str!("../../../migrations/014_conflicts.sql")),
            (
                15,
                include_str!("../../../migrations/015_contact_identity.sql"),
            ),
            (
                16,
                include_str!("../../../migrations/016_approval_queue.sql"),
            ),
            (
                22,
                include_str!("../../../migrations/022_adapter_poll_state.sql"),
            ),
            (
                23,
                include_str!("../../../migrations/023_conversations.sql"),
            ),
            (24, include_str!("../../../migrations/024_plans.sql")),
            (
                25,
                include_str!("../../../migrations/025_public_shares.sql"),
            ),
            (
                26,
                include_str!("../../../migrations/026_node_comments.sql"),
            ),
            (
                27,
                include_str!("../../../migrations/027_mcp_connectors.sql"),
            ),
            (
                28,
                include_str!("../../../migrations/028_sealed_node_payloads.sql"),
            ),
            (
                30,
                include_str!("../../../migrations/030_conversation_turn_sources.sql"),
            ),
            (
                31,
                include_str!("../../../migrations/031_knowledge_workspace_manifest.sql"),
            ),
            (
                32,
                include_str!("../../../migrations/032_interoperability_kernel.sql"),
            ),
            (
                33,
                include_str!("../../../migrations/033_governed_interoperability_registries.sql"),
            ),
            (
                34,
                include_str!("../../../migrations/034_context_node_registry.sql"),
            ),
            (
                35,
                include_str!("../../../migrations/035_authority_grants.sql"),
            ),
            (
                36,
                include_str!("../../../migrations/036_outbox_dispatch_and_action_receipts.sql"),
            ),
            (
                37,
                include_str!("../../../migrations/037_consumer_inbox_checkpoints.sql"),
            ),
            (
                38,
                include_str!("../../../migrations/038_work_orders_and_agent_runs.sql"),
            ),
            (
                39,
                include_str!("../../../migrations/039_command_admission_decisions.sql"),
            ),
            (
                40,
                include_str!("../../../migrations/040_identity_registry.sql"),
            ),
        ];

        // Migration 001 must always run first to create schema_version table.
        // After that, check which versions are already applied.
        conn.execute_batch(MIGRATIONS[0].1)
            .map_err(|e| MvError::Migration(format!("migration 001 failed: {e}")))?;

        let max_version: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_version",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        for &(version, sql) in &MIGRATIONS[1..] {
            if version <= max_version {
                continue;
            }
            conn.execute_batch(sql)
                .map_err(|e| MvError::Migration(format!("migration {version:03} failed: {e}")))?;
        }

        tracing::debug!(
            applied_up_to = MIGRATIONS.last().map(|(v, _)| *v).unwrap_or(0),
            "Migrations complete"
        );

        Ok(())
    }

    fn as_sql_conversion_error(column: usize, message: impl Into<String>) -> rusqlite::Error {
        rusqlite::Error::FromSqlConversionFailure(
            column,
            Type::Text,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                message.into(),
            )),
        )
    }

    fn derive_namespace_kek(&self, namespace: &str) -> MvResult<[u8; 32]> {
        let root = runtime_root_key_for_scope(self.runtime_scope()).ok_or(MvError::VaultSealed)?;
        let mut crypto = VaultCrypto::new();
        crypto.set_master_key(Zeroizing::new(root));
        let key = crypto
            .derive_namespace_kek(namespace)
            .map_err(|err| MvError::Storage(format!("derive namespace key failed: {err}")))?;
        Ok(*key)
    }

    fn encrypt_node_payload(&self, node: &KnowledgeNode) -> MvResult<(String, String)> {
        let kek = self.derive_namespace_kek(&node.namespace)?;
        let dek = VaultCrypto::generate_node_dek();
        let payload = SealedNodePayload {
            title: node.title.clone(),
            content: node.content.clone(),
            source: node.source.clone(),
            metadata: node.metadata.clone(),
        };
        let plaintext = serde_json::to_vec(&payload)
            .map_err(|err| MvError::Storage(format!("serialize sealed payload: {err}")))?;
        let ciphertext = VaultCrypto::aes_gcm_encrypt_pub(&dek, &plaintext)
            .map_err(|err| MvError::Storage(format!("encrypt sealed payload: {err}")))?;
        let wrapped_dek = VaultCrypto::wrap_node_dek(&kek, &dek)
            .map_err(|err| MvError::Storage(format!("wrap node dek failed: {err}")))?;
        Ok((wrapped_dek, BASE64.encode(ciphertext)))
    }

    fn decrypt_node_payload(
        &self,
        namespace: &str,
        wrapped_dek: &str,
        payload_ciphertext: &str,
    ) -> MvResult<SealedNodePayload> {
        let kek = self.derive_namespace_kek(namespace)?;
        let dek = VaultCrypto::unwrap_node_dek(&kek, wrapped_dek)
            .map_err(|err| MvError::Storage(format!("unwrap node dek failed: {err}")))?;
        let ciphertext = BASE64
            .decode(payload_ciphertext)
            .map_err(|err| MvError::Storage(format!("decode sealed payload failed: {err}")))?;
        let plaintext = VaultCrypto::aes_gcm_decrypt_pub(&dek, &ciphertext)
            .map_err(|err| MvError::Storage(format!("decrypt sealed payload failed: {err}")))?;
        let payload: SealedNodePayload = serde_json::from_slice(&plaintext)
            .map_err(|err| MvError::Storage(format!("parse sealed payload failed: {err}")))?;
        Ok(payload)
    }

    fn project_node_for_storage(&self, node: &KnowledgeNode) -> MvResult<StoredNodeProjection> {
        if self.sealed_mode() {
            let (wrapped_dek, ciphertext) = self.encrypt_node_payload(node)?;
            Ok((
                None,
                String::new(),
                None,
                None,
                Some(ciphertext),
                Some(wrapped_dek),
            ))
        } else {
            Ok((
                node.title.clone(),
                node.content.clone(),
                node.source.clone(),
                Some(serde_json::to_string(&node.metadata)?),
                None,
                None,
            ))
        }
    }

    fn encode_governance_record<T: Serialize>(
        &self,
        record: &T,
        label: &str,
    ) -> MvResult<(Vec<u8>, &'static str, Option<String>)> {
        let plaintext = serde_json::to_vec(record)
            .map_err(|err| MvError::Storage(format!("serialize {label}: {err}")))?;
        if !self.sealed_mode() {
            return Ok((plaintext, "json-v1", None));
        }

        let kek = self.derive_namespace_kek(INTEROPERABILITY_REGISTRY_NAMESPACE)?;
        let dek = VaultCrypto::generate_node_dek();
        let ciphertext = VaultCrypto::aes_gcm_encrypt_pub(&dek, &plaintext)
            .map_err(|err| MvError::Storage(format!("encrypt {label}: {err}")))?;
        let wrapped_dek = VaultCrypto::wrap_node_dek(&kek, &dek)
            .map_err(|err| MvError::Storage(format!("wrap {label} key: {err}")))?;
        Ok((ciphertext, "mvenc-v1", Some(wrapped_dek)))
    }

    /// Seal opaque content bytes for storage.
    ///
    /// Distinct from [`Self::encode_governance_record`], which serializes a
    /// governed record through serde_json. Artifact content is not a JSON
    /// document, and routing it through serde_json would store a JSON array of
    /// integers — roughly four bytes per byte of content.
    ///
    /// Sealing is identical to the record path: the same namespace KEK, a fresh
    /// per-record DEK, and the same `mvenc-v1` marker. Only the plaintext
    /// marker differs (`bytes-v1`), so a reader can never mistake opaque
    /// content for a decodable record.
    fn encode_governance_bytes(
        &self,
        plaintext: &[u8],
        label: &str,
    ) -> MvResult<(Vec<u8>, &'static str, Option<String>)> {
        if !self.sealed_mode() {
            return Ok((plaintext.to_vec(), "bytes-v1", None));
        }

        let kek = self.derive_namespace_kek(INTEROPERABILITY_REGISTRY_NAMESPACE)?;
        let dek = VaultCrypto::generate_node_dek();
        let ciphertext = VaultCrypto::aes_gcm_encrypt_pub(&dek, plaintext)
            .map_err(|err| MvError::Storage(format!("encrypt {label}: {err}")))?;
        let wrapped_dek = VaultCrypto::wrap_node_dek(&kek, &dek)
            .map_err(|err| MvError::Storage(format!("wrap {label} key: {err}")))?;
        Ok((ciphertext, "mvenc-v1", Some(wrapped_dek)))
    }

    /// Recover opaque content bytes sealed by [`Self::encode_governance_bytes`].
    fn decode_governance_bytes(
        &self,
        payload: &[u8],
        payload_format: &str,
        wrapped_dek: Option<&str>,
        label: &str,
    ) -> MvResult<Vec<u8>> {
        match payload_format {
            "bytes-v1" if wrapped_dek.is_none() => Ok(payload.to_vec()),
            "mvenc-v1" => {
                let wrapped_dek = wrapped_dek.ok_or_else(|| {
                    MvError::Storage(format!("encrypted {label} is missing its wrapped key"))
                })?;
                let kek = self.derive_namespace_kek(INTEROPERABILITY_REGISTRY_NAMESPACE)?;
                let dek = VaultCrypto::unwrap_node_dek(&kek, wrapped_dek)
                    .map_err(|err| MvError::Storage(format!("unwrap {label} key: {err}")))?;
                VaultCrypto::aes_gcm_decrypt_pub(&dek, payload)
                    .map_err(|err| MvError::Storage(format!("decrypt {label}: {err}")))
            }
            "bytes-v1" => Err(MvError::Storage(format!(
                "plaintext {label} unexpectedly has a wrapped key"
            ))),
            other => Err(MvError::Storage(format!(
                "unsupported {label} payload format: {other}"
            ))),
        }
    }

    fn decode_governance_record<T: DeserializeOwned>(
        &self,
        payload: &[u8],
        payload_format: &str,
        wrapped_dek: Option<&str>,
        label: &str,
    ) -> MvResult<T> {
        let plaintext = match payload_format {
            "json-v1" if wrapped_dek.is_none() => payload.to_vec(),
            "mvenc-v1" => {
                let wrapped_dek = wrapped_dek.ok_or_else(|| {
                    MvError::Storage(format!("encrypted {label} is missing its wrapped key"))
                })?;
                let kek = self.derive_namespace_kek(INTEROPERABILITY_REGISTRY_NAMESPACE)?;
                let dek = VaultCrypto::unwrap_node_dek(&kek, wrapped_dek)
                    .map_err(|err| MvError::Storage(format!("unwrap {label} key: {err}")))?;
                VaultCrypto::aes_gcm_decrypt_pub(&dek, payload)
                    .map_err(|err| MvError::Storage(format!("decrypt {label}: {err}")))?
            }
            "json-v1" => {
                return Err(MvError::Storage(format!(
                    "plaintext {label} unexpectedly has a wrapped key"
                )))
            }
            other => {
                return Err(MvError::Storage(format!(
                    "unsupported {label} payload format: {other}"
                )))
            }
        };

        serde_json::from_slice(&plaintext)
            .map_err(|err| MvError::Storage(format!("decode {label}: {err}")))
    }

    fn row_to_node(&self, row: &rusqlite::Row<'_>) -> rusqlite::Result<KnowledgeNode> {
        let id_str: String = row.get(0)?;
        let kind_str: String = row.get(1)?;
        let mut title: Option<String> = row.get(2)?;
        let content: String = row.get(3)?;
        let mut source: Option<String> = row.get(4)?;
        let namespace: String = row.get(5)?;
        let importance: f64 = row.get(6)?;
        let created_at: String = row.get(7)?;
        let updated_at: String = row.get(8)?;
        let last_accessed_at: String = row.get(9)?;
        let access_count: u64 = row.get(10)?;
        let version: u32 = row.get(11)?;
        let expires_at: Option<String> = row.get(12)?;
        let mut metadata_json: Option<String> = row.get(13)?;
        let payload_ciphertext: Option<String> = row.get(14).ok();
        let wrapped_dek: Option<String> = row.get(15).ok();

        if let (Some(payload_ciphertext), Some(wrapped_dek)) = (payload_ciphertext, wrapped_dek) {
            let payload = self
                .decrypt_node_payload(&namespace, &wrapped_dek, &payload_ciphertext)
                .map_err(|err| {
                    Self::as_sql_conversion_error(
                        14,
                        format!("failed to decrypt node payload: {err}"),
                    )
                })?;
            title = payload.title;
            source = payload.source;
            metadata_json = Some(serde_json::to_string(&payload.metadata).map_err(|err| {
                Self::as_sql_conversion_error(
                    13,
                    format!("failed to reserialize node metadata: {err}"),
                )
            })?);

            return Ok(KnowledgeNode {
                id: parse_uuid_str(0, &id_str)?,
                kind: kind_str
                    .parse()
                    .map_err(|err: String| Self::as_sql_conversion_error(1, err))?,
                title,
                content: payload.content,
                source,
                namespace,
                tags: Vec::new(),
                importance,
                temporal: TemporalMeta {
                    created_at: parse_dt_strict(7, &created_at)?,
                    updated_at: parse_dt_strict(8, &updated_at)?,
                    last_accessed_at: parse_dt_strict(9, &last_accessed_at)?,
                    access_count,
                    version,
                    expires_at: parse_optional_dt_strict(12, expires_at)?,
                },
                metadata: parse_metadata_json(metadata_json)?,
            });
        }

        let id = parse_uuid_str(0, &id_str)?;
        let kind: NodeKind = kind_str.parse().map_err(|err: String| {
            rusqlite::Error::FromSqlConversionFailure(
                1,
                Type::Text,
                Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, err)),
            )
        })?;
        let metadata = parse_metadata_json(metadata_json)?;

        Ok(KnowledgeNode {
            id,
            kind,
            title,
            content,
            source,
            namespace,
            tags: Vec::new(), // loaded separately
            importance,
            temporal: TemporalMeta {
                created_at: parse_dt_strict(7, &created_at)?,
                updated_at: parse_dt_strict(8, &updated_at)?,
                last_accessed_at: parse_dt_strict(9, &last_accessed_at)?,
                access_count,
                version,
                expires_at: parse_optional_dt_strict(12, expires_at)?,
            },
            metadata,
        })
    }

    fn load_tags(conn: &Connection, node_id: Uuid) -> MvResult<Vec<String>> {
        let mut stmt = conn
            .prepare("SELECT tag FROM node_tags WHERE node_id = ?1 ORDER BY tag")
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut tags = Vec::new();
        let rows = stmt
            .query_map(params![node_id.to_string()], |row| row.get(0))
            .map_err(|e| MvError::Storage(e.to_string()))?;

        for row in rows {
            tags.push(row.map_err(|e| MvError::Storage(e.to_string()))?);
        }

        Ok(tags)
    }

    fn save_tags(conn: &Connection, node_id: Uuid, tags: &[String]) -> MvResult<()> {
        conn.execute(
            "DELETE FROM node_tags WHERE node_id = ?1",
            params![node_id.to_string()],
        )
        .map_err(|e| MvError::Storage(e.to_string()))?;
        let mut stmt = conn
            .prepare("INSERT INTO node_tags (node_id, tag) VALUES (?1, ?2)")
            .map_err(|e| MvError::Storage(e.to_string()))?;
        for tag in tags {
            stmt.execute(params![node_id.to_string(), tag])
                .map_err(|e| MvError::Storage(e.to_string()))?;
        }
        Ok(())
    }

    fn log_change(
        conn: &Connection,
        node_id: Uuid,
        op: ChangeOp,
        diff: Option<&serde_json::Value>,
    ) -> MvResult<()> {
        let diff_str = diff.map(|d| serde_json::to_string(d).unwrap_or_default());
        conn.execute(
            "INSERT INTO changelog (node_id, operation, diff_json, timestamp) VALUES (?1, ?2, ?3, ?4)",
            params![
                node_id.to_string(),
                op.as_str(),
                diff_str,
                Utc::now().to_rfc3339(),
            ],
        )
        .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(())
    }
}

fn parse_uuid_str(column: usize, s: &str) -> rusqlite::Result<Uuid> {
    Uuid::parse_str(s)
        .map_err(|err| rusqlite::Error::FromSqlConversionFailure(column, Type::Text, Box::new(err)))
}

fn parse_dt_strict(column: usize, s: &str) -> rusqlite::Result<chrono::DateTime<Utc>> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|err| rusqlite::Error::FromSqlConversionFailure(column, Type::Text, Box::new(err)))
}

fn parse_optional_dt_strict(
    column: usize,
    s: Option<String>,
) -> rusqlite::Result<Option<chrono::DateTime<Utc>>> {
    match s {
        Some(value) => parse_dt_strict(column, &value).map(Some),
        None => Ok(None),
    }
}

fn event_binding_id(event: &EventEnvelope) -> MvResult<Uuid> {
    let value = event
        .data
        .get("binding_id")
        .and_then(|value| value.as_str())
        .ok_or_else(|| MvError::InvalidInput("event data is missing binding_id".into()))?;
    Uuid::parse_str(value)
        .map_err(|err| MvError::InvalidInput(format!("event binding_id is invalid: {err}")))
}

fn event_context_node_id(event: &EventEnvelope) -> MvResult<Uuid> {
    let value = event
        .data
        .get("node_id")
        .and_then(|value| value.as_str())
        .ok_or_else(|| MvError::InvalidInput("event data is missing node_id".into()))?;
    Uuid::parse_str(value)
        .map_err(|err| MvError::InvalidInput(format!("event node_id is invalid: {err}")))
}

fn event_context_node_revision(event: &EventEnvelope, default: u64) -> MvResult<u64> {
    match event.data.get("revision") {
        Some(value) => value.as_u64().ok_or_else(|| {
            MvError::InvalidInput("event context-node revision must be an unsigned integer".into())
        }),
        None => Ok(default),
    }
}

fn event_authority_grant_id(event: &EventEnvelope) -> MvResult<Uuid> {
    let value = event
        .data
        .get("grant_id")
        .and_then(|value| value.as_str())
        .ok_or_else(|| MvError::InvalidInput("event data is missing grant_id".into()))?;
    Uuid::parse_str(value)
        .map_err(|err| MvError::InvalidInput(format!("event grant_id is invalid: {err}")))
}

fn event_identity_principal_id(event: &EventEnvelope) -> MvResult<Uuid> {
    let value = event
        .data
        .get("principal_id")
        .and_then(|value| value.as_str())
        .ok_or_else(|| MvError::InvalidInput("event data is missing principal_id".into()))?;
    Uuid::parse_str(value)
        .map_err(|err| MvError::InvalidInput(format!("event principal_id is invalid: {err}")))
}

fn event_identity_revision(event: &EventEnvelope, default: u64) -> MvResult<u64> {
    match event.data.get("revision") {
        Some(value) => value.as_u64().ok_or_else(|| {
            MvError::InvalidInput("event identity revision must be an unsigned integer".into())
        }),
        None => Ok(default),
    }
}

fn event_authority_grant_revision(event: &EventEnvelope, default: u64) -> MvResult<u64> {
    match event.data.get("revision") {
        Some(value) => value.as_u64().ok_or_else(|| {
            MvError::InvalidInput(
                "event authority-grant revision must be an unsigned integer".into(),
            )
        }),
        None => Ok(default),
    }
}

fn parse_workspace_enum<T>(column: usize, value: &str) -> rusqlite::Result<T>
where
    T: std::str::FromStr<Err = String>,
{
    value.parse().map_err(|err| {
        rusqlite::Error::FromSqlConversionFailure(
            column,
            Type::Text,
            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, err)),
        )
    })
}

fn parse_workspace_revision(column: usize, revision: i64) -> rusqlite::Result<u64> {
    u64::try_from(revision).map_err(|err| {
        rusqlite::Error::FromSqlConversionFailure(column, Type::Integer, Box::new(err))
    })
}

fn row_to_knowledge_workspace(row: &rusqlite::Row<'_>) -> rusqlite::Result<KnowledgeWorkspace> {
    let id: String = row.get(0)?;
    let mode: String = row.get(2)?;
    let state: String = row.get(3)?;
    let payload_format: String = row.get(5)?;
    let created_at: String = row.get(7)?;
    let updated_at: String = row.get(8)?;
    let last_reconciled_at: Option<String> = row.get(9)?;

    Ok(KnowledgeWorkspace {
        id: parse_uuid_str(0, &id)?,
        namespace: row.get(1)?,
        mode: parse_workspace_enum(2, &mode)?,
        state: parse_workspace_enum(3, &state)?,
        descriptor_payload: row.get(4)?,
        payload_format: parse_workspace_enum(5, &payload_format)?,
        revision: parse_workspace_revision(6, row.get(6)?)?,
        created_at: parse_dt_strict(7, &created_at)?,
        updated_at: parse_dt_strict(8, &updated_at)?,
        last_reconciled_at: parse_optional_dt_strict(9, last_reconciled_at)?,
    })
}

fn row_to_workspace_document(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<KnowledgeWorkspaceDocument> {
    let id: String = row.get(0)?;
    let workspace_id: String = row.get(1)?;
    let payload_format: String = row.get(4)?;
    let lifecycle_state: String = row.get(5)?;
    let projection_state: String = row.get(6)?;
    let projected_node_id: Option<String> = row.get(7)?;
    let created_at: String = row.get(9)?;
    let updated_at: String = row.get(10)?;

    Ok(KnowledgeWorkspaceDocument {
        id: parse_uuid_str(0, &id)?,
        workspace_id: parse_uuid_str(1, &workspace_id)?,
        path_token: row.get(2)?,
        document_payload: row.get(3)?,
        payload_format: parse_workspace_enum(4, &payload_format)?,
        lifecycle_state: parse_workspace_enum(5, &lifecycle_state)?,
        projection_state: parse_workspace_enum(6, &projection_state)?,
        projected_node_id: projected_node_id
            .map(|value| parse_uuid_str(7, &value))
            .transpose()?,
        revision: parse_workspace_revision(8, row.get(8)?)?,
        created_at: parse_dt_strict(9, &created_at)?,
        updated_at: parse_dt_strict(10, &updated_at)?,
    })
}

fn parse_metadata_json(
    metadata_json: Option<String>,
) -> rusqlite::Result<std::collections::HashMap<String, serde_json::Value>> {
    match metadata_json {
        Some(raw) => serde_json::from_str(&raw).map_err(|err| {
            rusqlite::Error::FromSqlConversionFailure(13, Type::Text, Box::new(err))
        }),
        None => Ok(Default::default()),
    }
}

fn row_to_public_share(row: &rusqlite::Row<'_>) -> rusqlite::Result<PublicShare> {
    let id_str: String = row.get(0)?;
    let node_id_str: String = row.get(1)?;
    let token_hash: String = row.get(2)?;
    let created_at_str: String = row.get(3)?;
    let expires_at_str: Option<String> = row.get(4)?;
    let revoked_at_str: Option<String> = row.get(5)?;

    Ok(PublicShare {
        id: parse_uuid_str(0, &id_str)?,
        node_id: parse_uuid_str(1, &node_id_str)?,
        token_hash,
        created_at: parse_dt_strict(3, &created_at_str)?,
        expires_at: parse_optional_dt_strict(4, expires_at_str)?,
        revoked_at: parse_optional_dt_strict(5, revoked_at_str)?,
    })
}

fn row_to_node_comment(row: &rusqlite::Row<'_>) -> rusqlite::Result<NodeComment> {
    let id_str: String = row.get(0)?;
    let node_id_str: String = row.get(1)?;
    let author: Option<String> = row.get(2)?;
    let body: String = row.get(3)?;
    let created_at_str: String = row.get(4)?;
    let updated_at_str: String = row.get(5)?;
    let resolved_at_str: Option<String> = row.get(6)?;

    Ok(NodeComment {
        id: parse_uuid_str(0, &id_str)?,
        node_id: parse_uuid_str(1, &node_id_str)?,
        author,
        body,
        created_at: parse_dt_strict(4, &created_at_str)?,
        updated_at: parse_dt_strict(5, &updated_at_str)?,
        resolved_at: parse_optional_dt_strict(6, resolved_at_str)?,
    })
}

fn row_to_mcp_connector(row: &rusqlite::Row<'_>) -> rusqlite::Result<McpConnector> {
    let id_str: String = row.get(0)?;
    let name: String = row.get(1)?;
    let description: Option<String> = row.get(2)?;
    let publisher: Option<String> = row.get(3)?;
    let version: String = row.get(4)?;
    let homepage_url: Option<String> = row.get(5)?;
    let repository_url: Option<String> = row.get(6)?;
    let config_schema_json: String = row.get(7)?;
    let capabilities_json: String = row.get(8)?;
    let verified: i32 = row.get(9)?;
    let created_at_str: String = row.get(10)?;
    let updated_at_str: String = row.get(11)?;

    let config_schema =
        serde_json::from_str(&config_schema_json).unwrap_or_else(|_| serde_json::json!({}));
    let capabilities: Vec<String> = serde_json::from_str(&capabilities_json).unwrap_or_default();

    Ok(McpConnector {
        id: parse_uuid_str(0, &id_str)?,
        name,
        description,
        publisher,
        version,
        homepage_url,
        repository_url,
        config_schema,
        capabilities,
        verified: verified != 0,
        created_at: parse_dt_strict(10, &created_at_str)?,
        updated_at: parse_dt_strict(11, &updated_at_str)?,
    })
}

#[async_trait]
impl NodeStore for SqliteNodeStore {
    async fn insert(&self, node: &KnowledgeNode) -> MvResult<()> {
        self.with_conn(|conn| {
                let (title, content, source, metadata_json, payload_ciphertext, payload_wrapped_dek) =
                    self.project_node_for_storage(node)?;

            conn.execute(
                "INSERT INTO knowledge_nodes (id, kind, title, content, source, namespace, importance,
                 created_at, updated_at, last_accessed_at, access_count, version, expires_at, metadata_json, payload_ciphertext, payload_wrapped_dek)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
                params![
                    node.id.to_string(),
                    node.kind.as_str(),
                    title,
                    content,
                    source,
                    node.namespace,
                    node.importance,
                    node.temporal.created_at.to_rfc3339(),
                    node.temporal.updated_at.to_rfc3339(),
                    node.temporal.last_accessed_at.to_rfc3339(),
                    node.temporal.access_count,
                    node.temporal.version,
                    node.temporal.expires_at.map(|dt| dt.to_rfc3339()),
                    metadata_json,
                    payload_ciphertext,
                    payload_wrapped_dek,
                ],
            )
            .map_err(|e| MvError::Storage(format!("insert failed: {e}")))?;

            Self::save_tags(conn, node.id, &node.tags)?;
            Self::log_change(conn, node.id, ChangeOp::Create, None)?;
            Ok(())
        })
    }

    async fn get(&self, id: Uuid) -> MvResult<Option<KnowledgeNode>> {
        self.with_conn(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT id, kind, title, content, source, namespace, importance,
                     created_at, updated_at, last_accessed_at, access_count, version,
                     expires_at, metadata_json, payload_ciphertext, payload_wrapped_dek FROM knowledge_nodes WHERE id = ?1",
                )
                .map_err(|e| MvError::Storage(e.to_string()))?;

            let node = stmt
                .query_row(params![id.to_string()], |row| self.row_to_node(row))
                .optional()
                .map_err(|e| MvError::Storage(e.to_string()))?;

            if let Some(mut node) = node {
                node.tags = Self::load_tags(conn, node.id)?;
                Ok(Some(node))
            } else {
                Ok(None)
            }
        })
    }

    async fn update(&self, node: &KnowledgeNode) -> MvResult<()> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let (title, content, source, metadata_json, payload_ciphertext, payload_wrapped_dek) =
            self.project_node_for_storage(node)?;

        let rows = conn
            .execute(
                "UPDATE knowledge_nodes SET kind = ?2, title = ?3, content = ?4, source = ?5, payload_ciphertext = ?14, payload_wrapped_dek = ?15,
                 namespace = ?6, importance = ?7, updated_at = ?8, last_accessed_at = ?9,
                 access_count = ?10, version = ?11, expires_at = ?12, metadata_json = ?13
                 WHERE id = ?1",
                params![
                    node.id.to_string(),
                    node.kind.as_str(),
                    title,
                    content,
                    source,
                    node.namespace,
                    node.importance,
                    node.temporal.updated_at.to_rfc3339(),
                    node.temporal.last_accessed_at.to_rfc3339(),
                    node.temporal.access_count,
                    node.temporal.version,
                    node.temporal.expires_at.map(|dt| dt.to_rfc3339()),
                    metadata_json,
                    payload_ciphertext,
                    payload_wrapped_dek,
                ],
            )
            .map_err(|e| MvError::Storage(format!("update failed: {e}")))?;

        if rows == 0 {
            return Err(MvError::NodeNotFound(node.id));
        }

        Self::save_tags(&conn, node.id, &node.tags)?;
        Self::log_change(&conn, node.id, ChangeOp::Update, None)?;
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> MvResult<bool> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        Self::log_change(&conn, id, ChangeOp::Delete, None)?;
        let rows = conn
            .execute(
                "DELETE FROM knowledge_nodes WHERE id = ?1",
                params![id.to_string()],
            )
            .map_err(|e| MvError::Storage(format!("delete failed: {e}")))?;
        Ok(rows > 0)
    }

    async fn list(
        &self,
        filters: &QueryFilters,
        limit: usize,
        offset: usize,
    ) -> MvResult<Vec<KnowledgeNode>> {
        self.with_conn(|conn| {
        let mut sql = String::from(
            "SELECT id, kind, title, content, source, namespace, importance,
             created_at, updated_at, last_accessed_at, access_count, version,
             expires_at, metadata_json, payload_ciphertext, payload_wrapped_dek FROM knowledge_nodes WHERE 1=1",
        );
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        let mut param_idx = 1;

        if let Some(ref ns) = filters.namespace {
            sql.push_str(&format!(" AND namespace = ?{param_idx}"));
            param_values.push(Box::new(ns.clone()));
            param_idx += 1;
        }

        if let Some(ref kinds) = filters.kinds {
            if !kinds.is_empty() {
                let placeholders: Vec<String> = kinds
                    .iter()
                    .map(|_| {
                        let p = format!("?{param_idx}");
                        param_idx += 1;
                        p
                    })
                    .collect();
                sql.push_str(&format!(" AND kind IN ({})", placeholders.join(",")));
                for k in kinds {
                    param_values.push(Box::new(k.as_str().to_string()));
                }
            }
        }

        if let Some(min_imp) = filters.min_importance {
            sql.push_str(&format!(" AND importance >= ?{param_idx}"));
            param_values.push(Box::new(min_imp));
            param_idx += 1;
        }

        if let Some(ref after) = filters.created_after {
            sql.push_str(&format!(" AND created_at >= ?{param_idx}"));
            param_values.push(Box::new(after.to_rfc3339()));
            param_idx += 1;
        }

        if let Some(ref before) = filters.created_before {
            sql.push_str(&format!(" AND created_at <= ?{param_idx}"));
            param_values.push(Box::new(before.to_rfc3339()));
            param_idx += 1;
        }

        if let Some(ref filter_tags) = filters.tags {
            if !filter_tags.is_empty() {
                let placeholders: Vec<String> = filter_tags
                    .iter()
                    .map(|_| {
                        let p = format!("?{param_idx}");
                        param_idx += 1;
                        p
                    })
                    .collect();

                sql.push_str(&format!(
                    " AND EXISTS (SELECT 1 FROM node_tags nt WHERE nt.node_id = knowledge_nodes.id AND nt.tag IN ({}))",
                    placeholders.join(",")
                ));

                for tag in filter_tags {
                    param_values.push(Box::new(tag.clone()));
                }
            }
        }

        sql.push_str(&format!(
            " ORDER BY updated_at DESC LIMIT ?{param_idx} OFFSET ?{}",
            param_idx + 1
        ));
        param_values.push(Box::new(limit as i64));
        param_values.push(Box::new(offset as i64));

        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let rows = stmt
            .query_map(params_refs.as_slice(), |row| self.row_to_node(row))
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut nodes = Vec::new();
        for row in rows {
            let mut node = row.map_err(|e| MvError::Storage(e.to_string()))?;
            node.tags = Self::load_tags(conn, node.id)?;
            nodes.push(node);
        }

        Ok(nodes)
        })
    }

    async fn touch(&self, id: Uuid) -> MvResult<()> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let now = Utc::now().to_rfc3339();
        let rows = conn
            .execute(
                "UPDATE knowledge_nodes SET last_accessed_at = ?2, access_count = access_count + 1 WHERE id = ?1",
                params![id.to_string(), now],
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;
        if rows == 0 {
            return Err(MvError::NodeNotFound(id));
        }
        Ok(())
    }

    async fn count(&self, filters: &QueryFilters) -> MvResult<usize> {
        self.with_conn(|conn| {
        let mut sql = String::from("SELECT COUNT(*) FROM knowledge_nodes WHERE 1=1");
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        let mut param_idx = 1;

        if let Some(ref ns) = filters.namespace {
            sql.push_str(&format!(" AND namespace = ?{param_idx}"));
            param_values.push(Box::new(ns.clone()));
            param_idx += 1;
        }

        if let Some(ref kinds) = filters.kinds {
            if !kinds.is_empty() {
                let placeholders: Vec<String> = kinds
                    .iter()
                    .map(|_| {
                        let p = format!("?{param_idx}");
                        param_idx += 1;
                        p
                    })
                    .collect();
                sql.push_str(&format!(" AND kind IN ({})", placeholders.join(",")));
                for k in kinds {
                    param_values.push(Box::new(k.as_str().to_string()));
                }
            }
        }
        if let Some(min_imp) = filters.min_importance {
            sql.push_str(&format!(" AND importance >= ?{param_idx}"));
            param_values.push(Box::new(min_imp));
            param_idx += 1;
        }

        if let Some(ref after) = filters.created_after {
            sql.push_str(&format!(" AND created_at >= ?{param_idx}"));
            param_values.push(Box::new(after.to_rfc3339()));
            param_idx += 1;
        }

        if let Some(ref before) = filters.created_before {
            sql.push_str(&format!(" AND created_at <= ?{param_idx}"));
            param_values.push(Box::new(before.to_rfc3339()));
            param_idx += 1;
        }

        if let Some(ref filter_tags) = filters.tags {
            if !filter_tags.is_empty() {
                let placeholders: Vec<String> = filter_tags
                    .iter()
                    .map(|_| {
                        let p = format!("?{param_idx}");
                        param_idx += 1;
                        p
                    })
                    .collect();

                sql.push_str(&format!(
                    " AND EXISTS (SELECT 1 FROM node_tags nt WHERE nt.node_id = knowledge_nodes.id AND nt.tag IN ({}))",
                    placeholders.join(",")
                ));

                for tag in filter_tags {
                    param_values.push(Box::new(tag.clone()));
                }
            }
        }

        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();

        let count: usize = conn
            .query_row(&sql, params_refs.as_slice(), |row| row.get(0))
            .map_err(|e| MvError::Storage(e.to_string()))?;

        Ok(count)
        })
    }
}

impl SqliteNodeStore {
    fn source_lookup_key(&self, value: &str) -> MvResult<String> {
        let digest = if self.sealed_mode() {
            let key = self.derive_namespace_kek(INTEROPERABILITY_REGISTRY_NAMESPACE)?;
            let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(&key)
                .map_err(|err| MvError::Storage(format!("create source lookup HMAC: {err}")))?;
            mac.update(value.as_bytes());
            mac.finalize().into_bytes().to_vec()
        } else {
            Sha256::digest(value.as_bytes()).to_vec()
        };
        Ok(digest.iter().map(|byte| format!("{byte:02x}")).collect())
    }

    fn ensure_local_context_node(transaction: &rusqlite::Transaction<'_>) -> MvResult<Uuid> {
        let generated = Uuid::now_v7();
        transaction
            .execute(
                "INSERT OR IGNORE INTO interoperability_local_identity
                 (singleton, node_id, created_at) VALUES (1, ?1, ?2)",
                params![generated.to_string(), Utc::now().to_rfc3339()],
            )
            .map_err(|err| MvError::Storage(format!("ensure local context identity: {err}")))?;
        let node_id: String = transaction
            .query_row(
                "SELECT node_id FROM interoperability_local_identity WHERE singleton = 1",
                [],
                |row| row.get(0),
            )
            .map_err(|err| MvError::Storage(format!("load local context identity: {err}")))?;
        Uuid::parse_str(&node_id)
            .map_err(|err| MvError::Storage(format!("invalid local context identity: {err}")))
    }

    fn require_active_event_schema(connection: &Connection, event: &EventEnvelope) -> MvResult<()> {
        let schema: Option<(String, String)> = connection
            .query_row(
                "SELECT lifecycle, definition_json
                 FROM interoperability_public_schemas
                 WHERE schema_uri = ?1 AND schema_version = ?2",
                params![event.schema.uri.as_str(), &event.schema.version],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|err| MvError::Storage(format!("resolve event schema: {err}")))?;
        let Some((lifecycle, definition_json)) = schema else {
            return Err(MvError::InvalidInput(
                "event schema is not registered".into(),
            ));
        };
        if lifecycle != "active" {
            return Err(MvError::InvalidInput(format!(
                "event schema is not active: {lifecycle}"
            )));
        }
        let definition: serde_json::Value = serde_json::from_str(&definition_json)
            .map_err(|err| MvError::Storage(format!("decode registered event schema: {err}")))?;
        match definition
            .get(EVENT_TYPE_SCHEMA_EXTENSION)
            .and_then(|value| value.as_str())
        {
            Some(registered_type) if registered_type == event.event_type => Ok(()),
            Some(registered_type) => Err(MvError::InvalidInput(format!(
                "event type {} does not match registered schema type {registered_type}",
                event.event_type
            ))),
            None => Err(MvError::InvalidInput(
                "event schema is not registered for event-envelope admission".into(),
            )),
        }
    }

    fn validate_governance_event(
        connection: &Connection,
        event: &EventEnvelope,
        local_node_id: Uuid,
        expected_type: &str,
        expected_schema_name: &str,
    ) -> MvResult<()> {
        event.validate().map_err(MvError::InvalidInput)?;
        if event.source != StableUri::node(local_node_id) {
            return Err(MvError::InvalidInput(
                "governance event source must identify the local context node".into(),
            ));
        }
        let expected_schema = StableUri::schema(expected_schema_name)
            .map_err(|err| MvError::Storage(format!("invalid built-in schema URI: {err}")))?;
        if event.event_type != expected_type
            || event.schema.uri != expected_schema
            || event.schema.version != "1.0.0"
        {
            return Err(MvError::InvalidInput(format!(
                "{expected_type} requires its registered 1.0.0 event schema"
            )));
        }
        if !event.provenance.iter().any(|reference| {
            reference.resource == event.subject
                && reference.relation == ProvenanceRelation::PrimarySource
        }) {
            return Err(MvError::InvalidInput(
                "governance event must identify its subject as a primary source".into(),
            ));
        }
        Self::require_active_event_schema(connection, event)
    }

    fn resolve_governance_replay(
        connection: &Connection,
        event: &EventEnvelope,
        expected_type: &str,
    ) -> MvResult<Option<EventEnvelope>> {
        let existing: Option<(String, String, String)> = connection
            .query_row(
                "SELECT envelope_json, payload_digest, event_type
                 FROM interoperability_outbox
                 WHERE source_uri = ?1 AND principal_uri = ?2 AND idempotency_key = ?3",
                params![
                    event.source.as_str(),
                    event.principal.as_str(),
                    event.idempotency_key.as_str()
                ],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .map_err(|err| MvError::Storage(format!("check governance replay: {err}")))?;
        let Some((envelope_json, payload_digest, event_type)) = existing else {
            return Ok(None);
        };
        if payload_digest != event.payload_digest || event_type != expected_type {
            return Err(MvError::IdempotencyConflict(
                "idempotency key was already used by a different governance command".into(),
            ));
        }
        Self::decode_outbox_event(&envelope_json).map(Some)
    }

    fn insert_outbox_event(
        transaction: &rusqlite::Transaction<'_>,
        event: &EventEnvelope,
    ) -> MvResult<()> {
        event.validate().map_err(MvError::InvalidInput)?;
        Self::require_active_event_schema(transaction, event)?;
        let event_json = serde_json::to_string(event)?;
        transaction
            .execute(
                "INSERT INTO interoperability_outbox
                 (event_id, source_uri, principal_uri, event_type, subject_uri, schema_uri,
                  schema_version, correlation_id, causation_id, idempotency_key,
                  payload_digest, envelope_json, created_at, next_attempt_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?13, ?13)",
                params![
                    event.id.to_string(),
                    event.source.as_str(),
                    event.principal.as_str(),
                    &event.event_type,
                    event.subject.as_str(),
                    event.schema.uri.as_str(),
                    &event.schema.version,
                    event.correlation_id.to_string(),
                    event.causation_id.map(|id| id.to_string()),
                    event.idempotency_key.as_str(),
                    &event.payload_digest,
                    event_json,
                    event.occurred_at.to_rfc3339(),
                ],
            )
            .map_err(|err| MvError::Storage(format!("enqueue interoperability event: {err}")))?;
        Ok(())
    }

    fn row_to_public_schema(row: &rusqlite::Row<'_>) -> rusqlite::Result<PublicSchemaRecord> {
        let schema_uri: String = row.get(0)?;
        let schema_version: String = row.get(1)?;
        let definition_json: String = row.get(3)?;
        let lifecycle: String = row.get(5)?;
        let owner_uri: String = row.get(6)?;
        let created_at: String = row.get(7)?;
        let deprecated_at: Option<String> = row.get(8)?;
        let record = PublicSchemaRecord {
            schema: SchemaReference::new(
                StableUri::parse(schema_uri)
                    .map_err(|err| Self::as_sql_conversion_error(0, err))?,
                schema_version,
            )
            .map_err(|err| Self::as_sql_conversion_error(1, err))?,
            media_type: row.get(2)?,
            definition: serde_json::from_str(&definition_json).map_err(|err| {
                Self::as_sql_conversion_error(3, format!("invalid schema JSON: {err}"))
            })?,
            content_digest: row.get(4)?,
            lifecycle: lifecycle
                .parse()
                .map_err(|err: String| Self::as_sql_conversion_error(5, err))?,
            owner: StableUri::parse(owner_uri)
                .map_err(|err| Self::as_sql_conversion_error(6, err))?,
            created_at: parse_dt_strict(7, &created_at)?,
            deprecated_at: parse_optional_dt_strict(8, deprecated_at)?,
        };
        record
            .validate()
            .map_err(|err| Self::as_sql_conversion_error(3, err))?;
        Ok(record)
    }

    fn load_public_schema_from_connection(
        connection: &Connection,
        reference: &SchemaReference,
    ) -> MvResult<Option<PublicSchemaRecord>> {
        connection
            .query_row(
                "SELECT schema_uri, schema_version, media_type, definition_json,
                        content_digest, lifecycle, owner_uri, created_at, deprecated_at
                 FROM interoperability_public_schemas
                 WHERE schema_uri = ?1 AND schema_version = ?2",
                params![reference.uri.as_str(), &reference.version],
                Self::row_to_public_schema,
            )
            .optional()
            .map_err(|err| MvError::Storage(format!("load public schema: {err}")))
    }

    fn validate_context_node_event_record(
        event: &EventEnvelope,
        context_node: &ContextNodeRecord,
        require_revision: bool,
        require_capability_digest: bool,
    ) -> MvResult<()> {
        let node_id = event_context_node_id(event)?;
        let revision = event_context_node_revision(event, context_node.revision)?;
        let record_digest = context_node.semantic_digest();
        if event.subject != context_node.node_uri
            || node_id != context_node.node_id
            || (require_revision && revision != context_node.revision)
            || event
                .data
                .get("record_digest")
                .and_then(|value| value.as_str())
                != Some(record_digest.as_str())
            || event.payload_digest != record_digest
            || (require_capability_digest
                && event
                    .data
                    .get("capability_digest")
                    .and_then(|value| value.as_str())
                    != Some(context_node.capability_manifest.content_digest.as_str()))
        {
            return Err(MvError::InvalidInput(
                "context-node event must match the governed descriptor revision".into(),
            ));
        }
        Ok(())
    }

    fn row_to_context_node(&self, row: &rusqlite::Row<'_>) -> rusqlite::Result<ContextNodeRecord> {
        let node_id: String = row.get(0)?;
        let revision: u64 = row.get(1)?;
        let node_uri: String = row.get(2)?;
        let node_type: String = row.get(3)?;
        let owner_actor_uri: String = row.get(4)?;
        let governing_node_uri: String = row.get(5)?;
        let trust_class: String = row.get(6)?;
        let status: String = row.get(7)?;
        let capability_digest: String = row.get(8)?;
        let payload: Vec<u8> = row.get(9)?;
        let payload_format: String = row.get(10)?;
        let wrapped_dek: Option<String> = row.get(11)?;
        let created_at: String = row.get(12)?;
        let updated_at: String = row.get(13)?;

        let context_node: ContextNodeRecord = self
            .decode_governance_record(
                &payload,
                &payload_format,
                wrapped_dek.as_deref(),
                "context-node descriptor",
            )
            .map_err(|err| Self::as_sql_conversion_error(9, err.to_string()))?;
        context_node
            .validate()
            .map_err(|err| Self::as_sql_conversion_error(9, err))?;

        let stored_node_type: ContextNodeType = node_type
            .parse()
            .map_err(|err: String| Self::as_sql_conversion_error(3, err))?;
        let stored_trust_class = ContextNodeTrustClass::parse(trust_class)
            .map_err(|err| Self::as_sql_conversion_error(6, err))?;
        let stored_status: ContextNodeStatus = status
            .parse()
            .map_err(|err: String| Self::as_sql_conversion_error(7, err))?;

        if context_node.node_id != parse_uuid_str(0, &node_id)?
            || context_node.revision != revision
            || context_node.node_uri.as_str() != node_uri
            || context_node.node_type != stored_node_type
            || context_node.owner_actor_id.as_str() != owner_actor_uri
            || context_node.governing_node_id.as_str() != governing_node_uri
            || context_node.trust_class != stored_trust_class
            || context_node.status != stored_status
            || context_node.capability_manifest.content_digest != capability_digest
            || context_node.created_at != parse_dt_strict(12, &created_at)?
            || context_node.updated_at != parse_dt_strict(13, &updated_at)?
        {
            return Err(Self::as_sql_conversion_error(
                9,
                "context-node payload does not match its governed index",
            ));
        }
        Ok(context_node)
    }

    fn load_context_node_from_connection(
        &self,
        connection: &Connection,
        node_id: Uuid,
    ) -> MvResult<Option<ContextNodeRecord>> {
        connection
            .query_row(
                "SELECT node_id, revision, node_uri, node_type, owner_actor_uri,
                        governing_node_uri, trust_class, status, capability_digest,
                        record_payload, payload_format, payload_wrapped_dek, created_at, updated_at
                 FROM interoperability_context_nodes
                 WHERE node_id = ?1",
                params![node_id.to_string()],
                |row| self.row_to_context_node(row),
            )
            .optional()
            .map_err(|err| MvError::Storage(format!("load context node: {err}")))
    }

    fn load_context_node_revision_from_connection(
        &self,
        connection: &Connection,
        node_id: Uuid,
        revision: u64,
    ) -> MvResult<Option<ContextNodeRecord>> {
        connection
            .query_row(
                "SELECT node_id, revision, node_uri, node_type, owner_actor_uri,
                        governing_node_uri, trust_class, status, capability_digest,
                        record_payload, payload_format, payload_wrapped_dek, created_at, updated_at
                 FROM interoperability_context_nodes
                 WHERE node_id = ?1 AND revision = ?2
                 UNION ALL
                 SELECT node_id, revision, node_uri, node_type, owner_actor_uri,
                        governing_node_uri, trust_class, status, capability_digest,
                        record_payload, payload_format, payload_wrapped_dek, created_at, updated_at
                 FROM interoperability_context_node_history
                 WHERE node_id = ?1 AND revision = ?2
                 LIMIT 1",
                params![node_id.to_string(), revision],
                |row| self.row_to_context_node(row),
            )
            .optional()
            .map_err(|err| MvError::Storage(format!("load context-node revision: {err}")))
    }

    fn require_active_context_node(
        &self,
        connection: &Connection,
        node_id: Uuid,
    ) -> MvResult<ContextNodeRecord> {
        let context_node = self
            .load_context_node_from_connection(connection, node_id)?
            .ok_or_else(|| {
                MvError::InvalidInput("governing context node is not registered".into())
            })?;
        if context_node.status != ContextNodeStatus::Active {
            return Err(MvError::InvalidInput(format!(
                "governing context node is not active: {}",
                context_node.status.as_str()
            )));
        }
        Ok(context_node)
    }

    fn require_active_schema_references(
        connection: &Connection,
        manifest: &ContextCapabilityManifest,
    ) -> MvResult<()> {
        for reference in &manifest.supported_schema_versions {
            let lifecycle: Option<String> = connection
                .query_row(
                    "SELECT lifecycle
                     FROM interoperability_public_schemas
                     WHERE schema_uri = ?1 AND schema_version = ?2",
                    params![reference.uri.as_str(), &reference.version],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|err| {
                    MvError::Storage(format!("resolve advertised schema reference: {err}"))
                })?;
            match lifecycle.as_deref() {
                Some("active") => {}
                Some(other) => {
                    return Err(MvError::InvalidInput(format!(
                        "advertised schema is not active: {}@{} ({other})",
                        reference.uri, reference.version
                    )))
                }
                None => {
                    return Err(MvError::InvalidInput(format!(
                        "advertised schema is not registered: {}@{}",
                        reference.uri, reference.version
                    )))
                }
            }
        }
        Ok(())
    }

    fn validate_identity_event_record(
        event: &EventEnvelope,
        identity: &IdentityRecord,
        require_revision: bool,
        require_identity_fields: bool,
    ) -> MvResult<()> {
        let principal_id = event_identity_principal_id(event)?;
        let revision = event_identity_revision(event, identity.revision)?;
        let record_digest = identity.semantic_digest();
        if event.subject != identity.principal_uri
            || principal_id != identity.principal_id
            || (require_revision && revision != identity.revision)
            || event
                .data
                .get("record_digest")
                .and_then(|value| value.as_str())
                != Some(record_digest.as_str())
            || event.payload_digest != record_digest
            || (require_identity_fields
                && (event
                    .data
                    .get("actor_kind")
                    .and_then(|value| value.as_str())
                    != Some(identity.actor_kind.as_str())
                    || event
                        .data
                        .get("status")
                        .and_then(|value| value.as_str())
                        != Some(identity.status.as_str())
                    || event
                        .data
                        .get("subject_binding_digest")
                        .and_then(|value| value.as_str())
                        != Some(identity.subject_binding_digest.as_str())))
        {
            return Err(MvError::InvalidInput(
                "identity event must match the governed identity revision".into(),
            ));
        }
        Ok(())
    }

    fn row_to_identity(&self, row: &rusqlite::Row<'_>) -> rusqlite::Result<IdentityRecord> {
        let principal_id: String = row.get(0)?;
        let revision: u64 = row.get(1)?;
        let principal_uri: String = row.get(2)?;
        let governing_node_uri: String = row.get(3)?;
        let subject_binding: String = row.get(4)?;
        let subject_binding_digest: String = row.get(5)?;
        let actor_kind: String = row.get(6)?;
        let status: String = row.get(7)?;
        let record_digest: String = row.get(8)?;
        let payload: Vec<u8> = row.get(9)?;
        let payload_format: String = row.get(10)?;
        let wrapped_dek: Option<String> = row.get(11)?;

        let identity: IdentityRecord = self
            .decode_governance_record(
                &payload,
                &payload_format,
                wrapped_dek.as_deref(),
                "identity record",
            )
            .map_err(|err| Self::as_sql_conversion_error(9, err.to_string()))?;
        identity
            .validate()
            .map_err(|err| Self::as_sql_conversion_error(9, err))?;

        let stored_actor_kind: ActorKind = actor_kind
            .parse()
            .map_err(|err: String| Self::as_sql_conversion_error(6, err))?;
        let stored_status: IdentityStatus = status
            .parse()
            .map_err(|err: String| Self::as_sql_conversion_error(7, err))?;
        let stored_principal_id = parse_uuid_str(0, &principal_id)?;
        let stored_principal_uri = StableUri::parse(principal_uri)
            .map_err(|err| Self::as_sql_conversion_error(2, err))?;
        let stored_governing_node_uri = StableUri::parse(governing_node_uri)
            .map_err(|err| Self::as_sql_conversion_error(3, err))?;

        if identity.principal_id != stored_principal_id
            || identity.revision != revision
            || identity.principal_uri != stored_principal_uri
            || identity.governing_node_uri != stored_governing_node_uri
            || identity.subject_binding != subject_binding
            || identity.subject_binding_digest != subject_binding_digest
            || identity.actor_kind != stored_actor_kind
            || identity.status != stored_status
            || identity.semantic_digest() != record_digest
        {
            return Err(Self::as_sql_conversion_error(
                9,
                "identity indexed fields do not match its payload",
            ));
        }

        Ok(identity)
    }

    fn load_identity_from_connection(
        &self,
        connection: &Connection,
        principal_id: Uuid,
    ) -> MvResult<Option<IdentityRecord>> {
        connection
            .query_row(
                "SELECT principal_id, revision, principal_uri, governing_node_uri, subject_binding,
                        subject_binding_digest, actor_kind, status, record_digest, record_payload,
                        payload_format, payload_wrapped_dek, created_at, updated_at
                 FROM interoperability_identity_records
                 WHERE principal_id = ?1",
                params![principal_id.to_string()],
                |row| self.row_to_identity(row),
            )
            .optional()
            .map_err(|err| MvError::Storage(format!("load identity record: {err}")))
    }

    fn load_identity_by_subject_from_connection(
        &self,
        connection: &Connection,
        governing_node_uri: &StableUri,
        subject_binding: &str,
    ) -> MvResult<Option<IdentityRecord>> {
        let digest = IdentityRecord::subject_binding_digest(subject_binding);
        connection
            .query_row(
                "SELECT principal_id, revision, principal_uri, governing_node_uri, subject_binding,
                        subject_binding_digest, actor_kind, status, record_digest, record_payload,
                        payload_format, payload_wrapped_dek, created_at, updated_at
                 FROM interoperability_identity_records
                 WHERE governing_node_uri = ?1 AND subject_binding_digest = ?2",
                params![governing_node_uri.as_str(), digest],
                |row| self.row_to_identity(row),
            )
            .optional()
            .map_err(|err| MvError::Storage(format!("load identity by subject binding: {err}")))
    }

    fn insert_identity(
        &self,
        transaction: &rusqlite::Transaction<'_>,
        identity: &IdentityRecord,
    ) -> MvResult<()> {
        let (payload, payload_format, wrapped_dek) =
            self.encode_governance_record(identity, "identity record")?;
        transaction
            .execute(
                "INSERT INTO interoperability_identity_records
                 (principal_id, revision, principal_uri, governing_node_uri, subject_binding,
                  subject_binding_digest, actor_kind, status, record_digest, record_payload,
                  payload_format, payload_wrapped_dek, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                params![
                    identity.principal_id.to_string(),
                    identity.revision,
                    identity.principal_uri.as_str(),
                    identity.governing_node_uri.as_str(),
                    identity.subject_binding,
                    identity.subject_binding_digest,
                    identity.actor_kind.as_str(),
                    identity.status.as_str(),
                    identity.semantic_digest(),
                    payload,
                    payload_format,
                    wrapped_dek,
                    identity.created_at.to_rfc3339(),
                    identity.updated_at.to_rfc3339(),
                ],
            )
            .map_err(|err| MvError::Storage(format!("insert identity record: {err}")))?;
        Ok(())
    }
    fn insert_context_node(
        &self,
        transaction: &rusqlite::Transaction<'_>,
        context_node: &ContextNodeRecord,
    ) -> MvResult<()> {
        let (payload, payload_format, wrapped_dek) =
            self.encode_governance_record(context_node, "context-node descriptor")?;
        transaction
            .execute(
                "INSERT INTO interoperability_context_nodes
                 (node_id, revision, node_uri, node_type, owner_actor_uri, governing_node_uri,
                  trust_class, status, capability_digest, record_payload, payload_format,
                  payload_wrapped_dek, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                params![
                    context_node.node_id.to_string(),
                    context_node.revision,
                    context_node.node_uri.as_str(),
                    context_node.node_type.as_str(),
                    context_node.owner_actor_id.as_str(),
                    context_node.governing_node_id.as_str(),
                    context_node.trust_class.as_str(),
                    context_node.status.as_str(),
                    &context_node.capability_manifest.content_digest,
                    payload,
                    payload_format,
                    wrapped_dek,
                    context_node.created_at.to_rfc3339(),
                    context_node.updated_at.to_rfc3339(),
                ],
            )
            .map_err(|err| MvError::Storage(format!("insert context node: {err}")))?;
        Ok(())
    }

    fn update_context_node(
        &self,
        transaction: &rusqlite::Transaction<'_>,
        expected_revision: u64,
        replacement: &ContextNodeRecord,
    ) -> MvResult<()> {
        let (payload, payload_format, wrapped_dek) =
            self.encode_governance_record(replacement, "context-node descriptor")?;
        let changed = transaction
            .execute(
                "UPDATE interoperability_context_nodes
                 SET revision = ?3, node_uri = ?4, node_type = ?5, owner_actor_uri = ?6,
                     governing_node_uri = ?7, trust_class = ?8, status = ?9,
                     capability_digest = ?10, record_payload = ?11, payload_format = ?12,
                     payload_wrapped_dek = ?13, created_at = ?14, updated_at = ?15
                 WHERE node_id = ?1 AND revision = ?2",
                params![
                    replacement.node_id.to_string(),
                    expected_revision,
                    replacement.revision,
                    replacement.node_uri.as_str(),
                    replacement.node_type.as_str(),
                    replacement.owner_actor_id.as_str(),
                    replacement.governing_node_id.as_str(),
                    replacement.trust_class.as_str(),
                    replacement.status.as_str(),
                    &replacement.capability_manifest.content_digest,
                    payload,
                    payload_format,
                    wrapped_dek,
                    replacement.created_at.to_rfc3339(),
                    replacement.updated_at.to_rfc3339(),
                ],
            )
            .map_err(|err| MvError::Storage(format!("update context node: {err}")))?;
        if changed != 1 {
            return Err(MvError::InvalidInput(
                "context-node expected revision does not match current state".into(),
            ));
        }
        Ok(())
    }

    fn validate_authority_grant_event_record(
        event: &EventEnvelope,
        grant: &AuthorityGrant,
        require_revision: bool,
        require_identity_fields: bool,
    ) -> MvResult<()> {
        let grant_id = event_authority_grant_id(event)?;
        let revision = event_authority_grant_revision(event, grant.revision)?;
        let record_digest = grant.semantic_digest();
        if event.subject != grant.grant_uri
            || grant_id != grant.grant_id
            || (require_revision && revision != grant.revision)
            || event
                .data
                .get("record_digest")
                .and_then(|value| value.as_str())
                != Some(record_digest.as_str())
            || event.payload_digest != record_digest
            || (require_identity_fields
                && (event
                    .data
                    .get("grant_kind")
                    .and_then(|value| value.as_str())
                    != Some(grant.kind.as_str())
                    || event
                        .data
                        .get("grantee_uri")
                        .and_then(|value| value.as_str())
                        != Some(grant.grantee.as_str())
                    || event
                        .data
                        .get("governing_node_uri")
                        .and_then(|value| value.as_str())
                        != Some(grant.governing_node.as_str())))
        {
            return Err(MvError::InvalidInput(
                "authority-grant event must match the governed grant revision".into(),
            ));
        }
        Ok(())
    }

    fn row_to_authority_grant(&self, row: &rusqlite::Row<'_>) -> rusqlite::Result<AuthorityGrant> {
        let grant_id: String = row.get(0)?;
        let revision: u64 = row.get(1)?;
        let grant_uri: String = row.get(2)?;
        let grant_kind: String = row.get(3)?;
        let grantor_uri: String = row.get(4)?;
        let grantee_uri: String = row.get(5)?;
        let governing_node_uri: String = row.get(6)?;
        let status: String = row.get(7)?;
        let parent_grant_id: Option<String> = row.get(8)?;
        let not_before: String = row.get(9)?;
        let expires_at: String = row.get(10)?;
        let payload: Vec<u8> = row.get(11)?;
        let payload_format: String = row.get(12)?;
        let wrapped_dek: Option<String> = row.get(13)?;
        let created_at: String = row.get(14)?;
        let updated_at: String = row.get(15)?;

        let grant: AuthorityGrant = self
            .decode_governance_record(
                &payload,
                &payload_format,
                wrapped_dek.as_deref(),
                "authority grant",
            )
            .map_err(|err| Self::as_sql_conversion_error(11, err.to_string()))?;
        grant
            .validate()
            .map_err(|err| Self::as_sql_conversion_error(11, err))?;
        let stored_kind: AuthorityGrantKind = grant_kind
            .parse()
            .map_err(|err: String| Self::as_sql_conversion_error(3, err))?;
        let stored_status: AuthorityGrantStatus = status
            .parse()
            .map_err(|err: String| Self::as_sql_conversion_error(7, err))?;
        let stored_parent = parent_grant_id
            .as_deref()
            .map(|value| parse_uuid_str(8, value))
            .transpose()?;

        if grant.grant_id != parse_uuid_str(0, &grant_id)?
            || grant.revision != revision
            || grant.grant_uri.as_str() != grant_uri
            || grant.kind != stored_kind
            || grant.grantor.as_str() != grantor_uri
            || grant.grantee.as_str() != grantee_uri
            || grant.governing_node.as_str() != governing_node_uri
            || grant.status != stored_status
            || grant.parent_grant_id != stored_parent
            || grant.not_before != parse_dt_strict(9, &not_before)?
            || grant.expires_at != parse_dt_strict(10, &expires_at)?
            || grant.created_at != parse_dt_strict(14, &created_at)?
            || grant.updated_at != parse_dt_strict(15, &updated_at)?
        {
            return Err(Self::as_sql_conversion_error(
                11,
                "authority-grant payload does not match its governed index",
            ));
        }
        Ok(grant)
    }

    fn load_authority_grant_from_connection(
        &self,
        connection: &Connection,
        grant_id: Uuid,
    ) -> MvResult<Option<AuthorityGrant>> {
        connection
            .query_row(
                "SELECT grant_id, revision, grant_uri, grant_kind, grantor_uri, grantee_uri,
                        governing_node_uri, status, parent_grant_id, not_before, expires_at,
                        record_payload, payload_format, payload_wrapped_dek, created_at, updated_at
                 FROM interoperability_authority_grants
                 WHERE grant_id = ?1",
                params![grant_id.to_string()],
                |row| self.row_to_authority_grant(row),
            )
            .optional()
            .map_err(|err| MvError::Storage(format!("load authority grant: {err}")))
    }

    fn load_authority_grant_revision_from_connection(
        &self,
        connection: &Connection,
        grant_id: Uuid,
        revision: u64,
    ) -> MvResult<Option<AuthorityGrant>> {
        connection
            .query_row(
                "SELECT grant_id, revision, grant_uri, grant_kind, grantor_uri, grantee_uri,
                        governing_node_uri, status, parent_grant_id, not_before, expires_at,
                        record_payload, payload_format, payload_wrapped_dek, created_at, updated_at
                 FROM interoperability_authority_grants
                 WHERE grant_id = ?1 AND revision = ?2
                 UNION ALL
                 SELECT grant_id, revision, grant_uri, grant_kind, grantor_uri, grantee_uri,
                        governing_node_uri, status, parent_grant_id, not_before, expires_at,
                        record_payload, payload_format, payload_wrapped_dek, created_at, updated_at
                 FROM interoperability_authority_grant_history
                 WHERE grant_id = ?1 AND revision = ?2
                 LIMIT 1",
                params![grant_id.to_string(), revision],
                |row| self.row_to_authority_grant(row),
            )
            .optional()
            .map_err(|err| MvError::Storage(format!("load authority-grant revision: {err}")))
    }

    fn admission_decisions_match(left: &AdmissionDecision, right: &AdmissionDecision) -> bool {
        match (left, right) {
            (
                AdmissionDecision::Admitted {
                    grant_id: left_grant,
                    grant_uri: left_uri,
                    grant_kind: left_kind,
                    capability: left_capability,
                    delegation_depth_remaining: left_depth,
                    ..
                },
                AdmissionDecision::Admitted {
                    grant_id: right_grant,
                    grant_uri: right_uri,
                    grant_kind: right_kind,
                    capability: right_capability,
                    delegation_depth_remaining: right_depth,
                    ..
                },
            ) => {
                left_grant == right_grant
                    && left_uri == right_uri
                    && left_kind == right_kind
                    && left_capability == right_capability
                    && left_depth == right_depth
            }
            (
                AdmissionDecision::Denied {
                    reason: left_reason,
                    ..
                },
                AdmissionDecision::Denied {
                    reason: right_reason,
                    ..
                },
            ) => left_reason == right_reason,
            _ => false,
        }
    }

    fn insert_command_admission_decision(
        connection: &Connection,
        record: &CommandAdmissionDecisionRecord,
    ) -> MvResult<()> {
        let (
            decision,
            denial_reason,
            grant_id,
            grant_uri,
            grant_kind,
            capability,
            delegation_depth_remaining,
        ) = match &record.decision {
            AdmissionDecision::Admitted {
                grant_id,
                grant_uri,
                grant_kind,
                capability,
                delegation_depth_remaining,
                ..
            } => (
                "admitted",
                None::<String>,
                Some(grant_id.to_string()),
                Some(grant_uri.as_str().to_string()),
                Some(grant_kind.as_str().to_string()),
                Some(capability.as_str().to_string()),
                Some(i64::from(*delegation_depth_remaining)),
            ),
            AdmissionDecision::Denied { reason, .. } => (
                "denied",
                Some(reason.as_str().to_string()),
                None,
                None,
                None,
                None,
                None,
            ),
        };

        connection
            .execute(
                "INSERT INTO interoperability_command_admission_decisions (
                    decision_id, principal_uri, actor_uri, idempotency_key, correlation_id,
                    request_id, resource_uri, subject_uri, operation, required_grant_kind,
                    decision, denial_reason, grant_id, grant_uri, grant_kind, capability,
                    delegation_depth_remaining, admission_digest, decided_at, created_at
                 ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
                    ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20
                 )",
                params![
                    record.decision_id.to_string(),
                    record.principal.as_str(),
                    record.actor.as_str(),
                    record.idempotency_key.as_str(),
                    record.correlation_id.to_string(),
                    record.request_id.to_string(),
                    record.resource.as_str(),
                    record.subject.as_str(),
                    record.operation.as_str(),
                    record.required_grant_kind.as_str(),
                    decision,
                    denial_reason,
                    grant_id,
                    grant_uri,
                    grant_kind,
                    capability,
                    delegation_depth_remaining,
                    record.admission_digest,
                    record.decided_at.to_rfc3339(),
                    record.created_at.to_rfc3339(),
                ],
            )
            .map_err(|err| MvError::Storage(format!("insert admission decision: {err}")))?;
        Ok(())
    }

    fn load_command_admission_decision(
        connection: &Connection,
        principal: &StableUri,
        idempotency_key: &IdempotencyKey,
    ) -> MvResult<Option<CommandAdmissionDecisionRecord>> {
        let mut statement = connection
            .prepare(
                "SELECT decision_id, principal_uri, actor_uri, idempotency_key, correlation_id,
                        request_id, resource_uri, subject_uri, operation, required_grant_kind,
                        decision, denial_reason, grant_id, grant_uri, grant_kind, capability,
                        delegation_depth_remaining, admission_digest, decided_at, created_at
                 FROM interoperability_command_admission_decisions
                 WHERE principal_uri = ?1 AND idempotency_key = ?2",
            )
            .map_err(|err| MvError::Storage(format!("prepare admission-decision lookup: {err}")))?;
        let mut rows = statement
            .query(params![principal.as_str(), idempotency_key.as_str()])
            .map_err(|err| MvError::Storage(format!("query admission decision: {err}")))?;
        let Some(row) = rows
            .next()
            .map_err(|err| MvError::Storage(format!("read admission decision: {err}")))?
        else {
            return Ok(None);
        };
        Ok(Some(Self::row_to_command_admission_decision(row)?))
    }

    fn row_to_command_admission_decision(
        row: &rusqlite::Row<'_>,
    ) -> MvResult<CommandAdmissionDecisionRecord> {
        let decision_id = Uuid::parse_str(
            &row.get::<_, String>(0)
                .map_err(|err| MvError::Storage(format!("decision_id: {err}")))?,
        )
        .map_err(|err| MvError::Storage(format!("invalid decision_id: {err}")))?;
        let principal = StableUri::parse(
            row.get::<_, String>(1)
                .map_err(|err| MvError::Storage(format!("principal_uri: {err}")))?,
        )
        .map_err(MvError::InvalidInput)?;
        let actor = StableUri::parse(
            row.get::<_, String>(2)
                .map_err(|err| MvError::Storage(format!("actor_uri: {err}")))?,
        )
        .map_err(MvError::InvalidInput)?;
        let idempotency_key = IdempotencyKey::parse(
            row.get::<_, String>(3)
                .map_err(|err| MvError::Storage(format!("idempotency_key: {err}")))?,
        )
        .map_err(MvError::InvalidInput)?;
        let correlation_id = Uuid::parse_str(
            &row.get::<_, String>(4)
                .map_err(|err| MvError::Storage(format!("correlation_id: {err}")))?,
        )
        .map_err(|err| MvError::Storage(format!("invalid correlation_id: {err}")))?;
        let request_id = Uuid::parse_str(
            &row.get::<_, String>(5)
                .map_err(|err| MvError::Storage(format!("request_id: {err}")))?,
        )
        .map_err(|err| MvError::Storage(format!("invalid request_id: {err}")))?;
        let resource = StableUri::parse(
            row.get::<_, String>(6)
                .map_err(|err| MvError::Storage(format!("resource_uri: {err}")))?,
        )
        .map_err(MvError::InvalidInput)?;
        let subject = StableUri::parse(
            row.get::<_, String>(7)
                .map_err(|err| MvError::Storage(format!("subject_uri: {err}")))?,
        )
        .map_err(MvError::InvalidInput)?;
        let operation = ContextCapability::from_str(
            &row.get::<_, String>(8)
                .map_err(|err| MvError::Storage(format!("operation: {err}")))?,
        )
        .map_err(MvError::InvalidInput)?;
        let required_grant_kind = AuthorityGrantKind::from_str(
            &row.get::<_, String>(9)
                .map_err(|err| MvError::Storage(format!("required_grant_kind: {err}")))?,
        )
        .map_err(MvError::InvalidInput)?;
        let decision_token: String = row
            .get(10)
            .map_err(|err| MvError::Storage(format!("decision: {err}")))?;
        let denial_reason: Option<String> = row
            .get(11)
            .map_err(|err| MvError::Storage(format!("denial_reason: {err}")))?;
        let grant_id: Option<String> = row
            .get(12)
            .map_err(|err| MvError::Storage(format!("grant_id: {err}")))?;
        let grant_uri: Option<String> = row
            .get(13)
            .map_err(|err| MvError::Storage(format!("grant_uri: {err}")))?;
        let grant_kind: Option<String> = row
            .get(14)
            .map_err(|err| MvError::Storage(format!("grant_kind: {err}")))?;
        let capability: Option<String> = row
            .get(15)
            .map_err(|err| MvError::Storage(format!("capability: {err}")))?;
        let delegation_depth_remaining: Option<i64> = row
            .get(16)
            .map_err(|err| MvError::Storage(format!("delegation_depth_remaining: {err}")))?;
        let admission_digest: String = row
            .get(17)
            .map_err(|err| MvError::Storage(format!("admission_digest: {err}")))?;
        let decided_at = chrono::DateTime::parse_from_rfc3339(
            &row.get::<_, String>(18)
                .map_err(|err| MvError::Storage(format!("decided_at: {err}")))?,
        )
        .map_err(|err| MvError::Storage(format!("invalid decided_at: {err}")))?
        .with_timezone(&Utc);
        let created_at = chrono::DateTime::parse_from_rfc3339(
            &row.get::<_, String>(19)
                .map_err(|err| MvError::Storage(format!("created_at: {err}")))?,
        )
        .map_err(|err| MvError::Storage(format!("invalid created_at: {err}")))?
        .with_timezone(&Utc);

        let decision = match decision_token.as_str() {
            "admitted" => AdmissionDecision::Admitted {
                grant_id: Uuid::parse_str(
                    grant_id
                        .as_deref()
                        .ok_or_else(|| MvError::Storage("admitted row missing grant_id".into()))?,
                )
                .map_err(|err| MvError::Storage(format!("invalid grant_id: {err}")))?,
                grant_uri: StableUri::parse(
                    grant_uri
                        .ok_or_else(|| MvError::Storage("admitted row missing grant_uri".into()))?,
                )
                .map_err(MvError::InvalidInput)?,
                grant_kind: AuthorityGrantKind::from_str(
                    grant_kind.as_deref().ok_or_else(|| {
                        MvError::Storage("admitted row missing grant_kind".into())
                    })?,
                )
                .map_err(MvError::InvalidInput)?,
                capability: ContextCapability::from_str(
                    capability.as_deref().ok_or_else(|| {
                        MvError::Storage("admitted row missing capability".into())
                    })?,
                )
                .map_err(MvError::InvalidInput)?,
                delegation_depth_remaining: u8::try_from(delegation_depth_remaining.ok_or_else(
                    || MvError::Storage("admitted row missing delegation_depth_remaining".into()),
                )?)
                .map_err(|err| MvError::Storage(format!("delegation_depth_remaining: {err}")))?,
                decided_at,
            },
            "denied" => {
                AdmissionDecision::Denied {
                    reason: AdmissionDenialReason::from_str(denial_reason.as_deref().ok_or_else(
                        || MvError::Storage("denied row missing denial_reason".into()),
                    )?)
                    .map_err(MvError::InvalidInput)?,
                    decided_at,
                }
            }
            other => {
                return Err(MvError::Storage(format!(
                    "unknown admission decision token: {other}"
                )))
            }
        };

        Ok(CommandAdmissionDecisionRecord {
            decision_id,
            principal,
            actor,
            idempotency_key,
            correlation_id,
            request_id,
            resource,
            subject,
            operation,
            required_grant_kind,
            decision,
            admission_digest,
            decided_at,
            created_at,
        })
    }

    fn insert_authority_grant(
        &self,
        transaction: &rusqlite::Transaction<'_>,
        grant: &AuthorityGrant,
    ) -> MvResult<()> {
        let (payload, payload_format, wrapped_dek) =
            self.encode_governance_record(grant, "authority grant")?;
        transaction
            .execute(
                "INSERT INTO interoperability_authority_grants
                 (grant_id, revision, grant_uri, grant_kind, grantor_uri, grantee_uri,
                  governing_node_uri, status, parent_grant_id, not_before, expires_at,
                  record_payload, payload_format, payload_wrapped_dek, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
                params![
                    grant.grant_id.to_string(),
                    grant.revision,
                    grant.grant_uri.as_str(),
                    grant.kind.as_str(),
                    grant.grantor.as_str(),
                    grant.grantee.as_str(),
                    grant.governing_node.as_str(),
                    grant.status.as_str(),
                    grant.parent_grant_id.map(|id| id.to_string()),
                    grant.not_before.to_rfc3339(),
                    grant.expires_at.to_rfc3339(),
                    payload,
                    payload_format,
                    wrapped_dek,
                    grant.created_at.to_rfc3339(),
                    grant.updated_at.to_rfc3339(),
                ],
            )
            .map_err(|err| MvError::Storage(format!("insert authority grant: {err}")))?;
        Ok(())
    }

    fn update_authority_grant(
        &self,
        transaction: &rusqlite::Transaction<'_>,
        expected_revision: u64,
        replacement: &AuthorityGrant,
    ) -> MvResult<()> {
        let (payload, payload_format, wrapped_dek) =
            self.encode_governance_record(replacement, "authority grant")?;
        let changed = transaction
            .execute(
                "UPDATE interoperability_authority_grants
                 SET revision = ?3, grant_uri = ?4, grant_kind = ?5, grantor_uri = ?6,
                     grantee_uri = ?7, governing_node_uri = ?8, status = ?9,
                     parent_grant_id = ?10, not_before = ?11, expires_at = ?12,
                     record_payload = ?13, payload_format = ?14, payload_wrapped_dek = ?15,
                     created_at = ?16, updated_at = ?17
                 WHERE grant_id = ?1 AND revision = ?2",
                params![
                    replacement.grant_id.to_string(),
                    expected_revision,
                    replacement.revision,
                    replacement.grant_uri.as_str(),
                    replacement.kind.as_str(),
                    replacement.grantor.as_str(),
                    replacement.grantee.as_str(),
                    replacement.governing_node.as_str(),
                    replacement.status.as_str(),
                    replacement.parent_grant_id.map(|id| id.to_string()),
                    replacement.not_before.to_rfc3339(),
                    replacement.expires_at.to_rfc3339(),
                    payload,
                    payload_format,
                    wrapped_dek,
                    replacement.created_at.to_rfc3339(),
                    replacement.updated_at.to_rfc3339(),
                ],
            )
            .map_err(|err| MvError::Storage(format!("update authority grant: {err}")))?;
        if changed != 1 {
            return Err(MvError::InvalidInput(
                "authority-grant expected revision does not match current state".into(),
            ));
        }
        Ok(())
    }

    fn authority_grant_chain_is_effective(
        &self,
        connection: &Connection,
        grant: &AuthorityGrant,
        at: chrono::DateTime<Utc>,
    ) -> MvResult<bool> {
        let mut current = grant.clone();
        let mut seen = std::collections::HashSet::new();
        loop {
            if !current.is_effective_at(at) || !seen.insert(current.grant_id) {
                return Ok(false);
            }
            let Some(parent_id) = current.parent_grant_id else {
                return Ok(true);
            };
            if seen.len() > 17 {
                return Ok(false);
            }
            let Some(parent) = self.load_authority_grant_from_connection(connection, parent_id)?
            else {
                return Ok(false);
            };
            if !current.is_delegation_subset_of(&parent) {
                return Ok(false);
            }
            current = parent;
        }
    }

    fn row_to_source_binding(&self, row: &rusqlite::Row<'_>) -> rusqlite::Result<SourceBinding> {
        let binding_id: String = row.get(0)?;
        let revision: u64 = row.get(1)?;
        let resource_uri: String = row.get(2)?;
        let context_node_uri: String = row.get(3)?;
        let external_system: String = row.get(4)?;
        let external_account_key: String = row.get(5)?;
        let external_object_key: String = row.get(6)?;
        let status: String = row.get(7)?;
        let supersedes_binding_id: Option<String> = row.get(8)?;
        let payload: Vec<u8> = row.get(9)?;
        let payload_format: String = row.get(10)?;
        let wrapped_dek: Option<String> = row.get(11)?;
        let created_at: String = row.get(12)?;
        let updated_at: String = row.get(13)?;

        let binding: SourceBinding = self
            .decode_governance_record(
                &payload,
                &payload_format,
                wrapped_dek.as_deref(),
                "source binding",
            )
            .map_err(|err| Self::as_sql_conversion_error(9, err.to_string()))?;
        binding
            .validate()
            .map_err(|err| Self::as_sql_conversion_error(9, err))?;

        let stored_supersedes = supersedes_binding_id
            .map(|value| parse_uuid_str(8, &value))
            .transpose()?;
        if binding.binding_id != parse_uuid_str(0, &binding_id)?
            || binding.revision != revision
            || binding.resource_uri.as_str() != resource_uri
            || binding.context_node.as_str() != context_node_uri
            || binding.external_system != external_system
            || self
                .source_lookup_key(&binding.external_account_id)
                .map_err(|err| Self::as_sql_conversion_error(5, err.to_string()))?
                != external_account_key
            || self
                .source_lookup_key(&binding.external_object_id)
                .map_err(|err| Self::as_sql_conversion_error(6, err.to_string()))?
                != external_object_key
            || binding.status.as_str() != status
            || binding.supersedes_binding_id != stored_supersedes
            || binding.created_at != parse_dt_strict(12, &created_at)?
            || binding.updated_at != parse_dt_strict(13, &updated_at)?
        {
            return Err(Self::as_sql_conversion_error(
                9,
                "source binding payload does not match its governed index",
            ));
        }
        Ok(binding)
    }

    fn load_source_binding_from_connection(
        &self,
        connection: &Connection,
        binding_id: Uuid,
    ) -> MvResult<Option<SourceBinding>> {
        connection
            .query_row(
                "SELECT binding_id, revision, resource_uri, context_node_uri,
                        external_system, external_account_key, external_object_key,
                        status, supersedes_binding_id, record_payload, payload_format,
                        payload_wrapped_dek, created_at, updated_at
                 FROM interoperability_source_bindings
                 WHERE binding_id = ?1",
                params![binding_id.to_string()],
                |row| self.row_to_source_binding(row),
            )
            .optional()
            .map_err(|err| MvError::Storage(format!("load source binding: {err}")))
    }

    fn insert_source_binding(
        &self,
        transaction: &rusqlite::Transaction<'_>,
        binding: &SourceBinding,
    ) -> MvResult<()> {
        let (payload, payload_format, wrapped_dek) =
            self.encode_governance_record(binding, "source binding")?;
        transaction
            .execute(
                "INSERT INTO interoperability_source_bindings
                 (binding_id, revision, resource_uri, context_node_uri, external_system,
                  external_account_key, external_object_key, status, supersedes_binding_id,
                  record_payload, payload_format, payload_wrapped_dek, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                params![
                    binding.binding_id.to_string(),
                    binding.revision,
                    binding.resource_uri.as_str(),
                    binding.context_node.as_str(),
                    &binding.external_system,
                    self.source_lookup_key(&binding.external_account_id)?,
                    self.source_lookup_key(&binding.external_object_id)?,
                    binding.status.as_str(),
                    binding.supersedes_binding_id.map(|id| id.to_string()),
                    payload,
                    payload_format,
                    wrapped_dek,
                    binding.created_at.to_rfc3339(),
                    binding.updated_at.to_rfc3339(),
                ],
            )
            .map_err(|err| MvError::Storage(format!("insert source binding: {err}")))?;
        Ok(())
    }

    fn row_to_action_receipt(&self, row: &rusqlite::Row<'_>) -> rusqlite::Result<ActionReceipt> {
        let receipt_id: String = row.get(0)?;
        let receipt_version: String = row.get(1)?;
        let event_id: String = row.get(2)?;
        let claim_id: String = row.get(3)?;
        let attempt: u32 = row.get(4)?;
        let outcome: String = row.get(5)?;
        let executor: String = row.get(6)?;
        let destination: String = row.get(7)?;
        let subject: String = row.get(8)?;
        let principal: String = row.get(9)?;
        let actor: String = row.get(10)?;
        let correlation_id: String = row.get(11)?;
        let request_digest: String = row.get(12)?;
        let started_at: String = row.get(13)?;
        let completed_at: String = row.get(14)?;
        let sensitivity: String = row.get(15)?;
        let retention: String = row.get(16)?;
        let payload: Vec<u8> = row.get(17)?;
        let payload_format: String = row.get(18)?;
        let wrapped_dek: Option<String> = row.get(19)?;

        let receipt: ActionReceipt = self
            .decode_governance_record(
                &payload,
                &payload_format,
                wrapped_dek.as_deref(),
                "action receipt",
            )
            .map_err(|err| Self::as_sql_conversion_error(17, err.to_string()))?;
        receipt
            .validate()
            .map_err(|err| Self::as_sql_conversion_error(17, err))?;
        if receipt.receipt_id != parse_uuid_str(0, &receipt_id)?
            || receipt.receipt_version != receipt_version
            || receipt.event_id != parse_uuid_str(2, &event_id)?
            || receipt.claim_id != parse_uuid_str(3, &claim_id)?
            || receipt.attempt != attempt
            || receipt.outcome.as_str() != outcome
            || receipt.executor.as_str() != executor
            || receipt.destination.as_str() != destination
            || receipt.subject.as_str() != subject
            || receipt.principal.as_str() != principal
            || receipt.actor.as_str() != actor
            || receipt.correlation_id != parse_uuid_str(11, &correlation_id)?
            || receipt.request_digest != request_digest
            || receipt.started_at != parse_dt_strict(13, &started_at)?
            || receipt.completed_at != parse_dt_strict(14, &completed_at)?
            || receipt.sensitivity.as_str() != sensitivity
            || receipt.retention.as_str() != retention
        {
            return Err(Self::as_sql_conversion_error(
                17,
                "action receipt payload does not match its governed index",
            ));
        }
        Ok(receipt)
    }

    fn load_action_receipt_by_attempt(
        &self,
        connection: &Connection,
        event_id: Uuid,
        attempt: u32,
    ) -> MvResult<Option<ActionReceipt>> {
        connection
            .query_row(
                "SELECT receipt_id, receipt_version, event_id, claim_id, attempt_no, outcome,
                        executor_uri, destination_uri, subject_uri, principal_uri,
                        actor_uri, correlation_id, request_digest, started_at,
                        completed_at, sensitivity, retention, payload, payload_format,
                        payload_wrapped_dek
                 FROM interoperability_action_receipts
                 WHERE event_id = ?1 AND attempt_no = ?2",
                params![event_id.to_string(), attempt],
                |row| self.row_to_action_receipt(row),
            )
            .optional()
            .map_err(|err| MvError::Storage(format!("load action receipt attempt: {err}")))
    }

    fn row_to_consumer_inbox_admission(
        &self,
        row: &rusqlite::Row<'_>,
    ) -> rusqlite::Result<ConsumerInboxAdmission> {
        let inbox_sequence: u64 = row.get(0)?;
        let consumer_uri: String = row.get(1)?;
        let event_id: String = row.get(2)?;
        let event_digest: String = row.get(3)?;
        let source_uri: String = row.get(4)?;
        let subject_uri: String = row.get(5)?;
        let principal_uri: String = row.get(6)?;
        let actor_uri: String = row.get(7)?;
        let event_type: String = row.get(8)?;
        let schema_uri: String = row.get(9)?;
        let schema_version: String = row.get(10)?;
        let correlation_id: String = row.get(11)?;
        let occurred_at: String = row.get(12)?;
        let sensitivity: String = row.get(13)?;
        let retention: String = row.get(14)?;
        let payload: Vec<u8> = row.get(15)?;
        let payload_format: String = row.get(16)?;
        let wrapped_dek: Option<String> = row.get(17)?;
        let received_at: String = row.get(18)?;

        let event: EventEnvelope = self
            .decode_governance_record(
                &payload,
                &payload_format,
                wrapped_dek.as_deref(),
                "consumer inbox event",
            )
            .map_err(|err| Self::as_sql_conversion_error(15, err.to_string()))?;
        event
            .validate()
            .map_err(|err| Self::as_sql_conversion_error(15, err))?;
        let consumer = StableUri::parse(consumer_uri.clone())
            .map_err(|err| Self::as_sql_conversion_error(1, err))?;
        if event.id != parse_uuid_str(2, &event_id)?
            || event.content_digest() != event_digest
            || event.source.as_str() != source_uri
            || event.subject.as_str() != subject_uri
            || event.principal.as_str() != principal_uri
            || event.actor.as_str() != actor_uri
            || event.event_type != event_type
            || event.schema.uri.as_str() != schema_uri
            || event.schema.version != schema_version
            || event.correlation_id != parse_uuid_str(11, &correlation_id)?
            || event.occurred_at != parse_dt_strict(12, &occurred_at)?
            || event.sensitivity.as_str() != sensitivity
            || event.retention.as_str() != retention
        {
            return Err(Self::as_sql_conversion_error(
                15,
                "consumer inbox payload does not match its governed index",
            ));
        }
        Ok(ConsumerInboxAdmission {
            inbox_sequence,
            consumer,
            event,
            received_at: parse_dt_strict(18, &received_at)?,
            replayed: false,
        })
    }

    fn load_consumer_inbox_admission(
        &self,
        connection: &Connection,
        consumer: &StableUri,
        event_id: Uuid,
    ) -> MvResult<Option<ConsumerInboxAdmission>> {
        connection
            .query_row(
                "SELECT inbox_sequence, consumer_uri, event_id, event_digest, source_uri,
                        subject_uri, principal_uri, actor_uri, event_type, schema_uri,
                        schema_version, correlation_id, occurred_at, sensitivity, retention,
                        envelope_payload, payload_format, payload_wrapped_dek, received_at
                 FROM interoperability_consumer_inbox
                 WHERE consumer_uri = ?1 AND event_id = ?2",
                params![consumer.as_str(), event_id.to_string()],
                |row| self.row_to_consumer_inbox_admission(row),
            )
            .optional()
            .map_err(|err| MvError::Storage(format!("load consumer inbox event: {err}")))
    }

    fn load_consumer_inbox_admission_by_sequence(
        &self,
        connection: &Connection,
        inbox_sequence: u64,
    ) -> MvResult<Option<ConsumerInboxAdmission>> {
        connection
            .query_row(
                "SELECT inbox_sequence, consumer_uri, event_id, event_digest, source_uri,
                        subject_uri, principal_uri, actor_uri, event_type, schema_uri,
                        schema_version, correlation_id, occurred_at, sensitivity, retention,
                        envelope_payload, payload_format, payload_wrapped_dek, received_at
                 FROM interoperability_consumer_inbox
                 WHERE inbox_sequence = ?1",
                params![inbox_sequence],
                |row| self.row_to_consumer_inbox_admission(row),
            )
            .optional()
            .map_err(|err| MvError::Storage(format!("load consumer inbox sequence: {err}")))
    }

    fn row_to_consumer_application_receipt(
        &self,
        row: &rusqlite::Row<'_>,
    ) -> rusqlite::Result<ConsumerApplicationReceipt> {
        let receipt_id: String = row.get(0)?;
        let receipt_version: String = row.get(1)?;
        let inbox_sequence: u64 = row.get(2)?;
        let event_id: String = row.get(3)?;
        let claim_id: String = row.get(4)?;
        let attempt: u32 = row.get(5)?;
        let outcome: String = row.get(6)?;
        let consumer_uri: String = row.get(7)?;
        let processor_uri: String = row.get(8)?;
        let source_uri: String = row.get(9)?;
        let subject_uri: String = row.get(10)?;
        let principal_uri: String = row.get(11)?;
        let actor_uri: String = row.get(12)?;
        let correlation_id: String = row.get(13)?;
        let request_digest: String = row.get(14)?;
        let started_at: String = row.get(15)?;
        let completed_at: String = row.get(16)?;
        let sensitivity: String = row.get(17)?;
        let retention: String = row.get(18)?;
        let payload: Vec<u8> = row.get(19)?;
        let payload_format: String = row.get(20)?;
        let wrapped_dek: Option<String> = row.get(21)?;

        let receipt: ConsumerApplicationReceipt = self
            .decode_governance_record(
                &payload,
                &payload_format,
                wrapped_dek.as_deref(),
                "consumer application receipt",
            )
            .map_err(|err| Self::as_sql_conversion_error(19, err.to_string()))?;
        receipt
            .validate()
            .map_err(|err| Self::as_sql_conversion_error(19, err))?;
        if receipt.receipt_id != parse_uuid_str(0, &receipt_id)?
            || receipt.receipt_version != receipt_version
            || receipt.inbox_sequence != inbox_sequence
            || receipt.event_id != parse_uuid_str(3, &event_id)?
            || receipt.claim_id != parse_uuid_str(4, &claim_id)?
            || receipt.attempt != attempt
            || receipt.outcome.as_str() != outcome
            || receipt.consumer.as_str() != consumer_uri
            || receipt.processor.as_str() != processor_uri
            || receipt.source.as_str() != source_uri
            || receipt.subject.as_str() != subject_uri
            || receipt.principal.as_str() != principal_uri
            || receipt.actor.as_str() != actor_uri
            || receipt.correlation_id != parse_uuid_str(13, &correlation_id)?
            || receipt.request_digest != request_digest
            || receipt.started_at != parse_dt_strict(15, &started_at)?
            || receipt.completed_at != parse_dt_strict(16, &completed_at)?
            || receipt.sensitivity.as_str() != sensitivity
            || receipt.retention.as_str() != retention
        {
            return Err(Self::as_sql_conversion_error(
                19,
                "consumer receipt payload does not match its governed index",
            ));
        }
        Ok(receipt)
    }

    fn load_consumer_application_receipt_by_attempt(
        &self,
        connection: &Connection,
        inbox_sequence: u64,
        attempt: u32,
    ) -> MvResult<Option<ConsumerApplicationReceipt>> {
        connection
            .query_row(
                "SELECT receipt_id, receipt_version, inbox_sequence, event_id, claim_id, attempt_no,
                        outcome, consumer_uri, processor_uri, source_uri, subject_uri,
                        principal_uri, actor_uri, correlation_id, request_digest,
                        started_at, completed_at, sensitivity, retention, payload,
                        payload_format, payload_wrapped_dek
                 FROM interoperability_consumer_application_receipts
                 WHERE inbox_sequence = ?1 AND attempt_no = ?2",
                params![inbox_sequence, attempt],
                |row| self.row_to_consumer_application_receipt(row),
            )
            .optional()
            .map_err(|err| {
                MvError::Storage(format!("load consumer application receipt attempt: {err}"))
            })
    }

    fn row_to_consumer_checkpoint(row: &rusqlite::Row<'_>) -> rusqlite::Result<ConsumerCheckpoint> {
        let consumer_uri: String = row.get(0)?;
        let source_uri: String = row.get(1)?;
        let last_dispositioned_event_id: String = row.get(3)?;
        let last_applied_event_id: Option<String> = row.get(5)?;
        let updated_at: String = row.get(8)?;
        let checkpoint = ConsumerCheckpoint {
            consumer: StableUri::parse(consumer_uri)
                .map_err(|err| Self::as_sql_conversion_error(0, err))?,
            source: StableUri::parse(source_uri)
                .map_err(|err| Self::as_sql_conversion_error(1, err))?,
            last_dispositioned_sequence: row.get(2)?,
            last_dispositioned_event_id: parse_uuid_str(3, &last_dispositioned_event_id)?,
            last_applied_sequence: row.get(4)?,
            last_applied_event_id: last_applied_event_id
                .map(|event_id| parse_uuid_str(5, &event_id))
                .transpose()?,
            applied_count: row.get(6)?,
            dead_letter_count: row.get(7)?,
            updated_at: parse_dt_strict(8, &updated_at)?,
        };
        checkpoint
            .validate()
            .map_err(|err| Self::as_sql_conversion_error(2, err))?;
        Ok(checkpoint)
    }

    fn load_consumer_checkpoint(
        connection: &Connection,
        consumer: &StableUri,
        source: &StableUri,
    ) -> MvResult<Option<ConsumerCheckpoint>> {
        connection
            .query_row(
                "SELECT consumer_uri, source_uri, last_dispositioned_sequence,
                        last_dispositioned_event_id, last_applied_sequence,
                        last_applied_event_id, applied_count, dead_letter_count, updated_at
                 FROM interoperability_consumer_checkpoints
                 WHERE consumer_uri = ?1 AND source_uri = ?2",
                params![consumer.as_str(), source.as_str()],
                Self::row_to_consumer_checkpoint,
            )
            .optional()
            .map_err(|err| MvError::Storage(format!("load consumer checkpoint: {err}")))
    }
}

#[async_trait]
impl InteroperabilityStore for SqliteNodeStore {
    async fn local_context_node_id(&self) -> MvResult<Uuid> {
        let mut conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| MvError::Storage(format!("begin local identity transaction: {e}")))?;
        let generated = Uuid::now_v7();
        tx.execute(
            "INSERT OR IGNORE INTO interoperability_local_identity
             (singleton, node_id, created_at) VALUES (1, ?1, ?2)",
            params![generated.to_string(), Utc::now().to_rfc3339()],
        )
        .map_err(|e| MvError::Storage(format!("persist local context identity: {e}")))?;
        let node_id: String = tx
            .query_row(
                "SELECT node_id FROM interoperability_local_identity WHERE singleton = 1",
                [],
                |row| row.get(0),
            )
            .map_err(|e| MvError::Storage(format!("load local context identity: {e}")))?;
        let node_id = Uuid::parse_str(&node_id)
            .map_err(|e| MvError::Storage(format!("invalid local context identity: {e}")))?;
        tx.commit()
            .map_err(|e| MvError::Storage(format!("commit local identity transaction: {e}")))?;
        Ok(node_id)
    }

    async fn commit_context_node_with_event(
        &self,
        context_node: &ContextNodeRecord,
        event: &EventEnvelope,
    ) -> MvResult<IdempotentContextNodeCommit> {
        context_node.validate().map_err(MvError::InvalidInput)?;
        if context_node.revision != 1 {
            return Err(MvError::InvalidInput(
                "new context-node descriptors must start at revision one".into(),
            ));
        }
        Self::validate_context_node_event_record(event, context_node, false, true)?;
        if event.data.get("node_type").and_then(|value| value.as_str())
            != Some(context_node.node_type.as_str())
            || event.data.get("status").and_then(|value| value.as_str())
                != Some(context_node.status.as_str())
        {
            return Err(MvError::InvalidInput(
                "context-node registration event must identify its type and initial status".into(),
            ));
        }

        let mut connection = self
            .conn()
            .lock()
            .map_err(|err| MvError::Storage(err.to_string()))?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|err| MvError::Storage(format!("begin context-node registration: {err}")))?;
        let local_node_id = Self::ensure_local_context_node(&transaction)?;
        Self::validate_governance_event(
            &transaction,
            event,
            local_node_id,
            CONTEXT_NODE_REGISTERED_V1,
            "context-node-registered",
        )?;

        if let Some(existing_event) =
            Self::resolve_governance_replay(&transaction, event, CONTEXT_NODE_REGISTERED_V1)?
        {
            let existing_node_id = event_context_node_id(&existing_event)?;
            let existing_revision = event_context_node_revision(&existing_event, 1)?;
            let existing_context_node = self
                .load_context_node_revision_from_connection(
                    &transaction,
                    existing_node_id,
                    existing_revision,
                )?
                .ok_or_else(|| {
                    MvError::Storage(
                        "context-node replay references a missing descriptor revision".into(),
                    )
                })?;
            transaction
                .commit()
                .map_err(|err| MvError::Storage(format!("finish context-node replay: {err}")))?;
            return Ok(IdempotentContextNodeCommit {
                context_node: existing_context_node,
                event: existing_event,
                replayed: true,
            });
        }

        let is_discovered = context_node.status == ContextNodeStatus::Discovered
            && context_node.trust_class == ContextNodeTrustClass::untrusted();
        let is_local_bootstrap = context_node.node_id == local_node_id
            && context_node.node_uri == StableUri::node(local_node_id)
            && context_node.governing_node_id == context_node.node_uri
            && context_node.status == ContextNodeStatus::Active
            && context_node.trust_class == ContextNodeTrustClass::local();
        if !is_discovered && !is_local_bootstrap {
            return Err(MvError::InvalidInput(
                "new context nodes must be untrusted discoveries or the active self-governed local node"
                    .into(),
            ));
        }

        Self::require_active_schema_references(&transaction, &context_node.capability_manifest)?;
        self.insert_context_node(&transaction, context_node)?;
        Self::insert_outbox_event(&transaction, event)?;
        transaction
            .commit()
            .map_err(|err| MvError::Storage(format!("commit context-node registration: {err}")))?;
        Ok(IdempotentContextNodeCommit {
            context_node: context_node.clone(),
            event: event.clone(),
            replayed: false,
        })
    }

    async fn commit_identity_with_event(
        &self,
        identity: &IdentityRecord,
        event: &EventEnvelope,
    ) -> MvResult<IdempotentIdentityCommit> {
        identity.validate().map_err(MvError::InvalidInput)?;
        if identity.revision != 1 {
            return Err(MvError::InvalidInput(
                "new identity records must start at revision one".into(),
            ));
        }
        Self::validate_identity_event_record(event, identity, false, true)?;
        if event.data.get("actor_kind").and_then(|value| value.as_str())
            != Some(identity.actor_kind.as_str())
            || event.data.get("status").and_then(|value| value.as_str())
                != Some(identity.status.as_str())
            || event
                .data
                .get("subject_binding_digest")
                .and_then(|value| value.as_str())
                != Some(identity.subject_binding_digest.as_str())
        {
            return Err(MvError::InvalidInput(
                "identity registration event must identify actor kind, status, and subject binding digest".into(),
            ));
        }

        let mut connection = self
            .conn()
            .lock()
            .map_err(|err| MvError::Storage(err.to_string()))?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|err| MvError::Storage(format!("begin identity registration: {err}")))?;
        let local_node_id = Self::ensure_local_context_node(&transaction)?;
        Self::validate_governance_event(
            &transaction,
            event,
            local_node_id,
            IDENTITY_REGISTERED_V1,
            "identity-registered",
        )?;

        if let Some(existing_event) =
            Self::resolve_governance_replay(&transaction, event, IDENTITY_REGISTERED_V1)?
        {
            let existing_principal_id = event_identity_principal_id(&existing_event)?;
            let existing_identity = self
                .load_identity_from_connection(&transaction, existing_principal_id)?
                .ok_or_else(|| {
                    MvError::Storage(
                        "identity replay references a missing identity revision".into(),
                    )
                })?;
            transaction
                .commit()
                .map_err(|err| MvError::Storage(format!("finish identity replay: {err}")))?;
            return Ok(IdempotentIdentityCommit {
                identity: existing_identity,
                event: existing_event,
                replayed: true,
            });
        }

        if let Some(existing) = self.load_identity_by_subject_from_connection(
            &transaction,
            &identity.governing_node_uri,
            &identity.subject_binding,
        )? {
            transaction
                .commit()
                .map_err(|err| MvError::Storage(format!("finish existing identity lookup: {err}")))?;
            return Ok(IdempotentIdentityCommit {
                identity: existing,
                event: event.clone(),
                replayed: true,
            });
        }

        self.insert_identity(&transaction, identity)?;
        Self::insert_outbox_event(&transaction, event)?;
        transaction
            .commit()
            .map_err(|err| MvError::Storage(format!("commit identity registration: {err}")))?;
        Ok(IdempotentIdentityCommit {
            identity: identity.clone(),
            event: event.clone(),
            replayed: false,
        })
    }

    async fn get_identity(&self, principal_id: Uuid) -> MvResult<Option<IdentityRecord>> {
        self.with_conn(|connection| self.load_identity_from_connection(connection, principal_id))
    }

    async fn get_identity_by_subject_binding(
        &self,
        governing_node_uri: &StableUri,
        subject_binding: &str,
    ) -> MvResult<Option<IdentityRecord>> {
        self.with_conn(|connection| {
            self.load_identity_by_subject_from_connection(
                connection,
                governing_node_uri,
                subject_binding,
            )
        })
    }

    async fn list_identities(
        &self,
        governing_node_uri: Option<&StableUri>,
    ) -> MvResult<Vec<IdentityRecord>> {
        self.with_conn(|connection| {
            let sql = if governing_node_uri.is_some() {
                "SELECT principal_id, revision, principal_uri, governing_node_uri, subject_binding,
                        subject_binding_digest, actor_kind, status, record_digest, record_payload,
                        payload_format, payload_wrapped_dek, created_at, updated_at
                 FROM interoperability_identity_records
                 WHERE governing_node_uri = ?1
                 ORDER BY subject_binding ASC, principal_id ASC"
            } else {
                "SELECT principal_id, revision, principal_uri, governing_node_uri, subject_binding,
                        subject_binding_digest, actor_kind, status, record_digest, record_payload,
                        payload_format, payload_wrapped_dek, created_at, updated_at
                 FROM interoperability_identity_records
                 ORDER BY governing_node_uri ASC, subject_binding ASC, principal_id ASC"
            };
            let mut statement = connection
                .prepare(sql)
                .map_err(|err| MvError::Storage(format!("prepare identity query: {err}")))?;
            let mut identities = Vec::new();
            if let Some(governing_node_uri) = governing_node_uri {
                let rows = statement
                    .query_map(params![governing_node_uri.as_str()], |row| {
                        self.row_to_identity(row)
                    })
                    .map_err(|err| MvError::Storage(format!("query identities: {err}")))?;
                for row in rows {
                    identities.push(row.map_err(|err| {
                        MvError::Storage(format!("read identity record: {err}"))
                    })?);
                }
            } else {
                let rows = statement
                    .query_map([], |row| self.row_to_identity(row))
                    .map_err(|err| MvError::Storage(format!("query identities: {err}")))?;
                for row in rows {
                    identities.push(row.map_err(|err| {
                        MvError::Storage(format!("read identity record: {err}"))
                    })?);
                }
            }
            Ok(identities)
        })
    }

    async fn get_context_node(&self, node_id: Uuid) -> MvResult<Option<ContextNodeRecord>> {
        self.with_conn(|connection| self.load_context_node_from_connection(connection, node_id))
    }

    async fn list_context_nodes(
        &self,
        status: Option<ContextNodeStatus>,
    ) -> MvResult<Vec<ContextNodeRecord>> {
        self.with_conn(|connection| {
            let sql = if status.is_some() {
                "SELECT node_id, revision, node_uri, node_type, owner_actor_uri,
                        governing_node_uri, trust_class, status, capability_digest,
                        record_payload, payload_format, payload_wrapped_dek, created_at, updated_at
                 FROM interoperability_context_nodes
                 WHERE status = ?1
                 ORDER BY updated_at ASC, node_id ASC"
            } else {
                "SELECT node_id, revision, node_uri, node_type, owner_actor_uri,
                        governing_node_uri, trust_class, status, capability_digest,
                        record_payload, payload_format, payload_wrapped_dek, created_at, updated_at
                 FROM interoperability_context_nodes
                 ORDER BY updated_at ASC, node_id ASC"
            };
            let mut statement = connection
                .prepare(sql)
                .map_err(|err| MvError::Storage(format!("prepare context-node query: {err}")))?;
            let mut nodes = Vec::new();
            if let Some(status) = status {
                let rows = statement
                    .query_map(params![status.as_str()], |row| {
                        self.row_to_context_node(row)
                    })
                    .map_err(|err| MvError::Storage(format!("query context nodes: {err}")))?;
                for row in rows {
                    nodes.push(row.map_err(|err| {
                        MvError::Storage(format!("read context-node descriptor: {err}"))
                    })?);
                }
            } else {
                let rows = statement
                    .query_map([], |row| self.row_to_context_node(row))
                    .map_err(|err| MvError::Storage(format!("query context nodes: {err}")))?;
                for row in rows {
                    nodes.push(row.map_err(|err| {
                        MvError::Storage(format!("read context-node descriptor: {err}"))
                    })?);
                }
            }
            Ok(nodes)
        })
    }

    async fn update_context_node_descriptor_with_event(
        &self,
        expected_revision: u64,
        replacement: &ContextNodeRecord,
        event: &EventEnvelope,
    ) -> MvResult<IdempotentContextNodeCommit> {
        replacement.validate().map_err(MvError::InvalidInput)?;
        let next_revision = expected_revision
            .checked_add(1)
            .ok_or_else(|| MvError::InvalidInput("context-node revision overflow".into()))?;
        if replacement.revision != next_revision {
            return Err(MvError::InvalidInput(
                "replacement context-node revision must advance exactly once".into(),
            ));
        }
        Self::validate_context_node_event_record(event, replacement, true, true)?;

        let mut connection = self
            .conn()
            .lock()
            .map_err(|err| MvError::Storage(err.to_string()))?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|err| MvError::Storage(format!("begin context-node update: {err}")))?;
        let local_node_id = Self::ensure_local_context_node(&transaction)?;
        Self::validate_governance_event(
            &transaction,
            event,
            local_node_id,
            CONTEXT_NODE_DESCRIPTOR_UPDATED_V1,
            "context-node-descriptor-updated",
        )?;

        if let Some(existing_event) = Self::resolve_governance_replay(
            &transaction,
            event,
            CONTEXT_NODE_DESCRIPTOR_UPDATED_V1,
        )? {
            let existing_node_id = event_context_node_id(&existing_event)?;
            let existing_revision = event_context_node_revision(&existing_event, 1)?;
            let existing_context_node = self
                .load_context_node_revision_from_connection(
                    &transaction,
                    existing_node_id,
                    existing_revision,
                )?
                .ok_or_else(|| {
                    MvError::Storage(
                        "context-node replay references a missing descriptor revision".into(),
                    )
                })?;
            transaction
                .commit()
                .map_err(|err| MvError::Storage(format!("finish context-node replay: {err}")))?;
            return Ok(IdempotentContextNodeCommit {
                context_node: existing_context_node,
                event: existing_event,
                replayed: true,
            });
        }

        let current = self
            .load_context_node_from_connection(&transaction, replacement.node_id)?
            .ok_or_else(|| MvError::InvalidInput("context node does not exist".into()))?;
        if current.revision != expected_revision
            || replacement.node_uri != current.node_uri
            || replacement.node_type != current.node_type
            || replacement.owner_actor_id != current.owner_actor_id
            || replacement.governing_node_id != current.governing_node_id
            || replacement.status != current.status
            || replacement.created_at != current.created_at
            || replacement.updated_at < current.updated_at
        {
            return Err(MvError::InvalidInput(
                "descriptor updates must preserve stable identity, governance, lifecycle, and creation time"
                    .into(),
            ));
        }
        if replacement.capability_manifest != current.capability_manifest {
            let next_manifest_revision = current
                .capability_manifest
                .revision
                .checked_add(1)
                .ok_or_else(|| MvError::InvalidInput("capability revision overflow".into()))?;
            if replacement.capability_manifest.revision != next_manifest_revision {
                return Err(MvError::InvalidInput(
                    "changed capability manifests must advance exactly one revision".into(),
                ));
            }
        }

        Self::require_active_schema_references(&transaction, &replacement.capability_manifest)?;
        self.update_context_node(&transaction, expected_revision, replacement)?;
        Self::insert_outbox_event(&transaction, event)?;
        transaction
            .commit()
            .map_err(|err| MvError::Storage(format!("commit context-node update: {err}")))?;
        Ok(IdempotentContextNodeCommit {
            context_node: replacement.clone(),
            event: event.clone(),
            replayed: false,
        })
    }

    async fn transition_context_node_with_event(
        &self,
        expected_revision: u64,
        replacement: &ContextNodeRecord,
        event: &EventEnvelope,
    ) -> MvResult<IdempotentContextNodeCommit> {
        replacement.validate().map_err(MvError::InvalidInput)?;
        let next_revision = expected_revision
            .checked_add(1)
            .ok_or_else(|| MvError::InvalidInput("context-node revision overflow".into()))?;
        if replacement.revision != next_revision {
            return Err(MvError::InvalidInput(
                "replacement context-node revision must advance exactly once".into(),
            ));
        }
        Self::validate_context_node_event_record(event, replacement, true, false)?;

        let mut connection = self
            .conn()
            .lock()
            .map_err(|err| MvError::Storage(err.to_string()))?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|err| MvError::Storage(format!("begin context-node transition: {err}")))?;
        let local_node_id = Self::ensure_local_context_node(&transaction)?;
        Self::validate_governance_event(
            &transaction,
            event,
            local_node_id,
            CONTEXT_NODE_LIFECYCLE_TRANSITIONED_V1,
            "context-node-lifecycle-transitioned",
        )?;

        if let Some(existing_event) = Self::resolve_governance_replay(
            &transaction,
            event,
            CONTEXT_NODE_LIFECYCLE_TRANSITIONED_V1,
        )? {
            let existing_node_id = event_context_node_id(&existing_event)?;
            let existing_revision = event_context_node_revision(&existing_event, 1)?;
            let existing_context_node = self
                .load_context_node_revision_from_connection(
                    &transaction,
                    existing_node_id,
                    existing_revision,
                )?
                .ok_or_else(|| {
                    MvError::Storage(
                        "context-node replay references a missing descriptor revision".into(),
                    )
                })?;
            transaction
                .commit()
                .map_err(|err| MvError::Storage(format!("finish context-node replay: {err}")))?;
            return Ok(IdempotentContextNodeCommit {
                context_node: existing_context_node,
                event: existing_event,
                replayed: true,
            });
        }

        let current = self
            .load_context_node_from_connection(&transaction, replacement.node_id)?
            .ok_or_else(|| MvError::InvalidInput("context node does not exist".into()))?;
        if current.revision != expected_revision
            || !current.status.can_transition_to(replacement.status)
            || event
                .data
                .get("from_status")
                .and_then(|value| value.as_str())
                != Some(current.status.as_str())
            || event.data.get("to_status").and_then(|value| value.as_str())
                != Some(replacement.status.as_str())
        {
            return Err(MvError::InvalidInput(
                "context-node lifecycle transition is not allowed or does not match its event"
                    .into(),
            ));
        }
        let mut expected = current.clone();
        expected.revision = replacement.revision;
        expected.status = replacement.status;
        expected.updated_at = replacement.updated_at;
        if expected != *replacement {
            return Err(MvError::InvalidInput(
                "lifecycle transitions may change only status, revision, and update time".into(),
            ));
        }

        self.update_context_node(&transaction, expected_revision, replacement)?;
        Self::insert_outbox_event(&transaction, event)?;
        transaction
            .commit()
            .map_err(|err| MvError::Storage(format!("commit context-node transition: {err}")))?;
        Ok(IdempotentContextNodeCommit {
            context_node: replacement.clone(),
            event: event.clone(),
            replayed: false,
        })
    }

    async fn commit_authority_grant_with_event(
        &self,
        grant: &AuthorityGrant,
        event: &EventEnvelope,
    ) -> MvResult<IdempotentAuthorityGrantCommit> {
        grant.validate().map_err(MvError::InvalidInput)?;
        if grant.revision != 1 || grant.status != AuthorityGrantStatus::Active {
            return Err(MvError::InvalidInput(
                "new authority grants must start active at revision one".into(),
            ));
        }
        Self::validate_authority_grant_event_record(event, grant, false, true)?;

        let mut connection = self
            .conn()
            .lock()
            .map_err(|err| MvError::Storage(err.to_string()))?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|err| MvError::Storage(format!("begin authority-grant issuance: {err}")))?;
        let local_node_id = Self::ensure_local_context_node(&transaction)?;
        Self::validate_governance_event(
            &transaction,
            event,
            local_node_id,
            AUTHORITY_GRANT_ISSUED_V1,
            "authority-grant-issued",
        )?;

        if event.principal != grant.grantor || event.actor != grant.grantor {
            return Err(MvError::InvalidInput(
                "authority-grant issuance principal and actor must be the grantor".into(),
            ));
        }
        if grant.governing_node != StableUri::node(local_node_id) {
            return Err(MvError::InvalidInput(
                "authority grants must be governed by the local Context Node".into(),
            ));
        }
        let local_node = self
            .load_context_node_from_connection(&transaction, local_node_id)?
            .ok_or_else(|| {
                MvError::InvalidInput(
                    "authority grants require a registered local Context Node".into(),
                )
            })?;
        if local_node.status != ContextNodeStatus::Active {
            return Err(MvError::InvalidInput(
                "authority grants require an active local Context Node".into(),
            ));
        }
        if !grant.is_effective_at(event.occurred_at) {
            return Err(MvError::InvalidInput(
                "authority grant must be effective when its issuance event occurs".into(),
            ));
        }

        if let Some(existing_event) =
            Self::resolve_governance_replay(&transaction, event, AUTHORITY_GRANT_ISSUED_V1)?
        {
            let existing_grant_id = event_authority_grant_id(&existing_event)?;
            let existing_revision = event_authority_grant_revision(&existing_event, 1)?;
            let existing_grant = self
                .load_authority_grant_revision_from_connection(
                    &transaction,
                    existing_grant_id,
                    existing_revision,
                )?
                .ok_or_else(|| {
                    MvError::Storage(
                        "authority-grant replay references a missing grant revision".into(),
                    )
                })?;
            transaction
                .commit()
                .map_err(|err| MvError::Storage(format!("finish authority-grant replay: {err}")))?;
            return Ok(IdempotentAuthorityGrantCommit {
                grant: existing_grant,
                event: existing_event,
                replayed: true,
            });
        }

        if let Some(parent_id) = grant.parent_grant_id {
            let parent = self
                .load_authority_grant_from_connection(&transaction, parent_id)?
                .ok_or_else(|| {
                    MvError::InvalidInput("authority-grant parent does not exist".into())
                })?;
            if !grant.is_delegation_subset_of(&parent)
                || !self.authority_grant_chain_is_effective(
                    &transaction,
                    &parent,
                    event.occurred_at,
                )?
            {
                return Err(MvError::InvalidInput(
                    "delegated authority grant must be a strict subset of an effective parent chain"
                        .into(),
                ));
            }
        } else if grant.grantor.principal_context_node_uuid() != Some(local_node_id) {
            return Err(MvError::InvalidInput(
                "root authority grants must be issued by a local principal".into(),
            ));
        }

        self.insert_authority_grant(&transaction, grant)?;
        Self::insert_outbox_event(&transaction, event)?;
        transaction
            .commit()
            .map_err(|err| MvError::Storage(format!("commit authority-grant issuance: {err}")))?;
        Ok(IdempotentAuthorityGrantCommit {
            grant: grant.clone(),
            event: event.clone(),
            replayed: false,
        })
    }

    async fn get_authority_grant(&self, grant_id: Uuid) -> MvResult<Option<AuthorityGrant>> {
        self.with_conn(|connection| self.load_authority_grant_from_connection(connection, grant_id))
    }

    async fn list_authority_grants(
        &self,
        grantee: Option<&StableUri>,
        kind: Option<AuthorityGrantKind>,
        status: Option<AuthorityGrantStatus>,
    ) -> MvResult<Vec<AuthorityGrant>> {
        self.with_conn(|connection| {
            let mut statement = connection
                .prepare(
                    "SELECT grant_id, revision, grant_uri, grant_kind, grantor_uri, grantee_uri,
                            governing_node_uri, status, parent_grant_id, not_before, expires_at,
                            record_payload, payload_format, payload_wrapped_dek, created_at, updated_at
                     FROM interoperability_authority_grants
                     WHERE (?1 IS NULL OR grantee_uri = ?1)
                       AND (?2 IS NULL OR grant_kind = ?2)
                       AND (?3 IS NULL OR status = ?3)
                     ORDER BY updated_at ASC, grant_id ASC",
                )
                .map_err(|err| MvError::Storage(format!("prepare authority-grant query: {err}")))?;
            let grantee = grantee.map(StableUri::as_str);
            let kind = kind.map(|value| value.as_str());
            let status = status.map(|value| value.as_str());
            let rows = statement
                .query_map(params![grantee, kind, status], |row| {
                    self.row_to_authority_grant(row)
                })
                .map_err(|err| MvError::Storage(format!("query authority grants: {err}")))?;
            let mut grants = Vec::new();
            for row in rows {
                grants.push(
                    row.map_err(|err| MvError::Storage(format!("read authority grant: {err}")))?,
                );
            }
            Ok(grants)
        })
    }

    async fn find_authorizing_grant(
        &self,
        query: GrantQuery<'_>,
    ) -> MvResult<Option<AuthorityGrant>> {
        let GrantQuery {
            grantee,
            kind,
            target,
            capability,
            sensitivity,
            retention,
            at,
        } = query;
        self.with_conn(|connection| {
            let mut statement = connection
                .prepare(
                    "SELECT grant_id, revision, grant_uri, grant_kind, grantor_uri, grantee_uri,
                            governing_node_uri, status, parent_grant_id, not_before, expires_at,
                            record_payload, payload_format, payload_wrapped_dek, created_at, updated_at
                     FROM interoperability_authority_grants
                     WHERE grantee_uri = ?1
                       AND grant_kind = ?2
                       AND status = 'active'
                       AND not_before <= ?3
                       AND expires_at > ?3
                     ORDER BY expires_at ASC, grant_id ASC",
                )
                .map_err(|err| {
                    MvError::Storage(format!("prepare authority resolution query: {err}"))
                })?;
            let at_text = at.to_rfc3339();
            let rows = statement
                .query_map(params![grantee.as_str(), kind.as_str(), at_text], |row| {
                    self.row_to_authority_grant(row)
                })
                .map_err(|err| MvError::Storage(format!("query authorizing grants: {err}")))?;
            for row in rows {
                let grant = row.map_err(|err| {
                    MvError::Storage(format!("read authorizing grant candidate: {err}"))
                })?;
                if grant.allows(kind, target, capability, sensitivity, retention, at)
                    && self.authority_grant_chain_is_effective(connection, &grant, at)?
                {
                    return Ok(Some(grant));
                }
            }
            Ok(None)
        })
    }

    async fn transition_authority_grant_with_event(
        &self,
        expected_revision: u64,
        replacement: &AuthorityGrant,
        event: &EventEnvelope,
    ) -> MvResult<IdempotentAuthorityGrantCommit> {
        replacement.validate().map_err(MvError::InvalidInput)?;
        let next_revision = expected_revision
            .checked_add(1)
            .ok_or_else(|| MvError::InvalidInput("authority-grant revision overflow".into()))?;
        if replacement.revision != next_revision {
            return Err(MvError::InvalidInput(
                "replacement authority-grant revision must advance exactly once".into(),
            ));
        }
        Self::validate_authority_grant_event_record(event, replacement, true, false)?;

        let mut connection = self
            .conn()
            .lock()
            .map_err(|err| MvError::Storage(err.to_string()))?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|err| MvError::Storage(format!("begin authority-grant transition: {err}")))?;
        let local_node_id = Self::ensure_local_context_node(&transaction)?;
        Self::validate_governance_event(
            &transaction,
            event,
            local_node_id,
            AUTHORITY_GRANT_LIFECYCLE_TRANSITIONED_V1,
            "authority-grant-lifecycle-transitioned",
        )?;
        if event.principal != replacement.grantor || event.actor != replacement.grantor {
            return Err(MvError::InvalidInput(
                "authority-grant transition principal and actor must be the grantor".into(),
            ));
        }
        if replacement.governing_node != StableUri::node(local_node_id) {
            return Err(MvError::InvalidInput(
                "authority grants must be governed by the local Context Node".into(),
            ));
        }

        if let Some(existing_event) = Self::resolve_governance_replay(
            &transaction,
            event,
            AUTHORITY_GRANT_LIFECYCLE_TRANSITIONED_V1,
        )? {
            let existing_grant_id = event_authority_grant_id(&existing_event)?;
            let existing_revision = event_authority_grant_revision(&existing_event, 1)?;
            let existing_grant = self
                .load_authority_grant_revision_from_connection(
                    &transaction,
                    existing_grant_id,
                    existing_revision,
                )?
                .ok_or_else(|| {
                    MvError::Storage(
                        "authority-grant replay references a missing grant revision".into(),
                    )
                })?;
            transaction
                .commit()
                .map_err(|err| MvError::Storage(format!("finish authority-grant replay: {err}")))?;
            return Ok(IdempotentAuthorityGrantCommit {
                grant: existing_grant,
                event: existing_event,
                replayed: true,
            });
        }

        let current = self
            .load_authority_grant_from_connection(&transaction, replacement.grant_id)?
            .ok_or_else(|| MvError::InvalidInput("authority grant does not exist".into()))?;
        if current.revision != expected_revision
            || !current.status.can_transition_to(replacement.status)
            || event
                .data
                .get("from_status")
                .and_then(|value| value.as_str())
                != Some(current.status.as_str())
            || event.data.get("to_status").and_then(|value| value.as_str())
                != Some(replacement.status.as_str())
        {
            return Err(MvError::InvalidInput(
                "authority-grant lifecycle transition is not allowed or does not match its event"
                    .into(),
            ));
        }
        let mut expected = current.clone();
        expected.revision = replacement.revision;
        expected.status = replacement.status;
        expected.status_reason = replacement.status_reason.clone();
        expected.updated_at = replacement.updated_at;
        if expected != *replacement {
            return Err(MvError::InvalidInput(
                "authority-grant lifecycle transitions may change only status, reason, revision, and update time"
                    .into(),
            ));
        }
        if replacement.status == AuthorityGrantStatus::Expired
            && event.occurred_at < replacement.expires_at
        {
            return Err(MvError::InvalidInput(
                "authority grants cannot transition to expired before their expiry time".into(),
            ));
        }
        if replacement.status == AuthorityGrantStatus::Active
            && !self.authority_grant_chain_is_effective(
                &transaction,
                replacement,
                event.occurred_at,
            )?
        {
            return Err(MvError::InvalidInput(
                "authority grant cannot reactivate without an effective parent chain".into(),
            ));
        }

        self.update_authority_grant(&transaction, expected_revision, replacement)?;
        Self::insert_outbox_event(&transaction, event)?;
        transaction
            .commit()
            .map_err(|err| MvError::Storage(format!("commit authority-grant transition: {err}")))?;
        Ok(IdempotentAuthorityGrantCommit {
            grant: replacement.clone(),
            event: event.clone(),
            replayed: false,
        })
    }

    async fn commit_command_admission_decision(
        &self,
        record: &CommandAdmissionDecisionRecord,
    ) -> MvResult<IdempotentAdmissionDecisionCommit> {
        let mut connection = self
            .conn()
            .lock()
            .map_err(|err| MvError::Storage(err.to_string()))?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|err| MvError::Storage(format!("begin admission-decision commit: {err}")))?;

        if let Some(existing) = Self::load_command_admission_decision(
            &transaction,
            &record.principal,
            &record.idempotency_key,
        )? {
            if existing.admission_digest != record.admission_digest
                || !Self::admission_decisions_match(&existing.decision, &record.decision)
            {
                return Err(MvError::IdempotencyConflict(
                    "command admission decision conflicts with a prior record for the same principal and idempotency key"
                        .into(),
                ));
            }
            transaction.commit().map_err(|err| {
                MvError::Storage(format!("finish admission-decision replay: {err}"))
            })?;
            return Ok(IdempotentAdmissionDecisionCommit {
                record: existing,
                replayed: true,
            });
        }

        Self::insert_command_admission_decision(&transaction, record)?;
        transaction
            .commit()
            .map_err(|err| MvError::Storage(format!("commit admission decision: {err}")))?;
        Ok(IdempotentAdmissionDecisionCommit {
            record: record.clone(),
            replayed: false,
        })
    }

    async fn get_command_admission_decision(
        &self,
        principal: &StableUri,
        idempotency_key: &IdempotencyKey,
    ) -> MvResult<Option<CommandAdmissionDecisionRecord>> {
        self.with_conn(|connection| {
            Self::load_command_admission_decision(connection, principal, idempotency_key)
        })
    }

    async fn commit_public_schema_with_event(
        &self,
        schema: &PublicSchemaRecord,
        event: &EventEnvelope,
    ) -> MvResult<IdempotentSchemaCommit> {
        schema.validate().map_err(MvError::InvalidInput)?;
        if schema.lifecycle != PublicSchemaLifecycle::Active {
            return Err(MvError::InvalidInput(
                "new public schema versions must be registered as active".into(),
            ));
        }
        let expected_subject =
            StableUri::schema_version(&schema.schema.uri, &schema.schema.version)
                .map_err(MvError::InvalidInput)?;
        if event.subject != expected_subject
            || event
                .data
                .get("schema_uri")
                .and_then(|value| value.as_str())
                != Some(schema.schema.uri.as_str())
            || event
                .data
                .get("schema_version")
                .and_then(|value| value.as_str())
                != Some(schema.schema.version.as_str())
            || event
                .data
                .get("content_digest")
                .and_then(|value| value.as_str())
                != Some(schema.content_digest.as_str())
        {
            return Err(MvError::InvalidInput(
                "schema registration event must match the governed schema record".into(),
            ));
        }

        let mut connection = self
            .conn()
            .lock()
            .map_err(|err| MvError::Storage(err.to_string()))?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|err| MvError::Storage(format!("begin schema registration: {err}")))?;
        let local_node_id = Self::ensure_local_context_node(&transaction)?;
        Self::validate_governance_event(
            &transaction,
            event,
            local_node_id,
            PUBLIC_SCHEMA_REGISTERED_V1,
            "public-schema-registered",
        )?;

        if let Some(existing_event) =
            Self::resolve_governance_replay(&transaction, event, PUBLIC_SCHEMA_REGISTERED_V1)?
        {
            let schema_uri = existing_event
                .data
                .get("schema_uri")
                .and_then(|value| value.as_str())
                .ok_or_else(|| {
                    MvError::Storage("stored schema event is missing schema_uri".into())
                })?;
            let schema_version = existing_event
                .data
                .get("schema_version")
                .and_then(|value| value.as_str())
                .ok_or_else(|| {
                    MvError::Storage("stored schema event is missing schema_version".into())
                })?;
            let reference = SchemaReference::new(
                StableUri::parse(schema_uri).map_err(MvError::Storage)?,
                schema_version,
            )
            .map_err(MvError::Storage)?;
            let existing_schema =
                Self::load_public_schema_from_connection(&transaction, &reference)?.ok_or_else(
                    || MvError::Storage("schema replay references a missing schema record".into()),
                )?;
            transaction
                .commit()
                .map_err(|err| MvError::Storage(format!("finish schema replay: {err}")))?;
            return Ok(IdempotentSchemaCommit {
                schema: existing_schema,
                event: existing_event,
                replayed: true,
            });
        }

        let definition_json = serde_json::to_string(&schema.definition)?;
        transaction
            .execute(
                "INSERT INTO interoperability_public_schemas
                 (schema_uri, schema_version, media_type, definition_json, content_digest,
                  lifecycle, owner_uri, created_at, deprecated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    schema.schema.uri.as_str(),
                    &schema.schema.version,
                    &schema.media_type,
                    definition_json,
                    &schema.content_digest,
                    schema.lifecycle.as_str(),
                    schema.owner.as_str(),
                    schema.created_at.to_rfc3339(),
                    schema.deprecated_at.map(|value| value.to_rfc3339()),
                ],
            )
            .map_err(|err| MvError::Storage(format!("insert public schema: {err}")))?;
        Self::insert_outbox_event(&transaction, event)?;
        transaction
            .commit()
            .map_err(|err| MvError::Storage(format!("commit schema registration: {err}")))?;
        Ok(IdempotentSchemaCommit {
            schema: schema.clone(),
            event: event.clone(),
            replayed: false,
        })
    }

    async fn get_public_schema(
        &self,
        reference: &SchemaReference,
    ) -> MvResult<Option<PublicSchemaRecord>> {
        self.with_conn(|connection| Self::load_public_schema_from_connection(connection, reference))
    }

    async fn list_public_schema_versions(
        &self,
        schema_uri: &StableUri,
    ) -> MvResult<Vec<PublicSchemaRecord>> {
        self.with_conn(|connection| {
            let mut statement = connection
                .prepare(
                    "SELECT schema_uri, schema_version, media_type, definition_json,
                            content_digest, lifecycle, owner_uri, created_at, deprecated_at
                     FROM interoperability_public_schemas
                     WHERE schema_uri = ?1
                     ORDER BY created_at ASC, schema_version ASC",
                )
                .map_err(|err| {
                    MvError::Storage(format!("prepare public schema version query: {err}"))
                })?;
            let rows = statement
                .query_map(params![schema_uri.as_str()], Self::row_to_public_schema)
                .map_err(|err| MvError::Storage(format!("query public schema versions: {err}")))?;
            let mut schemas = Vec::new();
            for row in rows {
                schemas.push(
                    row.map_err(|err| MvError::Storage(format!("read public schema: {err}")))?,
                );
            }
            Ok(schemas)
        })
    }

    async fn commit_source_binding_with_event(
        &self,
        binding: &SourceBinding,
        event: &EventEnvelope,
    ) -> MvResult<IdempotentSourceBindingCommit> {
        binding.validate().map_err(MvError::InvalidInput)?;
        if binding.revision != 1
            || binding.status != SourceBindingStatus::Active
            || binding.supersedes_binding_id.is_some()
        {
            return Err(MvError::InvalidInput(
                "new source bindings must be active revision one without a predecessor".into(),
            ));
        }
        let local_node_id = binding.context_node.context_node_uuid().ok_or_else(|| {
            MvError::InvalidInput("source binding context node must end in its node UUID".into())
        })?;
        let expected_subject = StableUri::source_binding(local_node_id, binding.binding_id);
        if event.subject != expected_subject
            || event
                .data
                .get("binding_id")
                .and_then(|value| value.as_str())
                != Some(binding.binding_id.to_string().as_str())
            || event
                .data
                .get("resource_uri")
                .and_then(|value| value.as_str())
                != Some(binding.resource_uri.as_str())
            || event
                .data
                .get("external_system")
                .and_then(|value| value.as_str())
                != Some(binding.external_system.as_str())
            || event
                .data
                .get("materialization_mode")
                .and_then(|value| value.as_str())
                != Some(binding.materialization_mode.as_str())
        {
            return Err(MvError::InvalidInput(
                "source-binding event must match the governed binding record".into(),
            ));
        }

        let mut connection = self
            .conn()
            .lock()
            .map_err(|err| MvError::Storage(err.to_string()))?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|err| MvError::Storage(format!("begin source binding registration: {err}")))?;
        let persisted_local_node_id = Self::ensure_local_context_node(&transaction)?;
        if local_node_id != persisted_local_node_id
            || binding.context_node != StableUri::node(persisted_local_node_id)
        {
            return Err(MvError::InvalidInput(
                "source binding must be governed by the local context node".into(),
            ));
        }
        Self::validate_governance_event(
            &transaction,
            event,
            persisted_local_node_id,
            SOURCE_BINDING_REGISTERED_V1,
            "source-binding-registered",
        )?;

        if let Some(existing_event) =
            Self::resolve_governance_replay(&transaction, event, SOURCE_BINDING_REGISTERED_V1)?
        {
            let existing_id = event_binding_id(&existing_event)?;
            let existing_binding = self
                .load_source_binding_from_connection(&transaction, existing_id)?
                .ok_or_else(|| {
                    MvError::Storage("source-binding replay references a missing binding".into())
                })?;
            transaction
                .commit()
                .map_err(|err| MvError::Storage(format!("finish source-binding replay: {err}")))?;
            return Ok(IdempotentSourceBindingCommit {
                binding: existing_binding,
                event: existing_event,
                replayed: true,
            });
        }

        self.require_active_context_node(&transaction, persisted_local_node_id)?;
        self.insert_source_binding(&transaction, binding)?;
        Self::insert_outbox_event(&transaction, event)?;
        transaction
            .commit()
            .map_err(|err| MvError::Storage(format!("commit source binding: {err}")))?;
        Ok(IdempotentSourceBindingCommit {
            binding: binding.clone(),
            event: event.clone(),
            replayed: false,
        })
    }

    async fn get_source_binding(&self, binding_id: Uuid) -> MvResult<Option<SourceBinding>> {
        self.with_conn(|connection| {
            self.load_source_binding_from_connection(connection, binding_id)
        })
    }

    async fn find_active_source_binding(
        &self,
        context_node: &StableUri,
        external_account_id: &str,
        external_object_id: &str,
    ) -> MvResult<Option<SourceBinding>> {
        if external_account_id.is_empty() || external_object_id.is_empty() {
            return Err(MvError::InvalidInput(
                "external account and object identifiers must not be empty".into(),
            ));
        }
        self.with_conn(|connection| {
            connection
                .query_row(
                    "SELECT binding_id, revision, resource_uri, context_node_uri,
                            external_system, external_account_key, external_object_key,
                            status, supersedes_binding_id, record_payload, payload_format,
                            payload_wrapped_dek, created_at, updated_at
                     FROM interoperability_source_bindings
                     WHERE context_node_uri = ?1
                       AND external_account_key = ?2
                       AND external_object_key = ?3
                       AND status = 'active'",
                    params![
                        context_node.as_str(),
                        self.source_lookup_key(external_account_id)?,
                        self.source_lookup_key(external_object_id)?,
                    ],
                    |row| self.row_to_source_binding(row),
                )
                .optional()
                .map_err(|err| MvError::Storage(format!("find active source binding: {err}")))
        })
    }

    async fn rebind_source_with_event(
        &self,
        previous_binding_id: Uuid,
        replacement: &SourceBinding,
        event: &EventEnvelope,
    ) -> MvResult<IdempotentSourceBindingCommit> {
        replacement.validate().map_err(MvError::InvalidInput)?;
        if replacement.revision != 1
            || replacement.status != SourceBindingStatus::Active
            || replacement.supersedes_binding_id != Some(previous_binding_id)
        {
            return Err(MvError::InvalidInput(
                "replacement binding must be active revision one and name its predecessor".into(),
            ));
        }
        let local_node_id = replacement
            .context_node
            .context_node_uuid()
            .ok_or_else(|| {
                MvError::InvalidInput("replacement context node must end in its node UUID".into())
            })?;
        let expected_subject = StableUri::source_binding(local_node_id, replacement.binding_id);
        if event.subject != expected_subject
            || event_binding_id(event)? != replacement.binding_id
            || event
                .data
                .get("supersedes_binding_id")
                .and_then(|value| value.as_str())
                != Some(previous_binding_id.to_string().as_str())
        {
            return Err(MvError::InvalidInput(
                "source-rebinding event must identify the replacement and predecessor".into(),
            ));
        }

        let mut connection = self
            .conn()
            .lock()
            .map_err(|err| MvError::Storage(err.to_string()))?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|err| MvError::Storage(format!("begin source rebinding: {err}")))?;
        let persisted_local_node_id = Self::ensure_local_context_node(&transaction)?;
        if local_node_id != persisted_local_node_id
            || replacement.context_node != StableUri::node(persisted_local_node_id)
        {
            return Err(MvError::InvalidInput(
                "replacement must be governed by the local context node".into(),
            ));
        }
        Self::validate_governance_event(
            &transaction,
            event,
            persisted_local_node_id,
            SOURCE_BINDING_REBOUND_V1,
            "source-binding-rebound",
        )?;
        if let Some(existing_event) =
            Self::resolve_governance_replay(&transaction, event, SOURCE_BINDING_REBOUND_V1)?
        {
            let existing_id = event_binding_id(&existing_event)?;
            let existing_binding = self
                .load_source_binding_from_connection(&transaction, existing_id)?
                .ok_or_else(|| {
                    MvError::Storage("rebinding replay references a missing binding".into())
                })?;
            transaction
                .commit()
                .map_err(|err| MvError::Storage(format!("finish rebinding replay: {err}")))?;
            return Ok(IdempotentSourceBindingCommit {
                binding: existing_binding,
                event: existing_event,
                replayed: true,
            });
        }

        self.require_active_context_node(&transaction, persisted_local_node_id)?;
        let previous = self
            .load_source_binding_from_connection(&transaction, previous_binding_id)?
            .ok_or_else(|| MvError::InvalidInput("predecessor binding does not exist".into()))?;
        if previous.status != SourceBindingStatus::Active {
            return Err(MvError::InvalidInput(
                "only an active source binding can be rebound".into(),
            ));
        }
        if previous.context_node != replacement.context_node {
            return Err(MvError::InvalidInput(
                "rebinding cannot transfer context-node authority implicitly".into(),
            ));
        }
        if previous.resource_uri != replacement.resource_uri {
            return Err(MvError::InvalidInput(
                "rebinding must preserve the canonical resource identity".into(),
            ));
        }
        if event.occurred_at < previous.updated_at {
            return Err(MvError::InvalidInput(
                "rebinding event cannot predate the active source binding".into(),
            ));
        }

        let mut retired = previous.clone();
        retired.status = SourceBindingStatus::Migrated;
        retired.revision = previous.revision + 1;
        retired.updated_at = event.occurred_at;
        retired.validate().map_err(MvError::InvalidInput)?;
        let (payload, payload_format, wrapped_dek) =
            self.encode_governance_record(&retired, "retired source binding")?;
        let updated = transaction
            .execute(
                "UPDATE interoperability_source_bindings
                 SET revision = ?2, status = ?3, record_payload = ?4,
                     payload_format = ?5, payload_wrapped_dek = ?6, updated_at = ?7
                 WHERE binding_id = ?1 AND revision = ?8 AND status = 'active'",
                params![
                    previous.binding_id.to_string(),
                    retired.revision,
                    retired.status.as_str(),
                    payload,
                    payload_format,
                    wrapped_dek,
                    retired.updated_at.to_rfc3339(),
                    previous.revision,
                ],
            )
            .map_err(|err| MvError::Storage(format!("retire source binding: {err}")))?;
        if updated != 1 {
            return Err(MvError::IdempotencyConflict(
                "source binding changed during rebinding".into(),
            ));
        }
        self.insert_source_binding(&transaction, replacement)?;
        Self::insert_outbox_event(&transaction, event)?;
        transaction
            .commit()
            .map_err(|err| MvError::Storage(format!("commit source rebinding: {err}")))?;
        Ok(IdempotentSourceBindingCommit {
            binding: replacement.clone(),
            event: event.clone(),
            replayed: false,
        })
    }

    async fn commit_node_create_with_event(
        &self,
        node: &KnowledgeNode,
        event: &EventEnvelope,
    ) -> MvResult<IdempotentNodeCommit> {
        event.validate().map_err(MvError::InvalidInput)?;
        if event.subject.trailing_uuid() != Some(node.id) {
            return Err(MvError::InvalidInput(
                "event subject must identify the node being created".into(),
            ));
        }

        let mut conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| MvError::Storage(format!("begin interoperable mutation: {e}")))?;

        let local_node_id = Self::ensure_local_context_node(&tx)?;
        let expected_source = StableUri::node(local_node_id);
        let expected_subject = StableUri::knowledge_node(local_node_id, node.id);
        let expected_schema = StableUri::schema("knowledge-node-created")
            .map_err(|e| MvError::Storage(format!("invalid built-in event schema: {e}")))?;
        if event.source != expected_source {
            return Err(MvError::InvalidInput(
                "event source must identify the local context node".into(),
            ));
        }
        if event.subject != expected_subject {
            return Err(MvError::InvalidInput(
                "event subject must use the local canonical knowledge-node URI".into(),
            ));
        }
        if event.event_type != KNOWLEDGE_NODE_CREATED_V1
            || event.schema.uri != expected_schema
            || event.schema.version != "1.0.0"
        {
            return Err(MvError::InvalidInput(
                "node creation requires the supported knowledge-node-created event schema".into(),
            ));
        }
        Self::require_active_event_schema(&tx, event)?;
        if !event.provenance.iter().any(|reference| {
            reference.resource == expected_subject
                && reference.relation == ProvenanceRelation::PrimarySource
        }) {
            return Err(MvError::InvalidInput(
                "node creation event must identify the node as its primary source".into(),
            ));
        }
        if event
            .data
            .get("resource_kind")
            .and_then(|value| value.as_str())
            != Some("knowledge_node")
            || event.data.get("node_kind").and_then(|value| value.as_str())
                != Some(node.kind.as_str())
            || event.data.get("namespace").and_then(|value| value.as_str())
                != Some(node.namespace.as_str())
        {
            return Err(MvError::InvalidInput(
                "node creation event data must match the canonical node".into(),
            ));
        }

        if let Some(existing) = self.resolve_node_create_replay(
            &tx,
            &event.source,
            &event.principal,
            &event.idempotency_key,
            &event.payload_digest,
        )? {
            tx.commit()
                .map_err(|e| MvError::Storage(format!("finish idempotent replay: {e}")))?;
            return Ok(existing);
        }

        let (title, content, source, metadata_json, payload_ciphertext, payload_wrapped_dek) =
            self.project_node_for_storage(node)?;
        tx.execute(
            "INSERT INTO knowledge_nodes
             (id, kind, title, content, source, namespace, importance,
              created_at, updated_at, last_accessed_at, access_count, version,
              expires_at, metadata_json, payload_ciphertext, payload_wrapped_dek)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                node.id.to_string(),
                node.kind.as_str(),
                title,
                content,
                source,
                node.namespace,
                node.importance,
                node.temporal.created_at.to_rfc3339(),
                node.temporal.updated_at.to_rfc3339(),
                node.temporal.last_accessed_at.to_rfc3339(),
                node.temporal.access_count,
                node.temporal.version,
                node.temporal.expires_at.map(|dt| dt.to_rfc3339()),
                metadata_json,
                payload_ciphertext,
                payload_wrapped_dek,
            ],
        )
        .map_err(|e| MvError::Storage(format!("insert interoperable node: {e}")))?;
        Self::save_tags(&tx, node.id, &node.tags)?;
        Self::log_change(&tx, node.id, ChangeOp::Create, None)?;

        Self::insert_outbox_event(&tx, event)?;
        tx.commit()
            .map_err(|e| MvError::Storage(format!("commit interoperable mutation: {e}")))?;

        Ok(IdempotentNodeCommit {
            node: node.clone(),
            event: event.clone(),
            replayed: false,
        })
    }

    async fn find_node_create_replay(
        &self,
        source: &StableUri,
        principal: &StableUri,
        idempotency_key: &IdempotencyKey,
        payload_digest: &str,
    ) -> MvResult<Option<IdempotentNodeCommit>> {
        self.with_conn(|conn| {
            self.resolve_node_create_replay(
                conn,
                source,
                principal,
                idempotency_key,
                payload_digest,
            )
        })
    }

    async fn get_outbox_event(&self, event_id: Uuid) -> MvResult<Option<EventEnvelope>> {
        self.with_conn(|conn| {
            let envelope_json: Option<String> = conn
                .query_row(
                    "SELECT envelope_json FROM interoperability_outbox WHERE event_id = ?1",
                    params![event_id.to_string()],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|e| MvError::Storage(format!("load interoperability event: {e}")))?;
            envelope_json
                .map(|json| Self::decode_outbox_event(&json))
                .transpose()
        })
    }

    async fn list_pending_outbox_events(&self, limit: usize) -> MvResult<Vec<EventEnvelope>> {
        if !(1..=1000).contains(&limit) {
            return Err(MvError::InvalidInput(
                "outbox event limit must be between 1 and 1000".into(),
            ));
        }
        self.with_conn(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT envelope_json FROM interoperability_outbox
                     WHERE delivery_state = 'pending'
                     ORDER BY created_at ASC, event_id ASC
                     LIMIT ?1",
                )
                .map_err(|e| MvError::Storage(format!("prepare outbox query: {e}")))?;
            let rows = stmt
                .query_map(params![limit as i64], |row| row.get::<_, String>(0))
                .map_err(|e| MvError::Storage(format!("query pending outbox events: {e}")))?;
            let mut events = Vec::new();
            for row in rows {
                let json =
                    row.map_err(|e| MvError::Storage(format!("read pending outbox event: {e}")))?;
                events.push(Self::decode_outbox_event(&json)?);
            }
            Ok(events)
        })
    }

    async fn get_outbox_delivery_status(
        &self,
        event_id: Uuid,
    ) -> MvResult<Option<OutboxDeliveryStatus>> {
        self.with_conn(|conn| {
            conn.query_row(
                "SELECT event_id, delivery_state, delivery_attempts, next_attempt_at,
                        lease_expires_at, published_at, last_error, updated_at
                 FROM interoperability_outbox
                 WHERE event_id = ?1",
                params![event_id.to_string()],
                |row| {
                    let stored_event_id: String = row.get(0)?;
                    let state: String = row.get(1)?;
                    let next_attempt_at: String = row.get(3)?;
                    let lease_expires_at: Option<String> = row.get(4)?;
                    let published_at: Option<String> = row.get(5)?;
                    let updated_at: String = row.get(7)?;
                    Ok(OutboxDeliveryStatus {
                        event_id: parse_uuid_str(0, &stored_event_id)?,
                        state: state
                            .parse()
                            .map_err(|err: String| Self::as_sql_conversion_error(1, err))?,
                        attempts: row.get(2)?,
                        next_attempt_at: parse_dt_strict(3, &next_attempt_at)?,
                        lease_expires_at: parse_optional_dt_strict(4, lease_expires_at)?,
                        published_at: parse_optional_dt_strict(5, published_at)?,
                        last_error_code: row.get(6)?,
                        updated_at: parse_dt_strict(7, &updated_at)?,
                    })
                },
            )
            .optional()
            .map_err(|err| MvError::Storage(format!("load outbox delivery status: {err}")))
        })
    }

    async fn claim_outbox_events(
        &self,
        executor: &StableUri,
        destination: &StableUri,
        claimed_at: chrono::DateTime<Utc>,
        lease_expires_at: chrono::DateTime<Utc>,
        limit: usize,
    ) -> MvResult<Vec<OutboxDeliveryClaim>> {
        if !(1..=1000).contains(&limit) {
            return Err(MvError::InvalidInput(
                "outbox claim limit must be between 1 and 1000".into(),
            ));
        }
        if lease_expires_at <= claimed_at
            || lease_expires_at - claimed_at > chrono::Duration::hours(1)
        {
            return Err(MvError::InvalidInput(
                "outbox lease must be positive and no longer than one hour".into(),
            ));
        }

        let mut connection = self
            .conn()
            .lock()
            .map_err(|err| MvError::Storage(err.to_string()))?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|err| MvError::Storage(format!("begin outbox claim: {err}")))?;
        let candidates = {
            let mut statement = transaction
                .prepare(
                    "SELECT event_id, envelope_json, delivery_attempts
                     FROM interoperability_outbox
                     WHERE delivery_state = 'pending'
                       AND next_attempt_at <= ?1
                       AND (lease_id IS NULL OR lease_expires_at <= ?1)
                     ORDER BY created_at ASC, event_id ASC
                     LIMIT ?2",
                )
                .map_err(|err| MvError::Storage(format!("prepare outbox claim: {err}")))?;
            let rows = statement
                .query_map(params![claimed_at.to_rfc3339(), limit as i64], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, u32>(2)?,
                    ))
                })
                .map_err(|err| MvError::Storage(format!("query dispatchable events: {err}")))?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|err| MvError::Storage(format!("read dispatchable event: {err}")))?
        };

        let mut claims = Vec::with_capacity(candidates.len());
        for (event_id, envelope_json, previous_attempts) in candidates {
            let event = Self::decode_outbox_event(&envelope_json)?;
            if event.id.to_string() != event_id {
                return Err(MvError::Storage(
                    "outbox envelope ID does not match its governed index".into(),
                ));
            }
            let lease_id = Uuid::now_v7();
            let attempt = previous_attempts.checked_add(1).ok_or_else(|| {
                MvError::Storage("outbox delivery attempt counter overflow".into())
            })?;
            let updated = transaction
                .execute(
                    "UPDATE interoperability_outbox
                     SET delivery_attempts = ?2,
                         lease_id = ?3,
                         lease_owner_uri = ?4,
                         lease_destination_uri = ?5,
                         lease_expires_at = ?6,
                         last_attempt_at = ?7,
                         updated_at = ?7
                     WHERE event_id = ?1
                       AND delivery_state = 'pending'
                       AND delivery_attempts = ?8
                       AND next_attempt_at <= ?7
                       AND (lease_id IS NULL OR lease_expires_at <= ?7)",
                    params![
                        &event_id,
                        attempt,
                        lease_id.to_string(),
                        executor.as_str(),
                        destination.as_str(),
                        lease_expires_at.to_rfc3339(),
                        claimed_at.to_rfc3339(),
                        previous_attempts,
                    ],
                )
                .map_err(|err| MvError::Storage(format!("claim outbox event: {err}")))?;
            if updated != 1 {
                return Err(MvError::Storage(
                    "outbox event changed while its claim was being committed".into(),
                ));
            }
            let claim = OutboxDeliveryClaim {
                event,
                lease_id,
                attempt,
                executor: executor.clone(),
                destination: destination.clone(),
                claimed_at,
                lease_expires_at,
            };
            claim.validate().map_err(MvError::InvalidInput)?;
            claims.push(claim);
        }
        transaction
            .commit()
            .map_err(|err| MvError::Storage(format!("commit outbox claims: {err}")))?;
        Ok(claims)
    }

    async fn complete_outbox_delivery(
        &self,
        claim: &OutboxDeliveryClaim,
        completion: &OutboxDeliveryCompletion,
    ) -> MvResult<ActionReceipt> {
        completion
            .validate_for(claim)
            .map_err(MvError::InvalidInput)?;

        let mut connection = self
            .conn()
            .lock()
            .map_err(|err| MvError::Storage(err.to_string()))?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|err| MvError::Storage(format!("begin outbox completion: {err}")))?;

        if let Some(existing) =
            self.load_action_receipt_by_attempt(&transaction, claim.event.id, claim.attempt)?
        {
            if !existing.matches_delivery(claim, completion) {
                return Err(MvError::IdempotencyConflict(
                    "delivery attempt already has a different action receipt".into(),
                ));
            }
            transaction
                .commit()
                .map_err(|err| MvError::Storage(format!("finish receipt replay: {err}")))?;
            return Ok(existing);
        }

        let stored_claim: Option<(String, u32, String, String, String, String, String)> =
            transaction
                .query_row(
                    "SELECT delivery_state, delivery_attempts, lease_id, lease_owner_uri,
                            lease_destination_uri, last_attempt_at, envelope_json
                     FROM interoperability_outbox
                     WHERE event_id = ?1",
                    params![claim.event.id.to_string()],
                    |row| {
                        Ok((
                            row.get(0)?,
                            row.get(1)?,
                            row.get(2)?,
                            row.get(3)?,
                            row.get(4)?,
                            row.get(5)?,
                            row.get(6)?,
                        ))
                    },
                )
                .optional()
                .map_err(|err| MvError::Storage(format!("load active outbox claim: {err}")))?;
        let Some((
            delivery_state,
            attempt,
            lease_id,
            executor,
            destination,
            started_at,
            envelope_json,
        )) = stored_claim
        else {
            return Err(MvError::InvalidInput(format!(
                "outbox event not found: {}",
                claim.event.id
            )));
        };
        let stored_event = Self::decode_outbox_event(&envelope_json)?;
        if delivery_state != OutboxDeliveryState::Pending.as_str()
            || attempt != claim.attempt
            || lease_id != claim.lease_id.to_string()
            || executor != claim.executor.as_str()
            || destination != claim.destination.as_str()
            || started_at != claim.claimed_at.to_rfc3339()
            || stored_event != claim.event
        {
            return Err(MvError::IdempotencyConflict(
                "outbox completion references a stale or different claim".into(),
            ));
        }

        let receipt = ActionReceipt::from_outbox_delivery(claim, completion)
            .map_err(MvError::InvalidInput)?;
        let (payload, payload_format, wrapped_dek) =
            self.encode_governance_record(&receipt, "action receipt")?;
        transaction
            .execute(
                "INSERT INTO interoperability_action_receipts
                 (receipt_id, receipt_version, event_id, claim_id, attempt_no, outcome,
                  executor_uri, destination_uri, subject_uri, principal_uri, actor_uri,
                  correlation_id, request_digest, started_at, completed_at, sensitivity,
                  retention, payload, payload_format, payload_wrapped_dek, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                         ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?15)",
                params![
                    receipt.receipt_id.to_string(),
                    &receipt.receipt_version,
                    receipt.event_id.to_string(),
                    receipt.claim_id.to_string(),
                    receipt.attempt,
                    receipt.outcome.as_str(),
                    receipt.executor.as_str(),
                    receipt.destination.as_str(),
                    receipt.subject.as_str(),
                    receipt.principal.as_str(),
                    receipt.actor.as_str(),
                    receipt.correlation_id.to_string(),
                    &receipt.request_digest,
                    receipt.started_at.to_rfc3339(),
                    receipt.completed_at.to_rfc3339(),
                    receipt.sensitivity.as_str(),
                    receipt.retention.as_str(),
                    payload,
                    payload_format,
                    wrapped_dek,
                ],
            )
            .map_err(|err| MvError::Storage(format!("insert action receipt: {err}")))?;

        let (state, next_attempt_at, published_at, last_error) = match &completion.result {
            OutboxDeliveryResult::Published { .. } => (
                OutboxDeliveryState::Published,
                None,
                Some(completion.completed_at.to_rfc3339()),
                None,
            ),
            OutboxDeliveryResult::RetryScheduled {
                retry_at,
                error_code,
                ..
            } => (
                OutboxDeliveryState::Pending,
                Some(retry_at.to_rfc3339()),
                None,
                Some(error_code.as_str()),
            ),
            OutboxDeliveryResult::DeadLettered { error_code, .. } => (
                OutboxDeliveryState::DeadLetter,
                None,
                None,
                Some(error_code.as_str()),
            ),
        };
        let updated = match state {
            OutboxDeliveryState::Pending => transaction.execute(
                "UPDATE interoperability_outbox
                 SET delivery_state = ?2, next_attempt_at = ?3, published_at = NULL,
                     last_error = ?4, lease_id = NULL, lease_owner_uri = NULL,
                     lease_destination_uri = NULL, lease_expires_at = NULL,
                     updated_at = ?5
                 WHERE event_id = ?1 AND delivery_state = 'pending'
                   AND delivery_attempts = ?6 AND lease_id = ?7",
                params![
                    receipt.event_id.to_string(),
                    state.as_str(),
                    next_attempt_at,
                    last_error,
                    completion.completed_at.to_rfc3339(),
                    claim.attempt,
                    claim.lease_id.to_string(),
                ],
            ),
            OutboxDeliveryState::Published | OutboxDeliveryState::DeadLetter => transaction
                .execute(
                    "UPDATE interoperability_outbox
                 SET delivery_state = ?2, published_at = ?3, last_error = ?4,
                     lease_id = NULL, lease_owner_uri = NULL,
                     lease_destination_uri = NULL, lease_expires_at = NULL,
                     updated_at = ?5
                 WHERE event_id = ?1 AND delivery_state = 'pending'
                   AND delivery_attempts = ?6 AND lease_id = ?7",
                    params![
                        receipt.event_id.to_string(),
                        state.as_str(),
                        published_at,
                        last_error,
                        completion.completed_at.to_rfc3339(),
                        claim.attempt,
                        claim.lease_id.to_string(),
                    ],
                ),
        }
        .map_err(|err| MvError::Storage(format!("complete outbox delivery: {err}")))?;
        if updated != 1 {
            return Err(MvError::IdempotencyConflict(
                "outbox claim changed before completion".into(),
            ));
        }
        transaction
            .commit()
            .map_err(|err| MvError::Storage(format!("commit outbox completion: {err}")))?;
        Ok(receipt)
    }

    async fn get_action_receipt(&self, receipt_id: Uuid) -> MvResult<Option<ActionReceipt>> {
        self.with_conn(|connection| {
            connection
                .query_row(
                    "SELECT receipt_id, receipt_version, event_id, claim_id, attempt_no, outcome,
                            executor_uri, destination_uri, subject_uri, principal_uri,
                            actor_uri, correlation_id, request_digest, started_at,
                            completed_at, sensitivity, retention, payload, payload_format,
                            payload_wrapped_dek
                     FROM interoperability_action_receipts
                     WHERE receipt_id = ?1",
                    params![receipt_id.to_string()],
                    |row| self.row_to_action_receipt(row),
                )
                .optional()
                .map_err(|err| MvError::Storage(format!("load action receipt: {err}")))
        })
    }

    async fn list_action_receipts(
        &self,
        event_id: Uuid,
        limit: usize,
    ) -> MvResult<Vec<ActionReceipt>> {
        if !(1..=1000).contains(&limit) {
            return Err(MvError::InvalidInput(
                "action receipt limit must be between 1 and 1000".into(),
            ));
        }
        self.with_conn(|connection| {
            let mut statement = connection
                .prepare(
                    "SELECT receipt_id, receipt_version, event_id, claim_id, attempt_no, outcome,
                            executor_uri, destination_uri, subject_uri, principal_uri,
                            actor_uri, correlation_id, request_digest, started_at,
                            completed_at, sensitivity, retention, payload, payload_format,
                            payload_wrapped_dek
                     FROM interoperability_action_receipts
                     WHERE event_id = ?1
                     ORDER BY attempt_no ASC
                     LIMIT ?2",
                )
                .map_err(|err| MvError::Storage(format!("prepare action receipts: {err}")))?;
            let rows = statement
                .query_map(params![event_id.to_string(), limit as i64], |row| {
                    self.row_to_action_receipt(row)
                })
                .map_err(|err| MvError::Storage(format!("query action receipts: {err}")))?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|err| MvError::Storage(format!("read action receipt: {err}")))
        })
    }

    async fn admit_consumer_event(
        &self,
        consumer: &StableUri,
        event: &EventEnvelope,
        received_at: chrono::DateTime<Utc>,
    ) -> MvResult<ConsumerInboxAdmission> {
        event.validate().map_err(MvError::InvalidInput)?;
        let mut connection = self
            .conn()
            .lock()
            .map_err(|err| MvError::Storage(err.to_string()))?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|err| MvError::Storage(format!("begin consumer inbox admission: {err}")))?;

        if let Some(mut existing) =
            self.load_consumer_inbox_admission(&transaction, consumer, event.id)?
        {
            if existing.event != *event || existing.event.content_digest() != event.content_digest()
            {
                return Err(MvError::IdempotencyConflict(
                    "consumer inbox event ID already has different envelope content".into(),
                ));
            }
            existing.replayed = true;
            transaction
                .commit()
                .map_err(|err| MvError::Storage(format!("finish inbox admission replay: {err}")))?;
            return Ok(existing);
        }

        let event_digest = event.content_digest();
        let (payload, payload_format, wrapped_dek) =
            self.encode_governance_record(event, "consumer inbox event")?;
        transaction
            .execute(
                "INSERT INTO interoperability_consumer_inbox
                 (consumer_uri, event_id, event_digest, source_uri, subject_uri,
                  principal_uri, actor_uri, event_type, schema_uri, schema_version,
                  correlation_id, occurred_at, sensitivity, retention, envelope_payload,
                  payload_format, payload_wrapped_dek, state, attempts, next_attempt_at,
                  lease_id, lease_processor_uri, lease_expires_at, last_attempt_at,
                  applied_at, last_error_code, received_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                         ?14, ?15, ?16, ?17, 'pending', 0, ?18, NULL, NULL, NULL,
                         NULL, NULL, NULL, ?18, ?18)",
                params![
                    consumer.as_str(),
                    event.id.to_string(),
                    &event_digest,
                    event.source.as_str(),
                    event.subject.as_str(),
                    event.principal.as_str(),
                    event.actor.as_str(),
                    &event.event_type,
                    event.schema.uri.as_str(),
                    &event.schema.version,
                    event.correlation_id.to_string(),
                    event.occurred_at.to_rfc3339(),
                    event.sensitivity.as_str(),
                    event.retention.as_str(),
                    payload,
                    payload_format,
                    wrapped_dek,
                    received_at.to_rfc3339(),
                ],
            )
            .map_err(|err| MvError::Storage(format!("admit consumer inbox event: {err}")))?;
        let inbox_sequence = transaction.last_insert_rowid();
        let inbox_sequence = u64::try_from(inbox_sequence)
            .map_err(|_| MvError::Storage("consumer inbox sequence is invalid".into()))?;
        let admission = ConsumerInboxAdmission {
            inbox_sequence,
            consumer: consumer.clone(),
            event: event.clone(),
            received_at,
            replayed: false,
        };
        admission.validate().map_err(MvError::InvalidInput)?;
        transaction
            .commit()
            .map_err(|err| MvError::Storage(format!("commit consumer inbox admission: {err}")))?;
        Ok(admission)
    }

    async fn get_consumer_inbox_status(
        &self,
        consumer: &StableUri,
        event_id: Uuid,
    ) -> MvResult<Option<ConsumerInboxStatus>> {
        self.with_conn(|connection| {
            connection
                .query_row(
                    "SELECT inbox_sequence, consumer_uri, event_id, source_uri, state,
                            attempts, next_attempt_at, lease_expires_at, applied_at,
                            last_error_code, received_at, updated_at
                     FROM interoperability_consumer_inbox
                     WHERE consumer_uri = ?1 AND event_id = ?2",
                    params![consumer.as_str(), event_id.to_string()],
                    |row| {
                        let consumer_uri: String = row.get(1)?;
                        let stored_event_id: String = row.get(2)?;
                        let source_uri: String = row.get(3)?;
                        let state: String = row.get(4)?;
                        let next_attempt_at: String = row.get(6)?;
                        let lease_expires_at: Option<String> = row.get(7)?;
                        let applied_at: Option<String> = row.get(8)?;
                        let received_at: String = row.get(10)?;
                        let updated_at: String = row.get(11)?;
                        Ok(ConsumerInboxStatus {
                            inbox_sequence: row.get(0)?,
                            consumer: StableUri::parse(consumer_uri)
                                .map_err(|err| Self::as_sql_conversion_error(1, err))?,
                            event_id: parse_uuid_str(2, &stored_event_id)?,
                            source: StableUri::parse(source_uri)
                                .map_err(|err| Self::as_sql_conversion_error(3, err))?,
                            state: state
                                .parse()
                                .map_err(|err: String| Self::as_sql_conversion_error(4, err))?,
                            attempts: row.get(5)?,
                            next_attempt_at: parse_dt_strict(6, &next_attempt_at)?,
                            lease_expires_at: parse_optional_dt_strict(7, lease_expires_at)?,
                            applied_at: parse_optional_dt_strict(8, applied_at)?,
                            last_error_code: row.get(9)?,
                            received_at: parse_dt_strict(10, &received_at)?,
                            updated_at: parse_dt_strict(11, &updated_at)?,
                        })
                    },
                )
                .optional()
                .map_err(|err| MvError::Storage(format!("load consumer inbox status: {err}")))
        })
    }

    async fn claim_consumer_events(
        &self,
        consumer: &StableUri,
        processor: &StableUri,
        claimed_at: chrono::DateTime<Utc>,
        lease_expires_at: chrono::DateTime<Utc>,
        limit: usize,
    ) -> MvResult<Vec<ConsumerInboxClaim>> {
        if !(1..=1000).contains(&limit) {
            return Err(MvError::InvalidInput(
                "consumer inbox claim limit must be between 1 and 1000".into(),
            ));
        }
        if lease_expires_at <= claimed_at
            || lease_expires_at - claimed_at > chrono::Duration::hours(1)
        {
            return Err(MvError::InvalidInput(
                "consumer inbox lease must be positive and no longer than one hour".into(),
            ));
        }

        let mut connection = self
            .conn()
            .lock()
            .map_err(|err| MvError::Storage(err.to_string()))?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|err| MvError::Storage(format!("begin consumer inbox claim: {err}")))?;
        let candidates = {
            let mut statement = transaction
                .prepare(
                    "SELECT candidate.inbox_sequence, candidate.attempts
                     FROM interoperability_consumer_inbox candidate
                     WHERE candidate.consumer_uri = ?1
                       AND candidate.state = 'pending'
                       AND candidate.next_attempt_at <= ?2
                       AND (
                           candidate.lease_id IS NULL
                           OR candidate.lease_expires_at <= ?2
                       )
                       AND NOT EXISTS (
                           SELECT 1
                           FROM interoperability_consumer_inbox predecessor
                           WHERE predecessor.consumer_uri = candidate.consumer_uri
                             AND predecessor.source_uri = candidate.source_uri
                             AND predecessor.inbox_sequence < candidate.inbox_sequence
                             AND predecessor.state = 'pending'
                       )
                     ORDER BY candidate.inbox_sequence ASC
                     LIMIT ?3",
                )
                .map_err(|err| MvError::Storage(format!("prepare consumer inbox claim: {err}")))?;
            let rows = statement
                .query_map(
                    params![consumer.as_str(), claimed_at.to_rfc3339(), limit as i64],
                    |row| Ok((row.get::<_, u64>(0)?, row.get::<_, u32>(1)?)),
                )
                .map_err(|err| MvError::Storage(format!("query consumer inbox claim: {err}")))?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|err| MvError::Storage(format!("read consumer inbox claim: {err}")))?
        };

        let mut claims = Vec::with_capacity(candidates.len());
        for (inbox_sequence, previous_attempts) in candidates {
            let admission = self
                .load_consumer_inbox_admission_by_sequence(&transaction, inbox_sequence)?
                .ok_or_else(|| {
                    MvError::Storage("consumer inbox candidate disappeared during claim".into())
                })?;
            if admission.consumer != *consumer {
                return Err(MvError::Storage(
                    "consumer inbox candidate belongs to another consumer".into(),
                ));
            }
            let attempt = previous_attempts.checked_add(1).ok_or_else(|| {
                MvError::Storage("consumer application attempt counter overflow".into())
            })?;
            let lease_id = Uuid::now_v7();
            let updated = transaction
                .execute(
                    "UPDATE interoperability_consumer_inbox
                     SET attempts = ?2, lease_id = ?3, lease_processor_uri = ?4,
                         lease_expires_at = ?5, last_attempt_at = ?6, updated_at = ?6
                     WHERE inbox_sequence = ?1
                       AND consumer_uri = ?7
                       AND state = 'pending'
                       AND attempts = ?8
                       AND next_attempt_at <= ?6
                       AND (lease_id IS NULL OR lease_expires_at <= ?6)
                       AND NOT EXISTS (
                           SELECT 1
                           FROM interoperability_consumer_inbox predecessor
                           WHERE predecessor.consumer_uri =
                                     interoperability_consumer_inbox.consumer_uri
                             AND predecessor.source_uri =
                                     interoperability_consumer_inbox.source_uri
                             AND predecessor.inbox_sequence <
                                     interoperability_consumer_inbox.inbox_sequence
                             AND predecessor.state = 'pending'
                       )",
                    params![
                        inbox_sequence,
                        attempt,
                        lease_id.to_string(),
                        processor.as_str(),
                        lease_expires_at.to_rfc3339(),
                        claimed_at.to_rfc3339(),
                        consumer.as_str(),
                        previous_attempts,
                    ],
                )
                .map_err(|err| MvError::Storage(format!("claim consumer inbox event: {err}")))?;
            if updated != 1 {
                return Err(MvError::Storage(
                    "consumer inbox event changed while its claim was being committed".into(),
                ));
            }
            let claim = ConsumerInboxClaim {
                inbox_sequence,
                consumer: consumer.clone(),
                event: admission.event,
                lease_id,
                attempt,
                processor: processor.clone(),
                claimed_at,
                lease_expires_at,
            };
            claim.validate().map_err(MvError::InvalidInput)?;
            claims.push(claim);
        }
        transaction
            .commit()
            .map_err(|err| MvError::Storage(format!("commit consumer inbox claims: {err}")))?;
        Ok(claims)
    }

    async fn complete_consumer_event(
        &self,
        claim: &ConsumerInboxClaim,
        completion: &ConsumerApplicationCompletion,
    ) -> MvResult<ConsumerApplicationReceipt> {
        completion
            .validate_for(claim)
            .map_err(MvError::InvalidInput)?;

        let mut connection = self
            .conn()
            .lock()
            .map_err(|err| MvError::Storage(err.to_string()))?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|err| MvError::Storage(format!("begin consumer completion: {err}")))?;

        if let Some(existing) = self.load_consumer_application_receipt_by_attempt(
            &transaction,
            claim.inbox_sequence,
            claim.attempt,
        )? {
            if !existing.matches_application(claim, completion) {
                return Err(MvError::IdempotencyConflict(
                    "consumer attempt already has a different application receipt".into(),
                ));
            }
            transaction.commit().map_err(|err| {
                MvError::Storage(format!("finish consumer receipt replay: {err}"))
            })?;
            return Ok(existing);
        }

        let stored_claim: Option<(String, u32, String, String, String, String)> = transaction
            .query_row(
                "SELECT state, attempts, lease_id, lease_processor_uri, last_attempt_at,
                        lease_expires_at
                 FROM interoperability_consumer_inbox
                 WHERE inbox_sequence = ?1",
                params![claim.inbox_sequence],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                },
            )
            .optional()
            .map_err(|err| MvError::Storage(format!("load active consumer claim: {err}")))?;
        let Some((state, attempt, lease_id, processor, started_at, lease_expires_at)) =
            stored_claim
        else {
            return Err(MvError::InvalidInput(format!(
                "consumer inbox sequence not found: {}",
                claim.inbox_sequence
            )));
        };
        let stored = self
            .load_consumer_inbox_admission_by_sequence(&transaction, claim.inbox_sequence)?
            .ok_or_else(|| MvError::Storage("consumer inbox event disappeared".into()))?;
        if state != ConsumerInboxState::Pending.as_str()
            || attempt != claim.attempt
            || lease_id != claim.lease_id.to_string()
            || processor != claim.processor.as_str()
            || started_at != claim.claimed_at.to_rfc3339()
            || lease_expires_at != claim.lease_expires_at.to_rfc3339()
            || stored.consumer != claim.consumer
            || stored.event != claim.event
        {
            return Err(MvError::IdempotencyConflict(
                "consumer completion references a stale or different claim".into(),
            ));
        }

        let receipt = ConsumerApplicationReceipt::from_application(claim, completion)
            .map_err(MvError::InvalidInput)?;
        let (payload, payload_format, wrapped_dek) =
            self.encode_governance_record(&receipt, "consumer application receipt")?;
        transaction
            .execute(
                "INSERT INTO interoperability_consumer_application_receipts
                 (receipt_id, receipt_version, inbox_sequence, event_id, claim_id,
                  attempt_no, outcome, consumer_uri, processor_uri, source_uri,
                  subject_uri, principal_uri, actor_uri, correlation_id, request_digest,
                  started_at, completed_at, sensitivity, retention, payload,
                  payload_format, payload_wrapped_dek, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                         ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?17)",
                params![
                    receipt.receipt_id.to_string(),
                    &receipt.receipt_version,
                    receipt.inbox_sequence,
                    receipt.event_id.to_string(),
                    claim.lease_id.to_string(),
                    receipt.attempt,
                    receipt.outcome.as_str(),
                    receipt.consumer.as_str(),
                    receipt.processor.as_str(),
                    receipt.source.as_str(),
                    receipt.subject.as_str(),
                    receipt.principal.as_str(),
                    receipt.actor.as_str(),
                    receipt.correlation_id.to_string(),
                    &receipt.request_digest,
                    receipt.started_at.to_rfc3339(),
                    receipt.completed_at.to_rfc3339(),
                    receipt.sensitivity.as_str(),
                    receipt.retention.as_str(),
                    payload,
                    payload_format,
                    wrapped_dek,
                ],
            )
            .map_err(|err| {
                MvError::Storage(format!("insert consumer application receipt: {err}"))
            })?;

        if matches!(
            completion.result,
            ConsumerApplicationResult::Applied { .. }
                | ConsumerApplicationResult::DeadLettered { .. }
        ) {
            let existing_checkpoint =
                Self::load_consumer_checkpoint(&transaction, &claim.consumer, &claim.event.source)?;
            let is_applied = matches!(completion.result, ConsumerApplicationResult::Applied { .. });
            let (last_applied_sequence, last_applied_event_id, applied_count, dead_letter_count) =
                match existing_checkpoint.as_ref() {
                    Some(checkpoint) => (
                        if is_applied {
                            Some(claim.inbox_sequence)
                        } else {
                            checkpoint.last_applied_sequence
                        },
                        if is_applied {
                            Some(claim.event.id)
                        } else {
                            checkpoint.last_applied_event_id
                        },
                        checkpoint
                            .applied_count
                            .checked_add(u64::from(is_applied))
                            .ok_or_else(|| {
                                MvError::Storage("consumer applied counter overflow".into())
                            })?,
                        checkpoint
                            .dead_letter_count
                            .checked_add(u64::from(!is_applied))
                            .ok_or_else(|| {
                                MvError::Storage("consumer dead-letter counter overflow".into())
                            })?,
                    ),
                    None => (
                        is_applied.then_some(claim.inbox_sequence),
                        is_applied.then_some(claim.event.id),
                        u64::from(is_applied),
                        u64::from(!is_applied),
                    ),
                };
            let checkpoint_params = params![
                claim.consumer.as_str(),
                claim.event.source.as_str(),
                claim.inbox_sequence,
                claim.event.id.to_string(),
                last_applied_sequence,
                last_applied_event_id.map(|event_id| event_id.to_string()),
                applied_count,
                dead_letter_count,
                completion.completed_at.to_rfc3339(),
            ];
            if existing_checkpoint.is_some() {
                transaction.execute(
                    "UPDATE interoperability_consumer_checkpoints
                     SET last_dispositioned_sequence = ?3,
                         last_dispositioned_event_id = ?4,
                         last_applied_sequence = ?5,
                         last_applied_event_id = ?6,
                         applied_count = ?7,
                         dead_letter_count = ?8,
                         updated_at = ?9
                     WHERE consumer_uri = ?1 AND source_uri = ?2",
                    checkpoint_params,
                )
            } else {
                transaction.execute(
                    "INSERT INTO interoperability_consumer_checkpoints
                     (consumer_uri, source_uri, last_dispositioned_sequence,
                      last_dispositioned_event_id, last_applied_sequence,
                      last_applied_event_id, applied_count, dead_letter_count, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    checkpoint_params,
                )
            }
            .map_err(|err| MvError::Storage(format!("advance consumer checkpoint: {err}")))?;
        }

        let updated = match &completion.result {
            ConsumerApplicationResult::RetryScheduled {
                retry_at,
                error_code,
                ..
            } => transaction.execute(
                "UPDATE interoperability_consumer_inbox
                 SET state = 'pending', next_attempt_at = ?2, applied_at = NULL,
                     last_error_code = ?3, lease_id = NULL,
                     lease_processor_uri = NULL, lease_expires_at = NULL,
                     updated_at = ?4
                 WHERE inbox_sequence = ?1 AND state = 'pending'
                   AND attempts = ?5 AND lease_id = ?6",
                params![
                    claim.inbox_sequence,
                    retry_at.to_rfc3339(),
                    error_code,
                    completion.completed_at.to_rfc3339(),
                    claim.attempt,
                    claim.lease_id.to_string(),
                ],
            ),
            ConsumerApplicationResult::Applied { .. } => transaction.execute(
                "UPDATE interoperability_consumer_inbox
                 SET state = 'applied', applied_at = ?2, last_error_code = NULL,
                     lease_id = NULL, lease_processor_uri = NULL,
                     lease_expires_at = NULL, updated_at = ?2
                 WHERE inbox_sequence = ?1 AND state = 'pending'
                   AND attempts = ?3 AND lease_id = ?4",
                params![
                    claim.inbox_sequence,
                    completion.completed_at.to_rfc3339(),
                    claim.attempt,
                    claim.lease_id.to_string(),
                ],
            ),
            ConsumerApplicationResult::DeadLettered { error_code, .. } => transaction.execute(
                "UPDATE interoperability_consumer_inbox
                 SET state = 'dead_letter', applied_at = NULL, last_error_code = ?2,
                     lease_id = NULL, lease_processor_uri = NULL,
                     lease_expires_at = NULL, updated_at = ?3
                 WHERE inbox_sequence = ?1 AND state = 'pending'
                   AND attempts = ?4 AND lease_id = ?5",
                params![
                    claim.inbox_sequence,
                    error_code,
                    completion.completed_at.to_rfc3339(),
                    claim.attempt,
                    claim.lease_id.to_string(),
                ],
            ),
        }
        .map_err(|err| MvError::Storage(format!("complete consumer inbox event: {err}")))?;
        if updated != 1 {
            return Err(MvError::IdempotencyConflict(
                "consumer inbox claim changed before completion".into(),
            ));
        }
        transaction
            .commit()
            .map_err(|err| MvError::Storage(format!("commit consumer completion: {err}")))?;
        Ok(receipt)
    }

    async fn get_consumer_checkpoint(
        &self,
        consumer: &StableUri,
        source: &StableUri,
    ) -> MvResult<Option<ConsumerCheckpoint>> {
        self.with_conn(|connection| Self::load_consumer_checkpoint(connection, consumer, source))
    }

    async fn get_consumer_application_receipt(
        &self,
        receipt_id: Uuid,
    ) -> MvResult<Option<ConsumerApplicationReceipt>> {
        self.with_conn(|connection| {
            connection
                .query_row(
                    "SELECT receipt_id, receipt_version, inbox_sequence, event_id,
                            claim_id, attempt_no, outcome, consumer_uri, processor_uri, source_uri,
                            subject_uri, principal_uri, actor_uri, correlation_id,
                            request_digest, started_at, completed_at, sensitivity,
                            retention, payload, payload_format, payload_wrapped_dek
                     FROM interoperability_consumer_application_receipts
                     WHERE receipt_id = ?1",
                    params![receipt_id.to_string()],
                    |row| self.row_to_consumer_application_receipt(row),
                )
                .optional()
                .map_err(|err| {
                    MvError::Storage(format!("load consumer application receipt: {err}"))
                })
        })
    }

    async fn list_consumer_application_receipts(
        &self,
        consumer: &StableUri,
        event_id: Uuid,
        limit: usize,
    ) -> MvResult<Vec<ConsumerApplicationReceipt>> {
        if !(1..=1000).contains(&limit) {
            return Err(MvError::InvalidInput(
                "consumer receipt limit must be between 1 and 1000".into(),
            ));
        }
        self.with_conn(|connection| {
            let mut statement = connection
                .prepare(
                    "SELECT receipt_id, receipt_version, inbox_sequence, event_id,
                            claim_id, attempt_no, outcome, consumer_uri, processor_uri, source_uri,
                            subject_uri, principal_uri, actor_uri, correlation_id,
                            request_digest, started_at, completed_at, sensitivity,
                            retention, payload, payload_format, payload_wrapped_dek
                     FROM interoperability_consumer_application_receipts
                     WHERE consumer_uri = ?1 AND event_id = ?2
                     ORDER BY attempt_no ASC
                     LIMIT ?3",
                )
                .map_err(|err| {
                    MvError::Storage(format!("prepare consumer application receipts: {err}"))
                })?;
            let rows = statement
                .query_map(
                    params![consumer.as_str(), event_id.to_string(), limit as i64],
                    |row| self.row_to_consumer_application_receipt(row),
                )
                .map_err(|err| {
                    MvError::Storage(format!("query consumer application receipts: {err}"))
                })?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|err| MvError::Storage(format!("read consumer receipt: {err}")))
        })
    }

    // -----------------------------------------------------------------------
    // Governed agent execution graph
    // -----------------------------------------------------------------------

    async fn commit_work_order_with_event(
        &self,
        work_order: &WorkOrder,
        nodes: &[WorkOrderNode],
        edges: &[WorkOrderEdge],
        event: &EventEnvelope,
    ) -> MvResult<IdempotentWorkOrderCommit> {
        work_order.validate().map_err(MvError::InvalidInput)?;
        if nodes.is_empty() {
            return Err(MvError::InvalidInput(
                "a work order must contain at least one node contract".into(),
            ));
        }
        for node in nodes {
            node.validate().map_err(MvError::InvalidInput)?;
            if node.work_order_id != work_order.work_order_id {
                return Err(MvError::InvalidInput(
                    "node contracts must belong to the committed work order".into(),
                ));
            }
        }
        for edge in edges {
            edge.validate().map_err(MvError::InvalidInput)?;
            if edge.work_order_id != work_order.work_order_id {
                return Err(MvError::InvalidInput(
                    "edges must belong to the committed work order".into(),
                ));
            }
        }

        // Every intersecting write scope must carry a derived conflict edge.
        // Recomputing here means an admission path that forgot to derive them
        // fails closed instead of admitting an unguarded overlap.
        let expected =
            derive_conflict_edges(work_order.work_order_id, nodes, work_order.created_at);
        for required in &expected {
            let present = edges.iter().any(|edge| {
                edge.kind == EdgeKind::Conflict
                    && ((edge.from_node_id == required.from_node_id
                        && edge.to_node_id == required.to_node_id)
                        || (edge.from_node_id == required.to_node_id
                            && edge.to_node_id == required.from_node_id))
            });
            if !present {
                return Err(MvError::InvalidInput(
                    "intersecting write scopes require a derived conflict edge".into(),
                ));
            }
        }
        if let Some(cycle) = find_dependency_cycle(nodes, edges) {
            return Err(MvError::InvalidInput(format!(
                "work-order dependency graph contains a cycle across {} contracts",
                cycle.len()
            )));
        }

        let mut connection = self
            .conn()
            .lock()
            .map_err(|err| MvError::Storage(err.to_string()))?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|err| MvError::Storage(format!("begin work-order admission: {err}")))?;
        let local_node_id = Self::ensure_local_context_node(&transaction)?;
        Self::validate_governance_event(
            &transaction,
            event,
            local_node_id,
            WORK_ORDER_ADMITTED_V1,
            "work-order-admitted",
        )?;
        if work_order.governing_node != StableUri::node(local_node_id) {
            return Err(MvError::InvalidInput(
                "work orders must be governed by the local Context Node".into(),
            ));
        }

        if let Some(existing_event) =
            Self::resolve_governance_replay(&transaction, event, WORK_ORDER_ADMITTED_V1)?
        {
            let existing_id = existing_event
                .subject
                .trailing_uuid()
                .ok_or_else(|| MvError::Storage("work-order replay lost its subject".into()))?;
            let existing = self
                .load_work_order_from_connection(&transaction, existing_id)?
                .ok_or_else(|| {
                    MvError::Storage("work-order replay references a missing record".into())
                })?;
            transaction
                .commit()
                .map_err(|err| MvError::Storage(format!("finish work-order replay: {err}")))?;
            return Ok(IdempotentWorkOrderCommit {
                work_order: existing,
                event: existing_event,
                replayed: true,
            });
        }

        let (payload, format, wrapped_dek) =
            self.encode_governance_record(work_order, "work order")?;
        transaction
            .execute(
                "INSERT INTO work_orders
                 (work_order_id, revision, work_order_uri, principal_uri, actor_uri,
                  governing_node_uri, status, status_reason, sensitivity, retention,
                  correlation_id, causation_id, idempotency_key,
                  budget_wall_clock_secs, budget_run_attempts, budget_model_tokens,
                  budget_effect_actions, remaining_wall_clock_secs, remaining_run_attempts,
                  remaining_model_tokens, remaining_effect_actions,
                  record_payload, payload_format, payload_wrapped_dek, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                         ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26)",
                params![
                    work_order.work_order_id.to_string(),
                    work_order.revision,
                    work_order.work_order_uri.as_str(),
                    work_order.principal.as_str(),
                    work_order.actor.as_str(),
                    work_order.governing_node.as_str(),
                    work_order.status.as_str(),
                    work_order.status_reason,
                    work_order.sensitivity.as_str(),
                    work_order.retention.as_str(),
                    work_order.correlation_id.to_string(),
                    work_order.causation_id.map(|id| id.to_string()),
                    work_order.idempotency_key,
                    work_order.budget.wall_clock_secs as i64,
                    work_order.budget.run_attempts as i64,
                    work_order.budget.model_tokens as i64,
                    work_order.budget.effect_actions as i64,
                    work_order.remaining.wall_clock_secs as i64,
                    work_order.remaining.run_attempts as i64,
                    work_order.remaining.model_tokens as i64,
                    work_order.remaining.effect_actions as i64,
                    payload,
                    format,
                    wrapped_dek,
                    work_order.created_at.to_rfc3339(),
                    work_order.updated_at.to_rfc3339(),
                ],
            )
            .map_err(|err| MvError::Storage(format!("insert work order: {err}")))?;

        for node in nodes {
            let (node_payload, node_format, node_dek) =
                self.encode_governance_record(node, "work-order node")?;
            transaction
                .execute(
                    "INSERT INTO work_order_nodes
                     (node_id, work_order_id, node_uri, executor_kind, risk_tier, status,
                      timeout_secs, max_attempts, authorizing_grant_id,
                      record_payload, payload_format, payload_wrapped_dek, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                    params![
                        node.node_id.to_string(),
                        node.work_order_id.to_string(),
                        node.node_uri.as_str(),
                        node.executor_kind.as_str(),
                        node.risk_tier.as_str(),
                        node.status.as_str(),
                        node.timeout_secs,
                        node.max_attempts,
                        node.authorizing_grant_id.map(|id| id.to_string()),
                        node_payload,
                        node_format,
                        node_dek,
                        node.created_at.to_rfc3339(),
                        node.updated_at.to_rfc3339(),
                    ],
                )
                .map_err(|err| MvError::Storage(format!("insert work-order node: {err}")))?;

            // Digests, never plaintext targets: a sealed vault must not expose
            // scope through the conflict or lease indexes.
            for digest in node.write_target_digests() {
                transaction
                    .execute(
                        "INSERT INTO work_order_node_write_targets
                         (node_id, work_order_id, target_digest, created_at)
                         VALUES (?1, ?2, ?3, ?4)",
                        params![
                            node.node_id.to_string(),
                            node.work_order_id.to_string(),
                            digest,
                            node.created_at.to_rfc3339(),
                        ],
                    )
                    .map_err(|err| {
                        MvError::Storage(format!("insert declared write target: {err}"))
                    })?;
            }
        }

        for edge in edges {
            transaction
                .execute(
                    "INSERT INTO work_order_edges
                     (edge_id, work_order_id, from_node_id, to_node_id, edge_kind,
                      derived, detail, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        edge.edge_id.to_string(),
                        edge.work_order_id.to_string(),
                        edge.from_node_id.to_string(),
                        edge.to_node_id.to_string(),
                        edge.kind.as_str(),
                        i64::from(edge.derived),
                        edge.detail,
                        edge.created_at.to_rfc3339(),
                    ],
                )
                .map_err(|err| MvError::Storage(format!("insert work-order edge: {err}")))?;
        }

        Self::insert_outbox_event(&transaction, event)?;
        transaction
            .commit()
            .map_err(|err| MvError::Storage(format!("commit work-order admission: {err}")))?;

        Ok(IdempotentWorkOrderCommit {
            work_order: work_order.clone(),
            event: event.clone(),
            replayed: false,
        })
    }

    async fn get_work_order(&self, work_order_id: Uuid) -> MvResult<Option<WorkOrder>> {
        self.with_conn(|connection| self.load_work_order_from_connection(connection, work_order_id))
    }

    async fn list_work_orders(
        &self,
        status: Option<WorkOrderStatus>,
        limit: usize,
    ) -> MvResult<Vec<WorkOrder>> {
        if !(1..=1000).contains(&limit) {
            return Err(MvError::InvalidInput(
                "work-order limit must be between 1 and 1000".into(),
            ));
        }
        self.with_conn(|connection| {
            let mut statement = connection
                .prepare(
                    "SELECT work_order_id, revision, work_order_uri, principal_uri, actor_uri,
                            governing_node_uri, status, record_payload, payload_format,
                            payload_wrapped_dek, created_at, updated_at
                     FROM work_orders
                     WHERE (?1 IS NULL OR status = ?1)
                     ORDER BY updated_at DESC, work_order_id ASC
                     LIMIT ?2",
                )
                .map_err(|err| MvError::Storage(format!("prepare work-order list: {err}")))?;
            let rows = statement
                .query_map(
                    params![status.map(|value| value.as_str()), limit as i64],
                    |row| self.row_to_work_order(row),
                )
                .map_err(|err| MvError::Storage(format!("query work orders: {err}")))?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|err| MvError::Storage(format!("read work order: {err}")))
        })
    }

    async fn list_work_order_nodes(&self, work_order_id: Uuid) -> MvResult<Vec<WorkOrderNode>> {
        self.with_conn(|connection| {
            let mut statement = connection
                .prepare(
                    "SELECT record_payload, payload_format, payload_wrapped_dek
                     FROM work_order_nodes
                     WHERE work_order_id = ?1
                     ORDER BY created_at ASC, node_id ASC",
                )
                .map_err(|err| MvError::Storage(format!("prepare node list: {err}")))?;
            let rows = statement
                .query_map(params![work_order_id.to_string()], |row| {
                    let payload: Vec<u8> = row.get(0)?;
                    let format: String = row.get(1)?;
                    let dek: Option<String> = row.get(2)?;
                    self.decode_governance_record::<WorkOrderNode>(
                        &payload,
                        &format,
                        dek.as_deref(),
                        "work-order node",
                    )
                    .map_err(|err| Self::as_sql_conversion_error(0, err.to_string()))
                })
                .map_err(|err| MvError::Storage(format!("query work-order nodes: {err}")))?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|err| MvError::Storage(format!("read work-order node: {err}")))
        })
    }

    async fn list_work_order_edges(&self, work_order_id: Uuid) -> MvResult<Vec<WorkOrderEdge>> {
        self.with_conn(|connection| {
            let mut statement = connection
                .prepare(
                    "SELECT edge_id, work_order_id, from_node_id, to_node_id, edge_kind,
                            derived, detail, created_at
                     FROM work_order_edges
                     WHERE work_order_id = ?1
                     ORDER BY created_at ASC, edge_id ASC",
                )
                .map_err(|err| MvError::Storage(format!("prepare edge list: {err}")))?;
            let rows = statement
                .query_map(params![work_order_id.to_string()], |row| {
                    let kind_text: String = row.get(4)?;
                    let derived: i64 = row.get(5)?;
                    Ok(WorkOrderEdge {
                        edge_id: parse_uuid_str(0, &row.get::<_, String>(0)?)?,
                        work_order_id: parse_uuid_str(1, &row.get::<_, String>(1)?)?,
                        from_node_id: parse_uuid_str(2, &row.get::<_, String>(2)?)?,
                        to_node_id: parse_uuid_str(3, &row.get::<_, String>(3)?)?,
                        kind: kind_text
                            .parse()
                            .map_err(|err: String| Self::as_sql_conversion_error(4, err))?,
                        derived: derived != 0,
                        detail: row.get(6)?,
                        created_at: parse_dt_strict(7, &row.get::<_, String>(7)?)?,
                    })
                })
                .map_err(|err| MvError::Storage(format!("query work-order edges: {err}")))?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|err| MvError::Storage(format!("read work-order edge: {err}")))
        })
    }

    async fn transition_work_order_with_event(
        &self,
        expected_revision: u64,
        replacement: &WorkOrder,
        event: &EventEnvelope,
    ) -> MvResult<IdempotentWorkOrderCommit> {
        replacement.validate().map_err(MvError::InvalidInput)?;
        if replacement.revision != expected_revision.saturating_add(1) {
            return Err(MvError::InvalidInput(
                "replacement work-order revision must advance exactly once".into(),
            ));
        }

        let mut connection = self
            .conn()
            .lock()
            .map_err(|err| MvError::Storage(err.to_string()))?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|err| MvError::Storage(format!("begin work-order transition: {err}")))?;
        let local_node_id = Self::ensure_local_context_node(&transaction)?;
        Self::validate_governance_event(
            &transaction,
            event,
            local_node_id,
            WORK_ORDER_LIFECYCLE_TRANSITIONED_V1,
            "work-order-lifecycle-transitioned",
        )?;

        if let Some(existing_event) = Self::resolve_governance_replay(
            &transaction,
            event,
            WORK_ORDER_LIFECYCLE_TRANSITIONED_V1,
        )? {
            let existing = self
                .load_work_order_from_connection(&transaction, replacement.work_order_id)?
                .ok_or_else(|| {
                    MvError::Storage("work-order replay references a missing record".into())
                })?;
            transaction
                .commit()
                .map_err(|err| MvError::Storage(format!("finish work-order replay: {err}")))?;
            return Ok(IdempotentWorkOrderCommit {
                work_order: existing,
                event: existing_event,
                replayed: true,
            });
        }

        let current = self
            .load_work_order_from_connection(&transaction, replacement.work_order_id)?
            .ok_or_else(|| MvError::NotFound("work order not found".into()))?;
        if current.revision != expected_revision {
            return Err(MvError::IdempotencyConflict(
                "work order changed since the expected revision".into(),
            ));
        }
        if current.status != replacement.status
            && !current.status.can_transition_to(replacement.status)
        {
            return Err(MvError::InvalidInput(format!(
                "invalid work-order transition from {} to {}",
                current.status.as_str(),
                replacement.status.as_str()
            )));
        }

        Self::archive_work_order_revision(&transaction, &current, replacement.updated_at)?;
        self.write_work_order_revision(&transaction, replacement)?;
        Self::insert_outbox_event(&transaction, event)?;
        transaction
            .commit()
            .map_err(|err| MvError::Storage(format!("commit work-order transition: {err}")))?;

        Ok(IdempotentWorkOrderCommit {
            work_order: replacement.clone(),
            event: event.clone(),
            replayed: false,
        })
    }

    async fn commit_agent_run_with_event(
        &self,
        run: &AgentRun,
        spend: &WorkOrderSpend,
        event: &EventEnvelope,
    ) -> MvResult<IdempotentAgentRunCommit> {
        run.validate().map_err(MvError::InvalidInput)?;
        if run.status != AgentRunStatus::Ready {
            return Err(MvError::InvalidInput(
                "a new agent run starts in the ready state".into(),
            ));
        }

        let mut connection = self
            .conn()
            .lock()
            .map_err(|err| MvError::Storage(err.to_string()))?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|err| MvError::Storage(format!("begin agent-run start: {err}")))?;
        let local_node_id = Self::ensure_local_context_node(&transaction)?;
        Self::validate_governance_event(
            &transaction,
            event,
            local_node_id,
            AGENT_RUN_STARTED_V1,
            "agent-run-started",
        )?;

        if let Some(existing_event) =
            Self::resolve_governance_replay(&transaction, event, AGENT_RUN_STARTED_V1)?
        {
            let existing = self
                .load_agent_run_from_connection(&transaction, run.run_id)?
                .ok_or_else(|| {
                    MvError::Storage("agent-run replay references a missing record".into())
                })?;
            transaction
                .commit()
                .map_err(|err| MvError::Storage(format!("finish agent-run replay: {err}")))?;
            return Ok(IdempotentAgentRunCommit {
                run: existing,
                event: existing_event,
                replayed: true,
            });
        }

        // Budget decrement, run insertion, and event emission share this one
        // immediate transaction. The CHECK constraint prevents a negative row;
        // only this shared transaction prevents a double spend.
        let work_order = self
            .load_work_order_from_connection(&transaction, run.work_order_id)?
            .ok_or_else(|| MvError::NotFound("work order not found".into()))?;
        let remaining = work_order
            .remaining
            .checked_spend(spend)
            .map_err(MvError::InvalidInput)?;
        let mut spent = work_order.clone();
        spent.remaining = remaining;
        spent.revision = work_order.revision.saturating_add(1);
        spent.updated_at = run.created_at;
        Self::archive_work_order_revision(&transaction, &work_order, run.created_at)?;
        self.write_work_order_revision(&transaction, &spent)?;

        let (payload, format, wrapped_dek) = self.encode_governance_record(run, "agent run")?;
        transaction
            .execute(
                "INSERT INTO agent_runs
                 (run_id, run_uri, work_order_id, node_id, attempt_no, status, failure_class,
                  principal_uri, actor_uri, correlation_id, causation_id, started_at, ended_at,
                  record_payload, payload_format, payload_wrapped_dek, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
                params![
                    run.run_id.to_string(),
                    run.run_uri.as_str(),
                    run.work_order_id.to_string(),
                    run.node_id.to_string(),
                    run.attempt_no,
                    run.status.as_str(),
                    run.failure_class.map(|class| class.as_str()),
                    run.principal.as_str(),
                    run.actor.as_str(),
                    run.correlation_id.to_string(),
                    run.causation_id.map(|id| id.to_string()),
                    run.started_at.map(|at| at.to_rfc3339()),
                    run.ended_at.map(|at| at.to_rfc3339()),
                    payload,
                    format,
                    wrapped_dek,
                    run.created_at.to_rfc3339(),
                    run.updated_at.to_rfc3339(),
                ],
            )
            .map_err(|err| MvError::Storage(format!("insert agent run: {err}")))?;

        Self::insert_outbox_event(&transaction, event)?;
        transaction
            .commit()
            .map_err(|err| MvError::Storage(format!("commit agent-run start: {err}")))?;

        Ok(IdempotentAgentRunCommit {
            run: run.clone(),
            event: event.clone(),
            replayed: false,
        })
    }

    async fn get_agent_run(&self, run_id: Uuid) -> MvResult<Option<AgentRun>> {
        self.with_conn(|connection| self.load_agent_run_from_connection(connection, run_id))
    }

    async fn list_agent_runs(&self, work_order_id: Uuid, limit: usize) -> MvResult<Vec<AgentRun>> {
        if !(1..=1000).contains(&limit) {
            return Err(MvError::InvalidInput(
                "agent-run limit must be between 1 and 1000".into(),
            ));
        }
        self.with_conn(|connection| {
            let mut statement = connection
                .prepare(
                    "SELECT record_payload, payload_format, payload_wrapped_dek
                     FROM agent_runs
                     WHERE work_order_id = ?1
                     ORDER BY created_at ASC, run_id ASC
                     LIMIT ?2",
                )
                .map_err(|err| MvError::Storage(format!("prepare agent-run list: {err}")))?;
            let rows = statement
                .query_map(params![work_order_id.to_string(), limit as i64], |row| {
                    let payload: Vec<u8> = row.get(0)?;
                    let format: String = row.get(1)?;
                    let dek: Option<String> = row.get(2)?;
                    self.decode_governance_record::<AgentRun>(
                        &payload,
                        &format,
                        dek.as_deref(),
                        "agent run",
                    )
                    .map_err(|err| Self::as_sql_conversion_error(0, err.to_string()))
                })
                .map_err(|err| MvError::Storage(format!("query agent runs: {err}")))?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|err| MvError::Storage(format!("read agent run: {err}")))
        })
    }

    async fn transition_agent_run_with_event(
        &self,
        run: &AgentRun,
        release_reason: Option<LeaseReleaseReason>,
        event: &EventEnvelope,
    ) -> MvResult<IdempotentAgentRunCommit> {
        run.validate().map_err(MvError::InvalidInput)?;

        let mut connection = self
            .conn()
            .lock()
            .map_err(|err| MvError::Storage(err.to_string()))?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|err| MvError::Storage(format!("begin agent-run transition: {err}")))?;
        let local_node_id = Self::ensure_local_context_node(&transaction)?;
        Self::validate_governance_event(
            &transaction,
            event,
            local_node_id,
            AGENT_RUN_LIFECYCLE_TRANSITIONED_V1,
            "agent-run-lifecycle-transitioned",
        )?;

        if let Some(existing_event) = Self::resolve_governance_replay(
            &transaction,
            event,
            AGENT_RUN_LIFECYCLE_TRANSITIONED_V1,
        )? {
            let existing = self
                .load_agent_run_from_connection(&transaction, run.run_id)?
                .ok_or_else(|| {
                    MvError::Storage("agent-run replay references a missing record".into())
                })?;
            transaction
                .commit()
                .map_err(|err| MvError::Storage(format!("finish agent-run replay: {err}")))?;
            return Ok(IdempotentAgentRunCommit {
                run: existing,
                event: existing_event,
                replayed: true,
            });
        }

        let current = self
            .load_agent_run_from_connection(&transaction, run.run_id)?
            .ok_or_else(|| MvError::NotFound("agent run not found".into()))?;
        if current.status != run.status && !current.status.can_transition_to(run.status) {
            return Err(MvError::InvalidInput(format!(
                "invalid agent-run transition from {} to {}",
                current.status.as_str(),
                run.status.as_str()
            )));
        }

        // A state that may not hold leases releases them in the same
        // transaction as the transition. Approval is unbounded, so a parked run
        // holding leases would block every other run on those targets.
        if !run.status.may_hold_write_leases() {
            let reason = release_reason.unwrap_or(match run.status {
                AgentRunStatus::AwaitingApproval => LeaseReleaseReason::AwaitingApproval,
                AgentRunStatus::Cancelled => LeaseReleaseReason::Cancelled,
                AgentRunStatus::Completed => LeaseReleaseReason::Completed,
                _ => LeaseReleaseReason::Failed,
            });
            Self::release_run_leases(&transaction, run.run_id, reason, run.updated_at)?;
        }

        let (payload, format, wrapped_dek) = self.encode_governance_record(run, "agent run")?;
        let updated = transaction
            .execute(
                "UPDATE agent_runs
                 SET status = ?2, failure_class = ?3, started_at = ?4, ended_at = ?5,
                     record_payload = ?6, payload_format = ?7, payload_wrapped_dek = ?8,
                     updated_at = ?9
                 WHERE run_id = ?1",
                params![
                    run.run_id.to_string(),
                    run.status.as_str(),
                    run.failure_class.map(|class| class.as_str()),
                    run.started_at.map(|at| at.to_rfc3339()),
                    run.ended_at.map(|at| at.to_rfc3339()),
                    payload,
                    format,
                    wrapped_dek,
                    run.updated_at.to_rfc3339(),
                ],
            )
            .map_err(|err| MvError::Storage(format!("update agent run: {err}")))?;
        if updated != 1 {
            return Err(MvError::Storage(
                "agent run changed while its transition was being committed".into(),
            ));
        }

        Self::insert_outbox_event(&transaction, event)?;
        transaction
            .commit()
            .map_err(|err| MvError::Storage(format!("commit agent-run transition: {err}")))?;

        Ok(IdempotentAgentRunCommit {
            run: run.clone(),
            event: event.clone(),
            replayed: false,
        })
    }

    async fn claim_write_leases(
        &self,
        run_id: Uuid,
        target_digests: &[String],
        claimed_at: DateTime<Utc>,
        lease_expires_at: DateTime<Utc>,
    ) -> MvResult<Vec<WriteLease>> {
        if target_digests.is_empty() {
            return Ok(Vec::new());
        }
        if lease_expires_at <= claimed_at {
            return Err(MvError::InvalidInput(
                "write-lease interval must be non-empty".into(),
            ));
        }
        if lease_expires_at - claimed_at > chrono::Duration::seconds(MAX_WRITE_LEASE_SECS) {
            return Err(MvError::InvalidInput(
                "write leases are bounded to one hour".into(),
            ));
        }

        let mut connection = self
            .conn()
            .lock()
            .map_err(|err| MvError::Storage(err.to_string()))?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|err| MvError::Storage(format!("begin write-lease claim: {err}")))?;

        let run = self
            .load_agent_run_from_connection(&transaction, run_id)?
            .ok_or_else(|| MvError::NotFound("agent run not found".into()))?;
        let work_order = self
            .load_work_order_from_connection(&transaction, run.work_order_id)?
            .ok_or_else(|| MvError::NotFound("work order not found".into()))?;

        let mut leases = Vec::with_capacity(target_digests.len());
        for digest in target_digests {
            // An expired lease is replaced atomically; an unexpired one held by
            // another run blocks the claim, which is the point of the lease.
            transaction
                .execute(
                    "UPDATE agent_run_write_leases
                     SET released_at = ?2, release_reason = 'expired_replaced'
                     WHERE target_digest = ?1
                       AND governing_node_uri = ?3
                       AND released_at IS NULL
                       AND lease_expires_at <= ?2",
                    params![
                        digest,
                        claimed_at.to_rfc3339(),
                        work_order.governing_node.as_str()
                    ],
                )
                .map_err(|err| MvError::Storage(format!("reap expired write lease: {err}")))?;

            let next_attempt: i64 = transaction
                .query_row(
                    "SELECT COALESCE(MAX(attempt_no), 0) + 1
                     FROM agent_run_write_leases
                     WHERE target_digest = ?1 AND governing_node_uri = ?2",
                    params![digest, work_order.governing_node.as_str()],
                    |row| row.get(0),
                )
                .map_err(|err| MvError::Storage(format!("read write-lease attempt: {err}")))?;

            let lease = WriteLease {
                lease_id: Uuid::now_v7(),
                run_id,
                work_order_id: run.work_order_id,
                target_digest: digest.clone(),
                governing_node: work_order.governing_node.clone(),
                attempt_no: next_attempt as u32,
                claimed_at,
                lease_expires_at,
                released_at: None,
                release_reason: None,
                created_at: claimed_at,
            };
            lease.validate().map_err(MvError::InvalidInput)?;

            transaction
                .execute(
                    "INSERT INTO agent_run_write_leases
                     (lease_id, run_id, work_order_id, target_digest, governing_node_uri,
                      attempt_no, claimed_at, lease_expires_at, released_at, release_reason,
                      created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL, NULL, ?9)",
                    params![
                        lease.lease_id.to_string(),
                        lease.run_id.to_string(),
                        lease.work_order_id.to_string(),
                        lease.target_digest,
                        lease.governing_node.as_str(),
                        lease.attempt_no,
                        lease.claimed_at.to_rfc3339(),
                        lease.lease_expires_at.to_rfc3339(),
                        lease.created_at.to_rfc3339(),
                    ],
                )
                .map_err(|err| {
                    // All-or-nothing: a partial claim would let a run begin
                    // writing part of its scope while another holds the rest.
                    MvError::Conflict(format!("write lease unavailable: {err}"))
                })?;
            leases.push(lease);
        }

        transaction
            .commit()
            .map_err(|err| MvError::Storage(format!("commit write-lease claim: {err}")))?;
        Ok(leases)
    }

    async fn release_write_leases(
        &self,
        run_id: Uuid,
        reason: LeaseReleaseReason,
        released_at: DateTime<Utc>,
    ) -> MvResult<usize> {
        let mut connection = self
            .conn()
            .lock()
            .map_err(|err| MvError::Storage(err.to_string()))?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|err| MvError::Storage(format!("begin write-lease release: {err}")))?;
        let released = Self::release_run_leases(&transaction, run_id, reason, released_at)?;
        transaction
            .commit()
            .map_err(|err| MvError::Storage(format!("commit write-lease release: {err}")))?;
        Ok(released)
    }

    async fn list_write_leases(&self, run_id: Uuid) -> MvResult<Vec<WriteLease>> {
        self.with_conn(|connection| {
            let mut statement = connection
                .prepare(
                    "SELECT lease_id, run_id, work_order_id, target_digest, governing_node_uri,
                            attempt_no, claimed_at, lease_expires_at, released_at,
                            release_reason, created_at
                     FROM agent_run_write_leases
                     WHERE run_id = ?1
                     ORDER BY claimed_at ASC, lease_id ASC",
                )
                .map_err(|err| MvError::Storage(format!("prepare lease list: {err}")))?;
            let rows = statement
                .query_map(params![run_id.to_string()], |row| {
                    Self::row_to_write_lease(row)
                })
                .map_err(|err| MvError::Storage(format!("query write leases: {err}")))?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|err| MvError::Storage(format!("read write lease: {err}")))
        })
    }

    async fn conflicting_write_targets(
        &self,
        excluding_run: Uuid,
        target_digests: &[String],
        at: DateTime<Utc>,
    ) -> MvResult<Vec<String>> {
        if target_digests.is_empty() {
            return Ok(Vec::new());
        }
        self.with_conn(|connection| {
            let mut conflicts = Vec::new();
            let mut statement = connection
                .prepare(
                    "SELECT 1 FROM agent_run_write_leases
                     WHERE target_digest = ?1
                       AND run_id != ?2
                       AND released_at IS NULL
                       AND lease_expires_at > ?3
                     LIMIT 1",
                )
                .map_err(|err| MvError::Storage(format!("prepare conflict probe: {err}")))?;
            for digest in target_digests {
                let held = statement
                    .exists(params![digest, excluding_run.to_string(), at.to_rfc3339()])
                    .map_err(|err| MvError::Storage(format!("probe write conflict: {err}")))?;
                if held {
                    conflicts.push(digest.clone());
                }
            }
            Ok(conflicts)
        })
    }

    async fn record_gate_result(&self, result: &GateResult) -> MvResult<GateResult> {
        let mut connection = self
            .conn()
            .lock()
            .map_err(|err| MvError::Storage(err.to_string()))?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|err| MvError::Storage(format!("begin gate result: {err}")))?;

        let run = self
            .load_agent_run_from_connection(&transaction, result.run_id)?
            .ok_or_else(|| MvError::NotFound("agent run not found".into()))?;
        // Checked here as well as by the database trigger: the trigger is the
        // guarantee, this is the readable error.
        result.validate(&run).map_err(MvError::InvalidInput)?;

        transaction
            .execute(
                "INSERT INTO agent_run_gate_results
                 (result_id, run_id, work_order_id, gate, outcome, evaluator_actor_uri,
                  evidence_digest, detail, evaluated_at, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    result.result_id.to_string(),
                    result.run_id.to_string(),
                    result.work_order_id.to_string(),
                    result.gate.as_str(),
                    result.outcome.as_str(),
                    result.evaluator_actor.as_str(),
                    result.evidence_digest,
                    result.detail,
                    result.evaluated_at.to_rfc3339(),
                    result.created_at.to_rfc3339(),
                ],
            )
            .map_err(|err| MvError::Storage(format!("insert gate result: {err}")))?;
        transaction
            .commit()
            .map_err(|err| MvError::Storage(format!("commit gate result: {err}")))?;
        Ok(result.clone())
    }

    async fn list_gate_results(&self, run_id: Uuid) -> MvResult<Vec<GateResult>> {
        self.with_conn(|connection| {
            let mut statement = connection
                .prepare(
                    "SELECT result_id, run_id, work_order_id, gate, outcome,
                            evaluator_actor_uri, evidence_digest, detail, evaluated_at, created_at
                     FROM agent_run_gate_results
                     WHERE run_id = ?1
                     ORDER BY gate ASC",
                )
                .map_err(|err| MvError::Storage(format!("prepare gate list: {err}")))?;
            let rows = statement
                .query_map(params![run_id.to_string()], |row| {
                    let gate_text: String = row.get(3)?;
                    let outcome_text: String = row.get(4)?;
                    Ok(GateResult {
                        result_id: parse_uuid_str(0, &row.get::<_, String>(0)?)?,
                        run_id: parse_uuid_str(1, &row.get::<_, String>(1)?)?,
                        work_order_id: parse_uuid_str(2, &row.get::<_, String>(2)?)?,
                        gate: gate_text
                            .parse()
                            .map_err(|err: String| Self::as_sql_conversion_error(3, err))?,
                        outcome: outcome_text
                            .parse()
                            .map_err(|err: String| Self::as_sql_conversion_error(4, err))?,
                        evaluator_actor: StableUri::parse(row.get::<_, String>(5)?)
                            .map_err(|err| Self::as_sql_conversion_error(5, err))?,
                        evidence_digest: row.get(6)?,
                        detail: row.get(7)?,
                        evaluated_at: parse_dt_strict(8, &row.get::<_, String>(8)?)?,
                        created_at: parse_dt_strict(9, &row.get::<_, String>(9)?)?,
                    })
                })
                .map_err(|err| MvError::Storage(format!("query gate results: {err}")))?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|err| MvError::Storage(format!("read gate result: {err}")))
        })
    }

    async fn record_run_artifact(
        &self,
        artifact: &RunArtifact,
        payload: &[u8],
    ) -> MvResult<RunArtifact> {
        artifact.validate().map_err(MvError::InvalidInput)?;
        // The stored digest must describe the stored bytes, or G2 would verify
        // a claim rather than the content.
        let mut hasher = Sha256::new();
        hasher.update(payload);
        let computed = format!("{:x}", hasher.finalize());
        if computed != artifact.content_digest {
            return Err(MvError::InvalidInput(
                "artifact content digest does not match its payload".into(),
            ));
        }

        let (stored, format, wrapped_dek) =
            self.encode_governance_bytes(payload, "run artifact")?;
        let mut connection = self
            .conn()
            .lock()
            .map_err(|err| MvError::Storage(err.to_string()))?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|err| MvError::Storage(format!("begin artifact record: {err}")))?;
        transaction
            .execute(
                "INSERT INTO agent_run_artifacts
                 (artifact_id, artifact_uri, run_id, work_order_id, artifact_kind,
                  content_digest, schema_uri, schema_version, sensitivity, retention,
                  payload, payload_format, payload_wrapped_dek, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                params![
                    artifact.artifact_id.to_string(),
                    artifact.artifact_uri.as_str(),
                    artifact.run_id.to_string(),
                    artifact.work_order_id.to_string(),
                    artifact.artifact_kind,
                    artifact.content_digest,
                    artifact.schema.as_ref().map(|s| s.uri.as_str().to_string()),
                    artifact.schema.as_ref().map(|s| s.version.clone()),
                    artifact.sensitivity.as_str(),
                    artifact.retention.as_str(),
                    stored,
                    format,
                    wrapped_dek,
                    artifact.created_at.to_rfc3339(),
                ],
            )
            .map_err(|err| MvError::Storage(format!("insert run artifact: {err}")))?;

        for reference in &artifact.provenance {
            transaction
                .execute(
                    "INSERT INTO agent_run_artifact_provenance
                     (artifact_id, relation, reference_uri, created_at)
                     VALUES (?1, ?2, ?3, ?4)",
                    params![
                        artifact.artifact_id.to_string(),
                        format!("{:?}", reference.relation),
                        reference.resource.as_str(),
                        artifact.created_at.to_rfc3339(),
                    ],
                )
                .map_err(|err| MvError::Storage(format!("insert artifact provenance: {err}")))?;
        }

        transaction
            .commit()
            .map_err(|err| MvError::Storage(format!("commit artifact record: {err}")))?;
        Ok(artifact.clone())
    }

    async fn get_run_artifact(&self, artifact_id: Uuid) -> MvResult<Option<RunArtifact>> {
        self.with_conn(|connection| {
            let artifact = connection
                .query_row(
                    "SELECT artifact_id, artifact_uri, run_id, work_order_id, artifact_kind,
                            content_digest, schema_uri, schema_version, sensitivity,
                            retention, created_at
                     FROM agent_run_artifacts
                     WHERE artifact_id = ?1",
                    params![artifact_id.to_string()],
                    |row| self.row_to_run_artifact(row),
                )
                .optional()
                .map_err(|err| MvError::Storage(format!("load run artifact: {err}")))?;
            let Some(mut artifact) = artifact else {
                return Ok(None);
            };
            artifact.provenance = self.load_artifact_provenance(connection, artifact_id)?;
            Ok(Some(artifact))
        })
    }

    async fn list_run_artifacts(&self, work_order_id: Uuid) -> MvResult<Vec<RunArtifact>> {
        self.with_conn(|connection| {
            let mut statement = connection
                .prepare(
                    "SELECT artifact_id, artifact_uri, run_id, work_order_id, artifact_kind,
                            content_digest, schema_uri, schema_version, sensitivity,
                            retention, created_at
                     FROM agent_run_artifacts
                     WHERE work_order_id = ?1
                     ORDER BY created_at ASC, artifact_id ASC",
                )
                .map_err(|err| MvError::Storage(format!("prepare artifact list: {err}")))?;
            let rows = statement
                .query_map(params![work_order_id.to_string()], |row| {
                    self.row_to_run_artifact(row)
                })
                .map_err(|err| MvError::Storage(format!("query run artifacts: {err}")))?;
            let mut artifacts = rows
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|err| MvError::Storage(format!("read run artifact: {err}")))?;
            for artifact in &mut artifacts {
                artifact.provenance =
                    self.load_artifact_provenance(connection, artifact.artifact_id)?;
            }
            Ok(artifacts)
        })
    }

    async fn restore_work_order_graph(
        &self,
        export: &WorkOrderExport,
        artifact_payloads: &[Vec<u8>],
    ) -> MvResult<()> {
        if artifact_payloads.len() != export.artifacts.len() {
            return Err(MvError::InvalidInput(
                "restore was given a different number of artifact payloads than artifacts".into(),
            ));
        }

        // Encode outside the transaction: sealing derives a key per record, and
        // holding the write lock across that work would block other writers for
        // no reason.
        let order = &export.work_order;
        let (order_payload, order_format, order_dek) =
            self.encode_governance_record(order, "work order")?;

        let mut node_records = Vec::with_capacity(export.nodes.len());
        for node in &export.nodes {
            node_records.push((
                node,
                self.encode_governance_record(node, "work-order node")?,
            ));
        }
        let mut run_records = Vec::with_capacity(export.runs.len());
        for run in &export.runs {
            run_records.push((run, self.encode_governance_record(run, "agent run")?));
        }
        let mut artifact_records = Vec::with_capacity(export.artifacts.len());
        for (exported, payload) in export.artifacts.iter().zip(artifact_payloads) {
            artifact_records.push((
                &exported.artifact,
                self.encode_governance_bytes(payload, "run artifact")?,
            ));
        }

        let mut connection = self
            .conn()
            .lock()
            .map_err(|err| MvError::Storage(err.to_string()))?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|err| MvError::Storage(format!("begin work-order restore: {err}")))?;

        transaction
            .execute(
                "INSERT INTO work_orders
                 (work_order_id, revision, work_order_uri, principal_uri, actor_uri,
                  governing_node_uri, status, status_reason, sensitivity, retention,
                  correlation_id, causation_id, idempotency_key,
                  budget_wall_clock_secs, budget_run_attempts, budget_model_tokens,
                  budget_effect_actions, remaining_wall_clock_secs, remaining_run_attempts,
                  remaining_model_tokens, remaining_effect_actions,
                  record_payload, payload_format, payload_wrapped_dek, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                         ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26)",
                params![
                    order.work_order_id.to_string(),
                    order.revision,
                    order.work_order_uri.as_str(),
                    order.principal.as_str(),
                    order.actor.as_str(),
                    order.governing_node.as_str(),
                    order.status.as_str(),
                    order.status_reason,
                    order.sensitivity.as_str(),
                    order.retention.as_str(),
                    order.correlation_id.to_string(),
                    order.causation_id.map(|id| id.to_string()),
                    order.idempotency_key,
                    order.budget.wall_clock_secs as i64,
                    order.budget.run_attempts as i64,
                    order.budget.model_tokens as i64,
                    order.budget.effect_actions as i64,
                    order.remaining.wall_clock_secs as i64,
                    order.remaining.run_attempts as i64,
                    order.remaining.model_tokens as i64,
                    order.remaining.effect_actions as i64,
                    order_payload,
                    order_format,
                    order_dek,
                    order.created_at.to_rfc3339(),
                    order.updated_at.to_rfc3339(),
                ],
            )
            .map_err(|err| MvError::Storage(format!("restore work order: {err}")))?;

        for (node, (payload, format, dek)) in &node_records {
            // Authority does not travel with the record. A node contract points
            // at the AuthorityGrant that authorized its write scope; if that
            // grant is not present in *this* vault, the reference is dropped
            // rather than restored. Carrying it would make an export file a way
            // to move authority between vaults, and a hand-written export could
            // then assert a grant that was never issued here.
            //
            // The contract, its declared scope, and its evidence are preserved
            // in full — what is not preserved is permission to act on them. A
            // restored order must be re-authorized locally before any new run
            // can pass gate G0.
            let local_grant = match node.authorizing_grant_id {
                Some(grant_id) => transaction
                    .query_row(
                        "SELECT 1 FROM interoperability_authority_grants WHERE grant_id = ?1",
                        params![grant_id.to_string()],
                        |_| Ok(()),
                    )
                    .optional()
                    .map_err(|err| MvError::Storage(format!("resolve restored grant: {err}")))?
                    .map(|()| grant_id.to_string()),
                None => None,
            };

            // The reference must be stripped from the record too, not only from
            // the column. The read model reconstructs a node contract from its
            // sealed record payload, so nulling the column alone would leave a
            // restored contract still claiming an authority this vault never
            // issued.
            let stripped;
            let (payload, format, dek) = if node.authorizing_grant_id.is_some()
                && local_grant.is_none()
            {
                let mut without_authority = (*node).clone();
                without_authority.authorizing_grant_id = None;
                stripped = self.encode_governance_record(&without_authority, "work-order node")?;
                (&stripped.0, &stripped.1, &stripped.2)
            } else {
                (payload, format, dek)
            };

            transaction
                .execute(
                    "INSERT INTO work_order_nodes
                     (node_id, work_order_id, node_uri, executor_kind, risk_tier, status,
                      timeout_secs, max_attempts, authorizing_grant_id,
                      record_payload, payload_format, payload_wrapped_dek, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                    params![
                        node.node_id.to_string(),
                        node.work_order_id.to_string(),
                        node.node_uri.as_str(),
                        node.executor_kind.as_str(),
                        node.risk_tier.as_str(),
                        node.status.as_str(),
                        node.timeout_secs,
                        node.max_attempts,
                        local_grant,
                        payload,
                        format,
                        dek,
                        node.created_at.to_rfc3339(),
                        node.updated_at.to_rfc3339(),
                    ],
                )
                .map_err(|err| MvError::Storage(format!("restore work-order node: {err}")))?;

            // Rebuilt from the record, not carried in the export: the digest
            // index is derived state, and recomputing it means a tampered
            // index cannot travel between vaults.
            for digest in node.write_target_digests() {
                transaction
                    .execute(
                        "INSERT INTO work_order_node_write_targets
                         (node_id, work_order_id, target_digest, created_at)
                         VALUES (?1, ?2, ?3, ?4)",
                        params![
                            node.node_id.to_string(),
                            node.work_order_id.to_string(),
                            digest,
                            node.created_at.to_rfc3339(),
                        ],
                    )
                    .map_err(|err| {
                        MvError::Storage(format!("restore declared write target: {err}"))
                    })?;
            }
        }

        for edge in &export.edges {
            transaction
                .execute(
                    "INSERT INTO work_order_edges
                     (edge_id, work_order_id, from_node_id, to_node_id, edge_kind,
                      derived, detail, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        edge.edge_id.to_string(),
                        edge.work_order_id.to_string(),
                        edge.from_node_id.to_string(),
                        edge.to_node_id.to_string(),
                        edge.kind.as_str(),
                        i64::from(edge.derived),
                        edge.detail,
                        edge.created_at.to_rfc3339(),
                    ],
                )
                .map_err(|err| MvError::Storage(format!("restore work-order edge: {err}")))?;
        }

        for (run, (payload, format, dek)) in &run_records {
            transaction
                .execute(
                    "INSERT INTO agent_runs
                     (run_id, run_uri, work_order_id, node_id, attempt_no, status, failure_class,
                      principal_uri, actor_uri, correlation_id, causation_id, started_at, ended_at,
                      record_payload, payload_format, payload_wrapped_dek, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                             ?14, ?15, ?16, ?17, ?18)",
                    params![
                        run.run_id.to_string(),
                        run.run_uri.as_str(),
                        run.work_order_id.to_string(),
                        run.node_id.to_string(),
                        run.attempt_no,
                        run.status.as_str(),
                        run.failure_class.map(|class| class.as_str()),
                        run.principal.as_str(),
                        run.actor.as_str(),
                        run.correlation_id.to_string(),
                        run.causation_id.map(|id| id.to_string()),
                        run.started_at.map(|at| at.to_rfc3339()),
                        run.ended_at.map(|at| at.to_rfc3339()),
                        payload,
                        format,
                        dek,
                        run.created_at.to_rfc3339(),
                        run.updated_at.to_rfc3339(),
                    ],
                )
                .map_err(|err| MvError::Storage(format!("restore agent run: {err}")))?;
        }

        // The insert triggers still apply — including the one refusing evidence
        // that a run satisfied its own G5 — so a hand-edited export cannot
        // launder an independence violation through the restore path.
        for result in &export.gate_results {
            transaction
                .execute(
                    "INSERT INTO agent_run_gate_results
                     (result_id, run_id, work_order_id, gate, outcome, evaluator_actor_uri,
                      evidence_digest, detail, evaluated_at, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    params![
                        result.result_id.to_string(),
                        result.run_id.to_string(),
                        result.work_order_id.to_string(),
                        result.gate.as_str(),
                        result.outcome.as_str(),
                        result.evaluator_actor.as_str(),
                        result.evidence_digest,
                        result.detail,
                        result.evaluated_at.to_rfc3339(),
                        result.created_at.to_rfc3339(),
                    ],
                )
                .map_err(|err| MvError::Storage(format!("restore gate result: {err}")))?;
        }

        for (artifact, (payload, format, dek)) in &artifact_records {
            transaction
                .execute(
                    "INSERT INTO agent_run_artifacts
                     (artifact_id, artifact_uri, run_id, work_order_id, artifact_kind,
                      content_digest, schema_uri, schema_version, sensitivity, retention,
                      payload, payload_format, payload_wrapped_dek, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                    params![
                        artifact.artifact_id.to_string(),
                        artifact.artifact_uri.as_str(),
                        artifact.run_id.to_string(),
                        artifact.work_order_id.to_string(),
                        artifact.artifact_kind,
                        artifact.content_digest,
                        artifact.schema.as_ref().map(|s| s.uri.as_str().to_string()),
                        artifact.schema.as_ref().map(|s| s.version.clone()),
                        artifact.sensitivity.as_str(),
                        artifact.retention.as_str(),
                        payload,
                        format,
                        dek,
                        artifact.created_at.to_rfc3339(),
                    ],
                )
                .map_err(|err| MvError::Storage(format!("restore run artifact: {err}")))?;

            for reference in &artifact.provenance {
                transaction
                    .execute(
                        "INSERT INTO agent_run_artifact_provenance
                         (artifact_id, relation, reference_uri, created_at)
                         VALUES (?1, ?2, ?3, ?4)",
                        params![
                            artifact.artifact_id.to_string(),
                            format!("{:?}", reference.relation),
                            reference.resource.as_str(),
                            artifact.created_at.to_rfc3339(),
                        ],
                    )
                    .map_err(|err| {
                        MvError::Storage(format!("restore artifact provenance: {err}"))
                    })?;
            }
        }

        transaction
            .commit()
            .map_err(|err| MvError::Storage(format!("commit work-order restore: {err}")))
    }

    async fn read_run_artifact_payload(&self, artifact_id: Uuid) -> MvResult<Option<Vec<u8>>> {
        let row = self.with_conn(|connection| {
            connection
                .query_row(
                    "SELECT payload, payload_format, payload_wrapped_dek, content_digest
                     FROM agent_run_artifacts
                     WHERE artifact_id = ?1",
                    params![artifact_id.to_string()],
                    |row| {
                        Ok((
                            row.get::<_, Vec<u8>>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, Option<String>>(2)?,
                            row.get::<_, String>(3)?,
                        ))
                    },
                )
                .optional()
                .map_err(|err| MvError::Storage(format!("load artifact payload: {err}")))
        })?;
        let Some((stored, format, wrapped_dek, recorded_digest)) = row else {
            return Ok(None);
        };

        let payload =
            self.decode_governance_bytes(&stored, &format, wrapped_dek.as_deref(), "run artifact")?;

        // Re-verify rather than trusting the stored digest column. A digest
        // compared only against itself proves nothing, and G2 is supposed to
        // check content.
        let mut hasher = Sha256::new();
        hasher.update(&payload);
        let computed = format!("{:x}", hasher.finalize());
        if computed != recorded_digest {
            return Err(MvError::Storage(format!(
                "artifact {artifact_id} content does not match its recorded digest"
            )));
        }
        Ok(Some(payload))
    }
}

impl SqliteNodeStore {
    // -----------------------------------------------------------------------
    // Governed agent execution graph helpers
    // -----------------------------------------------------------------------

    fn row_to_work_order(&self, row: &rusqlite::Row<'_>) -> rusqlite::Result<WorkOrder> {
        let work_order_id: String = row.get(0)?;
        let revision: u64 = row.get(1)?;
        let work_order_uri: String = row.get(2)?;
        let principal_uri: String = row.get(3)?;
        let actor_uri: String = row.get(4)?;
        let governing_node_uri: String = row.get(5)?;
        let status: String = row.get(6)?;
        let payload: Vec<u8> = row.get(7)?;
        let payload_format: String = row.get(8)?;
        let wrapped_dek: Option<String> = row.get(9)?;
        let created_at: String = row.get(10)?;
        let updated_at: String = row.get(11)?;

        let work_order: WorkOrder = self
            .decode_governance_record(
                &payload,
                &payload_format,
                wrapped_dek.as_deref(),
                "work order",
            )
            .map_err(|err| Self::as_sql_conversion_error(7, err.to_string()))?;
        work_order
            .validate()
            .map_err(|err| Self::as_sql_conversion_error(7, err))?;
        let stored_status: WorkOrderStatus = status
            .parse()
            .map_err(|err: String| Self::as_sql_conversion_error(6, err))?;

        // The payload is authoritative; the indexed columns must agree with it
        // or the record has been tampered with outside the governed path.
        if work_order.work_order_id != parse_uuid_str(0, &work_order_id)?
            || work_order.revision != revision
            || work_order.work_order_uri.as_str() != work_order_uri
            || work_order.principal.as_str() != principal_uri
            || work_order.actor.as_str() != actor_uri
            || work_order.governing_node.as_str() != governing_node_uri
            || work_order.status != stored_status
            || work_order.created_at != parse_dt_strict(10, &created_at)?
            || work_order.updated_at != parse_dt_strict(11, &updated_at)?
        {
            return Err(Self::as_sql_conversion_error(
                7,
                "work-order payload does not match its governed index",
            ));
        }
        Ok(work_order)
    }

    fn load_work_order_from_connection(
        &self,
        connection: &Connection,
        work_order_id: Uuid,
    ) -> MvResult<Option<WorkOrder>> {
        connection
            .query_row(
                "SELECT work_order_id, revision, work_order_uri, principal_uri, actor_uri,
                        governing_node_uri, status, record_payload, payload_format,
                        payload_wrapped_dek, created_at, updated_at
                 FROM work_orders
                 WHERE work_order_id = ?1",
                params![work_order_id.to_string()],
                |row| self.row_to_work_order(row),
            )
            .optional()
            .map_err(|err| MvError::Storage(format!("load work order: {err}")))
    }

    fn archive_work_order_revision(
        transaction: &rusqlite::Transaction<'_>,
        current: &WorkOrder,
        archived_at: DateTime<Utc>,
    ) -> MvResult<()> {
        transaction
            .execute(
                "INSERT INTO work_order_history
                 SELECT work_order_id, revision, work_order_uri, principal_uri, actor_uri,
                        governing_node_uri, status, status_reason, sensitivity, retention,
                        correlation_id, causation_id, idempotency_key,
                        budget_wall_clock_secs, budget_run_attempts, budget_model_tokens,
                        budget_effect_actions, remaining_wall_clock_secs, remaining_run_attempts,
                        remaining_model_tokens, remaining_effect_actions,
                        record_payload, payload_format, payload_wrapped_dek,
                        created_at, updated_at, ?2
                 FROM work_orders
                 WHERE work_order_id = ?1",
                params![current.work_order_id.to_string(), archived_at.to_rfc3339()],
            )
            .map_err(|err| MvError::Storage(format!("archive work-order revision: {err}")))?;
        Ok(())
    }

    fn write_work_order_revision(
        &self,
        transaction: &rusqlite::Transaction<'_>,
        replacement: &WorkOrder,
    ) -> MvResult<()> {
        let (payload, format, wrapped_dek) =
            self.encode_governance_record(replacement, "work order")?;
        let updated = transaction
            .execute(
                "UPDATE work_orders
                 SET revision = ?2, status = ?3, status_reason = ?4,
                     remaining_wall_clock_secs = ?5, remaining_run_attempts = ?6,
                     remaining_model_tokens = ?7, remaining_effect_actions = ?8,
                     record_payload = ?9, payload_format = ?10, payload_wrapped_dek = ?11,
                     updated_at = ?12
                 WHERE work_order_id = ?1",
                params![
                    replacement.work_order_id.to_string(),
                    replacement.revision,
                    replacement.status.as_str(),
                    replacement.status_reason,
                    replacement.remaining.wall_clock_secs as i64,
                    replacement.remaining.run_attempts as i64,
                    replacement.remaining.model_tokens as i64,
                    replacement.remaining.effect_actions as i64,
                    payload,
                    format,
                    wrapped_dek,
                    replacement.updated_at.to_rfc3339(),
                ],
            )
            .map_err(|err| MvError::Storage(format!("update work order: {err}")))?;
        if updated != 1 {
            return Err(MvError::Storage(
                "work order changed while its revision was being committed".into(),
            ));
        }
        Ok(())
    }

    fn load_agent_run_from_connection(
        &self,
        connection: &Connection,
        run_id: Uuid,
    ) -> MvResult<Option<AgentRun>> {
        connection
            .query_row(
                "SELECT record_payload, payload_format, payload_wrapped_dek
                 FROM agent_runs
                 WHERE run_id = ?1",
                params![run_id.to_string()],
                |row| {
                    let payload: Vec<u8> = row.get(0)?;
                    let format: String = row.get(1)?;
                    let dek: Option<String> = row.get(2)?;
                    self.decode_governance_record::<AgentRun>(
                        &payload,
                        &format,
                        dek.as_deref(),
                        "agent run",
                    )
                    .map_err(|err| Self::as_sql_conversion_error(0, err.to_string()))
                },
            )
            .optional()
            .map_err(|err| MvError::Storage(format!("load agent run: {err}")))
    }

    /// Release every unreleased lease held by a run. Called in the same
    /// transaction as any transition into a state that may not hold leases.
    fn release_run_leases(
        transaction: &rusqlite::Transaction<'_>,
        run_id: Uuid,
        reason: LeaseReleaseReason,
        released_at: DateTime<Utc>,
    ) -> MvResult<usize> {
        transaction
            .execute(
                "UPDATE agent_run_write_leases
                 SET released_at = ?2, release_reason = ?3
                 WHERE run_id = ?1 AND released_at IS NULL",
                params![
                    run_id.to_string(),
                    released_at.to_rfc3339(),
                    reason.as_str()
                ],
            )
            .map_err(|err| MvError::Storage(format!("release write leases: {err}")))
    }

    fn row_to_write_lease(row: &rusqlite::Row<'_>) -> rusqlite::Result<WriteLease> {
        let release_reason: Option<String> = row.get(9)?;
        Ok(WriteLease {
            lease_id: parse_uuid_str(0, &row.get::<_, String>(0)?)?,
            run_id: parse_uuid_str(1, &row.get::<_, String>(1)?)?,
            work_order_id: parse_uuid_str(2, &row.get::<_, String>(2)?)?,
            target_digest: row.get(3)?,
            governing_node: StableUri::parse(row.get::<_, String>(4)?)
                .map_err(|err| Self::as_sql_conversion_error(4, err))?,
            attempt_no: row.get(5)?,
            claimed_at: parse_dt_strict(6, &row.get::<_, String>(6)?)?,
            lease_expires_at: parse_dt_strict(7, &row.get::<_, String>(7)?)?,
            released_at: row
                .get::<_, Option<String>>(8)?
                .map(|value| parse_dt_strict(8, &value))
                .transpose()?,
            release_reason: release_reason
                .map(|value| value.parse())
                .transpose()
                .map_err(|err: String| Self::as_sql_conversion_error(9, err))?,
            created_at: parse_dt_strict(10, &row.get::<_, String>(10)?)?,
        })
    }

    fn row_to_run_artifact(&self, row: &rusqlite::Row<'_>) -> rusqlite::Result<RunArtifact> {
        let schema_uri: Option<String> = row.get(6)?;
        let schema_version: Option<String> = row.get(7)?;
        let schema = match (schema_uri, schema_version) {
            (Some(uri), Some(version)) => Some(
                SchemaReference::new(
                    StableUri::parse(uri).map_err(|err| Self::as_sql_conversion_error(6, err))?,
                    version,
                )
                .map_err(|err| Self::as_sql_conversion_error(6, err))?,
            ),
            _ => None,
        };
        let sensitivity: String = row.get(8)?;
        let retention: String = row.get(9)?;
        Ok(RunArtifact {
            artifact_id: parse_uuid_str(0, &row.get::<_, String>(0)?)?,
            artifact_uri: StableUri::parse(row.get::<_, String>(1)?)
                .map_err(|err| Self::as_sql_conversion_error(1, err))?,
            run_id: parse_uuid_str(2, &row.get::<_, String>(2)?)?,
            work_order_id: parse_uuid_str(3, &row.get::<_, String>(3)?)?,
            artifact_kind: row.get(4)?,
            content_digest: row.get(5)?,
            schema,
            sensitivity: sensitivity
                .parse()
                .map_err(|err: String| Self::as_sql_conversion_error(8, err))?,
            retention: retention
                .parse()
                .map_err(|err: String| Self::as_sql_conversion_error(9, err))?,
            provenance: Vec::new(),
            created_at: parse_dt_strict(10, &row.get::<_, String>(10)?)?,
        })
    }

    fn load_artifact_provenance(
        &self,
        connection: &Connection,
        artifact_id: Uuid,
    ) -> MvResult<Vec<ProvenanceReference>> {
        let mut statement = connection
            .prepare(
                "SELECT relation, reference_uri
                 FROM agent_run_artifact_provenance
                 WHERE artifact_id = ?1
                 ORDER BY relation ASC, reference_uri ASC",
            )
            .map_err(|err| MvError::Storage(format!("prepare artifact provenance: {err}")))?;
        let rows = statement
            .query_map(params![artifact_id.to_string()], |row| {
                let relation: String = row.get(0)?;
                let resource: String = row.get(1)?;
                Ok((relation, resource))
            })
            .map_err(|err| MvError::Storage(format!("query artifact provenance: {err}")))?;
        let mut references = Vec::new();
        for row in rows {
            let (relation, resource) =
                row.map_err(|err| MvError::Storage(format!("read artifact provenance: {err}")))?;
            let relation = match relation.as_str() {
                "WasDerivedFrom" => ProvenanceRelation::WasDerivedFrom,
                "WasAttributedTo" => ProvenanceRelation::WasAttributedTo,
                "WasGeneratedBy" => ProvenanceRelation::WasGeneratedBy,
                "PrimarySource" => ProvenanceRelation::PrimarySource,
                other => {
                    return Err(MvError::Storage(format!(
                        "unknown artifact provenance relation: {other}"
                    )))
                }
            };
            references.push(ProvenanceReference {
                resource: StableUri::parse(resource).map_err(MvError::Storage)?,
                relation,
            });
        }
        Ok(references)
    }

    fn resolve_node_create_replay(
        &self,
        conn: &Connection,
        source: &StableUri,
        principal: &StableUri,
        idempotency_key: &IdempotencyKey,
        payload_digest: &str,
    ) -> MvResult<Option<IdempotentNodeCommit>> {
        let existing: Option<(String, String)> = conn
            .query_row(
                "SELECT envelope_json, payload_digest
                 FROM interoperability_outbox
                 WHERE source_uri = ?1 AND principal_uri = ?2 AND idempotency_key = ?3",
                params![
                    source.as_str(),
                    principal.as_str(),
                    idempotency_key.as_str()
                ],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|e| MvError::Storage(format!("check idempotency key: {e}")))?;
        let Some((existing_json, existing_digest)) = existing else {
            return Ok(None);
        };
        if existing_digest != payload_digest {
            return Err(MvError::IdempotencyConflict(
                "idempotency key was already used with a different payload".into(),
            ));
        }
        let existing_event = Self::decode_outbox_event(&existing_json)?;
        let existing_node_id = existing_event.subject.trailing_uuid().ok_or_else(|| {
            MvError::Storage("stored event subject does not identify a UUID".into())
        })?;
        let existing_node = self
            .load_node_from_connection(conn, existing_node_id)?
            .ok_or_else(|| {
                MvError::Storage("idempotency record references a missing knowledge node".into())
            })?;
        Ok(Some(IdempotentNodeCommit {
            node: existing_node,
            event: existing_event,
            replayed: true,
        }))
    }

    fn load_node_from_connection(
        &self,
        conn: &Connection,
        id: Uuid,
    ) -> MvResult<Option<KnowledgeNode>> {
        let mut stmt = conn
            .prepare(
                "SELECT id, kind, title, content, source, namespace, importance,
                 created_at, updated_at, last_accessed_at, access_count, version,
                 expires_at, metadata_json, payload_ciphertext, payload_wrapped_dek
                 FROM knowledge_nodes WHERE id = ?1",
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let mut node = stmt
            .query_row(params![id.to_string()], |row| self.row_to_node(row))
            .optional()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        if let Some(node) = &mut node {
            node.tags = Self::load_tags(conn, node.id)?;
        }
        Ok(node)
    }

    fn decode_outbox_event(envelope_json: &str) -> MvResult<EventEnvelope> {
        let event: EventEnvelope = serde_json::from_str(envelope_json)
            .map_err(|e| MvError::Storage(format!("decode stored event envelope: {e}")))?;
        event
            .validate()
            .map_err(|e| MvError::Storage(format!("stored event envelope is invalid: {e}")))?;
        Ok(event)
    }

    /// Find a node by its `source` field (exact match).
    /// Used for dedup during imports (e.g. Obsidian vault import).
    pub async fn find_by_source(&self, source: &str) -> MvResult<Option<KnowledgeNode>> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut stmt = conn
            .prepare(
                "SELECT id, kind, title, content, source, namespace, importance,
                 created_at, updated_at, last_accessed_at, access_count, version,
                 expires_at, metadata_json, payload_ciphertext, payload_wrapped_dek FROM knowledge_nodes WHERE source = ?1 LIMIT 1",
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let row = stmt
            .query_row(params![source], |row| self.row_to_node(row))
            .optional()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        match row {
            Some(mut node) => {
                node.tags = Self::load_tags(&conn, node.id)?;
                Ok(Some(node))
            }
            None => {
                if !self.sealed_mode() {
                    return Ok(None);
                }
                let mut stmt = conn
                    .prepare(
                        "SELECT id, kind, title, content, source, namespace, importance,
                         created_at, updated_at, last_accessed_at, access_count, version,
                         expires_at, metadata_json, payload_ciphertext, payload_wrapped_dek FROM knowledge_nodes WHERE payload_ciphertext IS NOT NULL",
                    )
                    .map_err(|e| MvError::Storage(e.to_string()))?;
                let rows = stmt
                    .query_map([], |row| self.row_to_node(row))
                    .map_err(|e| MvError::Storage(e.to_string()))?;
                for row in rows {
                    let mut node = row.map_err(|e| MvError::Storage(e.to_string()))?;
                    if node.source.as_deref() == Some(source) {
                        node.tags = Self::load_tags(&conn, node.id)?;
                        return Ok(Some(node));
                    }
                }
                Ok(None)
            }
        }
    }

    pub async fn insert_permission_template(&self, template: &PermissionTemplate) -> MvResult<()> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let scope_tags_json = serde_json::to_string(&template.scope_tags)?;
        let allow_kinds_json = serde_json::to_string(
            &template
                .allow_kinds
                .iter()
                .map(|kind| kind.as_str())
                .collect::<Vec<_>>(),
        )?;
        let allow_actions_json = serde_json::to_string(&template.allow_actions)?;

        conn.execute(
            "INSERT INTO permission_templates (id, name, description, tier, scope_namespace, scope_tags_json, allow_kinds_json, allow_actions_json, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                template.id.to_string(),
                template.name,
                template.description,
                template.tier.as_str(),
                template.scope_namespace,
                scope_tags_json,
                allow_kinds_json,
                allow_actions_json,
                template.created_at.to_rfc3339(),
                template.updated_at.to_rfc3339(),
            ],
        )
        .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(())
    }

    pub async fn update_permission_template(&self, template: &PermissionTemplate) -> MvResult<()> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let scope_tags_json = serde_json::to_string(&template.scope_tags)?;
        let allow_kinds_json = serde_json::to_string(
            &template
                .allow_kinds
                .iter()
                .map(|kind| kind.as_str())
                .collect::<Vec<_>>(),
        )?;
        let allow_actions_json = serde_json::to_string(&template.allow_actions)?;

        conn.execute(
            "UPDATE permission_templates SET name = ?2, description = ?3, tier = ?4, scope_namespace = ?5, scope_tags_json = ?6, allow_kinds_json = ?7, allow_actions_json = ?8, updated_at = ?9 WHERE id = ?1",
            params![
                template.id.to_string(),
                template.name,
                template.description,
                template.tier.as_str(),
                template.scope_namespace,
                scope_tags_json,
                allow_kinds_json,
                allow_actions_json,
                template.updated_at.to_rfc3339(),
            ],
        )
        .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(())
    }

    pub async fn get_permission_template(&self, id: Uuid) -> MvResult<Option<PermissionTemplate>> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut stmt = conn
            .prepare(
                "SELECT id, name, description, tier, scope_namespace, scope_tags_json, allow_kinds_json, allow_actions_json, created_at, updated_at FROM permission_templates WHERE id = ?1",
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let row = stmt
            .query_row(params![id.to_string()], |row| {
                row_to_permission_template(row)
            })
            .optional()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        Ok(row)
    }

    pub async fn get_permission_template_by_name(
        &self,
        name: &str,
    ) -> MvResult<Option<PermissionTemplate>> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut stmt = conn
            .prepare(
                "SELECT id, name, description, tier, scope_namespace, scope_tags_json, allow_kinds_json, allow_actions_json, created_at, updated_at FROM permission_templates WHERE lower(name) = lower(?1)",
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let row = stmt
            .query_row(params![name], row_to_permission_template)
            .optional()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        Ok(row)
    }

    pub async fn list_permission_templates(
        &self,
        limit: usize,
        offset: usize,
    ) -> MvResult<Vec<PermissionTemplate>> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut stmt = conn
            .prepare(
                "SELECT id, name, description, tier, scope_namespace, scope_tags_json, allow_kinds_json, allow_actions_json, created_at, updated_at FROM permission_templates ORDER BY created_at DESC LIMIT ?1 OFFSET ?2",
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let rows = stmt
            .query_map(params![limit as i64, offset as i64], |row| {
                row_to_permission_template(row)
            })
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut templates = Vec::new();
        for row in rows {
            templates.push(row.map_err(|e| MvError::Storage(e.to_string()))?);
        }

        Ok(templates)
    }

    pub async fn delete_permission_template(&self, id: Uuid) -> MvResult<bool> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let affected = conn
            .execute(
                "DELETE FROM permission_templates WHERE id = ?1",
                params![id.to_string()],
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;

        Ok(affected > 0)
    }

    pub async fn insert_access_key(&self, key: &AccessKey) -> MvResult<()> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        conn.execute(
            "INSERT INTO access_keys (id, name, template_id, key_hash, created_at, last_used_at, expires_at, revoked_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                key.id.to_string(),
                key.name,
                key.template_id.to_string(),
                key.key_hash,
                key.created_at.to_rfc3339(),
                key.last_used_at.map(|dt| dt.to_rfc3339()),
                key.expires_at.map(|dt| dt.to_rfc3339()),
                key.revoked_at.map(|dt| dt.to_rfc3339()),
            ],
        )
        .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(())
    }

    pub async fn list_access_keys(&self) -> MvResult<Vec<AccessKey>> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut stmt = conn
            .prepare(
                "SELECT id, name, template_id, key_hash, created_at, last_used_at, expires_at, revoked_at FROM access_keys ORDER BY created_at DESC",
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let rows = stmt
            .query_map([], row_to_access_key)
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut keys = Vec::new();
        for row in rows {
            keys.push(row.map_err(|e| MvError::Storage(e.to_string()))?);
        }

        Ok(keys)
    }

    pub async fn get_access_key(&self, id: Uuid) -> MvResult<Option<AccessKey>> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut stmt = conn
            .prepare(
                "SELECT id, name, template_id, key_hash, created_at, last_used_at, expires_at, revoked_at FROM access_keys WHERE id = ?1",
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let row = stmt
            .query_row(params![id.to_string()], row_to_access_key)
            .optional()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        Ok(row)
    }

    pub async fn get_access_key_by_hash(&self, key_hash: &str) -> MvResult<Option<AccessKey>> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut stmt = conn
            .prepare(
                "SELECT id, name, template_id, key_hash, created_at, last_used_at, expires_at, revoked_at FROM access_keys WHERE key_hash = ?1",
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let row = stmt
            .query_row(params![key_hash], row_to_access_key)
            .optional()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        Ok(row)
    }

    pub async fn update_access_key_last_used(
        &self,
        id: Uuid,
        when: chrono::DateTime<chrono::Utc>,
    ) -> MvResult<()> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        conn.execute(
            "UPDATE access_keys SET last_used_at = ?2 WHERE id = ?1",
            params![id.to_string(), when.to_rfc3339()],
        )
        .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(())
    }

    pub async fn revoke_access_key(
        &self,
        id: Uuid,
        when: chrono::DateTime<chrono::Utc>,
    ) -> MvResult<bool> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let affected = conn
            .execute(
                "UPDATE access_keys SET revoked_at = ?2 WHERE id = ?1",
                params![id.to_string(), when.to_rfc3339()],
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;

        Ok(affected > 0)
    }
}

fn row_to_permission_template(row: &rusqlite::Row<'_>) -> rusqlite::Result<PermissionTemplate> {
    let id_str: String = row.get(0)?;
    let name: String = row.get(1)?;
    let description: Option<String> = row.get(2)?;
    let tier_str: String = row.get(3)?;
    let scope_namespace: Option<String> = row.get(4)?;
    let scope_tags_json: Option<String> = row.get(5)?;
    let allow_kinds_json: Option<String> = row.get(6)?;
    let allow_actions_json: Option<String> = row.get(7)?;
    let created_at: String = row.get(8)?;
    let updated_at: String = row.get(9)?;

    let id = parse_uuid_str(0, &id_str)?;
    let tier: PermissionTier = tier_str.parse().map_err(|err: String| {
        rusqlite::Error::FromSqlConversionFailure(
            3,
            Type::Text,
            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, err)),
        )
    })?;

    let scope_tags = parse_json_vec::<String>(5, scope_tags_json)?;
    let allow_kinds_raw = parse_json_vec::<String>(6, allow_kinds_json)?;
    let mut allow_kinds = Vec::new();
    for raw in allow_kinds_raw {
        let parsed: NodeKind = raw.parse().map_err(|err: String| {
            rusqlite::Error::FromSqlConversionFailure(
                6,
                Type::Text,
                Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, err)),
            )
        })?;
        allow_kinds.push(parsed);
    }
    let allow_actions = parse_json_vec::<String>(7, allow_actions_json)?;

    Ok(PermissionTemplate {
        id,
        name,
        description,
        tier,
        scope_namespace,
        scope_tags,
        allow_kinds,
        allow_actions,
        created_at: parse_dt_strict(8, &created_at)?,
        updated_at: parse_dt_strict(9, &updated_at)?,
    })
}

fn row_to_access_key(row: &rusqlite::Row<'_>) -> rusqlite::Result<AccessKey> {
    let id_str: String = row.get(0)?;
    let name: Option<String> = row.get(1)?;
    let template_id_str: String = row.get(2)?;
    let key_hash: String = row.get(3)?;
    let created_at: String = row.get(4)?;
    let last_used_at: Option<String> = row.get(5)?;
    let expires_at: Option<String> = row.get(6)?;
    let revoked_at: Option<String> = row.get(7)?;

    Ok(AccessKey {
        id: parse_uuid_str(0, &id_str)?,
        name,
        template_id: parse_uuid_str(2, &template_id_str)?,
        key_hash,
        created_at: parse_dt_strict(4, &created_at)?,
        last_used_at: parse_optional_dt_strict(5, last_used_at)?,
        expires_at: parse_optional_dt_strict(6, expires_at)?,
        revoked_at: parse_optional_dt_strict(7, revoked_at)?,
    })
}

fn parse_json_vec<T: serde::de::DeserializeOwned>(
    column: usize,
    raw: Option<String>,
) -> rusqlite::Result<Vec<T>> {
    match raw {
        Some(value) => serde_json::from_str(&value).map_err(|err| {
            rusqlite::Error::FromSqlConversionFailure(column, Type::Text, Box::new(err))
        }),
        None => Ok(Vec::new()),
    }
}

// ---------------------------------------------------------------------------
// AgenticStore Implementation
// ---------------------------------------------------------------------------

use mv_core::{
    AgenticStore, CapturedIntent, ChronicleEntry, InsightType, IntentStatus, IntentType,
    ProactiveInsight,
};

#[async_trait]
impl AgenticStore for SqliteNodeStore {
    async fn log_intent(&self, intent: &CapturedIntent) -> MvResult<()> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let params_json = serde_json::to_string(&intent.parameters)?;

        conn.execute(
            "INSERT INTO captured_intents (id, node_id, intent_type, confidence, parameters, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                intent.id.to_string(),
                intent.node_id.to_string(),
                intent.intent_type.as_str(),
                intent.confidence as f64,
                params_json,
                intent.status.as_str(),
                intent.created_at.to_rfc3339(),
                intent.updated_at.map(|dt| dt.to_rfc3339()),
            ],
        )
        .map_err(|e| MvError::Storage(format!("insert intent failed: {e}")))?;
        Ok(())
    }

    async fn get_intent(&self, id: Uuid) -> MvResult<Option<CapturedIntent>> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, node_id, intent_type, confidence, parameters, status, created_at, updated_at
                 FROM captured_intents WHERE id = ?1",
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let result = stmt
            .query_row(params![id.to_string()], row_to_captured_intent)
            .optional()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(result)
    }

    async fn list_intents(
        &self,
        node_id: Option<Uuid>,
        status: Option<IntentStatus>,
        limit: usize,
        offset: usize,
    ) -> MvResult<Vec<CapturedIntent>> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut sql = String::from(
            "SELECT id, node_id, intent_type, confidence, parameters, status, created_at, updated_at
             FROM captured_intents WHERE 1=1",
        );
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        let mut param_idx = 1;

        if let Some(nid) = node_id {
            sql.push_str(&format!(" AND node_id = ?{param_idx}"));
            param_values.push(Box::new(nid.to_string()));
            param_idx += 1;
        }

        if let Some(st) = status {
            sql.push_str(&format!(" AND status = ?{param_idx}"));
            param_values.push(Box::new(st.as_str().to_string()));
            param_idx += 1;
        }

        sql.push_str(&format!(
            " ORDER BY created_at DESC LIMIT ?{param_idx} OFFSET ?{}",
            param_idx + 1
        ));
        param_values.push(Box::new(limit as i64));
        param_values.push(Box::new(offset as i64));

        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let rows = stmt
            .query_map(params_refs.as_slice(), row_to_captured_intent)
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut intents = Vec::new();
        for row in rows {
            intents.push(row.map_err(|e| MvError::Storage(e.to_string()))?);
        }
        Ok(intents)
    }

    async fn update_intent_status(&self, id: Uuid, status: IntentStatus) -> MvResult<bool> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let now = Utc::now().to_rfc3339();
        let affected = conn
            .execute(
                "UPDATE captured_intents SET status = ?2, updated_at = ?3 WHERE id = ?1",
                params![id.to_string(), status.as_str(), now],
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(affected > 0)
    }

    async fn log_insight(&self, insight: &ProactiveInsight) -> MvResult<()> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let related_ids_json = serde_json::to_string(&insight.related_node_ids)?;

        conn.execute(
            "INSERT INTO proactive_insights (id, title, content, insight_type, related_node_ids, importance, created_at, dismissed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                insight.id.to_string(),
                insight.title,
                insight.content,
                insight.insight_type.as_str(),
                related_ids_json,
                insight.importance as f64,
                insight.created_at.to_rfc3339(),
                insight.dismissed_at.map(|dt| dt.to_rfc3339()),
            ],
        )
        .map_err(|e| MvError::Storage(format!("insert insight failed: {e}")))?;
        Ok(())
    }

    async fn list_insights(&self, limit: usize, offset: usize) -> MvResult<Vec<ProactiveInsight>> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, title, content, insight_type, related_node_ids, importance, created_at, dismissed_at
                 FROM proactive_insights
                 WHERE dismissed_at IS NULL
                 ORDER BY created_at DESC
                 LIMIT ?1 OFFSET ?2",
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let rows = stmt
            .query_map(
                params![limit as i64, offset as i64],
                row_to_proactive_insight,
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut insights = Vec::new();
        for row in rows {
            insights.push(row.map_err(|e| MvError::Storage(e.to_string()))?);
        }
        Ok(insights)
    }

    async fn delete_insight(&self, id: Uuid) -> MvResult<bool> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let now = Utc::now().to_rfc3339();
        let affected = conn
            .execute(
                "UPDATE proactive_insights SET dismissed_at = ?2 WHERE id = ?1 AND dismissed_at IS NULL",
                params![id.to_string(), now],
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(affected > 0)
    }

    async fn log_chronicle(&self, entry: &ChronicleEntry) -> MvResult<()> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        conn.execute(
            "INSERT INTO chronicle_entries (id, node_id, step_name, logic, input_snapshot, output_snapshot, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                entry.id.to_string(),
                entry.node_id.map(|id| id.to_string()),
                entry.step_name,
                entry.logic,
                entry.input_snapshot,
                entry.output_snapshot,
                entry.timestamp.to_rfc3339(),
            ],
        )
        .map_err(|e| MvError::Storage(format!("insert chronicle failed: {e}")))?;
        Ok(())
    }

    async fn list_chronicles(
        &self,
        node_id: Option<Uuid>,
        limit: usize,
        offset: usize,
    ) -> MvResult<Vec<ChronicleEntry>> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let (sql, params_box): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = if let Some(nid) =
            node_id
        {
            (
                "SELECT id, node_id, step_name, logic, input_snapshot, output_snapshot, timestamp
                 FROM chronicle_entries WHERE node_id = ?1 ORDER BY timestamp DESC LIMIT ?2 OFFSET ?3".to_string(),
                vec![
                    Box::new(nid.to_string()),
                    Box::new(limit as i64),
                    Box::new(offset as i64),
                ],
            )
        } else {
            (
                "SELECT id, node_id, step_name, logic, input_snapshot, output_snapshot, timestamp
                 FROM chronicle_entries ORDER BY timestamp DESC LIMIT ?1 OFFSET ?2"
                    .to_string(),
                vec![Box::new(limit as i64), Box::new(offset as i64)],
            )
        };

        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            params_box.iter().map(|p| p.as_ref()).collect();
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let rows = stmt
            .query_map(params_refs.as_slice(), row_to_chronicle_entry)
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut entries = Vec::new();
        for row in rows {
            entries.push(row.map_err(|e| MvError::Storage(e.to_string()))?);
        }
        Ok(entries)
    }
}

fn row_to_captured_intent(row: &rusqlite::Row<'_>) -> rusqlite::Result<CapturedIntent> {
    let id_str: String = row.get(0)?;
    let node_id_str: String = row.get(1)?;
    let intent_type_str: String = row.get(2)?;
    let confidence: f64 = row.get(3)?;
    let params_json: Option<String> = row.get(4)?;
    let status_str: String = row.get(5)?;
    let created_at: String = row.get(6)?;
    let updated_at: Option<String> = row.get(7)?;

    let id = parse_uuid_str(0, &id_str)?;
    let node_id = parse_uuid_str(1, &node_id_str)?;
    let intent_type: IntentType = intent_type_str
        .parse()
        .unwrap_or(IntentType::Custom(intent_type_str));
    let status: IntentStatus = status_str.parse().map_err(|e: String| {
        rusqlite::Error::FromSqlConversionFailure(
            5,
            Type::Text,
            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
        )
    })?;
    let parameters: serde_json::Value = params_json
        .map(|s| serde_json::from_str(&s).unwrap_or(serde_json::Value::Null))
        .unwrap_or(serde_json::Value::Null);

    Ok(CapturedIntent {
        id,
        node_id,
        intent_type,
        confidence: confidence as f32,
        parameters,
        status,
        created_at: parse_dt_strict(6, &created_at)?,
        updated_at: parse_optional_dt_strict(7, updated_at)?,
    })
}

fn row_to_proactive_insight(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProactiveInsight> {
    let id_str: String = row.get(0)?;
    let title: String = row.get(1)?;
    let content: String = row.get(2)?;
    let insight_type_str: String = row.get(3)?;
    let related_ids_json: Option<String> = row.get(4)?;
    let importance: f64 = row.get(5)?;
    let created_at: String = row.get(6)?;
    let dismissed_at: Option<String> = row.get(7)?;

    let id = parse_uuid_str(0, &id_str)?;
    let insight_type: InsightType = insight_type_str.parse().map_err(|e: String| {
        rusqlite::Error::FromSqlConversionFailure(
            3,
            Type::Text,
            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
        )
    })?;

    let related_node_ids: Vec<Uuid> = related_ids_json
        .map(|s| {
            let strs: Vec<String> = serde_json::from_str(&s).unwrap_or_default();
            strs.iter()
                .filter_map(|id_str| Uuid::parse_str(id_str).ok())
                .collect()
        })
        .unwrap_or_default();

    Ok(ProactiveInsight {
        id,
        title,
        content,
        insight_type,
        related_node_ids,
        importance: importance as f32,
        metadata: std::collections::HashMap::new(),
        created_at: parse_dt_strict(6, &created_at)?,
        dismissed_at: parse_optional_dt_strict(7, dismissed_at)?,
    })
}

fn row_to_chronicle_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<ChronicleEntry> {
    let id_str: String = row.get(0)?;
    let node_id_str: Option<String> = row.get(1)?;
    let step_name: String = row.get(2)?;
    let logic: String = row.get(3)?;
    let input_snapshot: Option<String> = row.get(4)?;
    let output_snapshot: Option<String> = row.get(5)?;
    let timestamp: String = row.get(6)?;

    let id = parse_uuid_str(0, &id_str)?;
    let node_id = node_id_str.and_then(|s| Uuid::parse_str(&s).ok());

    Ok(ChronicleEntry {
        id,
        node_id,
        step_name,
        logic,
        input_snapshot,
        output_snapshot,
        timestamp: parse_dt_strict(6, &timestamp)?,
    })
}

// ---------------------------------------------------------------------------
// ExchangeStore Implementation
// ---------------------------------------------------------------------------

use chrono::DateTime;
use mv_core::{ExchangeStore, Proposal, ProposalAction, ProposalSender, ProposalState};

fn row_to_proposal(row: &rusqlite::Row<'_>) -> rusqlite::Result<Proposal> {
    let id_str: String = row.get(0)?;
    let node_id_str: Option<String> = row.get(1)?;
    let target_node_id_str: Option<String> = row.get(2)?;
    let sender_str: String = row.get(3)?;
    let action_str: String = row.get(4)?;
    let state_str: String = row.get(5)?;
    let confidence: f64 = row.get(6)?;
    let diff_preview: Option<String> = row.get(7)?;
    let payload_json: Option<String> = row.get(8)?;
    let created_at: String = row.get(9)?;
    let updated_at: Option<String> = row.get(10)?;
    let resolved_at: Option<String> = row.get(11)?;

    let id = parse_uuid_str(0, &id_str)?;
    let node_id = node_id_str.and_then(|s| Uuid::parse_str(&s).ok());
    let target_node_id = target_node_id_str.and_then(|s| Uuid::parse_str(&s).ok());

    let sender: ProposalSender = sender_str.parse().map_err(|e: String| {
        rusqlite::Error::FromSqlConversionFailure(
            3,
            Type::Text,
            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
        )
    })?;
    let action: ProposalAction = action_str
        .parse()
        .unwrap_or(ProposalAction::Custom(action_str));
    let state: ProposalState = state_str.parse().map_err(|e: String| {
        rusqlite::Error::FromSqlConversionFailure(
            5,
            Type::Text,
            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
        )
    })?;

    let payload: std::collections::HashMap<String, serde_json::Value> = payload_json
        .map(|s| serde_json::from_str(&s).unwrap_or_default())
        .unwrap_or_default();

    Ok(Proposal {
        id,
        node_id,
        target_node_id,
        sender,
        action,
        state,
        confidence: confidence as f32,
        diff_preview,
        payload,
        created_at: parse_dt_strict(9, &created_at)?,
        updated_at: parse_optional_dt_strict(10, updated_at)?,
        resolved_at: parse_optional_dt_strict(11, resolved_at)?,
    })
}

#[async_trait]
impl ExchangeStore for SqliteNodeStore {
    async fn submit_proposal(&self, proposal: &Proposal) -> MvResult<()> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let payload_json = serde_json::to_string(&proposal.payload)?;

        conn.execute(
            "INSERT INTO proposals (id, node_id, target_node_id, sender, action, state, confidence, diff_preview, payload, created_at, updated_at, resolved_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                proposal.id.to_string(),
                proposal.node_id.map(|id| id.to_string()),
                proposal.target_node_id.map(|id| id.to_string()),
                proposal.sender.as_str(),
                proposal.action.as_str(),
                proposal.state.as_str(),
                proposal.confidence as f64,
                proposal.diff_preview,
                payload_json,
                proposal.created_at.to_rfc3339(),
                proposal.updated_at.map(|dt| dt.to_rfc3339()),
                proposal.resolved_at.map(|dt| dt.to_rfc3339()),
            ],
        )
        .map_err(|e| MvError::Storage(format!("insert proposal failed: {e}")))?;
        Ok(())
    }

    async fn get_proposal(&self, id: Uuid) -> MvResult<Option<Proposal>> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, node_id, target_node_id, sender, action, state, confidence, diff_preview, payload, created_at, updated_at, resolved_at
                 FROM proposals WHERE id = ?1",
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let result = stmt
            .query_row(params![id.to_string()], row_to_proposal)
            .optional()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(result)
    }

    async fn list_proposals(
        &self,
        state: Option<ProposalState>,
        limit: usize,
        offset: usize,
    ) -> MvResult<Vec<Proposal>> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut sql = String::from(
            "SELECT id, node_id, target_node_id, sender, action, state, confidence, diff_preview, payload, created_at, updated_at, resolved_at
             FROM proposals WHERE 1=1",
        );
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        let mut param_idx = 1;

        if let Some(st) = state {
            sql.push_str(&format!(" AND state = ?{param_idx}"));
            param_values.push(Box::new(st.as_str().to_string()));
            param_idx += 1;
        }

        sql.push_str(&format!(
            " ORDER BY created_at DESC LIMIT ?{param_idx} OFFSET ?{}",
            param_idx + 1
        ));
        param_values.push(Box::new(limit as i64));
        param_values.push(Box::new(offset as i64));

        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let rows = stmt
            .query_map(params_refs.as_slice(), row_to_proposal)
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut proposals = Vec::new();
        for row in rows {
            proposals.push(row.map_err(|e| MvError::Storage(e.to_string()))?);
        }
        Ok(proposals)
    }

    async fn resolve_proposal(&self, id: Uuid, state: ProposalState) -> MvResult<bool> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let now = Utc::now().to_rfc3339();
        let affected = conn
            .execute(
                "UPDATE proposals SET state = ?2, updated_at = ?3, resolved_at = ?3 WHERE id = ?1",
                params![id.to_string(), state.as_str(), now],
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(affected > 0)
    }

    async fn count_proposals(&self, state: Option<ProposalState>) -> MvResult<usize> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let (sql, params_box): (String, Vec<Box<dyn rusqlite::types::ToSql>>) =
            if let Some(st) = state {
                (
                    "SELECT COUNT(*) FROM proposals WHERE state = ?1".to_string(),
                    vec![Box::new(st.as_str().to_string())],
                )
            } else {
                ("SELECT COUNT(*) FROM proposals".to_string(), vec![])
            };

        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            params_box.iter().map(|p| p.as_ref()).collect();

        let count: usize = conn
            .query_row(&sql, params_refs.as_slice(), |row| row.get(0))
            .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(count)
    }

    async fn expire_proposals(&self, before: DateTime<Utc>) -> MvResult<usize> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let now = Utc::now().to_rfc3339();
        let affected = conn
            .execute(
                "UPDATE proposals SET state = 'expired', updated_at = ?1, resolved_at = ?1 WHERE state = 'pending' AND created_at < ?2",
                params![now, before.to_rfc3339()],
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(affected)
    }
}

// ---------------------------------------------------------------------------
// SafeguardStore Implementation
// ---------------------------------------------------------------------------

use mv_core::{AutoApproveRule, BlockedSender, SafeguardStore, UndoSnapshot};

fn row_to_blocked_sender(row: &rusqlite::Row<'_>) -> rusqlite::Result<BlockedSender> {
    let id_str: String = row.get(0)?;
    let sender_type: String = row.get(1)?;
    let sender_pattern: String = row.get(2)?;
    let reason: Option<String> = row.get(3)?;
    let blocked_at: String = row.get(4)?;
    let expires_at: Option<String> = row.get(5)?;

    let id = parse_uuid_str(0, &id_str)?;

    Ok(BlockedSender {
        id,
        sender_type,
        sender_pattern,
        reason,
        blocked_at: parse_dt_strict(4, &blocked_at)?,
        expires_at: parse_optional_dt_strict(5, expires_at)?,
    })
}

fn row_to_auto_approve_rule(row: &rusqlite::Row<'_>) -> rusqlite::Result<AutoApproveRule> {
    let id_str: String = row.get(0)?;
    let name: String = row.get(1)?;
    let sender_pattern: Option<String> = row.get(2)?;
    let action_types_csv: Option<String> = row.get(3)?;
    let min_confidence: f64 = row.get(4)?;
    let enabled: bool = row.get(5)?;
    let created_at: String = row.get(6)?;
    let updated_at: Option<String> = row.get(7)?;

    let id = parse_uuid_str(0, &id_str)?;
    let action_types: Vec<String> = action_types_csv
        .map(|csv| {
            csv.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();

    Ok(AutoApproveRule {
        id,
        name,
        sender_pattern,
        action_types,
        min_confidence: min_confidence as f32,
        enabled,
        created_at: parse_dt_strict(6, &created_at)?,
        updated_at: parse_optional_dt_strict(7, updated_at)?,
    })
}

fn row_to_undo_snapshot(row: &rusqlite::Row<'_>) -> rusqlite::Result<UndoSnapshot> {
    let id_str: String = row.get(0)?;
    let proposal_id_str: String = row.get(1)?;
    let snapshot_data_str: String = row.get(2)?;
    let created_at: String = row.get(3)?;
    let expires_at: String = row.get(4)?;
    let used: bool = row.get(5)?;

    let id = parse_uuid_str(0, &id_str)?;
    let proposal_id = parse_uuid_str(1, &proposal_id_str)?;
    let snapshot_data: serde_json::Value =
        serde_json::from_str(&snapshot_data_str).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(
                2,
                Type::Text,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    e.to_string(),
                )),
            )
        })?;

    Ok(UndoSnapshot {
        id,
        proposal_id,
        snapshot_data,
        created_at: parse_dt_strict(3, &created_at)?,
        expires_at: parse_dt_strict(4, &expires_at)?,
        used,
    })
}

/// Simple glob matching: supports `*` (match all), `prefix*`, and exact match.
fn safeguard_glob_match(pattern: &str, value: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if let Some(stripped) = pattern.strip_suffix('*') {
        value.starts_with(stripped)
    } else {
        pattern == value
    }
}

#[async_trait]
impl SafeguardStore for SqliteNodeStore {
    async fn add_blocked_sender(&self, sender: &BlockedSender) -> MvResult<()> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        conn.execute(
			"INSERT INTO blocked_senders (id, sender_type, sender_pattern, reason, blocked_at, expires_at)
			 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
			params![
				sender.id.to_string(),
				sender.sender_type,
				sender.sender_pattern,
				sender.reason,
				sender.blocked_at.to_rfc3339(),
				sender.expires_at.map(|dt| dt.to_rfc3339()),
			],
		)
		.map_err(|e| MvError::Storage(format!("insert blocked_sender failed: {e}")))?;
        Ok(())
    }

    async fn remove_blocked_sender(&self, id: Uuid) -> MvResult<bool> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let affected = conn
            .execute(
                "DELETE FROM blocked_senders WHERE id = ?1",
                params![id.to_string()],
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(affected > 0)
    }

    async fn list_blocked_senders(&self) -> MvResult<Vec<BlockedSender>> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, sender_type, sender_pattern, reason, blocked_at, expires_at
				 FROM blocked_senders ORDER BY blocked_at DESC",
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let rows = stmt
            .query_map([], row_to_blocked_sender)
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(|e| MvError::Storage(e.to_string()))?);
        }
        Ok(result)
    }

    async fn is_sender_blocked(&self, sender_type: &str, sender_name: &str) -> MvResult<bool> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let now = Utc::now().to_rfc3339();

        // Fetch all active rules for this sender_type (not expired)
        let mut stmt = conn
            .prepare(
                "SELECT id, sender_type, sender_pattern, reason, blocked_at, expires_at
				 FROM blocked_senders
				 WHERE sender_type = ?1 AND (expires_at IS NULL OR expires_at > ?2)",
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let rows = stmt
            .query_map(params![sender_type, now], row_to_blocked_sender)
            .map_err(|e| MvError::Storage(e.to_string()))?;

        for row in rows {
            let blocked = row.map_err(|e| MvError::Storage(e.to_string()))?;
            if safeguard_glob_match(&blocked.sender_pattern, sender_name) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    async fn add_auto_approve_rule(&self, rule: &AutoApproveRule) -> MvResult<()> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let action_types_csv = if rule.action_types.is_empty() {
            None
        } else {
            Some(rule.action_types.join(","))
        };

        conn.execute(
			"INSERT INTO auto_approve_rules (id, name, sender_pattern, action_types, min_confidence, enabled, created_at, updated_at)
			 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
			params![
				rule.id.to_string(),
				rule.name,
				rule.sender_pattern,
				action_types_csv,
				rule.min_confidence as f64,
				rule.enabled,
				rule.created_at.to_rfc3339(),
				rule.updated_at.map(|dt| dt.to_rfc3339()),
			],
		)
		.map_err(|e| MvError::Storage(format!("insert auto_approve_rule failed: {e}")))?;
        Ok(())
    }

    async fn remove_auto_approve_rule(&self, id: Uuid) -> MvResult<bool> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let affected = conn
            .execute(
                "DELETE FROM auto_approve_rules WHERE id = ?1",
                params![id.to_string()],
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(affected > 0)
    }

    async fn list_auto_approve_rules(&self) -> MvResult<Vec<AutoApproveRule>> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let mut stmt = conn
			.prepare(
				"SELECT id, name, sender_pattern, action_types, min_confidence, enabled, created_at, updated_at
				 FROM auto_approve_rules ORDER BY created_at DESC",
			)
			.map_err(|e| MvError::Storage(e.to_string()))?;

        let rows = stmt
            .query_map([], row_to_auto_approve_rule)
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(|e| MvError::Storage(e.to_string()))?);
        }
        Ok(result)
    }

    async fn update_auto_approve_rule(&self, rule: &AutoApproveRule) -> MvResult<bool> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let action_types_csv = if rule.action_types.is_empty() {
            None
        } else {
            Some(rule.action_types.join(","))
        };
        let now = Utc::now().to_rfc3339();

        let affected = conn
			.execute(
				"UPDATE auto_approve_rules SET name = ?2, sender_pattern = ?3, action_types = ?4, min_confidence = ?5, enabled = ?6, updated_at = ?7 WHERE id = ?1",
				params![
					rule.id.to_string(),
					rule.name,
					rule.sender_pattern,
					action_types_csv,
					rule.min_confidence as f64,
					rule.enabled,
					now,
				],
			)
			.map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(affected > 0)
    }

    async fn save_undo_snapshot(&self, snapshot: &UndoSnapshot) -> MvResult<()> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let snapshot_json = serde_json::to_string(&snapshot.snapshot_data)?;

        conn.execute(
			"INSERT INTO proposal_undo_snapshots (id, proposal_id, snapshot_data, created_at, expires_at, used)
			 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
			params![
				snapshot.id.to_string(),
				snapshot.proposal_id.to_string(),
				snapshot_json,
				snapshot.created_at.to_rfc3339(),
				snapshot.expires_at.to_rfc3339(),
				snapshot.used,
			],
		)
		.map_err(|e| MvError::Storage(format!("insert undo_snapshot failed: {e}")))?;
        Ok(())
    }

    async fn get_undo_snapshot(&self, proposal_id: Uuid) -> MvResult<Option<UndoSnapshot>> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, proposal_id, snapshot_data, created_at, expires_at, used
				 FROM proposal_undo_snapshots WHERE proposal_id = ?1",
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let result = stmt
            .query_row(params![proposal_id.to_string()], row_to_undo_snapshot)
            .optional()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(result)
    }

    async fn mark_undo_used(&self, id: Uuid) -> MvResult<bool> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let affected = conn
            .execute(
                "UPDATE proposal_undo_snapshots SET used = 1 WHERE id = ?1",
                params![id.to_string()],
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(affected > 0)
    }

    async fn cleanup_expired_snapshots(&self) -> MvResult<usize> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let now = Utc::now().to_rfc3339();
        let affected = conn
            .execute(
                "DELETE FROM proposal_undo_snapshots WHERE expires_at < ?1",
                params![now],
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(affected)
    }
}

// ---------------------------------------------------------------------------
// FeedbackStore – Agent Feedback / Reflection
// ---------------------------------------------------------------------------

fn row_to_agent_feedback(row: &rusqlite::Row<'_>) -> rusqlite::Result<AgentFeedback> {
    let id_str: String = row.get(0)?;
    let intent_id_str: Option<String> = row.get(1)?;
    let intent_type: String = row.get(2)?;
    let action: String = row.get(3)?;
    let confidence_at_time: Option<f64> = row.get(4)?;
    let user_edit_delta: Option<f64> = row.get(5)?;
    let response_time_ms: Option<i64> = row.get(6)?;
    let created_at: String = row.get(7)?;

    let id = parse_uuid_str(0, &id_str)?;
    let intent_id = intent_id_str.and_then(|s| Uuid::parse_str(&s).ok());
    let created = chrono::DateTime::parse_from_rfc3339(&created_at)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());

    Ok(AgentFeedback {
        id,
        intent_id,
        intent_type,
        action,
        confidence_at_time: confidence_at_time.map(|v| v as f32),
        user_edit_delta: user_edit_delta.map(|v| v as f32),
        response_time_ms: response_time_ms.map(|v| v as u64),
        created_at: created,
    })
}

fn row_to_confidence_override(row: &rusqlite::Row<'_>) -> rusqlite::Result<ConfidenceOverride> {
    let intent_type: String = row.get(0)?;
    let base_adjustment: f64 = row.get(1)?;
    let auto_apply_threshold: f64 = row.get(2)?;
    let suppress_below: f64 = row.get(3)?;
    let updated_at: String = row.get(4)?;

    let updated = chrono::DateTime::parse_from_rfc3339(&updated_at)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());

    Ok(ConfidenceOverride {
        intent_type,
        base_adjustment: base_adjustment as f32,
        auto_apply_threshold: auto_apply_threshold as f32,
        suppress_below: suppress_below as f32,
        updated_at: updated,
    })
}

#[async_trait]
impl FeedbackStore for SqliteNodeStore {
    async fn record_feedback(&self, fb: &AgentFeedback) -> MvResult<()> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        conn.execute(
			"INSERT INTO agent_feedback (id, intent_id, intent_type, action, confidence_at_time, user_edit_delta, response_time_ms, created_at)
			 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
			params![
				fb.id.to_string(),
				fb.intent_id.map(|id| id.to_string()),
				fb.intent_type,
				fb.action,
				fb.confidence_at_time.map(|v| v as f64),
				fb.user_edit_delta.map(|v| v as f64),
				fb.response_time_ms.map(|v| v as i64),
				fb.created_at.to_rfc3339(),
			],
		)
		.map_err(|e| MvError::Storage(format!("insert agent_feedback failed: {e}")))?;
        Ok(())
    }

    async fn list_feedback(
        &self,
        intent_type: Option<&str>,
        limit: usize,
    ) -> MvResult<Vec<AgentFeedback>> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut sql = String::from(
			"SELECT id, intent_id, intent_type, action, confidence_at_time, user_edit_delta, response_time_ms, created_at
			 FROM agent_feedback WHERE 1=1",
		);
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        let mut param_idx = 1;

        if let Some(it) = intent_type {
            sql.push_str(&format!(" AND intent_type = ?{param_idx}"));
            param_values.push(Box::new(it.to_string()));
            param_idx += 1;
        }

        sql.push_str(&format!(" ORDER BY created_at DESC LIMIT ?{param_idx}"));
        param_values.push(Box::new(limit as i64));

        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let rows = stmt
            .query_map(params_refs.as_slice(), row_to_agent_feedback)
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| MvError::Storage(e.to_string()))?);
        }
        Ok(results)
    }

    async fn get_acceptance_rate(&self, intent_type: &str) -> MvResult<(usize, usize)> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let total: usize = conn
            .query_row(
                "SELECT COUNT(*) FROM agent_feedback WHERE intent_type = ?1",
                params![intent_type],
                |row| row.get(0),
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let applied: usize = conn
            .query_row(
                "SELECT COUNT(*) FROM agent_feedback WHERE intent_type = ?1 AND action = 'applied'",
                params![intent_type],
                |row| row.get(0),
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;

        Ok((applied, total))
    }

    async fn set_confidence_override(&self, override_: &ConfidenceOverride) -> MvResult<()> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        conn.execute(
			"INSERT OR REPLACE INTO agent_confidence_overrides (intent_type, base_adjustment, auto_apply_threshold, suppress_below, updated_at)
			 VALUES (?1, ?2, ?3, ?4, ?5)",
			params![
				override_.intent_type,
				override_.base_adjustment as f64,
				override_.auto_apply_threshold as f64,
				override_.suppress_below as f64,
				override_.updated_at.to_rfc3339(),
			],
		)
		.map_err(|e| MvError::Storage(format!("upsert confidence_override failed: {e}")))?;
        Ok(())
    }

    async fn get_confidence_override(
        &self,
        intent_type: &str,
    ) -> MvResult<Option<ConfidenceOverride>> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let mut stmt = conn
			.prepare(
				"SELECT intent_type, base_adjustment, auto_apply_threshold, suppress_below, updated_at
				 FROM agent_confidence_overrides WHERE intent_type = ?1",
			)
			.map_err(|e| MvError::Storage(e.to_string()))?;

        let result = stmt
            .query_row(params![intent_type], row_to_confidence_override)
            .optional()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(result)
    }

    async fn list_confidence_overrides(&self) -> MvResult<Vec<ConfidenceOverride>> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let mut stmt = conn
			.prepare(
				"SELECT intent_type, base_adjustment, auto_apply_threshold, suppress_below, updated_at
				 FROM agent_confidence_overrides ORDER BY intent_type",
			)
			.map_err(|e| MvError::Storage(e.to_string()))?;

        let rows = stmt
            .query_map([], row_to_confidence_override)
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| MvError::Storage(e.to_string()))?);
        }
        Ok(results)
    }
}

// ---------------------------------------------------------------------------
// ProfileStore – Owner Profile
// ---------------------------------------------------------------------------

use mv_core::{OwnerProfile, ProfileStore, UpdateProfileRequest};

#[async_trait]
impl ProfileStore for SqliteNodeStore {
    async fn get_profile(&self) -> MvResult<OwnerProfile> {
        self.with_conn(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT display_name, avatar_url, bio, email, preferred_namespace,
                            default_node_kind, preferred_llm_provider, timezone,
                            signature_name, signature_public_key, metadata,
                            created_at, updated_at
                     FROM owner_profile WHERE id = 'owner'",
                )
                .map_err(|e| MvError::Storage(e.to_string()))?;

            let profile = stmt
                .query_row([], |row| {
                    let metadata_str: String = row.get(10)?;
                    let metadata: std::collections::HashMap<String, serde_json::Value> =
                        serde_json::from_str(&metadata_str).unwrap_or_default();
                    let created_str: String = row.get(11)?;
                    let updated_str: String = row.get(12)?;
                    Ok(OwnerProfile {
                        display_name: row.get(0)?,
                        avatar_url: row.get(1)?,
                        bio: row.get(2)?,
                        email: row.get(3)?,
                        preferred_namespace: row.get(4)?,
                        default_node_kind: row.get(5)?,
                        preferred_llm_provider: row.get(6)?,
                        timezone: row.get(7)?,
                        signature_name: row.get(8)?,
                        signature_public_key: row.get(9)?,
                        metadata,
                        created_at: chrono::DateTime::parse_from_rfc3339(&created_str)
                            .map(|d| d.with_timezone(&chrono::Utc))
                            .unwrap_or_else(|_| chrono::Utc::now()),
                        updated_at: chrono::DateTime::parse_from_rfc3339(&updated_str)
                            .map(|d| d.with_timezone(&chrono::Utc))
                            .unwrap_or_else(|_| chrono::Utc::now()),
                    })
                })
                .map_err(|e| MvError::Storage(e.to_string()))?;

            Ok(profile)
        })
    }

    async fn update_profile(&self, req: &UpdateProfileRequest) -> MvResult<OwnerProfile> {
        self.with_conn(|conn| {
            let mut sets: Vec<&str> = Vec::new();
            let mut values: Vec<String> = Vec::new();

            if let Some(ref v) = req.display_name {
                sets.push("display_name = ?");
                values.push(v.clone());
            }
            if let Some(ref v) = req.avatar_url {
                sets.push("avatar_url = ?");
                values.push(v.clone());
            }
            if let Some(ref v) = req.bio {
                sets.push("bio = ?");
                values.push(v.clone());
            }
            if let Some(ref v) = req.email {
                sets.push("email = ?");
                values.push(v.clone());
            }
            if let Some(ref v) = req.preferred_namespace {
                sets.push("preferred_namespace = ?");
                values.push(v.clone());
            }
            if let Some(ref v) = req.default_node_kind {
                sets.push("default_node_kind = ?");
                values.push(v.clone());
            }
            if let Some(ref v) = req.preferred_llm_provider {
                sets.push("preferred_llm_provider = ?");
                values.push(v.clone());
            }
            if let Some(ref v) = req.timezone {
                sets.push("timezone = ?");
                values.push(v.clone());
            }
            if let Some(ref v) = req.signature_name {
                sets.push("signature_name = ?");
                values.push(v.clone());
            }
            if let Some(ref v) = req.signature_public_key {
                sets.push("signature_public_key = ?");
                values.push(v.clone());
            }
            if let Some(ref v) = req.metadata {
                sets.push("metadata = ?");
                values.push(serde_json::to_string(v).unwrap_or_default());
            }

            if !sets.is_empty() {
                sets.push("updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now')");
                let sql = format!(
                    "UPDATE owner_profile SET {} WHERE id = 'owner'",
                    sets.join(", ")
                );
                let params: Vec<&dyn rusqlite::types::ToSql> = values
                    .iter()
                    .map(|v| v as &dyn rusqlite::types::ToSql)
                    .collect();
                conn.execute(&sql, params.as_slice())
                    .map_err(|e| MvError::Storage(e.to_string()))?;
            }
            Ok(())
        })?;

        self.get_profile().await
    }
}

// ---------------------------------------------------------------------------
// AutonomyStore – Autonomy & Precision Controls (Phase 3.1)
// ---------------------------------------------------------------------------

use mv_core::{AutonomyActionLog, AutonomyDecision, AutonomyRule, AutonomyStore};

fn row_to_autonomy_rule(row: &rusqlite::Row<'_>) -> rusqlite::Result<AutonomyRule> {
    let id_str: String = row.get(0)?;
    let rule_type: String = row.get(1)?;
    let scope_key: Option<String> = row.get(2)?;
    let auto_apply_threshold: f64 = row.get(3)?;
    let max_actions_per_hour: i64 = row.get(4)?;
    let allowed_csv: Option<String> = row.get(5)?;
    let blocked_csv: Option<String> = row.get(6)?;
    let quiet_hours_start: Option<String> = row.get(7)?;
    let quiet_hours_end: Option<String> = row.get(8)?;
    let quiet_hours_timezone: String = row.get(9)?;
    let enabled: bool = row.get(10)?;
    let created_at: String = row.get(11)?;
    let updated_at: Option<String> = row.get(12)?;

    let id = parse_uuid_str(0, &id_str)?;

    let allowed_intent_types: Vec<String> = allowed_csv
        .map(|csv| {
            csv.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();
    let blocked_intent_types: Vec<String> = blocked_csv
        .map(|csv| {
            csv.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();

    Ok(AutonomyRule {
        id,
        rule_type,
        scope_key,
        auto_apply_threshold: auto_apply_threshold as f32,
        max_actions_per_hour: max_actions_per_hour as u32,
        allowed_intent_types,
        blocked_intent_types,
        quiet_hours_start,
        quiet_hours_end,
        quiet_hours_timezone,
        enabled,
        created_at: parse_dt_strict(11, &created_at)?,
        updated_at: parse_optional_dt_strict(12, updated_at)?,
    })
}

fn row_to_autonomy_action_log(row: &rusqlite::Row<'_>) -> rusqlite::Result<AutonomyActionLog> {
    let id_str: String = row.get(0)?;
    let rule_id_str: Option<String> = row.get(1)?;
    let intent_type: String = row.get(2)?;
    let decision_str: String = row.get(3)?;
    let confidence: Option<f64> = row.get(4)?;
    let reason: Option<String> = row.get(5)?;
    let created_at: String = row.get(6)?;

    let id = parse_uuid_str(0, &id_str)?;
    let rule_id = rule_id_str.and_then(|s| Uuid::parse_str(&s).ok());
    let decision: AutonomyDecision = decision_str.parse().map_err(|e: String| {
        rusqlite::Error::FromSqlConversionFailure(
            3,
            Type::Text,
            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
        )
    })?;

    Ok(AutonomyActionLog {
        id,
        rule_id,
        intent_type,
        decision,
        confidence: confidence.map(|v| v as f32),
        reason,
        created_at: parse_dt_strict(6, &created_at)?,
    })
}

#[async_trait]
impl AutonomyStore for SqliteNodeStore {
    async fn add_autonomy_rule(&self, rule: &AutonomyRule) -> MvResult<()> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let allowed_csv = if rule.allowed_intent_types.is_empty() {
            None
        } else {
            Some(rule.allowed_intent_types.join(","))
        };
        let blocked_csv = if rule.blocked_intent_types.is_empty() {
            None
        } else {
            Some(rule.blocked_intent_types.join(","))
        };

        conn.execute(
			"INSERT INTO autonomy_rules (id, rule_type, scope_key, auto_apply_threshold, max_actions_per_hour, allowed_intent_types, blocked_intent_types, quiet_hours_start, quiet_hours_end, quiet_hours_timezone, enabled, created_at, updated_at)
			 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
			params![
				rule.id.to_string(),
				rule.rule_type,
				rule.scope_key,
				rule.auto_apply_threshold as f64,
				rule.max_actions_per_hour as i64,
				allowed_csv,
				blocked_csv,
				rule.quiet_hours_start,
				rule.quiet_hours_end,
				rule.quiet_hours_timezone,
				rule.enabled,
				rule.created_at.to_rfc3339(),
				rule.updated_at.map(|dt| dt.to_rfc3339()),
			],
		)
		.map_err(|e| MvError::Storage(format!("insert autonomy_rule failed: {e}")))?;
        Ok(())
    }

    async fn get_autonomy_rule(&self, id: Uuid) -> MvResult<Option<AutonomyRule>> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let mut stmt = conn
			.prepare(
				"SELECT id, rule_type, scope_key, auto_apply_threshold, max_actions_per_hour, allowed_intent_types, blocked_intent_types, quiet_hours_start, quiet_hours_end, quiet_hours_timezone, enabled, created_at, updated_at
				 FROM autonomy_rules WHERE id = ?1",
			)
			.map_err(|e| MvError::Storage(e.to_string()))?;

        let result = stmt
            .query_row(params![id.to_string()], row_to_autonomy_rule)
            .optional()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(result)
    }

    async fn list_autonomy_rules(&self) -> MvResult<Vec<AutonomyRule>> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let mut stmt = conn
			.prepare(
				"SELECT id, rule_type, scope_key, auto_apply_threshold, max_actions_per_hour, allowed_intent_types, blocked_intent_types, quiet_hours_start, quiet_hours_end, quiet_hours_timezone, enabled, created_at, updated_at
				 FROM autonomy_rules ORDER BY created_at DESC",
			)
			.map_err(|e| MvError::Storage(e.to_string()))?;

        let rows = stmt
            .query_map([], row_to_autonomy_rule)
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(|e| MvError::Storage(e.to_string()))?);
        }
        Ok(result)
    }

    async fn update_autonomy_rule(&self, rule: &AutonomyRule) -> MvResult<bool> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let allowed_csv = if rule.allowed_intent_types.is_empty() {
            None
        } else {
            Some(rule.allowed_intent_types.join(","))
        };
        let blocked_csv = if rule.blocked_intent_types.is_empty() {
            None
        } else {
            Some(rule.blocked_intent_types.join(","))
        };
        let now = Utc::now().to_rfc3339();

        let affected = conn
			.execute(
				"UPDATE autonomy_rules SET rule_type = ?2, scope_key = ?3, auto_apply_threshold = ?4, max_actions_per_hour = ?5, allowed_intent_types = ?6, blocked_intent_types = ?7, quiet_hours_start = ?8, quiet_hours_end = ?9, quiet_hours_timezone = ?10, enabled = ?11, updated_at = ?12 WHERE id = ?1",
				params![
					rule.id.to_string(),
					rule.rule_type,
					rule.scope_key,
					rule.auto_apply_threshold as f64,
					rule.max_actions_per_hour as i64,
					allowed_csv,
					blocked_csv,
					rule.quiet_hours_start,
					rule.quiet_hours_end,
					rule.quiet_hours_timezone,
					rule.enabled,
					now,
				],
			)
			.map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(affected > 0)
    }

    async fn delete_autonomy_rule(&self, id: Uuid) -> MvResult<bool> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let affected = conn
            .execute(
                "DELETE FROM autonomy_rules WHERE id = ?1",
                params![id.to_string()],
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(affected > 0)
    }

    async fn log_autonomy_action(&self, log: &AutonomyActionLog) -> MvResult<()> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        conn.execute(
			"INSERT INTO autonomy_action_log (id, rule_id, intent_type, decision, confidence, reason, created_at)
			 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
			params![
				log.id.to_string(),
				log.rule_id.map(|id| id.to_string()),
				log.intent_type,
				log.decision.to_string(),
				log.confidence.map(|v| v as f64),
				log.reason,
				log.created_at.to_rfc3339(),
			],
		)
		.map_err(|e| MvError::Storage(format!("insert autonomy_action_log failed: {e}")))?;
        Ok(())
    }

    async fn count_recent_actions(
        &self,
        rule_id: Option<Uuid>,
        since: DateTime<Utc>,
    ) -> MvResult<usize> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let (sql, params_box): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = if let Some(rid) =
            rule_id
        {
            (
				"SELECT COUNT(*) FROM autonomy_action_log WHERE rule_id = ?1 AND decision = 'auto_apply' AND created_at >= ?2".to_string(),
				vec![Box::new(rid.to_string()), Box::new(since.to_rfc3339())],
			)
        } else {
            (
				"SELECT COUNT(*) FROM autonomy_action_log WHERE decision = 'auto_apply' AND created_at >= ?1".to_string(),
				vec![Box::new(since.to_rfc3339())],
			)
        };

        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            params_box.iter().map(|p| p.as_ref()).collect();

        let count: usize = conn
            .query_row(&sql, params_refs.as_slice(), |row| row.get(0))
            .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(count)
    }

    async fn list_autonomy_action_log(&self, limit: usize) -> MvResult<Vec<AutonomyActionLog>> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, rule_id, intent_type, decision, confidence, reason, created_at
				 FROM autonomy_action_log ORDER BY created_at DESC LIMIT ?1",
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let rows = stmt
            .query_map(params![limit as i64], row_to_autonomy_action_log)
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(|e| MvError::Storage(e.to_string()))?);
        }
        Ok(result)
    }
}

// ---------------------------------------------------------------------------
// RelayStore – Communication Relay Network (Phase 3.2)
// ---------------------------------------------------------------------------

use mv_core::{
    ChannelType, ContentType, MessageDirection, MessageStatus, RelayChannel, RelayContact,
    RelayMessage, RelayStore, TrustLevel,
};

fn row_to_relay_contact(row: &rusqlite::Row<'_>) -> rusqlite::Result<RelayContact> {
    let id_str: String = row.get(0)?;
    let display_name: String = row.get(1)?;
    let public_key: String = row.get(2)?;
    let vault_address: Option<String> = row.get(3)?;
    let trust_level_str: String = row.get(4)?;
    let autonomy_rule_id_str: Option<String> = row.get(5)?;
    let notes: Option<String> = row.get(6)?;
    let created_at_str: String = row.get(7)?;
    let updated_at_str: Option<String> = row.get(8)?;

    let id = parse_uuid_str(0, &id_str)?;
    let trust_level: TrustLevel = trust_level_str.parse().unwrap_or(TrustLevel::RelayOnly);
    let autonomy_rule_id = autonomy_rule_id_str.and_then(|s| Uuid::parse_str(&s).ok());
    let created_at = parse_dt_strict(7, &created_at_str)?;
    let updated_at = parse_optional_dt_strict(8, updated_at_str)?;

    Ok(RelayContact {
        id,
        display_name,
        public_key,
        vault_address,
        trust_level,
        autonomy_rule_id,
        notes,
        created_at,
        updated_at,
    })
}

fn row_to_relay_channel(row: &rusqlite::Row<'_>) -> rusqlite::Result<RelayChannel> {
    let id_str: String = row.get(0)?;
    let name: Option<String> = row.get(1)?;
    let channel_type_str: String = row.get(2)?;
    let member_ids_json: String = row.get(3)?;
    let created_at_str: String = row.get(4)?;
    let updated_at_str: Option<String> = row.get(5)?;

    let id = parse_uuid_str(0, &id_str)?;
    let channel_type: ChannelType = channel_type_str.parse().unwrap_or(ChannelType::Direct);
    let member_contact_ids: Vec<Uuid> = serde_json::from_str(&member_ids_json).unwrap_or_default();
    let created_at = parse_dt_strict(4, &created_at_str)?;
    let updated_at = parse_optional_dt_strict(5, updated_at_str)?;

    Ok(RelayChannel {
        id,
        name,
        channel_type,
        member_contact_ids,
        created_at,
        updated_at,
    })
}

fn row_to_relay_message(row: &rusqlite::Row<'_>) -> rusqlite::Result<RelayMessage> {
    let id_str: String = row.get(0)?;
    let channel_id_str: String = row.get(1)?;
    let thread_id_str: Option<String> = row.get(2)?;
    let sender_contact_id_str: Option<String> = row.get(3)?;
    let recipient_contact_id_str: Option<String> = row.get(4)?;
    let direction_str: String = row.get(5)?;
    let content: String = row.get(6)?;
    let content_type_str: String = row.get(7)?;
    let status_str: String = row.get(8)?;
    let vault_node_id_str: Option<String> = row.get(9)?;
    let metadata_json: String = row.get(10)?;
    let created_at_str: String = row.get(11)?;
    let updated_at_str: Option<String> = row.get(12)?;

    let id = parse_uuid_str(0, &id_str)?;
    let channel_id = parse_uuid_str(1, &channel_id_str)?;
    let thread_id = thread_id_str.and_then(|s| Uuid::parse_str(&s).ok());
    let sender_contact_id = sender_contact_id_str.and_then(|s| Uuid::parse_str(&s).ok());
    let recipient_contact_id = recipient_contact_id_str.and_then(|s| Uuid::parse_str(&s).ok());
    let direction: MessageDirection = direction_str.parse().unwrap_or(MessageDirection::Outbound);
    let content_type: ContentType = content_type_str.parse().unwrap_or(ContentType::Text);
    let status: MessageStatus = status_str.parse().unwrap_or(MessageStatus::Pending);
    let vault_node_id = vault_node_id_str.and_then(|s| Uuid::parse_str(&s).ok());
    let metadata: std::collections::HashMap<String, serde_json::Value> =
        serde_json::from_str(&metadata_json).unwrap_or_default();
    let created_at = parse_dt_strict(11, &created_at_str)?;
    let updated_at = parse_optional_dt_strict(12, updated_at_str)?;

    Ok(RelayMessage {
        id,
        channel_id,
        thread_id,
        sender_contact_id,
        recipient_contact_id,
        direction,
        content,
        content_type,
        status,
        vault_node_id,
        metadata,
        created_at,
        updated_at,
    })
}

#[async_trait]
impl RelayStore for SqliteNodeStore {
    // ── Contacts ──────────────────────────────────────────────────────────

    async fn add_relay_contact(&self, contact: &RelayContact) -> MvResult<()> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        conn.execute(
			"INSERT INTO relay_contacts (id, display_name, public_key, vault_address, trust_level, autonomy_rule_id, notes, created_at, updated_at)
			 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
			params![
				contact.id.to_string(),
				contact.display_name,
				contact.public_key,
				contact.vault_address,
				contact.trust_level.to_string(),
				contact.autonomy_rule_id.map(|id| id.to_string()),
				contact.notes,
				contact.created_at.to_rfc3339(),
				contact.updated_at.map(|dt| dt.to_rfc3339()),
			],
		)
		.map_err(|e| MvError::Storage(format!("insert relay_contact failed: {e}")))?;
        Ok(())
    }

    async fn get_relay_contact(&self, id: Uuid) -> MvResult<Option<RelayContact>> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let mut stmt = conn
			.prepare(
				"SELECT id, display_name, public_key, vault_address, trust_level, autonomy_rule_id, notes, created_at, updated_at
				 FROM relay_contacts WHERE id = ?1",
			)
			.map_err(|e| MvError::Storage(e.to_string()))?;

        let result = stmt
            .query_row(params![id.to_string()], row_to_relay_contact)
            .optional()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(result)
    }

    async fn list_relay_contacts(&self) -> MvResult<Vec<RelayContact>> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let mut stmt = conn
			.prepare(
				"SELECT id, display_name, public_key, vault_address, trust_level, autonomy_rule_id, notes, created_at, updated_at
				 FROM relay_contacts ORDER BY created_at DESC",
			)
			.map_err(|e| MvError::Storage(e.to_string()))?;

        let rows = stmt
            .query_map([], row_to_relay_contact)
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(|e| MvError::Storage(e.to_string()))?);
        }
        Ok(result)
    }

    async fn update_relay_contact(&self, contact: &RelayContact) -> MvResult<bool> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let now = Utc::now().to_rfc3339();
        let affected = conn
			.execute(
				"UPDATE relay_contacts SET display_name = ?2, public_key = ?3, vault_address = ?4, trust_level = ?5, autonomy_rule_id = ?6, notes = ?7, updated_at = ?8 WHERE id = ?1",
				params![
					contact.id.to_string(),
					contact.display_name,
					contact.public_key,
					contact.vault_address,
					contact.trust_level.to_string(),
					contact.autonomy_rule_id.map(|id| id.to_string()),
					contact.notes,
					now,
				],
			)
			.map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(affected > 0)
    }

    async fn delete_relay_contact(&self, id: Uuid) -> MvResult<bool> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let affected = conn
            .execute(
                "DELETE FROM relay_contacts WHERE id = ?1",
                params![id.to_string()],
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(affected > 0)
    }

    // ── Channels ──────────────────────────────────────────────────────────

    async fn add_relay_channel(&self, channel: &RelayChannel) -> MvResult<()> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let member_ids_json = serde_json::to_string(&channel.member_contact_ids)
            .map_err(|e| MvError::Storage(e.to_string()))?;
        conn.execute(
			"INSERT INTO relay_channels (id, name, channel_type, member_contact_ids, created_at, updated_at)
			 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
			params![
				channel.id.to_string(),
				channel.name,
				channel.channel_type.to_string(),
				member_ids_json,
				channel.created_at.to_rfc3339(),
				channel.updated_at.map(|dt| dt.to_rfc3339()),
			],
		)
		.map_err(|e| MvError::Storage(format!("insert relay_channel failed: {e}")))?;
        Ok(())
    }

    async fn get_relay_channel(&self, id: Uuid) -> MvResult<Option<RelayChannel>> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, channel_type, member_contact_ids, created_at, updated_at
				 FROM relay_channels WHERE id = ?1",
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let result = stmt
            .query_row(params![id.to_string()], row_to_relay_channel)
            .optional()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(result)
    }

    async fn list_relay_channels(&self) -> MvResult<Vec<RelayChannel>> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, channel_type, member_contact_ids, created_at, updated_at
				 FROM relay_channels ORDER BY created_at DESC",
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let rows = stmt
            .query_map([], row_to_relay_channel)
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(|e| MvError::Storage(e.to_string()))?);
        }
        Ok(result)
    }

    async fn delete_relay_channel(&self, id: Uuid) -> MvResult<bool> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let affected = conn
            .execute(
                "DELETE FROM relay_channels WHERE id = ?1",
                params![id.to_string()],
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(affected > 0)
    }

    // ── Messages ──────────────────────────────────────────────────────────

    async fn add_relay_message(&self, message: &RelayMessage) -> MvResult<()> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let metadata_json = serde_json::to_string(&message.metadata)
            .map_err(|e| MvError::Storage(e.to_string()))?;
        conn.execute(
			"INSERT INTO relay_messages (id, channel_id, thread_id, sender_contact_id, recipient_contact_id, direction, content, content_type, status, vault_node_id, metadata, created_at, updated_at)
			 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
			params![
				message.id.to_string(),
				message.channel_id.to_string(),
				message.thread_id.map(|id| id.to_string()),
				message.sender_contact_id.map(|id| id.to_string()),
				message.recipient_contact_id.map(|id| id.to_string()),
				message.direction.to_string(),
				message.content,
				message.content_type.to_string(),
				message.status.to_string(),
				message.vault_node_id.map(|id| id.to_string()),
				metadata_json,
				message.created_at.to_rfc3339(),
				message.updated_at.map(|dt| dt.to_rfc3339()),
			],
		)
		.map_err(|e| MvError::Storage(format!("insert relay_message failed: {e}")))?;
        Ok(())
    }

    async fn get_relay_message(&self, id: Uuid) -> MvResult<Option<RelayMessage>> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let mut stmt = conn
			.prepare(
				"SELECT id, channel_id, thread_id, sender_contact_id, recipient_contact_id, direction, content, content_type, status, vault_node_id, metadata, created_at, updated_at
				 FROM relay_messages WHERE id = ?1",
			)
			.map_err(|e| MvError::Storage(e.to_string()))?;

        let result = stmt
            .query_row(params![id.to_string()], row_to_relay_message)
            .optional()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(result)
    }

    async fn list_relay_messages(
        &self,
        channel_id: Uuid,
        limit: usize,
        offset: usize,
    ) -> MvResult<Vec<RelayMessage>> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let mut stmt = conn
			.prepare(
				"SELECT id, channel_id, thread_id, sender_contact_id, recipient_contact_id, direction, content, content_type, status, vault_node_id, metadata, created_at, updated_at
				 FROM relay_messages WHERE channel_id = ?1 ORDER BY created_at DESC LIMIT ?2 OFFSET ?3",
			)
			.map_err(|e| MvError::Storage(e.to_string()))?;

        let rows = stmt
            .query_map(
                params![channel_id.to_string(), limit as i64, offset as i64],
                row_to_relay_message,
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(|e| MvError::Storage(e.to_string()))?);
        }
        Ok(result)
    }

    async fn update_message_status(&self, id: Uuid, status: MessageStatus) -> MvResult<bool> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let now = Utc::now().to_rfc3339();
        let affected = conn
            .execute(
                "UPDATE relay_messages SET status = ?2, updated_at = ?3 WHERE id = ?1",
                params![id.to_string(), status.to_string(), now],
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(affected > 0)
    }

    async fn list_thread_messages(
        &self,
        thread_id: Uuid,
        limit: usize,
    ) -> MvResult<Vec<RelayMessage>> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let mut stmt = conn
			.prepare(
				"SELECT id, channel_id, thread_id, sender_contact_id, recipient_contact_id, direction, content, content_type, status, vault_node_id, metadata, created_at, updated_at
				 FROM relay_messages WHERE thread_id = ?1 ORDER BY created_at ASC LIMIT ?2",
			)
			.map_err(|e| MvError::Storage(e.to_string()))?;

        let rows = stmt
            .query_map(
                params![thread_id.to_string(), limit as i64],
                row_to_relay_message,
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(|e| MvError::Storage(e.to_string()))?);
        }
        Ok(result)
    }

    async fn count_unread_messages(&self, channel_id: Option<Uuid>) -> MvResult<usize> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let (sql, params_box): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = if let Some(cid) =
            channel_id
        {
            (
				"SELECT COUNT(*) FROM relay_messages WHERE status NOT IN ('read') AND direction = 'inbound' AND channel_id = ?1".to_string(),
				vec![Box::new(cid.to_string())],
			)
        } else {
            (
				"SELECT COUNT(*) FROM relay_messages WHERE status NOT IN ('read') AND direction = 'inbound'".to_string(),
				vec![],
			)
        };

        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            params_box.iter().map(|p| p.as_ref()).collect();

        let count: usize = conn
            .query_row(&sql, params_refs.as_slice(), |row| row.get(0))
            .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(count)
    }
}

// ---------------------------------------------------------------------------
// ConsumerStore – Consumer Profiles for AI identity
// ---------------------------------------------------------------------------

use mv_core::{ConsumerProfile, ConsumerStore};

#[async_trait]
impl ConsumerStore for SqliteNodeStore {
    async fn create_consumer(&self, profile: &ConsumerProfile) -> MvResult<()> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let metadata_json = serde_json::to_string(&profile.metadata)
            .map_err(|e| MvError::Storage(e.to_string()))?;
        conn.execute(
            "INSERT INTO consumer_profiles (id, name, description, token_hash, created_at, last_used_at, revoked_at, metadata_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                profile.id.to_string(),
                profile.name,
                profile.description,
                profile.token_hash,
                profile.created_at.to_rfc3339(),
                profile.last_used_at.map(|dt| dt.to_rfc3339()),
                profile.revoked_at.map(|dt| dt.to_rfc3339()),
                metadata_json,
            ],
        )
        .map_err(|e| MvError::Storage(format!("insert consumer_profile failed: {e}")))?;
        Ok(())
    }

    async fn get_consumer(&self, id: Uuid) -> MvResult<Option<ConsumerProfile>> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, description, token_hash, created_at, last_used_at, revoked_at, metadata_json
                 FROM consumer_profiles WHERE id = ?1",
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let result = stmt
            .query_row(params![id.to_string()], row_to_consumer)
            .optional()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(result)
    }

    async fn get_consumer_by_name(&self, name: &str) -> MvResult<Option<ConsumerProfile>> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, description, token_hash, created_at, last_used_at, revoked_at, metadata_json
                 FROM consumer_profiles WHERE name = ?1",
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let result = stmt
            .query_row(params![name], row_to_consumer)
            .optional()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(result)
    }

    async fn get_consumer_by_token_hash(
        &self,
        token_hash: &str,
    ) -> MvResult<Option<ConsumerProfile>> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, description, token_hash, created_at, last_used_at, revoked_at, metadata_json
                 FROM consumer_profiles WHERE token_hash = ?1 AND revoked_at IS NULL",
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let result = stmt
            .query_row(params![token_hash], row_to_consumer)
            .optional()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(result)
    }

    async fn list_consumers(&self) -> MvResult<Vec<ConsumerProfile>> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, description, token_hash, created_at, last_used_at, revoked_at, metadata_json
                 FROM consumer_profiles ORDER BY created_at DESC",
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let rows = stmt
            .query_map([], row_to_consumer)
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(|e| MvError::Storage(e.to_string()))?);
        }
        Ok(result)
    }

    async fn revoke_consumer(&self, id: Uuid) -> MvResult<bool> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let now = Utc::now().to_rfc3339();
        let affected = conn
            .execute(
                "UPDATE consumer_profiles SET revoked_at = ?2 WHERE id = ?1 AND revoked_at IS NULL",
                params![id.to_string(), now],
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(affected > 0)
    }

    async fn touch_consumer(&self, id: Uuid) -> MvResult<()> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE consumer_profiles SET last_used_at = ?2 WHERE id = ?1",
            params![id.to_string(), now],
        )
        .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(())
    }
}

fn row_to_consumer(row: &rusqlite::Row<'_>) -> rusqlite::Result<ConsumerProfile> {
    let id_str: String = row.get(0)?;
    let name: String = row.get(1)?;
    let description: Option<String> = row.get(2)?;
    let token_hash: String = row.get(3)?;
    let created_at_str: String = row.get(4)?;
    let last_used_at_str: Option<String> = row.get(5)?;
    let revoked_at_str: Option<String> = row.get(6)?;
    let metadata_json: Option<String> = row.get(7)?;

    let id = parse_uuid_str(0, &id_str)?;
    let metadata = metadata_json
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    Ok(ConsumerProfile {
        id,
        name,
        description,
        token_hash,
        created_at: parse_dt_strict(4, &created_at_str)?,
        last_used_at: parse_optional_dt_strict(5, last_used_at_str)?,
        revoked_at: parse_optional_dt_strict(6, revoked_at_str)?,
        metadata,
    })
}

// ---------------------------------------------------------------------------
// PolicyStore – Access Policies (ABAC with default-deny)
// ---------------------------------------------------------------------------

use mv_core::{AccessPolicy, PolicyStore};

#[async_trait]
impl PolicyStore for SqliteNodeStore {
    async fn set_policy(&self, policy: &AccessPolicy) -> MvResult<()> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let scopes_json =
            serde_json::to_string(&policy.scopes).map_err(|e| MvError::Storage(e.to_string()))?;
        conn.execute(
            "INSERT OR REPLACE INTO access_policies (id, secret_key, consumer, allowed, scopes_json, max_ttl_seconds, expires_at, require_approval, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                policy.id.to_string(),
                policy.secret_key,
                policy.consumer,
                policy.allowed as i32,
                scopes_json,
                policy.max_ttl_seconds,
                policy.expires_at.map(|dt| dt.to_rfc3339()),
                policy.require_approval as i32,
                policy.created_at.to_rfc3339(),
                policy.updated_at.to_rfc3339(),
            ],
        )
        .map_err(|e| MvError::Storage(format!("upsert access_policy failed: {e}")))?;
        Ok(())
    }

    async fn get_policy(&self, id: Uuid) -> MvResult<Option<AccessPolicy>> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, secret_key, consumer, allowed, scopes_json, max_ttl_seconds, expires_at, require_approval, created_at, updated_at
                 FROM access_policies WHERE id = ?1",
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let result = stmt
            .query_row(params![id.to_string()], row_to_policy)
            .optional()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(result)
    }

    async fn get_policy_for(
        &self,
        secret_key: &str,
        consumer: &str,
    ) -> MvResult<Option<AccessPolicy>> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, secret_key, consumer, allowed, scopes_json, max_ttl_seconds, expires_at, require_approval, created_at, updated_at
                 FROM access_policies WHERE secret_key = ?1 AND consumer = ?2",
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let result = stmt
            .query_row(params![secret_key, consumer], row_to_policy)
            .optional()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(result)
    }

    async fn list_policies(
        &self,
        secret_key: Option<&str>,
        consumer: Option<&str>,
    ) -> MvResult<Vec<AccessPolicy>> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut sql = "SELECT id, secret_key, consumer, allowed, scopes_json, max_ttl_seconds, expires_at, require_approval, created_at, updated_at FROM access_policies".to_string();
        let mut conditions: Vec<String> = Vec::new();
        let mut params_box: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(sk) = secret_key {
            conditions.push(format!("secret_key = ?{}", params_box.len() + 1));
            params_box.push(Box::new(sk.to_string()));
        }
        if let Some(c) = consumer {
            conditions.push(format!("consumer = ?{}", params_box.len() + 1));
            params_box.push(Box::new(c.to_string()));
        }

        if !conditions.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&conditions.join(" AND "));
        }
        sql.push_str(" ORDER BY created_at DESC");

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            params_box.iter().map(|p| p.as_ref()).collect();
        let rows = stmt
            .query_map(params_refs.as_slice(), row_to_policy)
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(|e| MvError::Storage(e.to_string()))?);
        }
        Ok(result)
    }

    async fn delete_policy(&self, id: Uuid) -> MvResult<bool> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let affected = conn
            .execute(
                "DELETE FROM access_policies WHERE id = ?1",
                params![id.to_string()],
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(affected > 0)
    }
}

#[async_trait]
impl ShareStore for SqliteNodeStore {
    async fn insert_public_share(&self, share: &PublicShare) -> MvResult<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO public_shares (id, node_id, token_hash, created_at, expires_at, revoked_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    share.id.to_string(),
                    share.node_id.to_string(),
                    share.token_hash,
                    share.created_at.to_rfc3339(),
                    share.expires_at.map(|dt| dt.to_rfc3339()),
                    share.revoked_at.map(|dt| dt.to_rfc3339()),
                ],
            )
            .map_err(|e| MvError::Storage(format!("insert public_share failed: {e}")))?;
            Ok(())
        })
    }

    async fn get_public_share(&self, id: Uuid) -> MvResult<Option<PublicShare>> {
        self.with_conn(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT id, node_id, token_hash, created_at, expires_at, revoked_at
                     FROM public_shares WHERE id = ?1",
                )
                .map_err(|e| MvError::Storage(e.to_string()))?;
            let share = stmt
                .query_row(params![id.to_string()], row_to_public_share)
                .optional()
                .map_err(|e| MvError::Storage(format!("select public_share failed: {e}")))?;
            Ok(share)
        })
    }

    async fn get_public_share_by_hash(&self, token_hash: &str) -> MvResult<Option<PublicShare>> {
        self.with_conn(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT id, node_id, token_hash, created_at, expires_at, revoked_at
                     FROM public_shares WHERE token_hash = ?1",
                )
                .map_err(|e| MvError::Storage(e.to_string()))?;
            let share = stmt
                .query_row(params![token_hash], row_to_public_share)
                .optional()
                .map_err(|e| MvError::Storage(format!("select public_share failed: {e}")))?;
            Ok(share)
        })
    }

    async fn list_public_shares(
        &self,
        node_id: Option<Uuid>,
        include_revoked: bool,
    ) -> MvResult<Vec<PublicShare>> {
        self.with_conn(|conn| {
            let mut sql =
                "SELECT id, node_id, token_hash, created_at, expires_at, revoked_at FROM public_shares"
                    .to_string();
            let mut params_refs: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
            let mut conditions: Vec<String> = Vec::new();

            if let Some(node_id) = node_id {
                conditions.push(format!("node_id = ?{}", params_refs.len() + 1));
                params_refs.push(Box::new(node_id.to_string()));
            }

            if !include_revoked {
                conditions.push("revoked_at IS NULL".to_string());
            }

            if !conditions.is_empty() {
                sql.push_str(" WHERE ");
                sql.push_str(&conditions.join(" AND "));
            }

            sql.push_str(" ORDER BY created_at DESC");

            let mut stmt = conn
                .prepare(&sql)
                .map_err(|e| MvError::Storage(e.to_string()))?;

            let param_slice: Vec<&dyn rusqlite::ToSql> =
                params_refs.iter().map(|p| p.as_ref() as &dyn rusqlite::ToSql).collect();
            let shares = stmt
                .query_map(param_slice.as_slice(), row_to_public_share)
                .map_err(|e| MvError::Storage(format!("list public_shares failed: {e}")))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| MvError::Storage(format!("collect public_shares failed: {e}")))?;

            Ok(shares)
        })
    }

    async fn revoke_public_share(&self, id: Uuid, revoked_at: DateTime<Utc>) -> MvResult<bool> {
        self.with_conn(|conn| {
            let updated = conn
                .execute(
                    "UPDATE public_shares SET revoked_at = ?2 WHERE id = ?1",
                    params![id.to_string(), revoked_at.to_rfc3339()],
                )
                .map_err(|e| MvError::Storage(format!("revoke public_share failed: {e}")))?;
            Ok(updated > 0)
        })
    }
}

#[async_trait]
impl CommentStore for SqliteNodeStore {
    async fn insert_comment(&self, comment: &NodeComment) -> MvResult<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO node_comments (id, node_id, author, body, created_at, updated_at, resolved_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    comment.id.to_string(),
                    comment.node_id.to_string(),
                    comment.author,
                    comment.body,
                    comment.created_at.to_rfc3339(),
                    comment.updated_at.to_rfc3339(),
                    comment.resolved_at.map(|dt| dt.to_rfc3339()),
                ],
            )
            .map_err(|e| MvError::Storage(format!("insert comment failed: {e}")))?;
            Ok(())
        })
    }

    async fn get_comment(&self, id: Uuid) -> MvResult<Option<NodeComment>> {
        self.with_conn(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT id, node_id, author, body, created_at, updated_at, resolved_at
                     FROM node_comments WHERE id = ?1",
                )
                .map_err(|e| MvError::Storage(e.to_string()))?;
            let comment = stmt
                .query_row(params![id.to_string()], row_to_node_comment)
                .optional()
                .map_err(|e| MvError::Storage(format!("select comment failed: {e}")))?;
            Ok(comment)
        })
    }

    async fn list_comments(
        &self,
        node_id: Uuid,
        include_resolved: bool,
    ) -> MvResult<Vec<NodeComment>> {
        self.with_conn(|conn| {
            let mut sql = String::from(
                "SELECT id, node_id, author, body, created_at, updated_at, resolved_at
                 FROM node_comments WHERE node_id = ?1",
            );
            if !include_resolved {
                sql.push_str(" AND resolved_at IS NULL");
            }
            sql.push_str(" ORDER BY created_at DESC");

            let mut stmt = conn
                .prepare(&sql)
                .map_err(|e| MvError::Storage(e.to_string()))?;
            let comments = stmt
                .query_map(params![node_id.to_string()], row_to_node_comment)
                .map_err(|e| MvError::Storage(format!("list comments failed: {e}")))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| MvError::Storage(format!("collect comments failed: {e}")))?;
            Ok(comments)
        })
    }

    async fn resolve_comment(&self, id: Uuid, resolved_at: DateTime<Utc>) -> MvResult<bool> {
        self.with_conn(|conn| {
            let updated = conn
                .execute(
                    "UPDATE node_comments SET resolved_at = ?2, updated_at = ?2 WHERE id = ?1",
                    params![id.to_string(), resolved_at.to_rfc3339()],
                )
                .map_err(|e| MvError::Storage(format!("resolve comment failed: {e}")))?;
            Ok(updated > 0)
        })
    }

    async fn delete_comment(&self, id: Uuid) -> MvResult<bool> {
        self.with_conn(|conn| {
            let affected = conn
                .execute(
                    "DELETE FROM node_comments WHERE id = ?1",
                    params![id.to_string()],
                )
                .map_err(|e| MvError::Storage(format!("delete comment failed: {e}")))?;
            Ok(affected > 0)
        })
    }
}

#[async_trait]
impl McpConnectorStore for SqliteNodeStore {
    async fn insert_mcp_connector(&self, connector: &McpConnector) -> MvResult<()> {
        self.with_conn(|conn| {
            let config_schema_json = serde_json::to_string(&connector.config_schema)?;
            let capabilities_json = serde_json::to_string(&connector.capabilities)?;
            conn.execute(
                "INSERT INTO mcp_connectors (id, name, description, publisher, version, homepage_url, repository_url, config_schema, capabilities_json, verified, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    connector.id.to_string(),
                    connector.name,
                    connector.description,
                    connector.publisher,
                    connector.version,
                    connector.homepage_url,
                    connector.repository_url,
                    config_schema_json,
                    capabilities_json,
                    if connector.verified { 1 } else { 0 },
                    connector.created_at.to_rfc3339(),
                    connector.updated_at.to_rfc3339(),
                ],
            )
            .map_err(|e| MvError::Storage(format!("insert mcp connector failed: {e}")))?;
            Ok(())
        })
    }

    async fn get_mcp_connector(&self, id: Uuid) -> MvResult<Option<McpConnector>> {
        self.with_conn(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT id, name, description, publisher, version, homepage_url, repository_url, config_schema, capabilities_json, verified, created_at, updated_at
                     FROM mcp_connectors WHERE id = ?1",
                )
                .map_err(|e| MvError::Storage(e.to_string()))?;
            let connector = stmt
                .query_row(params![id.to_string()], row_to_mcp_connector)
                .optional()
                .map_err(|e| MvError::Storage(format!("select mcp connector failed: {e}")))?;
            Ok(connector)
        })
    }

    async fn list_mcp_connectors(
        &self,
        publisher: Option<&str>,
        verified: Option<bool>,
        limit: usize,
        offset: usize,
    ) -> MvResult<Vec<McpConnector>> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let mut sql = String::from(
            "SELECT id, name, description, publisher, version, homepage_url, repository_url, config_schema, capabilities_json, verified, created_at, updated_at
             FROM mcp_connectors WHERE 1=1",
        );
        let mut params_box: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(publisher) = publisher {
            sql.push_str(&format!(" AND publisher = ?{}", params_box.len() + 1));
            params_box.push(Box::new(publisher.to_string()));
        }
        if let Some(verified) = verified {
            sql.push_str(&format!(" AND verified = ?{}", params_box.len() + 1));
            params_box.push(Box::new(if verified { 1 } else { 0 }));
        }

        sql.push_str(&format!(
            " ORDER BY name ASC LIMIT ?{} OFFSET ?{}",
            params_box.len() + 1,
            params_box.len() + 2
        ));
        params_box.push(Box::new(limit as i64));
        params_box.push(Box::new(offset as i64));

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            params_box.iter().map(|p| p.as_ref()).collect();
        let rows = stmt
            .query_map(params_refs.as_slice(), row_to_mcp_connector)
            .map_err(|e| MvError::Storage(format!("list mcp connectors failed: {e}")))?;
        let mut connectors = Vec::new();
        for row in rows {
            connectors.push(row.map_err(|e| MvError::Storage(e.to_string()))?);
        }
        Ok(connectors)
    }

    async fn update_mcp_connector(&self, connector: &McpConnector) -> MvResult<bool> {
        self.with_conn(|conn| {
            let config_schema_json = serde_json::to_string(&connector.config_schema)?;
            let capabilities_json = serde_json::to_string(&connector.capabilities)?;
            let updated = conn
                .execute(
                    "UPDATE mcp_connectors
                     SET name = ?2, description = ?3, publisher = ?4, version = ?5, homepage_url = ?6,
                         repository_url = ?7, config_schema = ?8, capabilities_json = ?9, verified = ?10, updated_at = ?11
                     WHERE id = ?1",
                    params![
                        connector.id.to_string(),
                        connector.name,
                        connector.description,
                        connector.publisher,
                        connector.version,
                        connector.homepage_url,
                        connector.repository_url,
                        config_schema_json,
                        capabilities_json,
                        if connector.verified { 1 } else { 0 },
                        connector.updated_at.to_rfc3339(),
                    ],
                )
                .map_err(|e| MvError::Storage(format!("update mcp connector failed: {e}")))?;
            Ok(updated > 0)
        })
    }

    async fn delete_mcp_connector(&self, id: Uuid) -> MvResult<bool> {
        self.with_conn(|conn| {
            let affected = conn
                .execute(
                    "DELETE FROM mcp_connectors WHERE id = ?1",
                    params![id.to_string()],
                )
                .map_err(|e| MvError::Storage(format!("delete mcp connector failed: {e}")))?;
            Ok(affected > 0)
        })
    }
}

fn row_to_policy(row: &rusqlite::Row<'_>) -> rusqlite::Result<AccessPolicy> {
    let id_str: String = row.get(0)?;
    let secret_key: String = row.get(1)?;
    let consumer: String = row.get(2)?;
    let allowed: i32 = row.get(3)?;
    let scopes_json: String = row.get(4)?;
    let max_ttl_seconds: Option<i64> = row.get(5)?;
    let expires_at_str: Option<String> = row.get(6)?;
    let require_approval: i32 = row.get(7)?;
    let created_at_str: String = row.get(8)?;
    let updated_at_str: String = row.get(9)?;

    let id = parse_uuid_str(0, &id_str)?;
    let scopes: Vec<String> = serde_json::from_str(&scopes_json).unwrap_or_default();

    Ok(AccessPolicy {
        id,
        secret_key,
        consumer,
        allowed: allowed != 0,
        scopes,
        max_ttl_seconds,
        expires_at: parse_optional_dt_strict(6, expires_at_str)?,
        require_approval: require_approval != 0,
        created_at: parse_dt_strict(8, &created_at_str)?,
        updated_at: parse_dt_strict(9, &updated_at_str)?,
    })
}

// ---------------------------------------------------------------------------
// ProxyAuditStore – Proxy Audit Log
// ---------------------------------------------------------------------------

use mv_core::{ProxyAuditEntry, ProxyAuditStore};

#[async_trait]
impl ProxyAuditStore for SqliteNodeStore {
    async fn log_proxy_audit(&self, entry: &ProxyAuditEntry) -> MvResult<()> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        conn.execute(
            "INSERT INTO proxy_audit_log (id, consumer, secret_ref, action, target, intent, timestamp, success, sanitized, error, request_summary, response_status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                entry.id.to_string(),
                entry.consumer,
                entry.secret_ref,
                entry.action,
                entry.target,
                entry.intent,
                entry.timestamp.to_rfc3339(),
                entry.success.map(|v| v as i32),
                entry.sanitized as i32,
                entry.error,
                entry.request_summary,
                entry.response_status,
            ],
        )
        .map_err(|e| MvError::Storage(format!("insert proxy_audit_log failed: {e}")))?;
        Ok(())
    }

    async fn update_proxy_audit(
        &self,
        id: Uuid,
        success: bool,
        sanitized: bool,
        error: Option<&str>,
        response_status: Option<i32>,
    ) -> MvResult<()> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        conn.execute(
            "UPDATE proxy_audit_log SET success = ?2, sanitized = ?3, error = ?4, response_status = ?5 WHERE id = ?1",
            params![
                id.to_string(),
                success as i32,
                sanitized as i32,
                error,
                response_status,
            ],
        )
        .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(())
    }

    async fn list_proxy_audit(
        &self,
        consumer: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> MvResult<Vec<ProxyAuditEntry>> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let (sql, params_box): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = if let Some(c) =
            consumer
        {
            (
                "SELECT id, consumer, secret_ref, action, target, intent, timestamp, success, sanitized, error, request_summary, response_status
                 FROM proxy_audit_log WHERE consumer = ?1 ORDER BY timestamp DESC LIMIT ?2 OFFSET ?3".to_string(),
                vec![Box::new(c.to_string()), Box::new(limit as i64), Box::new(offset as i64)],
            )
        } else {
            (
                "SELECT id, consumer, secret_ref, action, target, intent, timestamp, success, sanitized, error, request_summary, response_status
                 FROM proxy_audit_log ORDER BY timestamp DESC LIMIT ?1 OFFSET ?2".to_string(),
                vec![Box::new(limit as i64), Box::new(offset as i64)],
            )
        };

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            params_box.iter().map(|p| p.as_ref()).collect();
        let rows = stmt
            .query_map(params_refs.as_slice(), row_to_proxy_audit)
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(|e| MvError::Storage(e.to_string()))?);
        }
        Ok(result)
    }
}

fn row_to_proxy_audit(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProxyAuditEntry> {
    let id_str: String = row.get(0)?;
    let consumer: String = row.get(1)?;
    let secret_ref: String = row.get(2)?;
    let action: String = row.get(3)?;
    let target: String = row.get(4)?;
    let intent: String = row.get(5)?;
    let timestamp_str: String = row.get(6)?;
    let success: Option<i32> = row.get(7)?;
    let sanitized: i32 = row.get(8)?;
    let error: Option<String> = row.get(9)?;
    let request_summary: String = row.get(10)?;
    let response_status: Option<i32> = row.get(11)?;

    let id = parse_uuid_str(0, &id_str)?;

    Ok(ProxyAuditEntry {
        id,
        consumer,
        secret_ref,
        action,
        target,
        intent,
        timestamp: parse_dt_strict(6, &timestamp_str)?,
        success: success.map(|v| v != 0),
        sanitized: sanitized != 0,
        error,
        request_summary,
        response_status,
    })
}

// ---------------------------------------------------------------------------
// ApprovalStore
// ---------------------------------------------------------------------------

#[async_trait::async_trait]
impl ApprovalStore for SqliteNodeStore {
    async fn create_approval(&self, request: &ApprovalRequest) -> MvResult<()> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let scopes_json = serde_json::to_string(&request.scopes)
            .map_err(|e| MvError::Storage(format!("serialize scopes: {e}")))?;
        conn.execute(
            "INSERT INTO proxy_approvals (id, consumer, secret_key, intent, request_summary, state, created_at, expires_at, scopes)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                request.id.to_string(),
                request.consumer,
                request.secret_key,
                request.intent,
                request.request_summary,
                request.state.as_str(),
                request.created_at.to_rfc3339(),
                request.expires_at.to_rfc3339(),
                scopes_json,
            ],
        )
        .map_err(|e| MvError::Storage(format!("insert proxy_approvals failed: {e}")))?;
        Ok(())
    }

    async fn get_approval(&self, id: Uuid) -> MvResult<Option<ApprovalRequest>> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let mut stmt = conn.prepare(
            "SELECT id, consumer, secret_key, intent, request_summary, state, created_at, expires_at, decided_at, decided_by, deny_reason, scopes
             FROM proxy_approvals WHERE id = ?1"
        ).map_err(|e| MvError::Storage(e.to_string()))?;

        let mut rows = stmt
            .query_map(params![id.to_string()], row_to_approval)
            .map_err(|e| MvError::Storage(e.to_string()))?;

        match rows.next() {
            Some(Ok(a)) => Ok(Some(a)),
            Some(Err(e)) => Err(MvError::Storage(e.to_string())),
            None => Ok(None),
        }
    }

    async fn list_pending_approvals(
        &self,
        consumer: Option<&str>,
    ) -> MvResult<Vec<ApprovalRequest>> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let (sql, params_box): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = if let Some(c) =
            consumer
        {
            (
                "SELECT id, consumer, secret_key, intent, request_summary, state, created_at, expires_at, decided_at, decided_by, deny_reason, scopes
                 FROM proxy_approvals WHERE state = 'pending' AND consumer = ?1 ORDER BY created_at DESC".to_string(),
                vec![Box::new(c.to_string())],
            )
        } else {
            (
                "SELECT id, consumer, secret_key, intent, request_summary, state, created_at, expires_at, decided_at, decided_by, deny_reason, scopes
                 FROM proxy_approvals WHERE state = 'pending' ORDER BY created_at DESC".to_string(),
                vec![],
            )
        };

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            params_box.iter().map(|p| p.as_ref()).collect();
        let rows = stmt
            .query_map(params_refs.as_slice(), row_to_approval)
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(|e| MvError::Storage(e.to_string()))?);
        }
        Ok(result)
    }

    async fn decide_approval(
        &self,
        id: Uuid,
        approved: bool,
        decided_by: Option<&str>,
        deny_reason: Option<&str>,
    ) -> MvResult<bool> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let new_state = if approved { "approved" } else { "denied" };
        let now = chrono::Utc::now().to_rfc3339();
        let affected = conn.execute(
            "UPDATE proxy_approvals SET state = ?2, decided_at = ?3, decided_by = ?4, deny_reason = ?5
             WHERE id = ?1 AND state = 'pending'",
            params![
                id.to_string(),
                new_state,
                now,
                decided_by,
                deny_reason,
            ],
        )
        .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(affected > 0)
    }

    async fn expire_approvals(&self) -> MvResult<usize> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let now = chrono::Utc::now().to_rfc3339();
        let affected = conn.execute(
            "UPDATE proxy_approvals SET state = 'expired' WHERE state = 'pending' AND expires_at <= ?1",
            params![now],
        )
        .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(affected)
    }

    async fn find_active_approval(
        &self,
        consumer: &str,
        secret_key: &str,
    ) -> MvResult<Option<ApprovalRequest>> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let now = chrono::Utc::now().to_rfc3339();
        let mut stmt = conn.prepare(
            "SELECT id, consumer, secret_key, intent, request_summary, state, created_at, expires_at, decided_at, decided_by, deny_reason, scopes
             FROM proxy_approvals
             WHERE consumer = ?1 AND secret_key = ?2 AND state = 'approved' AND expires_at > ?3
             ORDER BY decided_at DESC LIMIT 1"
        ).map_err(|e| MvError::Storage(e.to_string()))?;

        let mut rows = stmt
            .query_map(params![consumer, secret_key, now], row_to_approval)
            .map_err(|e| MvError::Storage(e.to_string()))?;

        match rows.next() {
            Some(Ok(a)) => Ok(Some(a)),
            Some(Err(e)) => Err(MvError::Storage(e.to_string())),
            None => Ok(None),
        }
    }
}

fn row_to_approval(row: &rusqlite::Row<'_>) -> rusqlite::Result<ApprovalRequest> {
    let id_str: String = row.get(0)?;
    let consumer: String = row.get(1)?;
    let secret_key: String = row.get(2)?;
    let intent: String = row.get(3)?;
    let request_summary: String = row.get(4)?;
    let state_str: String = row.get(5)?;
    let created_at_str: String = row.get(6)?;
    let expires_at_str: String = row.get(7)?;
    let decided_at_str: Option<String> = row.get(8)?;
    let decided_by: Option<String> = row.get(9)?;
    let deny_reason: Option<String> = row.get(10)?;
    let scopes_json: String = row.get(11)?;

    let id = parse_uuid_str(0, &id_str)?;
    let state: ApprovalState = state_str.parse().map_err(|e: String| {
        rusqlite::Error::FromSqlConversionFailure(
            5,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
        )
    })?;
    let scopes: Vec<String> = serde_json::from_str(&scopes_json).unwrap_or_default();

    Ok(ApprovalRequest {
        id,
        consumer,
        secret_key,
        intent,
        request_summary,
        state,
        created_at: parse_dt_strict(6, &created_at_str)?,
        expires_at: parse_dt_strict(7, &expires_at_str)?,
        decided_at: decided_at_str.as_deref().and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(s)
                .ok()
                .map(|d| d.with_timezone(&chrono::Utc))
        }),
        decided_by,
        deny_reason,
        scopes,
    })
}

// ---------------------------------------------------------------------------
// ConflictStore
// ---------------------------------------------------------------------------

#[async_trait]
impl ConflictStore for SqliteNodeStore {
    async fn insert_conflict(&self, alert: &ConflictAlert) -> MvResult<()> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        conn.execute(
            "INSERT OR IGNORE INTO conflicts (id, node_a, node_b, conflict_type, score, explanation, resolved, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                alert.id.to_string(),
                alert.node_a.to_string(),
                alert.node_b.to_string(),
                alert.conflict_type.as_str(),
                alert.score,
                alert.explanation,
                alert.resolved as i32,
                alert.created_at.to_rfc3339(),
            ],
        )
        .map_err(|e| MvError::Storage(format!("insert conflict: {e}")))?;
        Ok(())
    }

    async fn get_conflict(&self, id: Uuid) -> MvResult<Option<ConflictAlert>> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let result = conn
            .query_row(
                "SELECT id, node_a, node_b, conflict_type, score, explanation, resolved, created_at
                 FROM conflicts WHERE id = ?1",
                params![id.to_string()],
                row_to_conflict,
            )
            .optional()
            .map_err(|e| MvError::Storage(format!("get conflict: {e}")))?;
        Ok(result)
    }

    async fn list_conflicts(
        &self,
        resolved: Option<bool>,
        limit: usize,
        offset: usize,
    ) -> MvResult<Vec<ConflictAlert>> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let limit_i = limit as i64;
        let offset_i = offset as i64;
        let results = match resolved {
            Some(r) => {
                let resolved_i = r as i32;
                let mut stmt = conn
                    .prepare(
                        "SELECT id, node_a, node_b, conflict_type, score, explanation, resolved, created_at
                         FROM conflicts WHERE resolved = ?1 ORDER BY created_at DESC LIMIT ?2 OFFSET ?3",
                    )
                    .map_err(|e| MvError::Storage(format!("list conflicts prepare: {e}")))?;
                let rows = stmt
                    .query_map(params![resolved_i, limit_i, offset_i], row_to_conflict)
                    .map_err(|e| MvError::Storage(format!("list conflicts query: {e}")))?;
                let mut v = Vec::new();
                for row in rows {
                    v.push(row.map_err(|e| MvError::Storage(format!("list conflicts row: {e}")))?);
                }
                v
            }
            None => {
                let mut stmt = conn
                    .prepare(
                        "SELECT id, node_a, node_b, conflict_type, score, explanation, resolved, created_at
                         FROM conflicts ORDER BY created_at DESC LIMIT ?1 OFFSET ?2",
                    )
                    .map_err(|e| MvError::Storage(format!("list conflicts prepare: {e}")))?;
                let rows = stmt
                    .query_map(params![limit_i, offset_i], row_to_conflict)
                    .map_err(|e| MvError::Storage(format!("list conflicts query: {e}")))?;
                let mut v = Vec::new();
                for row in rows {
                    v.push(row.map_err(|e| MvError::Storage(format!("list conflicts row: {e}")))?);
                }
                v
            }
        };
        Ok(results)
    }

    async fn resolve_conflict(&self, id: Uuid) -> MvResult<bool> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let updated = conn
            .execute(
                "UPDATE conflicts SET resolved = 1 WHERE id = ?1 AND resolved = 0",
                params![id.to_string()],
            )
            .map_err(|e| MvError::Storage(format!("resolve conflict: {e}")))?;
        Ok(updated > 0)
    }
}

fn row_to_conflict(row: &rusqlite::Row<'_>) -> rusqlite::Result<ConflictAlert> {
    let id_str: String = row.get(0)?;
    let node_a_str: String = row.get(1)?;
    let node_b_str: String = row.get(2)?;
    let conflict_type_str: String = row.get(3)?;
    let score: f64 = row.get(4)?;
    let explanation: String = row.get(5)?;
    let resolved_int: i32 = row.get(6)?;
    let created_at_str: String = row.get(7)?;

    let id = parse_uuid_str(0, &id_str)?;
    let node_a = parse_uuid_str(1, &node_a_str)?;
    let node_b = parse_uuid_str(2, &node_b_str)?;
    let conflict_type: ConflictType = conflict_type_str.parse().map_err(|e: String| {
        rusqlite::Error::FromSqlConversionFailure(
            3,
            Type::Text,
            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
        )
    })?;

    Ok(ConflictAlert {
        id,
        node_a,
        node_b,
        conflict_type,
        score,
        explanation,
        resolved: resolved_int != 0,
        created_at: parse_dt_strict(7, &created_at_str)?,
    })
}

// ---------------------------------------------------------------------------
// ContactIdentityStore
// ---------------------------------------------------------------------------

#[async_trait]
impl ContactIdentityStore for SqliteNodeStore {
    async fn add_contact_identity(&self, identity: &ContactIdentity) -> MvResult<()> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        conn.execute(
            "INSERT INTO contact_identities (id, contact_id, identity_type, identity_value, verified, verified_at, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                identity.id.to_string(),
                identity.contact_id.to_string(),
                identity.identity_type.as_str(),
                identity.identity_value,
                identity.verified as i32,
                identity.verified_at.map(|dt| dt.to_rfc3339()),
                identity.created_at.to_rfc3339(),
            ],
        )
        .map_err(|e| MvError::Storage(format!("insert contact identity: {e}")))?;
        Ok(())
    }

    async fn list_contact_identities(&self, contact_id: Uuid) -> MvResult<Vec<ContactIdentity>> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, contact_id, identity_type, identity_value, verified, verified_at, created_at
                 FROM contact_identities WHERE contact_id = ?1 ORDER BY created_at DESC",
            )
            .map_err(|e| MvError::Storage(format!("list identities prepare: {e}")))?;
        let rows = stmt
            .query_map(params![contact_id.to_string()], row_to_contact_identity)
            .map_err(|e| MvError::Storage(format!("list identities query: {e}")))?;
        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| MvError::Storage(format!("list identities row: {e}")))?);
        }
        Ok(results)
    }

    async fn delete_contact_identity(&self, id: Uuid) -> MvResult<bool> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let deleted = conn
            .execute(
                "DELETE FROM contact_identities WHERE id = ?1",
                params![id.to_string()],
            )
            .map_err(|e| MvError::Storage(format!("delete identity: {e}")))?;
        Ok(deleted > 0)
    }

    async fn verify_contact_identity(&self, id: Uuid) -> MvResult<bool> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let updated = conn
            .execute(
                "UPDATE contact_identities SET verified = 1, verified_at = strftime('%Y-%m-%dT%H:%M:%SZ','now')
                 WHERE id = ?1 AND verified = 0",
                params![id.to_string()],
            )
            .map_err(|e| MvError::Storage(format!("verify identity: {e}")))?;
        Ok(updated > 0)
    }

    async fn get_trust_model(&self, contact_id: Uuid) -> MvResult<Option<TrustModel>> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let result = conn
            .query_row(
                "SELECT contact_id, can_query, can_inject_context, can_auto_reply, allowed_namespaces, max_confidence_override, updated_at
                 FROM trust_models WHERE contact_id = ?1",
                params![contact_id.to_string()],
                row_to_trust_model,
            )
            .optional()
            .map_err(|e| MvError::Storage(format!("get trust model: {e}")))?;
        Ok(result)
    }

    async fn set_trust_model(&self, model: &TrustModel) -> MvResult<()> {
        let conn = self
            .conn()
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let ns_json = serde_json::to_string(&model.allowed_namespaces)
            .map_err(|e| MvError::Storage(format!("serialize namespaces: {e}")))?;
        conn.execute(
            "INSERT INTO trust_models (contact_id, can_query, can_inject_context, can_auto_reply, allowed_namespaces, max_confidence_override, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, strftime('%Y-%m-%dT%H:%M:%SZ','now'))
             ON CONFLICT(contact_id) DO UPDATE SET
               can_query = excluded.can_query,
               can_inject_context = excluded.can_inject_context,
               can_auto_reply = excluded.can_auto_reply,
               allowed_namespaces = excluded.allowed_namespaces,
               max_confidence_override = excluded.max_confidence_override,
               updated_at = excluded.updated_at",
            params![
                model.contact_id.to_string(),
                model.can_query as i32,
                model.can_inject_context as i32,
                model.can_auto_reply as i32,
                ns_json,
                model.max_confidence_override,
            ],
        )
        .map_err(|e| MvError::Storage(format!("set trust model: {e}")))?;
        Ok(())
    }
}

fn row_to_contact_identity(row: &rusqlite::Row<'_>) -> rusqlite::Result<ContactIdentity> {
    let id_str: String = row.get(0)?;
    let contact_id_str: String = row.get(1)?;
    let identity_type_str: String = row.get(2)?;
    let identity_value: String = row.get(3)?;
    let verified_int: i32 = row.get(4)?;
    let verified_at_str: Option<String> = row.get(5)?;
    let created_at_str: String = row.get(6)?;

    let id = parse_uuid_str(0, &id_str)?;
    let contact_id = parse_uuid_str(1, &contact_id_str)?;
    let identity_type: IdentityType = identity_type_str.parse().map_err(|e: String| {
        rusqlite::Error::FromSqlConversionFailure(
            2,
            Type::Text,
            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
        )
    })?;

    let verified_at = verified_at_str
        .as_deref()
        .map(|s| parse_dt_strict(5, s))
        .transpose()?;

    Ok(ContactIdentity {
        id,
        contact_id,
        identity_type,
        identity_value,
        verified: verified_int != 0,
        verified_at,
        created_at: parse_dt_strict(6, &created_at_str)?,
    })
}

fn row_to_trust_model(row: &rusqlite::Row<'_>) -> rusqlite::Result<TrustModel> {
    let contact_id_str: String = row.get(0)?;
    let can_query: i32 = row.get(1)?;
    let can_inject_context: i32 = row.get(2)?;
    let can_auto_reply: i32 = row.get(3)?;
    let ns_json: String = row.get(4)?;
    let max_confidence_override: Option<f64> = row.get(5)?;
    let updated_at_str: String = row.get(6)?;

    let contact_id = parse_uuid_str(0, &contact_id_str)?;
    let allowed_namespaces: Vec<String> = serde_json::from_str(&ns_json).unwrap_or_default();

    Ok(TrustModel {
        contact_id,
        can_query: can_query != 0,
        can_inject_context: can_inject_context != 0,
        can_auto_reply: can_auto_reply != 0,
        allowed_namespaces,
        max_confidence_override,
        updated_at: parse_dt_strict(6, &updated_at_str)?,
    })
}

// ---------------------------------------------------------------------------
// AdapterPollStore
// ---------------------------------------------------------------------------

#[async_trait]
impl AdapterPollStore for SqliteNodeStore {
    async fn get_poll_state(&self, adapter_name: &str) -> MvResult<Option<AdapterPollState>> {
        let name = adapter_name.to_string();
        self.with_conn(|conn| {
            let mut stmt = conn
                .prepare("SELECT adapter_name, cursor, last_poll_at, messages_received FROM adapter_poll_state WHERE adapter_name = ?1")
                .map_err(|e| MvError::Storage(e.to_string()))?;
            let result = stmt
                .query_row(params![name], |row| {
                    Ok(AdapterPollState {
                        adapter_name: row.get(0)?,
                        cursor: row.get(1)?,
                        last_poll_at: row.get(2)?,
                        messages_received: row.get::<_, i64>(3)? as u64,
                    })
                })
                .optional()
                .map_err(|e| MvError::Storage(e.to_string()))?;
            Ok(result)
        })
    }

    async fn upsert_poll_state(
        &self,
        adapter_name: &str,
        cursor: &str,
        messages_received: u64,
    ) -> MvResult<()> {
        let name = adapter_name.to_string();
        let cursor = cursor.to_string();
        let now = Utc::now().to_rfc3339();
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO adapter_poll_state (adapter_name, cursor, last_poll_at, messages_received)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(adapter_name) DO UPDATE SET
                     cursor = excluded.cursor,
                     last_poll_at = excluded.last_poll_at,
                     messages_received = adapter_poll_state.messages_received + excluded.messages_received",
                params![name, cursor, now, messages_received as i64],
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;
            Ok(())
        })
    }

    async fn list_poll_states(&self) -> MvResult<Vec<AdapterPollState>> {
        self.with_conn(|conn| {
            let mut stmt = conn
                .prepare("SELECT adapter_name, cursor, last_poll_at, messages_received FROM adapter_poll_state ORDER BY adapter_name")
                .map_err(|e| MvError::Storage(e.to_string()))?;
            let rows = stmt
                .query_map([], |row| {
                    Ok(AdapterPollState {
                        adapter_name: row.get(0)?,
                        cursor: row.get(1)?,
                        last_poll_at: row.get(2)?,
                        messages_received: row.get::<_, i64>(3)? as u64,
                    })
                })
                .map_err(|e| MvError::Storage(e.to_string()))?;
            let mut result = Vec::new();
            for row in rows {
                result.push(row.map_err(|e| MvError::Storage(e.to_string()))?);
            }
            Ok(result)
        })
    }

    async fn delete_poll_state(&self, adapter_name: &str) -> MvResult<bool> {
        let name = adapter_name.to_string();
        self.with_conn(|conn| {
            let affected = conn
                .execute(
                    "DELETE FROM adapter_poll_state WHERE adapter_name = ?1",
                    params![name],
                )
                .map_err(|e| MvError::Storage(e.to_string()))?;
            Ok(affected > 0)
        })
    }
}

// ---------------------------------------------------------------------------
// ConversationStore — Phase 3 WI-3a
// ---------------------------------------------------------------------------

#[async_trait]
impl ConversationStore for SqliteNodeStore {
    async fn create_conversation(&self, id: Uuid, title: Option<&str>) -> MvResult<()> {
        let id_s = id.to_string();
        let title_s = title.map(|t| t.to_string());
        self.with_conn(move |conn| {
            conn.execute(
                "INSERT INTO conversations (id, title) VALUES (?1, ?2)",
                params![id_s, title_s],
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;
            Ok(())
        })
    }

    async fn add_message(
        &self,
        conversation_id: Uuid,
        role: &str,
        content: &str,
        sources_json: Option<&str>,
    ) -> MvResult<Uuid> {
        let msg_id = Uuid::now_v7();
        let conv_s = conversation_id.to_string();
        let msg_s = msg_id.to_string();
        let role_s = role.to_string();
        let content_s = content.to_string();
        let sources_s = sources_json.map(|value| value.to_string());
        let token_count = (content_s.len() / 4) as i64; // rough estimate
        self.with_conn(move |conn| {
            conn.execute(
                "INSERT INTO conversation_turns (id, conversation_id, role, content, token_count, sources_json)                  VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![msg_s, conv_s, role_s, content_s, token_count, sources_s],
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;
            // Update conversation's updated_at
            conn.execute(
                "UPDATE conversations SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')                  WHERE id = ?1",
                params![conv_s],
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;
            Ok(msg_id)
        })
    }

    async fn get_messages(
        &self,
        conversation_id: Uuid,
        limit: usize,
    ) -> MvResult<
        Vec<(
            Uuid,
            String,
            String,
            Option<String>,
            chrono::DateTime<chrono::Utc>,
        )>,
    > {
        let conv_s = conversation_id.to_string();
        self.with_conn(move |conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT id, role, content, sources_json, created_at FROM conversation_turns                      WHERE conversation_id = ?1 ORDER BY created_at ASC LIMIT ?2",
                )
                .map_err(|e| MvError::Storage(e.to_string()))?;
            let rows = stmt
                .query_map(params![conv_s, limit as i64], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, String>(4)?,
                    ))
                })
                .map_err(|e| MvError::Storage(e.to_string()))?;

            let mut messages = Vec::new();
            for row in rows {
                let (id_str, role, content, sources_json, ts_str) =
                    row.map_err(|e| MvError::Storage(e.to_string()))?;
                let id = Uuid::parse_str(&id_str)
                    .map_err(|e| MvError::Storage(format!("invalid uuid: {e}")))?;
                let ts = chrono::DateTime::parse_from_rfc3339(&ts_str)
                    .map_err(|e| MvError::Storage(format!("invalid timestamp: {e}")))?
                    .with_timezone(&chrono::Utc);
                messages.push((id, role, content, sources_json, ts));
            }
            Ok(messages)
        })
    }

    async fn delete_conversation(&self, id: Uuid) -> MvResult<bool> {
        let id_s = id.to_string();
        self.with_conn(move |conn| {
            let affected = conn
                .execute("DELETE FROM conversations WHERE id = ?1", params![id_s])
                .map_err(|e| MvError::Storage(e.to_string()))?;
            Ok(affected > 0)
        })
    }

    async fn list_conversations(
        &self,
        limit: usize,
        offset: usize,
    ) -> MvResult<Vec<(Uuid, Option<String>, chrono::DateTime<chrono::Utc>)>> {
        self.with_conn(move |conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT id, title, updated_at FROM conversations \
                     ORDER BY updated_at DESC LIMIT ?1 OFFSET ?2",
                )
                .map_err(|e| MvError::Storage(e.to_string()))?;
            let rows = stmt
                .query_map(params![limit as i64, offset as i64], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                })
                .map_err(|e| MvError::Storage(e.to_string()))?;

            let mut convs = Vec::new();
            for row in rows {
                let (id_str, title, ts_str) = row.map_err(|e| MvError::Storage(e.to_string()))?;
                let id = Uuid::parse_str(&id_str)
                    .map_err(|e| MvError::Storage(format!("invalid uuid: {e}")))?;
                let ts = chrono::DateTime::parse_from_rfc3339(&ts_str)
                    .map_err(|e| MvError::Storage(format!("invalid timestamp: {e}")))?
                    .with_timezone(&chrono::Utc);
                convs.push((id, title, ts));
            }
            Ok(convs)
        })
    }

    async fn expire_conversations(&self, max_age_secs: u64) -> MvResult<usize> {
        self.with_conn(move |conn| {
            let affected = conn
                .execute(
                    "DELETE FROM conversations WHERE updated_at < \
                     strftime('%Y-%m-%dT%H:%M:%fZ', 'now', ?1)",
                    params![format!("-{max_age_secs} seconds")],
                )
                .map_err(|e| MvError::Storage(e.to_string()))?;
            Ok(affected)
        })
    }
}

// ---------------------------------------------------------------------------
// KnowledgeWorkspaceManifestStore
// ---------------------------------------------------------------------------

const WORKSPACE_SELECT: &str = "SELECT id, namespace, mode, state, descriptor_payload, \
    payload_format, revision, created_at, updated_at, last_reconciled_at FROM workspaces";
const WORKSPACE_DOCUMENT_SELECT: &str = "SELECT id, workspace_id, path_token, document_payload, \
    payload_format, lifecycle_state, projection_state, projected_node_id, revision, created_at, \
    updated_at FROM workspace_documents";

fn checked_workspace_revision(revision: u64) -> MvResult<i64> {
    if revision == 0 {
        return Err(MvError::InvalidInput(
            "workspace manifest revision must be greater than zero".into(),
        ));
    }
    i64::try_from(revision)
        .map_err(|_| MvError::InvalidInput("workspace manifest revision is too large".into()))
}

fn validate_revision_advance(revision: u64, expected_revision: u64) -> MvResult<()> {
    let next_revision = expected_revision
        .checked_add(1)
        .ok_or_else(|| MvError::InvalidInput("workspace manifest revision overflow".into()))?;
    if revision != next_revision {
        return Err(MvError::InvalidInput(format!(
            "replacement revision must be {next_revision}, got {revision}"
        )));
    }
    checked_workspace_revision(revision)?;
    Ok(())
}

fn validate_manifest_payload(
    sealed_mode: bool,
    payload: &[u8],
    payload_format: WorkspaceManifestPayloadFormat,
) -> MvResult<()> {
    if payload.is_empty() {
        return Err(MvError::InvalidInput(
            "workspace manifest payload must not be empty".into(),
        ));
    }
    if sealed_mode && payload_format != WorkspaceManifestPayloadFormat::MvencV1 {
        return Err(MvError::InvalidInput(
            "sealed storage requires mvenc-v1 workspace manifest payloads".into(),
        ));
    }
    Ok(())
}

fn validate_workspace_record(sealed_mode: bool, workspace: &KnowledgeWorkspace) -> MvResult<i64> {
    if workspace.namespace.trim().is_empty() {
        return Err(MvError::InvalidInput(
            "knowledge workspace namespace must not be empty".into(),
        ));
    }
    validate_manifest_payload(
        sealed_mode,
        &workspace.descriptor_payload,
        workspace.payload_format,
    )?;
    checked_workspace_revision(workspace.revision)
}

fn validate_workspace_document_record(
    sealed_mode: bool,
    document: &KnowledgeWorkspaceDocument,
) -> MvResult<i64> {
    if document.path_token.len() != 64
        || !document
            .path_token
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(MvError::InvalidInput(
            "workspace document path token must be 64 lowercase hexadecimal characters".into(),
        ));
    }
    validate_manifest_payload(
        sealed_mode,
        &document.document_payload,
        document.payload_format,
    )?;
    checked_workspace_revision(document.revision)
}

#[async_trait]
impl KnowledgeWorkspaceManifestStore for SqliteNodeStore {
    async fn insert_knowledge_workspace(&self, workspace: &KnowledgeWorkspace) -> MvResult<()> {
        let revision = validate_workspace_record(self.sealed_mode(), workspace)?;
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO workspaces (
                    id, namespace, mode, state, descriptor_payload, payload_format,
                    revision, created_at, updated_at, last_reconciled_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    workspace.id.to_string(),
                    workspace.namespace,
                    workspace.mode.as_str(),
                    workspace.state.as_str(),
                    workspace.descriptor_payload,
                    workspace.payload_format.as_str(),
                    revision,
                    workspace.created_at.to_rfc3339(),
                    workspace.updated_at.to_rfc3339(),
                    workspace.last_reconciled_at.map(|value| value.to_rfc3339()),
                ],
            )
            .map_err(|err| MvError::Storage(format!("insert knowledge workspace failed: {err}")))?;
            Ok(())
        })
    }

    async fn get_knowledge_workspace(&self, id: Uuid) -> MvResult<Option<KnowledgeWorkspace>> {
        self.with_conn(|conn| {
            conn.query_row(
                &format!("{WORKSPACE_SELECT} WHERE id = ?1"),
                params![id.to_string()],
                row_to_knowledge_workspace,
            )
            .optional()
            .map_err(|err| MvError::Storage(format!("get knowledge workspace failed: {err}")))
        })
    }

    async fn list_knowledge_workspaces(
        &self,
        namespace: Option<&str>,
    ) -> MvResult<Vec<KnowledgeWorkspace>> {
        self.with_conn(|conn| {
            let sql = match namespace {
                Some(_) => {
                    format!("{WORKSPACE_SELECT} WHERE namespace = ?1 ORDER BY created_at, id")
                }
                None => format!("{WORKSPACE_SELECT} ORDER BY created_at, id"),
            };
            let mut statement = conn
                .prepare(&sql)
                .map_err(|err| MvError::Storage(format!("prepare workspace list failed: {err}")))?;
            let rows = match namespace {
                Some(value) => statement.query_map(params![value], row_to_knowledge_workspace),
                None => statement.query_map([], row_to_knowledge_workspace),
            }
            .map_err(|err| MvError::Storage(format!("list knowledge workspaces failed: {err}")))?;
            rows.collect::<Result<Vec<_>, _>>().map_err(|err| {
                MvError::Storage(format!("collect knowledge workspaces failed: {err}"))
            })
        })
    }

    async fn update_knowledge_workspace(
        &self,
        workspace: &KnowledgeWorkspace,
        expected_revision: u64,
    ) -> MvResult<bool> {
        validate_revision_advance(workspace.revision, expected_revision)?;
        let revision = validate_workspace_record(self.sealed_mode(), workspace)?;
        let expected_revision = checked_workspace_revision(expected_revision)?;
        self.with_conn(|conn| {
            let updated = conn
                .execute(
                    "UPDATE workspaces SET
                        namespace = ?2, mode = ?3, state = ?4, descriptor_payload = ?5,
                        payload_format = ?6, revision = ?7, updated_at = ?8,
                        last_reconciled_at = ?9
                     WHERE id = ?1 AND revision = ?10",
                    params![
                        workspace.id.to_string(),
                        workspace.namespace,
                        workspace.mode.as_str(),
                        workspace.state.as_str(),
                        workspace.descriptor_payload,
                        workspace.payload_format.as_str(),
                        revision,
                        workspace.updated_at.to_rfc3339(),
                        workspace.last_reconciled_at.map(|value| value.to_rfc3339()),
                        expected_revision,
                    ],
                )
                .map_err(|err| {
                    MvError::Storage(format!("update knowledge workspace failed: {err}"))
                })?;
            Ok(updated > 0)
        })
    }

    async fn insert_workspace_document(
        &self,
        document: &KnowledgeWorkspaceDocument,
    ) -> MvResult<()> {
        let revision = validate_workspace_document_record(self.sealed_mode(), document)?;
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO workspace_documents (
                    id, workspace_id, path_token, document_payload, payload_format,
                    lifecycle_state, projection_state, projected_node_id, revision,
                    created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    document.id.to_string(),
                    document.workspace_id.to_string(),
                    document.path_token,
                    document.document_payload,
                    document.payload_format.as_str(),
                    document.lifecycle_state.as_str(),
                    document.projection_state.as_str(),
                    document.projected_node_id.map(|value| value.to_string()),
                    revision,
                    document.created_at.to_rfc3339(),
                    document.updated_at.to_rfc3339(),
                ],
            )
            .map_err(|err| MvError::Storage(format!("insert workspace document failed: {err}")))?;
            Ok(())
        })
    }

    async fn get_workspace_document(
        &self,
        id: Uuid,
    ) -> MvResult<Option<KnowledgeWorkspaceDocument>> {
        self.with_conn(|conn| {
            conn.query_row(
                &format!("{WORKSPACE_DOCUMENT_SELECT} WHERE id = ?1"),
                params![id.to_string()],
                row_to_workspace_document,
            )
            .optional()
            .map_err(|err| MvError::Storage(format!("get workspace document failed: {err}")))
        })
    }

    async fn get_workspace_document_by_path_token(
        &self,
        workspace_id: Uuid,
        path_token: &str,
    ) -> MvResult<Option<KnowledgeWorkspaceDocument>> {
        self.with_conn(|conn| {
            conn.query_row(
                &format!("{WORKSPACE_DOCUMENT_SELECT} WHERE workspace_id = ?1 AND path_token = ?2"),
                params![workspace_id.to_string(), path_token],
                row_to_workspace_document,
            )
            .optional()
            .map_err(|err| {
                MvError::Storage(format!(
                    "get workspace document by path token failed: {err}"
                ))
            })
        })
    }

    async fn list_workspace_documents(
        &self,
        workspace_id: Uuid,
    ) -> MvResult<Vec<KnowledgeWorkspaceDocument>> {
        self.with_conn(|conn| {
            let mut statement = conn
                .prepare(&format!(
                    "{WORKSPACE_DOCUMENT_SELECT} WHERE workspace_id = ?1 ORDER BY path_token, id"
                ))
                .map_err(|err| {
                    MvError::Storage(format!("prepare workspace document list failed: {err}"))
                })?;
            let rows = statement
                .query_map(params![workspace_id.to_string()], row_to_workspace_document)
                .map_err(|err| {
                    MvError::Storage(format!("list workspace documents failed: {err}"))
                })?;
            rows.collect::<Result<Vec<_>, _>>().map_err(|err| {
                MvError::Storage(format!("collect workspace documents failed: {err}"))
            })
        })
    }

    async fn update_workspace_document(
        &self,
        document: &KnowledgeWorkspaceDocument,
        expected_revision: u64,
    ) -> MvResult<bool> {
        validate_revision_advance(document.revision, expected_revision)?;
        let revision = validate_workspace_document_record(self.sealed_mode(), document)?;
        let expected_revision = checked_workspace_revision(expected_revision)?;
        self.with_conn(|conn| {
            let updated = conn
                .execute(
                    "UPDATE workspace_documents SET
                        path_token = ?3, document_payload = ?4,
                        payload_format = ?5, lifecycle_state = ?6, projection_state = ?7,
                        projected_node_id = ?8, revision = ?9, updated_at = ?10
                     WHERE id = ?1 AND workspace_id = ?2 AND revision = ?11",
                    params![
                        document.id.to_string(),
                        document.workspace_id.to_string(),
                        document.path_token,
                        document.document_payload,
                        document.payload_format.as_str(),
                        document.lifecycle_state.as_str(),
                        document.projection_state.as_str(),
                        document.projected_node_id.map(|value| value.to_string()),
                        revision,
                        document.updated_at.to_rfc3339(),
                        expected_revision,
                    ],
                )
                .map_err(|err| {
                    MvError::Storage(format!("update workspace document failed: {err}"))
                })?;
            Ok(updated > 0)
        })
    }

    async fn apply_workspace_reconciliation(
        &self,
        reconciliation: &WorkspaceManifestReconciliation,
    ) -> MvResult<bool> {
        let workspace = &reconciliation.workspace_replacement;
        validate_revision_advance(
            workspace.revision,
            reconciliation.expected_workspace_revision,
        )?;
        let workspace_revision = validate_workspace_record(self.sealed_mode(), workspace)?;
        let expected_workspace_revision =
            checked_workspace_revision(reconciliation.expected_workspace_revision)?;

        let mut document_ids = std::collections::HashSet::new();
        for document in &reconciliation.document_inserts {
            if document.workspace_id != workspace.id {
                return Err(MvError::InvalidInput(
                    "reconciliation document insert belongs to another workspace".into(),
                ));
            }
            if !document_ids.insert(document.id) {
                return Err(MvError::InvalidInput(
                    "reconciliation contains a duplicate document id".into(),
                ));
            }
            validate_workspace_document_record(self.sealed_mode(), document)?;
        }
        for update in &reconciliation.document_updates {
            let document = &update.replacement;
            if document.workspace_id != workspace.id {
                return Err(MvError::InvalidInput(
                    "reconciliation document update belongs to another workspace".into(),
                ));
            }
            if !document_ids.insert(document.id) {
                return Err(MvError::InvalidInput(
                    "reconciliation contains a duplicate document id".into(),
                ));
            }
            validate_revision_advance(document.revision, update.expected_revision)?;
            validate_workspace_document_record(self.sealed_mode(), document)?;
            checked_workspace_revision(update.expected_revision)?;
        }

        self.with_conn(|conn| {
            let transaction = conn.unchecked_transaction().map_err(|err| {
                MvError::Storage(format!("begin workspace reconciliation failed: {err}"))
            })?;

            let workspace_updated = transaction
                .execute(
                    "UPDATE workspaces SET
                        namespace = ?2, mode = ?3, state = ?4, descriptor_payload = ?5,
                        payload_format = ?6, revision = ?7, updated_at = ?8,
                        last_reconciled_at = ?9
                     WHERE id = ?1 AND revision = ?10",
                    params![
                        workspace.id.to_string(),
                        workspace.namespace,
                        workspace.mode.as_str(),
                        workspace.state.as_str(),
                        workspace.descriptor_payload,
                        workspace.payload_format.as_str(),
                        workspace_revision,
                        workspace.updated_at.to_rfc3339(),
                        workspace.last_reconciled_at.map(|value| value.to_rfc3339()),
                        expected_workspace_revision,
                    ],
                )
                .map_err(|err| {
                    MvError::Storage(format!(
                        "update workspace during reconciliation failed: {err}"
                    ))
                })?;
            if workspace_updated == 0 {
                return Ok(false);
            }

            for document in &reconciliation.document_inserts {
                let revision = checked_workspace_revision(document.revision)?;
                transaction
                    .execute(
                        "INSERT INTO workspace_documents (
                            id, workspace_id, path_token, document_payload, payload_format,
                            lifecycle_state, projection_state, projected_node_id, revision,
                            created_at, updated_at
                         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                        params![
                            document.id.to_string(),
                            document.workspace_id.to_string(),
                            document.path_token,
                            document.document_payload,
                            document.payload_format.as_str(),
                            document.lifecycle_state.as_str(),
                            document.projection_state.as_str(),
                            document.projected_node_id.map(|value| value.to_string()),
                            revision,
                            document.created_at.to_rfc3339(),
                            document.updated_at.to_rfc3339(),
                        ],
                    )
                    .map_err(|err| {
                        MvError::Storage(format!(
                            "insert document during reconciliation failed: {err}"
                        ))
                    })?;
            }

            for update in &reconciliation.document_updates {
                let document = &update.replacement;
                let revision = checked_workspace_revision(document.revision)?;
                let expected_revision = checked_workspace_revision(update.expected_revision)?;
                let updated = transaction
                    .execute(
                        "UPDATE workspace_documents SET
                            path_token = ?3, document_payload = ?4,
                            payload_format = ?5, lifecycle_state = ?6, projection_state = ?7,
                            projected_node_id = ?8, revision = ?9, updated_at = ?10
                         WHERE id = ?1 AND workspace_id = ?2 AND revision = ?11",
                        params![
                            document.id.to_string(),
                            document.workspace_id.to_string(),
                            document.path_token,
                            document.document_payload,
                            document.payload_format.as_str(),
                            document.lifecycle_state.as_str(),
                            document.projection_state.as_str(),
                            document.projected_node_id.map(|value| value.to_string()),
                            revision,
                            document.updated_at.to_rfc3339(),
                            expected_revision,
                        ],
                    )
                    .map_err(|err| {
                        MvError::Storage(format!(
                            "update document during reconciliation failed: {err}"
                        ))
                    })?;
                if updated == 0 {
                    return Ok(false);
                }
            }

            transaction.commit().map_err(|err| {
                MvError::Storage(format!("commit workspace reconciliation failed: {err}"))
            })?;
            Ok(true)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sealed_runtime::{
        clear_runtime_root_key_for_scope, runtime_scope_from_parent, set_runtime_root_key_for_scope,
    };
    use tempfile::tempdir;
    use uuid::Uuid;

    struct SealedRuntimeReset {
        scope: String,
    }

    impl Drop for SealedRuntimeReset {
        fn drop(&mut self) {
            clear_runtime_root_key_for_scope(&self.scope);
        }
    }

    fn install_scoped_runtime_key(path: &std::path::Path, key: [u8; 32]) -> SealedRuntimeReset {
        let scope = runtime_scope_from_parent(path);
        set_runtime_root_key_for_scope(&scope, key, false);
        SealedRuntimeReset { scope }
    }

    fn bytes_contains(haystack: &[u8], needle: &[u8]) -> bool {
        if needle.is_empty() || haystack.len() < needle.len() {
            return false;
        }
        haystack
            .windows(needle.len())
            .any(|window| window == needle)
    }

    fn node_created_event(
        local_node_id: Uuid,
        node_id: Uuid,
        key: &str,
        digest: &str,
    ) -> EventEnvelope {
        let subject = StableUri::knowledge_node(local_node_id, node_id);
        let principal = StableUri::principal(
            local_node_id,
            Uuid::new_v5(&local_node_id, b"interoperability-test-principal"),
        );
        EventEnvelope::new(NewEventEnvelope {
            event_type: KNOWLEDGE_NODE_CREATED_V1.into(),
            source: StableUri::node(local_node_id),
            subject: subject.clone(),
            schema: SchemaReference::new(
                StableUri::schema("knowledge-node-created").unwrap(),
                "1.0.0",
            )
            .unwrap(),
            principal: principal.clone(),
            actor: principal,
            correlation_id: Uuid::now_v7(),
            causation_id: None,
            idempotency_key: IdempotencyKey::parse(key).unwrap(),
            payload_digest: digest.into(),
            sensitivity: Sensitivity::Internal,
            retention: RetentionClass::Durable,
            provenance: vec![ProvenanceReference {
                resource: subject,
                relation: ProvenanceRelation::PrimarySource,
            }],
            data: serde_json::json!({
                "resource_kind": "knowledge_node",
                "node_kind": "fact",
                "namespace": "interoperability",
            }),
        })
        .unwrap()
    }

    fn governance_event(
        local_node_id: Uuid,
        event_type: &str,
        schema_name: &str,
        subject: StableUri,
        key: &str,
        data: serde_json::Value,
    ) -> EventEnvelope {
        let principal = StableUri::principal(
            local_node_id,
            Uuid::new_v5(&local_node_id, b"governance-test-principal"),
        );
        EventEnvelope::new(NewEventEnvelope {
            event_type: event_type.into(),
            source: StableUri::node(local_node_id),
            subject: subject.clone(),
            schema: SchemaReference::new(StableUri::schema(schema_name).unwrap(), "1.0.0").unwrap(),
            principal: principal.clone(),
            actor: principal,
            correlation_id: Uuid::now_v7(),
            causation_id: None,
            idempotency_key: IdempotencyKey::parse(key).unwrap(),
            payload_digest: canonical_json_sha256(&data),
            sensitivity: Sensitivity::Internal,
            retention: RetentionClass::Durable,
            provenance: vec![ProvenanceReference {
                resource: subject,
                relation: ProvenanceRelation::PrimarySource,
            }],
            data,
        })
        .unwrap()
    }

    fn context_node_event(
        local_node_id: Uuid,
        event_type: &str,
        schema_name: &str,
        context_node: &ContextNodeRecord,
        key: &str,
        data: serde_json::Value,
    ) -> EventEnvelope {
        let mut event = governance_event(
            local_node_id,
            event_type,
            schema_name,
            context_node.node_uri.clone(),
            key,
            data,
        );
        event.payload_digest = context_node.semantic_digest();
        event
    }

    fn authority_grant_event(
        local_node_id: Uuid,
        event_type: &str,
        schema_name: &str,
        grant: &AuthorityGrant,
        key: &str,
        data: serde_json::Value,
    ) -> EventEnvelope {
        let mut event = governance_event(
            local_node_id,
            event_type,
            schema_name,
            grant.grant_uri.clone(),
            key,
            data,
        );
        event.principal = grant.grantor.clone();
        event.actor = grant.grantor.clone();
        event.payload_digest = grant.semantic_digest();
        event
    }

    fn test_local_context_node(local_node_id: Uuid) -> ContextNodeRecord {
        let manifest = ContextCapabilityManifest::new(
            vec![ContextCapability::Discover],
            Vec::new(),
            Vec::new(),
        )
        .unwrap();
        let mut context_node = ContextNodeRecord::discovered(
            local_node_id,
            ContextNodeType::Personal,
            StableUri::principal(
                local_node_id,
                Uuid::new_v5(&local_node_id, b"local-context-owner"),
            ),
            StableUri::node(local_node_id),
            "Personal Vault",
            manifest,
        )
        .unwrap();
        context_node.trust_class = ContextNodeTrustClass::local();
        context_node.status = ContextNodeStatus::Active;
        context_node
    }

    async fn register_local_context_node(store: &SqliteNodeStore) -> ContextNodeRecord {
        let local_node_id = store.local_context_node_id().await.unwrap();
        let context_node = test_local_context_node(local_node_id);
        let data = serde_json::json!({
            "node_id": context_node.node_id,
            "node_type": context_node.node_type.as_str(),
            "status": context_node.status.as_str(),
            "record_digest": context_node.semantic_digest(),
            "capability_digest": context_node.capability_manifest.content_digest,
        });
        let event = context_node_event(
            local_node_id,
            CONTEXT_NODE_REGISTERED_V1,
            "context-node-registered",
            &context_node,
            "register-local-context-node",
            data,
        );
        store
            .commit_context_node_with_event(&context_node, &event)
            .await
            .unwrap();
        context_node
    }

    fn test_source_binding(
        local_node_id: Uuid,
        external_account_id: &str,
        external_object_id: &str,
    ) -> SourceBinding {
        let resource_id = Uuid::now_v7();
        SourceBinding::new(
            StableUri::knowledge_node(local_node_id, resource_id),
            StableUri::node(local_node_id),
            "calendar",
            external_account_id,
            external_object_id,
            StableUri::parse("mindvault://sources/calendar").unwrap(),
            StableUri::knowledge_node(local_node_id, resource_id),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn test_insert_and_get() {
        let store = SqliteNodeStore::open_in_memory().unwrap();
        let node = KnowledgeNode::new(NodeKind::Fact, "Rust is fast")
            .with_title("Rust Speed")
            .with_tags(vec!["rust".into(), "performance".into()]);

        let id = node.id;
        store.insert(&node).await.unwrap();

        let retrieved = store.get(id).await.unwrap().unwrap();
        assert_eq!(retrieved.content, "Rust is fast");
        assert_eq!(retrieved.tags, vec!["performance", "rust"]); // sorted
        assert_eq!(retrieved.kind, NodeKind::Fact);
    }

    #[tokio::test]
    async fn test_sealed_node_payload_persists_encrypted_columns() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("sealed_payload.sqlite");
        let _reset = install_scoped_runtime_key(&db_path, [7u8; 32]);

        let store = SqliteNodeStore::open_with_mode(&db_path, true).unwrap();
        let mut node = KnowledgeNode::new(NodeKind::Fact, "sealed-content")
            .with_title("sealed-title")
            .with_namespace("default");
        node.source = Some("sealed-source".to_string());
        node.metadata.insert("k".into(), serde_json::json!("v"));
        let id = node.id;

        store.insert(&node).await.unwrap();

        let raw = store
            .with_conn(|conn| {
                conn.query_row(
                    "SELECT title, content, metadata_json, payload_ciphertext, payload_wrapped_dek FROM knowledge_nodes WHERE id = ?1",
                    params![id.to_string()],
                    |row| {
                        Ok((
                            row.get::<_, Option<String>>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, Option<String>>(2)?,
                            row.get::<_, Option<String>>(3)?,
                            row.get::<_, Option<String>>(4)?,
                        ))
                    },
                )
                .map_err(|e| MvError::Storage(e.to_string()))
            })
            .unwrap();

        assert_eq!(raw.0, None);
        assert_eq!(raw.1, "");
        assert_eq!(raw.2, None);
        assert!(raw.3.is_some());
        assert!(raw.4.is_some());

        let roundtrip = store.get(id).await.unwrap().unwrap();
        assert_eq!(roundtrip.title.as_deref(), Some("sealed-title"));
        assert_eq!(roundtrip.content, "sealed-content");
        assert_eq!(roundtrip.source.as_deref(), Some("sealed-source"));
        assert_eq!(
            roundtrip
                .metadata
                .get("k")
                .and_then(serde_json::Value::as_str),
            Some("v")
        );
    }

    #[tokio::test]
    async fn test_sealed_node_update_refreshes_encrypted_payload() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("sealed_update.sqlite");
        let _reset = install_scoped_runtime_key(&db_path, [9u8; 32]);

        let store = SqliteNodeStore::open_with_mode(&db_path, true).unwrap();
        let mut node = KnowledgeNode::new(NodeKind::Fact, "v1-content")
            .with_title("v1-title")
            .with_namespace("default");
        node.source = Some("v1-source".to_string());
        node.metadata.insert("ver".into(), serde_json::json!("v1"));
        let id = node.id;
        store.insert(&node).await.unwrap();

        node.content = "v2-content".to_string();
        node.title = Some("v2-title".to_string());
        node.source = Some("v2-source".to_string());
        node.temporal.version += 1;
        node.temporal.updated_at = Utc::now();
        node.metadata.insert("ver".into(), serde_json::json!("v2"));
        store.update(&node).await.unwrap();

        let raw = store
            .with_conn(|conn| {
                conn.query_row(
                    "SELECT title, content, metadata_json, payload_ciphertext, payload_wrapped_dek FROM knowledge_nodes WHERE id = ?1",
                    params![id.to_string()],
                    |row| {
                        Ok((
                            row.get::<_, Option<String>>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, Option<String>>(2)?,
                            row.get::<_, Option<String>>(3)?,
                            row.get::<_, Option<String>>(4)?,
                        ))
                    },
                )
                .map_err(|e| MvError::Storage(e.to_string()))
            })
            .unwrap();
        assert_eq!(raw.0, None);
        assert_eq!(raw.1, "");
        assert_eq!(raw.2, None);
        assert!(raw.3.is_some());
        assert!(raw.4.is_some());

        let roundtrip = store.get(id).await.unwrap().unwrap();
        assert_eq!(roundtrip.title.as_deref(), Some("v2-title"));
        assert_eq!(roundtrip.content, "v2-content");
        assert_eq!(roundtrip.source.as_deref(), Some("v2-source"));
        assert_eq!(
            roundtrip
                .metadata
                .get("ver")
                .and_then(serde_json::Value::as_str),
            Some("v2")
        );
    }

    #[tokio::test]
    async fn test_sealed_sqlite_file_does_not_contain_plaintext_marker() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("mindvault.sqlite");
        let _reset = install_scoped_runtime_key(&db_path, [11u8; 32]);
        let store = SqliteNodeStore::open_with_mode(&db_path, true).expect("open sqlite store");
        let marker = format!("sealed-sqlite-marker-{}", Uuid::now_v7());

        let mut node = KnowledgeNode::new(NodeKind::Fact, marker.clone())
            .with_title(marker.clone())
            .with_namespace("default");
        node.source = Some(marker.clone());
        node.metadata
            .insert("marker".into(), serde_json::Value::String(marker.clone()));

        store.insert(&node).await.expect("insert sealed node");

        let bytes = std::fs::read(&db_path).expect("read sqlite file");
        assert!(
            !bytes_contains(&bytes, marker.as_bytes()),
            "sqlite file must not contain plaintext marker"
        );
    }

    #[tokio::test]
    async fn test_update() {
        let store = SqliteNodeStore::open_in_memory().unwrap();
        let mut node = KnowledgeNode::new(NodeKind::Fact, "original");
        let id = node.id;
        store.insert(&node).await.unwrap();

        node.content = "updated".into();
        node.temporal.version = 2;
        node.temporal.updated_at = Utc::now();
        store.update(&node).await.unwrap();

        let retrieved = store.get(id).await.unwrap().unwrap();
        assert_eq!(retrieved.content, "updated");
        assert_eq!(retrieved.temporal.version, 2);
    }

    #[tokio::test]
    async fn test_delete() {
        let store = SqliteNodeStore::open_in_memory().unwrap();
        let node = KnowledgeNode::new(NodeKind::Fact, "to delete");
        let id = node.id;
        store.insert(&node).await.unwrap();

        assert!(store.delete(id).await.unwrap());
        assert!(store.get(id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_list_with_filters() {
        let store = SqliteNodeStore::open_in_memory().unwrap();

        let n1 = KnowledgeNode::new(NodeKind::Fact, "fact one")
            .with_namespace("dev")
            .with_tags(vec!["rust".into()]);
        let n2 = KnowledgeNode::new(NodeKind::Decision, "decision one")
            .with_namespace("dev")
            .with_tags(vec!["planning".into()]);
        let n3 = KnowledgeNode::new(NodeKind::Fact, "fact two")
            .with_namespace("personal")
            .with_tags(vec!["rust".into()]);

        store.insert(&n1).await.unwrap();
        store.insert(&n2).await.unwrap();
        store.insert(&n3).await.unwrap();

        let filters = QueryFilters {
            namespace: Some("dev".into()),
            ..Default::default()
        };
        let results = store.list(&filters, 10, 0).await.unwrap();
        assert_eq!(results.len(), 2);

        let filters = QueryFilters {
            kinds: Some(vec![NodeKind::Fact]),
            ..Default::default()
        };
        let results = store.list(&filters, 10, 0).await.unwrap();
        assert_eq!(results.len(), 2);

        let filters = QueryFilters {
            tags: Some(vec!["planning".into()]),
            ..Default::default()
        };
        let results = store.list(&filters, 10, 0).await.unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_touch() {
        let store = SqliteNodeStore::open_in_memory().unwrap();
        let node = KnowledgeNode::new(NodeKind::Fact, "touchable");
        let id = node.id;
        store.insert(&node).await.unwrap();

        store.touch(id).await.unwrap();
        let retrieved = store.get(id).await.unwrap().unwrap();
        assert_eq!(retrieved.temporal.access_count, 1);
    }

    #[tokio::test]
    async fn test_count() {
        let store = SqliteNodeStore::open_in_memory().unwrap();
        store
            .insert(&KnowledgeNode::new(NodeKind::Fact, "a"))
            .await
            .unwrap();
        store
            .insert(&KnowledgeNode::new(NodeKind::Fact, "b"))
            .await
            .unwrap();
        store
            .insert(&KnowledgeNode::new(NodeKind::Decision, "c"))
            .await
            .unwrap();

        let all = store.count(&QueryFilters::default()).await.unwrap();
        assert_eq!(all, 3);

        let facts = store
            .count(&QueryFilters {
                kinds: Some(vec![NodeKind::Fact]),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(facts, 2);
    }

    #[tokio::test]
    async fn test_public_share_lifecycle() {
        let store = SqliteNodeStore::open_in_memory().unwrap();
        let node = KnowledgeNode::new(NodeKind::Fact, "Shared note")
            .with_title("Shared")
            .with_namespace("default");
        let node_id = node.id;
        store.insert(&node).await.unwrap();

        let share = PublicShare {
            id: Uuid::now_v7(),
            node_id,
            token_hash: "hash-abc".to_string(),
            created_at: Utc::now(),
            expires_at: None,
            revoked_at: None,
        };

        store.insert_public_share(&share).await.unwrap();

        let fetched = store.get_public_share(share.id).await.unwrap().unwrap();
        assert_eq!(fetched.node_id, node_id);
        assert_eq!(fetched.token_hash, "hash-abc");

        let by_hash = store
            .get_public_share_by_hash("hash-abc")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(by_hash.id, share.id);

        let shares = store
            .list_public_shares(Some(node_id), false)
            .await
            .unwrap();
        assert_eq!(shares.len(), 1);

        let revoked = store
            .revoke_public_share(share.id, Utc::now())
            .await
            .unwrap();
        assert!(revoked);

        let active = store
            .list_public_shares(Some(node_id), false)
            .await
            .unwrap();
        assert!(active.is_empty());

        let all = store.list_public_shares(Some(node_id), true).await.unwrap();
        assert_eq!(all.len(), 1);
        assert!(all[0].revoked_at.is_some());
    }

    // -----------------------------------------------------------------------
    // Integration tests for Phase 1–3 storage features
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_adapter_poll_state_round_trip() {
        let store = SqliteNodeStore::open_in_memory().unwrap();

        // Initially empty
        let state = store.get_poll_state("discord").await.unwrap();
        assert!(state.is_none());

        // Upsert
        store
            .upsert_poll_state("discord", "cursor-abc-123", 5)
            .await
            .unwrap();

        let state = store.get_poll_state("discord").await.unwrap().unwrap();
        assert_eq!(state.adapter_name, "discord");
        assert_eq!(state.cursor, "cursor-abc-123");
        assert_eq!(state.messages_received, 5);

        // Upsert again — cursor updates, messages_received accumulates
        store
            .upsert_poll_state("discord", "cursor-def-456", 3)
            .await
            .unwrap();

        let state = store.get_poll_state("discord").await.unwrap().unwrap();
        assert_eq!(state.cursor, "cursor-def-456");
        assert_eq!(state.messages_received, 8); // 5 + 3

        // List
        store
            .upsert_poll_state("slack", "slack-cursor", 1)
            .await
            .unwrap();
        let all = store.list_poll_states().await.unwrap();
        assert_eq!(all.len(), 2);

        // Delete
        let deleted = store.delete_poll_state("discord").await.unwrap();
        assert!(deleted);
        assert!(store.get_poll_state("discord").await.unwrap().is_none());

        let not_found = store.delete_poll_state("nonexistent").await.unwrap();
        assert!(!not_found);
    }

    #[tokio::test]
    async fn test_contact_identity_storage() {
        let store = SqliteNodeStore::open_in_memory().unwrap();

        let contact_id = Uuid::now_v7();
        let identity = ContactIdentity {
            id: Uuid::now_v7(),
            contact_id,
            identity_type: IdentityType::Email,
            identity_value: "bob@example.com".into(),
            verified: false,
            verified_at: None,
            created_at: Utc::now(),
        };

        // Add
        store.add_contact_identity(&identity).await.unwrap();

        // List
        let list = store.list_contact_identities(contact_id).await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].identity_value, "bob@example.com");
        assert!(!list[0].verified);

        // Verify
        let verified = store.verify_contact_identity(identity.id).await.unwrap();
        assert!(verified);

        let list = store.list_contact_identities(contact_id).await.unwrap();
        assert!(list[0].verified);
        assert!(list[0].verified_at.is_some());

        // Verify again (already verified) — should return false
        let re_verified = store.verify_contact_identity(identity.id).await.unwrap();
        assert!(!re_verified);

        // Delete
        let deleted = store.delete_contact_identity(identity.id).await.unwrap();
        assert!(deleted);
        assert!(store
            .list_contact_identities(contact_id)
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn conversation_messages_persist_sources_json() {
        use mv_core::traits::ConversationStore;

        let store = SqliteNodeStore::open_in_memory().unwrap();
        let conversation_id = Uuid::now_v7();
        store
            .create_conversation(conversation_id, Some("Grounded chat"))
            .await
            .unwrap();

        let sources =
            r#"[{"node_id":"n1","title":"Launch","kind":"fact","score":0.9,"preview":"Ship it"}]"#;
        let msg_id = store
            .add_message(conversation_id, "assistant", "Ship it [1].", Some(sources))
            .await
            .unwrap();

        let messages = store.get_messages(conversation_id, 20).await.unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].0, msg_id);
        assert_eq!(messages[0].1, "assistant");
        assert_eq!(messages[0].2, "Ship it [1].");
        assert_eq!(messages[0].3.as_deref(), Some(sources));

        let plain_id = store
            .add_message(conversation_id, "user", "thanks", None)
            .await
            .unwrap();
        let messages = store.get_messages(conversation_id, 20).await.unwrap();
        assert_eq!(messages.len(), 2);
        let plain = messages.iter().find(|m| m.0 == plain_id).unwrap();
        assert!(plain.3.is_none());
    }

    #[tokio::test]
    async fn test_trust_model_storage() {
        let store = SqliteNodeStore::open_in_memory().unwrap();

        let contact_id = Uuid::now_v7();

        // Not present initially
        assert!(store.get_trust_model(contact_id).await.unwrap().is_none());

        // Set
        let model = TrustModel {
            contact_id,
            can_query: true,
            can_inject_context: false,
            can_auto_reply: false,
            allowed_namespaces: vec!["research".into(), "notes".into()],
            max_confidence_override: Some(0.9),
            updated_at: Utc::now(),
        };
        store.set_trust_model(&model).await.unwrap();

        let stored = store.get_trust_model(contact_id).await.unwrap().unwrap();
        assert!(stored.can_query);
        assert!(!stored.can_inject_context);
        assert_eq!(stored.allowed_namespaces, vec!["research", "notes"]);
        assert!((stored.max_confidence_override.unwrap() - 0.9).abs() < f64::EPSILON);

        // Update (upsert)
        let updated_model = TrustModel {
            contact_id,
            can_query: true,
            can_inject_context: true,
            can_auto_reply: true,
            allowed_namespaces: vec!["all".into()],
            max_confidence_override: None,
            updated_at: Utc::now(),
        };
        store.set_trust_model(&updated_model).await.unwrap();

        let stored = store.get_trust_model(contact_id).await.unwrap().unwrap();
        assert!(stored.can_inject_context);
        assert!(stored.can_auto_reply);
        assert_eq!(stored.allowed_namespaces, vec!["all"]);
        assert!(stored.max_confidence_override.is_none());
    }

    #[test]
    fn knowledge_workspace_migration_installs_complete_manifest_contract() {
        let store = SqliteNodeStore::open_in_memory().unwrap();
        let expected_tables = [
            "workspace_manifest_versions",
            "workspaces",
            "workspace_documents",
            "workspace_events",
            "workspace_document_versions",
            "workspace_conflicts",
            "workspace_migrations",
            "workspace_migration_items",
        ];

        store
            .with_conn(|conn| {
                for table in expected_tables {
                    let present: bool = conn
                        .query_row(
                            "SELECT EXISTS(
                                SELECT 1 FROM sqlite_master
                                WHERE type = 'table' AND name = ?1
                             )",
                            params![table],
                            |row| row.get(0),
                        )
                        .map_err(|err| MvError::Storage(err.to_string()))?;
                    assert!(present, "expected manifest table {table}");
                }

                let contract: String = conn
                    .query_row(
                        "SELECT contract_name FROM workspace_manifest_versions WHERE version = 1",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(|err| MvError::Storage(err.to_string()))?;
                assert_eq!(contract, "knowledge-workspace-manifest-v1");

                let schema_version: i64 = conn
                    .query_row("SELECT MAX(version) FROM schema_version", [], |row| {
                        row.get(0)
                    })
                    .map_err(|err| MvError::Storage(err.to_string()))?;
                assert_eq!(schema_version, 38);
                Ok(())
            })
            .unwrap();
    }

    #[tokio::test]
    async fn knowledge_workspace_manifest_round_trips_without_filesystem_access() {
        let store = SqliteNodeStore::open_in_memory().unwrap();
        let workspace = KnowledgeWorkspace::new(
            "personal",
            KnowledgeWorkspaceMode::Mounted,
            br#"{"display_name":"Vault"}"#.to_vec(),
            WorkspaceManifestPayloadFormat::JsonV1,
        );
        store.insert_knowledge_workspace(&workspace).await.unwrap();

        assert_eq!(
            store.get_knowledge_workspace(workspace.id).await.unwrap(),
            Some(workspace.clone())
        );
        assert_eq!(
            store
                .list_knowledge_workspaces(Some("personal"))
                .await
                .unwrap(),
            vec![workspace.clone()]
        );
        assert!(store
            .list_knowledge_workspaces(Some("other"))
            .await
            .unwrap()
            .is_empty());

        let raw_path = KnowledgeWorkspaceDocument::new(
            workspace.id,
            "notes/private.md",
            br#"{"relative_path":"notes/private.md"}"#.to_vec(),
            WorkspaceManifestPayloadFormat::JsonV1,
        );
        assert!(matches!(
            store.insert_workspace_document(&raw_path).await,
            Err(MvError::InvalidInput(message))
                if message.contains("64 lowercase hexadecimal")
        ));

        let path_token = "a".repeat(64);
        let document = KnowledgeWorkspaceDocument::new(
            workspace.id,
            path_token.clone(),
            br#"{"relative_path":"notes/hello.md","content_hash":"sha256:ab12"}"#.to_vec(),
            WorkspaceManifestPayloadFormat::JsonV1,
        );
        store.insert_workspace_document(&document).await.unwrap();

        assert_eq!(
            store.get_workspace_document(document.id).await.unwrap(),
            Some(document.clone())
        );
        assert_eq!(
            store
                .get_workspace_document_by_path_token(workspace.id, &path_token)
                .await
                .unwrap(),
            Some(document.clone())
        );
        assert_eq!(
            store.list_workspace_documents(workspace.id).await.unwrap(),
            vec![document.clone()]
        );

        let duplicate_path = KnowledgeWorkspaceDocument::new(
            workspace.id,
            document.path_token.clone(),
            br#"{"relative_path":"different.md"}"#.to_vec(),
            WorkspaceManifestPayloadFormat::JsonV1,
        );
        assert!(store
            .insert_workspace_document(&duplicate_path)
            .await
            .is_err());
    }

    #[tokio::test]
    async fn knowledge_workspace_updates_require_the_expected_revision() {
        let store = SqliteNodeStore::open_in_memory().unwrap();
        let mut workspace = KnowledgeWorkspace::new(
            "personal",
            KnowledgeWorkspaceMode::ManagedPlaintext,
            br#"{"display_name":"Managed Vault"}"#.to_vec(),
            WorkspaceManifestPayloadFormat::JsonV1,
        );
        store.insert_knowledge_workspace(&workspace).await.unwrap();

        workspace.state = KnowledgeWorkspaceState::Ready;
        workspace.revision = 2;
        workspace.updated_at = Utc::now();
        assert!(store
            .update_knowledge_workspace(&workspace, 1)
            .await
            .unwrap());

        let mut stale = workspace.clone();
        stale.state = KnowledgeWorkspaceState::Offline;
        assert!(!store.update_knowledge_workspace(&stale, 1).await.unwrap());
        assert_eq!(
            store
                .get_knowledge_workspace(workspace.id)
                .await
                .unwrap()
                .unwrap()
                .state,
            KnowledgeWorkspaceState::Ready
        );

        let mut invalid_advance = workspace.clone();
        invalid_advance.revision = 4;
        assert!(matches!(
            store.update_knowledge_workspace(&invalid_advance, 2).await,
            Err(MvError::InvalidInput(_))
        ));

        let mut document = KnowledgeWorkspaceDocument::new(
            workspace.id,
            "b".repeat(64),
            br#"{"relative_path":"notes/revision.md"}"#.to_vec(),
            WorkspaceManifestPayloadFormat::JsonV1,
        );
        store.insert_workspace_document(&document).await.unwrap();

        document.projection_state = WorkspaceProjectionState::Ready;
        document.revision = 2;
        document.updated_at = Utc::now();
        assert!(store.update_workspace_document(&document, 1).await.unwrap());
        assert!(!store.update_workspace_document(&document, 1).await.unwrap());

        let original_workspace_id = document.workspace_id;
        document.workspace_id = Uuid::now_v7();
        document.revision = 3;
        assert!(!store.update_workspace_document(&document, 2).await.unwrap());
        assert_eq!(
            store
                .get_workspace_document(document.id)
                .await
                .unwrap()
                .unwrap()
                .workspace_id,
            original_workspace_id
        );
    }

    #[tokio::test]
    async fn workspace_reconciliation_rolls_back_when_any_revision_is_stale() {
        let store = SqliteNodeStore::open_in_memory().unwrap();
        let workspace = KnowledgeWorkspace::new(
            "personal",
            KnowledgeWorkspaceMode::Mounted,
            br#"{"display_name":"Atomic Vault"}"#.to_vec(),
            WorkspaceManifestPayloadFormat::JsonV1,
        );
        store.insert_knowledge_workspace(&workspace).await.unwrap();

        let document = KnowledgeWorkspaceDocument::new(
            workspace.id,
            "c".repeat(64),
            br#"{"schema":"test","relative_path":"one.md"}"#.to_vec(),
            WorkspaceManifestPayloadFormat::JsonV1,
        );
        store.insert_workspace_document(&document).await.unwrap();

        let mut workspace_replacement = workspace.clone();
        workspace_replacement.state = KnowledgeWorkspaceState::Ready;
        workspace_replacement.revision = 2;
        workspace_replacement.updated_at = Utc::now();

        let inserted = KnowledgeWorkspaceDocument::new(
            workspace.id,
            "d".repeat(64),
            br#"{"schema":"test","relative_path":"two.md"}"#.to_vec(),
            WorkspaceManifestPayloadFormat::JsonV1,
        );
        let mut stale_replacement = document.clone();
        stale_replacement.lifecycle_state = WorkspaceDocumentLifecycle::Missing;
        stale_replacement.revision = 3;
        stale_replacement.updated_at = Utc::now();
        let reconciliation = WorkspaceManifestReconciliation {
            expected_workspace_revision: 1,
            workspace_replacement,
            document_inserts: vec![inserted.clone()],
            document_updates: vec![WorkspaceDocumentManifestUpdate {
                expected_revision: 2,
                replacement: stale_replacement,
            }],
        };

        assert!(!store
            .apply_workspace_reconciliation(&reconciliation)
            .await
            .unwrap());
        assert_eq!(
            store
                .get_knowledge_workspace(workspace.id)
                .await
                .unwrap()
                .unwrap()
                .revision,
            1
        );
        assert!(store
            .get_workspace_document(inserted.id)
            .await
            .unwrap()
            .is_none());
        assert_eq!(
            store
                .get_workspace_document(document.id)
                .await
                .unwrap()
                .unwrap()
                .lifecycle_state,
            WorkspaceDocumentLifecycle::Active
        );
    }

    #[tokio::test]
    async fn knowledge_workspace_sealed_storage_rejects_plaintext_payloads() {
        let store = SqliteNodeStore::open_in_memory_with_mode(true).unwrap();
        let workspace = KnowledgeWorkspace::new(
            "personal",
            KnowledgeWorkspaceMode::Mounted,
            br#"{"root":"/sensitive/path"}"#.to_vec(),
            WorkspaceManifestPayloadFormat::JsonV1,
        );

        assert!(matches!(
            store.insert_knowledge_workspace(&workspace).await,
            Err(MvError::InvalidInput(message))
                if message.contains("requires mvenc-v1")
        ));
    }

    fn identity_event(
        local_node_id: Uuid,
        identity: &IdentityRecord,
        key: &str,
        data: serde_json::Value,
    ) -> EventEnvelope {
        let mut event = governance_event(
            local_node_id,
            IDENTITY_REGISTERED_V1,
            "identity-registered",
            identity.principal_uri.clone(),
            key,
            data,
        );
        event.principal = identity.principal_uri.clone();
        event.actor = identity.principal_uri.clone();
        event.payload_digest = identity.semantic_digest();
        event
    }

    #[tokio::test]
    async fn identity_registry_registers_and_reads_actor_kind() {
        let store = SqliteNodeStore::open_in_memory().unwrap();
        let local_node_id = store.local_context_node_id().await.unwrap();
        let identity = IdentityRecord::bootstrap(
            local_node_id,
            "owner",
            ActorKind::Human,
            "Owner",
        )
        .unwrap();
        let data = serde_json::json!({
            "principal_id": identity.principal_id,
            "actor_kind": identity.actor_kind.as_str(),
            "status": identity.status.as_str(),
            "record_digest": identity.semantic_digest(),
            "subject_binding_digest": identity.subject_binding_digest,
        });
        let event = identity_event(
            local_node_id,
            &identity,
            "identity-register-owner",
            data,
        );
        let commit = store
            .commit_identity_with_event(&identity, &event)
            .await
            .unwrap();
        assert!(!commit.replayed);
        let loaded = store.get_identity(identity.principal_id).await.unwrap().unwrap();
        assert_eq!(loaded.actor_kind, ActorKind::Human);
        assert_eq!(loaded.subject_binding, "owner");
    }

    #[tokio::test]
    async fn identity_registry_subject_binding_lookup_returns_record() {
        let store = SqliteNodeStore::open_in_memory().unwrap();
        let local_node_id = store.local_context_node_id().await.unwrap();
        let node_uri = StableUri::node(local_node_id);
        let identity = IdentityRecord::bootstrap(
            local_node_id,
            "local-system",
            ActorKind::Human,
            "Local System",
        )
        .unwrap();
        let data = serde_json::json!({
            "principal_id": identity.principal_id,
            "actor_kind": identity.actor_kind.as_str(),
            "status": identity.status.as_str(),
            "record_digest": identity.semantic_digest(),
            "subject_binding_digest": identity.subject_binding_digest,
        });
        store
            .commit_identity_with_event(
                &identity,
                &identity_event(local_node_id, &identity, "identity-register-local-system", data),
            )
            .await
            .unwrap();
        let loaded = store
            .get_identity_by_subject_binding(&node_uri, "local-system")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(loaded.principal_id, identity.principal_id);
    }

    #[tokio::test]
    async fn identity_registry_local_system_principal_id_matches_v5_derivation() {
        let local_node_id = Uuid::now_v7();
        let identity = IdentityRecord::bootstrap(
            local_node_id,
            "local-system",
            ActorKind::Human,
            "Local System",
        )
        .unwrap();
        assert_eq!(
            identity.principal_id,
            IdentityRecord::principal_id_for_subject(local_node_id, "local-system")
        );
        assert_eq!(
            identity.principal_uri,
            StableUri::principal(
                local_node_id,
                Uuid::new_v5(&local_node_id, b"local-system"),
            )
        );
    }

    #[tokio::test]
    async fn identity_registry_duplicate_subject_binding_is_idempotent() {
        let store = SqliteNodeStore::open_in_memory().unwrap();
        let local_node_id = store.local_context_node_id().await.unwrap();
        let identity = IdentityRecord::bootstrap(
            local_node_id,
            "local-context-owner",
            ActorKind::Human,
            "Local Context Owner",
        )
        .unwrap();
        let data = serde_json::json!({
            "principal_id": identity.principal_id,
            "actor_kind": identity.actor_kind.as_str(),
            "status": identity.status.as_str(),
            "record_digest": identity.semantic_digest(),
            "subject_binding_digest": identity.subject_binding_digest,
        });
        let event = identity_event(
            local_node_id,
            &identity,
            "identity-register-owner-once",
            data.clone(),
        );
        let first = store
            .commit_identity_with_event(&identity, &event)
            .await
            .unwrap();
        assert!(!first.replayed);
        let retry = identity_event(
            local_node_id,
            &identity,
            "identity-register-owner-again",
            data,
        );
        let second = store
            .commit_identity_with_event(&identity, &retry)
            .await
            .unwrap();
        assert!(second.replayed);
        assert_eq!(second.identity.principal_id, identity.principal_id);
    }

    #[tokio::test]
    async fn identity_registry_bootstrap_schema_digest_matches_definition() {
        let store = SqliteNodeStore::open_in_memory().unwrap();
        let schema_reference =
            SchemaReference::new(StableUri::schema("identity-registered").unwrap(), "1.0.0")
                .unwrap();
        let schema = store
            .get_public_schema(&schema_reference)
            .await
            .unwrap()
            .expect("identity registration schema");

        assert_eq!(
            schema.definition["$schema"],
            "https://json-schema.org/draft/2020-12/schema"
        );
        assert_eq!(
            schema.content_digest,
            "95c98ffd9423c88e1483973ba594583760b8d1a119f5b3d26a5c602cf984d7b1"
        );
        assert_eq!(
            schema.content_digest,
            canonical_json_sha256(&schema.definition)
        );
    }

    #[tokio::test]
    async fn context_node_registration_is_atomic_listable_and_idempotent() {
        let store = SqliteNodeStore::open_in_memory().unwrap();
        let local_node_id = store.local_context_node_id().await.unwrap();
        let context_node = test_local_context_node(local_node_id);
        let data = serde_json::json!({
            "node_id": context_node.node_id,
            "node_type": context_node.node_type.as_str(),
            "status": context_node.status.as_str(),
            "record_digest": context_node.semantic_digest(),
            "capability_digest": context_node.capability_manifest.content_digest,
        });
        let event = context_node_event(
            local_node_id,
            CONTEXT_NODE_REGISTERED_V1,
            "context-node-registered",
            &context_node,
            "context-node-registration-idempotency",
            data.clone(),
        );
        let first = store
            .commit_context_node_with_event(&context_node, &event)
            .await
            .unwrap();
        assert!(!first.replayed);
        assert_eq!(
            store.get_context_node(local_node_id).await.unwrap(),
            Some(context_node.clone())
        );
        assert_eq!(
            store
                .list_context_nodes(Some(ContextNodeStatus::Active))
                .await
                .unwrap(),
            vec![context_node.clone()]
        );

        let retry_event = context_node_event(
            local_node_id,
            CONTEXT_NODE_REGISTERED_V1,
            "context-node-registered",
            &context_node,
            "context-node-registration-idempotency",
            data,
        );
        let replay = store
            .commit_context_node_with_event(&context_node, &retry_event)
            .await
            .unwrap();
        assert!(replay.replayed);
        assert_eq!(replay.event.id, first.event.id);
        assert_eq!(replay.context_node, context_node);
    }

    #[tokio::test]
    async fn context_node_updates_archive_revisions_and_replay_exact_history() {
        let store = SqliteNodeStore::open_in_memory().unwrap();
        let original = register_local_context_node(&store).await;

        let mut descriptor_update = original.clone();
        descriptor_update.revision = 2;
        descriptor_update.display_name = "Personal Vault v2".into();
        descriptor_update.updated_at = Utc::now();
        let update_data = serde_json::json!({
            "node_id": descriptor_update.node_id,
            "revision": descriptor_update.revision,
            "record_digest": descriptor_update.semantic_digest(),
            "capability_digest": descriptor_update.capability_manifest.content_digest,
        });
        let update_event = context_node_event(
            original.node_id,
            CONTEXT_NODE_DESCRIPTOR_UPDATED_V1,
            "context-node-descriptor-updated",
            &descriptor_update,
            "update-local-context-node",
            update_data.clone(),
        );
        let updated = store
            .update_context_node_descriptor_with_event(1, &descriptor_update, &update_event)
            .await
            .unwrap();
        assert!(!updated.replayed);

        let mut suspended = descriptor_update.clone();
        suspended.revision = 3;
        suspended.status = ContextNodeStatus::Suspended;
        suspended.updated_at = Utc::now();
        let transition_data = serde_json::json!({
            "node_id": suspended.node_id,
            "revision": suspended.revision,
            "from_status": ContextNodeStatus::Active.as_str(),
            "to_status": suspended.status.as_str(),
            "record_digest": suspended.semantic_digest(),
        });
        let transition_event = context_node_event(
            original.node_id,
            CONTEXT_NODE_LIFECYCLE_TRANSITIONED_V1,
            "context-node-lifecycle-transitioned",
            &suspended,
            "suspend-local-context-node",
            transition_data,
        );
        store
            .transition_context_node_with_event(2, &suspended, &transition_event)
            .await
            .unwrap();

        let retry_event = context_node_event(
            original.node_id,
            CONTEXT_NODE_DESCRIPTOR_UPDATED_V1,
            "context-node-descriptor-updated",
            &descriptor_update,
            "update-local-context-node",
            update_data,
        );
        let replay = store
            .update_context_node_descriptor_with_event(1, &descriptor_update, &retry_event)
            .await
            .unwrap();
        assert!(replay.replayed);
        assert_eq!(replay.event.id, update_event.id);
        assert_eq!(replay.context_node, descriptor_update);
        assert_eq!(
            store
                .get_context_node(original.node_id)
                .await
                .unwrap()
                .unwrap(),
            suspended
        );
    }

    #[tokio::test]
    async fn invalid_context_node_transition_leaves_no_event_or_mutation() {
        let store = SqliteNodeStore::open_in_memory().unwrap();
        let original = register_local_context_node(&store).await;
        let mut retired = original.clone();
        retired.revision = 2;
        retired.status = ContextNodeStatus::Retired;
        retired.updated_at = Utc::now();
        let data = serde_json::json!({
            "node_id": retired.node_id,
            "revision": retired.revision,
            "from_status": original.status.as_str(),
            "to_status": retired.status.as_str(),
            "record_digest": retired.semantic_digest(),
        });
        let event = context_node_event(
            original.node_id,
            CONTEXT_NODE_LIFECYCLE_TRANSITIONED_V1,
            "context-node-lifecycle-transitioned",
            &retired,
            "reject-context-node-lifecycle-skip",
            data,
        );
        assert!(matches!(
            store
                .transition_context_node_with_event(1, &retired, &event)
                .await,
            Err(MvError::InvalidInput(message)) if message.contains("not allowed")
        ));
        assert_eq!(
            store.get_context_node(original.node_id).await.unwrap(),
            Some(original)
        );
        assert!(store.get_outbox_event(event.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn sealed_context_node_payload_hides_descriptor_details() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("context-node-sealed.sqlite");
        let _reset = install_scoped_runtime_key(&db_path, [21u8; 32]);
        let store = SqliteNodeStore::open_with_mode(&db_path, true).unwrap();
        let local_node_id = store.local_context_node_id().await.unwrap();
        let marker = format!("sealed-context-marker-{}", Uuid::now_v7());
        let manifest = ContextCapabilityManifest::new(
            vec![ContextCapability::Discover],
            vec![ContextProtocolProfile {
                protocol: "mcp".into(),
                version: "2025-11-25".into(),
                roles: vec!["server".into()],
            }],
            Vec::new(),
        )
        .unwrap();
        let mut context_node = ContextNodeRecord::discovered(
            local_node_id,
            ContextNodeType::Personal,
            StableUri::principal(local_node_id, Uuid::now_v7()),
            StableUri::node(local_node_id),
            marker.clone(),
            manifest,
        )
        .unwrap();
        context_node.trust_class = ContextNodeTrustClass::local();
        context_node.status = ContextNodeStatus::Active;
        context_node.endpoints = vec![ContextNodeEndpoint {
            protocol: "mcp".into(),
            uri: format!("https://example.invalid/{marker}"),
        }];
        context_node.public_keys = vec![ContextNodePublicKey {
            key_id: "primary".into(),
            algorithm: "ed25519".into(),
            public_key_multibase: format!("z{marker}"),
        }];
        let data = serde_json::json!({
            "node_id": context_node.node_id,
            "node_type": context_node.node_type.as_str(),
            "status": context_node.status.as_str(),
            "record_digest": context_node.semantic_digest(),
            "capability_digest": context_node.capability_manifest.content_digest,
        });
        let event = context_node_event(
            local_node_id,
            CONTEXT_NODE_REGISTERED_V1,
            "context-node-registered",
            &context_node,
            "register-sealed-local-context-node",
            data,
        );
        store
            .commit_context_node_with_event(&context_node, &event)
            .await
            .unwrap();

        let (payload, format, wrapped): (Vec<u8>, String, Option<String>) = store
            .with_conn(|connection| {
                connection
                    .query_row(
                        "SELECT record_payload, payload_format, payload_wrapped_dek
                         FROM interoperability_context_nodes WHERE node_id = ?1",
                        params![local_node_id.to_string()],
                        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                    )
                    .map_err(|err| MvError::Storage(err.to_string()))
            })
            .unwrap();
        assert_eq!(format, "mvenc-v1");
        assert!(wrapped.is_some());
        assert!(!bytes_contains(&payload, marker.as_bytes()));
        assert_eq!(
            store.get_context_node(local_node_id).await.unwrap(),
            Some(context_node)
        );
    }

    #[tokio::test]
    async fn authority_grant_issuance_is_atomic_queryable_and_idempotent() {
        let store = SqliteNodeStore::open_in_memory().unwrap();
        let context_node = register_local_context_node(&store).await;
        let grantee = StableUri::principal(Uuid::now_v7(), Uuid::now_v7());
        let target = StableUri::knowledge_node(context_node.node_id, Uuid::now_v7());
        let grant = AuthorityGrant::new_context(
            context_node.node_uri.clone(),
            StableUri::principal(context_node.node_id, Uuid::now_v7()),
            grantee.clone(),
            vec![target.clone()],
            vec![ContextCapability::Read],
            "Read one governed knowledge node",
            Utc::now() + chrono::Duration::hours(1),
        )
        .unwrap();

        let mut invalid_grantor = grant.clone();
        invalid_grantor.grantor = StableUri::parse(format!(
            "mindvault://{}/identity/principal/{}/delegated",
            context_node.node_id,
            Uuid::now_v7()
        ))
        .unwrap();
        let invalid_event = authority_grant_event(
            context_node.node_id,
            AUTHORITY_GRANT_ISSUED_V1,
            "authority-grant-issued",
            &invalid_grantor,
            "reject-noncanonical-root-grantor",
            serde_json::json!({
                "grant_id": invalid_grantor.grant_id,
                "grant_kind": invalid_grantor.kind.as_str(),
                "grantee_uri": invalid_grantor.grantee.as_str(),
                "governing_node_uri": invalid_grantor.governing_node.as_str(),
                "record_digest": invalid_grantor.semantic_digest(),
            }),
        );
        assert!(matches!(
            store
                .commit_authority_grant_with_event(&invalid_grantor, &invalid_event)
                .await,
            Err(MvError::InvalidInput(message)) if message.contains("local principal")
        ));
        assert!(store
            .get_outbox_event(invalid_event.id)
            .await
            .unwrap()
            .is_none());

        let data = serde_json::json!({
            "grant_id": grant.grant_id,
            "grant_kind": grant.kind.as_str(),
            "grantee_uri": grant.grantee.as_str(),
            "governing_node_uri": grant.governing_node.as_str(),
            "record_digest": grant.semantic_digest(),
        });
        let event = authority_grant_event(
            context_node.node_id,
            AUTHORITY_GRANT_ISSUED_V1,
            "authority-grant-issued",
            &grant,
            "issue-context-grant-idempotently",
            data.clone(),
        );

        let first = store
            .commit_authority_grant_with_event(&grant, &event)
            .await
            .unwrap();
        assert!(!first.replayed);
        assert_eq!(
            store.get_authority_grant(grant.grant_id).await.unwrap(),
            Some(grant.clone())
        );
        assert_eq!(
            store
                .list_authority_grants(
                    Some(&grantee),
                    Some(AuthorityGrantKind::Context),
                    Some(AuthorityGrantStatus::Active),
                )
                .await
                .unwrap(),
            vec![grant.clone()]
        );
        assert_eq!(
            store
                .find_authorizing_grant(GrantQuery {
                    grantee: &grantee,
                    kind: AuthorityGrantKind::Context,
                    target: &target,
                    capability: ContextCapability::Read,
                    sensitivity: Sensitivity::Internal,
                    retention: RetentionClass::Operational,
                    at: Utc::now(),
                })
                .await
                .unwrap(),
            Some(grant.clone())
        );
        assert!(store
            .find_authorizing_grant(GrantQuery {
                grantee: &grantee,
                kind: AuthorityGrantKind::Tool,
                target: &target,
                capability: ContextCapability::Read,
                sensitivity: Sensitivity::Internal,
                retention: RetentionClass::Operational,
                at: Utc::now(),
            })
            .await
            .unwrap()
            .is_none());

        let retry_event = authority_grant_event(
            context_node.node_id,
            AUTHORITY_GRANT_ISSUED_V1,
            "authority-grant-issued",
            &grant,
            "issue-context-grant-idempotently",
            data,
        );
        let replay = store
            .commit_authority_grant_with_event(&grant, &retry_event)
            .await
            .unwrap();
        assert!(replay.replayed);
        assert_eq!(replay.event.id, first.event.id);
        assert_eq!(replay.grant, grant);
    }

    #[tokio::test]
    async fn parent_suspension_invalidates_delegated_authority() {
        let store = SqliteNodeStore::open_in_memory().unwrap();
        let context_node = register_local_context_node(&store).await;
        let delegate = StableUri::principal(Uuid::now_v7(), Uuid::now_v7());
        let recipient = StableUri::principal(Uuid::now_v7(), Uuid::now_v7());
        let target = StableUri::knowledge_node(context_node.node_id, Uuid::now_v7());
        let parent_expiry = Utc::now() + chrono::Duration::hours(2);
        let mut parent = AuthorityGrant::new_context(
            context_node.node_uri.clone(),
            StableUri::principal(context_node.node_id, Uuid::now_v7()),
            delegate.clone(),
            vec![target.clone()],
            vec![ContextCapability::Query, ContextCapability::Read],
            "Delegate bounded knowledge access",
            parent_expiry,
        )
        .unwrap();
        parent.delegation_depth_remaining = 2;
        parent.validate().unwrap();
        let parent_event = authority_grant_event(
            context_node.node_id,
            AUTHORITY_GRANT_ISSUED_V1,
            "authority-grant-issued",
            &parent,
            "issue-parent-context-grant",
            serde_json::json!({
                "grant_id": parent.grant_id,
                "grant_kind": parent.kind.as_str(),
                "grantee_uri": parent.grantee.as_str(),
                "governing_node_uri": parent.governing_node.as_str(),
                "record_digest": parent.semantic_digest(),
            }),
        );
        store
            .commit_authority_grant_with_event(&parent, &parent_event)
            .await
            .unwrap();

        let mut child = AuthorityGrant::new_context(
            context_node.node_uri.clone(),
            delegate,
            recipient.clone(),
            vec![target.clone()],
            vec![ContextCapability::Read],
            "Read through one bounded delegation",
            parent_expiry - chrono::Duration::minutes(30),
        )
        .unwrap();
        child.parent_grant_id = Some(parent.grant_id);
        child.delegation_depth_remaining = 1;
        child.validate().unwrap();
        let child_event = authority_grant_event(
            context_node.node_id,
            AUTHORITY_GRANT_ISSUED_V1,
            "authority-grant-issued",
            &child,
            "issue-child-context-grant",
            serde_json::json!({
                "grant_id": child.grant_id,
                "grant_kind": child.kind.as_str(),
                "grantee_uri": child.grantee.as_str(),
                "governing_node_uri": child.governing_node.as_str(),
                "record_digest": child.semantic_digest(),
            }),
        );
        store
            .commit_authority_grant_with_event(&child, &child_event)
            .await
            .unwrap();
        assert!(store
            .find_authorizing_grant(GrantQuery {
                grantee: &recipient,
                kind: AuthorityGrantKind::Context,
                target: &target,
                capability: ContextCapability::Read,
                sensitivity: Sensitivity::Internal,
                retention: RetentionClass::Operational,
                at: Utc::now(),
            })
            .await
            .unwrap()
            .is_some());

        let mut suspended = parent.clone();
        suspended.revision = 2;
        suspended.status = AuthorityGrantStatus::Suspended;
        suspended.status_reason = Some("Delegation under review".into());
        suspended.updated_at = Utc::now();
        let transition_event = authority_grant_event(
            context_node.node_id,
            AUTHORITY_GRANT_LIFECYCLE_TRANSITIONED_V1,
            "authority-grant-lifecycle-transitioned",
            &suspended,
            "suspend-parent-context-grant",
            serde_json::json!({
                "grant_id": suspended.grant_id,
                "revision": suspended.revision,
                "from_status": parent.status.as_str(),
                "to_status": suspended.status.as_str(),
                "record_digest": suspended.semantic_digest(),
            }),
        );
        store
            .transition_authority_grant_with_event(1, &suspended, &transition_event)
            .await
            .unwrap();

        assert!(store
            .find_authorizing_grant(GrantQuery {
                grantee: &recipient,
                kind: AuthorityGrantKind::Context,
                target: &target,
                capability: ContextCapability::Read,
                sensitivity: Sensitivity::Internal,
                retention: RetentionClass::Operational,
                at: Utc::now(),
            })
            .await
            .unwrap()
            .is_none());
        let archived_revisions: i64 = store
            .with_conn(|connection| {
                connection
                    .query_row(
                        "SELECT COUNT(*) FROM interoperability_authority_grant_history
                         WHERE grant_id = ?1",
                        params![parent.grant_id.to_string()],
                        |row| row.get(0),
                    )
                    .map_err(|err| MvError::Storage(err.to_string()))
            })
            .unwrap();
        assert_eq!(archived_revisions, 1);
    }

    #[tokio::test]
    async fn sealed_authority_grant_payload_hides_sensitive_terms() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("authority-grant-sealed.sqlite");
        let _reset = install_scoped_runtime_key(&db_path, [23u8; 32]);
        let store = SqliteNodeStore::open_with_mode(&db_path, true).unwrap();
        let context_node = register_local_context_node(&store).await;
        let marker = format!("sealed-grant-purpose-{}", Uuid::now_v7());
        let grant = AuthorityGrant::new_tool(
            context_node.node_uri.clone(),
            StableUri::principal(context_node.node_id, Uuid::now_v7()),
            StableUri::principal(Uuid::now_v7(), Uuid::now_v7()),
            vec![StableUri::knowledge_node(
                context_node.node_id,
                Uuid::now_v7(),
            )],
            vec![ContextCapability::Execute],
            marker.clone(),
            Utc::now() + chrono::Duration::hours(1),
        )
        .unwrap();
        let event = authority_grant_event(
            context_node.node_id,
            AUTHORITY_GRANT_ISSUED_V1,
            "authority-grant-issued",
            &grant,
            "issue-sealed-tool-grant",
            serde_json::json!({
                "grant_id": grant.grant_id,
                "grant_kind": grant.kind.as_str(),
                "grantee_uri": grant.grantee.as_str(),
                "governing_node_uri": grant.governing_node.as_str(),
                "record_digest": grant.semantic_digest(),
            }),
        );
        store
            .commit_authority_grant_with_event(&grant, &event)
            .await
            .unwrap();

        let (payload, format, wrapped): (Vec<u8>, String, Option<String>) = store
            .with_conn(|connection| {
                connection
                    .query_row(
                        "SELECT record_payload, payload_format, payload_wrapped_dek
                         FROM interoperability_authority_grants WHERE grant_id = ?1",
                        params![grant.grant_id.to_string()],
                        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                    )
                    .map_err(|err| MvError::Storage(err.to_string()))
            })
            .unwrap();
        assert_eq!(format, "mvenc-v1");
        assert!(wrapped.is_some());
        assert!(!bytes_contains(&payload, marker.as_bytes()));
        assert_eq!(
            store.get_authority_grant(grant.grant_id).await.unwrap(),
            Some(grant)
        );
    }

    #[tokio::test]
    async fn public_schema_registration_is_immutable_atomic_and_idempotent() {
        let store = SqliteNodeStore::open_in_memory().unwrap();
        let local_node_id = store.local_context_node_id().await.unwrap();
        let bootstrap_reference = SchemaReference::new(
            StableUri::schema("knowledge-node-created").unwrap(),
            "1.0.0",
        )
        .unwrap();
        let bootstrap = store
            .get_public_schema(&bootstrap_reference)
            .await
            .unwrap()
            .expect("bootstrap event schema");
        assert_eq!(
            bootstrap.definition["x-mindvault-event-type"],
            KNOWLEDGE_NODE_CREATED_V1
        );

        let schema = PublicSchemaRecord::new(
            SchemaReference::new(
                StableUri::schema("portable-source-record").unwrap(),
                "1.0.0",
            )
            .unwrap(),
            serde_json::json!({
                "$schema": JSON_SCHEMA_DRAFT_2020_12,
                "type": "object",
                "required": ["resource_uri"],
            }),
            StableUri::node(local_node_id),
        )
        .unwrap();
        let subject =
            StableUri::schema_version(&schema.schema.uri, &schema.schema.version).unwrap();
        let data = serde_json::json!({
            "schema_uri": schema.schema.uri.as_str(),
            "schema_version": schema.schema.version,
            "content_digest": schema.content_digest,
        });
        let event = governance_event(
            local_node_id,
            PUBLIC_SCHEMA_REGISTERED_V1,
            "public-schema-registered",
            subject.clone(),
            "register-portable-source-record",
            data.clone(),
        );

        let first = store
            .commit_public_schema_with_event(&schema, &event)
            .await
            .unwrap();
        assert!(!first.replayed);
        assert_eq!(
            store.get_public_schema(&schema.schema).await.unwrap(),
            Some(schema.clone())
        );

        let retry = governance_event(
            local_node_id,
            PUBLIC_SCHEMA_REGISTERED_V1,
            "public-schema-registered",
            subject,
            "register-portable-source-record",
            data,
        );
        let replay = store
            .commit_public_schema_with_event(&schema, &retry)
            .await
            .unwrap();
        assert!(replay.replayed);
        assert_eq!(replay.event.id, first.event.id);
        assert_eq!(
            store
                .list_public_schema_versions(&schema.schema.uri)
                .await
                .unwrap(),
            vec![schema.clone()]
        );
        let immutable_update = store.with_conn(|connection| {
            connection
                .execute(
                    "UPDATE interoperability_public_schemas
                     SET owner_uri = 'mindvault://schemas/other-owner'
                     WHERE schema_uri = ?1 AND schema_version = ?2",
                    params![schema.schema.uri.as_str(), &schema.schema.version],
                )
                .map(|_| ())
                .map_err(|err| MvError::Storage(err.to_string()))
        });
        assert!(matches!(immutable_update, Err(MvError::Storage(message))
            if message.contains("immutable")));

        let mut changed = schema.clone();
        changed.definition["type"] = serde_json::json!("array");
        assert!(matches!(
            store
                .commit_public_schema_with_event(&changed, &retry)
                .await,
            Err(MvError::InvalidInput(_))
        ));
    }

    #[tokio::test]
    async fn outbox_schema_admission_rejects_a_mismatched_event_type() {
        let store = SqliteNodeStore::open_in_memory().unwrap();
        let local_node_id = store.local_context_node_id().await.unwrap();
        let subject =
            StableUri::schema_version(&StableUri::schema("portable-mismatch").unwrap(), "1.0.0")
                .unwrap();
        let event = governance_event(
            local_node_id,
            PUBLIC_SCHEMA_REGISTERED_V1,
            "public-schema-registered",
            subject,
            "reject-mismatched-event-schema",
            serde_json::json!({
                "schema_uri": "mindvault://schemas/portable-mismatch",
                "schema_version": "1.0.0",
                "content_digest": "a".repeat(64),
            }),
        );
        let envelope_json = serde_json::to_string(&event).unwrap();
        let mismatched_schema = StableUri::schema("knowledge-node-created").unwrap();

        let result = store.with_conn(|connection| {
            connection
                .execute(
                    "INSERT INTO interoperability_outbox
                     (event_id, source_uri, principal_uri, event_type, subject_uri, schema_uri,
                      schema_version, correlation_id, causation_id, idempotency_key,
                      payload_digest, envelope_json, created_at, next_attempt_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                             ?13, ?13)",
                    params![
                        event.id.to_string(),
                        event.source.as_str(),
                        event.principal.as_str(),
                        &event.event_type,
                        event.subject.as_str(),
                        mismatched_schema.as_str(),
                        &event.schema.version,
                        event.correlation_id.to_string(),
                        event.causation_id.map(|id| id.to_string()),
                        event.idempotency_key.as_str(),
                        &event.payload_digest,
                        envelope_json,
                        event.occurred_at.to_rfc3339(),
                    ],
                )
                .map(|_| ())
                .map_err(|err| MvError::Storage(err.to_string()))
        });

        assert!(matches!(result, Err(MvError::Storage(message))
            if message.contains("not admitted")));
        assert!(store.get_outbox_event(event.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn source_binding_requires_an_active_registered_context_node() {
        let store = SqliteNodeStore::open_in_memory().unwrap();
        let local_node_id = store.local_context_node_id().await.unwrap();
        let binding = test_source_binding(local_node_id, "account-unregistered", "event-1");
        let event = governance_event(
            local_node_id,
            SOURCE_BINDING_REGISTERED_V1,
            "source-binding-registered",
            StableUri::source_binding(local_node_id, binding.binding_id),
            "reject-unregistered-context-node",
            serde_json::json!({
                "binding_id": binding.binding_id,
                "resource_uri": binding.resource_uri.as_str(),
                "external_system": binding.external_system,
                "materialization_mode": binding.materialization_mode.as_str(),
            }),
        );
        assert!(matches!(
            store.commit_source_binding_with_event(&binding, &event).await,
            Err(MvError::InvalidInput(message)) if message.contains("not registered")
        ));
        assert!(store.get_outbox_event(event.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn duplicate_active_binding_is_rejected_without_an_orphan_event() {
        let store = SqliteNodeStore::open_in_memory().unwrap();
        let local_node_id = register_local_context_node(&store).await.node_id;
        let binding = test_source_binding(local_node_id, "account-1", "event-1");
        let event_data = serde_json::json!({
            "binding_id": binding.binding_id,
            "resource_uri": binding.resource_uri.as_str(),
            "external_system": binding.external_system,
            "materialization_mode": binding.materialization_mode.as_str(),
        });
        let event = governance_event(
            local_node_id,
            SOURCE_BINDING_REGISTERED_V1,
            "source-binding-registered",
            StableUri::source_binding(local_node_id, binding.binding_id),
            "register-calendar-event-1",
            event_data,
        );
        store
            .commit_source_binding_with_event(&binding, &event)
            .await
            .unwrap();

        let duplicate = test_source_binding(local_node_id, "account-1", "event-1");
        let duplicate_data = serde_json::json!({
            "binding_id": duplicate.binding_id,
            "resource_uri": duplicate.resource_uri.as_str(),
            "external_system": duplicate.external_system,
            "materialization_mode": duplicate.materialization_mode.as_str(),
        });
        let duplicate_event = governance_event(
            local_node_id,
            SOURCE_BINDING_REGISTERED_V1,
            "source-binding-registered",
            StableUri::source_binding(local_node_id, duplicate.binding_id),
            "duplicate-calendar-event-1",
            duplicate_data,
        );
        assert!(store
            .commit_source_binding_with_event(&duplicate, &duplicate_event)
            .await
            .is_err());
        assert!(store
            .get_outbox_event(duplicate_event.id)
            .await
            .unwrap()
            .is_none());
        assert_eq!(
            store
                .find_active_source_binding(
                    &StableUri::node(local_node_id),
                    "account-1",
                    "event-1",
                )
                .await
                .unwrap(),
            Some(binding)
        );
    }

    #[tokio::test]
    async fn rebinding_is_atomic_and_preserves_the_active_predecessor_revision() {
        let store = SqliteNodeStore::open_in_memory().unwrap();
        let local_node_id = register_local_context_node(&store).await.node_id;
        let binding = test_source_binding(local_node_id, "account-2", "event-2");
        let create_event = governance_event(
            local_node_id,
            SOURCE_BINDING_REGISTERED_V1,
            "source-binding-registered",
            StableUri::source_binding(local_node_id, binding.binding_id),
            "register-calendar-event-2",
            serde_json::json!({
                "binding_id": binding.binding_id,
                "resource_uri": binding.resource_uri.as_str(),
                "external_system": binding.external_system,
                "materialization_mode": binding.materialization_mode.as_str(),
            }),
        );
        store
            .commit_source_binding_with_event(&binding, &create_event)
            .await
            .unwrap();

        let mut replacement = test_source_binding(local_node_id, "account-2", "event-2");
        replacement.resource_uri = binding.resource_uri.clone();
        replacement.supersedes_binding_id = Some(binding.binding_id);
        let rebind_data = serde_json::json!({
            "binding_id": replacement.binding_id,
            "supersedes_binding_id": binding.binding_id,
        });
        let rebind_event = governance_event(
            local_node_id,
            SOURCE_BINDING_REBOUND_V1,
            "source-binding-rebound",
            StableUri::source_binding(local_node_id, replacement.binding_id),
            "rebind-calendar-event-2",
            rebind_data.clone(),
        );
        let committed = store
            .rebind_source_with_event(binding.binding_id, &replacement, &rebind_event)
            .await
            .unwrap();
        assert!(!committed.replayed);

        let retired = store
            .get_source_binding(binding.binding_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(retired.status, SourceBindingStatus::Migrated);
        assert_eq!(retired.revision, 2);
        assert_eq!(
            store
                .find_active_source_binding(
                    &StableUri::node(local_node_id),
                    "account-2",
                    "event-2",
                )
                .await
                .unwrap(),
            Some(replacement.clone())
        );

        store
            .with_conn(|connection| {
                let archived: (u64, String) = connection
                    .query_row(
                        "SELECT revision, status
                         FROM interoperability_source_binding_history
                         WHERE binding_id = ?1",
                        params![binding.binding_id.to_string()],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .map_err(|err| MvError::Storage(err.to_string()))?;
                assert_eq!(archived, (1, "active".into()));
                Ok(())
            })
            .unwrap();

        let retry = governance_event(
            local_node_id,
            SOURCE_BINDING_REBOUND_V1,
            "source-binding-rebound",
            StableUri::source_binding(local_node_id, replacement.binding_id),
            "rebind-calendar-event-2",
            rebind_data,
        );
        let replay = store
            .rebind_source_with_event(binding.binding_id, &replacement, &retry)
            .await
            .unwrap();
        assert!(replay.replayed);
        assert_eq!(replay.event.id, committed.event.id);
    }

    #[tokio::test]
    async fn sealed_source_binding_payload_hides_external_identifiers_and_cursor() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("sealed_source_binding.sqlite");
        let _reset = install_scoped_runtime_key(&db_path, [31u8; 32]);
        let store = SqliteNodeStore::open_with_mode(&db_path, true).unwrap();
        let local_node_id = register_local_context_node(&store).await.node_id;
        let marker = format!("private-account-{}", Uuid::now_v7());
        let cursor = format!("private-cursor-{}", Uuid::now_v7());
        let mut binding = test_source_binding(local_node_id, &marker, "private-event");
        binding.last_sync_cursor = Some(cursor.clone());
        let event = governance_event(
            local_node_id,
            SOURCE_BINDING_REGISTERED_V1,
            "source-binding-registered",
            StableUri::source_binding(local_node_id, binding.binding_id),
            "register-sealed-source-binding",
            serde_json::json!({
                "binding_id": binding.binding_id,
                "resource_uri": binding.resource_uri.as_str(),
                "external_system": binding.external_system,
                "materialization_mode": binding.materialization_mode.as_str(),
            }),
        );
        store
            .commit_source_binding_with_event(&binding, &event)
            .await
            .unwrap();

        store
            .with_conn(|connection| {
                let (payload, payload_format, account_key): (Vec<u8>, String, String) = connection
                    .query_row(
                        "SELECT record_payload, payload_format, external_account_key
                         FROM interoperability_source_bindings
                         WHERE binding_id = ?1",
                        params![binding.binding_id.to_string()],
                        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                    )
                    .map_err(|err| MvError::Storage(err.to_string()))?;
                assert_eq!(payload_format, "mvenc-v1");
                assert!(!bytes_contains(&payload, marker.as_bytes()));
                assert!(!bytes_contains(&payload, cursor.as_bytes()));
                assert_eq!(account_key, store.source_lookup_key(&marker).unwrap());
                Ok(())
            })
            .unwrap();
        assert_eq!(
            store.get_source_binding(binding.binding_id).await.unwrap(),
            Some(binding)
        );
    }

    #[tokio::test]
    async fn interoperable_node_create_is_atomic_and_idempotent() {
        let store = SqliteNodeStore::open_in_memory().unwrap();
        let local_node_id = store.local_context_node_id().await.unwrap();
        assert_eq!(store.local_context_node_id().await.unwrap(), local_node_id);

        let node =
            KnowledgeNode::new(NodeKind::Fact, "portable fact").with_namespace("interoperability");
        let event = node_created_event(
            local_node_id,
            node.id,
            "create-portable-fact",
            &"a".repeat(64),
        );
        let first = store
            .commit_node_create_with_event(&node, &event)
            .await
            .unwrap();
        assert!(!first.replayed);
        assert_eq!(first.node.id, node.id);

        let preflight = store
            .find_node_create_replay(
                &event.source,
                &event.principal,
                &event.idempotency_key,
                &event.payload_digest,
            )
            .await
            .unwrap()
            .unwrap();
        assert!(preflight.replayed);
        assert_eq!(preflight.node.id, first.node.id);
        assert_eq!(preflight.event.id, first.event.id);

        let retry_node =
            KnowledgeNode::new(NodeKind::Fact, "portable fact").with_namespace("interoperability");
        let retry_event = node_created_event(
            local_node_id,
            retry_node.id,
            "create-portable-fact",
            &"a".repeat(64),
        );
        let replay = store
            .commit_node_create_with_event(&retry_node, &retry_event)
            .await
            .unwrap();
        assert!(replay.replayed);
        assert_eq!(replay.node.id, first.node.id);
        assert_eq!(replay.event.id, first.event.id);

        let pending = store.list_pending_outbox_events(10).await.unwrap();
        assert_eq!(pending, vec![first.event.clone()]);
        assert_eq!(
            store.get_outbox_event(first.event.id).await.unwrap(),
            Some(first.event)
        );

        let conflicting_event = node_created_event(
            local_node_id,
            retry_node.id,
            "create-portable-fact",
            &"b".repeat(64),
        );
        assert!(matches!(
            store
                .commit_node_create_with_event(&retry_node, &conflicting_event)
                .await,
            Err(MvError::IdempotencyConflict(_))
        ));
    }

    #[tokio::test]
    async fn outbox_delivery_retries_then_publishes_with_immutable_receipts() {
        let store = SqliteNodeStore::open_in_memory().unwrap();
        let local_node_id = store.local_context_node_id().await.unwrap();
        let node = KnowledgeNode::new(NodeKind::Fact, "dispatch lifecycle")
            .with_namespace("interoperability");
        let event = node_created_event(
            local_node_id,
            node.id,
            "dispatch-lifecycle",
            &"d".repeat(64),
        );
        store
            .commit_node_create_with_event(&node, &event)
            .await
            .unwrap();

        let executor = StableUri::parse("mindvault://dispatchers/local").unwrap();
        let destination = StableUri::parse("mindvault://destinations/test").unwrap();
        let first_claimed_at = event.occurred_at + chrono::Duration::seconds(1);
        let first = store
            .claim_outbox_events(
                &executor,
                &destination,
                first_claimed_at,
                first_claimed_at + chrono::Duration::minutes(5),
                10,
            )
            .await
            .unwrap();
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].attempt, 1);
        assert!(store
            .claim_outbox_events(
                &executor,
                &destination,
                first_claimed_at,
                first_claimed_at + chrono::Duration::minutes(5),
                10,
            )
            .await
            .unwrap()
            .is_empty());

        let receiptless_completion = store.with_conn(|connection| {
            connection
                .execute(
                    "UPDATE interoperability_outbox
                     SET delivery_state = 'published', published_at = ?2,
                         lease_id = NULL, lease_owner_uri = NULL,
                         lease_destination_uri = NULL, lease_expires_at = NULL,
                         updated_at = ?2
                     WHERE event_id = ?1",
                    params![
                        event.id.to_string(),
                        (first_claimed_at + chrono::Duration::seconds(1)).to_rfc3339(),
                    ],
                )
                .map(|_| ())
                .map_err(|err| MvError::Storage(err.to_string()))
        });
        assert!(
            matches!(receiptless_completion, Err(MvError::Storage(message))
            if message.contains("invalid outbox delivery completion"))
        );

        let retry_at = first_claimed_at + chrono::Duration::seconds(30);
        let retry = OutboxDeliveryCompletion {
            event_id: event.id,
            lease_id: first[0].lease_id,
            attempt: first[0].attempt,
            completed_at: first_claimed_at + chrono::Duration::seconds(2),
            result: OutboxDeliveryResult::RetryScheduled {
                retry_at,
                error_code: "destination_unavailable".into(),
                error_summary: "temporary provider failure".into(),
            },
        };
        let retry_receipt = store
            .complete_outbox_delivery(&first[0], &retry)
            .await
            .unwrap();
        assert_eq!(retry_receipt.outcome, ActionReceiptOutcome::RetryScheduled);
        let retry_status = store
            .get_outbox_delivery_status(event.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(retry_status.state, OutboxDeliveryState::Pending);
        assert_eq!(retry_status.attempts, 1);
        assert_eq!(retry_status.next_attempt_at, retry_at);
        assert_eq!(
            retry_status.last_error_code.as_deref(),
            Some("destination_unavailable")
        );
        assert!(store
            .claim_outbox_events(
                &executor,
                &destination,
                retry_at - chrono::Duration::milliseconds(1),
                retry_at + chrono::Duration::minutes(5),
                10,
            )
            .await
            .unwrap()
            .is_empty());

        let second = store
            .claim_outbox_events(
                &executor,
                &destination,
                retry_at,
                retry_at + chrono::Duration::minutes(5),
                10,
            )
            .await
            .unwrap();
        assert_eq!(second.len(), 1);
        assert_eq!(second[0].attempt, 2);
        let published = OutboxDeliveryCompletion {
            event_id: event.id,
            lease_id: second[0].lease_id,
            attempt: second[0].attempt,
            completed_at: retry_at + chrono::Duration::seconds(1),
            result: OutboxDeliveryResult::Published {
                delivery_reference: "test-message-42".into(),
                response_digest: Some("e".repeat(64)),
            },
        };
        let published_receipt = store
            .complete_outbox_delivery(&second[0], &published)
            .await
            .unwrap();
        let replayed_receipt = store
            .complete_outbox_delivery(&second[0], &published)
            .await
            .unwrap();
        assert_eq!(replayed_receipt, published_receipt);

        let published_status = store
            .get_outbox_delivery_status(event.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(published_status.state, OutboxDeliveryState::Published);
        assert_eq!(published_status.attempts, 2);
        assert_eq!(
            store.list_action_receipts(event.id, 10).await.unwrap(),
            vec![retry_receipt, published_receipt.clone()]
        );
        assert_eq!(
            store
                .get_action_receipt(published_receipt.receipt_id)
                .await
                .unwrap(),
            Some(published_receipt.clone())
        );

        let immutable_update = store.with_conn(|connection| {
            connection
                .execute(
                    "UPDATE interoperability_action_receipts
                     SET outcome = 'dead_lettered'
                     WHERE receipt_id = ?1",
                    params![published_receipt.receipt_id.to_string()],
                )
                .map(|_| ())
                .map_err(|err| MvError::Storage(err.to_string()))
        });
        assert!(matches!(immutable_update, Err(MvError::Storage(message))
            if message.contains("immutable")));
    }

    #[tokio::test]
    async fn expired_outbox_claim_is_recoverable_and_stale_completion_is_rejected() {
        let store = SqliteNodeStore::open_in_memory().unwrap();
        let local_node_id = store.local_context_node_id().await.unwrap();
        let node = KnowledgeNode::new(NodeKind::Fact, "expired dispatch")
            .with_namespace("interoperability");
        let event = node_created_event(local_node_id, node.id, "expired-dispatch", &"f".repeat(64));
        store
            .commit_node_create_with_event(&node, &event)
            .await
            .unwrap();

        let executor = StableUri::parse("mindvault://dispatchers/local").unwrap();
        let destination = StableUri::parse("mindvault://destinations/test").unwrap();
        let first_claimed_at = event.occurred_at + chrono::Duration::seconds(1);
        let first = store
            .claim_outbox_events(
                &executor,
                &destination,
                first_claimed_at,
                first_claimed_at + chrono::Duration::seconds(10),
                1,
            )
            .await
            .unwrap()
            .remove(0);
        let second_claimed_at = first.lease_expires_at;
        let second = store
            .claim_outbox_events(
                &executor,
                &destination,
                second_claimed_at,
                second_claimed_at + chrono::Duration::minutes(1),
                1,
            )
            .await
            .unwrap()
            .remove(0);
        assert_eq!(second.attempt, 2);
        assert_ne!(second.lease_id, first.lease_id);

        let stale_completion = OutboxDeliveryCompletion {
            event_id: event.id,
            lease_id: first.lease_id,
            attempt: first.attempt,
            completed_at: first.claimed_at + chrono::Duration::seconds(1),
            result: OutboxDeliveryResult::Published {
                delivery_reference: "stale-message".into(),
                response_digest: None,
            },
        };
        assert!(matches!(
            store
                .complete_outbox_delivery(&first, &stale_completion)
                .await,
            Err(MvError::IdempotencyConflict(_))
        ));
        assert!(store
            .list_action_receipts(event.id, 10)
            .await
            .unwrap()
            .is_empty());

        let dead_letter = OutboxDeliveryCompletion {
            event_id: event.id,
            lease_id: second.lease_id,
            attempt: second.attempt,
            completed_at: second.claimed_at + chrono::Duration::seconds(1),
            result: OutboxDeliveryResult::DeadLettered {
                error_code: "invalid_destination".into(),
                error_summary: "destination is permanently invalid".into(),
            },
        };
        let receipt = store
            .complete_outbox_delivery(&second, &dead_letter)
            .await
            .unwrap();
        assert_eq!(receipt.outcome, ActionReceiptOutcome::DeadLettered);
        assert_eq!(
            store
                .get_outbox_delivery_status(event.id)
                .await
                .unwrap()
                .unwrap()
                .state,
            OutboxDeliveryState::DeadLetter
        );
        assert!(store
            .claim_outbox_events(
                &executor,
                &destination,
                dead_letter.completed_at + chrono::Duration::seconds(1),
                dead_letter.completed_at + chrono::Duration::minutes(1),
                1,
            )
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn sealed_action_receipt_payload_hides_provider_details() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("sealed-action-receipt.sqlite");
        let _reset = install_scoped_runtime_key(&db_path, [37u8; 32]);
        let store = SqliteNodeStore::open_with_mode(&db_path, true).unwrap();
        let local_node_id = store.local_context_node_id().await.unwrap();
        let node = KnowledgeNode::new(NodeKind::Fact, "sealed dispatch")
            .with_namespace("interoperability");
        let event = node_created_event(local_node_id, node.id, "sealed-dispatch", &"1".repeat(64));
        store
            .commit_node_create_with_event(&node, &event)
            .await
            .unwrap();

        let claimed_at = event.occurred_at + chrono::Duration::seconds(1);
        let claim = store
            .claim_outbox_events(
                &StableUri::parse("mindvault://dispatchers/local").unwrap(),
                &StableUri::parse("mindvault://destinations/test").unwrap(),
                claimed_at,
                claimed_at + chrono::Duration::minutes(1),
                1,
            )
            .await
            .unwrap()
            .remove(0);
        let secret_summary = "provider-token-redacted-marker";
        let completion = OutboxDeliveryCompletion {
            event_id: event.id,
            lease_id: claim.lease_id,
            attempt: claim.attempt,
            completed_at: claimed_at + chrono::Duration::seconds(1),
            result: OutboxDeliveryResult::DeadLettered {
                error_code: "provider_rejected".into(),
                error_summary: secret_summary.into(),
            },
        };
        let receipt = store
            .complete_outbox_delivery(&claim, &completion)
            .await
            .unwrap();

        let (payload, format, wrapped): (Vec<u8>, String, Option<String>) = store
            .with_conn(|connection| {
                connection
                    .query_row(
                        "SELECT payload, payload_format, payload_wrapped_dek
                         FROM interoperability_action_receipts
                         WHERE receipt_id = ?1",
                        params![receipt.receipt_id.to_string()],
                        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                    )
                    .map_err(|err| MvError::Storage(err.to_string()))
            })
            .unwrap();
        assert_eq!(format, "mvenc-v1");
        assert!(wrapped.is_some());
        assert!(!bytes_contains(&payload, secret_summary.as_bytes()));
        assert_eq!(
            store.get_action_receipt(receipt.receipt_id).await.unwrap(),
            Some(receipt)
        );
    }

    #[tokio::test]
    async fn consumer_inbox_retries_then_applies_with_checkpointed_receipts() {
        let store = SqliteNodeStore::open_in_memory().unwrap();
        let local_node_id = store.local_context_node_id().await.unwrap();
        let event = node_created_event(
            local_node_id,
            Uuid::now_v7(),
            "consumer-inbox-retry",
            &"2".repeat(64),
        );
        let consumer = StableUri::parse("mindvault://consumers/local-index").unwrap();
        let processor = StableUri::parse("mindvault://processors/local-index").unwrap();
        let received_at = event.occurred_at + chrono::Duration::seconds(1);

        let admission = store
            .admit_consumer_event(&consumer, &event, received_at)
            .await
            .unwrap();
        assert!(!admission.replayed);
        let replay = store
            .admit_consumer_event(
                &consumer,
                &event,
                received_at + chrono::Duration::seconds(1),
            )
            .await
            .unwrap();
        assert!(replay.replayed);
        assert_eq!(replay.inbox_sequence, admission.inbox_sequence);
        assert_eq!(replay.received_at, received_at);

        let first = store
            .claim_consumer_events(
                &consumer,
                &processor,
                received_at,
                received_at + chrono::Duration::minutes(1),
                10,
            )
            .await
            .unwrap()
            .remove(0);
        assert_eq!(first.attempt, 1);

        let receiptless_completion = store.with_conn(|connection| {
            connection
                .execute(
                    "UPDATE interoperability_consumer_inbox
                     SET state = 'applied', applied_at = ?2,
                         lease_id = NULL, lease_processor_uri = NULL,
                         lease_expires_at = NULL, updated_at = ?2
                     WHERE inbox_sequence = ?1",
                    params![
                        first.inbox_sequence,
                        (first.claimed_at + chrono::Duration::seconds(1)).to_rfc3339(),
                    ],
                )
                .map(|_| ())
                .map_err(|err| MvError::Storage(err.to_string()))
        });
        assert!(
            matches!(receiptless_completion, Err(MvError::Storage(message))
            if message.contains("invalid consumer inbox completion"))
        );

        let retry_at = received_at + chrono::Duration::seconds(20);
        let retry = ConsumerApplicationCompletion {
            inbox_sequence: first.inbox_sequence,
            event_id: event.id,
            lease_id: first.lease_id,
            attempt: first.attempt,
            completed_at: received_at + chrono::Duration::seconds(2),
            result: ConsumerApplicationResult::RetryScheduled {
                retry_at,
                error_code: "index_unavailable".into(),
                error_summary: "local index is temporarily unavailable".into(),
            },
        };
        let retry_receipt = store.complete_consumer_event(&first, &retry).await.unwrap();
        assert_eq!(
            retry_receipt.outcome,
            ConsumerApplicationOutcome::RetryScheduled
        );
        assert_eq!(retry_receipt.claim_id, first.lease_id);
        assert!(store
            .get_consumer_checkpoint(&consumer, &event.source)
            .await
            .unwrap()
            .is_none());
        assert!(store
            .claim_consumer_events(
                &consumer,
                &processor,
                retry_at - chrono::Duration::milliseconds(1),
                retry_at + chrono::Duration::minutes(1),
                10,
            )
            .await
            .unwrap()
            .is_empty());

        let second = store
            .claim_consumer_events(
                &consumer,
                &processor,
                retry_at,
                retry_at + chrono::Duration::minutes(1),
                10,
            )
            .await
            .unwrap()
            .remove(0);
        assert_eq!(second.attempt, 2);
        assert_ne!(second.lease_id, first.lease_id);

        let applied = ConsumerApplicationCompletion {
            inbox_sequence: second.inbox_sequence,
            event_id: event.id,
            lease_id: second.lease_id,
            attempt: second.attempt,
            completed_at: retry_at + chrono::Duration::seconds(1),
            result: ConsumerApplicationResult::Applied {
                application_reference: "local-index-entry-42".into(),
                effect_digest: Some("3".repeat(64)),
            },
        };
        let applied_receipt = store
            .complete_consumer_event(&second, &applied)
            .await
            .unwrap();
        let replayed_receipt = store
            .complete_consumer_event(&second, &applied)
            .await
            .unwrap();
        assert_eq!(replayed_receipt, applied_receipt);
        assert_eq!(applied_receipt.claim_id, second.lease_id);

        let status = store
            .get_consumer_inbox_status(&consumer, event.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(status.state, ConsumerInboxState::Applied);
        assert_eq!(status.attempts, 2);
        assert_eq!(status.applied_at, Some(applied.completed_at));
        let checkpoint = store
            .get_consumer_checkpoint(&consumer, &event.source)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            checkpoint.last_dispositioned_sequence,
            admission.inbox_sequence
        );
        assert_eq!(checkpoint.last_dispositioned_event_id, event.id);
        assert_eq!(
            checkpoint.last_applied_sequence,
            Some(admission.inbox_sequence)
        );
        assert_eq!(checkpoint.last_applied_event_id, Some(event.id));
        assert_eq!(checkpoint.applied_count, 1);
        assert_eq!(checkpoint.dead_letter_count, 0);
        assert_eq!(
            store
                .list_consumer_application_receipts(&consumer, event.id, 10)
                .await
                .unwrap(),
            vec![retry_receipt, applied_receipt.clone()]
        );
        assert_eq!(
            store
                .get_consumer_application_receipt(applied_receipt.receipt_id)
                .await
                .unwrap(),
            Some(applied_receipt.clone())
        );

        let immutable_update = store.with_conn(|connection| {
            connection
                .execute(
                    "UPDATE interoperability_consumer_application_receipts
                     SET outcome = 'dead_lettered'
                     WHERE receipt_id = ?1",
                    params![applied_receipt.receipt_id.to_string()],
                )
                .map(|_| ())
                .map_err(|err| MvError::Storage(err.to_string()))
        });
        assert!(matches!(immutable_update, Err(MvError::Storage(message))
            if message.contains("immutable")));
    }

    #[tokio::test]
    async fn consumer_inbox_orders_each_source_and_checkpoints_dead_letters() {
        let store = SqliteNodeStore::open_in_memory().unwrap();
        let local_node_id = store.local_context_node_id().await.unwrap();
        let first_event = node_created_event(
            local_node_id,
            Uuid::now_v7(),
            "consumer-order-first",
            &"4".repeat(64),
        );
        let second_event = node_created_event(
            local_node_id,
            Uuid::now_v7(),
            "consumer-order-second",
            &"5".repeat(64),
        );
        let consumer = StableUri::parse("mindvault://consumers/ordered-index").unwrap();
        let processor = StableUri::parse("mindvault://processors/ordered-index").unwrap();
        let received_at = first_event.occurred_at + chrono::Duration::seconds(1);
        let first_admission = store
            .admit_consumer_event(&consumer, &first_event, received_at)
            .await
            .unwrap();
        let second_admission = store
            .admit_consumer_event(
                &consumer,
                &second_event,
                received_at + chrono::Duration::milliseconds(1),
            )
            .await
            .unwrap();

        let mut conflicting_event = first_event.clone();
        conflicting_event.data["namespace"] = serde_json::json!("conflicting-replay");
        assert!(matches!(
            store
                .admit_consumer_event(
                    &consumer,
                    &conflicting_event,
                    received_at + chrono::Duration::seconds(1),
                )
                .await,
            Err(MvError::IdempotencyConflict(_))
        ));

        let first_claims = store
            .claim_consumer_events(
                &consumer,
                &processor,
                received_at + chrono::Duration::seconds(2),
                received_at + chrono::Duration::minutes(1),
                10,
            )
            .await
            .unwrap();
        assert_eq!(first_claims.len(), 1);
        assert_eq!(
            first_claims[0].inbox_sequence,
            first_admission.inbox_sequence
        );
        assert!(store
            .claim_consumer_events(
                &consumer,
                &processor,
                received_at + chrono::Duration::seconds(3),
                received_at + chrono::Duration::minutes(1),
                10,
            )
            .await
            .unwrap()
            .is_empty());

        let first_claim = &first_claims[0];
        let first_completion = ConsumerApplicationCompletion {
            inbox_sequence: first_claim.inbox_sequence,
            event_id: first_claim.event.id,
            lease_id: first_claim.lease_id,
            attempt: first_claim.attempt,
            completed_at: first_claim.claimed_at + chrono::Duration::seconds(1),
            result: ConsumerApplicationResult::Applied {
                application_reference: "ordered-index-entry-1".into(),
                effect_digest: Some("6".repeat(64)),
            },
        };
        store
            .complete_consumer_event(first_claim, &first_completion)
            .await
            .unwrap();

        let second_claim = store
            .claim_consumer_events(
                &consumer,
                &processor,
                first_completion.completed_at + chrono::Duration::seconds(1),
                first_completion.completed_at + chrono::Duration::minutes(1),
                10,
            )
            .await
            .unwrap()
            .remove(0);
        assert_eq!(second_claim.inbox_sequence, second_admission.inbox_sequence);
        let second_completion = ConsumerApplicationCompletion {
            inbox_sequence: second_claim.inbox_sequence,
            event_id: second_claim.event.id,
            lease_id: second_claim.lease_id,
            attempt: second_claim.attempt,
            completed_at: second_claim.claimed_at + chrono::Duration::seconds(1),
            result: ConsumerApplicationResult::DeadLettered {
                error_code: "unsupported_projection".into(),
                error_summary: "the consumer cannot project this event".into(),
            },
        };
        store
            .complete_consumer_event(&second_claim, &second_completion)
            .await
            .unwrap();

        let checkpoint = store
            .get_consumer_checkpoint(&consumer, &first_event.source)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            checkpoint.last_dispositioned_sequence,
            second_admission.inbox_sequence
        );
        assert_eq!(
            checkpoint.last_dispositioned_event_id,
            second_admission.event.id
        );
        assert_eq!(
            checkpoint.last_applied_sequence,
            Some(first_admission.inbox_sequence)
        );
        assert_eq!(
            checkpoint.last_applied_event_id,
            Some(first_admission.event.id)
        );
        assert_eq!(checkpoint.applied_count, 1);
        assert_eq!(checkpoint.dead_letter_count, 1);
        assert_eq!(
            store
                .get_consumer_inbox_status(&consumer, second_event.id)
                .await
                .unwrap()
                .unwrap()
                .state,
            ConsumerInboxState::DeadLetter
        );
    }

    #[tokio::test]
    async fn expired_consumer_claim_is_recoverable_and_stale_completion_is_rejected() {
        let store = SqliteNodeStore::open_in_memory().unwrap();
        let local_node_id = store.local_context_node_id().await.unwrap();
        let event = node_created_event(
            local_node_id,
            Uuid::now_v7(),
            "consumer-expired-claim",
            &"7".repeat(64),
        );
        let consumer = StableUri::parse("mindvault://consumers/recoverable-index").unwrap();
        let processor = StableUri::parse("mindvault://processors/recoverable-index").unwrap();
        let received_at = event.occurred_at + chrono::Duration::seconds(1);
        store
            .admit_consumer_event(&consumer, &event, received_at)
            .await
            .unwrap();

        let first = store
            .claim_consumer_events(
                &consumer,
                &processor,
                received_at,
                received_at + chrono::Duration::minutes(1),
                1,
            )
            .await
            .unwrap()
            .remove(0);
        let second = store
            .claim_consumer_events(
                &consumer,
                &processor,
                first.lease_expires_at,
                first.lease_expires_at + chrono::Duration::minutes(1),
                1,
            )
            .await
            .unwrap()
            .remove(0);
        assert_eq!(second.attempt, 2);
        assert_ne!(second.lease_id, first.lease_id);

        let stale_completion = ConsumerApplicationCompletion {
            inbox_sequence: first.inbox_sequence,
            event_id: event.id,
            lease_id: first.lease_id,
            attempt: first.attempt,
            completed_at: first.claimed_at + chrono::Duration::seconds(30),
            result: ConsumerApplicationResult::Applied {
                application_reference: "stale-index-entry".into(),
                effect_digest: None,
            },
        };
        assert!(matches!(
            store
                .complete_consumer_event(&first, &stale_completion)
                .await,
            Err(MvError::IdempotencyConflict(_))
        ));
        assert!(store
            .list_consumer_application_receipts(&consumer, event.id, 10)
            .await
            .unwrap()
            .is_empty());

        let recovered_completion = ConsumerApplicationCompletion {
            inbox_sequence: second.inbox_sequence,
            event_id: event.id,
            lease_id: second.lease_id,
            attempt: second.attempt,
            completed_at: second.claimed_at + chrono::Duration::seconds(1),
            result: ConsumerApplicationResult::DeadLettered {
                error_code: "application_rejected".into(),
                error_summary: "the recovered attempt rejected the event".into(),
            },
        };
        let receipt = store
            .complete_consumer_event(&second, &recovered_completion)
            .await
            .unwrap();
        assert_eq!(receipt.outcome, ConsumerApplicationOutcome::DeadLettered);
        assert_eq!(
            store
                .get_consumer_checkpoint(&consumer, &event.source)
                .await
                .unwrap()
                .unwrap()
                .dead_letter_count,
            1
        );
    }

    #[tokio::test]
    async fn sealed_consumer_records_hide_event_and_application_details() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("sealed-consumer-inbox.sqlite");
        let _reset = install_scoped_runtime_key(&db_path, [41u8; 32]);
        let store = SqliteNodeStore::open_with_mode(&db_path, true).unwrap();
        let local_node_id = store.local_context_node_id().await.unwrap();
        let event_marker = "consumer-event-sensitive-marker";
        let error_marker = "consumer-application-sensitive-marker";
        let mut event = node_created_event(
            local_node_id,
            Uuid::now_v7(),
            "sealed-consumer-event",
            &"8".repeat(64),
        );
        event.data["sensitive_marker"] = serde_json::json!(event_marker);
        let consumer = StableUri::parse("mindvault://consumers/sealed-index").unwrap();
        let processor = StableUri::parse("mindvault://processors/sealed-index").unwrap();
        let received_at = event.occurred_at + chrono::Duration::seconds(1);
        let admission = store
            .admit_consumer_event(&consumer, &event, received_at)
            .await
            .unwrap();

        let (event_payload, event_format, event_wrapped): (Vec<u8>, String, Option<String>) = store
            .with_conn(|connection| {
                connection
                    .query_row(
                        "SELECT envelope_payload, payload_format, payload_wrapped_dek
                         FROM interoperability_consumer_inbox
                         WHERE inbox_sequence = ?1",
                        params![admission.inbox_sequence],
                        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                    )
                    .map_err(|err| MvError::Storage(err.to_string()))
            })
            .unwrap();
        assert_eq!(event_format, "mvenc-v1");
        assert!(event_wrapped.is_some());
        assert!(!bytes_contains(&event_payload, event_marker.as_bytes()));

        let claim = store
            .claim_consumer_events(
                &consumer,
                &processor,
                received_at + chrono::Duration::seconds(1),
                received_at + chrono::Duration::minutes(1),
                1,
            )
            .await
            .unwrap()
            .remove(0);
        let completion = ConsumerApplicationCompletion {
            inbox_sequence: claim.inbox_sequence,
            event_id: event.id,
            lease_id: claim.lease_id,
            attempt: claim.attempt,
            completed_at: claim.claimed_at + chrono::Duration::seconds(1),
            result: ConsumerApplicationResult::DeadLettered {
                error_code: "sealed_rejection".into(),
                error_summary: error_marker.into(),
            },
        };
        let receipt = store
            .complete_consumer_event(&claim, &completion)
            .await
            .unwrap();

        let (receipt_payload, receipt_format, receipt_wrapped): (Vec<u8>, String, Option<String>) =
            store
                .with_conn(|connection| {
                    connection
                        .query_row(
                            "SELECT payload, payload_format, payload_wrapped_dek
                         FROM interoperability_consumer_application_receipts
                         WHERE receipt_id = ?1",
                            params![receipt.receipt_id.to_string()],
                            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                        )
                        .map_err(|err| MvError::Storage(err.to_string()))
                })
                .unwrap();
        assert_eq!(receipt_format, "mvenc-v1");
        assert!(receipt_wrapped.is_some());
        assert!(!bytes_contains(&receipt_payload, error_marker.as_bytes()));
        assert_eq!(
            store
                .get_consumer_application_receipt(receipt.receipt_id)
                .await
                .unwrap(),
            Some(receipt)
        );
    }

    #[tokio::test]
    async fn failed_node_insert_does_not_leave_an_outbox_event() {
        let store = SqliteNodeStore::open_in_memory().unwrap();
        let local_node_id = store.local_context_node_id().await.unwrap();
        let node =
            KnowledgeNode::new(NodeKind::Fact, "duplicate").with_namespace("interoperability");
        store.insert(&node).await.unwrap();
        let event = node_created_event(local_node_id, node.id, "duplicate-node", &"c".repeat(64));

        assert!(store
            .commit_node_create_with_event(&node, &event)
            .await
            .is_err());
        assert!(store.get_outbox_event(event.id).await.unwrap().is_none());
    }

    // -----------------------------------------------------------------------
    // Governed agent execution graph
    // -----------------------------------------------------------------------

    fn work_order_fixture(local_node_id: Uuid, budget: WorkOrderBudget) -> WorkOrder {
        let work_order_id = Uuid::now_v7();
        let principal = StableUri::principal(
            local_node_id,
            Uuid::new_v5(&local_node_id, b"governance-test-principal"),
        );
        let now = Utc::now();
        WorkOrder {
            work_order_id,
            revision: 1,
            work_order_uri: StableUri::work_order(local_node_id, work_order_id),
            principal: principal.clone(),
            actor: principal,
            governing_node: StableUri::node(local_node_id),
            goal: "summarize the meeting into candidate decisions".into(),
            non_goals: vec!["do not contact external services".into()],
            anchors: Vec::new(),
            success_criteria: vec!["candidate decisions exist with provenance".into()],
            prohibited_outcomes: Vec::new(),
            budget,
            remaining: budget,
            sensitivity: Sensitivity::Internal,
            retention: RetentionClass::Operational,
            correlation_id: Uuid::now_v7(),
            causation_id: None,
            idempotency_key: format!("wo-{work_order_id}"),
            status: WorkOrderStatus::Draft,
            status_reason: None,
            created_at: now,
            updated_at: now,
        }
    }

    fn node_fixture(
        local_node_id: Uuid,
        work_order_id: Uuid,
        writes: &[&str],
        tier: RiskTier,
    ) -> WorkOrderNode {
        let node_id = Uuid::now_v7();
        let now = Utc::now();
        WorkOrderNode {
            node_id,
            work_order_id,
            node_uri: StableUri::work_order_node(local_node_id, work_order_id, node_id),
            purpose: "extract candidate decisions".into(),
            executor_kind: ExecutorKind::Engine,
            risk_tier: tier,
            status: WorkOrderNodeStatus::Pending,
            read_scope: Vec::new(),
            write_scope: writes
                .iter()
                .map(|value| StableUri::parse(*value).unwrap())
                .collect(),
            inputs: Vec::new(),
            timeout_secs: 600,
            max_attempts: 3,
            authorizing_grant_id: None,
            created_at: now,
            updated_at: now,
        }
    }

    fn run_fixture(
        local_node_id: Uuid,
        work_order: &WorkOrder,
        node: &WorkOrderNode,
        actor_seed: &str,
    ) -> AgentRun {
        let run_id = Uuid::now_v7();
        let now = Utc::now();
        AgentRun {
            run_id,
            run_uri: StableUri::agent_run(local_node_id, run_id),
            work_order_id: work_order.work_order_id,
            node_id: node.node_id,
            attempt_no: 1,
            status: AgentRunStatus::Ready,
            failure_class: None,
            principal: work_order.principal.clone(),
            actor: StableUri::principal(
                local_node_id,
                Uuid::new_v5(&local_node_id, actor_seed.as_bytes()),
            ),
            correlation_id: work_order.correlation_id,
            causation_id: None,
            started_at: None,
            ended_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    fn work_order_admitted_event(
        local_node_id: Uuid,
        work_order: &WorkOrder,
        nodes: usize,
        edges: usize,
        key: &str,
    ) -> EventEnvelope {
        let data = serde_json::json!({
            "work_order_id": work_order.work_order_id,
            "node_count": nodes,
            "edge_count": edges,
            "governing_node_uri": work_order.governing_node.as_str(),
            "record_digest": "a".repeat(64),
        });
        let mut event = governance_event(
            local_node_id,
            WORK_ORDER_ADMITTED_V1,
            "work-order-admitted",
            work_order.work_order_uri.clone(),
            key,
            data,
        );
        event.principal = work_order.principal.clone();
        event.actor = work_order.actor.clone();
        event
    }

    fn agent_run_started_event(local_node_id: Uuid, run: &AgentRun, key: &str) -> EventEnvelope {
        let data = serde_json::json!({
            "run_id": run.run_id,
            "work_order_id": run.work_order_id,
            "node_id": run.node_id,
            "attempt_no": run.attempt_no,
            "record_digest": "b".repeat(64),
        });
        let mut event = governance_event(
            local_node_id,
            AGENT_RUN_STARTED_V1,
            "agent-run-started",
            run.run_uri.clone(),
            key,
            data,
        );
        event.principal = run.principal.clone();
        event.actor = run.principal.clone();
        event
    }

    fn agent_run_transition_event(
        local_node_id: Uuid,
        run: &AgentRun,
        from: AgentRunStatus,
        key: &str,
    ) -> EventEnvelope {
        let data = serde_json::json!({
            "run_id": run.run_id,
            "from_status": from.as_str(),
            "to_status": run.status.as_str(),
            "record_digest": "c".repeat(64),
        });
        let mut event = governance_event(
            local_node_id,
            AGENT_RUN_LIFECYCLE_TRANSITIONED_V1,
            "agent-run-lifecycle-transitioned",
            run.run_uri.clone(),
            key,
            data,
        );
        event.principal = run.principal.clone();
        event.actor = run.principal.clone();
        event
    }

    /// Admit a one-contract work order and start one run against it.
    async fn admitted_run(
        store: &SqliteNodeStore,
        local_node_id: Uuid,
        writes: &[&str],
        tier: RiskTier,
        budget: WorkOrderBudget,
        seed: &str,
    ) -> (WorkOrder, WorkOrderNode, AgentRun) {
        let work_order = work_order_fixture(local_node_id, budget);
        let node = node_fixture(local_node_id, work_order.work_order_id, writes, tier);
        let event =
            work_order_admitted_event(local_node_id, &work_order, 1, 0, &format!("admit-{seed}"));
        store
            .commit_work_order_with_event(&work_order, std::slice::from_ref(&node), &[], &event)
            .await
            .unwrap();

        let run = run_fixture(local_node_id, &work_order, &node, seed);
        let start = agent_run_started_event(local_node_id, &run, &format!("start-{seed}"));
        store
            .commit_agent_run_with_event(&run, &WorkOrderSpend::one_run_attempt(), &start)
            .await
            .unwrap();
        (work_order, node, run)
    }

    #[tokio::test]
    async fn work_order_admission_round_trips_with_contracts_and_edges() {
        let store = SqliteNodeStore::open_in_memory().unwrap();
        let local_node_id = store.local_context_node_id().await.unwrap();
        register_local_context_node(&store).await;

        let budget = WorkOrderBudget {
            wall_clock_secs: 3600,
            run_attempts: 5,
            model_tokens: 100_000,
            effect_actions: 10,
        };
        let work_order = work_order_fixture(local_node_id, budget);
        let first = node_fixture(
            local_node_id,
            work_order.work_order_id,
            &["mindvault://schemas/alpha"],
            RiskTier::Standard,
        );
        let second = node_fixture(
            local_node_id,
            work_order.work_order_id,
            &["mindvault://schemas/beta"],
            RiskTier::Low,
        );
        let edge = WorkOrderEdge {
            edge_id: Uuid::now_v7(),
            work_order_id: work_order.work_order_id,
            from_node_id: first.node_id,
            to_node_id: second.node_id,
            kind: EdgeKind::Data,
            derived: false,
            detail: None,
            created_at: work_order.created_at,
        };
        let event = work_order_admitted_event(local_node_id, &work_order, 2, 1, "admit-round-trip");

        let commit = store
            .commit_work_order_with_event(
                &work_order,
                &[first.clone(), second.clone()],
                std::slice::from_ref(&edge),
                &event,
            )
            .await
            .unwrap();
        assert!(!commit.replayed);

        let stored = store
            .get_work_order(work_order.work_order_id)
            .await
            .unwrap()
            .expect("work order");
        assert_eq!(stored.goal, work_order.goal);
        assert_eq!(stored.non_goals, work_order.non_goals);

        let contracts = store
            .list_work_order_nodes(work_order.work_order_id)
            .await
            .unwrap();
        assert_eq!(contracts.len(), 2);
        let edges = store
            .list_work_order_edges(work_order.work_order_id)
            .await
            .unwrap();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].kind, EdgeKind::Data);

        // The admission event is durable in the same transaction (law 4).
        assert!(store.get_outbox_event(event.id).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn admission_rejects_intersecting_write_scopes_without_a_conflict_edge() {
        let store = SqliteNodeStore::open_in_memory().unwrap();
        let local_node_id = store.local_context_node_id().await.unwrap();
        register_local_context_node(&store).await;

        let budget = WorkOrderBudget {
            wall_clock_secs: 60,
            run_attempts: 2,
            model_tokens: 100,
            effect_actions: 1,
        };
        let work_order = work_order_fixture(local_node_id, budget);
        let first = node_fixture(
            local_node_id,
            work_order.work_order_id,
            &["mindvault://schemas/shared"],
            RiskTier::Low,
        );
        let second = node_fixture(
            local_node_id,
            work_order.work_order_id,
            &["mindvault://schemas/shared"],
            RiskTier::Low,
        );
        let nodes = vec![first, second];
        let event = work_order_admitted_event(local_node_id, &work_order, 2, 0, "admit-conflict");

        // Omitting the derived edge must fail closed rather than admit an
        // unguarded overlap.
        let err = store
            .commit_work_order_with_event(&work_order, &nodes, &[], &event)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("conflict edge"), "got: {err}");

        // Supplying it succeeds.
        let derived =
            derive_conflict_edges(work_order.work_order_id, &nodes, work_order.created_at);
        assert_eq!(derived.len(), 1);
        store
            .commit_work_order_with_event(&work_order, &nodes, &derived, &event)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn admission_rejects_a_dependency_cycle() {
        let store = SqliteNodeStore::open_in_memory().unwrap();
        let local_node_id = store.local_context_node_id().await.unwrap();
        register_local_context_node(&store).await;

        let budget = WorkOrderBudget {
            wall_clock_secs: 60,
            run_attempts: 2,
            model_tokens: 100,
            effect_actions: 1,
        };
        let work_order = work_order_fixture(local_node_id, budget);
        let first = node_fixture(
            local_node_id,
            work_order.work_order_id,
            &["mindvault://schemas/alpha"],
            RiskTier::Low,
        );
        let second = node_fixture(
            local_node_id,
            work_order.work_order_id,
            &["mindvault://schemas/beta"],
            RiskTier::Low,
        );
        let edges = vec![
            WorkOrderEdge {
                edge_id: Uuid::now_v7(),
                work_order_id: work_order.work_order_id,
                from_node_id: first.node_id,
                to_node_id: second.node_id,
                kind: EdgeKind::Data,
                derived: false,
                detail: None,
                created_at: work_order.created_at,
            },
            WorkOrderEdge {
                edge_id: Uuid::now_v7(),
                work_order_id: work_order.work_order_id,
                from_node_id: second.node_id,
                to_node_id: first.node_id,
                kind: EdgeKind::Data,
                derived: false,
                detail: None,
                created_at: work_order.created_at,
            },
        ];
        let event = work_order_admitted_event(local_node_id, &work_order, 2, 2, "admit-cycle");

        let err = store
            .commit_work_order_with_event(&work_order, &[first, second], &edges, &event)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("cycle"), "got: {err}");
    }

    #[tokio::test]
    async fn a_second_run_cannot_claim_a_held_write_target() {
        let store = SqliteNodeStore::open_in_memory().unwrap();
        let local_node_id = store.local_context_node_id().await.unwrap();
        register_local_context_node(&store).await;

        let budget = WorkOrderBudget {
            wall_clock_secs: 3600,
            run_attempts: 5,
            model_tokens: 1000,
            effect_actions: 5,
        };
        let target = "mindvault://schemas/contested";
        let digest = target_digest(&StableUri::parse(target).unwrap());

        let (_, _, first_run) = admitted_run(
            &store,
            local_node_id,
            &[target],
            RiskTier::Low,
            budget,
            "first",
        )
        .await;
        let (_, _, second_run) = admitted_run(
            &store,
            local_node_id,
            &[target],
            RiskTier::Low,
            budget,
            "second",
        )
        .await;

        let now = Utc::now();
        let expires = now + chrono::Duration::minutes(30);
        let held = store
            .claim_write_leases(
                first_run.run_id,
                std::slice::from_ref(&digest),
                now,
                expires,
            )
            .await
            .unwrap();
        assert_eq!(held.len(), 1);

        let blocked = store
            .claim_write_leases(
                second_run.run_id,
                std::slice::from_ref(&digest),
                now,
                expires,
            )
            .await;
        assert!(blocked.is_err(), "an unexpired lease must block the claim");

        let conflicts = store
            .conflicting_write_targets(second_run.run_id, std::slice::from_ref(&digest), now)
            .await
            .unwrap();
        assert_eq!(conflicts, vec![digest.clone()]);

        // The holder does not conflict with itself.
        assert!(store
            .conflicting_write_targets(first_run.run_id, &[digest], now)
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn a_run_awaiting_approval_releases_its_leases_and_unblocks_others() {
        let store = SqliteNodeStore::open_in_memory().unwrap();
        let local_node_id = store.local_context_node_id().await.unwrap();
        register_local_context_node(&store).await;

        let budget = WorkOrderBudget {
            wall_clock_secs: 3600,
            run_attempts: 5,
            model_tokens: 1000,
            effect_actions: 5,
        };
        let target = "mindvault://schemas/approval-target";
        let digest = target_digest(&StableUri::parse(target).unwrap());

        let (_, _, mut run) = admitted_run(
            &store,
            local_node_id,
            &[target],
            RiskTier::High,
            budget,
            "parked",
        )
        .await;
        let (_, _, other) = admitted_run(
            &store,
            local_node_id,
            &[target],
            RiskTier::Low,
            budget,
            "waiting",
        )
        .await;

        let now = Utc::now();
        let expires = now + chrono::Duration::minutes(30);
        store
            .claim_write_leases(run.run_id, std::slice::from_ref(&digest), now, expires)
            .await
            .unwrap();

        // ready -> leased -> running -> awaiting_approval
        for (next, key) in [
            (AgentRunStatus::Leased, "to-leased"),
            (AgentRunStatus::Running, "to-running"),
            (AgentRunStatus::AwaitingApproval, "to-approval"),
        ] {
            let from = run.status;
            run.status = next;
            run.updated_at = Utc::now();
            if next == AgentRunStatus::Running {
                run.started_at = Some(run.updated_at);
            }
            let event = agent_run_transition_event(local_node_id, &run, from, key);
            store
                .transition_agent_run_with_event(&run, None, &event)
                .await
                .unwrap();
        }

        // Approval is unbounded, so the parked run must hold nothing.
        let leases = store.list_write_leases(run.run_id).await.unwrap();
        assert!(
            leases.iter().all(|lease| !lease.is_active()),
            "a run awaiting approval must hold no write leases"
        );
        assert_eq!(
            leases[0].release_reason,
            Some(LeaseReleaseReason::AwaitingApproval)
        );

        // The other run can now proceed rather than waiting on a human.
        let claimed = store
            .claim_write_leases(
                other.run_id,
                &[digest],
                Utc::now(),
                Utc::now() + chrono::Duration::minutes(30),
            )
            .await
            .unwrap();
        assert_eq!(claimed.len(), 1);
        assert_eq!(claimed[0].attempt_no, 2, "the claim counter advances");
    }

    #[tokio::test]
    async fn an_expired_lease_is_replaced_and_its_history_is_kept() {
        let store = SqliteNodeStore::open_in_memory().unwrap();
        let local_node_id = store.local_context_node_id().await.unwrap();
        register_local_context_node(&store).await;

        let budget = WorkOrderBudget {
            wall_clock_secs: 3600,
            run_attempts: 5,
            model_tokens: 1000,
            effect_actions: 5,
        };
        let target = "mindvault://schemas/abandoned";
        let digest = target_digest(&StableUri::parse(target).unwrap());

        let (_, _, crashed) = admitted_run(
            &store,
            local_node_id,
            &[target],
            RiskTier::Low,
            budget,
            "crashed",
        )
        .await;
        let (_, _, successor) = admitted_run(
            &store,
            local_node_id,
            &[target],
            RiskTier::Low,
            budget,
            "successor",
        )
        .await;

        // A lease that already expired: the crashed run never released it.
        let claimed_at = Utc::now() - chrono::Duration::minutes(90);
        store
            .claim_write_leases(
                crashed.run_id,
                std::slice::from_ref(&digest),
                claimed_at,
                claimed_at + chrono::Duration::minutes(30),
            )
            .await
            .unwrap();

        // Without replacement the target would be locked forever.
        let now = Utc::now();
        let replacement = store
            .claim_write_leases(
                successor.run_id,
                std::slice::from_ref(&digest),
                now,
                now + chrono::Duration::minutes(30),
            )
            .await
            .unwrap();
        assert_eq!(replacement[0].attempt_no, 2);

        let history = store.list_write_leases(crashed.run_id).await.unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(
            history[0].release_reason,
            Some(LeaseReleaseReason::ExpiredReplaced),
            "the reaped lease is retained as history, not deleted"
        );
    }

    #[tokio::test]
    async fn a_lease_longer_than_one_hour_is_refused() {
        let store = SqliteNodeStore::open_in_memory().unwrap();
        let local_node_id = store.local_context_node_id().await.unwrap();
        register_local_context_node(&store).await;

        let budget = WorkOrderBudget {
            wall_clock_secs: 3600,
            run_attempts: 5,
            model_tokens: 1000,
            effect_actions: 5,
        };
        let target = "mindvault://schemas/bounded";
        let digest = target_digest(&StableUri::parse(target).unwrap());
        let (_, _, run) = admitted_run(
            &store,
            local_node_id,
            &[target],
            RiskTier::Low,
            budget,
            "bounded",
        )
        .await;

        let now = Utc::now();
        assert!(store
            .claim_write_leases(
                run.run_id,
                std::slice::from_ref(&digest),
                now,
                now + chrono::Duration::minutes(61)
            )
            .await
            .is_err());
        assert!(store
            .claim_write_leases(
                run.run_id,
                &[digest],
                now,
                now + chrono::Duration::minutes(60)
            )
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn budget_exhaustion_blocks_a_further_run_rather_than_reducing_scope() {
        let store = SqliteNodeStore::open_in_memory().unwrap();
        let local_node_id = store.local_context_node_id().await.unwrap();
        register_local_context_node(&store).await;

        // Exactly one run attempt is affordable.
        let budget = WorkOrderBudget {
            wall_clock_secs: 3600,
            run_attempts: 1,
            model_tokens: 1000,
            effect_actions: 5,
        };
        let work_order = work_order_fixture(local_node_id, budget);
        let node = node_fixture(
            local_node_id,
            work_order.work_order_id,
            &["mindvault://schemas/budgeted"],
            RiskTier::Low,
        );
        let event = work_order_admitted_event(local_node_id, &work_order, 1, 0, "admit-budget");
        store
            .commit_work_order_with_event(&work_order, std::slice::from_ref(&node), &[], &event)
            .await
            .unwrap();

        let first = run_fixture(local_node_id, &work_order, &node, "budget-first");
        store
            .commit_agent_run_with_event(
                &first,
                &WorkOrderSpend::one_run_attempt(),
                &agent_run_started_event(local_node_id, &first, "budget-start-1"),
            )
            .await
            .unwrap();

        let spent = store
            .get_work_order(work_order.work_order_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(spent.remaining.run_attempts, 0);
        assert_eq!(spent.budget.run_attempts, 1, "the ceiling is immutable");
        assert_eq!(spent.revision, 2, "the spend advanced the revision");

        let mut second = run_fixture(local_node_id, &work_order, &node, "budget-second");
        second.attempt_no = 2;
        let err = store
            .commit_agent_run_with_event(
                &second,
                &WorkOrderSpend::one_run_attempt(),
                &agent_run_started_event(local_node_id, &second, "budget-start-2"),
            )
            .await
            .unwrap_err();
        assert!(err.to_string().contains("exhausted"), "got: {err}");

        // The refused run left nothing behind.
        assert!(store.get_agent_run(second.run_id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn a_run_cannot_record_its_own_g5() {
        let store = SqliteNodeStore::open_in_memory().unwrap();
        let local_node_id = store.local_context_node_id().await.unwrap();
        register_local_context_node(&store).await;

        let budget = WorkOrderBudget {
            wall_clock_secs: 3600,
            run_attempts: 5,
            model_tokens: 1000,
            effect_actions: 5,
        };
        let (work_order, _, run) = admitted_run(
            &store,
            local_node_id,
            &["mindvault://schemas/reviewed"],
            RiskTier::High,
            budget,
            "reviewed",
        )
        .await;

        let mut result = GateResult {
            result_id: Uuid::now_v7(),
            run_id: run.run_id,
            work_order_id: work_order.work_order_id,
            gate: GateId::G5,
            outcome: GateOutcome::Pass,
            evaluator_actor: run.actor.clone(),
            evidence_digest: "d".repeat(64),
            detail: None,
            evaluated_at: Utc::now(),
            created_at: Utc::now(),
        };
        assert!(store.record_gate_result(&result).await.is_err());

        result.evaluator_actor = work_order.principal.clone();
        store.record_gate_result(&result).await.unwrap();

        let recorded = store.list_gate_results(run.run_id).await.unwrap();
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0].gate, GateId::G5);
        assert_eq!(missing_gates(RiskTier::Low, &recorded).len(), 4);
    }

    #[tokio::test]
    async fn an_artifact_digest_must_describe_its_stored_bytes() {
        let store = SqliteNodeStore::open_in_memory().unwrap();
        let local_node_id = store.local_context_node_id().await.unwrap();
        register_local_context_node(&store).await;

        let budget = WorkOrderBudget {
            wall_clock_secs: 3600,
            run_attempts: 5,
            model_tokens: 1000,
            effect_actions: 5,
        };
        let (work_order, _, run) = admitted_run(
            &store,
            local_node_id,
            &["mindvault://schemas/produced"],
            RiskTier::Low,
            budget,
            "artifact",
        )
        .await;

        let payload = b"candidate decision summary";
        let mut hasher = Sha256::new();
        hasher.update(payload);
        let digest = format!("{:x}", hasher.finalize());
        let artifact_id = Uuid::now_v7();
        let anchor = StableUri::parse("mindvault://schemas/meeting-source").unwrap();

        let mut artifact = RunArtifact {
            artifact_id,
            artifact_uri: StableUri::run_artifact(local_node_id, artifact_id),
            run_id: run.run_id,
            work_order_id: work_order.work_order_id,
            artifact_kind: "decision-summary".into(),
            content_digest: "e".repeat(64),
            schema: None,
            sensitivity: Sensitivity::Internal,
            retention: RetentionClass::Operational,
            provenance: vec![ProvenanceReference {
                resource: anchor.clone(),
                relation: ProvenanceRelation::WasDerivedFrom,
            }],
            created_at: Utc::now(),
        };

        // A mismatched digest would make G2 verify a claim, not the content.
        assert!(store.record_run_artifact(&artifact, payload).await.is_err());

        artifact.content_digest = digest.clone();
        store.record_run_artifact(&artifact, payload).await.unwrap();

        let stored = store.get_run_artifact(artifact_id).await.unwrap().unwrap();
        assert_eq!(stored.content_digest, digest);
        // Derived knowledge never erases the authority of its evidence.
        assert_eq!(stored.provenance.len(), 1);
        assert_eq!(stored.provenance[0].resource, anchor);
        assert_eq!(
            stored.provenance[0].relation,
            ProvenanceRelation::WasDerivedFrom
        );

        // The bytes must come back. A write-only artifact store would let G2
        // compare a recorded digest against itself, and would make the
        // portability guarantee unmeetable.
        let read_back = store
            .read_run_artifact_payload(artifact_id)
            .await
            .unwrap()
            .expect("recorded artifact content must be readable");
        assert_eq!(read_back.as_slice(), payload);

        assert!(store
            .read_run_artifact_payload(Uuid::now_v7())
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn artifact_content_survives_arbitrary_bytes_and_is_not_json_inflated() {
        let store = SqliteNodeStore::open_in_memory().unwrap();
        let local_node_id = store.local_context_node_id().await.unwrap();
        register_local_context_node(&store).await;

        let budget = WorkOrderBudget {
            wall_clock_secs: 3600,
            run_attempts: 5,
            model_tokens: 1000,
            effect_actions: 5,
        };
        let (work_order, _, run) = admitted_run(
            &store,
            local_node_id,
            &["mindvault://schemas/produced"],
            RiskTier::Low,
            budget,
            "artifact-bytes",
        )
        .await;

        // Every byte value, including NUL and invalid UTF-8 sequences. An
        // artifact is opaque content, not a string.
        let payload: Vec<u8> = (0u16..=255).map(|b| b as u8).collect();
        let mut hasher = Sha256::new();
        hasher.update(&payload);
        let digest = format!("{:x}", hasher.finalize());
        let artifact_id = Uuid::now_v7();

        let artifact = RunArtifact {
            artifact_id,
            artifact_uri: StableUri::run_artifact(local_node_id, artifact_id),
            run_id: run.run_id,
            work_order_id: work_order.work_order_id,
            artifact_kind: "binary-diff".into(),
            content_digest: digest,
            schema: None,
            sensitivity: Sensitivity::Internal,
            retention: RetentionClass::Operational,
            provenance: vec![ProvenanceReference {
                resource: StableUri::parse("mindvault://schemas/binary-source").unwrap(),
                relation: ProvenanceRelation::WasDerivedFrom,
            }],
            created_at: Utc::now(),
        };
        store
            .record_run_artifact(&artifact, &payload)
            .await
            .unwrap();

        let read_back = store
            .read_run_artifact_payload(artifact_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(read_back, payload, "content must round-trip byte for byte");

        // Stored size tracks content size. Serializing through serde_json would
        // store `[0,1,2,...]` — several bytes per byte of content.
        let stored_len: i64 = store
            .with_conn(|connection| {
                connection
                    .query_row(
                        "SELECT length(payload) FROM agent_run_artifacts WHERE artifact_id = ?1",
                        params![artifact_id.to_string()],
                        |row| row.get(0),
                    )
                    .map_err(|err| MvError::Storage(err.to_string()))
            })
            .unwrap();
        assert_eq!(
            stored_len,
            payload.len() as i64,
            "unsealed artifact content is stored verbatim"
        );
    }

    #[tokio::test]
    async fn an_artifact_without_provenance_is_refused() {
        let store = SqliteNodeStore::open_in_memory().unwrap();
        let local_node_id = store.local_context_node_id().await.unwrap();
        register_local_context_node(&store).await;

        let budget = WorkOrderBudget {
            wall_clock_secs: 3600,
            run_attempts: 5,
            model_tokens: 1000,
            effect_actions: 5,
        };
        let (work_order, _, run) = admitted_run(
            &store,
            local_node_id,
            &["mindvault://schemas/unsourced"],
            RiskTier::Low,
            budget,
            "unsourced",
        )
        .await;

        let payload = b"unsourced claim";
        let mut hasher = Sha256::new();
        hasher.update(payload);
        let artifact_id = Uuid::now_v7();
        let artifact = RunArtifact {
            artifact_id,
            artifact_uri: StableUri::run_artifact(local_node_id, artifact_id),
            run_id: run.run_id,
            work_order_id: work_order.work_order_id,
            artifact_kind: "decision-summary".into(),
            content_digest: format!("{:x}", hasher.finalize()),
            schema: None,
            sensitivity: Sensitivity::Internal,
            retention: RetentionClass::Operational,
            provenance: Vec::new(),
            created_at: Utc::now(),
        };
        assert!(store.record_run_artifact(&artifact, payload).await.is_err());
    }

    #[tokio::test]
    async fn admission_replay_returns_the_original_work_order() {
        let store = SqliteNodeStore::open_in_memory().unwrap();
        let local_node_id = store.local_context_node_id().await.unwrap();
        register_local_context_node(&store).await;

        let budget = WorkOrderBudget {
            wall_clock_secs: 60,
            run_attempts: 2,
            model_tokens: 100,
            effect_actions: 1,
        };
        let work_order = work_order_fixture(local_node_id, budget);
        let node = node_fixture(
            local_node_id,
            work_order.work_order_id,
            &["mindvault://schemas/idempotent"],
            RiskTier::Low,
        );
        let event = work_order_admitted_event(local_node_id, &work_order, 1, 0, "admit-replay");

        let first = store
            .commit_work_order_with_event(&work_order, std::slice::from_ref(&node), &[], &event)
            .await
            .unwrap();
        assert!(!first.replayed);

        let replay = store
            .commit_work_order_with_event(&work_order, &[node], &[], &event)
            .await
            .unwrap();
        assert!(replay.replayed, "retrying admission must not duplicate");
        assert_eq!(replay.work_order.work_order_id, work_order.work_order_id);
        assert_eq!(replay.work_order.revision, 1);
    }

    #[tokio::test]
    async fn sealed_storage_does_not_expose_work_order_scope_or_goal() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("sealed_work_order.sqlite");
        let _reset = install_scoped_runtime_key(&db_path, [11u8; 32]);
        let store = SqliteNodeStore::open_with_mode(&db_path, true).unwrap();
        let local_node_id = store.local_context_node_id().await.unwrap();
        register_local_context_node(&store).await;

        let budget = WorkOrderBudget {
            wall_clock_secs: 60,
            run_attempts: 2,
            model_tokens: 100,
            effect_actions: 1,
        };
        let mut work_order = work_order_fixture(local_node_id, budget);
        work_order.goal = "secret-goal-text".into();
        let node = node_fixture(
            local_node_id,
            work_order.work_order_id,
            &["mindvault://schemas/secret-target"],
            RiskTier::Low,
        );
        let event = work_order_admitted_event(local_node_id, &work_order, 1, 0, "admit-sealed");
        store
            .commit_work_order_with_event(&work_order, &[node], &[], &event)
            .await
            .unwrap();

        let (payload, digest_row): (Vec<u8>, String) = store
            .with_conn(|conn| {
                conn.query_row(
                    "SELECT w.record_payload, t.target_digest
                     FROM work_orders w
                     JOIN work_order_node_write_targets t ON t.work_order_id = w.work_order_id
                     WHERE w.work_order_id = ?1",
                    params![work_order.work_order_id.to_string()],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .map_err(|e| MvError::Storage(e.to_string()))
            })
            .unwrap();

        assert!(
            !bytes_contains(&payload, b"secret-goal-text"),
            "a sealed work order must not store its goal in plaintext"
        );
        assert!(
            !digest_row.contains("secret-target"),
            "write scope is indexed as a digest, never as a readable target"
        );

        // It still round-trips for an authorized reader.
        let reopened = store
            .get_work_order(work_order.work_order_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(reopened.goal, "secret-goal-text");
    }
}
