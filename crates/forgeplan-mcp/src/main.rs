use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // MCP protocol uses stdout — all logs must go to stderr
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    let cwd = std::env::current_dir()?;
    forgeplan_mcp::run_stdio(cwd).await
}
