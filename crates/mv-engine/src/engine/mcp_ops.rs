use chrono::Utc;
use mv_core::*;
use uuid::Uuid;

use super::MindVaultEngine;

impl MindVaultEngine {
    // ── MCP Connectors ───────────────────────────────────────────────

    #[allow(clippy::too_many_arguments)]
    pub async fn create_mcp_connector(
        &self,
        name: String,
        description: Option<String>,
        publisher: Option<String>,
        version: String,
        homepage_url: Option<String>,
        repository_url: Option<String>,
        config_schema: serde_json::Value,
        capabilities: Vec<String>,
        verified: bool,
    ) -> MvResult<McpConnector> {
        let now = Utc::now();
        let connector = McpConnector {
            id: Uuid::now_v7(),
            name,
            description,
            publisher,
            version,
            homepage_url,
            repository_url,
            config_schema,
            capabilities,
            verified,
            created_at: now,
            updated_at: now,
        };

        self.store.nodes.insert_mcp_connector(&connector).await?;
        Ok(connector)
    }

    pub async fn list_mcp_connectors(
        &self,
        publisher: Option<&str>,
        verified: Option<bool>,
        limit: usize,
        offset: usize,
    ) -> MvResult<Vec<McpConnector>> {
        self.store
            .nodes
            .list_mcp_connectors(publisher, verified, limit, offset)
            .await
    }

    pub async fn get_mcp_connector(&self, connector_id: Uuid) -> MvResult<Option<McpConnector>> {
        self.store.nodes.get_mcp_connector(connector_id).await
    }

    pub async fn update_mcp_connector(&self, connector: McpConnector) -> MvResult<bool> {
        self.store.nodes.update_mcp_connector(&connector).await
    }

    pub async fn delete_mcp_connector(&self, connector_id: Uuid) -> MvResult<bool> {
        self.store.nodes.delete_mcp_connector(connector_id).await
    }
}
