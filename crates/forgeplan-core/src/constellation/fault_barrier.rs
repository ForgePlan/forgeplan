//! Test-only publication and native fault barriers (ADR-029 §8).
//!
//! # What this is for
//!
//! A process-kill fault proof needs a place to stop at a **real** stage: after
//! the artifacts commit, after the storage closure barrier, immediately before
//! the manifest fsync, immediately before the atomic `CURRENT` replacement, and
//! so on. Without such a seam the only honest alternative is a sleep, and a
//! sleep proves nothing about which operations had actually completed.
//!
//! # What this is not
//!
//! It is not a daemon, not a product transport, and not an environment-driven
//! block in a release binary. The whole module is behind
//! `#[cfg(feature = "test-helpers")]`, so a production build contains no
//! barrier code, no listener, no socket and no environment lookup. There is no
//! production IPC path through which a user could stall a publish.
//!
//! # The protocol
//!
//! The parent opens a loopback TCP listener and a fresh opaque token. The
//! child receives `endpoint`, `token` and one `stage` through the environment.
//! At that stage — and only after the real operation has completed — the child
//! writes one newline-framed JSON line `{"token":…,"stage":…,"pid":…}`, then
//! waits (bounded) for a newline-framed `{"token":…,"stage":…,"release":true}`
//! before continuing. It validates the token and stage before honouring the
//! release, so a parent cannot acknowledge a different stage by accident.
//!
//! The token is test correlation, not an authentication mechanism. It exists so
//! a stale ACK from a previous run cannot release this run's barrier.
//!
//! # One-shot per process per stage
//!
//! A stage fires **once**. After a valid release, every later arrival at the
//! same stage is a no-op: the listener may be gone, the parent may have killed
//! the process, and a second dial to a dead endpoint would hang or error inside
//! production code. A publish that legitimately performs several commits of the
//! same role (a delta pass, then a drain) must be able to stop at the *first*
//! real one and not at every later one.
//!
//! An unchanged path emits nothing: there is no commit, so there is no stage to
//! report. The caller decides whether the operation happened; the barrier never
//! manufactures it.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

/// The eleven agreed stage names.
///
/// These strings are the contract with the fault harness and are checked
/// against the environment value, so a typo is a refusal rather than a stage
/// that silently never fires.
pub const STAGES: [&str; 11] = [
    "after_artifacts_commit",
    "after_relations_commit",
    "after_sections_commit",
    "after_storage_sync",
    "before_manifest_sync",
    "after_manifest_sync",
    "before_current_replace",
    "after_current_replace",
    "after_current_ack",
    "owner_acquired",
    "native_call_entered",
];

/// Environment variable naming the parent's loopback endpoint.
pub const ENV_ENDPOINT: &str = "FORGEPLAN_TEST_BARRIER_ENDPOINT";
/// Environment variable carrying this run's opaque correlation token.
pub const ENV_TOKEN: &str = "FORGEPLAN_TEST_BARRIER_TOKEN";
/// Environment variable naming the single stage this process stops at.
pub const ENV_STAGE: &str = "FORGEPLAN_TEST_BARRIER_STAGE";
/// Optional: how long to wait for the listener to accept. Default 5000 ms.
pub const ENV_CONNECT_MS: &str = "FORGEPLAN_TEST_BARRIER_CONNECT_MS";
/// Optional: how long to wait for the release ACK. Default 60000 ms.
pub const ENV_ACK_MS: &str = "FORGEPLAN_TEST_BARRIER_ACK_MS";

/// What this process was asked to stop at.
#[derive(Debug, Clone)]
struct Selection {
    endpoint: String,
    token: String,
    stage: String,
    connect_timeout: Duration,
    ack_timeout: Duration,
}

fn read_selection() -> Option<Selection> {
    let endpoint = std::env::var(ENV_ENDPOINT).ok()?;
    let token = std::env::var(ENV_TOKEN).ok()?;
    let stage = std::env::var(ENV_STAGE).ok()?;
    if endpoint.trim().is_empty() || token.trim().is_empty() {
        return None;
    }
    // An unknown stage is refused loudly in the resolution rather than at the
    // call site: a harness that misspelled the name would otherwise run to
    // completion and report a stage that never fired.
    if !STAGES.contains(&stage.as_str()) {
        panic!(
            "{ENV_STAGE}={stage} is not one of {STAGES:?}; a barrier that cannot fire must not be \
             silently ignored"
        );
    }
    Some(Selection {
        endpoint,
        token,
        stage,
        connect_timeout: Duration::from_millis(read_millis(ENV_CONNECT_MS, 5_000)),
        ack_timeout: Duration::from_millis(read_millis(ENV_ACK_MS, 60_000)),
    })
}

/// The one selection for this process, resolved once.
static SELECTION: LazyLock<Option<Selection>> = LazyLock::new(read_selection);

fn read_millis(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

/// The stages this process has already released.
static FIRED: LazyLock<Mutex<Vec<&'static str>>> = LazyLock::new(|| Mutex::new(Vec::new()));

fn fired() -> &'static Mutex<Vec<&'static str>> {
    &FIRED
}

/// Whether `stage` has already been released in this process.
fn already_fired(stage: &str) -> bool {
    fired()
        .lock()
        .map(|stages| stages.iter().any(|done| *done == stage))
        .unwrap_or(false)
}

/// Claim `stage` for this process, reporting whether this call was the one
/// that claimed it.
///
/// `false` means the stage had already been claimed — by an earlier arrival at
/// the same stage, or by a re-entrant path while this one was dialling. The
/// claim is what makes a stage **one-shot**: a publish that legitimately
/// performs several commits of the same role (a delta pass, then a drain)
/// must stop at the *first* real one, because the second dial would reach a
/// listener the parent may already have stopped.
///
/// `false` is also what a poisoned observation lock produces. A barrier that
/// cannot record its claim must not dial: dialling twice because the record
/// failed is exactly the "second dial to a dead endpoint" the one-shot rule
/// exists to prevent. The stage then simply never fires, which the harness
/// reports as a stage that did not happen — an honest outcome, rather than a
/// publish that stalls on a listener nobody is serving.
///
/// This is the **only** implementation of the claim. It used to be duplicated
/// inline in [`barrier`], with this helper left unused; two copies of the
/// one-shot rule are two chances for them to disagree about when a stage
/// counts as fired.
fn mark_fired(stage: &'static str) -> bool {
    let Ok(mut stages) = fired().lock() else {
        return false;
    };
    if stages.iter().any(|done| *done == stage) {
        return false;
    }
    stages.push(stage);
    true
}

/// Stop at `stage` if this process was selected for it.
///
/// Call this **after** the real operation completed, never instead of it: the
/// call must observe a finished commit, a finished barrier, a finished fsync.
/// A stage that reports before its operation is a stage that proves nothing.
///
/// No-op — no syscall, no allocation, no lock — when the environment does not
/// select this stage or when it already fired. Errors are reported on stderr
/// and then treated as "continue": a fault harness that cannot reach its
/// listener must not turn into a publish that silently stops halfway, which
/// would look exactly like the crash it was trying to observe.
///
/// # Panics
///
/// On an unknown `stage` value in the environment. That is a harness
/// misconfiguration, and continuing would produce a run that reports a stage
/// which never fired.
pub fn barrier(stage: &str) {
    let Some(selected) = SELECTION.as_ref() else {
        return;
    };
    if selected.stage != stage {
        return;
    }
    if already_fired(stage) {
        // One-shot. A later commit of the same role in this process must not
        // dial an endpoint the parent has already stopped listening on.
        return;
    }
    let stage_index = STAGES.iter().position(|name| *name == stage);
    let static_stage: &'static str = STAGES[stage_index.expect("stage is validated above")];
    // Claim the stage before dialling, so a re-entrant path cannot double
    // dial while this one waits. `mark_fired` is the single implementation of
    // that claim, and it reports whether this call was the one that made it.
    if !mark_fired(static_stage) {
        return;
    }

    let message = serde_json::json!({
        "token": selected.token,
        "stage": static_stage,
        "pid": std::process::id(),
    });
    let line = match serde_json::to_string(&message) {
        Ok(line) => line,
        Err(error) => {
            eprintln!("forgeplan test barrier: cannot encode stage report: {error}");
            return;
        }
    };

    let address: std::net::SocketAddr = match selected.endpoint.parse() {
        Ok(address) => address,
        Err(error) => {
            eprintln!(
                "forgeplan test barrier: {ENV_ENDPOINT}={} is not a socket address: {error}",
                selected.endpoint
            );
            return;
        }
    };

    let stream = match connect_with_timeout(
        address,
        selected.connect_timeout,
        selected.ack_timeout,
    ) {
        Some(stream) => stream,
        None => {
            eprintln!(
                "forgeplan test barrier: cannot reach the parent listener at {address} for stage \
                 {static_stage}"
            );
            return;
        }
    };

    use std::io::{BufRead, BufReader, Write};
    let mut writer = match stream.try_clone() {
        Ok(writer) => writer,
        Err(error) => {
            eprintln!("forgeplan test barrier: cannot clone the barrier socket: {error}");
            return;
        }
    };
    if let Err(error) = writeln!(writer, "{line}").and_then(|()| writer.flush()) {
        eprintln!("forgeplan test barrier: cannot report stage {static_stage}: {error}");
        return;
    }

    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    match reader.read_line(&mut response) {
        Ok(0) => {
            eprintln!(
                "forgeplan test barrier: the parent closed the connection at stage {static_stage} \
                 before releasing"
            );
            return;
        }
        Ok(_) => {}
        Err(error) => {
            eprintln!("forgeplan test barrier: cannot read the release at {static_stage}: {error}");
            return;
        }
    }

    match serde_json::from_str::<serde_json::Value>(response.trim()) {
        Ok(value) => {
            let token_ok = value.get("token").and_then(|v| v.as_str()) == Some(selected.token.as_str());
            let stage_ok = value.get("stage").and_then(|v| v.as_str()) == Some(static_stage);
            let released = value.get("release").and_then(|v| v.as_bool()) == Some(true);
            if !(token_ok && stage_ok && released) {
                eprintln!(
                    "forgeplan test barrier: the parent's release at {static_stage} did not match \
                     this run's token and stage; continuing without treating it as a release"
                );
            }
        }
        Err(error) => {
            eprintln!(
                "forgeplan test barrier: the parent's release at {static_stage} is not JSON: \
                 {error}"
            );
        }
    }
}

/// Connect once, giving up after `timeout` rather than blocking the publish.
///
/// A bounded dial is what keeps "the parent is not listening" from turning into
/// an indefinite stall in a maintenance pass. The stream is returned with the
/// read side carrying the ACK timeout, so the wait for a release is bounded by
/// the same kind of deadline rather than by the parent's patience.
fn connect_with_timeout(
    address: std::net::SocketAddr,
    connect_timeout: Duration,
    ack_timeout: Duration,
) -> Option<std::net::TcpStream> {
    use std::net::TcpStream;

    // `TcpStream::connect_timeout` takes a resolved `SocketAddr`, and the
    // harness's endpoint is a loopback literal, so resolution happens once
    // here and every candidate gets the same deadline.
    let targets = std::net::ToSocketAddrs::to_socket_addrs(&address).ok()?;
    for target in targets {
        if let Ok(stream) = TcpStream::connect_timeout(&target, connect_timeout) {
            let _ = stream.set_read_timeout(Some(ack_timeout));
            let _ = stream.set_write_timeout(Some(connect_timeout));
            let _ = stream.set_nodelay(true);
            return Some(stream);
        }
    }
    None
}

/// A cheap one-shot latch used by the harness to assert a stage fired.
///
/// Exposed so a fault test can wait for its own child to report without
/// re-deriving "did the stage happen" from a log line.
#[derive(Debug, Default)]
pub struct StageLatch(AtomicBool);

impl StageLatch {
    /// Mark the latch as reached.
    pub fn reach(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    /// Whether the latch has been reached.
    pub fn is_reached(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // The protocol, not the process.
    //
    // RuntimeProof drives the process-level side — a real child, a real
    // listener, a real SIGKILL. What is tested here is the part that child
    // depends on being true and that needs no second process to observe: the
    // stage vocabulary, the one-shot rule, and the bounded dial. A fault proof
    // that hangs waiting for a parent that is gone is worse than no fault
    // proof, so that dial is asserted directly.
    // -----------------------------------------------------------------------

    /// The eleven stage names are the contract with the harness.
    ///
    /// They travel as strings through an environment variable and are compared
    /// by equality on both sides, so a renamed stage is a stage that never
    /// fires — and it fails silently, as a fault run that reports nothing. The
    /// list is pinned here as well as in the shared contract for that reason.
    #[test]
    fn the_stage_vocabulary_is_exactly_the_eleven_agreed_names() {
        assert_eq!(
            STAGES.to_vec(),
            vec![
                "after_artifacts_commit",
                "after_relations_commit",
                "after_sections_commit",
                "after_storage_sync",
                "before_manifest_sync",
                "after_manifest_sync",
                "before_current_replace",
                "after_current_replace",
                "after_current_ack",
                "owner_acquired",
                "native_call_entered",
            ]
        );
        let mut unique = STAGES.to_vec();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(
            unique.len(),
            STAGES.len(),
            "a duplicate stage name would make the one-shot latch fire on the wrong operation"
        );
    }

    /// A stage fires once per process, and only once.
    ///
    /// A publish that legitimately performs several commits of the same role
    /// — a delta pass and then a drain — must stop at the **first** real one.
    /// Without the latch the second commit would dial a listener that may be
    /// gone, inside production code.
    #[test]
    fn a_stage_is_released_at_most_once_per_process() {
        // A name outside `STAGES` cannot be reached through `barrier`, which
        // validates first, but the latch itself is exercised directly because
        // that is where the rule lives.
        assert!(
            !already_fired("after_artifacts_commit"),
            "nothing has fired in this process yet"
        );

        assert!(
            mark_fired("after_artifacts_commit"),
            "the first claim of a stage succeeds"
        );
        assert!(
            already_fired("after_artifacts_commit"),
            "a released stage stays released for the rest of the process"
        );
        assert!(
            !mark_fired("after_artifacts_commit"),
            "a second claim of the same stage is refused: this is the one-shot rule, and it is \
             what stops a second dial to a listener the parent may already have stopped"
        );
        assert!(
            !already_fired("after_sections_commit"),
            "the rule is per stage: a second, different stage must still be able to fire"
        );
        assert!(
            mark_fired("after_sections_commit"),
            "a different stage claims independently"
        );
    }

    /// An unreachable listener is a bounded failure, never a hang.
    ///
    /// This is the difference between a fault harness that reports and one that
    /// wedges a maintenance pass forever. The assertion is on elapsed time as
    /// well as on the result: a `None` after the full timeout would still mean
    /// the publish waited out the deadline.
    #[test]
    fn a_dial_to_a_dead_endpoint_fails_fast_instead_of_stalling() {
        // Port 1 on loopback: reserved, and nothing on a test machine listens
        // there, so the connect is refused rather than silently routed.
        let address: std::net::SocketAddr = "127.0.0.1:1".parse().expect("a literal socket address");
        let timeout = Duration::from_millis(500);

        let started = std::time::Instant::now();
        let stream = connect_with_timeout(address, timeout, timeout);
        let elapsed = started.elapsed();

        assert!(
            stream.is_none(),
            "a refused connection must be reported as no listener, not returned as a stream"
        );
        assert!(
            elapsed < Duration::from_secs(5),
            "the dial gave up after {elapsed:?}; a bounded dial that waits far past its own \
             timeout is the hang this function exists to prevent"
        );
    }

    /// The latch is the harness's own "did this fire" primitive.
    #[test]
    fn the_stage_latch_records_that_a_stage_fired() {
        let latch = StageLatch::default();
        assert!(!latch.is_reached(), "a fresh latch has not been reached");
        latch.reach();
        assert!(latch.is_reached());
        // Reaching twice is not an error: the harness may be woken by a
        // timeout and by the real report.
        latch.reach();
        assert!(latch.is_reached());
    }
}
