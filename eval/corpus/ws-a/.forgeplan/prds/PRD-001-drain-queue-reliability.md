---
author: eval-corpus
depth: deep
id: PRD-001
kind: prd
links:
- target: SPEC-003
  relation: refines
- target: EPIC-001
  relation: relates_to
status: active
title: Drain Queue Reliability
---

# Drain Queue Reliability

## Problem

The drain queue moves messages from ingest topics to per-tenant sinks.
Today a worker crash silently loses the in-flight drain batch: the lease
on the partition expires, another worker picks the partition up, and the
un-acked tail of the dead worker's batch is never re-driven because the
checkpoint was already advanced. Customers see this as "the queue said
done but the sink is short N messages".

## Goals

- Zero acknowledged-but-undelivered messages across worker crashes.
- A partition may be claimed by at most one live owner at a time.
- Every constraint below is normative; violating it is a Sev-2 defect.

## Constraints

- Owner lock TTL must be 30 seconds with heartbeat every 10 seconds; a
  partition is stealable only after three consecutive missed heartbeats.
- Retry policy for a failed drain batch: at most 5 attempts with
  exponential backoff starting at 250ms and capped at 8s.
- A drained item must be acked within the lease window or it requeues;
  the checkpoint never advances past the oldest un-acked offset.
- The binding predicate for every queued job must include epoch,
  checkout incarnation and row revision — see SPEC-001.

## Non-goals

- Cross-region replication of the queue itself.
- Ordering guarantees beyond per-partition FIFO.

## Architecture Notes

Each partition row carries `owner_id`, `fence`, `lease_until`,
`last_heartbeat`. The owner writes heartbeats into the same row; a
stealing worker must CAS `fence` upward before touching the offset
cursor. Workers never buffer acks — an ack is a synchronous cursor
update on the partition row, so a crash can lose at most the un-acked
suffix.

The dispatcher assigns partitions by consistent hashing over live
worker ids. Rebalance churn is bounded by the steal rule, not by a
coordinator — there is no coordinator to fail.

## Appendix: February Partition Event Timeline

On 2026-02-14 the eu-west cell lost three drain workers inside one
minute after a bad rollout. The partition table shows what followed:

- T+0s: workers w-14, w-31, w-77 die mid-batch on partitions p-03, p-09,
  p-12. Heartbeats stop; `lease_until` continues to tick down.
- T+11s to T+30s: surviving workers observe the first missed heartbeat
  and back off — the steal rule requires three misses, so nothing moves.
- T+34s: p-03 is fenced by w-05 after the third missed heartbeat
  (missed at T+11, T+21, T+31). Recovery begins from the last acked
  offset, not from the dead worker's in-memory position.
- T+41s: p-09 fenced by w-22; p-12 fenced by w-05. All three
  partitions resume within the 30s TTL plus one heartbeat interval.
- Post-incident audit found 41,207 messages had been claimed but not
  yet acked at the moment of the crash. Every one of them was re-driven
  by the checkpoint rule — zero silent loss, exactly because the ack
  cursor had not advanced past them.

The audit conclusion: the steal-after-three-heartbeats rule is the
reason the outage produced re-driven duplicates (acceptable, sinks are
idempotent) instead of silent gaps (unacceptable). Weakening the steal
rule to two heartbeats was considered and rejected: a single GC pause
on the old owner would then split-brain the partition.
