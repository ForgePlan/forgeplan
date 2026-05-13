---
depth: standard
id: EVID-125
kind: evidence
last_modified_at: 2026-05-12T17:35:40.414137+00:00
last_modified_by: claude-code/2.1.138
links:
- target: PROB-068
  relation: informs
status: active
title: PROB-068 perf stress at 1000-artifact scale
---

## Summary

PROB-068 fix (Option C — auto-backup of artifact directories at `forgeplan init --force`) measured at production-scale workspace size: 1000 artifacts across 9 ARTIFACT_DIRS kinds. Threshold contract: <30s runtime + byte-equal content preservation.

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: measurement

## Measurement

- **Workspace size**: 1000 markdown artifacts, round-robin across `prds/`, `rfcs/`, `adrs/`, `specs/`, `epics/`, `problems/`, `solutions/`, `evidence/`, `notes/`
- **Body shape**: realistic frontmatter (`author:`, `tags:`, `links:`, `custom_index:`) + multi-section markdown body (~650 bytes/artifact)
- **Backup payload size**: 653 027 bytes (~638 KB across 1000 files)
- **Runtime**: **571 ms** (`init --force` with default auto-backup, release build, macOS APFS, M-class SSD)
- **Headroom vs threshold**: 30 000 ms / 571 ms ≈ **52× margin**
- **Test runtime (both tests, --test-threads=1)**: 2.90 s wall

## Tests

New file `crates/forgeplan-cli/tests/cli_init_perf_stress.rs`:

1. `init_force_backup_1000_artifacts_under_30s` — populates 1000 artifacts in-process (direct `fs::write`, no CLI subprocess), runs real `forgeplan init -y --force` via `assert_cmd`, asserts `elapsed.as_secs() < 30`. Emits perf log line to stderr capturing n, runtime, backup dir, backup size.
2. `init_force_backup_preserves_all_bodies` — populates same cohort, snapshots 50 deterministic samples (stride = n/50, covers every kind via round-robin), runs `--force`, locates newest `.forgeplan-backup-*/`, asserts byte-equal match against pre-backup bodies. Zero mismatches.

## Integrity result

50/50 samples byte-equal across 9 artifact kinds. No partial-copy failures, no body truncation, no encoding drift.

## Conclusion

Auto-backup is production-acceptable at 1000-artifact scale:
- Runtime well under the 30s budget (52× headroom).
- Content preservation verified at scale (not just on 1-artifact populate_workspace fixture).
- No follow-up PROB-069 required — no perf bottleneck observed.

If a real user workspace ever crosses ~50 000 artifacts the linear `copy_dir_recursive` walk would become the dominant cost (≈ 28 s extrapolated). At that scale a hard-link strategy on POSIX (`fs::hard_link` instead of `tokio::fs::copy`) would deliver an order-of-magnitude speedup, but that is speculative — track only if a real workspace surfaces.

## References

- PROB-068 (the parent problem)
- Tests in `crates/forgeplan-cli/tests/cli_init_perf_stress.rs`
- Wave 8A of v0.31.0 sprint (2026-05-12)
- Pipeline gate: fmt clean, check clean, clippy `-D warnings` clean, smoke pass, target test 2/2 pass



