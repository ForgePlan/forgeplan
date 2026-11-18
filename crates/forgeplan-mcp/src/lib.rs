pub mod constellation_read;
pub mod convert;
pub mod server;
pub mod types;

use std::path::{Path, PathBuf};

use rmcp::ServiceExt;

pub use server::ForgeplanServer;

/// Run the MCP server over stdio transport, with no maintenance.
///
/// Kept as the smallest possible entry point: it is what a caller that has
/// decided maintenance is somebody else's problem calls. See
/// [`serve_with_maintenance`] for the one that attaches a host.
pub async fn run_stdio(workspace_root: PathBuf) -> anyhow::Result<()> {
    serve_with_maintenance(workspace_root, None, false).await
}

/// Run the MCP server over stdio, attaching a maintenance host when this
/// process is entitled to one.
///
/// # Why the MCP server has a host at all
///
/// The read surface answers from whatever generation is published. On a
/// migrated workspace nothing else keeps that generation current: the legacy
/// `LanceStore` is retired, so without a host the index stays at whatever the
/// last `forgeplan index` published and every later edit is invisible until a
/// person runs a command. The standalone `forgeplan-mcp` binary had no host at
/// all, which is exactly that failure.
///
/// # Why it is still not the default
///
/// A host is a publisher, and publishing is a write. It is attached only when
/// one of two things holds:
///
/// * `auto_index` — an operator asked for it by name; or
/// * the resolved workspace is **already migrated**, in which case the
///   constellation index is the only projection that exists and there is no
///   legacy writer for a host to race.
///
/// A workspace that is neither gets no host. That is not caution for its own
/// sake: a constellation host running against a legacy workspace races the
/// `LanceStore` that still owns it, which is the split-brain the migration
/// exists to end. The reason is reported on stderr rather than dropped.
///
/// # Explicit workspace
///
/// `explicit_workspace` is the `--workspace` argument. It selects the server's
/// default workspace and the maintenance host, instead of the launch directory.
/// A request-level `workspace` parameter still overrides that default through
/// the normal resolution chain (param → env → default → cwd).
///
/// One host per index: [`MaintenanceHost::spawn`] takes the capability rather
/// than minting one, and [`MaintenanceSupervisor::register`] is idempotent per
/// physical index, so two hosts that reach one index serialise their passes
/// instead of racing for one owner lock.
pub async fn serve_with_maintenance(
    workspace_root: PathBuf,
    explicit_workspace: Option<&Path>,
    auto_index: bool,
) -> anyhow::Result<()> {
    let server_root = explicit_workspace.unwrap_or(&workspace_root).to_path_buf();
    let server = ForgeplanServer::new(server_root).await;
    let host = start_maintenance(&server, auto_index).await;
    let service = match server.serve(rmcp::transport::stdio()).await {
        Ok(service) => service,
        Err(error) => {
            if let Some(runner) = host {
                runner.stop().await;
            }
            return Err(error.into());
        }
    };
    let result = service.waiting().await;

    // Shutdown only after the transport is drained. A host torn down while a
    // request was still in flight would leave a pass owning storage commits
    // with nobody waiting for it.
    if let Some(runner) = host {
        runner.stop().await;
    }
    result.map(|_| ()).map_err(Into::into)
}

/// Start the host when this process is entitled to one.
///
/// Returns `None`, reported on stderr, when no maintenance scope is permitted.
/// The server can still answer reads; this is not a claim that it maintains
/// the selected index.
async fn start_maintenance(
    server: &ForgeplanServer,
    auto_index: bool,
) -> Option<forgeplan_core::constellation::runtime::MaintenanceRunner> {
    use forgeplan_core::constellation::facade::{MaintainCapability, workspace_is_migrated};
    use forgeplan_core::constellation::runtime::{MaintenanceHost, ReportSink};

    let ws = match server
        .resolve_workspace_dir(None, crate::server::DetectionPolicy::Strict)
        .await
    {
        Ok((workspace, _)) => workspace,
        Err(error) => {
            eprintln!("mcp: maintenance workspace resolution failed: {error}");
            return None;
        }
    };
    if !auto_index && !workspace_is_migrated(&ws) {
        eprintln!(
            "mcp: {} is not migrated and no explicit auto-index scope was given — \
             maintenance not started",
            ws.display()
        );
        return None;
    }
    let canonical = ws.canonicalize().unwrap_or(ws);
    match MaintenanceHost::spawn(
        MaintainCapability::for_strict_maintenance(canonical),
        ReportSink::Background { label: "mcp" },
    )
    .await
    {
        Ok(runner) => Some(runner),
        Err(e) => {
            eprintln!("mcp: maintenance host did not start: {e}");
            None
        }
    }
}
