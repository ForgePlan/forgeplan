---
depth: tactical
id: PROB-102
kind: problem
links:
- target: PRD-086
  relation: informs
status: draft
title: CI compiles the semantic-search surface and never runs it
---

---
assigned_number: 102
predicted_number: 102
slug: prob-ci-compiles-the-semantic-search-surface-and-never-runs-it
---

# PROB-102: the embedding oracle has never run

## Signal

`crates/forgeplan-core/tests/embedding_reference.rs` describes itself as the
correctness oracle for the embedding engine — *"catches the one failure mode an
engine swap has that nothing else would."* It holds three tests pinning vectors
against values captured from the pre-tract engine.

It executes zero of them in CI:

```
$ cargo test -p forgeplan-core --test embedding_reference
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored
```

The file is gated on `feature = "semantic-search"`. CI's test step is:

```yaml
- name: cargo nextest run
  run: cargo nextest run --workspace --all-targets
```

No `--features`. The two steps above it *do* pass the feature:

```yaml
- run: cargo check --workspace --all-targets --features semantic-search
- run: cargo clippy --workspace --all-targets --features semantic-search
```

So the semantic-search surface is compiled and linted on every PR, and executed
on none. `0 passed` is reported as success.

## Why this matters more than the count

v0.35.0 replaced the entire embedding engine — ONNX Runtime to tract. This
oracle is the safety net that swap was performed over. It was written for
exactly that release, and it did not run during it. The vector-equivalence
claim in the v0.35.0 notes (max deviation 7.0e-07 across six reference cases)
came from a manual local run, not from the gate.

The same hole explains PROB-103 (`embed` loading the model when there is
nothing to embed, 8.22s on a current workspace): no CI job ever executes
`embed`, so its cost was invisible until a user noticed the message.

This is the release's own theme applied to the harness: a check that exists,
is documented, is well-intentioned, and never runs — while reporting a green
result.

## Scope

Only `embedding_reference.rs` is a fully-gated test file. Inline `#[cfg]`
blocks elsewhere hide additional cases, but a reliable count needs a proper
audit rather than a grep — an earlier attempt here produced inflated numbers by
matching from the first `cfg` line to end of file, and was discarded.

## Fix direction

Add a nextest invocation with the feature enabled. It cannot simply replace the
current one: the default-feature run is what proves the shipped-without-feature
build still works, and that path has its own refusal branches worth executing.
Two runs, or one matrix, both configurations.

Cost is real and should be stated rather than discovered: the feature pulls the
model at runtime, so the job needs the model cache or the tests need to be
marked as requiring it. That is the reason it was left out, and it is a
solvable reason, not a justification for reporting `0 passed` as green.

## Related

| Artifact | Relation |
|---|---|
| PRD-086 | informs |



