//! The blocking inference worker: one native call at a time, in this process.
//!
//! # Why a `std::thread` and not `tokio::spawn_blocking`
//!
//! The engine call is **blocking and native**. `spawn_blocking` holds a slot in
//! the blocking pool that the runtime cannot reclaim until the call returns, so
//! a hung `tract` call prevents process shutdown from completing. A dedicated
//! `std::thread` moves that problem somewhere it can be *decided* rather than
//! inherited — but a dedicated thread is only half an answer, because
//! `JoinHandle::join` blocks for exactly as long as the call does. So this
//! module joins only when joining is bounded, and detaches otherwise:
//! [`InferenceWorker::shutdown`] reports which of the two it did, and a
//! detached thread is provably incapable of writing anything.
//!
//! # What this thread never holds
//!
//! **No guard of any kind.** It does not own the `IndexOwner`, it does not hold
//! a `WorkspaceLock`, and it does not call `prepare_embedding_batch`,
//! `complete_embedding_batch`, `IndexFactory` or `maintain_index`. It produces
//! an [`InferenceResult`] — the job plus its vector — on a bounded channel, and
//! the facade writes it afterwards, outside this module. An inference that ran
//! under the owner guard would hold an OS-wide exclusive lock for the length of
//! a native call.
//!
//! # Why the permit is structural
//!
//! The worker handles **one job at a time**, and the thread *is* the permit: a
//! second job cannot begin until the first `embed` returns, because the code
//! that would start it is the code that has not been reached yet.
//!
//! A soft deadline that released the permit would allow a second inference to
//! start while the first is still running, and "one worker" would become a
//! comment rather than a property. So **no timer in this module cancels a
//! call**. [`INFERENCE_SOFT_DEADLINE_MS`] bounds how long a *caller* waits for
//! a result; it never bounds the work. A caller that stops waiting **abandons
//! the batch** ([`InferenceWorker::begin_batch`]): the vector that arrives
//! afterwards is expired and is discarded, and the job stays durably pending
//! for the next pass.
//!
//! # Two things that are bounded here, and how
//!
//! **Channels.** [`InferenceWorker::submit`] writes to a `sync_channel` with
//! [`READY_QUEUE_CAPACITY`] slots and never blocks: a full queue yields
//! [`SubmitOutcome::QueueFull`], which is not a loss because the durable pending
//! set in the manifest still names the job. Finished vectors travel on a
//! bounded channel too, and the worker `try_send`s rather than blocking — a
//! worker that blocked handing back a vector nobody had asked for any more
//! would be a second way to hang.
//!
//! **Native calls.** At most one is in flight in the whole process, held by the
//! `NATIVE_CALL_PERMIT` below. That is stronger than "one per worker": a worker
//! detached by [`InferenceWorker::shutdown`] may still be inside its call while
//! a fresh worker is constructed, and the permit is what stops those two from
//! overlapping. A worker that cannot take the permit **waits**, and stops
//! waiting the moment admission is closed — so waiting is never a third way to
//! hang.
//!
//! # What is in this module
//!
//! Three layers, in increasing order of how much they do:
//!
//! * [`InferenceWorker`] — the blocking thread and nothing else.
//! * [`ReadyQueue`] — the bounded, coalescing work list, and [`mark_dirty`]’s
//!   registry behind it: who is allowed to signal a live runtime.
//! * [`MaintenanceSupervisor`] — the registered runtime itself: one per
//!   physical index, taking its own passes, recovering the durable pending set
//!   after a restart, and owning the single inference worker its drains use.
//!
//! A watcher registers a supervisor and calls [`MaintenanceSupervisor::capture_pass`]
//! on its own tick. Nothing in this module starts a thread on its own.

use super::{ConstellationError, DiagnosticCode};

/// Rapid successive saves are one scan. **A chosen project bound, not a
/// measured SLA**: fast enough that a human's second edit is folded into the
/// first scan, slow enough that a burst of atomic-save writes does not produce a
/// publish per write.
pub const DEBOUNCE_MS: u64 = 250;

/// How often a live runtime re-checks for changes it was not told about.
/// **A chosen bound, not a measurement**: it is the ceiling on how long a
/// missed event (an overflow, a lost notification, a sleep/resume) can hide a
/// change. Events are hints; this is what makes them safe to miss.
pub const FULL_SCAN_PERIOD_MS: u64 = 30_000;

/// Distinct keys that may wait for admission at once. **A chosen bound**: large
/// enough for a whole workspace to be re-embedded in one pass, small enough that
/// the durable pending map — which survives a crash — cannot grow without limit.
/// Overflow defers to the next scan; it does not drop work.
pub const READY_QUEUE_CAPACITY: usize = 64;

/// After the model is reported unavailable, do not re-probe for this long.
/// **A chosen bound**: re-probing a missing cache on every event turns a clear
/// `unavailable` into a busy loop. A download is not waiting on this.
pub const MODEL_UNAVAILABLE_COOLDOWN_MS: u64 = 60_000;

/// How long a caller waits for one inference before giving up on the *result*.
/// **A chosen bound, not a timeout on the work**: see the module docs. It does
/// not release the permit and cannot cancel a native call; a late result is
/// discarded by the caller, and the inference still ran to completion.
pub const INFERENCE_SOFT_DEADLINE_MS: u64 = 120_000;

/// Why a drain gave up waiting for vectors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrainGiveUp {
    /// The host is stopping. Nothing further is admitted, and the batch that
    /// was in flight is abandoned rather than waited on.
    Stopping,
    /// The control scan is owed. The drain yields so the host can run it
    /// inside its [`FULL_SCAN_PERIOD_MS`] bound, and picks the remaining jobs
    /// up on a later pass.
    ScanDue,
}

impl DrainGiveUp {
    fn as_str(self) -> &'static str {
        match self {
            Self::Stopping => "the maintenance host is stopping",
            Self::ScanDue => "the control scan is due",
        }
    }
}

/// The host's answer to "must this drain stop waiting?".
///
/// # Why this exists instead of a deadline
///
/// The drain's soft deadline ([`INFERENCE_SOFT_DEADLINE_MS`]) answers "how long
/// may a caller wait for one batch". It does **not** answer "may the host keep
/// the loop out of its control scan while a native call hangs", and it cannot:
/// the deadline is 120 s and the scan period is 30 s, so a hung encoder
/// delays a stop signal and the control scan by two minutes.
///
/// This type is polled by the drain **between polls** — never by a timer
/// inside the model path — so yielding is decided by the party that owns the
/// loop, and a give-up is a reported fact rather than a silent expiry.
///
/// # A yield is not a failure
///
/// The batch in flight is abandoned ([`InferenceWorker::begin_batch`]), so a
/// vector that arrives afterwards is discarded where it is recognised, and the
/// jobs stay durably pending. Nothing is recorded `failed`, because nothing
/// failed: a model that is merely slow must not burn retry attempts every time
/// the control scan comes due.
#[derive(Debug, Clone, Default)]
pub struct DrainControl {
    inner: std::sync::Arc<DrainControlInner>,
}

#[derive(Debug, Default)]
struct DrainControlInner {
    /// Set by the host when it is shutting down. Never cleared: a stopped host
    /// does not start admitting again within one process.
    stop: std::sync::atomic::AtomicBool,
    /// The instant the control scan becomes owed, if it has been armed.
    scan_due: std::sync::Mutex<Option<std::time::Instant>>,
    /// Wakes a waiter when a stop is requested, so the response does not wait
    /// for the drain's next poll interval.
    notify: tokio::sync::Notify,
}

impl DrainControl {
    /// A control nobody has asked anything of.
    ///
    /// For a caller with no loop to serve — a one-shot CLI drain, a git hook —
    /// which is therefore bounded by the soft deadline alone until it arms the
    /// control itself or requests a stop.
    pub fn never() -> Self {
        Self::default()
    }

    /// Ask every drain sharing this control to stop waiting.
    ///
    /// Idempotent, and observable from another task: this is what
    /// [`MaintenanceHost::run_until`] calls while a pass is in flight.
    pub fn request_stop(&self) {
        self.inner
            .stop
            .store(true, std::sync::atomic::Ordering::SeqCst);
        // `notify_one` and not `notify_waiters`: it stores a permit when
        // nothing is waiting yet, so a stop that lands between a waiter's
        // flag check and its await cannot be lost. There is one waiter — the
        // host loop — and the drain polls the flag rather than waiting.
        self.inner.notify.notify_one();
    }

    /// Whether a stop has been requested.
    pub fn stop_requested(&self) -> bool {
        self.inner.stop.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Yield once the control scan is owed at `at`.
    ///
    /// `None` disarms. Arming an instant already in the past yields at the
    /// drain's next poll, which is the correct answer for a scan that is
    /// already overdue.
    pub fn arm_scan(&self, at: std::time::Instant) {
        if let Ok(mut slot) = self.inner.scan_due.lock() {
            *slot = Some(at);
        }
    }

    /// Stop yielding for the control scan.
    pub fn disarm_scan(&self) {
        if let Ok(mut slot) = self.inner.scan_due.lock() {
            *slot = None;
        }
    }

    /// Whether a drain must stop waiting, and why.
    ///
    /// A stop outranks a due scan: when both are true the process is going
    /// away and there is no scan left to run.
    pub fn give_up(&self) -> Option<DrainGiveUp> {
        if self.stop_requested() {
            return Some(DrainGiveUp::Stopping);
        }
        let due = self
            .inner
            .scan_due
            .lock()
            .ok()
            .and_then(|slot| *slot)
            .is_some_and(|at| std::time::Instant::now() >= at);
        if due {
            Some(DrainGiveUp::ScanDue)
        } else {
            None
        }
    }

    /// The reason as a caller reports it.
    pub fn describe(reason: DrainGiveUp) -> &'static str {
        reason.as_str()
    }

    /// Resolve once a stop has been requested.
    ///
    /// Used by the host to wake out of its wait between ticks; the drain
    /// itself polls [`Self::give_up`] between result polls, because its wait
    /// is a bounded sleep and not a select over this.
    pub async fn cancelled(&self) {
        loop {
            let notified = self.inner.notify.notified();
            if self.stop_requested() {
                return;
            }
            notified.await;
        }
    }
}

/// One inference request.
///
/// Carries the **job**, not just a key. A key alone cannot tell a late result
/// from a current one: two admissions for the same canonical key under
/// different profiles produce two vectors with one name, and only the job says
/// which one this is. A worker that returned `(key, vector)` would have left
/// the caller to re-derive that association from a key it matched by hand,
/// which is precisely the rebinding this protocol removed.
pub struct InferenceJob {
    /// The job this vector is being computed for.
    pub job: super::jobs::EmbeddingJob,
    /// The exact text the vector must be computed from — the string this
    /// job's `input_digest` was computed over. Never re-read from the file.
    pub input_text: String,
}

/// One computed vector, carrying the job it was computed for.
pub struct InferenceResult {
    /// The job this vector answers to. Returned unchanged, so a result cannot
    /// be re-pointed at a different job on the way back.
    pub job: super::jobs::EmbeddingJob,
    pub vector: Vec<f32>,
}

impl InferenceJob {
    /// The key this job's vector belongs to.
    pub fn canonical_key(&self) -> &str {
        &self.job.canonical_key
    }
}

impl InferenceResult {
    /// The key this vector answers to.
    pub fn canonical_key(&self) -> &str {
        &self.job.canonical_key
    }
}

/// What [`InferenceWorker::submit`] did with a job.
///
/// A submission that returns an outcome has been **told what happened to it**.
/// The previous signature returned nothing and discarded the channel's error,
/// so a worker whose thread had already exited looked exactly like a worker
/// that had queued the job: the caller went on to wait for a vector that could
/// never arrive, and the drain only ended because its deadline expired.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubmitOutcome {
    /// The job is on the bounded channel and the worker will run it.
    Queued,
    /// The bounded queue is full. **Not a loss**: the job stays durably
    /// pending in the manifest and the next full scan re-admits it.
    QueueFull,
    /// The worker thread has exited. No retry against this worker can succeed;
    /// the caller must construct a new one after its cooldown.
    WorkerStopped,
}

/// How [`InferenceWorker::shutdown`] ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownOutcome {
    /// The thread was joined: its function returned, so the call it may have
    /// been inside has returned. No inference is running anywhere.
    Joined,
    /// A native call was still in flight, so the thread was **detached** — the
    /// `JoinHandle` was dropped without joining.
    ///
    /// This is not a failure and not a silent success. It says exactly what it
    /// is: the process may exit while that thread is still inside native code.
    /// What the detached thread provably cannot do is publish — it holds no
    /// publisher capability, and the channels it would report on are dropped
    /// with this handle, so its next send fails and it stops. It also cannot
    /// start another call: admission is closed, so nothing else will be queued
    /// behind it.
    ///
    /// A caller that must report this state — rather than accept it — should
    /// log it. Reporting is what makes a hung model visible; a shutdown that
    /// returned `Joined` here would be claiming a call finished that has not.
    Detached,
}

/// A handle on one batch of jobs a worker has been asked to compute.
///
/// Each batch is numbered, and the number is what makes a late result
/// **discardable rather than fatal**. A caller that stops waiting no longer
/// leaves the previous batch's answers to be mistaken for the current one's:
/// [`InferenceWorker::begin_batch`] moves the worker to a new batch, and any
/// vector that arrives stamped with an older batch is dropped by
/// [`InferenceWorker::try_result`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BatchToken(pub u64);

/// The process-wide permit for native inference.
///
/// **Not** a counter that could disagree with reality: it is set immediately
/// before a call and cleared immediately after, and a worker that cannot take
/// it *waits* rather than proceeding. That is what keeps a detached worker
/// still inside its call from overlapping with a fresh worker's first call.
///
/// The wait is bounded by the caller's gate, not by time: the acquiring loop
/// re-reads that gate on every pass and gives up when the worker has been shut
/// down. There is no timeout that could release the permit while a call is
/// still running.
#[cfg(feature = "semantic-search")]
static NATIVE_CALL_PERMIT: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// Held for the duration of one native call; releases the permit on drop,
/// including on panic.
#[cfg(feature = "semantic-search")]
struct NativeCallPermit;

#[cfg(feature = "semantic-search")]
impl NativeCallPermit {
    /// Take the permit, waiting while another call holds it.
    ///
    /// Returns `None` when `gate` was closed first, so a shutdown never waits
    /// behind a call it has already given up on.
    fn acquire(gate: &CallGate) -> Option<Self> {
        while NATIVE_CALL_PERMIT.compare_exchange(
            false,
            true,
            std::sync::atomic::Ordering::SeqCst,
            std::sync::atomic::Ordering::SeqCst,
        ) != Ok(false)
        {
            if gate.is_closed() {
                return None;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        Some(Self)
    }
}

#[cfg(feature = "semantic-search")]
impl Drop for NativeCallPermit {
    fn drop(&mut self) {
        NATIVE_CALL_PERMIT.store(false, std::sync::atomic::Ordering::SeqCst);
    }
}

/// One worker's admission state and its in-call state, as **one** atomic word.
///
/// They are one word because they are one decision, and separating them
/// reintroduced the race they were meant to settle: a shutdown that read
/// "not in a call" and then blocked on `join` would still be waiting on a call
/// the worker started in the window between that read and the join.
///
/// The two bits are independent, and both operations are single atomic RMWs, so
/// neither can read a state the other has already overtaken:
///
/// * shutdown's `fetch_or` lands first → the worker's `compare_exchange`
///   from `Idle` fails, so it never enters a call, and joining is bounded.
/// * the worker's exchange lands first → shutdown's `fetch_or` returns a
///   previous value with the in-call bit set, so it detaches rather than joins.
///
/// There is no third ordering, because there is only one word: a separate
/// "in flight" flag read before a join would leave exactly the window this
/// structure exists to close.
#[cfg(feature = "semantic-search")]
struct CallGate(std::sync::Arc<std::sync::atomic::AtomicU8>);

/// Not in a call, still admitting.
#[cfg(feature = "semantic-search")]
const GATE_IDLE: u8 = 0;
/// Inside a native call, still admitting.
#[cfg(feature = "semantic-search")]
const GATE_IN_CALL: u8 = 0b01;
/// Admission closed. The thread will not enter another call.
#[cfg(feature = "semantic-search")]
const GATE_CLOSED: u8 = 0b10;

#[cfg(feature = "semantic-search")]
impl CallGate {
    fn new() -> Self {
        Self(std::sync::Arc::new(std::sync::atomic::AtomicU8::new(
            GATE_IDLE,
        )))
    }

    fn is_closed(&self) -> bool {
        self.0.load(std::sync::atomic::Ordering::SeqCst) & GATE_CLOSED != 0
    }

    /// Whether a call is running right now.
    pub fn in_call(&self) -> bool {
        self.0.load(std::sync::atomic::Ordering::SeqCst) & GATE_IN_CALL != 0
    }

    /// Enter a call. `false` when the gate is closed, which is the worker's cue
    /// to leave.
    ///
    /// The exchange names the exact state it expects, so a worker cannot enter a
    /// call in the window after a shutdown closed admission.
    fn enter_call(&self) -> bool {
        self.0.compare_exchange(
            GATE_IDLE,
            GATE_IN_CALL,
            std::sync::atomic::Ordering::SeqCst,
            std::sync::atomic::Ordering::SeqCst,
        ) == Ok(GATE_IDLE)
    }

    /// Leave the call, clearing the in-call bit and keeping the closed bit —
    /// which is why it is a `fetch_and` and not a store: a store back to
    /// `GATE_IDLE` would reopen admission the shutdown had already closed.
    fn leave_call(&self) {
        self.0
            .fetch_and(!GATE_IN_CALL, std::sync::atomic::Ordering::SeqCst);
    }

    /// Close admission. Returns whether a call was in flight at the moment
    /// admission closed — the fact that decides join versus detach.
    ///
    /// `fetch_or` rather than a compare-exchange: setting the closed bit is
    /// idempotent and does not disturb the in-call bit, so the previous value
    /// is exactly the state the worker was in when admission shut.
    fn close(&self) -> bool {
        self.0
            .fetch_or(GATE_CLOSED, std::sync::atomic::Ordering::SeqCst)
            & GATE_IN_CALL
            != 0
    }
}

#[cfg(feature = "semantic-search")]
#[derive(PartialEq, Eq)]
struct QueryEngineKey {
    cache_dir: std::path::PathBuf,
    model_name: String,
    profile_id: Option<String>,
    body_prefix_chars: usize,
}

#[cfg(feature = "semantic-search")]
struct QueryRequest {
    key: QueryEngineKey,
    text: String,
    gate: CallGate,
    reply: tokio::sync::oneshot::Sender<Result<(String, Vec<f32>), ConstellationError>>,
}

#[cfg(feature = "semantic-search")]
struct CloseQueryOnDrop(CallGate);

#[cfg(feature = "semantic-search")]
impl Drop for CloseQueryOnDrop {
    fn drop(&mut self) {
        self.0.close();
    }
}

/// Embed a query on one cached-only, process-wide dedicated thread.
///
/// Shares the maintenance inference permit. Cancelling the response closes
/// admission but does not release a running call's permit or block process exit.
#[cfg(feature = "semantic-search")]
pub async fn cached_query_embedding(
    model_name: &str,
    profile_id: Option<&str>,
    body_prefix_chars: usize,
    text: &str,
) -> Result<(String, Vec<f32>), ConstellationError> {
    static WORKER: std::sync::LazyLock<Result<std::sync::mpsc::SyncSender<QueryRequest>, String>> =
        std::sync::LazyLock::new(|| {
            let (sender, requests) = std::sync::mpsc::sync_channel::<QueryRequest>(1);
            std::thread::Builder::new()
                .name("forgeplan-query-inference".to_string())
                .spawn(move || {
                    let mut cached: Option<(QueryEngineKey, super::jobs::CachedQueryEngine)> = None;
                    while let Ok(request) = requests.recv() {
                        if request.reply.is_closed() {
                            continue;
                        }
                        let Some(_permit) = NativeCallPermit::acquire(&request.gate) else {
                            continue;
                        };
                        if !request.gate.enter_call() {
                            continue;
                        }
                        if cached.as_ref().is_none_or(|(key, _)| key != &request.key) {
                            match super::jobs::open_cached_query_engine_in(
                                &request.key.cache_dir,
                                &request.key.model_name,
                                request.key.profile_id.as_deref(),
                                request.key.body_prefix_chars,
                            ) {
                                Ok(engine) => cached = Some((request.key, engine)),
                                Err(error) => {
                                    request.gate.leave_call();
                                    let _ = request.reply.send(Err(error));
                                    continue;
                                }
                            }
                        }
                        let (_, engine) = cached.as_mut().expect("a matching engine was loaded");
                        let result = engine
                            .embed(&request.text)
                            .map(|vector| (engine.profile_id.clone(), vector));
                        request.gate.leave_call();
                        let _ = request.reply.send(result);
                    }
                })
                .map(|_| sender)
                .map_err(|error| error.to_string())
        });
    let unavailable = |detail: String| ConstellationError::ModelUnavailable { detail };
    let sender = WORKER
        .as_ref()
        .map_err(|error| unavailable(format!("cannot start cached query worker: {error}")))?;
    let gate = CallGate::new();
    let _close = CloseQueryOnDrop(CallGate(std::sync::Arc::clone(&gate.0)));
    let (reply, response) = tokio::sync::oneshot::channel();
    sender
        .try_send(QueryRequest {
            key: QueryEngineKey {
                cache_dir: crate::embed::resolve_cache_dir(),
                model_name: model_name.to_string(),
                profile_id: profile_id.map(str::to_string),
                body_prefix_chars,
            },
            text: text.to_string(),
            gate,
            reply,
        })
        .map_err(|error| unavailable(format!("cached query worker unavailable: {error}")))?;
    tokio::time::timeout(
        std::time::Duration::from_millis(INFERENCE_SOFT_DEADLINE_MS),
        response,
    )
    .await
    .map_err(|_| unavailable("cached query inference exceeded its response deadline".to_string()))?
    .map_err(|_| unavailable("cached query worker stopped without a result".to_string()))?
}

#[cfg(all(test, feature = "semantic-search"))]
mod cached_query_tests {
    #[tokio::test(flavor = "current_thread")]
    async fn an_unsupported_query_model_reports_unavailability_without_downloading() {
        let error = super::cached_query_embedding(
            "unsupported-query-model",
            None,
            super::super::jobs::DEFAULT_BODY_PREFIX_CHARS,
            "query",
        )
        .await
        .expect_err("unsupported models must not become usable query engines");
        assert_eq!(error.code(), super::DiagnosticCode::ModelUnavailable);
    }
}

/// Set when the worker refuses to construct, or could not construct its engine.
///
/// Reported, never panicked: a missing model cache is `unavailable`, which is a
/// state a caller can carry and report, not a crash.
pub fn refuse_without_semantic_search() -> ConstellationError {
    ConstellationError::ModelUnavailable {
        detail: "this build was compiled without the `semantic-search` feature; no inference \
                 worker can be constructed and no thread is spawned."
            .to_string(),
    }
}

#[cfg(feature = "semantic-search")]
mod imp {
    use std::sync::mpsc::{Receiver, SyncSender, TryRecvError, TrySendError};

    use super::*;
    use crate::embed::Embedder;

    /// Slots for jobs handed to the worker thread.
    ///
    /// The same number as the ready queue, because the two bound the same
    /// thing: what one process may be holding in memory on behalf of an
    /// embedding pass. **Unbounded here would be the same defect as an
    /// unbounded pending map** — the memory would simply move from disk to the
    /// heap, where nothing survives a restart to give it a second chance.
    const JOB_CHANNEL_CAPACITY: usize = READY_QUEUE_CAPACITY;

    /// Slots for finished vectors on the way back.
    ///
    /// A worker that could not hand back a vector would block, and a blocked
    /// worker is a worker that cannot be shut down. So the channel is bounded
    /// and the sender never blocks: see [`Self::try_result`].
    const RESULT_CHANNEL_CAPACITY: usize = READY_QUEUE_CAPACITY;

    /// Slots for reported failures. Bounded for the same reason as the rest:
    /// an unbounded channel on an error path is still a memory path.
    const FAILURE_CHANNEL_CAPACITY: usize = 8;

    /// One queued job, stamped with the batch that issued it.
    ///
    /// The stamp is what makes a late answer **discardable**. Without it, a
    /// vector computed for an abandoned batch is indistinguishable from one
    /// computed for the batch now running, and the caller — which did not
    /// issue the job it is holding — is right to treat that as an error. With
    /// it, the answer is simply expired and is dropped where it is recognised.
    struct QueuedJob {
        batch: BatchToken,
        job: InferenceJob,
    }

    /// One finished vector, stamped with the batch that issued its job.
    struct StampedResult {
        batch: BatchToken,
        result: InferenceResult,
    }

    /// A handle to the single inference thread.
    ///
    /// Owns the thread and the sender that keeps it fed. Dropping the handle
    /// without calling [`Self::shutdown`] detaches the thread: the sender drops,
    /// the receiver disconnects, and the thread ends after the job in flight.
    ///
    /// [`Self::shutdown`] joins **only when joining is bounded** and detaches
    /// otherwise, reporting which it did in a [`ShutdownOutcome`]. It never
    /// blocks on a call it cannot cancel.
    pub struct InferenceWorker {
        jobs: SyncSender<QueuedJob>,
        results: Receiver<StampedResult>,
        failures: Receiver<String>,
        handle: Option<std::thread::JoinHandle<Result<(), String>>>,
        /// Admission and in-call state as one atomic word, so "should I join?"
        /// is a decision the worker and the shutdown make against the same
        /// value rather than against two flags that can be read out of step.
        /// See [`CallGate`].
        gate: CallGate,
        /// The batch any submission made now belongs to.
        batch: std::sync::Arc<std::sync::atomic::AtomicU64>,
        /// Set by the worker thread as it returns, including on panic. A
        /// supervisor that parks a worker whose thread has already exited is
        /// parking a handle that can never answer again, so this is what
        /// decides whether a worker may be reused.
        stopped: std::sync::Arc<std::sync::atomic::AtomicBool>,
    }

    /// Whether the channel has something, is merely empty, or is gone.
    ///
    /// The distinction matters and is **not** collapsed: "no work has
    /// finished yet" and "the worker has stopped" are different states, and
    /// a caller that cannot tell them apart reads a dead worker as a slow
    /// one. `Disconnected` is how a thread that failed is observed without
    /// polling `try_failure` to find out why.
    #[derive(Debug, PartialEq, Eq)]
    pub enum Poll<T> {
        Ready(T),
        Empty,
        Disconnected,
    }

    /// The worker's shared state, so the spawn paths do not each assemble their
    /// own set.
    struct Flags {
        gate: CallGate,
        batch: std::sync::Arc<std::sync::atomic::AtomicU64>,
        stopped: std::sync::Arc<std::sync::atomic::AtomicBool>,
    }

    impl Flags {
        fn new() -> Self {
            Self {
                gate: CallGate::new(),
                batch: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(1)),
                stopped: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            }
        }
    }

    /// Marks the thread finished on **every** exit, including a panic.
    ///
    /// Without this, a panicking worker leaves `stopped` false and a
    /// supervisor that checks it will park a handle whose thread is gone —
    /// the exact defect that made a failed worker permanent.
    struct StoppedOnExit(std::sync::Arc<std::sync::atomic::AtomicBool>);

    impl Drop for StoppedOnExit {
        fn drop(&mut self) {
            self.0.store(true, std::sync::atomic::Ordering::SeqCst);
        }
    }

    impl InferenceWorker {
        /// Spawn the worker, constructing the engine **on the worker thread**.
        ///
        /// Construction is on-thread so the engine is created, used and dropped
        /// in the same place and never appears on any other thread's stack. It
        /// is `Send`, so moving one in is also possible; doing it here keeps the
        /// engine's lifetime identical to the thread's.
        ///
        /// **This is the explicit path and it may download.** `Embedder::new()`
        /// fetches weights on first use. Only a command whose whole purpose is
        /// to obtain vectors — `forgeplan embed`, `forgeplan index
        /// `--embed-profile` — may call it. Every automatic caller wants
        /// [`Self::spawn_resolved`], which cannot.
        ///
        /// An engine that cannot be built is returned as
        /// [`ConstellationError::ModelUnavailable`].
        pub fn spawn() -> Result<Self, ConstellationError> {
            Self::spawn_with(|| Embedder::new())
        }

        /// Spawn a worker that will **never** download.
        ///
        /// The constructor every automatic path — watch, serve, the git hooks
        /// — must use. [`Self::spawn`] calls `Embedder::new()`, which fetches
        /// weights on first use; a watch event firing that would turn every
        /// filesystem notification into a multi-gigabyte download. Here a
        /// missing cache is `unavailable` and the rows stay `pending`, which is
        /// the honest state.
        pub fn spawn_cached(model_name: &str) -> Result<Self, ConstellationError> {
            let model_name = model_name.to_string();
            Self::spawn_with(move || Embedder::with_cached_model(&model_name))
        }

        /// Spawn a worker from a profile that has **already** been resolved.
        ///
        /// This is the constructor every automatic path uses.
        /// [`Self::spawn_cached`] looks the snapshot up for itself, and the
        /// caller that resolved a profile has already looked it up once — so
        /// between the two reads `refs/main` can move, and the caller would
        /// advertise one revision while the encoder loaded another. Both are
        /// 1024 unit-length float vectors; nothing downstream would notice.
        ///
        /// Here the root is the one the profile's checksums were measured from,
        /// by construction: there is exactly one lookup, and the engine opens
        /// the directory the caller already proved complete.
        ///
        /// It still cannot download — [`crate::embed::Embedder::from_snapshot`]
        /// opens files that `ResolvedProfile::open` already hashed.
        ///
        /// A root whose files no longer load does not fail here — the engine is
        /// built lazily on the first job, so the failure is reported through
        /// the worker's failure channel and stops its thread, exactly as it
        /// does for [`Self::spawn`]. The job row stays `pending`; a vector is
        /// never published against an engine that failed to open.
        pub fn spawn_resolved(
            model_name: &str,
            resolved: &crate::constellation::jobs::ResolvedProfile,
        ) -> Result<Self, ConstellationError> {
            let snapshot_root = resolved.snapshot_root.clone();
            let model_name = model_name.to_string();
            Self::spawn_with(move || Embedder::from_snapshot(&snapshot_root, &model_name))
        }

        /// Spawn the thread around one engine constructor.
        ///
        /// The engine is built lazily on the first job so `spawn` reports
        /// thread-creation success; a construction failure is reported through
        /// the failure channel and stops the thread, rather than being panicked
        /// on or silently retried in a loop.
        fn spawn_with<F>(build: F) -> Result<Self, ConstellationError>
        where
            F: FnOnce() -> anyhow::Result<Embedder> + Send + 'static,
        {
            let mut build = Some(build);
            let mut engine: Option<Embedder> = None;
            Self::spawn_thread("forgeplan-inference", move |job| {
                // The engine exists only inside this closure, which lives only
                // inside the worker thread: it is created, used and dropped in
                // the same place and never appears on another thread's stack.
                // The closure is `FnMut`, so holding it across calls needs no
                // interior mutability and no `thread_local`.
                if engine.is_none() {
                    // Built on first use; a failure is reported and stops the
                    // thread rather than spinning on a model that cannot load.
                    match build.take().expect("one engine per thread")() {
                        Ok(e) => engine = Some(e),
                        Err(e) => {
                            return Err(format!(
                                "the inference engine could not be constructed: {e}"
                            ));
                        }
                    }
                }
                let engine = engine.as_mut().expect("engine constructed above");
                engine
                    .embed(&job.input_text)
                    .map_err(|e| format!("inference failed for {}: {e}", job.job.canonical_key))
            })
        }

        /// Spawn the thread around one compute callable.
        ///
        /// **One implementation, two constructors.** [`Self::spawn`] /
        /// [`Self::spawn_cached`] supply an engine; [`Self::spawn_with_engine`]
        /// supplies a closure. Everything that carries a security property — the
        /// bounded channels, the batch stamping, the process-wide call permit,
        /// the stopped flag — lives *here*, so a second thread body could not
        /// implement those properties differently from the first. Two copies of
        /// a concurrency protocol is one more copy than there should be.
        fn spawn_thread<F>(name: &str, mut compute: F) -> Result<Self, ConstellationError>
        where
            F: FnMut(&InferenceJob) -> Result<Vec<f32>, String> + Send + 'static,
        {
            let (jobs_tx, jobs_rx) =
                std::sync::mpsc::sync_channel::<QueuedJob>(JOB_CHANNEL_CAPACITY);
            let (result_tx, result_rx) =
                std::sync::mpsc::sync_channel::<StampedResult>(RESULT_CHANNEL_CAPACITY);
            let (fail_tx, fail_rx) =
                std::sync::mpsc::sync_channel::<String>(FAILURE_CHANNEL_CAPACITY);
            let flags = Flags::new();
            let thread_gate = CallGate(std::sync::Arc::clone(&flags.gate.0));
            let thread_batch = std::sync::Arc::clone(&flags.batch);
            let thread_stopped = std::sync::Arc::clone(&flags.stopped);

            let handle = std::thread::Builder::new()
                .name(name.to_string())
                .spawn(move || -> Result<(), String> {
                    // Set on every exit, including a panic: a worker that
                    // panicked has no thread left to join, and a supervisor
                    // that does not learn this will park a dead handle.
                    let _stopped = StoppedOnExit(thread_stopped);
                    let gate = thread_gate;
                    while let Ok(queued) = jobs_rx.recv() {
                        // A job from an abandoned batch is not run. Its caller
                        // stopped waiting, so a vector computed now could only
                        // be thrown away — and running it would keep the native
                        // permit busy for work nobody wants.
                        if queued.batch.0 != thread_batch.load(std::sync::atomic::Ordering::SeqCst)
                        {
                            continue;
                        }
                        // Waiting for the process-wide permit is not a native
                        // call. Shutdown may close this gate and join the waiter.
                        let Some(permit) = NativeCallPermit::acquire(&gate) else {
                            break;
                        };
                        if queued.batch.0 != thread_batch.load(std::sync::atomic::Ordering::SeqCst)
                        {
                            drop(permit);
                            continue;
                        }
                        // Only native-call entry competes with shutdown's
                        // atomic close; a close that wins prevents computation.
                        if !gate.enter_call() {
                            drop(permit);
                            break;
                        }
                        let computed = {
                            let _permit = permit;
                            compute(&queued.job)
                        };
                        gate.leave_call();
                        match computed {
                            Ok(vector) => {
                                // `try_send`, never `send`: this thread must
                                // stay joinable. A worker blocked handing back
                                // a vector nobody wants is a worker whose
                                // shutdown cannot finish.
                                let stamped = StampedResult {
                                    batch: queued.batch,
                                    result: InferenceResult {
                                        job: queued.job.job,
                                        vector,
                                    },
                                };
                                match result_tx.try_send(stamped) {
                                    Ok(()) => {}
                                    Err(TrySendError::Full(_)) => {
                                        // Every slot is taken and the owner is
                                        // gone. The work is not lost — the job
                                        // stays durably pending and the next
                                        // scan re-admits it — so this is a stop,
                                        // not a retry loop against a full
                                        // channel nobody is draining.
                                        break;
                                    }
                                    Err(TrySendError::Disconnected(_)) => break,
                                }
                            }
                            Err(detail) => {
                                // Report and stop. Returning would close the
                                // channel with no explanation; silently
                                // continuing would spin on a model that cannot
                                // load.
                                let _ = fail_tx.try_send(detail);
                                return Ok(());
                            }
                        }
                    }
                    Ok(())
                })
                .map_err(|e| ConstellationError::ModelUnavailable {
                    detail: format!("the inference thread could not be spawned: {e}"),
                })?;

            Ok(Self {
                jobs: jobs_tx,
                results: result_rx,
                failures: fail_rx,
                handle: Some(handle),
                gate: flags.gate,
                batch: flags.batch,
                stopped: flags.stopped,
            })
        }

        /// Open a new batch and abandon the previous one.
        ///
        /// Call this when a wait ends without the caller getting what it
        /// asked for — a soft deadline, or a worker that stopped. Everything
        /// still queued under the old batch will be skipped by the worker, and
        /// any vector that arrives for it is **dropped here instead of being
        /// handed to a caller that never issued it**.
        ///
        /// That is the difference between this and the previous behaviour, where
        /// a timed-out drain left its jobs queued, the next drain submitted
        /// more, and the first late answer arrived carrying a token the new
        /// caller had never seen — which it then had to treat as a fatal
        /// integrity error. A late answer is not an integrity error. It is an
        /// answer to a question nobody is asking any more.
        ///
        /// Returns the new batch's token, which the caller may keep for
        /// diagnostics; submission does not require it.
        pub fn begin_batch(&self) -> BatchToken {
            let next = self.batch.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
            BatchToken(next)
        }

        /// The batch submissions made now belong to.
        pub fn current_batch(&self) -> BatchToken {
            BatchToken(self.batch.load(std::sync::atomic::Ordering::SeqCst))
        }

        /// Take the next finished vector, if one is ready. Never blocks.
        ///
        /// A vector belonging to an **abandoned** batch is discarded rather
        /// than returned. Discarding it is not hiding anything: the caller that
        /// wanted it stopped waiting, the job it was computed for is still
        /// durably pending, and the next pass recomputes it from the canonical
        /// capture. Returning it would hand a caller a result for a job it
        /// never issued, which is the failure this discards.
        pub fn try_result(&self) -> Poll<InferenceResult> {
            let current = self.batch.load(std::sync::atomic::Ordering::SeqCst);
            loop {
                match self.results.try_recv() {
                    Ok(stamped) if stamped.batch.0 != current => continue,
                    Ok(stamped) => return Poll::Ready(stamped.result),
                    Err(TryRecvError::Empty) => return Poll::Empty,
                    Err(TryRecvError::Disconnected) => return Poll::Disconnected,
                }
            }
        }

        /// Take the next reported failure, if any. Never blocks.
        ///
        /// An engine that cannot be constructed, or an `embed` that errors, is
        /// **reported here and the thread stops**. It does not panic, and it
        /// does not drop the job silently: a caller that read only
        /// [`Self::try_result`] would otherwise see an empty channel and
        /// conclude nothing had been submitted.
        pub fn try_failure(&self) -> Poll<String> {
            match self.failures.try_recv() {
                Ok(d) => Poll::Ready(d),
                Err(TryRecvError::Empty) => Poll::Empty,
                Err(TryRecvError::Disconnected) => Poll::Disconnected,
            }
        }

        /// Whether this worker's thread has returned.
        ///
        /// A stopped worker cannot answer anything, so a caller that parks one
        /// — to reuse its engine — is parking a handle that will silently
        /// reject every submission until the process restarts. Check this
        /// before reusing.
        pub fn is_stopped(&self) -> bool {
            self.stopped.load(std::sync::atomic::Ordering::SeqCst)
        }

        /// Whether a native call is running right now on this worker's thread.
        pub fn is_in_call(&self) -> bool {
            self.gate.in_call()
        }

        /// Queue one inference. Does not wait, and never blocks.
        ///
        /// Reports what happened to the job. The previous signature returned
        /// nothing and threw the channel's error away, so a worker whose thread
        /// had already exited was indistinguishable from one that had accepted
        /// the job — and the caller's only symptom was a drain that ran to its
        /// deadline with nothing to show.
        pub fn submit(&self, job: InferenceJob) -> SubmitOutcome {
            let batch = BatchToken(self.batch.load(std::sync::atomic::Ordering::SeqCst));
            match self.jobs.try_send(QueuedJob { batch, job }) {
                Ok(()) => SubmitOutcome::Queued,
                // Not a loss: the job stays durably pending in the manifest and
                // the next full scan re-admits it. This is the same bargain
                // `ReadyQueue::admit` makes, on the same bound.
                Err(TrySendError::Full(_)) => SubmitOutcome::QueueFull,
                Err(TrySendError::Disconnected(_)) => SubmitOutcome::WorkerStopped,
            }
        }

        /// A worker whose "engine" is a closure under test.
        ///
        /// Gated on **both** `semantic-search` and `test-helpers`, so no
        /// production binary carries it. It is a second constructor, not a flag
        /// threaded through [`Self::spawn`]: a `bool` would be a production
        /// affordance pretending to be a test one.
        ///
        /// The bounds are enforced by the signature alone — `FnMut` because a
        /// real engine mutates during inference, `Send + 'static` because the
        /// closure moves onto the worker thread.
        #[cfg(feature = "test-helpers")]
        pub fn spawn_with_engine<F>(engine: F) -> Result<Self, ConstellationError>
        where
            F: FnMut(&str) -> Result<Vec<f32>, String> + Send + 'static,
        {
            let mut engine = engine;
            Self::spawn_thread("forgeplan-inference-test", move |job| {
                engine(&job.input_text)
                    .map_err(|e| format!("inference failed for {}: {e}", job.job.canonical_key))
            })
        }

        /// Stop admission, drop the sender so the receiver disconnects, and
        /// then either **join** or **detach** — decided by whether a native call
        /// is actually in flight.
        ///
        /// # Why the decision is made here rather than by the caller
        ///
        /// `JoinHandle::join` blocks for exactly as long as the call does. A
        /// caller that knows its call finished may join; a caller that cannot
        /// know that — a watcher whose model call hung, an index command whose
        /// deadline expired — cannot, and joining anyway makes process shutdown
        /// unbounded. That was the whole defect: the join was unconditional, so
        /// the guarantee it offered ("this call is finished") was one the
        /// shutdown could not afford to wait for.
        ///
        /// The worker thread is the only party that knows whether it is between
        /// calls or inside one, and it says so through [`CallGate`] — the same
        /// atomic word admission closes through, so the two cannot disagree.
        /// Reading a separate "in flight" flag and then joining would leave a
        /// window in which the worker starts a call after the read. There is no
        /// timer anywhere in this path: a deadline that could expire mid-join
        /// would reintroduce the same unbounded wait by a different route.
        ///
        /// # What a detached thread provably cannot do
        ///
        /// It cannot publish. Detachment happens after supervised storage
        /// commits have finished, so nothing is left that the thread could
        /// write to, and dropping this handle drops the channels it would report
        /// on — its next send fails and it stops. It also cannot start another
        /// call: admission is closed, and [`NATIVE_CALL_PERMIT`] is
        /// process-wide, so even a caller that immediately constructs a fresh
        /// worker gets a worker that *waits* rather than a second concurrent
        /// native call.
        pub fn shutdown(mut self) -> Result<ShutdownOutcome, ConstellationError> {
            // Closing admission and learning whether a call was in flight are
            // one atomic step, so the answer cannot be overtaken by a worker
            // entering a call in between. That is what makes the join below
            // bounded: if `in_flight` is false, the worker provably will not
            // start another call, and leaving is all it has left to do.
            let in_flight = self.gate.close();
            // Replaced with a sender whose receiver is already dropped: the old
            // sender goes with it, so the worker's `recv` disconnects and an
            // idle thread leaves without waiting to be told.
            let (dead_tx, dead_rx) = std::sync::mpsc::sync_channel::<QueuedJob>(0);
            drop(dead_rx);
            self.jobs = dead_tx;
            let Some(handle) = self.handle.take() else {
                return Ok(ShutdownOutcome::Joined);
            };
            if in_flight {
                // Dropping the handle detaches. The thread finishes its call,
                // fails to send the vector anywhere, and ends.
                drop(handle);
                return Ok(ShutdownOutcome::Detached);
            }
            match handle.join() {
                Ok(Ok(())) => Ok(ShutdownOutcome::Joined),
                Ok(Err(detail)) => Err(ConstellationError::ModelUnavailable { detail }),
                Err(_) => Err(ConstellationError::ModelUnavailable {
                    detail: "the inference thread panicked".to_string(),
                }),
            }
        }
    }
}

/// The non-semantic build: a worker that cannot exist.
#[cfg(not(feature = "semantic-search"))]
mod imp {
    use super::*;

    pub struct InferenceWorker;

    impl InferenceWorker {
        /// Refuses **before** any thread is spawned — there is no engine to
        /// construct, so there is nothing a thread could do.
        pub fn spawn() -> Result<Self, ConstellationError> {
            Err(refuse_without_semantic_search())
        }

        /// Never observable: no worker of this kind is ever constructed.
        pub fn is_stopped(&self) -> bool {
            true
        }

        /// Never observable: no worker of this kind is ever constructed.
        pub fn is_in_call(&self) -> bool {
            false
        }

        /// Never observable: no worker of this kind is ever constructed.
        pub fn begin_batch(&self) -> BatchToken {
            BatchToken(1)
        }

        /// Never observable: no worker of this kind is ever constructed.
        pub fn current_batch(&self) -> BatchToken {
            BatchToken(1)
        }

        /// Never observable: no worker of this kind is ever constructed.
        pub fn submit(&self, _job: InferenceJob) -> SubmitOutcome {
            SubmitOutcome::WorkerStopped
        }

        /// Never observable: no worker of this kind is ever constructed.
        pub fn shutdown(self) -> Result<ShutdownOutcome, ConstellationError> {
            Ok(ShutdownOutcome::Joined)
        }
    }
}

/// The in-memory ready queue: distinct canonical keys, newest input per key.
///
/// # Why coalescing is structural, not timed
///
/// Membership is a **set of keys with a payload**, not a list of events. A
/// second notification for a key already present overwrites that key's input
/// and changes nothing else — so N rapid saves of one artifact admit one job
/// with the last body, and no number of events can grow the queue past the
/// number of distinct keys. Coalescing is a property of the data structure; a
/// timer is not involved and cannot be defeated by timing.
///
/// # Why overflow is not loss
///
/// [`Self::admit`] drops the newest key when the queue is full. That looks like
/// loss and is not: **the durable pending set lives in the manifest**, not
/// here. This queue is an accelerator over state that survives a crash, so a
/// key it declines is picked up by the next full scan. Nothing that reaches
/// admission is forgotten; nothing is double-queued.
///
/// # Not a permit
///
/// [`Self::admit`] never starts work and never releases anything. The worker
/// permit belongs to the `embed` call's return path — see the module docs.
#[derive(Debug, Clone)]
pub struct ReadyQueue {
    order: Vec<String>,
    inputs: std::collections::HashMap<String, std::sync::Arc<str>>,
    capacity: usize,
}

impl ReadyQueue {
    /// A queue of `capacity` distinct keys.
    pub fn new(capacity: usize) -> Self {
        Self {
            order: Vec::new(),
            inputs: std::collections::HashMap::new(),
            capacity,
        }
    }

    /// Admit `canonical_key` with `input_text`, coalescing with any earlier
    /// notification for the same key.
    ///
    /// Returns `true` if this key is now queued, `false` if the queue was full
    /// and declined it. A `false` is not a lost job: the manifest's durable
    /// pending still names the key.
    ///
    /// The text is `Arc<str>`, not `String`: a queued input is handed to an
    /// inference thread, and cloning the corpus once per queued job would make
    /// the queue's memory a function of the corpus rather than of the queue.
    pub fn admit(
        &mut self,
        canonical_key: impl Into<String>,
        input_text: impl Into<std::sync::Arc<str>>,
    ) -> bool {
        let key = canonical_key.into();
        let text = input_text.into();

        match self.inputs.get_mut(&key) {
            Some(existing) => {
                // Already queued: replace the stale input, keep the position.
                // This is the coalescing — the key is not re-appended, so the
                // queue does not grow on a repeat notification.
                *existing = text;
                true
            }
            None => {
                if self.order.len() >= self.capacity {
                    return false;
                }
                self.order.push(key.clone());
                self.inputs.insert(key, text);
                true
            }
        }
    }

    /// Take the next key and its current input, removing it from the queue.
    ///
    /// This is the **only** way work leaves the queue, so a key is admitted
    /// once per admission and runs once with the input it held at that moment.
    pub fn take_next(&mut self) -> Option<(String, std::sync::Arc<str>)> {
        let key = self.order.first()?.clone();
        let text = self.inputs.remove(&key)?;
        self.order.remove(0);
        Some((key, text))
    }

    /// Number of distinct keys held. This is bounded by `capacity` and by the
    /// number of distinct keys ever admitted — never by the number of events.
    pub fn len(&self) -> usize {
        self.order.len()
    }

    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }

    /// Whether `canonical_key` is currently queued.
    pub fn contains(&self, canonical_key: &str) -> bool {
        self.inputs.contains_key(canonical_key)
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

/// Signal an already-live runtime that something changed.
///
/// A live runtime observes the change and schedules its own capture. With no
/// runtime live this **persists nothing and returns `false`**: the canonical
/// Markdown is the durable truth, and the next capture finds the change
/// regardless. A dropped notification is therefore not lost work, which is
/// exactly why watchers need no durable queue of their own.
///
/// It takes no lock, opens no store, starts no thread, and starts no runtime.
/// A short CLI call to this is a no-op with a truthful meaning, not a daemon
/// kick-off.
///
/// # What it can and cannot reach
///
/// The registry is **process-local**. A mutation in another process — a CLI
/// command, a git hook, an editor — cannot signal a watcher, and this does not
/// pretend otherwise: what the watcher could miss, the
/// [`FULL_SCAN_PERIOD_MS`] clock finds.
///
/// Within one process, the reach depends entirely on whether a supervisor was
/// registered for that index, and today exactly one caller registers one:
/// `forgeplan watch --constellation`. `serve --auto-index` runs a single pass
/// and registers no runtime, and no MCP write handler calls this — so calling
/// it from an MCP handler today would return `false` and change nothing. That
/// is why the in-tree caller is the watcher's filesystem event, where a
/// supervisor provably exists. Anyone adding a second caller has to register
/// the supervisor for it to mean anything; the `false` return is what says so
/// rather than leaving it to be assumed.
///
/// The return value is the honest answer to "did anything receive this", not a
/// promise that the change was applied. A `false` means nobody was listening,
/// and the caller must not read it as "nothing changed".
pub fn mark_dirty(workspace: &std::path::Path) -> bool {
    let key = index_key(workspace);
    let Ok(registered) = registry().lock() else {
        // A poisoned registry mutex means another thread panicked while
        // registering. Reporting `false` is the truthful answer: this signal
        // reached nobody.
        return false;
    };
    match registered.get(&key).and_then(std::sync::Weak::upgrade) {
        Some(state) => {
            state.dirty.store(true, std::sync::atomic::Ordering::SeqCst);
            true
        }
        None => false,
    }
}

/// The key a workspace is registered under: its canonical physical index
/// root.
///
/// The **index**, not the workspace, because the index is what the owner lock
/// serialises. Two spellings that reach one physical index — a symlinked
/// checkout, a `.forgeplan` reached by two parents — are one index, and two
/// supervisors for it would be two publishers racing for one lock.
///
/// The workspace is canonicalised and the index directory joined onto it,
/// rather than the index directory being canonicalised: `lance/` does not
/// exist until the first bootstrap, so canonicalising it would make the key
/// depend on whether this index has been built yet, and `mark_dirty` would
/// then miss a workspace whose runtime is registered.
fn index_key(workspace: &std::path::Path) -> std::path::PathBuf {
    let workspace = workspace
        .canonicalize()
        .unwrap_or_else(|_| workspace.to_path_buf());
    workspace.join(super::facade::MaintainCapability::INDEX_DIR)
}

/// Registered runtimes, keyed by [`index_key`].
///
/// `Weak`, so a dropped supervisor deregisters itself rather than pinning a
/// runtime nobody can reach — and so [`MaintenanceSupervisor::registered`]
/// cannot resurrect a stopped one. The mutex is only ever held for a map
/// lookup and a flag store; no pass, capture or lock acquisition happens
/// inside it.
fn registry() -> &'static std::sync::Mutex<
    std::collections::BTreeMap<std::path::PathBuf, std::sync::Weak<SupervisorState>>,
> {
    static REGISTRY: std::sync::LazyLock<
        std::sync::Mutex<
            std::collections::BTreeMap<std::path::PathBuf, std::sync::Weak<SupervisorState>>,
        >,
    > = std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::BTreeMap::new()));
    &REGISTRY
}

/// One live runtime's mutable state, shared by every handle to it.
struct SupervisorState {
    /// The dirty bit [`mark_dirty`] sets and a pass clears.
    dirty: std::sync::atomic::AtomicBool,
    /// When the last pass ran. `None` until one has — a runtime that has never
    /// captured is immediately due for its first full scan.
    last_full_scan: std::sync::Mutex<Option<std::time::Instant>>,
    /// Bounded, coalescing work list. See [`ReadyQueue`] for why a decline
    /// here is not a loss.
    queue: std::sync::Mutex<ReadyQueue>,
    /// Until when the model is known unavailable. Re-probing a missing cache
    /// on every event turns a clear `unavailable` into a busy loop.
    #[cfg_attr(not(feature = "semantic-search"), allow(dead_code))]
    model_unavailable_until: std::sync::Mutex<Option<std::time::Instant>>,
    /// The one inference worker for this runtime, parked here between passes.
    ///
    /// Taken out for the duration of a drain and put back afterwards, rather
    /// than borrowed across the `await`: a `MutexGuard` held across an await is
    /// not `Send`, and the drain has to be. `draining` is what stops two
    /// callers from each taking the empty slot and constructing an engine of
    /// their own.
    #[cfg(feature = "semantic-search")]
    worker: std::sync::Mutex<Option<(String, InferenceWorker)>>,
    /// Held for the length of one drain. Set before the worker is taken and
    /// cleared after it is parked, so "one inference at a time on this index"
    /// is a property of this flag rather than a comment.
    #[cfg(feature = "semantic-search")]
    draining: std::sync::atomic::AtomicBool,
    /// Serialises passes for this index, so a debounce batch, a 30-second
    /// timer and a signal cannot run two captures at once.
    pass_lock: tokio::sync::Mutex<()>,
}

impl SupervisorState {
    fn new() -> Self {
        Self {
            dirty: std::sync::atomic::AtomicBool::new(false),
            last_full_scan: std::sync::Mutex::new(None),
            queue: std::sync::Mutex::new(ReadyQueue::new(READY_QUEUE_CAPACITY)),
            model_unavailable_until: std::sync::Mutex::new(None),
            #[cfg(feature = "semantic-search")]
            worker: std::sync::Mutex::new(None),
            #[cfg(feature = "semantic-search")]
            draining: std::sync::atomic::AtomicBool::new(false),
            pass_lock: tokio::sync::Mutex::new(()),
        }
    }
}

/// What made a pass run, when it ran at all.
///
/// The distinction a caller needs: a **signalled** pass follows an observed
/// change and may legitimately be followed by an inference drain, while a
/// **full-scan** pass is the ceiling on how long a missed event can hide one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassTrigger {
    /// A change was signalled — [`mark_dirty`] or an explicit
    /// [`MaintenanceSupervisor::signal`].
    Signalled,
    /// [`FULL_SCAN_PERIOD_MS`] elapsed since the last pass.
    FullScan,
}

/// What one supervisor pass did.
#[derive(Debug, Clone, Default)]
pub struct SupervisorPass {
    /// `false` when nothing was due: no signal and no full scan elapsed. Every
    /// other field is then meaningless rather than zero, which is why `ran` is
    /// the first thing a caller checks.
    pub ran: bool,
    pub trigger: Option<PassTrigger>,
    /// The generation the pass published, or the one already published when
    /// it had nothing to commit.
    pub generation_id: String,
    pub commits: usize,
    /// Keys admitted into the ready queue by this pass.
    pub queued: usize,
    /// Keys the bounded queue declined. Not lost — the durable pending set in
    /// the manifest names them and the next full scan picks them up.
    pub declined: usize,
    /// Pending work recovery could not judge.
    pub gaps: Vec<(String, RecoveryGap)>,
}

/// A registered maintenance runtime for one physical index.
///
/// # What registering does and does not start
///
/// It publishes a signal other code in this process can reach, and it holds
/// the state a pass needs. It does **not** spawn a thread, does not construct
/// an engine, and does not begin capturing. The inference worker is created
/// lazily by [`Self::drain_inference`], and only when there is work to submit
/// — which is what keeps a short CLI call to [`mark_dirty`] a no-op rather
/// than a daemon kick-off.
///
/// # One per physical index
///
/// [`Self::register`] is idempotent: a second call for the same index returns
/// a handle onto the same state. Two supervisors for one index would be two
/// publishers racing for one owner lock, and the second would lose every pass
/// to `index_busy` for no gain.
#[derive(Clone)]
pub struct MaintenanceSupervisor {
    key: std::path::PathBuf,
    workspace: std::path::PathBuf,
    state: std::sync::Arc<SupervisorState>,
}

impl MaintenanceSupervisor {
    /// Register — or join — the runtime for `workspace`'s index.
    pub fn register(workspace: &std::path::Path) -> Self {
        let key = index_key(workspace);
        let workspace = workspace
            .canonicalize()
            .unwrap_or_else(|_| workspace.to_path_buf());
        let Ok(mut registered) = registry().lock() else {
            // A poisoned registry must not stop a watcher from running. This
            // supervisor is simply not reachable by `mark_dirty`, which is the
            // honest cost, not a reason to refuse to maintain.
            return Self {
                key,
                workspace,
                state: std::sync::Arc::new(SupervisorState::new()),
            };
        };
        if let Some(existing) = registered.get(&key).and_then(std::sync::Weak::upgrade) {
            return Self {
                key,
                workspace,
                state: existing,
            };
        }
        let state = std::sync::Arc::new(SupervisorState::new());
        registered.insert(key.clone(), std::sync::Arc::downgrade(&state));
        Self {
            key,
            workspace,
            state,
        }
    }

    /// The runtime already registered for `workspace`, if any.
    ///
    /// The lookup a caller that only wants to *use* a live runtime makes. It
    /// never registers: asking whether someone is listening must not be a way
    /// to start listening.
    pub fn registered(workspace: &std::path::Path) -> Option<Self> {
        let key = index_key(workspace);
        let registered = registry().lock().ok()?;
        let state = registered.get(&key).and_then(std::sync::Weak::upgrade)?;
        let workspace = workspace
            .canonicalize()
            .unwrap_or_else(|_| workspace.to_path_buf());
        Some(Self {
            key,
            workspace,
            state,
        })
    }

    /// The workspace this runtime maintains.
    pub fn workspace(&self) -> &std::path::Path {
        &self.workspace
    }

    /// The physical index root this runtime is registered under.
    pub fn index_key(&self) -> &std::path::Path {
        &self.key
    }

    /// Whether a change has been signalled since the last pass consumed one.
    pub fn is_dirty(&self) -> bool {
        self.state.dirty.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Signal this runtime directly, for a caller already holding a handle.
    ///
    /// [`mark_dirty`] is the same operation through the registry; this is the
    /// in-handle form a watcher uses after registering.
    pub fn signal(&self) {
        self.state
            .dirty
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    /// Whether the full-scan period has elapsed. A runtime that has never
    /// captured is due immediately.
    pub fn full_scan_due(&self) -> bool {
        let Ok(last) = self.state.last_full_scan.lock() else {
            return true;
        };
        match *last {
            None => true,
            Some(at) => {
                std::time::Instant::now().saturating_duration_since(at)
                    >= std::time::Duration::from_millis(FULL_SCAN_PERIOD_MS)
            }
        }
    }

    /// Distinct keys this runtime currently has queued.
    pub fn queued(&self) -> usize {
        self.state.queue.lock().map(|q| q.len()).unwrap_or(0)
    }

    /// Whether the model is inside its post-failure cooldown.
    pub fn model_in_cooldown(&self) -> bool {
        let Ok(until) = self.state.model_unavailable_until.lock() else {
            return false;
        };
        until.is_some_and(|at| std::time::Instant::now() < at)
    }

    /// Remove this runtime from the registry.
    ///
    /// Idempotent, and it only removes an entry that is still this runtime's:
    /// a handle to an old state cannot evict the runtime that replaced it.
    /// The inference worker is **not** shut down here — [`Self::shutdown`]
    /// does that, and does it by joining.
    pub fn unregister(&self) {
        let Ok(mut registered) = registry().lock() else {
            return;
        };
        if registered.get(&self.key).is_some_and(|weak| {
            weak.upgrade()
                .is_some_and(|s| std::sync::Arc::ptr_eq(&s, &self.state))
        }) {
            registered.remove(&self.key);
        }
    }

    /// Stop being reachable and stop the inference worker.
    ///
    /// The registry entry goes first, so a concurrent [`mark_dirty`] cannot
    /// hand work to a runtime that is stopping.
    ///
    /// Whether the worker's thread is **joined** or **detached** is decided by
    /// the worker, from whether a native call is actually in flight — see
    /// [`InferenceWorker::shutdown`]. A watcher that stopped for any reason
    /// other than a hung model gets `Joined`, which is the strong statement; a
    /// watcher interrupted by Ctrl+C during a call that has not returned gets
    /// `Detached`, which is the truthful one. Neither blocks the process.
    #[cfg(feature = "semantic-search")]
    pub fn shutdown(self) -> Result<ShutdownOutcome, ConstellationError> {
        self.unregister();
        let worker = self
            .state
            .worker
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        match worker {
            Some((_model, worker)) => worker.shutdown(),
            None => Ok(ShutdownOutcome::Joined),
        }
    }

    /// Stop being reachable. Feature-off has no worker to join.
    #[cfg(not(feature = "semantic-search"))]
    pub fn shutdown(self) -> Result<ShutdownOutcome, ConstellationError> {
        self.unregister();
        Ok(ShutdownOutcome::Joined)
    }

    /// Wait until no capture pass owns this runtime's storage commits.
    ///
    /// The wait is on the same per-runtime lock [`Self::capture_pass`] takes,
    /// so it is exact: when this returns, either no pass had started, or the
    /// one that had has published and released its commits.
    ///
    /// # Why shutdown needs it
    ///
    /// [`Self::shutdown`] removes the registry entry and stops the inference
    /// worker. Doing that while a pass is between "stage written" and "CURRENT
    /// replaced" releases publication ownership underneath a writer: the next
    /// process can take the owner lock and publish from the same index root
    /// while this pass is still committing. That is the split-brain the owner
    /// protocol exists to prevent, and it is invisible — both publishers
    /// succeed and the last one wins.
    ///
    /// It is `async` because the wait has to suspend: blocking a runtime thread
    /// on a `tokio::sync::Mutex` is how a shutdown deadlocks its own executor.
    /// It is **not** a timeout. A pass that hangs must be waited on rather than
    /// abandoned — abandoning it is exactly the ownership release above — and
    /// the caller's own bounded scope (the process exiting) is what bounds the
    /// wait from the outside.
    pub async fn pass_finished(&self) {
        let _pass = self.state.pass_lock.lock().await;
    }

    /// One maintenance pass, if one is due.
    ///
    /// Runs [`super::facade::maintain_index`] and then re-derives the work
    /// list from what a previous process left behind
    /// ([`recover_pending`]), admitting it into the bounded queue.
    ///
    /// # Why it can decline to run
    ///
    /// Not dirty and not due returns immediately with `ran: false`. That is
    /// what makes it safe to call on every watcher tick: the alternative — a
    /// capture per tick — would capture the whole corpus several times a
    /// second to notice nothing.
    ///
    /// # Lock discipline
    ///
    /// The owner and the workspace lock are taken and released by
    /// `maintain_index` exactly as they are everywhere else. This function
    /// holds neither; its own lock is per-runtime and serialises passes for
    /// this index so a signal, a timer and a debounce batch cannot overlap.
    pub async fn capture_pass(
        &self,
        capability: &super::facade::MaintainCapability,
        capture_limits: super::capture::CaptureLimits,
        deadline: std::time::Instant,
    ) -> Result<SupervisorPass, ConstellationError> {
        let _pass = self.state.pass_lock.lock().await;

        let signalled = self
            .state
            .dirty
            .swap(false, std::sync::atomic::Ordering::SeqCst);
        let due = self.full_scan_due();
        if !signalled && !due {
            return Ok(SupervisorPass::default());
        }
        let trigger = if signalled {
            PassTrigger::Signalled
        } else {
            PassTrigger::FullScan
        };

        let (view, outcome) =
            super::facade::maintain_index(capability, capture_limits, deadline).await?;

        // The clock moves on success only. A failed pass leaves the runtime
        // due, so the next tick retries rather than waiting a full period for
        // a failure that may already be over.
        if let Ok(mut last) = self.state.last_full_scan.lock() {
            *last = Some(std::time::Instant::now());
        }

        let (queued, declined, gaps) = self.recover(capability, deadline).await?;

        Ok(SupervisorPass {
            ran: true,
            trigger: Some(trigger),
            generation_id: view.manifest.generation_id.clone(),
            commits: outcome.commits.len(),
            queued,
            declined,
            gaps,
        })
    }

    /// Re-derive the durable work list and admit it into the queue.
    ///
    /// Returns `(queued, declined, gaps)`. The gaps are **reported, never
    /// dropped**: a pending job whose eligibility cannot be decided is a fact
    /// about the index, and a caller that showed only the admitted list would
    /// be presenting a partial answer as a complete one.
    async fn recover(
        &self,
        capability: &super::facade::MaintainCapability,
        deadline: std::time::Instant,
    ) -> Result<(usize, usize, Vec<(String, RecoveryGap)>), ConstellationError> {
        if std::time::Instant::now() >= deadline {
            return Ok((0, 0, Vec::new()));
        }
        let index_root = capability.index_root().to_path_buf();
        let Some(descriptor) = super::storage::read_current(&index_root)? else {
            // Nothing published: there is no pending set to recover from, and
            // inventing one would report work that does not exist.
            return Ok((0, 0, Vec::new()));
        };
        let manifest = super::storage::read_manifest(&index_root, &descriptor)?;
        if manifest.pending_jobs.is_empty() {
            return Ok((0, 0, Vec::new()));
        }
        let body_prefix_chars = crate::workspace::init::load_config(capability.workspace())
            .ok()
            .and_then(|config| config.embedding.map(|embedding| embedding.chunk_size))
            .unwrap_or(super::jobs::DEFAULT_BODY_PREFIX_CHARS);

        let captured = super::publisher::capture_under_workspace_lock(
            capability.workspace(),
            super::capture::CaptureLimits::default(),
            Some(std::sync::Arc::new(manifest.clone())),
        )
        .await?;
        let (jobs, gaps) = recover_pending(&manifest, &captured, body_prefix_chars);

        let mut queued = 0usize;
        let mut declined = 0usize;
        {
            let Ok(mut queue) = self.state.queue.lock() else {
                return Err(ConstellationError::Io {
                    detail: "the runtime ready queue is poisoned; the work list cannot be updated"
                        .to_string(),
                });
            };
            for job in jobs {
                if queue.admit(job.job.canonical_key.clone(), job.input_text) {
                    queued += 1;
                } else {
                    declined += 1;
                }
            }
        }
        Ok((queued, declined, gaps))
    }

    /// Drain the queued work through admission → worker → completion.
    ///
    /// Returns `Ok(None)` when the drain was skipped without an error: the
    /// model is inside its post-failure cooldown. `Ok(None)` is not a failure
    /// and not a silent success — the caller reports "no drain" and the next
    /// pass tries again.
    ///
    /// # Why this does not gate on an empty queue
    ///
    /// The queue records what **recovery** found outstanding; the drain's own
    /// first phase re-derives eligibility from the captured rows and mints jobs
    /// for anything still owed. A runtime whose queue is empty can therefore
    /// still have work — a workspace that has never been embedded has none of
    /// either. Gating on the queue would make the first embedding in a
    /// workspace unreachable, which is the whole reason the caller asks.
    /// Deciding *when* to call is the caller's job, and the caller's evidence
    /// (a signal, or recovered work) is what this reports back.
    ///
    /// # The model is never fetched here
    ///
    /// The worker is [`InferenceWorker::spawn_resolved`] over a profile the
    /// caller already resolved from the cache, so a cache that is not there is
    /// `ModelUnavailable` and the cooldown starts. A watch event, a serve tick
    /// and a git hook therefore never begin a download, and they do not
    /// re-probe for one every [`MODEL_UNAVAILABLE_COOLDOWN_MS`] either.
    ///
    /// # One worker, one call
    ///
    /// The worker is parked in the supervisor between drains and is taken out
    /// for the duration of one. A drain that arrives while another is running
    /// finds the slot empty and skips — which is what makes "one inference at a
    /// time on this index" a property of the slot rather than a comment.
    ///
    /// # `control`
    ///
    /// The host's authority over this drain's waiting. A stop or a due control
    /// scan ends the drain through [`DrainControl`]; the batch is abandoned,
    /// the outstanding jobs stay durably pending, and nothing is recorded
    /// failed. A caller with no loop to serve passes [`DrainControl::never`]
    /// and is bounded by `deadline` alone.
    #[cfg(feature = "semantic-search")]
    pub async fn drain_inference(
        &self,
        capability: &super::facade::MaintainCapability,
        model_name: &str,
        resolved: &super::jobs::ResolvedProfile,
        deadline: std::time::Instant,
        control: &DrainControl,
    ) -> Result<Option<super::facade::EmbeddingQueueOutcome>, ConstellationError> {
        if self.model_in_cooldown() {
            return Ok(None);
        }
        // The permit is the flag, not a counter that could disagree with
        // reality: a second caller that arrives mid-drain finds it set and
        // returns instead of building a second engine to run beside the first.
        if self
            .state
            .draining
            .compare_exchange(
                false,
                true,
                std::sync::atomic::Ordering::SeqCst,
                std::sync::atomic::Ordering::SeqCst,
            )
            .is_err()
        {
            return Ok(None);
        }
        // Cancellation drops this future without reaching its normal epilogue.
        // Release admission on every exit, including unwind and cancellation.
        struct DrainReset<'a>(&'a std::sync::atomic::AtomicBool);
        impl Drop for DrainReset<'_> {
            fn drop(&mut self) {
                self.0.store(false, std::sync::atomic::Ordering::SeqCst);
            }
        }
        let _drain_reset = DrainReset(&self.state.draining);
        let mut worker = match self.take_worker(model_name, resolved) {
            Some(worker) => worker,
            None => {
                self.note_model_unavailable();
                return Err(ConstellationError::ModelUnavailable {
                    detail: format!(
                        "no usable cached embedding model for `{model_name}` at {}; the queue \
                         stays pending and nothing is downloaded",
                        resolved.snapshot_root.display()
                    ),
                });
            }
        };
        let result = super::facade::drain_embedding_queue(
            capability,
            &resolved.profile,
            &mut worker,
            deadline,
            control,
        )
        .await;
        match &result {
            // Whatever the drain managed, the in-memory list is now stale: the
            // durable pending set in the manifest is what the next pass reads.
            // Keeping it would re-submit keys that already published.
            Ok(_) | Err(ConstellationError::ModelUnavailable { .. }) => self.clear_queue(),
            _ => {}
        }
        if matches!(result, Err(ConstellationError::ModelUnavailable { .. })) {
            self.note_model_unavailable();
        }
        // Abandon whatever batch the drain left open. A drain that ended on its
        // deadline — or on a worker that stopped — may have left jobs queued and
        // a call in flight; both belong to a batch nobody is waiting for any
        // more. Opening the next batch here means their late answers are
        // discarded where they are recognised instead of arriving at the next
        // drain, which never issued them.
        worker.begin_batch();
        self.put_worker(model_name, resolved, worker);
        result.map(Some)
    }

    /// Take the parked worker, constructing it if this runtime has none.
    ///
    /// Three things disqualify a parked worker, and all three are the same
    /// kind of thing: it cannot answer.
    ///
    /// A **different embedding space** — the key is the resolved snapshot
    /// root plus the full `profile_id`, so a changed revision **and** a changed
    /// `body_prefix_chars` both disqualify reuse. Keying on the model *name*
    /// alone let a chunk-size change reuse an engine whose vectors belong to a
    /// different prefix: both are 1024 unit-length floats, so the drain would
    /// publish rows whose `input_digest` was never embedded. A **stopped
    /// thread** — the engine failed to construct, or `embed` errored, or the
    /// thread panicked, and the thread returned; reusing it would park a handle
    /// whose submissions all fail, which is the defect that made repairing a
    /// cached model take a process restart to take effect. And a worker parked
    /// under the same key whose call is still in flight cannot be parked twice
    /// anyway, because the `draining` flag above is still set for the drain
    /// that holds it.
    ///
    /// Dropping a parked worker detaches a thread that can no longer publish
    /// anything, because the only handle that could write is the one going away.
    #[cfg(feature = "semantic-search")]
    fn take_worker(
        &self,
        model_name: &str,
        resolved: &super::jobs::ResolvedProfile,
    ) -> Option<InferenceWorker> {
        let wanted = worker_reuse_key(resolved);
        let mut slot = self.state.worker.lock().ok()?;
        let parked = slot.take();
        let reusable = matches!(
            &parked,
            Some((parked_key, worker)) if parked_key == &wanted && !worker.is_stopped()
        );
        if reusable {
            return parked.map(|(_, worker)| worker);
        }
        drop(parked);
        match InferenceWorker::spawn_resolved(model_name, resolved) {
            Ok(worker) => Some(worker),
            Err(e) => {
                tracing::warn!(
                    "inference worker unavailable for `{model_name}` at {}: {e}",
                    resolved.snapshot_root.display()
                );
                None
            }
        }
    }

    /// Park the worker again for the next pass — **unless it is finished**.
    ///
    /// A worker whose thread has returned is not parked. Parking it would keep
    /// a dead handle in the hot path: the next pass would take it, its key
    /// would match, and every submission would come back
    /// [`SubmitOutcome::WorkerStopped`] until the process restarted. Repairing
    /// the model would change nothing, which is exactly the recovery this
    /// refuses to deny.
    #[cfg(feature = "semantic-search")]
    fn put_worker(
        &self,
        model_name: &str,
        resolved: &super::jobs::ResolvedProfile,
        worker: InferenceWorker,
    ) {
        if worker.is_stopped() {
            // Already dead: dropping it costs nothing, because its thread has
            // returned and there is nothing to detach.
            return;
        }
        let key = worker_reuse_key(resolved);
        // The name is kept beside the key purely so the warning a future
        // reader sees names the configuration a human can act on.
        tracing::debug!("parked inference worker for `{model_name}` under {key}");
        if let Ok(mut slot) = self.state.worker.lock() {
            *slot = Some((key, worker));
        }
    }

    /// Start the post-failure cooldown.
    #[cfg(feature = "semantic-search")]
    fn note_model_unavailable(&self) {
        if let Ok(mut until) = self.state.model_unavailable_until.lock() {
            *until = Some(
                std::time::Instant::now()
                    + std::time::Duration::from_millis(MODEL_UNAVAILABLE_COOLDOWN_MS),
            );
        }
    }

    /// Drop the in-memory work list. The durable pending set is unaffected.
    #[cfg(feature = "semantic-search")]
    fn clear_queue(&self) {
        if let Ok(mut queue) = self.state.queue.lock() {
            *queue = ReadyQueue::new(READY_QUEUE_CAPACITY);
        }
    }
}

/// The identity an inference engine may be reused under.
///
/// Two parts, and both are load-bearing.
///
/// The **snapshot root and revision** say *which weights* the resident engine
/// holds. `refs/main` can move under a long-running host; a profile resolved
/// before the move and a profile resolved after it have the same repo and two
/// different sets of bytes, and both hash to 1024 unit-length float vectors.
///
/// The **full `profile_id`** says *what text was embedded*. `profile_id`
/// covers `body_prefix_chars` along with pooling, normalisation, prefixes,
/// truncation and chunking, so a chunk-size edit changes it — which is the
/// point: reusing the engine across it would publish vectors computed from a
/// prefix the rows' `input_digest` does not describe.
///
/// This is deliberately **not** the model name. Two model names that resolve
/// to the same repo would produce two engines for one space; one name whose
/// snapshot changed must not reuse the engine it had a minute ago.
#[cfg(feature = "semantic-search")]
fn worker_reuse_key(resolved: &super::jobs::ResolvedProfile) -> String {
    format!(
        "{}@{}#{}",
        resolved.snapshot_root.display(),
        resolved.snapshot_id,
        resolved.profile.profile_id()
    )
}

/// The admission generation a stored job's `captured_input_ref` names.
///
/// A recovered job is re-minted against the **manifest's** generation, not
/// against the fresh capture's timestamp: the recovered job's whole point is
/// to be traceable to the admission that promised the work, and a ref naming a
/// generation nobody ever published names nothing. Returns the stored prefix
/// as-is when the ref does not have the canonical shape, so a hand-edited ref
/// degrades to "whatever was recorded" rather than to a guess.
fn admission_generation_for(job: &super::jobs::EmbeddingJob) -> &str {
    match job.captured_input_ref.rsplit_once(':') {
        Some((generation, key)) if key == job.canonical_key => generation,
        _ => job.captured_input_ref.as_str(),
    }
}

#[cfg(test)]
mod queue_tests {
    use super::ReadyQueue;
    use crate::constellation::runtime::READY_QUEUE_CAPACITY;

    /// L14 — five notifications for one key admit **once**.
    ///
    /// Breaks if coalescing is by timing rather than structure: a
    /// timer-driven design admits five. Breaks if a repeat appends to the
    /// order vector: `len()` becomes 5.
    #[test]
    fn five_notifications_for_one_key_admit_once_with_the_newest_input() {
        let mut q = ReadyQueue::new(READY_QUEUE_CAPACITY);
        for i in 0..5 {
            assert!(
                q.admit("PRD-001", format!("body {i}")),
                "admission {i} must succeed"
            );
        }
        assert_eq!(q.len(), 1, "five events for one key must leave one entry");

        let (key, text) = q.take_next().expect("the key must be queued exactly once");
        assert_eq!(key, "PRD-001");
        assert_eq!(
            text.as_ref(),
            "body 4",
            "the queued input must be the newest one, not the first"
        );
        assert!(q.is_empty(), "taking the only key must empty the queue");
    }

    /// The queue is O(distinct keys), never O(events).
    ///
    /// Breaks if any notification appends: 100 events would leave 100 entries.
    #[test]
    fn a_hundred_notifications_over_ten_keys_leave_at_most_ten_entries() {
        let mut q = ReadyQueue::new(READY_QUEUE_CAPACITY);
        for i in 0..100 {
            let key = format!("PRD-{:03}", i % 10);
            assert!(q.admit(key, format!("revision {i}")));
        }
        assert_eq!(
            q.len(),
            10,
            "queue size must track distinct keys, not notifications"
        );
    }

    /// Full queue: the newest key is declined, and everything already queued is
    /// intact and still drainable. **None is lost** — the declined key remains
    /// named in the durable manifest and is recovered by the next full scan.
    ///
    /// Breaks if admission overwrites or evicts an existing key: the first key
    /// would be missing from the drain.
    #[test]
    fn a_full_queue_declines_the_next_key_and_loses_nothing_already_queued() {
        let cap = 4;
        let mut q = ReadyQueue::new(cap);
        for i in 0..cap {
            assert!(q.admit(format!("PRD-{i}"), "body"));
        }
        assert_eq!(q.len(), cap);

        let admitted = q.admit("PRD-OVERFLOW", "body");
        assert!(!admitted, "a full queue must decline rather than grow");
        assert_eq!(
            q.len(),
            cap,
            "a declined admission must not change the size"
        );

        // Everything already queued drains, in order, unmodified.
        for i in 0..cap {
            let (key, text) = q.take_next().expect("a queued key must be drainable");
            assert_eq!(key, format!("PRD-{i}"), "queued keys keep their order");
            assert_eq!(text.as_ref(), "body");
        }
        assert!(q.is_empty());

        // The declined key is still admissible once there is room: nothing was
        // forgotten, only deferred.
        assert!(
            q.admit("PRD-OVERFLOW", "body"),
            "a declined key must be admissible later"
        );
    }

    /// Two keys interleave and coalesce independently.
    ///
    /// Breaks if the queue conflates keys, or if replacing one key's input
    /// disturbs another's.
    #[test]
    fn keys_coalesce_independently_and_keep_admission_order() {
        let mut q = ReadyQueue::new(READY_QUEUE_CAPACITY);
        assert!(q.admit("PRD-A", "a1"));
        assert!(q.admit("PRD-B", "b1"));
        assert!(q.admit("PRD-A", "a2"));
        assert!(q.contains("PRD-A"));
        assert_eq!(q.len(), 2);

        let (k1, t1) = q.take_next().unwrap();
        assert_eq!((k1.as_str(), &*t1), ("PRD-A", "a2"));
        let (k2, t2) = q.take_next().unwrap();
        assert_eq!((k2.as_str(), &*t2), ("PRD-B", "b1"));
    }

    /// `mark_dirty` with no live runtime persists nothing and starts nothing.
    ///
    /// Asserts the honest answer rather than a mock: nothing registered this
    /// workspace, so the truthful report is `false`, and a caller must not
    /// read it as work being scheduled.
    #[test]
    fn mark_dirty_with_no_live_runtime_reports_no_runtime() {
        let absent = tempfile::TempDir::new().unwrap();
        assert!(
            !crate::constellation::runtime::mark_dirty(absent.path()),
            "with no live runtime there is nothing to signal, and the report must say so"
        );
    }

    /// A registered runtime is reachable by name and only by its own index.
    ///
    /// The registry is the whole of what `mark_dirty` can reach, so this is the
    /// behaviour the "no automatic download / no phantom runtime" claim rests
    /// on: registration makes a signal land, unregistration makes it stop
    /// landing, and a different workspace is never reached.
    #[test]
    fn a_registered_runtime_receives_a_signal_and_stops_receiving_it_when_unregistered() {
        use crate::constellation::runtime::{MaintenanceSupervisor, mark_dirty};

        let mine = tempfile::TempDir::new().unwrap();
        let theirs = tempfile::TempDir::new().unwrap();

        assert!(
            !mark_dirty(mine.path()),
            "before registration nothing is reachable"
        );

        let supervisor = MaintenanceSupervisor::register(mine.path());
        assert!(
            mark_dirty(mine.path()),
            "a registered runtime must be reachable"
        );
        assert!(
            supervisor.is_dirty(),
            "the signal must land on the runtime that was registered"
        );
        assert!(
            !mark_dirty(theirs.path()),
            "one index's runtime must not receive another's signal"
        );

        // The lookup a caller that only wants to *use* a live runtime makes
        // finds it by name — and a neighbouring path is not it.
        assert!(
            MaintenanceSupervisor::registered(mine.path()).is_some(),
            "a registered runtime must be findable by name"
        );
        assert!(
            MaintenanceSupervisor::registered(&mine.path().join("nested-not-required")).is_none(),
            "an unregistered path must not resolve to a runtime by proximity"
        );

        supervisor.unregister();
        assert!(
            !mark_dirty(mine.path()),
            "an unregistered runtime must stop receiving signals"
        );
        assert!(
            MaintenanceSupervisor::registered(mine.path()).is_none(),
            "unregistration must remove the runtime, not just hide it"
        );
    }

    /// Two registrations for one index share one runtime.
    ///
    /// Breaks if `register` always constructs: the second handle would be a
    /// second publisher for one owner lock, and every pass would collide with
    /// the first as `index_busy`.
    #[test]
    fn registering_one_index_twice_yields_one_runtime() {
        use crate::constellation::runtime::MaintenanceSupervisor;

        let tmp = tempfile::TempDir::new().unwrap();
        let first = MaintenanceSupervisor::register(tmp.path());
        let second = MaintenanceSupervisor::register(tmp.path());
        assert_eq!(
            first.index_key(),
            second.index_key(),
            "both handles name the same physical index"
        );

        first.signal();
        assert!(
            second.is_dirty(),
            "a signal through one handle must be visible to the other — they are one runtime"
        );
        second.unregister();
    }

    /// The host maintains **the capability's** workspace, never a directory it
    /// inferred for itself.
    ///
    /// This is what makes an explicit `workspace: Y` work while the server runs
    /// in X: the host has no other source of truth to pick up, so there is no
    /// path by which it can end up watching X.
    #[tokio::test]
    async fn the_host_watches_the_capabilitys_workspace_and_not_a_directory_it_inferred() {
        use crate::constellation::runtime::{MaintenanceHost, ReportSink};

        // Another test in this binary may have stopped admission
        // process-wide; a startup pass that refused for that reason would look
        // exactly like a host defect.
        crate::constellation::facade::resume_admission();

        let (target, sibling) = two_workspaces();
        // Canonical, as every production caller spells it: `TempDir` lands
        // under the `/var` alias on macOS, and the assertion below is about
        // WHICH workspace the host watches, not about it rewriting the
        // spelling it was handed.
        let capability = crate::constellation::facade::MaintainCapability::for_strict_maintenance(
            target.path().canonicalize().unwrap(),
        );
        let host = MaintenanceHost::start(capability, ReportSink::Foreground)
            .await
            .expect("the target has artifact directories");

        assert_eq!(
            host.workspace(),
            target.path().canonicalize().unwrap(),
            "the host must watch the workspace its capability names"
        );
        assert_ne!(
            host.workspace(),
            sibling.path().canonicalize().unwrap(),
            "and must not have fallen back to the other workspace"
        );
        host.shutdown().await.expect("a clean shutdown");
    }

    /// Two hosts on one index are one runtime, not two publishers.
    ///
    /// The observable is the registry: a signal through either handle reaches
    /// the other. If it did not, the second host would capture and publish on
    /// its own schedule and the two would fight for one owner lock — the
    /// split-brain the single-supervisor registry exists to prevent.
    #[tokio::test]
    async fn two_hosts_for_one_index_share_one_supervisor() {
        use crate::constellation::runtime::{MaintenanceHost, MaintenanceSupervisor, ReportSink};

        crate::constellation::facade::resume_admission();

        let (target, _sibling) = two_workspaces();
        let capability = crate::constellation::facade::MaintainCapability::for_strict_maintenance(
            target.path().to_path_buf(),
        );
        let first = MaintenanceHost::start(capability.clone(), ReportSink::Foreground)
            .await
            .expect("the target has artifact directories");
        let second = MaintenanceHost::start(capability, ReportSink::Foreground)
            .await
            .expect("the target has artifact directories");

        let supervisor = MaintenanceSupervisor::registered(first.workspace())
            .expect("a running host keeps its runtime registered");
        supervisor.signal();
        assert!(
            supervisor.is_dirty(),
            "the signal must survive: one index, one runtime, one dirty bit"
        );

        first.shutdown().await.expect("clean");
        second.shutdown().await.expect("clean");
    }

    /// A workspace with no artifact directories gets **no** host.
    ///
    /// A host that started anyway would watch nothing, publish nothing, and
    /// report success — which reads exactly like "maintenance is running" while
    /// nothing keeps the index current.
    #[tokio::test]
    async fn a_workspace_with_no_artifact_directories_refuses_to_start_a_host() {
        use crate::constellation::runtime::{MaintenanceHost, ReportSink};

        let tmp = tempfile::TempDir::new().unwrap();
        let capability = crate::constellation::facade::MaintainCapability::for_strict_maintenance(
            tmp.path().to_path_buf(),
        );
        // Pattern-match rather than `expect_err`: a `Debug` bound on
        // `MaintenanceHost` would exist only to satisfy this assertion.
        let Err(err) = MaintenanceHost::start(capability, ReportSink::Foreground).await else {
            panic!("a workspace with no artifact directories must refuse to start a host");
        };
        assert!(
            err.to_string().contains("artifact directories"),
            "the refusal must name what is missing: {err}"
        );
    }

    /// `pass_finished` really waits: it cannot be acquired while a pass holds
    /// the runtime, and it is free once no pass does.
    ///
    /// This is the property shutdown depends on. If it returned immediately, a
    /// host could unregister its supervisor while a pass was between staging
    /// and publishing, and the next process's publisher would take the owner
    /// lock on an index this one was still committing to.
    #[tokio::test]
    async fn pass_finished_waits_for_a_pass_that_owns_storage_commits() {
        use crate::constellation::runtime::MaintenanceSupervisor;
        use std::sync::Arc;

        let tmp = tempfile::TempDir::new().unwrap();
        let supervisor = MaintenanceSupervisor::register(tmp.path());

        // Nothing running: the wait completes immediately.
        tokio::time::timeout(
            std::time::Duration::from_secs(5),
            supervisor.pass_finished(),
        )
        .await
        .expect("with no pass in flight the wait must not block");

        // A pass holding the runtime blocks it, and the block is released when
        // the pass lets go.
        //
        // The lock held here is the supervisor's OWN `pass_lock` — the one
        // `capture_pass` takes and `pass_finished` waits on. A stand-in mutex
        // would let the assertion pass for the wrong reason: the wait would
        // be unblocked immediately, so "the waiter is blocked" would be testing
        // nothing.
        let held = Arc::clone(&supervisor.state);
        let pass = tokio::spawn(async move {
            let _guard = held.pass_lock.lock().await;
            tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        });
        // Give the task a moment to take the lock the pass models.
        tokio::time::sleep(std::time::Duration::from_millis(30)).await;

        let waiter = {
            let supervisor = supervisor.clone();
            tokio::spawn(async move { supervisor.pass_finished().await })
        };
        // The waiter must still be blocked: the pass owns the runtime. Poll the
        // task rather than handing the handle to `timeout` — a timeout that
        // expires still consumed it, and the second await below needs it.
        tokio::time::sleep(std::time::Duration::from_millis(30)).await;
        assert!(
            !waiter.is_finished(),
            "shutdown must not be able to proceed while a pass owns the commits"
        );
        let _ = pass.await;
        tokio::time::timeout(std::time::Duration::from_secs(5), waiter)
            .await
            .expect("the wait must complete once the pass releases")
            .expect("the waiter task must not panic");

        supervisor.unregister();
    }

    /// Two real workspaces, both with artifact directories, so a test can
    /// prove which one a host picked.
    fn two_workspaces() -> (tempfile::TempDir, tempfile::TempDir) {
        let make = || {
            let tmp = tempfile::TempDir::new().unwrap();
            // `/var` is a symlink to `/private/var` on macOS; a host minted a
            // capability for the alias would refuse its own index root.
            let root = tmp.path().canonicalize().unwrap();
            for dir in crate::workspace::ARTIFACT_DIRS {
                std::fs::create_dir_all(root.join(dir)).unwrap();
            }
            std::fs::write(
                root.join("prds").join("PRD-001-auth.md"),
                "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Auth\n---\n\nBody.\n",
            )
            .unwrap();
            tmp
        };
        (make(), make())
    }

    /// A runtime that has never captured is immediately due, and a signal is
    /// what makes the *next* pass a signalled one rather than a scan.
    ///
    /// This is the coalescing the full-scan clock exists to complement: many
    /// signals before a pass still produce one signalled pass.
    #[tokio::test]
    async fn an_unchanged_runtime_skips_its_pass_and_a_signalled_one_runs() {
        use crate::constellation::runtime::{MaintenanceSupervisor, PassTrigger};

        // Another test in this binary may have exercised the shutdown path,
        // which stops admission process-wide; a pass that refused for that
        // reason would look exactly like a supervisor defect.
        crate::constellation::facade::resume_admission();

        let tmp = tempfile::TempDir::new().unwrap();
        // Canonicalised for the same reason `two_workspaces` is: `/var` is an
        // alias on macOS and the durability barrier refuses it.
        let root = tmp.path().canonicalize().unwrap();
        for dir in crate::workspace::ARTIFACT_DIRS {
            std::fs::create_dir_all(root.join(dir)).unwrap();
        }
        std::fs::write(
            root.join("prds").join("PRD-001-auth.md"),
            "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Auth\n---\n\nBody.\n",
        )
        .unwrap();
        let supervisor = MaintenanceSupervisor::register(&root);
        let capability =
            crate::constellation::facade::MaintainCapability::for_strict_maintenance(root.clone());
        let deadline = || std::time::Instant::now() + std::time::Duration::from_secs(120);

        assert!(
            supervisor.full_scan_due(),
            "a runtime that has never captured must be due immediately"
        );

        // First pass bootstraps whatever is there and marks itself scanned.
        let first = supervisor
            .capture_pass(
                &capability,
                crate::constellation::capture::CaptureLimits::default(),
                deadline(),
            )
            .await
            .expect("the first pass must run");
        assert!(first.ran);
        assert_eq!(first.trigger, Some(PassTrigger::FullScan));
        assert!(
            !supervisor.full_scan_due(),
            "a runtime that just captured is not due again until the period elapses"
        );

        // Nothing signalled, nothing due: the pass declines rather than
        // re-capturing the corpus on every watcher tick.
        let quiet = supervisor
            .capture_pass(
                &capability,
                crate::constellation::capture::CaptureLimits::default(),
                deadline(),
            )
            .await
            .expect("a declined pass is not an error");
        assert!(
            !quiet.ran,
            "an unchanged runtime must not capture again on the next tick"
        );

        // A signal produces exactly one signalled pass, no matter how many
        // signals arrived.
        supervisor.signal();
        supervisor.signal();
        supervisor.signal();
        assert!(supervisor.is_dirty());
        let signalled = supervisor
            .capture_pass(
                &capability,
                crate::constellation::capture::CaptureLimits::default(),
                deadline(),
            )
            .await
            .expect("a signalled pass must run");
        assert!(signalled.ran);
        assert_eq!(
            signalled.trigger,
            Some(PassTrigger::Signalled),
            "three signals coalesce into one signalled pass"
        );
        assert!(
            !supervisor.is_dirty(),
            "the pass must consume the signal it acted on"
        );

        // …and the one after that is quiet again.
        let after = supervisor
            .capture_pass(
                &capability,
                crate::constellation::capture::CaptureLimits::default(),
                deadline(),
            )
            .await
            .expect("a declined pass is not an error");
        assert!(!after.ran);

        supervisor.unregister();
    }
}

/// Why a pending job was **not** re-scheduled.
///
/// A gap is never a silent success. Each variant names what could not be
/// decided, so the caller can report it rather than presenting a shorter work
/// list as a complete one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryGap {
    /// The job names a canonical key the fresh capture does not contain.
    ///
    /// The source was deleted, renamed, or its identity moved between the
    /// published generation and this capture. Reported as
    /// [`DiagnosticCode::SourceChanged`] rather than a bespoke "vanished"
    /// code: **the source changed under the job** is the accurate statement,
    /// and a new variant would encode a conclusion the data cannot support —
    /// nothing here distinguishes a tombstone from a rename.
    NoCaptureEntry { code: DiagnosticCode },
    /// The capture entry exists but its eligibility cannot be decided —
    /// a required binding field is absent or unreadable.
    ///
    /// Admitting it would guess. Not admitting it silently would report a
    /// complete recovery over a source that was not fully readable.
    Unjudgeable { reason: String },
}

/// Re-schedule the pending work a previous process left behind, by
/// reconciling the published manifest against a **fresh capture**.
///
/// Returns the jobs to admit and the gaps that were not admitted.
///
/// # Why this is a reconciliation and not a replay
///
/// `GenerationManifest.pending_jobs` (`manifest.rs:84`) is a record of what
/// *may* have been outstanding, not a record of what was. Nothing in
/// [`super::jobs::EmbeddingJob`] says whether its result reached the index —
/// so a stored flag cannot be trusted even if one existed. Ready state lives on
/// the captured row (`capture.rs:198`), and eligibility is recomputed from
/// [`jobs::eligibility_for`] against the current capture.
///
/// That is what makes *"a killed in-flight job returns as pending, never
/// ready"* true **by construction**: an unpublished result cannot be mistaken
/// for a published one, because readiness is never read from the manifest.
///
/// - `Current` — already published. **Not admitted**; admitting would
///   resurrect a vector over newer content.
/// - `Eligible` — needs computing. **Admitted.** This is the killed-in-flight
///   case.
/// - `CannotJudge` — **not admitted**, reported as [`RecoveryGap::Unjudgeable`].
pub fn recover_pending(
    manifest: &super::manifest::GenerationManifest,
    capture: &super::capture::CapturedSourceView,
    body_prefix_chars: usize,
) -> (Vec<InferenceJob>, Vec<(String, RecoveryGap)>) {
    let mut admitted = Vec::new();
    let mut gaps = Vec::new();

    for (key, job) in &manifest.pending_jobs {
        // A pending job may name an artifact key or a section key
        // (`"{parent}#s{ordinal}"`). Resolve the captured input text and the
        // binding's parent entry accordingly — a section borrows its parent's
        // durable binding, so the entry lookup is always on the parent key.
        let (input_text, parent_key) = if let Some(artifact) = capture.artifact(key) {
            // `ParsedArtifact::embedding_input` (`capture.rs:179`) — the
            // accessor the facade and every other reader already use. Not
            // `embedding_input_text` called with the same fields: two call
            // sites that look identical are two places for the text to
            // drift, and a vector computed over a different string than the
            // one it will be bound to is a stale vector the binding cannot
            // detect.
            (artifact.embedding_input(body_prefix_chars), key.as_str())
        } else if let Some(section) = capture.section(key) {
            (
                capture.section_input(section, body_prefix_chars),
                section.parent_key.as_str(),
            )
        } else {
            gaps.push((
                key.clone(),
                RecoveryGap::NoCaptureEntry {
                    code: DiagnosticCode::SourceChanged,
                },
            ));
            continue;
        };

        // The binding is rebuilt from the **fresh capture**, by the same
        // function the publisher uses, with the profile the job was created
        // under. It is never read from the manifest: a binding from a previous
        // generation would decide eligibility against itself and always say
        // `Current`.
        let Some(entry) = capture
            .entries
            .iter()
            .find(|e| e.canonical_key == parent_key)
        else {
            gaps.push((
                key.clone(),
                RecoveryGap::NoCaptureEntry {
                    code: DiagnosticCode::SourceChanged,
                },
            ));
            continue;
        };

        let captured_binding = if let Some(section) = capture.section(key) {
            match super::publisher::binding_for_section(section, entry, &job.profile_id) {
                Ok(binding) => binding,
                Err(_) => {
                    gaps.push((
                        key.clone(),
                        RecoveryGap::Unjudgeable {
                            reason: format!(
                                "the captured section `{key}` produced no binding, so \
                                 eligibility cannot be judged"
                            ),
                        },
                    ));
                    continue;
                }
            }
        } else {
            match super::publisher::binding_for(entry, &job.profile_id) {
                Ok(binding) => binding,
                Err(_) => {
                    gaps.push((
                        key.clone(),
                        RecoveryGap::Unjudgeable {
                            reason: format!(
                                "the captured row for `{key}` carries no input digest, so no \
                                 binding can be built and eligibility cannot be judged"
                            ),
                        },
                    ));
                    continue;
                }
            }
        };

        // The stored side is the job's own recorded binding — that is the
        // state the previous process was working against.
        let stored = super::jobs::StoredBinding {
            index_epoch: Some(job.index_epoch),
            checkout_id: Some(job.checkout_id.clone()),
            incarnation: Some(job.incarnation),
            row_revision: Some(job.row_revision),
            source_digest: Some(job.source_digest.clone()),
            input_digest: Some(job.input_digest.clone()),
            profile_id: Some(job.profile_id.clone()),
            semantic_state: Some(entry.semantic_state.as_str().to_string()),
        };

        match super::jobs::eligibility_for(&stored, &captured_binding) {
            super::jobs::Eligibility::Eligible => {
                // A key still inside its recorded retry delay is not re-queued
                // on restart. Without this a crash-restart is the one path
                // that ignores the delay, and a permanently failing model is
                // re-attempted the moment the process comes back.
                if !job.retry_is_due(chrono::Utc::now()) {
                    continue;
                }
                // The admitted job is rebuilt from the **fresh capture's**
                // binding, carrying over the attempt counter the previous
                // process had reached. It is never the manifest's job reused:
                // that one names the bytes the previous process was working
                // against, and only the fresh binding can be published onto.
                let mut recovered = super::jobs::EmbeddingJob::for_binding(
                    &captured_binding,
                    admission_generation_for(job),
                )
                .unwrap_or_else(|_| job.clone());
                recovered.attempt = job.attempt;
                recovered.retry_after = job.retry_after.clone();
                admitted.push(InferenceJob {
                    job: recovered,
                    // Built above, once, beside the binding that will check
                    // it — artifact input or section input, never a text the
                    // digest did not cover.
                    input_text,
                });
            }
            super::jobs::Eligibility::Current => {
                // Published. Not work. Silently dropping it is correct: there
                // is nothing to report, the row is current.
            }
            super::jobs::Eligibility::CannotJudge { reason } => {
                gaps.push((key.clone(), RecoveryGap::Unjudgeable { reason }));
            }
        }
    }

    (admitted, gaps)
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "semantic-search")]
    use super::imp::Poll;
    use super::*;

    /// TEST 1 — a build without `semantic-search` refuses to construct, and
    /// constructs no engine.
    ///
    /// Fails if `spawn` succeeds, or returns anything other than
    /// `ModelUnavailable`: a non-semantic build that *could* spawn would have
    /// no engine and would accept jobs it can never run.
    ///
    /// The "spawns no thread" half is **not asserted** — there is no cheap,
    /// portable way to count threads in this crate, and I am not going to assert
    /// something I did not measure. See the module docs: the non-semantic
    /// `spawn` returns `Err` before any `Builder::spawn` is reached, which is
    /// what makes the claim true, but that is read from the source, not tested.
    #[cfg(not(feature = "semantic-search"))]
    #[test]
    fn a_build_without_semantic_search_refuses_and_reports_model_unavailable() {
        let err = match InferenceWorker::spawn() {
            Ok(_) => panic!("a non-semantic build must refuse to construct an inference worker"),
            Err(e) => e,
        };
        match err {
            ConstellationError::ModelUnavailable { detail } => {
                assert!(
                    detail.contains("semantic-search"),
                    "the report must name the missing feature, was: {detail}"
                );
            }
            other => panic!("expected ModelUnavailable, got: {other}"),
        }
    }

    /// TEST 3 — a submitted job is never silently dropped.
    ///
    /// This does **not** mock the absent cache: it calls the real `spawn` and
    /// the real `Embedder::new`. On this machine the cache is gone, so the
    /// honest outcome is a described failure; on a machine with the model, the
    /// honest outcome is a vector. Both are acceptable; a job that produces
    /// *neither* is the defect this test exists to catch.
    ///
    /// Fails if the worker panics, and fails if after the job is accounted for
    /// both channels stay `Empty` with the thread still alive — the silent
    /// drop that existed before the `failures` channel.
    #[cfg(feature = "semantic-search")]
    #[test]
    fn a_submitted_job_is_either_answered_or_reported_never_dropped() {
        let worker = InferenceWorker::spawn().expect("the worker thread must spawn");

        let submitted = worker.submit(InferenceJob {
            job: crate::constellation::jobs::EmbeddingJob {
                index_epoch: 1,
                checkout_id: "local-x".to_string(),
                canonical_key: "PRD-001".to_string(),
                incarnation: 1,
                row_revision: 1,
                source_digest: "sd".to_string(),
                input_digest: "id".to_string(),
                profile_id: "pid".to_string(),
                captured_input_ref: "gen-1:PRD-001".to_string(),
                attempt: 0,
                retry_after: None,
            },
            input_text: "some text to embed".to_string(),
        });
        assert_eq!(
            submitted,
            SubmitOutcome::Queued,
            "a job handed to a live worker must be reported as queued, so that a rejected \
                 submission cannot pass for an accepted one"
        );

        // Bounded block: a `semantic-search` build loads real weights, which
        // can take tens of seconds on a cold disk — two instant try-polls
        // would report a healthy worker as a silent drop. The deadline is a
        // wall-clock bound, not a sleep: a dropped job still fails, just after
        // the budget rather than after a fixed two polls.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(180);
        let mut answered = false;
        let mut reported = false;
        while std::time::Instant::now() < deadline {
            match worker.try_result() {
                Poll::Ready(r) => {
                    assert_eq!(
                        r.canonical_key(),
                        "PRD-001",
                        "a result must carry the key it was submitted for"
                    );
                    assert_eq!(
                        r.job.captured_input_ref, "gen-1:PRD-001",
                        "a result must carry the job it was submitted for, \
                         unchanged — a result re-pointed at another job is the \
                         rebinding this protocol removed"
                    );
                    assert!(!r.vector.is_empty(), "a ready vector must not be empty");
                    answered = true;
                }
                Poll::Disconnected => break,
                Poll::Empty => {}
            }
            match worker.try_failure() {
                Poll::Ready(detail) => {
                    assert!(
                        !detail.is_empty(),
                        "a reported failure must describe itself"
                    );
                    assert!(
                        detail.contains("PRD-001"),
                        "an inference failure must name the key it failed on: {detail}"
                    );
                    reported = true;
                }
                Poll::Disconnected => break,
                Poll::Empty => {}
            }
            if answered || reported {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        assert!(
            answered || reported,
            "the submitted job produced neither a vector nor a reported failure — \
             it was silently dropped"
        );

        worker
            .shutdown()
            .expect("shutdown must join cleanly after a settled job");
    }

    /// A worker whose engine closure is a barrier this test controls.
    ///
    /// Every test in this module that needs a call to be *in flight* builds one
    /// of these. It is a channel handshake, never a sleep: the test knows the
    /// worker is inside the closure because the closure said so, and it knows
    /// the closure is still inside because it has not released it. A sleep
    /// would prove nothing about ordering, and the ordering is the whole
    /// claim.
    #[cfg(all(feature = "semantic-search", feature = "test-helpers"))]
    struct BlockingEngine {
        entered: std::sync::mpsc::Sender<String>,
        release: std::sync::mpsc::Receiver<()>,
        finished: std::sync::Arc<std::sync::atomic::AtomicBool>,
    }

    #[cfg(all(feature = "semantic-search", feature = "test-helpers"))]
    impl BlockingEngine {
        fn new() -> (
            Self,
            std::sync::mpsc::Receiver<String>,
            std::sync::mpsc::Sender<()>,
            std::sync::Arc<std::sync::atomic::AtomicBool>,
        ) {
            let (entered_tx, entered_rx) = std::sync::mpsc::channel::<String>();
            let (release_tx, release_rx) = std::sync::mpsc::channel::<()>();
            let finished = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let flag = std::sync::Arc::clone(&finished);
            let engine = Self {
                entered: entered_tx,
                release: release_rx,
                finished: flag,
            };
            (engine, entered_rx, release_tx, finished)
        }

        fn closure(self) -> impl FnMut(&str) -> Result<Vec<f32>, String> + Send + 'static {
            move |text: &str| {
                let _ = self.entered.send(text.to_string());
                // Blocks here until the test releases it. A receiver dropped
                // by an early return would also unblock, so the flag below is
                // the honest witness that the closure ran to its tail.
                let _ = self.release.recv();
                self.finished
                    .store(true, std::sync::atomic::Ordering::SeqCst);
                Ok(vec![0.5f32; 4])
            }
        }
    }

    /// Whether the blocking call has run to its tail.
    ///
    /// A plain read of the flag the closure sets. Used to tell "still inside
    /// the call" from "the call returned and the vector is on its way back",
    /// which is the distinction a detach test has to make and cannot infer from
    /// the worker's own state — the handle that could report anything has
    /// already been consumed by `shutdown`.
    #[cfg(all(feature = "semantic-search", feature = "test-helpers"))]
    fn call_finished(flag: &std::sync::atomic::AtomicBool) -> bool {
        flag.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// The identity an inference engine is reused under must be the whole
    /// embedding space, not the model name.
    ///
    /// The failure this prevents is quiet. A chunk-size edit leaves the model
    /// name, the repo and the revision untouched and changes only what text
    /// gets embedded. A key that ignored it would hand the next drain the
    /// engine loaded for the **old** prefix, and every vector it published
    /// would be consistent in shape — 1024 unit-length floats — while
    /// describing text no row's `input_digest` matches.
    ///
    /// The other half matters just as much: the same profile twice must produce
    /// the *same* key, or every pass would reload two gigabytes and the reuse
    /// the supervisor exists for would never happen.
    #[cfg(feature = "semantic-search")]
    #[test]
    fn an_engine_is_reused_only_for_the_same_snapshot_and_the_same_chunk_size() {
        use crate::constellation::jobs::{
            DEFAULT_MAX_TOKENS, DEFAULT_MODEL_REPO, EmbeddingProfile, ResolvedProfile,
        };

        fn resolved(body_prefix_chars: usize, revision: &str) -> ResolvedProfile {
            ResolvedProfile {
                profile: EmbeddingProfile {
                    model_repo: DEFAULT_MODEL_REPO.to_string(),
                    weights_revision: revision.to_string(),
                    weights_checksums: vec!["0".repeat(64)],
                    tokenizer_checksum: "1".repeat(64),
                    tokenizer_config_checksum: "2".repeat(64),
                    dimensions: 1024,
                    engine_version: "test".to_string(),
                    pooling: "cls".to_string(),
                    normalization: "l2".to_string(),
                    query_prefix: String::new(),
                    document_prefix: String::new(),
                    body_prefix_chars,
                    max_tokens: DEFAULT_MAX_TOKENS,
                    padding: "none".to_string(),
                    truncation: "longest_first".to_string(),
                },
                snapshot_root: std::path::PathBuf::from("/cache/snapshots").join(revision),
                snapshot_id: revision.to_string(),
            }
        }

        let base = resolved(512, "rev1");
        assert_eq!(
            worker_reuse_key(&base),
            worker_reuse_key(&resolved(512, "rev1")),
            "the same snapshot and the same chunk size must reuse the engine, or every pass \
             reloads the model"
        );
        assert_ne!(
            worker_reuse_key(&base),
            worker_reuse_key(&resolved(4096, "rev1")),
            "a chunk-size change is a different embedding space and must not reuse the engine"
        );
        assert_ne!(
            worker_reuse_key(&base),
            worker_reuse_key(&resolved(512, "rev2")),
            "a moved refs/main must not reuse the engine loaded from the previous revision"
        );
    }

    #[cfg(all(feature = "semantic-search", feature = "test-helpers"))]
    fn test_job(key: &str) -> InferenceJob {
        InferenceJob {
            job: crate::constellation::jobs::EmbeddingJob {
                index_epoch: 1,
                checkout_id: "local-x".to_string(),
                canonical_key: key.to_string(),
                incarnation: 1,
                row_revision: 1,
                source_digest: "sd".to_string(),
                input_digest: "id".to_string(),
                profile_id: "pid".to_string(),
                captured_input_ref: format!("gen-1:{key}"),
                attempt: 0,
                retry_after: None,
            },
            input_text: format!("text for {key}"),
        }
    }

    /// The returned vector carries the key it was computed for, which is what
    /// lets a test tell two calls apart without a model.
    #[cfg(all(feature = "semantic-search", feature = "test-helpers"))]
    fn vector_for(text: &str) -> Vec<f32> {
        // The distinctive element is derived from the text, so a vector
        // arriving for the wrong job is visible rather than merely
        // present.
        let fill = text.bytes().fold(0.25f32, |acc, b| acc + b as f32 / 1024.0);
        vec![fill; 4]
    }

    /// Shutdown **joins** when no native call is in flight.
    ///
    /// This is the strong half of the contract and it must not be traded away:
    /// an idle worker leaves promptly once admission closes, so joining is
    /// bounded and it *proves* the thread is gone. Breaks if shutdown detaches
    /// unconditionally — a worker that has returned would then be left with a
    /// live `JoinHandle` nobody ever joins, and "joined" would stop meaning
    /// anything.
    #[cfg(all(feature = "semantic-search", feature = "test-helpers"))]
    #[test]
    fn shutdown_joins_an_idle_worker_because_joining_is_bounded_there() {
        let worker = InferenceWorker::spawn_with_engine(|_text: &str| Ok(vec![0.5f32; 4]))
            .expect("the test worker must spawn");
        // Nothing submitted, so the thread is parked in `recv` and no call can
        // be in flight: exactly the state in which a join is safe.
        assert!(
            !worker.is_in_call(),
            "an idle worker must not report a call in flight"
        );

        let outcome = worker.shutdown().expect("shutdown must not error");
        assert_eq!(
            outcome,
            ShutdownOutcome::Joined,
            "an idle worker has no call to wait for, so its thread must be joined \
             — otherwise `Joined` would stop meaning the thread is gone"
        );
    }

    /// Shutdown **detaches** while a native call is in flight, and returns.
    ///
    /// The defect this exists for: `join()` on a hung call blocks process exit
    /// for as long as the call hangs, which is unbounded. Here the call is
    /// released only *after* shutdown has returned, so a join-based
    /// implementation would deadlock and the test would fail on its timeout
    /// rather than on an assertion — which is the honest way for this failure
    /// to show up.
    ///
    /// The flip side is asserted too: the outcome is `Detached`, never `Joined`,
    /// so a caller is never told the call finished when it has not.
    #[cfg(all(feature = "semantic-search", feature = "test-helpers"))]
    #[test]
    fn shutdown_detaches_while_a_native_call_is_in_flight_and_says_so() {
        let (engine, entered, release, finished) = BlockingEngine::new();
        let worker = InferenceWorker::spawn_with_engine(engine.closure())
            .expect("the test worker must spawn");
        assert_eq!(worker.submit(test_job("PRD-001")), SubmitOutcome::Queued);

        entered
            .recv_timeout(std::time::Duration::from_secs(30))
            .expect("the worker must enter the call");

        // The release is held back until shutdown has returned, so a join here
        // is a deadlock rather than a slow pass.
        let outcome = worker
            .shutdown()
            .expect("shutdown must not error on a hung call");
        assert_eq!(
            outcome,
            ShutdownOutcome::Detached,
            "a call still in flight must be reported as detached, not joined — `Joined` here \
             would claim a call finished that has not"
        );
        assert!(
            !call_finished(&finished),
            "the detached call had not returned when shutdown reported on it"
        );

        // Release afterwards so the detached thread ends rather than lingering
        // for the rest of the test binary. Its send now fails — the handle that
        // owned the channel is gone — which is what stops it.
        release
            .send(())
            .expect("the release must reach the detached call");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        while !call_finished(&finished) && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(
            call_finished(&finished),
            "the detached thread must finish its call once released"
        );
    }

    /// A job submitted to a worker whose thread has exited is **reported**,
    /// not silently dropped.
    ///
    /// The old signature returned nothing and discarded the channel error, so a
    /// stopped worker looked exactly like one that had accepted the job — and
    /// the caller's only symptom was a drain that ran to its deadline with
    /// nothing to show. Breaks if `submit` returns `Queued` for a dead worker.
    #[cfg(all(feature = "semantic-search", feature = "test-helpers"))]
    #[test]
    fn a_submission_to_a_stopped_worker_is_reported_not_swallowed() {
        let worker = InferenceWorker::spawn_with_engine(|_text: &str| {
            Err("the model cannot be loaded".to_string())
        })
        .expect("the test worker must spawn");

        assert_eq!(
            worker.submit(test_job("PRD-001")),
            SubmitOutcome::Queued,
            "the first submission is genuinely queued — the failure comes later"
        );

        // The thread reports the failure and returns.
        let detail = failed_channel_detail(&worker);
        assert!(
            detail.contains("PRD-001"),
            "a reported inference failure must name the key it failed on: {detail}"
        );

        // Wait for the thread to actually return — that is the state a caller
        // has to be able to recognise.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        while !worker.is_stopped() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(
            worker.is_stopped(),
            "the failing thread must report itself stopped — a supervisor that cannot see this \
             will park a dead handle and refuse every submission until the process restarts"
        );

        assert_eq!(
            worker.submit(test_job("PRD-002")),
            SubmitOutcome::WorkerStopped,
            "a submission to a stopped worker must be reported as such — the old signature \
             discarded this and the caller waited for a vector that could never arrive"
        );

        assert_eq!(
            worker.shutdown().expect("shutdown must not error"),
            ShutdownOutcome::Joined,
            "a thread that already returned is joined, and joining costs nothing"
        );
    }

    /// Read one reported failure, blocking up to a bound.
    #[cfg(all(feature = "semantic-search", feature = "test-helpers"))]
    fn failed_channel_detail(worker: &InferenceWorker) -> String {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        loop {
            match worker.try_failure() {
                Poll::Ready(detail) => return detail,
                Poll::Disconnected => panic!("the failure channel closed without a detail"),
                Poll::Empty => {
                    assert!(
                        std::time::Instant::now() < deadline,
                        "a failing inference must report a failure within the budget"
                    );
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
            }
        }
    }

    /// A vector for an abandoned batch is **discarded**, not handed to the next
    /// caller.
    ///
    /// This is the resubmission-after-timeout defect. Batch A is submitted and
    /// the caller stops waiting; batch B is opened; A's vector finally arrives.
    /// Handing it to B would give B a result for a job B never issued, which B
    /// can only treat as a fatal integrity error — so a single timed-out drain
    /// would abort every drain after it.
    ///
    /// Breaks if `try_result` returns the stale vector: the assertion on the
    /// key sees `PRD-OLD` where it expects B's own `PRD-NEW`.
    #[cfg(all(feature = "semantic-search", feature = "test-helpers"))]
    #[test]
    fn a_late_vector_from_an_abandoned_batch_is_discarded_not_returned() {
        let (entered_tx, entered_rx) = std::sync::mpsc::channel::<String>();
        let (release_tx, release_rx) = std::sync::mpsc::channel::<()>();
        // Gate the FIRST call only. Gating every call deadlocked the batch's
        // own successor: the channel carries one item, so `PRD-NEW`'s call
        // blocked forever on a `recv()` that would never be satisfied, and the
        // test ended in the 30 s budget rather than in an assertion. Only the
        // stale call needs holding — the ordering the test is about is "old
        // call running, new batch opened, old call returns".
        let gate_first = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        let engine_gate = std::sync::Arc::clone(&gate_first);
        let engine_release = std::sync::Arc::new(std::sync::Mutex::new(release_rx));
        let worker = InferenceWorker::spawn_with_engine(move |text: &str| {
            let _ = entered_tx.send(text.to_string());
            if engine_gate.swap(false, std::sync::atomic::Ordering::SeqCst) {
                if let Ok(gate) = engine_release.lock() {
                    let _ = gate.recv_timeout(std::time::Duration::from_secs(30));
                }
            }
            Ok(vector_for(text))
        })
        .expect("the test worker must spawn");

        // ---- batch A, submitted and then abandoned ----
        let batch_a = worker.current_batch();
        assert_eq!(worker.submit(test_job("PRD-OLD")), SubmitOutcome::Queued);
        entered_rx
            .recv_timeout(std::time::Duration::from_secs(30))
            .expect("the worker must enter the first call");

        // The caller stops waiting and opens the next batch while A is still in
        // flight — the shape of a drain that hit its deadline.
        let batch_b = worker.begin_batch();
        assert!(
            batch_b > batch_a,
            "a new batch must be distinguishable from the one it abandoned"
        );

        // Let A finish, and queue one job under the new batch behind it. The
        // worker is single-threaded, so A's vector is sent before B's: reading
        // until `PRD-NEW` arrives observes A's answer first if it is delivered
        // at all. That is what makes the test terminate with an assertion
        // rather than with a timeout.
        release_tx
            .send(())
            .expect("the first call must be releasable");
        assert_eq!(
            worker.submit(test_job("PRD-NEW")),
            SubmitOutcome::Queued,
            "the new batch's job must be accepted — the batch that is being tested for is the \
             one that must not interfere with it"
        );

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        let result = loop {
            match worker.try_result() {
                Poll::Ready(result) => {
                    assert_ne!(
                        result.canonical_key(),
                        "PRD-OLD",
                        "a vector for the abandoned batch was handed to the caller of batch \
                         `{}`, which never issued that job",
                        worker.current_batch().0
                    );
                    break result;
                }
                Poll::Disconnected => panic!("the worker must survive a discarded late vector"),
                Poll::Empty => {
                    assert!(
                        std::time::Instant::now() < deadline,
                        "the current batch's vector must arrive within the budget"
                    );
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
            }
        };

        assert_eq!(
            result.canonical_key(),
            "PRD-NEW",
            "the current batch's vector must still be delivered after a stale one was discarded"
        );
        worker.shutdown().expect("shutdown must not error");
    }

    /// A vector for the **current** batch is still delivered — the discard is
    /// not "throw everything away".
    ///
    /// Without this, a worker that discarded indiscriminately would pass the
    /// test above while publishing nothing at all. Breaks if `try_result` drops
    /// a current-batch result or if the batch comparison is inverted.
    #[cfg(all(feature = "semantic-search", feature = "test-helpers"))]
    #[test]
    fn a_vector_for_the_current_batch_is_delivered_with_its_own_job() {
        let worker = InferenceWorker::spawn_with_engine(|text: &str| Ok(vector_for(text)))
            .expect("the test worker must spawn");

        worker.begin_batch();
        assert_eq!(worker.submit(test_job("PRD-NEW")), SubmitOutcome::Queued);

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        let result = loop {
            match worker.try_result() {
                Poll::Ready(r) => break r,
                Poll::Disconnected => panic!("the worker must answer a current-batch job"),
                Poll::Empty => {
                    assert!(
                        std::time::Instant::now() < deadline,
                        "a current-batch vector must arrive within the budget"
                    );
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
            }
        };

        assert_eq!(
            result.canonical_key(),
            "PRD-NEW",
            "the delivered vector must be the one that was computed for"
        );
        assert_eq!(
            result.job.captured_input_ref, "gen-1:PRD-NEW",
            "the result must carry the job it was computed for, unchanged"
        );
        assert_eq!(
            result.vector,
            vector_for("text for PRD-NEW"),
            "the vector must be the one computed from that job's input"
        );
        // The vector has been taken and no other job is outstanding, so no call
        // is in flight and the shutdown joins rather than detaching.
        assert_eq!(
            worker.shutdown().expect("shutdown must not error"),
            ShutdownOutcome::Joined,
            "a drained worker must join: there is nothing left to wait for"
        );
    }

    /// A queued job from an abandoned batch is never **run**.
    ///
    /// The discard above covers a vector that was already computed. This covers
    /// the job that had not started yet: running it would keep the native
    /// permit busy for work nobody wants, and — since the permit is
    /// process-wide — it would delay every other index's inference too.
    ///
    /// The closure records every text it is asked for, so the assertion is on
    /// what the engine was actually called with rather than on a count.
    #[cfg(all(feature = "semantic-search", feature = "test-helpers"))]
    #[test]
    fn a_queued_job_from_an_abandoned_batch_is_never_computed() {
        let (entered_tx, entered_rx) = std::sync::mpsc::channel::<String>();
        // A second gate the test holds: the engine reports that it has been
        // entered and then BLOCKS. Without it the closure returned instantly,
        // so "entered" only proved the call had STARTED — the worker could
        // already be past it, and past PRD-SECOND, before `begin_batch()`
        // ran. That race made the two-call expectation below depend on
        // scheduling rather than on the behaviour under test.
        let (release_tx, release_rx) = std::sync::mpsc::channel::<()>();
        let release_rx = std::sync::Arc::new(std::sync::Mutex::new(release_rx));
        let engine_release = std::sync::Arc::clone(&release_rx);
        let worker = InferenceWorker::spawn_with_engine(move |text: &str| {
            let _ = entered_tx.send(text.to_string());
            // Only the FIRST call blocks; later calls proceed, so the test
            // still observes the worker draining after the batch moved on.
            if text == "text for PRD-FIRST" {
                if let Ok(gate) = engine_release.lock() {
                    let _ = gate.recv_timeout(std::time::Duration::from_secs(30));
                }
            }
            Ok(vector_for(text))
        })
        .expect("the test worker must spawn");

        // Fill the first slot and wait until the worker is provably inside it,
        // so the second job is certainly queued rather than running.
        assert_eq!(worker.submit(test_job("PRD-FIRST")), SubmitOutcome::Queued);
        entered_rx
            .recv_timeout(std::time::Duration::from_secs(30))
            .expect("the worker must enter the first call");

        // Both of these are submitted under the batch that is about to be
        // abandoned.
        assert_eq!(worker.submit(test_job("PRD-SECOND")), SubmitOutcome::Queued);
        assert_eq!(worker.submit(test_job("PRD-THIRD")), SubmitOutcome::Queued);
        worker.begin_batch();
        // Release the first call only now that the batch has moved on.
        let _ = release_tx.send(());

        // Now submit one job under the new batch. Whatever the worker runs next
        // must be this one: the two abandoned jobs were never wanted.
        assert_eq!(
            worker.submit(test_job("PRD-CURRENT")),
            SubmitOutcome::Queued
        );

        // Observe every call the engine receives from here on. Per
        // `begin_batch`'s contract the in-flight FIRST job is DROPPED rather
        // than re-run, and the queued SECOND/THIRD jobs are skipped — so the
        // only call that may arrive is the current one.
        //
        // The earlier version waited for TWO calls and expected
        // `[FIRST, CURRENT]`. That was asking for a retry `begin_batch`
        // deliberately does not perform, and it had already consumed FIRST's
        // own entry in the entry handshake above — so the wait could only
        // ever end in a timeout, however correct the worker behaved.
        let mut seen = Vec::new();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        loop {
            match entered_rx.recv_timeout(std::time::Duration::from_millis(200)) {
                Ok(text) => {
                    seen.push(text);
                    if seen.last().map(String::as_str) == Some("text for PRD-CURRENT") {
                        break;
                    }
                }
                Err(_) => {
                    assert!(
                        std::time::Instant::now() < deadline,
                        "the worker must reach the current batch's job; it only ran {seen:?}"
                    );
                }
            }
        }

        assert_eq!(
            seen,
            vec!["text for PRD-CURRENT".to_string()],
            "an abandoned batch's queued jobs must never reach the engine — they are work \
             nobody is waiting for, and running them would hold the process-wide native permit \
             away from the job that is wanted"
        );

        // Drain the current batch's vector so the shutdown below has no call in
        // flight, then close the worker.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        loop {
            match worker.try_result() {
                Poll::Ready(_) => break,
                Poll::Disconnected => panic!("the worker must answer the current batch's job"),
                Poll::Empty => {
                    assert!(
                        std::time::Instant::now() < deadline,
                        "the current batch's vector must arrive"
                    );
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
            }
        }
        worker.shutdown().expect("shutdown must not error");
    }

    /// A second worker does not run a native call while the first is still
    /// inside one.
    ///
    /// The permit is process-wide precisely so a *fresh* worker cannot overlap
    /// with a *detached* one: the first worker here is detached mid-call, and
    /// the second is constructed afterwards. Two concurrent native calls on one
    /// process is the thing the dedicated-thread design exists to prevent, and
    /// a per-worker guarantee would not cover this case at all.
    ///
    /// Breaks if the second closure runs before the first is released: the
    /// assertion on the observed order fails.
    #[cfg(all(feature = "semantic-search", feature = "test-helpers"))]
    #[test]
    fn a_detached_calls_permit_stops_a_fresh_worker_from_running_concurrently() {
        let (first_tx, first_rx) = std::sync::mpsc::channel::<String>();
        let (release_tx, release_rx) = std::sync::mpsc::channel::<()>();
        let first = InferenceWorker::spawn_with_engine(move |text: &str| {
            let _ = first_tx.send(text.to_string());
            let _ = release_rx.recv();
            Ok(vector_for(text))
        })
        .expect("the first test worker must spawn");
        assert_eq!(
            first.submit(test_job("PRD-BLOCKING")),
            SubmitOutcome::Queued
        );
        first_rx
            .recv_timeout(std::time::Duration::from_secs(30))
            .expect("the first worker must enter its call");

        // Detach it mid-call: it stays inside native code with nobody waiting.
        assert_eq!(
            first.shutdown().expect("detach must not error"),
            ShutdownOutcome::Detached
        );

        // A fresh worker, constructed while the detached one is still inside
        // its call.
        let (second_tx, second_rx) = std::sync::mpsc::channel::<String>();
        let second = InferenceWorker::spawn_with_engine(move |text: &str| {
            let _ = second_tx.send(text.to_string());
            Ok(vector_for(text))
        })
        .expect("the second test worker must spawn");
        assert_eq!(second.submit(test_job("PRD-OTHER")), SubmitOutcome::Queued);

        // The second worker must still be waiting. A short bound is a bound on
        // the *defect*, not on correctness: if the permit were not held, the
        // second call would arrive here immediately. The closure not being
        // entered is the assertion — it is what "no concurrent native call"
        // means, and the flag alone would not show it, since a worker waiting
        // for the permit has committed to its call.
        assert!(
            second_rx
                .recv_timeout(std::time::Duration::from_millis(500))
                .is_err(),
            "the second worker ran a native call while a detached one was still inside its own — \
             two concurrent native calls on one process"
        );

        // Release the detached call; the second may then proceed.
        release_tx
            .send(())
            .expect("the detached call must be releasable");
        second_rx
            .recv_timeout(std::time::Duration::from_secs(30))
            .expect("the second worker must run once the permit is free");
        // Take the second worker's vector before stopping it, so the shutdown
        // below has no call in flight and joins rather than detaching.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        loop {
            match second.try_result() {
                Poll::Ready(_) => break,
                Poll::Disconnected => panic!("the second worker must answer its job"),
                Poll::Empty => {
                    assert!(
                        std::time::Instant::now() < deadline,
                        "the second worker must deliver its vector"
                    );
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
            }
        }
        assert_eq!(
            second.shutdown().expect("the second worker must stop"),
            ShutdownOutcome::Joined,
            "with no call in flight the thread is joined, not detached"
        );
    }

    /// A bounded job channel: the worker cannot accumulate unbounded work, and
    /// the submission that does not fit is **reported**.
    ///
    /// The engine blocks, so nothing drains the channel and the bound is what is
    /// under test. Breaks if the channel is unbounded: every submission would
    /// come back `Queued`, and the retained input text would grow with the
    /// number of events rather than with the queue bound.
    #[cfg(all(feature = "semantic-search", feature = "test-helpers"))]
    #[test]
    fn the_job_channel_is_bounded_and_reports_the_submission_it_declines() {
        let (entered_tx, entered_rx) = std::sync::mpsc::channel::<String>();
        let (release_tx, release_rx) = std::sync::mpsc::channel::<()>();
        let worker = InferenceWorker::spawn_with_engine(move |text: &str| {
            let _ = entered_tx.send(text.to_string());
            // Block: nothing is drained, so the channel fills to its bound.
            let _ = release_rx.recv();
            Ok(vector_for(text))
        })
        .expect("the test worker must spawn");

        // The first job is taken by the worker and blocks inside the call.
        assert_eq!(worker.submit(test_job("PRD-000")), SubmitOutcome::Queued);
        entered_rx
            .recv_timeout(std::time::Duration::from_secs(30))
            .expect("the worker must enter the call");

        // Exactly the bound is accepted; one more must be declined and reported.
        let mut queued = 0usize;
        let mut declined = 0usize;
        for i in 1..=READY_QUEUE_CAPACITY {
            match worker.submit(test_job(&format!("PRD-{i:03}"))) {
                SubmitOutcome::Queued => queued += 1,
                SubmitOutcome::QueueFull => declined += 1,
                other => panic!("unexpected submit outcome: {other:?}"),
            }
        }
        assert_eq!(
            queued, READY_QUEUE_CAPACITY,
            "the bounded channel must accept exactly its bound"
        );
        assert_eq!(
            declined, 0,
            "a bound-sized burst must fit without a decline"
        );
        assert_eq!(
            worker.submit(test_job("PRD-OVERFLOW")),
            SubmitOutcome::QueueFull,
            "beyond the bound the submission must be reported as declined, not silently accepted \
             — a declined job stays durably pending, so this is a deferral and not a loss"
        );

        // The declined job is still admissible once there is room, which is
        // what makes the bound a deferral rather than a drop: release the
        // blocked call so the worker drains the channel.
        release_tx
            .send(())
            .expect("the blocked call must be releasable");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        let mut re_accepted = false;
        while std::time::Instant::now() < deadline {
            if worker.submit(test_job("PRD-OVERFLOW")) == SubmitOutcome::Queued {
                re_accepted = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(
            re_accepted,
            "a declined job must become admissible again once the channel drains — \
             it stays durably pending, so nothing was lost, only deferred"
        );
        worker.shutdown().expect("the worker must stop");
    }

    /// A pending job may name a **section** key (`"{parent}#s{ordinal}"`).
    /// It must recover against the section's own input digest and input text
    /// while borrowing its parent's durable binding — a vector published
    /// under the parent's digest would be bound to bytes it was never
    /// computed from.
    #[test]
    fn a_pending_section_job_recovers_with_its_own_input_and_binding() {
        use super::super::capture::{
            CapturedSection, CapturedSourceView, SourceEntry, UnusableSource,
        };
        use super::super::jobs::{DEFAULT_BODY_PREFIX_CHARS, EmbeddingJob};
        use super::super::manifest::{GenerationManifest, TableVersions};

        let section = CapturedSection {
            canonical_key: "PRD-001#s1".to_string(),
            parent_key: "PRD-001".to_string(),
            ordinal: 1,
            heading: Some("Problem".to_string()),
            level: Some(2),
            content: std::sync::Arc::from("section body"),
            input_digest: "section-input-digest".to_string(),
            parent_input_digest: "parent-input-digest".to_string(),
        };
        let entry = SourceEntry {
            canonical_key: "PRD-001".to_string(),
            relative_path: "prds/PRD-001.md".to_string(),
            checkout_id: "local-x".to_string(),
            index_epoch: 1,
            incarnation: 1,
            row_revision: 3,
            source_digest: "sd".to_string(),
            input_digest: Some("parent-input-digest".to_string()),
            profile_id: None,
            // `ready` — the job-recovery path judges the recorded binding
            // against this state, and `unavailable` would make a matching
            // binding read as `Eligible` instead of `Current`.
            semantic_state: super::super::jobs::SemanticState::Ready,
            diagnostic_codes: Vec::new(),
            observed_at: "2026-10-17T00:00:00.000Z".to_string(),
        };
        let capture = CapturedSourceView {
            workspace: std::path::PathBuf::from("/tmp/ws/.forgeplan"),
            checkout_id: "local-x".to_string(),
            index_epoch: 1,
            snapshot_id: "snap".to_string(),
            evaluated_at: "2026-10-17T00:00:00.000Z".to_string(),
            source_set_digest: "ss".to_string(),
            config_digest: "cd".to_string(),
            body_prefix_chars: super::super::jobs::DEFAULT_BODY_PREFIX_CHARS,
            artifacts: Vec::new(),
            sections: vec![section.clone()],
            entries: vec![entry],
            unusable: Vec::<UnusableSource>::new(),
            enumerated_paths: vec!["prds/PRD-001.md".to_string()],
            coverage: super::super::capture::Coverage {
                expected: 1,
                read: 1,
                invalid: 0,
                unavailable: 0,
                excluded: 0,
            },
            diagnostics: Vec::new(),
            confirmed: true,
            filesystem_support: super::super::capture::FilesystemSupport::Supported,
            attempts: 1,
        };

        let mut manifest =
            GenerationManifest::from_capture(&capture, "gen-1", TableVersions::new(1, 1));
        // The job records the binding the previous process was working
        // against: everything matches the fresh capture *except* the
        // recorded `input_digest`, which is what makes it work rather than
        // done.
        manifest.pending_jobs.insert(
            "PRD-001#s1".to_string(),
            EmbeddingJob {
                index_epoch: 1,
                checkout_id: "local-x".to_string(),
                canonical_key: "PRD-001#s1".to_string(),
                incarnation: 1,
                row_revision: 3,
                source_digest: "sd".to_string(),
                input_digest: "older-section-input".to_string(),
                profile_id: "pid".to_string(),
                captured_input_ref: "prds/PRD-001.md".to_string(),
                attempt: 0,
                retry_after: None,
            },
        );

        let (admitted, gaps) = recover_pending(&manifest, &capture, DEFAULT_BODY_PREFIX_CHARS);
        assert!(gaps.is_empty(), "a resolvable section job is not a gap");
        assert_eq!(admitted.len(), 1);
        assert_eq!(admitted[0].canonical_key(), "PRD-001#s1");
        assert_eq!(
            admitted[0].job.input_digest, "section-input-digest",
            "the recovered job must be rebuilt from the fresh capture's binding, \
             not reused from the manifest — a manifest job names the bytes the \
             previous process was working against"
        );
        assert_eq!(
            admitted[0].input_text,
            capture.section_input(&section, DEFAULT_BODY_PREFIX_CHARS),
            "the recovered input is the section input, not the artifact prefix"
        );

        // Same binding recorded as completed → `Current`, not re-admitted.
        manifest.pending_jobs.insert(
            "PRD-001#s1".to_string(),
            EmbeddingJob {
                input_digest: "section-input-digest".to_string(),
                ..manifest.pending_jobs["PRD-001#s1"].clone()
            },
        );
        let (admitted, gaps) = recover_pending(&manifest, &capture, DEFAULT_BODY_PREFIX_CHARS);
        assert!(admitted.is_empty(), "a current section job is not work");
        assert!(gaps.is_empty());

        // A failed key still inside its retry delay is not re-queued on
        // restart — otherwise a crash-restart is the one path that ignores
        // the delay and a permanently failing model is re-attempted the
        // moment the process comes back.
        manifest.pending_jobs.insert(
            "PRD-001#s1".to_string(),
            EmbeddingJob {
                input_digest: "older-section-input".to_string(),
                attempt: 2,
                retry_after: Some(super::super::jobs::retry_after_at(2, chrono::Utc::now())),
                ..manifest.pending_jobs["PRD-001#s1"].clone()
            },
        );
        let (admitted, gaps) = recover_pending(&manifest, &capture, DEFAULT_BODY_PREFIX_CHARS);
        assert!(
            admitted.is_empty(),
            "a key inside its retry delay must not be re-queued by recovery"
        );
        assert!(gaps.is_empty(), "a delayed key is not a gap, it is waiting");

        // Once the delay has passed it is work again, with its attempt count
        // carried over rather than reset.
        manifest.pending_jobs.insert(
            "PRD-001#s1".to_string(),
            EmbeddingJob {
                attempt: 2,
                retry_after: Some("2020-01-01T00:00:00.000Z".to_string()),
                ..manifest.pending_jobs["PRD-001#s1"].clone()
            },
        );
        let (admitted, gaps) = recover_pending(&manifest, &capture, DEFAULT_BODY_PREFIX_CHARS);
        assert_eq!(admitted.len(), 1, "a due key is work again");
        assert_eq!(
            admitted[0].job.attempt, 2,
            "recovery must carry the attempt counter forward, not reset it"
        );
        assert!(gaps.is_empty());

        // A section job whose parent entry vanished is a named gap, never
        // silently dropped.
        manifest.pending_jobs.insert(
            "GHOST#s0".to_string(),
            EmbeddingJob {
                canonical_key: "GHOST#s0".to_string(),
                ..manifest.pending_jobs["PRD-001#s1"].clone()
            },
        );
        let (_admitted, gaps) = recover_pending(&manifest, &capture, DEFAULT_BODY_PREFIX_CHARS);
        assert!(
            gaps.iter().any(|(key, _)| key == "GHOST#s0"),
            "an unresolvable section job must be reported"
        );
    }

    // -------------------------------------------------------------------
    // DrainControl — the loop's authority over a drain that is waiting
    // -------------------------------------------------------------------

    /// An unbounded control yields nothing on its own.
    ///
    /// This is what a one-shot CLI drain and a git hook pass: they have no
    /// loop to serve and no control scan to keep, so the soft deadline is the
    /// only bound they honour. A control that yielded on a scan nobody would
    /// ever run would turn one bound into two and neither would be a promise.
    #[test]
    fn an_unbounded_control_asks_a_drain_to_stop_for_nothing() {
        let control = DrainControl::never();
        assert_eq!(control.give_up(), None);
        assert!(!control.stop_requested());
        // Arming is what a host does; a caller that never arms is never due.
        control.arm_scan(std::time::Instant::now() + std::time::Duration::from_secs(3_600));
        assert_eq!(control.give_up(), None);
    }

    /// An armed scan yields at the drain's next poll, and disarming stops it.
    ///
    /// The instant is what the host computes from its own supervisor clock, so
    /// this is the whole "the control scan is not delayed by a hung encoder"
    /// contract with the clock factored out.
    #[tokio::test]
    async fn an_armed_scan_yields_exactly_once_it_is_owed() {
        let control = DrainControl::never();
        assert_eq!(
            control.give_up(),
            None,
            "an arming in the future is not a yield"
        );
        control.arm_scan(std::time::Instant::now() + std::time::Duration::from_millis(40));
        assert_eq!(control.give_up(), None, "the bound has not been reached");

        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            while control.give_up().is_none() {
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("an armed bound must be reached");
        assert_eq!(control.give_up(), Some(DrainGiveUp::ScanDue));

        control.disarm_scan();
        assert_eq!(
            control.give_up(),
            None,
            "the next pass drains without an inherited bound"
        );
    }

    /// A stop outranks a due scan.
    ///
    /// Both can be true at once — a Ctrl-C arriving just as the scan comes
    /// due — and the answer must be the one that ends the process. Reporting
    /// `ScanDue` there would send the loop back for one more pass on the way
    /// down.
    #[tokio::test]
    async fn a_stop_outranks_a_due_scan_and_wakes_a_waiter_at_once() {
        let control = DrainControl::never();
        control.arm_scan(std::time::Instant::now() - std::time::Duration::from_secs(1));
        assert_eq!(control.give_up(), Some(DrainGiveUp::ScanDue));

        let woken = {
            let control = control.clone();
            tokio::spawn(async move { control.cancelled().await })
        };
        control.request_stop();
        tokio::time::timeout(std::time::Duration::from_secs(5), woken)
            .await
            .expect("a stop must wake the loop's wait, not wait for the next tick")
            .expect("the waiter must not panic");
        assert!(control.stop_requested());
        assert_eq!(
            control.give_up(),
            Some(DrainGiveUp::Stopping),
            "a stopping host has no scan left to run"
        );
    }

    /// A stop is idempotent and observable by a drain that is already waiting.
    #[tokio::test]
    async fn a_stop_is_idempotent_and_visible_to_another_task() {
        let control = DrainControl::never();
        assert!(!control.stop_requested());
        control.request_stop();
        control.request_stop();
        assert!(control.stop_requested());
        assert_eq!(control.give_up(), Some(DrainGiveUp::Stopping));
    }

    // -------------------------------------------------------------------
    // The same two yields, driven through a real blocked native call
    // -------------------------------------------------------------------

    /// A workspace whose index has published a generation, so a drain has
    /// rows to admit.
    #[cfg(all(feature = "semantic-search", feature = "test-helpers"))]
    async fn published_workspace() -> (tempfile::TempDir, super::super::facade::MaintainCapability)
    {
        crate::constellation::facade::resume_admission();
        let tmp = tempfile::TempDir::new().unwrap();
        for dir in crate::workspace::ARTIFACT_DIRS {
            std::fs::create_dir_all(tmp.path().join(dir)).unwrap();
        }
        std::fs::write(
            tmp.path().join("prds").join("PRD-001-auth.md"),
            "---\nid: PRD-001\nkind: prd\nstatus: draft\ntitle: Auth\n---\n\nBody.\n",
        )
        .unwrap();
        let capability = super::super::facade::MaintainCapability::for_strict_maintenance(
            tmp.path().canonicalize().unwrap(),
        );
        super::super::facade::maintain_index(
            &capability,
            super::super::capture::CaptureLimits::default(),
            std::time::Instant::now() + std::time::Duration::from_secs(120),
        )
        .await
        .expect("the first pass publishes a generation");
        (tmp, capability)
    }

    /// A profile no real model produces, used only so admission has an
    /// identity to admit against. No weights are involved.
    #[cfg(all(feature = "semantic-search", feature = "test-helpers"))]
    fn synthetic_profile() -> super::super::jobs::EmbeddingProfile {
        super::super::jobs::EmbeddingProfile {
            model_repo: super::super::jobs::DEFAULT_MODEL_REPO.to_string(),
            weights_revision: "test-revision".to_string(),
            weights_checksums: vec!["weights-sha256".to_string()],
            tokenizer_checksum: "tokenizer-sha256".to_string(),
            tokenizer_config_checksum: "tokenizer-config-sha256".to_string(),
            dimensions: super::super::lance_tables::EMBEDDING_DIM as u32,
            engine_version: super::super::facade::ENGINE_VERSION.to_string(),
            pooling: "mean".to_string(),
            normalization: "l2".to_string(),
            query_prefix: String::new(),
            document_prefix: String::new(),
            body_prefix_chars: 2000,
            max_tokens: 512,
            padding: "none".to_string(),
            truncation: "longest_first".to_string(),
        }
    }

    /// A drain blocked inside a native call observes a stop promptly, publishes
    /// nothing, and leaves the hung call detached with no write capability.
    ///
    /// The bound this test exists to prove is the one the soft deadline does
    /// **not** provide: the drain's deadline here is 120 s away, and the stop
    /// arrives while the encoder is inside its call. If the drain could not
    /// leave the wait, the assertion below would hold for two minutes and then
    /// fail.
    #[cfg(all(feature = "semantic-search", feature = "test-helpers"))]
    #[tokio::test]
    async fn a_stop_ends_a_drain_that_is_waiting_on_a_blocked_native_call() {
        use super::super::facade;
        let (_tmp, capability) = published_workspace().await;
        let profile = synthetic_profile();
        let (engine, entered, release, finished) = BlockingEngine::new();
        let mut worker = InferenceWorker::spawn_with_engine(engine.closure())
            .expect("the test worker must spawn");
        let control = DrainControl::never();

        // Scoped: the drain borrows the worker, and the assertion after it
        // consumes the worker.
        let outcome = {
            let drain = facade::drain_embedding_queue(
                &capability,
                &profile,
                &mut worker,
                std::time::Instant::now()
                    + std::time::Duration::from_millis(INFERENCE_SOFT_DEADLINE_MS),
                &control,
            );
            tokio::pin!(drain);

            // The worker tells the test it is inside the call. That is a
            // handshake, not a sleep: until the closure runs, a drain that
            // "returned early" would only mean it had not submitted anything.
            // The outer timeout is a bound on the fixture, not a wait for the
            // encoder — the encoder is released only after the assertions.
            tokio::time::timeout(std::time::Duration::from_secs(120), async {
                let entered = tokio::task::spawn_blocking(move || entered.recv());
                tokio::select! {
                    biased;
                    entered = entered => {
                        assert!(entered.is_ok(), "the drain must have submitted a job to the worker");
                        control.request_stop();
                    }
                    result = &mut drain => {
                        panic!("the drain finished before the encoder was entered: {result:?}");
                    }
                }
            })
            .await
            .expect("the drain must reach the worker and enter the native call");

            tokio::time::timeout(std::time::Duration::from_secs(30), drain)
                .await
                .expect("a stop must end the wait without waiting out the inference deadline")
                .expect_err("a drain that gave up must not report success")
        };
        match outcome {
            super::super::ConstellationError::IndexBusy { detail } => assert!(
                detail.contains("stopping"),
                "the drain must name why it stopped waiting: {detail}"
            ),
            other => panic!("expected an IndexBusy give-up, got: {other}"),
        }

        // Nothing was published from the abandoned batch: the rows are still
        // pending, not ready and not failed. A yield must never look like work
        // that finished, nor spend a retry attempt on a model that did not fail.
        let page = facade::list_pinned_documents(capability.index_root(), 0, 100)
            .await
            .unwrap();
        assert!(
            page.documents
                .iter()
                .all(|d| d.semantic_state.as_deref() != Some("ready")),
            "an abandoned batch must publish nothing: {:?}",
            page.documents
                .iter()
                .map(|d| (d.canonical_key.as_str(), d.semantic_state.as_deref()))
                .collect::<Vec<_>>()
        );
        assert!(
            page.documents
                .iter()
                .all(|d| d.semantic_state.as_deref() != Some("failed")),
            "a stop is not a model failure and must not burn a retry attempt"
        );

        // The hung call is detached, which is the state that says "it did not
        // finish" rather than pretending it did. Detachment is what makes the
        // late vector unreachable: nothing is left to hand it to.
        assert_eq!(
            worker
                .shutdown()
                .expect("a detached worker is not an error"),
            ShutdownOutcome::Detached,
            "a call still in flight must be reported as detached"
        );
        assert!(
            !call_finished(&finished),
            "the detached call had not returned when shutdown reported on it"
        );
        // Release afterwards so the detached thread ends rather than lingering
        // for the rest of the test binary.
        let _ = release.send(());
        let until = std::time::Instant::now() + std::time::Duration::from_secs(30);
        while !call_finished(&finished) && std::time::Instant::now() < until {
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
        assert!(
            call_finished(&finished),
            "the detached thread must finish its call once released — and its vector went \
             nowhere, because the drain abandoned that batch"
        );
    }

    /// The control scan is still owed while a drain waits, and the drain yields
    /// to it — publishing nothing and failing nothing.
    ///
    /// This is the second half of the same defect: the bound the host arms is
    /// [`FULL_SCAN_PERIOD_MS`], and a drain that ignored it would hold the loop
    /// out of the scan for as long as the encoder hangs.
    #[cfg(all(feature = "semantic-search", feature = "test-helpers"))]
    #[tokio::test]
    async fn a_due_control_scan_yields_a_drain_that_is_waiting_on_a_blocked_native_call() {
        use super::super::facade;
        let (_tmp, capability) = published_workspace().await;
        let profile = synthetic_profile();
        let (engine, entered, release, finished) = BlockingEngine::new();
        let mut worker = InferenceWorker::spawn_with_engine(engine.closure())
            .expect("the test worker must spawn");
        let control = DrainControl::never();

        let outcome = {
            let drain = facade::drain_embedding_queue(
                &capability,
                &profile,
                &mut worker,
                std::time::Instant::now()
                    + std::time::Duration::from_millis(INFERENCE_SOFT_DEADLINE_MS),
                &control,
            );
            tokio::pin!(drain);

            tokio::time::timeout(std::time::Duration::from_secs(120), async {
                let entered = tokio::task::spawn_blocking(move || entered.recv());
                tokio::select! {
                    biased;
                    entered = entered => {
                        assert!(entered.is_ok(), "the drain must have submitted a job to the worker");
                        // What `run_pass` arms once a pass has run.
                        control.arm_scan(std::time::Instant::now());
                    }
                    result = &mut drain => {
                        panic!("the drain finished before the encoder was entered: {result:?}");
                    }
                }
            })
            .await
            .expect("the drain must reach the worker and enter the native call");

            tokio::time::timeout(std::time::Duration::from_secs(30), drain)
                .await
                .expect("an owed scan must end the drain's wait")
                .expect_err("a drain that yielded must not report success")
        };
        match outcome {
            super::super::ConstellationError::IndexBusy { detail } => assert!(
                detail.contains("control scan is due"),
                "the drain must name why it stopped waiting: {detail}"
            ),
            other => panic!("expected an IndexBusy give-up, got: {other}"),
        }

        let page = facade::list_pinned_documents(capability.index_root(), 0, 100)
            .await
            .unwrap();
        assert!(
            page.documents
                .iter()
                .all(|d| !matches!(d.semantic_state.as_deref(), Some("ready") | Some("failed"))),
            "a scan-due yield leaves the rows exactly as they were — pending"
        );

        assert_eq!(
            worker
                .shutdown()
                .expect("a detached worker is not an error"),
            ShutdownOutcome::Detached
        );
        let _ = release.send(());
        let until = std::time::Instant::now() + std::time::Duration::from_secs(30);
        while !call_finished(&finished) && std::time::Instant::now() < until {
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
        assert!(call_finished(&finished));
    }
}

pub use imp::InferenceWorker;

#[cfg(feature = "semantic-search")]
pub use imp::Poll;

// ===========================================================================
// MaintenanceHost — the long-lived upkeep loop
// ===========================================================================
//
// # Why this lives here and not in the CLI
//
// `forgeplan serve`, `forgeplan watch --constellation` and the standalone
// `forgeplan-mcp` server all need the same thing: a watcher, a control-scan
// clock, a registered supervisor and a clean shutdown. The CLI cannot own it,
// because `forgeplan-mcp` depends on this crate and not the other way round —
// a host in the CLI would make the standalone server unable to keep its index
// current, and duplicating it there would produce exactly the two-publishers
// race this is meant to prevent.
//
// # What it owns, and what it does not
//
// It owns the pieces that must outlive any single request: the registered
// [`MaintenanceSupervisor`] (one per physical index, so two hosts on one index
// cannot be two publishers), the recursive debounced watcher, the coalesced
// dirty bit and the [`FULL_SCAN_PERIOD_MS`] control scan that covers a dropped
// or overflowed notification.
//
// It schedules nothing new. The queue bound, the debounce interval, the scan
// period and the inference deadline are the constants above, read rather than
// repeated.
//
// # Events are permission to look, never evidence
//
// A notification can be lost, coalesced away or overflowed, so an event only
// marks the index dirty; the answer always comes from a bounded capture. The
// control scan runs whether or not any event arrives, which is what makes a
// `git checkout`, a `git reset` or a rebase converge anyway.
//
// # The index's own writes are not events
//
// The watcher covers artifact directories only, and the index lives under
// `.forgeplan/lance/` beside the model cache and job state. A host that watched
// the workspace root would wake on its own publishes and feed itself: each
// pass would dirty the bit that scheduled the next pass.

/// Longest a single blocking wait for events lasts before the loop re-checks
/// its shutdown signal and the control scan.
///
/// Shorter than the control-scan period on purpose: the control scan is a
/// *floor* on how long a missed event can hide a change, and it can only be
/// that floor if the loop actually wakes up to notice the clock.
const HOST_TICK: std::time::Duration = std::time::Duration::from_millis(200);

/// How long one capture pass may take before it gives up and reports.
///
/// The same bound as [`INFERENCE_SOFT_DEADLINE_MS`] and deliberately the same
/// *kind* of thing: a bound on how long the host waits, never a cancellation of
/// the pass itself.
const PASS_BUDGET: std::time::Duration = std::time::Duration::from_secs(120);

/// How long a drain may hold the loop after a pass before the control scan is
/// owed again.
///
/// One [`FULL_SCAN_PERIOD_MS`] from the pass that just ran — which is the
/// supervisor's own clock, not a second one. The drain is therefore bounded by
/// the *earlier* of "the model is slow" and "the scan is due", and the scan
/// keeps its period whether or not a model is configured.
const SCAN_BOUND: std::time::Duration = std::time::Duration::from_millis(FULL_SCAN_PERIOD_MS);

/// Where a host reports what it did.
///
/// `serve` and the MCP server run behind a stdio protocol and their stderr is
/// a log; `watch` is a foreground command whose output a human is reading. The
/// passes are the same work, so the difference is presentation only, and it is
/// named rather than left to `println!` at four call sites.
///
/// stdout is never used by [`ReportSink::Background`]: on a stdio transport a
/// line on stdout is a corrupt protocol frame, not a log line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportSink {
    /// A foreground watcher: generation ids and gaps on stdout.
    Foreground,
    /// A server: the same facts on stderr, prefixed, never on stdout.
    Background { label: &'static str },
}

impl ReportSink {
    fn pass(&self, pass: &SupervisorPass) {
        match self {
            Self::Foreground => match pass.trigger {
                Some(PassTrigger::FullScan) => println!(
                    "SCAN {} — {} commit(s), full scan",
                    pass.generation_id, pass.commits
                ),
                _ => println!("INDEX {} — {} commit(s)", pass.generation_id, pass.commits),
            },
            Self::Background { label } => eprintln!(
                "{label}: {} {} — {} commit(s)",
                pass.generation_id,
                match pass.trigger {
                    Some(PassTrigger::FullScan) => "full scan",
                    Some(PassTrigger::Signalled) => "signalled",
                    None => "pass",
                },
                pass.commits
            ),
        }
    }

    fn problem(&self, message: &str) {
        match self {
            Self::Foreground => eprintln!("ERROR constellation maintenance: {message}"),
            Self::Background { label } => eprintln!("{label}: maintenance failed: {message}"),
        }
    }

    fn gap(&self, key: &str, gap: &RecoveryGap) {
        match self {
            Self::Foreground => eprintln!("WARN: {key} was not re-scheduled — {gap:?}"),
            Self::Background { label } => {
                eprintln!("{label}: {key} was not re-scheduled — {gap:?}")
            }
        }
    }

    fn note(&self, message: &str) {
        match self {
            Self::Foreground => eprintln!("{message}"),
            Self::Background { label } => eprintln!("{label}: {message}"),
        }
    }
}

/// What one turn of the loop decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tick {
    /// A capture pass ran and published.
    Passed,
    /// Nothing was due — no signal and no control scan elapsed.
    Idle,
}

/// A host running on its own task, and the handle that stops it.
pub struct MaintenanceRunner {
    stop_tx: std::sync::mpsc::Sender<()>,
    handle: tokio::task::JoinHandle<()>,
}

impl MaintenanceRunner {
    /// Ask the host to stop and wait for the task to finish.
    ///
    /// Bounded by what the host's own shutdown is bounded by: the watcher is
    /// dropped, the supervisor unregisters, and a native call still in flight
    /// is detached rather than waited on. A hung encoder therefore delays
    /// process exit by one *thread* the process is about to leave anyway, not
    /// indefinitely.
    pub async fn stop(self) {
        let _ = self.stop_tx.send(());
        let _ = self.handle.await;
    }
}

/// A workspace's upkeep, held for as long as the process needs it.
///
/// Dropping a host is not a clean stop: the watcher and the registration go
/// with it, but a pass already running belongs to the supervisor and finishes
/// on its own. Call [`MaintenanceHost::shutdown`] when the process intends to
/// stop serving — it is the only path that reports what the pass did.
pub struct MaintenanceHost {
    workspace: std::path::PathBuf,
    capability: super::facade::MaintainCapability,
    supervisor: MaintenanceSupervisor,
    /// The stop request and the control-scan bound, shared with every drain
    /// this host runs.
    ///
    /// Held here rather than created per pass so a stop requested while a
    /// native call is in flight reaches the drain that is waiting on it. It is
    /// also the same object the loop watches, which is what makes "stop was
    /// requested" one fact rather than two independent signals that can
    /// disagree.
    control: DrainControl,
    /// Held for its lifetime: dropping the debouncer stops the OS watcher.
    debouncer: notify_debouncer_mini::Debouncer<notify::RecommendedWatcher>,
    events: std::sync::mpsc::Receiver<notify_debouncer_mini::DebounceEventResult>,
    watched: std::collections::BTreeSet<String>,
    sink: ReportSink,
}

impl MaintenanceHost {
    /// Start maintaining `capability`'s workspace, and capture once before
    /// returning.
    ///
    /// The caller supplies the capability; the host never mints one.
    ///
    /// # Why the capability is a parameter
    ///
    /// [`super::facade::MaintainCapability`] is the protocol's write token —
    /// the only way to reach `maintain_index` — and it is documented as issued
    /// by the resolver's Strict branch. A host that built its own would be a
    /// back door around exactly that: maintenance reached because a path could
    /// mint one, rather than because the caller was entitled to it.
    ///
    /// The caller therefore states *why* this workspace may be maintained, and
    /// the host serves the capability it is given. The workspace it watches is
    /// the capability's workspace, so the two cannot drift: a host started for
    /// an explicit `workspace: Y` watches Y, never the process's cwd.
    ///
    /// # Errors
    ///
    /// Propagated from watcher setup, and only from watcher setup. A host that
    /// started without watching the directories would silently never see the
    /// artifacts it was started for, so a partial start is not offered — but a
    /// *failed* startup pass is reported and the host still starts, because an
    /// index that could not be brought up to date is stale rather than broken,
    /// and a stale index still answers reads honestly.
    pub async fn start(
        capability: super::facade::MaintainCapability,
        sink: ReportSink,
    ) -> Result<Self, ConstellationError> {
        let workspace = capability.workspace().to_path_buf();
        // One supervisor per physical index. `register` is idempotent, so two
        // hosts that somehow reach the same index share state and serialise
        // their passes instead of racing for one owner lock.
        let supervisor = MaintenanceSupervisor::register(&workspace);

        let (tx, events) = std::sync::mpsc::channel();
        // The runtime's own debounce bound, not an arbitrary one: it is the
        // constant the maintenance protocol defines for folding a burst of
        // atomic saves into one capture.
        let mut debouncer =
            notify_debouncer_mini::new_debouncer(std::time::Duration::from_millis(DEBOUNCE_MS), tx)
                .map_err(|e| ConstellationError::Io {
                    detail: format!("cannot create the maintenance watcher: {e}"),
                })?;
        let mut watched = std::collections::BTreeSet::new();
        watch_artifact_dirs(&workspace, debouncer.watcher(), &mut watched)?;
        if watched.is_empty() {
            return Err(ConstellationError::Io {
                detail: format!(
                    "no artifact directories found under {} — run `forgeplan init` first",
                    workspace.display()
                ),
            });
        }

        let mut host = Self {
            workspace,
            capability,
            supervisor,
            control: DrainControl::never(),
            debouncer,
            events,
            watched,
            sink,
        };
        // Startup capture. A failure here is reported and the host still
        // starts: what it may not do is stop the watcher from ever bringing the
        // index up to date afterwards.
        if let Err(e) = host.run_pass("startup").await {
            host.sink.problem(&e.to_string());
        }
        Ok(host)
    }

    /// The workspace this host maintains — the capability's, never the cwd.
    pub fn workspace(&self) -> &std::path::Path {
        &self.workspace
    }

    /// Drive this host on its own task until [`MaintenanceRunner::stop`].
    ///
    /// The join handle is returned rather than discarded on purpose. A
    /// detached task that outlived the server would keep a watcher and a
    /// registered supervisor alive after the process stopped serving, and the
    /// next process's publisher would then lose every pass to `index_busy`.
    pub fn drive(self) -> MaintenanceRunner {
        let (stop_tx, stop_rx) = std::sync::mpsc::channel::<()>();
        let handle = tokio::spawn(async move {
            let mut host = self;
            if let Err(e) = host.run_until(stop_rx).await {
                host.sink.problem(&e.to_string());
            }
            let sink = host.sink;
            if let Err(e) = host.shutdown().await {
                sink.note(&format!("maintenance host did not stop cleanly: {e}"));
            }
        });
        MaintenanceRunner { stop_tx, handle }
    }

    /// Start the host **and** drive it on its own task.
    ///
    /// [`Self::start`] runs a startup capture, so awaiting it inline would make
    /// server startup wait for a full pass — the stdio transport would not open
    /// until the index had been reconciled, which is the opposite of what a
    /// read server should do. This spawns first and starts inside the task.
    ///
    /// The returned runner is the same handle [`Self::drive`] produces, so the
    /// caller still owns shutdown and nothing outlives it.
    ///
    /// # Errors
    ///
    /// Propagated from [`Self::start`] — watcher setup, or a workspace with no
    /// artifact directories. A caller that treats maintenance as optional can
    /// report and carry on; a caller that cannot must not pretend the host is
    /// running.
    pub async fn spawn(
        capability: super::facade::MaintainCapability,
        sink: ReportSink,
    ) -> Result<MaintenanceRunner, ConstellationError> {
        let (stop_tx, stop_rx) = std::sync::mpsc::channel::<()>();
        let handle = tokio::spawn(async move {
            let mut host = match Self::start(capability, sink).await {
                Ok(host) => host,
                Err(e) => {
                    // Reported, not swallowed: the server still serves reads
                    // from whatever generation is published, and the operator
                    // needs to know the index will not be kept current.
                    eprintln!("maintenance host did not start: {e}");
                    return;
                }
            };
            if let Err(e) = host.run_until(stop_rx).await {
                host.sink.problem(&e.to_string());
            }
            if let Err(e) = host.shutdown().await {
                sink.note(&format!("maintenance host did not stop cleanly: {e}"));
            }
        });
        Ok(MaintenanceRunner { stop_tx, handle })
    }

    /// Wait for work, then do at most one capture pass.
    ///
    /// Returns [`Tick::Idle`] when the supervisor says nothing is due — which
    /// is the common case, and the reason this is not "capture on every wake".
    pub async fn tick(&mut self) -> Result<Tick, ConstellationError> {
        self.absorb_events();
        // A directory created after startup has to be watched before the next
        // pass can see what is written into it. `forgeplan init` in a second
        // worktree, or a new artifact kind, both land this way.
        self.refresh_watches()?;
        self.run_pass("tick").await
    }

    /// Run the loop until `shutdown` fires, then shut down cleanly.
    ///
    /// The caller owns the signal because only it knows what "stop" means:
    /// Ctrl-C in a foreground watcher, a drained request in a server.
    ///
    /// # Stop is observed while a pass is in flight
    ///
    /// The signal arrives on a synchronous channel, so it cannot be awaited
    /// directly. It is bridged once, here, into [`DrainControl::cancelled`]:
    /// a blocking wait on the receiver flips the control's stop flag, and both
    /// the loop's wait and any drain already awaiting a native call observe
    /// the same flag.
    ///
    /// Without that bridge, a tick could not return before its drain finished —
    /// and a drain bounded only by the 120 s soft deadline makes a Ctrl-C wait
    /// up to two minutes while the encoder hangs. The give-up **abandons** the
    /// batch (the jobs stay durably pending, a late vector is discarded by its
    /// batch token) and never interrupts a storage commit: the completion of a
    /// batch is an `await` the give-up does not observe, so a commit that has
    /// started still runs to its end.
    pub async fn run_until(
        &mut self,
        shutdown: std::sync::mpsc::Receiver<()>,
    ) -> Result<(), ConstellationError> {
        let bridge = {
            let control = self.control.clone();
            // One blocking wait per host, not per tick: it ends when the
            // sender fires or drops, and it holds nothing else.
            tokio::task::spawn_blocking(move || {
                let _ = shutdown.recv();
                control.request_stop();
            })
        };
        loop {
            if self.control.stop_requested() {
                break;
            }
            // Waiting on a synchronous receiver here blocks the Tokio worker
            // and consumes the batch before tick can mark its source dirty.
            // Leave every notification queued for tick's nonblocking drain.
            //
            // The control is raced in so a stop does not wait for this tick to
            // finish: the pass observes the same flag from inside, and yields
            // within its result poll rather than at the 120 s deadline.
            tokio::select! {
                biased;
                () = self.control.cancelled() => {}
                () = tokio::time::sleep(HOST_TICK) => {}
            }
            if self.control.stop_requested() {
                break;
            }
            if let Err(e) = self.tick().await {
                self.sink.problem(&e.to_string());
            }
        }
        // The bridge has already returned (it is what set the flag); joining it
        // keeps the blocking task's lifetime inside this call.
        let _ = bridge.await;
        Ok(())
    }

    /// Stop the host: stop signalling, let the in-flight pass finish,
    /// unregister, and stop the worker.
    ///
    /// # Four orders, all of them load-bearing
    ///
    /// 0. **A stop is requested first.** `DrainControl::request_stop` reaches
    ///    a drain that is already waiting on a native call, so `pass_finished`
    ///    below does not have to wait out the soft deadline to get the loop
    ///    back. Idempotent, so a `run_until` that already observed the signal
    ///    changes nothing.
    /// 1. **The watcher and the event channel go first.** They are fields of
    ///    `self`, dropped before this body runs, so no event can arrive at a
    ///    runtime that is stopping.
    /// 2. **An in-flight pass is awaited before the supervisor unregisters.**
    ///    [`MaintenanceSupervisor::shutdown`] is **synchronous** — it removes
    ///    the registry entry and shuts the worker down — and treating it as if
    ///    it were `async` is how "shutdown returned while a pass was still
    ///    publishing" happens. [`MaintenanceSupervisor::pass_finished`] is the
    ///    explicit wait for a pass that owns storage commits, so publication
    ///    ownership is never released underneath one.
    /// 3. **The registry entry goes before the worker stops.** A concurrent
    ///    `mark_dirty` cannot hand work to a runtime nobody is driving.
    ///
    /// A native inference call still in flight is **detached**, not waited on —
    /// the supervisor's `ShutdownOutcome` reports which happened, and a
    /// detached thread holds no write capability, so its late result cannot be
    /// published.
    pub async fn shutdown(self) -> Result<(), ConstellationError> {
        // (0). A stop requested here must reach a drain that is already
        // waiting on a native call, or `pass_finished` below would wait out
        // the soft deadline before the unregister that follows. Requesting it
        // is idempotent, so a `run_until` that already saw the signal changes
        // nothing.
        self.control.request_stop();
        // (1) and (2).
        self.supervisor.pass_finished().await;
        self.supervisor.shutdown()?;
        Ok(())
    }

    /// Fold every queued debounce batch into the dirty bit.
    fn absorb_events(&mut self) {
        while let Ok(batch) = self.events.try_recv() {
            let events = match batch {
                Ok(events) => events,
                Err(errors) => {
                    self.sink.problem(&format!("watch error: {errors:?}"));
                    mark_dirty(&self.workspace);
                    continue;
                }
            };
            if events.iter().any(|event| {
                event.kind == notify_debouncer_mini::DebouncedEventKind::Any
                    && (event
                        .path
                        .extension()
                        .is_some_and(|extension| extension == "md")
                        || event.path.file_name().is_some_and(|name| {
                            matches!(
                                name.to_str(),
                                Some("config.yaml" | ".forgeplanignore" | ".gitignore")
                            )
                        }))
            }) {
                // One bit, however many files the batch named. A burst of
                // atomic saves is one capture, not one capture per file.
                mark_dirty(&self.workspace);
            }
        }
    }

    /// Watch artifact directories that appeared after startup.
    ///
    /// The set starts as what is already watched, so an unchanged directory is
    /// never re-watched: re-registering a watch on every tick would make the
    /// backend re-deliver its backlog each time and turn the control scan into
    /// a busy loop that looks like change.
    fn refresh_watches(&mut self) -> Result<(), ConstellationError> {
        let mut watched = std::mem::take(&mut self.watched);
        let result = watch_artifact_dirs(&self.workspace, self.debouncer.watcher(), &mut watched);
        self.watched = watched;
        result
    }

    /// One bounded capture pass, and the drain that follows it.
    ///
    /// The supervisor decides whether a pass is due; this only reacts to what
    /// it reports. A failed pass leaves the published generation untouched, so
    /// the index degrades to stale rather than to torn.
    async fn run_pass(&mut self, trigger_label: &str) -> Result<Tick, ConstellationError> {
        let pass = self
            .supervisor
            .capture_pass(
                &self.capability,
                super::capture::CaptureLimits::default(),
                std::time::Instant::now() + PASS_BUDGET,
            )
            .await?;
        if !pass.ran {
            return Ok(Tick::Idle);
        }
        self.sink.pass(&pass);
        for (key, gap) in &pass.gaps {
            self.sink.gap(key, gap);
        }
        if pass.declined > 0 {
            self.sink.note(&format!(
                "note: {} pending key(s) deferred by the bounded queue; the next full scan \
                 picks them up",
                pass.declined
            ));
        }

        // A configured model drains here too: normal operation needs no manual
        // `forgeplan embed`, and an unconfigured model is a no-op. A full scan
        // only drains when recovery actually found outstanding work —
        // resolving a profile reads the snapshot's digests, and "nothing
        // changed" is the common case.
        if pass.queued > 0 || pass.trigger == Some(PassTrigger::Signalled) {
            // The scan this pass just ran is what the period counts from, so
            // the drain yields when the *next* one comes due rather than
            // sitting on the loop until the inference deadline. Without this
            // arming, a hung encoder delays the control scan by the full soft
            // deadline and the "no longer than one scan period" guarantee is
            // an accident of how fast the model usually is.
            self.control
                .arm_scan(std::time::Instant::now() + SCAN_BOUND);
            // The shared drain, not a host-private one: `serve`, `watch`, the
            // post-mutation maintenance in the CLI and this host must all reach
            // the queue the same way, or they would disagree about what "the
            // model is configured" means.
            let _ = drain_queue_if_configured(
                &self.capability,
                std::time::Instant::now()
                    + std::time::Duration::from_millis(INFERENCE_SOFT_DEADLINE_MS),
                trigger_label,
                &self.control,
            )
            .await;
            self.control.disarm_scan();
        }
        Ok(Tick::Passed)
    }
}

/// Drain the pending embedding queue **only when** a cached model is
/// configured and resolvable.
///
/// This is the "no manual `forgeplan embed`" half of maintenance, and it is
/// reachable from four places — a [`MaintenanceHost`] pass, the CLI's
/// post-mutation upkeep, a git hook and a serve tick. They all come through
/// here so they cannot disagree about what "a model is configured" means.
///
/// **Cached snapshot only, on every one of those paths.** A watch event, a
/// serve tick or a git hook must never pull gigabytes, and a model that is
/// absent has to stay visibly absent rather than turning every event into a
/// download attempt. Feature-off or unconfigured is a silent no-op: the queue
/// drains through binding-predicated completion, so a skipped drain means
/// pending rows stay pending — visible in the coverage report, never corrupted.
///
/// `label` prefixes the stderr report so `watch`, `serve`, `hook` and
/// `maintenance` lines are distinguishable.
///
/// `control` is the host's answer to "must this drain stop waiting now". A
/// caller with no loop to serve — a one-shot CLI drain, a git hook — passes
/// [`DrainControl::never`] and is bounded by `deadline` alone; a
/// [`MaintenanceHost`] passes its own, which yields the drain when the host is
/// stopping or when the control scan is owed, so neither is delayed by a hung
/// native call.
#[cfg(feature = "semantic-search")]
pub async fn drain_queue_if_configured(
    capability: &super::facade::MaintainCapability,
    deadline: std::time::Instant,
    label: &str,
    control: &DrainControl,
) -> Result<(), ConstellationError> {
    use super::{facade, jobs, storage};

    // The workspace under maintenance, not the process cwd. A background pass
    // may be serving a workspace the caller never `cd`-ed into, and the cwd's
    // config would then describe a different project's chunk size — the same
    // class of bug as resolving a profile for the wrong repository.
    //
    // A config that cannot be read is **not** silently replaced with the
    // default. The default chunk size would resolve a different `profile_id`
    // than admission uses, so every row would look stale and the drain would
    // re-embed the corpus into a space nothing else agrees with. Say so and
    // leave the rows pending, which is the honest state.
    let config = match crate::workspace::load_config(capability.workspace()) {
        Ok(config) => config,
        Err(e) => {
            eprintln!(
                "{label}: workspace config unreadable — queue skipped, rows stay pending: {e}"
            );
            return Ok(());
        }
    };
    let Some(model) = config.embedding.as_ref().map(|e| e.model.clone()) else {
        return Ok(());
    };
    let chunk_size = config
        .embedding
        .as_ref()
        .map(|e| e.chunk_size)
        .unwrap_or(jobs::DEFAULT_BODY_PREFIX_CHARS);
    let Some(repo) = crate::embed::resolve_repo(&model) else {
        eprintln!("{label}: embedding model `{model}` unsupported — queue skipped");
        return Ok(());
    };
    // The profile comes from the cached snapshot's own content, read **once**
    // and carried alongside the engine that same snapshot opens. A snapshot
    // that is not cached leaves the rows pending and says so — it never
    // resolves to a guessed profile.
    let cache_dir = crate::embed::resolve_cache_dir();
    let resolved =
        match jobs::ResolvedProfile::open(&cache_dir, repo, facade::ENGINE_VERSION, chunk_size) {
            Ok(resolved) => resolved,
            Err(e) => {
                eprintln!("{label}: profile for `{model}` unresolved — queue skipped: {e}");
                return Ok(());
            }
        };
    let profile = resolved.profile.clone();
    if storage::read_current(capability.index_root()).is_ok_and(|c| c.is_none()) {
        eprintln!("{label}: no published generation — queue skipped");
        return Ok(());
    }

    // A live runtime owns the one inference worker for this index and reuses it
    // across passes. Registering nothing (a git hook, a one-shot CLI pass)
    // still constructs and joins its own: the difference is a parked engine
    // versus a fresh one, never a second engine running at once.
    if let Some(supervisor) = MaintenanceSupervisor::registered(capability.workspace()) {
        match supervisor
            .drain_inference(capability, &model, &resolved, deadline, control)
            .await
        {
            Ok(Some(outcome)) => eprintln!(
                "{label}: queue drained — {} embedded, {} stale, {} failed, {} still eligible",
                outcome.embedded,
                outcome.stale_completions,
                outcome.failed,
                outcome.after.eligible_keys().len()
            ),
            Ok(None) => eprintln!(
                "{label}: queue drain skipped — the model is in its post-failure cooldown"
            ),
            Err(e) => eprintln!("{label}: queue drain failed: {e}"),
        }
        return Ok(());
    }

    let mut worker = match InferenceWorker::spawn_resolved(&model, &resolved) {
        Ok(worker) => worker,
        Err(e) => {
            eprintln!("{label}: embedding model `{model}` unavailable — queue skipped: {e}");
            return Ok(());
        }
    };
    let outcome =
        facade::drain_embedding_queue(capability, &profile, &mut worker, deadline, control).await;
    let shutdown = worker.shutdown();
    if matches!(&shutdown, Ok(ShutdownOutcome::Detached)) {
        eprintln!("{label}: in-flight inference thread detached; native call has not finished");
    }
    match (outcome, shutdown) {
        (Ok(outcome), Ok(_)) => eprintln!(
            "{label}: queue drained — {} embedded, {} stale, {} failed, {} still eligible",
            outcome.embedded,
            outcome.stale_completions,
            outcome.failed,
            outcome.after.eligible_keys().len()
        ),
        (Err(e), _) => eprintln!("{label}: queue drain failed: {e}"),
        (Ok(_), Err(e)) => eprintln!("{label}: inference worker did not stop cleanly: {e}"),
    }
    Ok(())
}

/// Feature-off has no embedder and no queue: a no-op that says so once, rather
/// than an error every maintenance pass would have to handle.
#[cfg(not(feature = "semantic-search"))]
pub async fn drain_queue_if_configured(
    _capability: &super::facade::MaintainCapability,
    _deadline: std::time::Instant,
    label: &str,
    _control: &DrainControl,
) -> Result<(), ConstellationError> {
    tracing::debug!("{label}: no semantic-search feature in this build — queue not drained");
    Ok(())
}

/// Watch every artifact directory that exists, and only those.
///
/// The set is what makes this incremental: an already-watched directory is not
/// re-watched. Only artifact directories — never the index, the model cache or
/// any other output of the host itself, because a host that watches its own
/// writes wakes on every publish and schedules the next pass it will publish.
///
/// # Recursive, and why it has to be
///
/// `ARTIFACT_DIRS` names the *top-level* artifact directories, and everything
/// the index actually reads lives below them: `prds/PRD-074/PRD-074-auth.md`
/// is a supported layout, and so is a grouped `prds/by-status/…`. A
/// non-recursive watch sees the directory creation and nothing inside it, so
/// the file the query is about produces no event — the change then waits for
/// the next control scan, up to a full [`FULL_SCAN_PERIOD_MS`] later. That
/// turns "no longer than one control-scan period" from a guarantee into an
/// accident, and it is exactly what the control scan exists to make safe.
fn watch_artifact_dirs(
    workspace: &std::path::Path,
    watcher: &mut dyn notify::Watcher,
    watched: &mut std::collections::BTreeSet<String>,
) -> Result<(), ConstellationError> {
    for dir_name in crate::workspace::ARTIFACT_DIRS {
        if watched.contains(*dir_name) {
            continue;
        }
        let dir = workspace.join(dir_name);
        if dir.is_dir() {
            watcher
                .watch(&dir, notify::RecursiveMode::Recursive)
                .map_err(|e| ConstellationError::Io {
                    detail: format!("cannot watch {}: {e}", dir.display()),
                })?;
            watched.insert((*dir_name).to_string());
        }
    }
    Ok(())
}
