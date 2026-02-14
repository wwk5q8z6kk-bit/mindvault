use std::path::PathBuf;

use base64::Engine as _;
use mv_core::*;
use mv_storage::vault_crypto::VaultCrypto;

use super::{MindVaultEngine, SEALED_BLOB_MAGIC};

impl MindVaultEngine {
    // ── Initialization & Sealed-Mode Lifecycle ─────────────────────

    pub async fn rebuild_runtime_indexes(&self) -> MvResult<()> {
        if !self.config.sealed_mode {
            return Ok(());
        }
        if !self.keychain.is_unsealed_sync() {
            return Err(MvError::VaultSealed);
        }
        self.keychain.sync_runtime_storage_key().await?;

        let nodes = self
            .store
            .nodes
            .list(&QueryFilters::default(), 100_000, 0)
            .await?;
        for node in &nodes {
            self.fts.index_node(node)?;
            if let Some(ref vectors) = self.store.vectors {
                if let Ok(embedding) = self.store.embedder.embed(&node.content).await {
                    let _ = vectors
                        .upsert(node.id, embedding, &node.content, Some(&node.namespace))
                        .await;
                }
            }
        }
        self.fts.commit()?;
        Ok(())
    }

    pub(crate) async fn encrypt_blob_for_namespace(
        &self,
        namespace: &str,
        plaintext: &[u8],
    ) -> MvResult<Vec<u8>> {
        let dek = VaultCrypto::generate_node_dek();
        let wrapped_dek = self.keychain.wrap_namespace_dek(namespace, &dek).await?;
        let ciphertext = VaultCrypto::aes_gcm_encrypt_pub(&dek, plaintext)
            .map_err(|err| MvError::Storage(format!("blob encrypt failed: {err}")))?;
        let envelope = serde_json::json!({
            "v": 1,
            "wrapped_dek": wrapped_dek,
            "ciphertext": base64::engine::general_purpose::STANDARD.encode(ciphertext),
        });
        let body = serde_json::to_vec(&envelope)
            .map_err(|err| MvError::Storage(format!("blob envelope encode failed: {err}")))?;
        let mut out = Vec::with_capacity(SEALED_BLOB_MAGIC.len() + body.len());
        out.extend_from_slice(SEALED_BLOB_MAGIC);
        out.extend_from_slice(&body);
        Ok(out)
    }

    async fn migrate_legacy_attachments_for_node(&self, node: &KnowledgeNode) -> MvResult<usize> {
        let attachments = node
            .metadata
            .get("attachments")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();

        if attachments.is_empty() {
            return Ok(0);
        }

        let expected_base = PathBuf::from(&self.config.data_dir)
            .join("blobs")
            .join(node.id.to_string());
        let canonical_base = match tokio::fs::canonicalize(&expected_base).await {
            Ok(path) => path,
            Err(_) => return Ok(0),
        };

        let mut migrated = 0usize;
        for attachment in attachments {
            let Some(stored_path) = attachment
                .get("stored_path")
                .and_then(serde_json::Value::as_str)
            else {
                continue;
            };

            let candidate = PathBuf::from(stored_path);
            let canonical_candidate = match tokio::fs::canonicalize(&candidate).await {
                Ok(path) => path,
                Err(_) => continue,
            };
            if !canonical_candidate.starts_with(&canonical_base) {
                continue;
            }

            let bytes = match tokio::fs::read(&canonical_candidate).await {
                Ok(bytes) => bytes,
                Err(_) => continue,
            };
            if bytes.starts_with(SEALED_BLOB_MAGIC) {
                continue;
            }

            let encrypted = self
                .encrypt_blob_for_namespace(&node.namespace, &bytes)
                .await?;
            tokio::fs::write(&canonical_candidate, encrypted)
                .await
                .map_err(|err| MvError::Storage(format!("rewrite attachment failed: {err}")))?;
            migrated += 1;
        }

        Ok(migrated)
    }

    pub async fn migrate_sealed_storage(&self) -> MvResult<()> {
        if !self.config.sealed_mode {
            return Ok(());
        }
        if !self.keychain.is_unsealed_sync() {
            return Err(MvError::VaultSealed);
        }
        self.keychain.sync_runtime_storage_key().await?;

        let nodes = self
            .store
            .nodes
            .list(&QueryFilters::default(), 100_000, 0)
            .await?;
        let mut migrated_nodes = 0usize;
        let mut migrated_blobs = 0usize;
        for node in &nodes {
            self.store.nodes.update(node).await?;
            migrated_nodes += 1;
            migrated_blobs += self.migrate_legacy_attachments_for_node(node).await?;
        }

        for legacy_index_dir in ["tantivy", "lancedb"] {
            let path = PathBuf::from(&self.config.data_dir).join(legacy_index_dir);
            if tokio::fs::metadata(&path).await.is_ok() {
                let _ = tokio::fs::remove_dir_all(&path).await;
            }
        }

        tracing::info!(
            migrated_nodes,
            migrated_blobs,
            "sealed storage migration completed"
        );
        Ok(())
    }

    /// Returns true when sealed mode is enabled in runtime config.
    pub fn is_sealed(&self) -> bool {
        self.config.sealed_mode && !self.keychain.is_unsealed_sync()
    }

    pub(crate) async fn ensure_unsealed_for_node_io(&self) -> MvResult<()> {
        if self.is_sealed() {
            return Err(MvError::VaultSealed);
        }
        if self.config.sealed_mode {
            self.keychain.sync_runtime_storage_key().await?;
        }
        Ok(())
    }

    /// Set up the enrichment pipeline. Returns the worker that should be spawned.
    /// Must be called after `init_arc()` and before using enrichment features.
    pub fn setup_enrichment(
        &mut self,
        change_tx: tokio::sync::broadcast::Sender<mv_core::ChangeNotification>,
    ) -> Option<crate::enrichment::EnrichmentWorker> {
        if !self.config.ai.enrichment_enabled {
            tracing::info!("enrichment pipeline disabled by config");
            return None;
        }

        let (pipeline, worker) = crate::enrichment::EnrichmentPipeline::new(
            std::sync::Arc::clone(&self.store),
            self.config.ai.clone(),
            self.llm.clone(),
            change_tx,
        );
        self.enrichment = Some(pipeline);
        tracing::info!("enrichment pipeline initialized");
        Some(worker)
    }
}
