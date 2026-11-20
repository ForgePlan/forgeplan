use tracing_subscriber::EnvFilter;

/// Standalone MCP server.
///
/// Serves the **full** tool surface over stdio from the process's working
/// directory. `forgeplan serve` is the same server with the CLI's extra
/// surface; this binary exists so an MCP client can launch the server without
/// the CLI.
///
/// # `--auto-index` and `--workspace`
///
/// The full server keeps a migrated workspace's index current, because nothing
/// else does — the legacy `LanceStore` is retired there. A workspace that is
/// **not** migrated gets no host: a constellation host would race the store that
/// still owns it. `--auto-index` is how an operator says "yes, maintain this
/// one anyway", and `--workspace` says *which* one — it wins over the working
/// directory, so a server launched in X can be pointed at Y without changing
/// where each request resolves its own workspace.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // MCP protocol uses stdout — all logs must go to stderr
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    let mut workspace: Option<std::path::PathBuf> = None;
    let mut auto_index = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            // Explicitly "serve". Accepted and ignored: stdio is the only
            // transport, and an MCP client config that says `serve` must not
            // fail against this binary.
            "serve" => {}
            "--auto-index" => auto_index = true,
            "--workspace" => {
                workspace =
                    Some(std::path::PathBuf::from(args.next().ok_or_else(|| {
                        anyhow::anyhow!("--workspace needs a path")
                    })?));
            }
            other => anyhow::bail!(
                "unknown argument {other:?} — `forgeplan-mcp [serve] [--auto-index] \
                 [--workspace PATH]`"
            ),
        }
    }

    let cwd = std::env::current_dir()?;
    forgeplan_mcp::serve_with_maintenance(cwd, workspace.as_deref(), auto_index).await
}
