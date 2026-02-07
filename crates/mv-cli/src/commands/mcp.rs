use std::sync::Arc;

use anyhow::Result;
use mv_mcp::server::McpServer;

pub async fn run(config_path: &str) -> Result<()> {
	let engine = super::load_engine(config_path).await?;
	let engine = Arc::new(engine);
	let server = McpServer::new(engine);
	server
		.run_stdio()
		.await
		.map_err(|e| anyhow::anyhow!("MCP server error: {e}"))?;
	Ok(())
}
