---
created: 2026-04-20
depth: tactical
id: PROB-043
kind: problem
status: active
title: Activity log flush missing — CI flaky append_creates_file_and_directory test
updated: 2026-04-20
---

# PROB-043: Activity log missing explicit flush → CI flaky test

## Problem Statement

`crates/forgeplan-core/src/activity/mod.rs::append` пишет в файл через `tokio::fs::File::write_all` и возвращает `Ok(())` без explicit `file.flush().await`. `tokio::fs::File::drop` **не делает** async flush. На GitHub Actions runners (ubuntu-24.04, Linux overlayfs) test `activity::tests::append_creates_file_and_directory` intermittently видит пустой файл при чтении сразу после `append` returns — content.matches('\n').count() == 0 вместо expected 1.

## Signal

CI failure в PR #202 build `24661185656`:
```
thread 'activity::tests::append_creates_file_and_directory' panicked at
  crates/forgeplan-core/src/activity/mod.rs:239:9:
assertion `left == right` failed
  left: 0
  right: 1
test result: FAILED. 1075 passed; 1 failed
```

Локально (macOS APFS) тест PASS — filesystem flushes быстрее. На CI (Linux overlayfs) buffer сидит дольше, test читает before OS flush.

## Root Cause

`tokio::fs::File` buffers writes. Explicit flush — ответственность caller. Наш PRD-054 (activity log) не flush'и́л для latency reasons (обосновано в комментарии "We do NOT fsync every write"). Но есть разница: **flush to OS buffer** (дешёвый) vs **fsync to disk** (дорогой). Нужен flush, не fsync.

## Proposed Solution

Добавить `file.flush().await?` перед `Ok(())`. Это только flush к OS buffer (не disk) — latency ~1μs, не влияет на performance. Durability (on crash) не меняется — OS всё равно буферизует.

Комментарий обновлён — объясняет разницу flush vs fsync.

## Acceptance Criteria

- [x] `append` вызывает `file.flush().await?` перед return
- [x] Комментарий обновлён: flush to OS buffer vs fsync to disk
- [x] Локально `cargo test -p forgeplan-core --lib activity` PASS (18/18)
- [x] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] CI rerun — `append_creates_file_and_directory` PASS

## Related Artifacts

| Artifact | Type | Relation |
|----------|------|----------|
| PRD-054 | PRD | informs (PRD that introduced activity log module) |


