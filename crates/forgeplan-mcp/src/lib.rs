pub mod convert;
pub mod server;
pub mod types;

use std::path::PathBuf;

use rmcp::ServiceExt;

pub use server::ForgeplanServer;

/// Run the MCP server over stdio transport.
pub async fn run_stdio(workspace_root: PathBuf) -> anyhow::Result<()> {
    let server = ForgeplanServer::new(workspace_root).await;
    let service = server.serve(rmcp::transport::stdio()).await?;
    service.waiting().await?;
    Ok(())
}
