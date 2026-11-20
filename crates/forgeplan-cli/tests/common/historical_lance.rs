use std::path::{Path, PathBuf};

/// Build an isolated historical layout, independently of current CLI init.
pub fn init_legacy_workspace(root: &Path, project_name: &str) -> PathBuf {
    let workspace = forgeplan_core::workspace::init_workspace(root, project_name)
        .expect("initialize fixture workspace files");
    // Some callers already run under Tokio; keep this synchronous fixture's
    // runtime on a separate thread rather than nesting block_on.
    std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("create fixture runtime")
            .block_on(async {
                let guard = forgeplan_core::workspace::acquire_workspace_lock(&workspace)
                    .await
                    .expect("lock fixture workspace");
                forgeplan_core::db::store::LanceStore::init_legacy_for_test(&workspace, &guard)
                    .await
                    .expect("initialize historical legacy fixture");
            });
        workspace
    })
    .join()
    .expect("historical fixture worker")
}
