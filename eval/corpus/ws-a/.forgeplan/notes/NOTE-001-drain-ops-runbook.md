---
author: eval-corpus
depth: tactical
id: NOTE-001
kind: note
status: active
title: Drain Ops Runbook
---

# Drain Ops Runbook

## Reading the Partition Table

`owner_id` + `fence` + `lease_until` tell you who holds what and until
when. A partition with `last_heartbeat` older than 30s is a steal
candidate — check for a dead worker before forcing anything.

## Constraints

- Never delete a partition row manually: deletion races an in-flight
  steal CAS and has caused split-brain twice (2025-11, 2026-01).
- To drain a tenant faster, raise pool concurrency on the worker flag
  `--max-partitions`, never by editing `lease_until`.
- Replaying dead-letter batches requires the failure class recorded by
  SPEC-002 — replay without it re-poisons the partition.
