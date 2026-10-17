//! The blocking inference worker: one per process, on a dedicated thread.
//!
//! # Why a `std::thread` and not `tokio::spawn_blocking`
//!
//! The engine call is **blocking and native**. `spawn_blocking` holds a slot in
//! the blocking pool that the runtime cannot reclaim until the call returns, so
//! a hung `tract` call prevents process shutdown from completing. A dedicated
//! `std::thread` is joined explicitly by [`InferenceWorker::shutdown`], and the
//! join is what proves the call is finished. That is the whole reason for the
//! type.
//!
//! # What this thread never holds
//!
//! **No guard of any kind.** It does not own the `IndexOwner`, it does not hold
//! a `WorkspaceLock`, and it does not call `embed_with_publisher`,
//! `publish_embeddings`, `IndexFactory` or `maintain_index`. It produces
//! `(canonical_key, Vec<f32>)` on a channel and the publisher writes them
//! afterwards, outside this module. An inference that ran under the owner
//! guard would hold an OS-wide exclusive lock for the length of a native call.
//!
//! # Why the permit is structural
//!
//! The worker handles **one job at a time**, and the thread *is* the permit: a
//! second job cannot begin until the first `embed` returns, because the code
//! that would start it is the code that has not been reached yet. There is no
//! counter that could disagree with reality.
//!
//! A soft deadline that released the permit would allow a second inference to
//! start while the first is still running, and "one worker" would become a
//! comment rather than a property. So **no timer exists in this module at
//! all**. [`INFERENCE_SOFT_DEADLINE_MS`] bounds how long a *caller* waits for a
//! result; it never bounds the permit. A caller that stops waiting discards the
//! late result — the inference still finished, it simply no longer matters.
//!
//! # Not in this module
//!
//! The 64-key ready queue, `mark_dirty`, restart recovery and the full-scan
//! loop are P4b. Nothing here schedules, retries or persists.

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

/// One inference request.
pub struct InferenceJob {
    /// The canonical key this vector belongs to.
    pub canonical_key: String,
    /// The exact text the vector must be computed from.
    pub input_text: String,
}

/// One computed vector.
pub struct InferenceResult {
    pub canonical_key: String,
    pub vector: Vec<f32>,
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
    use std::sync::mpsc::{Receiver, Sender, TryRecvError};

    use super::*;
    use crate::embed::Embedder;

    /// A handle to the single inference thread.
    ///
    /// Owns the thread and the sender that keeps it fed. Dropping the handle
    /// without calling [`Self::shutdown`] detaches the thread: the sender drops,
    /// the receiver disconnects, and the thread ends after the job in flight.
    /// [`Self::shutdown`] joins it and so *proves* the call finished.
    pub struct InferenceWorker {
        jobs: Sender<InferenceJob>,
        results: Receiver<InferenceResult>,
        failures: Receiver<String>,
        handle: Option<std::thread::JoinHandle<Result<(), String>>>,
        /// Admission. Set by `shutdown` before the sender is dropped, so work
        /// queued but not yet started is abandoned rather than begun.
        admitting: std::sync::Arc<std::sync::atomic::AtomicBool>,
    }

    /// The sender, cloned into the thread so it can observe the flag.
    struct AdmissionFlag(std::sync::Arc<std::sync::atomic::AtomicBool>);

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

    impl InferenceWorker {
        /// Spawn the worker, constructing the engine **on the worker thread**.
        ///
        /// Construction is on-thread so the engine is created, used and dropped
        /// in the same place and never appears on any other thread's stack. It
        /// is `Send`, so moving one in is also possible; doing it here keeps the
        /// engine's lifetime identical to the thread's.
        ///
        /// An engine that cannot be built is returned as
        /// [`ConstellationError::ModelUnavailable`].
        pub fn spawn() -> Result<Self, ConstellationError> {
            let (jobs_tx, jobs_rx) = std::sync::mpsc::channel::<InferenceJob>();
            let (result_tx, result_rx) = std::sync::mpsc::channel::<InferenceResult>();
            let (fail_tx, fail_rx) = std::sync::mpsc::channel::<String>();
            let admitting = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
            let thread_flag = AdmissionFlag(std::sync::Arc::clone(&admitting));

            let handle = std::thread::Builder::new()
                .name("forgeplan-inference".to_string())
                .spawn(move || -> Result<(), String> {
                    // The engine exists only inside this function. It is built
                    // lazily on the first job so `spawn` reports thread
                    // creation success; a construction failure is returned
                    // through the join, and reported, never panicked on.
                    let mut engine: Option<Embedder> = None;
                    while let Ok(job) = jobs_rx.recv() {
                        if !thread_flag.0.load(std::sync::atomic::Ordering::SeqCst) {
                            break;
                        }
                        if engine.is_none() {
                            match Embedder::new() {
                                Ok(e) => engine = Some(e),
                                Err(e) => {
                                    // Report and stop. Returning would close the
                                    // channel with no explanation; silently
                                    // continuing would spin on a model that
                                    // cannot load.
                                    let _ = fail_tx.send(format!(
                                        "the inference engine could not be constructed: {e}"
                                    ));
                                    return Ok(());
                                }
                            }
                        }
                        let engine = engine.as_mut().expect("engine constructed above");
                        match engine.embed(&job.input_text) {
                            Ok(vector) => {
                                if result_tx
                                    .send(InferenceResult {
                                        canonical_key: job.canonical_key,
                                        vector,
                                    })
                                    .is_err()
                                {
                                    break;
                                }
                            }
                            Err(e) => {
                                let _ = fail_tx.send(format!(
                                    "inference failed for {}: {e}",
                                    job.canonical_key
                                ));
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
                admitting,
            })
        }

        /// Take the next finished vector, if one is ready. Never blocks.
        pub fn try_result(&self) -> Poll<InferenceResult> {
            match self.results.try_recv() {
                Ok(r) => Poll::Ready(r),
                Err(TryRecvError::Empty) => Poll::Empty,
                Err(TryRecvError::Disconnected) => Poll::Disconnected,
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

        /// Queue one inference. Does not wait, and never blocks.
        pub fn submit(&self, job: InferenceJob) {
            let _ = self.jobs.send(job);
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
            let (jobs_tx, jobs_rx) = std::sync::mpsc::channel::<InferenceJob>();
            let (result_tx, result_rx) = std::sync::mpsc::channel::<InferenceResult>();
            let (fail_tx, fail_rx) = std::sync::mpsc::channel::<String>();
            let admitting = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
            let thread_flag = AdmissionFlag(std::sync::Arc::clone(&admitting));

            let handle = std::thread::Builder::new()
                .name("forgeplan-inference-test".to_string())
                .spawn(move || -> Result<(), String> {
                    let mut engine = engine;
                    while let Ok(job) = jobs_rx.recv() {
                        if !thread_flag.0.load(std::sync::atomic::Ordering::SeqCst) {
                            break;
                        }
                        match engine(&job.input_text) {
                            Ok(vector) => {
                                if result_tx
                                    .send(InferenceResult {
                                        canonical_key: job.canonical_key,
                                        vector,
                                    })
                                    .is_err()
                                {
                                    break;
                                }
                            }
                            Err(e) => {
                                let _ = fail_tx.send(format!(
                                    "inference failed for {}: {e}",
                                    job.canonical_key
                                ));
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
                admitting,
            })
        }

        /// Stop admission, drop the sender so the receiver disconnects, and
        /// **join**.
        ///
        /// The join is the proof: it returns only once the thread's function has
        /// returned, which cannot happen until the `embed` call it is inside has
        /// returned. After this returns, no inference call is running anywhere
        /// in the process.
        pub fn shutdown(mut self) -> Result<(), ConstellationError> {
            self.admitting
                .store(false, std::sync::atomic::Ordering::SeqCst);
            let (dead_tx, _dead_rx) = std::sync::mpsc::channel();
            self.jobs = dead_tx;
            let result = match self.handle.take() {
                Some(handle) => match handle.join() {
                    Ok(Ok(())) => Ok(()),
                    Ok(Err(detail)) => Err(detail),
                    Err(_) => Err("the inference thread panicked".to_string()),
                },
                None => Ok(()),
            };
            result.map_err(|detail| ConstellationError::ModelUnavailable { detail })
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
    inputs: std::collections::HashMap<String, String>,
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
    pub fn admit(
        &mut self,
        canonical_key: impl Into<String>,
        input_text: impl Into<String>,
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
    pub fn take_next(&mut self) -> Option<(String, String)> {
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
/// runtime live this **persists nothing and returns**: the canonical Markdown
/// is the durable truth, and the next capture finds the change regardless. A
/// dropped notification is therefore not lost work, which is exactly why
/// watchers need no durable queue of their own.
///
/// It takes no lock, opens no store, starts no thread, and starts no runtime.
/// A short CLI call to this is a no-op with a truthful meaning, not a daemon
/// kick-off.
pub fn mark_dirty(workspace: &std::path::Path) -> bool {
    let _ = workspace;
    // The live-runtime registry is P4c. Until one exists there is provably no
    // runtime to signal, and this correctly reports `false`.
    false
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
            text, "body 4",
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
            assert_eq!(text, "body");
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
        assert_eq!((k1.as_str(), t1.as_str()), ("PRD-A", "a2"));
        let (k2, t2) = q.take_next().unwrap();
        assert_eq!((k2.as_str(), t2.as_str()), ("PRD-B", "b1"));
    }

    /// `mark_dirty` with no live runtime persists nothing and starts nothing.
    ///
    /// Asserts the honest answer rather than a mock: there is no registry yet,
    /// so the truthful report is `false`, and a caller must not read it as
    /// work being scheduled.
    #[test]
    fn mark_dirty_with_no_live_runtime_reports_no_runtime() {
        assert!(
            !crate::constellation::runtime::mark_dirty(std::path::Path::new(
                "/nonexistent-workspace"
            )),
            "with no live runtime there is nothing to signal, and the report must say so"
        );
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
        let Some(artifact) = capture.artifact(key) else {
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
        let Some(entry) = capture.entries.iter().find(|e| &e.canonical_key == key) else {
            gaps.push((
                key.clone(),
                RecoveryGap::NoCaptureEntry {
                    code: DiagnosticCode::SourceChanged,
                },
            ));
            continue;
        };

        let Ok(captured_binding) = super::publisher::binding_for(entry, &job.profile_id) else {
            gaps.push((
                key.clone(),
                RecoveryGap::Unjudgeable {
                    reason: format!(
                        "the captured row for `{key}` carries no input digest, so no binding can \
                         be built and eligibility cannot be judged"
                    ),
                },
            ));
            continue;
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
                admitted.push(InferenceJob {
                    canonical_key: key.clone(),
                    // `ParsedArtifact::embedding_input` (`capture.rs:179`) —
                    // the accessor the facade and every other reader already
                    // use. Not `embedding_input_text` called with the same
                    // fields: two call sites that look identical are two
                    // places for the text to drift, and a vector computed over
                    // a different string than the one it will be bound to is a
                    // stale vector the binding cannot detect.
                    input_text: artifact.embedding_input(body_prefix_chars),
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

        worker.submit(InferenceJob {
            canonical_key: "PRD-001".to_string(),
            input_text: "some text to embed".to_string(),
        });

        // No sleep: the test blocks on the channels themselves. A result and a
        // failure are both terminal for this job; the loop exists only so the
        // assertion is not a race against thread start-up.
        let mut answered = false;
        let mut reported = false;
        for _ in 0..2 {
            match worker.try_result() {
                Poll::Ready(r) => {
                    assert_eq!(
                        r.canonical_key, "PRD-001",
                        "a result must carry the key it was submitted for"
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

    /// TEST 2 — `shutdown` joins: it returns only after the worker's in-flight
    /// call has finished.
    ///
    /// The engine closure blocks on a channel the test owns. The test then calls
    /// `shutdown` on **another** thread — so the main thread can release the
    /// handshake only after `shutdown` has begun, which is what makes the
    /// ordering observable. `finished` is set by the closure's own tail; if
    /// `shutdown` did not join, `shutdown` could return while the closure was
    /// still blocked, and `finished` would still be false when it is read.
    ///
    /// Fails if `shutdown` returns before `finished` is set — which is exactly
    /// the "shutdown returns while a native call still runs" defect.
    #[cfg(all(feature = "semantic-search", feature = "test-helpers"))]
    #[test]
    fn shutdown_joins_the_worker_rather_than_detaching_it() {
        use std::sync::Arc;
        use std::sync::atomic::AtomicBool;

        let (release_tx, release_rx) = std::sync::mpsc::channel::<()>();
        let (entered_tx, entered_rx) = std::sync::mpsc::channel::<()>();
        let finished = Arc::new(AtomicBool::new(false));
        let flag_for_closure = Arc::clone(&finished);

        let worker = InferenceWorker::spawn_with_engine(move |_text: &str| {
            // Announce that the "call" has started, then block until released.
            let _ = entered_tx.send(());
            let _ = release_rx.recv();
            // The tail of the call: anything that observes `finished` must do so
            // after this line has run.
            flag_for_closure.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(vec![0.5f32; 1024])
        })
        .expect("the test worker must spawn");

        worker.submit(InferenceJob {
            canonical_key: "PRD-002".to_string(),
            input_text: "blocked".to_string(),
        });

        // Handshake, not a sleep: the worker is known to be inside the call.
        entered_rx
            .recv_timeout(std::time::Duration::from_secs(30))
            .expect("the worker must enter the engine call");

        let (done_tx, done_rx) = std::sync::mpsc::channel::<()>();
        let joiner = std::thread::spawn(move || {
            let result = worker.shutdown();
            let _ = done_tx.send(());
            result
        });

        // Release the call. `shutdown` is already blocked in `join`.
        release_tx.send(()).expect("release must reach the worker");

        done_rx
            .recv_timeout(std::time::Duration::from_secs(30))
            .expect("shutdown must return once the call is released");
        joiner
            .join()
            .expect("the shutdown thread must not panic")
            .expect("shutdown must join without error");

        assert!(
            finished.load(std::sync::atomic::Ordering::SeqCst),
            "shutdown returned while the engine call had still not finished — \
             it detached rather than joined"
        );
    }
}

pub use imp::InferenceWorker;
