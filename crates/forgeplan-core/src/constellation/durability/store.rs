//! `DurabilityTrackingStore` — the wrapper that makes Lance's writes
//! observable.
//!
//! # Why this exists at all
//!
//! `lance_io::object_store::WrappingObjectStore` hands a store an inner
//! `Arc<dyn ObjectStore>` and gets back a replacement. `NativeTable::create`,
//! `create_empty` and `open_with_params` all take it as
//! `write_store_wrapper`, and `ReadParams::store_options.object_store_wrapper`
//! installs it on reopen. That is the entire supported seam, and there is no
//! second one: a core boundary that opened a table any other way would write
//! straight past this tracker.
//!
//! # The three methods that must be overridden even though they have defaults
//!
//! `ObjectStore` provides `put`, `put_multipart` and `rename` in terms of
//! `put_opts`, `put_multipart_opts`, `copy` and `delete`. Their defaults
//! would therefore still reach our overrides — but only for as long as the
//! call arrives at *this* instance. `put_multipart` in particular is
//! documented as defaulting to `put_multipart_opts`, and `rename` to
//! `copy` + `delete`: both are exactly the shape where a future default could
//! grow a fast path that bypasses the tracker. Overriding them here is what
//! makes "no mutation reaches the inner store untracked" a property of this
//! file rather than a property of someone else's default methods.
//!
//! # Reads never mutate
//!
//! `get`, `get_opts`, `get_range`, `get_ranges`, `head`, `list`,
//! `list_with_offset` and `list_with_delimiter` are delegated untouched. They
//! cannot register intent, and they must not: a read that marked a file as
//! written would put unproven data into the closure barrier.

use std::fmt;
use std::sync::Arc;

use async_trait::async_trait;
use futures::stream::BoxStream;
use object_store::path::Path as ObjectPath;
// `UploadPart` is re-exported at the crate root (`object_store-0.12.5/src/lib.rs:567`
// is `pub use upload::*`), and the `upload` module itself is private.
use object_store::UploadPart;
use object_store::{
    GetOptions, GetResult, ListResult, MultipartUpload, ObjectMeta, ObjectStore,
    PutMultipartOptions, PutOptions, PutPayload, PutResult, Result as ObjectStoreResult,
};

use super::StageTracker;

/// Store wrapper that records every mutation into a shared [`StageTracker`].
///
/// One instance wraps one inner store; **one [`StageTracker`] is shared by
/// every wrapper in a generation** — the artifacts table and the relations
/// table each get a wrapper, and both point at the same tracker. Two trackers
/// would each see half the closure, and `finish_writes` would report the
/// other half as untouched.
pub struct DurabilityTrackingStore {
    inner: Arc<dyn ObjectStore>,
    tracker: Arc<StageTracker>,
    /// The store prefix Lance passed to `wrap`. Kept so the diagnostic
    /// identity of a wrapper is the store it actually wraps.
    store_prefix: String,
}

impl fmt::Debug for DurabilityTrackingStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DurabilityTrackingStore")
            .field("store_prefix", &self.store_prefix)
            .field("inner", &self.inner.to_string())
            .finish()
    }
}

impl fmt::Display for DurabilityTrackingStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DurabilityTrackingStore({})", self.store_prefix)
    }
}

impl DurabilityTrackingStore {
    /// Wrap `inner`, recording into `tracker`.
    pub fn new(
        inner: Arc<dyn ObjectStore>,
        tracker: Arc<StageTracker>,
        store_prefix: impl Into<String>,
    ) -> Self {
        Self {
            inner,
            tracker,
            store_prefix: store_prefix.into(),
        }
    }

    /// The shared tracker. Both tables reach the same one through this.
    pub fn tracker(&self) -> &Arc<StageTracker> {
        &self.tracker
    }
}

#[async_trait]
impl ObjectStore for DurabilityTrackingStore {
    // ---- mutations -------------------------------------------------------

    async fn put_opts(
        &self,
        location: &ObjectPath,
        payload: PutPayload,
        opts: PutOptions,
    ) -> ObjectStoreResult<PutResult> {
        let key = location.to_string();
        // Intent first: a crash mid-write must leave an unresolved
        // destination behind, never nothing at all.
        self.tracker.register_intent(&key);
        match self.inner.put_opts(location, payload, opts).await {
            Ok(result) => {
                self.tracker.record_write(&key);
                Ok(result)
            }
            Err(e) => {
                self.tracker.poison(format!("put_opts {key}: {e}"));
                Err(e)
            }
        }
    }

    /// Overridden even though `ObjectStore` provides it.
    async fn put(
        &self,
        location: &ObjectPath,
        payload: PutPayload,
    ) -> ObjectStoreResult<PutResult> {
        let key = location.to_string();
        self.tracker.register_intent(&key);
        match self
            .inner
            .put_opts(location, payload, PutOptions::default())
            .await
        {
            Ok(result) => {
                self.tracker.record_write(&key);
                Ok(result)
            }
            Err(e) => {
                self.tracker.poison(format!("put {key}: {e}"));
                Err(e)
            }
        }
    }

    /// Overridden even though `ObjectStore` provides it.
    async fn put_multipart(
        &self,
        location: &ObjectPath,
    ) -> ObjectStoreResult<Box<dyn MultipartUpload>> {
        let key = location.to_string();
        self.tracker.register_intent(&key);
        match self
            .inner
            .put_multipart_opts(location, PutMultipartOptions::default())
            .await
        {
            Ok(upload) => Ok(Box::new(TrackedMultipartUpload {
                inner: upload,
                tracker: Arc::clone(&self.tracker),
                location: key,
                settled: false,
            })),
            Err(e) => {
                self.tracker.poison(format!("put_multipart {key}: {e}"));
                Err(e)
            }
        }
    }

    async fn put_multipart_opts(
        &self,
        location: &ObjectPath,
        opts: PutMultipartOptions,
    ) -> ObjectStoreResult<Box<dyn MultipartUpload>> {
        let key = location.to_string();
        self.tracker.register_intent(&key);
        match self.inner.put_multipart_opts(location, opts).await {
            Ok(upload) => Ok(Box::new(TrackedMultipartUpload {
                inner: upload,
                tracker: Arc::clone(&self.tracker),
                location: key,
                settled: false,
            })),
            Err(e) => {
                self.tracker
                    .poison(format!("put_multipart_opts {key}: {e}"));
                Err(e)
            }
        }
    }

    async fn copy(&self, from: &ObjectPath, to: &ObjectPath) -> ObjectStoreResult<()> {
        let (from_key, to_key) = (from.to_string(), to.to_string());
        self.tracker.register_intent(&from_key);
        match self.inner.copy(from, to).await {
            Ok(()) => {
                self.tracker.record_copy(&from_key, &to_key);
                Ok(())
            }
            Err(e) => {
                self.tracker
                    .poison(format!("copy {from_key} -> {to_key}: {e}"));
                Err(e)
            }
        }
    }

    async fn copy_if_not_exists(
        &self,
        from: &ObjectPath,
        to: &ObjectPath,
    ) -> ObjectStoreResult<()> {
        let (from_key, to_key) = (from.to_string(), to.to_string());
        self.tracker.register_intent(&from_key);
        match self.inner.copy_if_not_exists(from, to).await {
            Ok(()) => {
                self.tracker.record_copy(&from_key, &to_key);
                Ok(())
            }
            Err(e) => {
                self.tracker
                    .poison(format!("copy_if_not_exists {from_key} -> {to_key}: {e}"));
                Err(e)
            }
        }
    }

    /// Overridden even though `ObjectStore` provides it as `copy` + `delete`.
    ///
    /// Going through our own `copy` and `delete` would work today, but the
    /// provided body is precisely the kind of default that could later gain a
    /// fast path. Tracking both endpoints directly also records **both**
    /// parents, which the copy-then-delete default would record as two
    /// unrelated mutations.
    async fn rename(&self, from: &ObjectPath, to: &ObjectPath) -> ObjectStoreResult<()> {
        let (from_key, to_key) = (from.to_string(), to.to_string());
        self.tracker.register_intent(&from_key);
        match self.inner.rename(from, to).await {
            Ok(()) => {
                self.tracker.record_rename(&from_key, &to_key);
                Ok(())
            }
            Err(e) => {
                self.tracker
                    .poison(format!("rename {from_key} -> {to_key}: {e}"));
                Err(e)
            }
        }
    }

    /// Overridden even though `ObjectStore` provides it, for the same reason.
    async fn rename_if_not_exists(
        &self,
        from: &ObjectPath,
        to: &ObjectPath,
    ) -> ObjectStoreResult<()> {
        let (from_key, to_key) = (from.to_string(), to.to_string());
        self.tracker.register_intent(&from_key);
        match self.inner.rename_if_not_exists(from, to).await {
            Ok(()) => {
                self.tracker.record_rename(&from_key, &to_key);
                Ok(())
            }
            Err(e) => {
                self.tracker
                    .poison(format!("rename_if_not_exists {from_key} -> {to_key}: {e}"));
                Err(e)
            }
        }
    }

    async fn delete(&self, location: &ObjectPath) -> ObjectStoreResult<()> {
        let key = location.to_string();
        self.tracker.register_intent(&key);
        match self.inner.delete(location).await {
            Ok(()) => {
                self.tracker.record_delete(&key);
                Ok(())
            }
            Err(e) => {
                self.tracker.poison(format!("delete {key}: {e}"));
                Err(e)
            }
        }
    }

    /// Overridden even though `ObjectStore` provides it.
    ///
    /// The provided body fans the stream out concurrently, which would let
    /// deletes interleave with each other but still had to pass through our
    /// `delete` to be seen. Driving the inner store directly here keeps the
    /// one-threaded ordering the tracker needs and, more importantly, means a
    /// failure anywhere in the stream poisons the stage exactly once, at the
    /// place it happened.
    fn delete_stream<'a>(
        &'a self,
        locations: BoxStream<'a, ObjectStoreResult<ObjectPath>>,
    ) -> BoxStream<'a, ObjectStoreResult<ObjectPath>> {
        let inner = Arc::clone(&self.inner);
        let tracker = Arc::clone(&self.tracker);
        Box::pin(futures::stream::unfold(
            (inner, tracker, locations),
            |(inner, tracker, mut locations)| async move {
                use futures::StreamExt;
                let location = locations.next().await?;
                let outcome = match location {
                    Ok(path) => {
                        let key = path.to_string();
                        tracker.register_intent(&key);
                        match inner.delete(&path).await {
                            Ok(()) => {
                                tracker.record_delete(&key);
                                Ok(path)
                            }
                            Err(e) => {
                                tracker.poison(format!("delete_stream {key}: {e}"));
                                Err(e)
                            }
                        }
                    }
                    Err(e) => Err(e),
                };
                Some((outcome, (inner, tracker, locations)))
            },
        ))
    }

    // ---- reads: delegated untouched --------------------------------------
    //
    // `get`, `get_range`, `get_ranges` and `head` are **not** overridden.
    // `ObjectStore` gives each of them a provided implementation built on
    // `get_opts`, which is the only read without a default — so they already
    // route through the single method that delegates, and overriding them
    // would add three chances to write a fast path by mistake.

    async fn get_opts(
        &self,
        location: &ObjectPath,
        options: GetOptions,
    ) -> ObjectStoreResult<GetResult> {
        self.inner.get_opts(location, options).await
    }

    fn list(
        &self,
        prefix: Option<&ObjectPath>,
    ) -> BoxStream<'static, ObjectStoreResult<ObjectMeta>> {
        self.inner.list(prefix)
    }

    fn list_with_offset(
        &self,
        prefix: Option<&ObjectPath>,
        offset: &ObjectPath,
    ) -> BoxStream<'static, ObjectStoreResult<ObjectMeta>> {
        self.inner.list_with_offset(prefix, offset)
    }

    async fn list_with_delimiter(
        &self,
        prefix: Option<&ObjectPath>,
    ) -> ObjectStoreResult<ListResult> {
        self.inner.list_with_delimiter(prefix).await
    }
}

impl lance_io::object_store::WrappingObjectStore for DurabilityTrackingStore {
    fn wrap(&self, store_prefix: &str, original: Arc<dyn ObjectStore>) -> Arc<dyn ObjectStore> {
        // A fresh wrapper per `wrap`, all pointing at the same tracker: this
        // is what makes one tracker serve both tables.
        Arc::new(DurabilityTrackingStore::new(
            original,
            Arc::clone(&self.tracker),
            store_prefix,
        ))
    }
}

/// Multipart upload whose completion and abort both reach the tracker.
///
/// The intent was registered when the upload was *created*, not when it
/// completed. An upload that is created and then dropped — or aborted after a
/// partial failure — therefore leaves an unresolved destination rather than
/// silently disappearing from the record.
/// `Debug` is required by `MultipartUpload: Send + std::fmt::Debug`
/// (`object_store-0.12.5/src/upload.rs:41`), so it is derived rather than
/// hand-written.
#[derive(Debug)]
struct TrackedMultipartUpload {
    inner: Box<dyn MultipartUpload>,
    tracker: Arc<StageTracker>,
    location: String,
    /// Guards against a double settle: `complete` then `abort` on the same
    /// handle must not record a write and a poison in the same stage.
    settled: bool,
}

#[async_trait]
impl MultipartUpload for TrackedMultipartUpload {
    fn put_part(&mut self, data: PutPayload) -> UploadPart {
        // Parts are opaque handles; only `complete` turns them into an
        // object, and only `complete` and `abort` change the closure.
        self.inner.put_part(data)
    }

    async fn complete(&mut self) -> ObjectStoreResult<PutResult> {
        let result = self.inner.complete().await;
        if self.settled {
            self.tracker.poison(format!(
                "multipart complete after settle: {}",
                self.location
            ));
            return result;
        }
        self.settled = true;
        match &result {
            Ok(_) => self.tracker.record_write(&self.location),
            Err(e) => self
                .tracker
                .poison(format!("multipart complete {}: {e}", self.location)),
        }
        result
    }

    async fn abort(&mut self) -> ObjectStoreResult<()> {
        let result = self.inner.abort().await;
        if self.settled {
            self.tracker
                .poison(format!("multipart abort after settle: {}", self.location));
            return result;
        }
        self.settled = true;
        // An abort is not a success and not a no-op: the stage cannot be
        // trusted afterwards, because we cannot know how far the inner store
        // got before it was torn down.
        self.tracker
            .poison(format!("multipart abort {}", self.location));
        result
    }
}

/// Test helper that drives a multipart write end to end.
///
/// `#[cfg(test)]` because nothing in production calls it: our writer path uses
/// `merge_insert`, and Lance — not this crate — reaches `put_multipart` on a
/// wrapped store. Keeping it as a plain `pub(crate)` helper would be a
/// production API with no production caller. It stays compiled so the
/// success path of [`TrackedMultipartUpload::complete`] is still covered.
#[cfg(test)]
pub(crate) async fn write_multipart_tracked(
    store: &dyn ObjectStore,
    location: &ObjectPath,
    chunks: Vec<PutPayload>,
) -> ObjectStoreResult<PutResult> {
    let mut upload = store.put_multipart(location).await?;
    let mut parts = Vec::with_capacity(chunks.len());
    for chunk in chunks {
        parts.push(upload.put_part(chunk));
    }
    // Parts are awaited to completion before `complete`, which is what makes
    // the object exist; the tracker only learns about it here.
    for part in parts {
        part.await?;
    }
    upload.complete().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use object_store::memory::InMemory;

    fn wrapped() -> (
        Arc<StageTracker>,
        Arc<dyn ObjectStore>,
        DurabilityTrackingStore,
    ) {
        let tracker = Arc::new(StageTracker::default());
        tracker.begin_stage(1);
        let inner: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let wrapper = DurabilityTrackingStore::new(
            Arc::clone(&inner),
            Arc::clone(&tracker),
            "file-object-store",
        );
        (tracker, inner, wrapper)
    }

    #[tokio::test]
    async fn a_put_is_tracked_with_its_containing_directory() {
        let (tracker, _inner, wrapper) = wrapped();
        wrapper
            .put_opts(
                &ObjectPath::from("table/data/0.arrow"),
                PutPayload::from("payload".as_bytes().to_vec()),
                PutOptions::default(),
            )
            .await
            .unwrap();

        let set = tracker.finish_writes().unwrap();
        assert!(set.files.contains("table/data/0.arrow"));
        assert!(set.directories.contains("table/data"));
        assert!(!tracker.is_poisoned());
    }

    #[tokio::test]
    async fn a_stage_is_clean_after_a_successful_write() {
        // The failure path is covered by the abort and double-settle tests
        // below, which exercise real failures rather than a simulated store.
        // What this pins is the precondition those depend on: a successful
        // write leaves no unresolved destination behind.
        let (tracker, _inner, wrapper) = wrapped();
        wrapper
            .put_opts(
                &ObjectPath::from("t/a"),
                PutPayload::from("x".as_bytes().to_vec()),
                PutOptions::default(),
            )
            .await
            .unwrap();
        assert!(!tracker.is_poisoned());
        assert!(tracker.finish_writes().is_ok());
    }

    #[tokio::test]
    async fn a_rename_records_both_parents_and_the_new_path() {
        let (tracker, _inner, wrapper) = wrapped();
        wrapper
            .put_opts(
                &ObjectPath::from("old/a"),
                PutPayload::from("x".as_bytes().to_vec()),
                PutOptions::default(),
            )
            .await
            .unwrap();
        wrapper
            .rename(&ObjectPath::from("old/a"), &ObjectPath::from("new/b"))
            .await
            .unwrap();

        let set = tracker.finish_writes().unwrap();
        assert!(set.files.contains("new/b"));
        assert!(set.directories.contains("old"));
        assert!(set.directories.contains("new"));
    }

    #[tokio::test]
    async fn a_delete_records_the_deletion_and_its_parent() {
        let (tracker, _inner, wrapper) = wrapped();
        wrapper
            .put_opts(
                &ObjectPath::from("t/a"),
                PutPayload::from("x".as_bytes().to_vec()),
                PutOptions::default(),
            )
            .await
            .unwrap();
        wrapper.delete(&ObjectPath::from("t/a")).await.unwrap();

        let set = tracker.finish_writes().unwrap();
        assert!(set.deletions.contains("t/a"));
        assert!(set.directories.contains("t"));
    }

    #[tokio::test]
    async fn a_delete_stream_tracks_every_path_it_yields() {
        let (tracker, _inner, wrapper) = wrapped();
        for name in ["t/a", "t/b"] {
            wrapper
                .put_opts(
                    &ObjectPath::from(name),
                    PutPayload::from("x".as_bytes().to_vec()),
                    PutOptions::default(),
                )
                .await
                .unwrap();
        }

        let paths: Vec<ObjectPath> = vec![ObjectPath::from("t/a"), ObjectPath::from("t/b")];
        let stream =
            wrapper.delete_stream(Box::pin(futures::stream::iter(paths.into_iter().map(Ok))));
        use futures::StreamExt;
        let results: Vec<_> = stream.collect().await;
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.is_ok()));

        let set = tracker.finish_writes().unwrap();
        assert!(set.deletions.contains("t/a"));
        assert!(set.deletions.contains("t/b"));
    }

    #[tokio::test]
    async fn an_aborted_multipart_poisons_the_stage() {
        let (tracker, _inner, wrapper) = wrapped();
        let mut upload = wrapper
            .put_multipart(&ObjectPath::from("t/upload"))
            .await
            .unwrap();
        upload.abort().await.unwrap();

        assert!(tracker.is_poisoned());
        let err = tracker.finish_writes().unwrap_err();
        assert!(err.to_string().contains("multipart abort"), "{err}");
    }

    #[tokio::test]
    async fn a_completed_multipart_records_exactly_one_write() {
        let (tracker, _inner, wrapper) = wrapped();
        write_multipart_tracked(
            &wrapper,
            &ObjectPath::from("t/upload"),
            vec![
                PutPayload::from("one".as_bytes().to_vec()),
                PutPayload::from("two".as_bytes().to_vec()),
            ],
        )
        .await
        .unwrap();

        let set = tracker.finish_writes().unwrap();
        assert!(set.files.contains("t/upload"));
        assert!(!tracker.is_poisoned());
    }

    #[tokio::test]
    async fn settling_a_multipart_twice_poisons_the_stage() {
        let (tracker, _inner, wrapper) = wrapped();
        let mut upload = wrapper
            .put_multipart(&ObjectPath::from("t/upload"))
            .await
            .unwrap();
        upload.complete().await.unwrap();
        upload.abort().await.unwrap();
        assert!(
            tracker.is_poisoned(),
            "complete-then-abort on one handle is a state the stage cannot trust"
        );
    }

    #[tokio::test]
    async fn reads_do_not_touch_the_stage() {
        let (tracker, _inner, wrapper) = wrapped();
        wrapper
            .put_opts(
                &ObjectPath::from("t/a"),
                PutPayload::from("payload".as_bytes().to_vec()),
                PutOptions::default(),
            )
            .await
            .unwrap();

        let read = wrapper
            .get_opts(&ObjectPath::from("t/a"), GetOptions::default())
            .await;
        assert!(read.is_ok(), "a read must succeed for a written object");

        let set = tracker.finish_writes().unwrap();
        assert_eq!(
            set.files.iter().cloned().collect::<Vec<_>>(),
            ["t/a"],
            "a read must not add anything to the closure"
        );
    }

    #[test]
    fn wrapping_yields_a_new_wrapper_sharing_one_tracker() {
        let tracker = Arc::new(StageTracker::default());
        let wrapper = DurabilityTrackingStore::new(
            Arc::new(InMemory::new()),
            Arc::clone(&tracker),
            "file-object-store",
        );
        use lance_io::object_store::WrappingObjectStore;
        let inner: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let wrapped = wrapper.wrap("file-object-store", inner);
        // Sharing happens by `Arc::clone`: this handle, the factory wrapper,
        // and the store it just produced are three owners of one tracker —
        // that is the property two tables depend on.
        assert_eq!(Arc::strong_count(&tracker), 3);
        assert!(!format!("{wrapped}").is_empty());
    }
}
