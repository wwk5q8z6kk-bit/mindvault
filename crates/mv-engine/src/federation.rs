//! Federated query protocol.
//! Enables read-only queries across trusted peer vaults via REST.
//!
//! Each peer is another MindVault instance exposing `/api/v1/recall`.
//! Queries run in parallel with per-peer timeouts and error isolation.

use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use mv_core::*;
use mv_storage::unified::UnifiedStore;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A trusted federation peer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationPeer {
    pub id: Uuid,
    pub vault_id: String,
    pub display_name: String,
    pub endpoint: String,
    pub public_key: Option<String>,
    pub allowed_namespaces: Vec<String>,
    pub max_results: usize,
    pub enabled: bool,
    pub last_seen: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl FederationPeer {
    pub fn new(
        vault_id: impl Into<String>,
        display_name: impl Into<String>,
        endpoint: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::now_v7(),
            vault_id: vault_id.into(),
            display_name: display_name.into(),
            endpoint: endpoint.into(),
            public_key: None,
            allowed_namespaces: Vec::new(),
            max_results: 50,
            enabled: true,
            last_seen: None,
            created_at: Utc::now(),
        }
    }

    /// Base URL for this peer's REST API (strips trailing slash).
    fn api_base(&self) -> &str {
        self.endpoint.trim_end_matches('/')
    }
}

/// Result from a federated query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedResult {
    pub source_vault: String,
    pub source_peer_name: String,
    pub node: KnowledgeNode,
    pub relevance_score: f64,
}

/// REST recall response shape (mirrors mv-server's JSON format).
#[derive(Deserialize)]
struct PeerRecallResponse {
    results: Vec<PeerSearchResult>,
}

#[derive(Deserialize)]
struct PeerSearchResult {
    node: KnowledgeNode,
    score: f64,
}

/// REST health response shape.
#[derive(Deserialize)]
struct PeerHealthResponse {
    status: String,
}

/// Federation engine manages peers and dispatches queries.
pub struct FederationEngine {
    #[allow(dead_code)]
    store: Arc<UnifiedStore>,
    peers: tokio::sync::RwLock<Vec<FederationPeer>>,
    client: reqwest::Client,
}

impl FederationEngine {
    pub fn new(store: Arc<UnifiedStore>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .connect_timeout(Duration::from_secs(5))
            .user_agent("MindVault-Federation/0.1")
            .build()
            .unwrap_or_default();

        Self {
            store,
            peers: tokio::sync::RwLock::new(Vec::new()),
            client,
        }
    }

    /// Add a trusted peer.
    pub async fn add_peer(&self, peer: FederationPeer) {
        self.peers.write().await.push(peer);
    }

    /// Remove a peer by ID.
    pub async fn remove_peer(&self, id: Uuid) -> bool {
        let mut peers = self.peers.write().await;
        let len_before = peers.len();
        peers.retain(|p| p.id != id);
        peers.len() < len_before
    }

    /// List all peers.
    pub async fn list_peers(&self) -> Vec<FederationPeer> {
        self.peers.read().await.clone()
    }

    /// Get a peer by ID.
    pub async fn get_peer(&self, id: Uuid) -> Option<FederationPeer> {
        self.peers.read().await.iter().find(|p| p.id == id).cloned()
    }

    /// Health check a peer by calling its `/api/v1/health` endpoint.
    pub async fn health_check(&self, peer_id: Uuid) -> MvResult<bool> {
        let peers = self.peers.read().await;
        let Some(peer) = peers.iter().find(|p| p.id == peer_id) else {
            return Ok(false);
        };

        if !peer.enabled {
            return Ok(false);
        }

        let url = format!("{}/api/v1/health", peer.api_base());
        tracing::debug!(peer = %peer.display_name, %url, "Federation health check");

        match self.client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(body) = resp.json::<PeerHealthResponse>().await {
                    let healthy = body.status == "ok";
                    // Update last_seen on success
                    drop(peers);
                    self.update_last_seen(peer_id).await;
                    Ok(healthy)
                } else {
                    Ok(false)
                }
            }
            Ok(resp) => {
                tracing::warn!(
                    peer = %peer.display_name,
                    status = %resp.status(),
                    "Peer health check returned non-OK status"
                );
                Ok(false)
            }
            Err(e) => {
                tracing::warn!(
                    peer = %peer.display_name,
                    error = %e,
                    "Peer health check failed"
                );
                Ok(false)
            }
        }
    }

    /// Query across all enabled peers via their REST recall endpoints.
    /// Runs queries in parallel; errors from individual peers are logged but don't fail the batch.
    pub async fn federated_query(
        &self,
        query: &str,
        limit: usize,
    ) -> MvResult<Vec<FederatedResult>> {
        let peers = self.peers.read().await;
        let enabled: Vec<_> = peers.iter().filter(|p| p.enabled).cloned().collect();
        drop(peers);

        if enabled.is_empty() {
            return Ok(Vec::new());
        }

        tracing::info!(
            query = query,
            peer_count = enabled.len(),
            "Dispatching federated query to peers"
        );

        // Fire parallel requests to all enabled peers
        let mut handles = Vec::with_capacity(enabled.len());
        for peer in &enabled {
            let client = self.client.clone();
            let peer = peer.clone();
            let query = query.to_string();
            let per_peer_limit = limit.min(peer.max_results);

            handles.push(tokio::spawn(async move {
                query_peer(&client, &peer, &query, per_peer_limit).await
            }));
        }

        // Collect results, logging errors per-peer
        let mut all_results = Vec::new();
        for (i, handle) in handles.into_iter().enumerate() {
            match handle.await {
                Ok(Ok(results)) => {
                    if !results.is_empty() {
                        tracing::debug!(
                            peer = %enabled[i].display_name,
                            count = results.len(),
                            "Peer returned results"
                        );
                    }
                    all_results.extend(results);
                }
                Ok(Err(e)) => {
                    tracing::warn!(
                        peer = %enabled[i].display_name,
                        error = %e,
                        "Peer query failed"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        peer = %enabled[i].display_name,
                        error = %e,
                        "Peer query task panicked"
                    );
                }
            }
        }

        // Update last_seen for peers that responded (best-effort)
        for peer in &enabled {
            self.update_last_seen(peer.id).await;
        }

        // Sort by relevance descending, then truncate to requested limit
        all_results.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        all_results.truncate(limit);

        Ok(all_results)
    }

    /// Update the last_seen timestamp for a peer.
    async fn update_last_seen(&self, peer_id: Uuid) {
        let mut peers = self.peers.write().await;
        if let Some(peer) = peers.iter_mut().find(|p| p.id == peer_id) {
            peer.last_seen = Some(Utc::now());
        }
    }
}

/// Query a single peer's recall endpoint.
async fn query_peer(
    client: &reqwest::Client,
    peer: &FederationPeer,
    query: &str,
    limit: usize,
) -> Result<Vec<FederatedResult>, String> {
    let url = format!("{}/api/v1/recall", peer.api_base());

    let mut body = serde_json::json!({
        "text": query,
        "limit": limit,
        "strategy": "hybrid",
    });

    // Scope to allowed namespaces if configured
    if peer.allowed_namespaces.len() == 1 {
        body["namespace"] = serde_json::json!(peer.allowed_namespaces[0]);
    }

    let response = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("request failed: {e}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("peer returned {status}: {text}"));
    }

    let recall: PeerRecallResponse = response
        .json()
        .await
        .map_err(|e| format!("failed to parse peer response: {e}"))?;

    Ok(recall
        .results
        .into_iter()
        .map(|r| FederatedResult {
            source_vault: peer.vault_id.clone(),
            source_peer_name: peer.display_name.clone(),
            relevance_score: r.score,
            node: r.node,
        })
        .collect())
}
