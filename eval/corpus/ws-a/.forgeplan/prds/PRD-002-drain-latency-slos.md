---
author: eval-corpus
depth: standard
id: PRD-002
kind: prd
links:
- target: PRD-001
  relation: relates_to
status: active
title: Drain Latency SLOs
---

# Drain Latency SLOs

## Problem

Throughput and freshness of the drain queue need contractual numbers so
capacity alerts mean something.

## Constraints

- p99 drain latency (enqueue to sink-ack) must stay under 800ms per
  tenant in every 5-minute window.
- Sustained drain rate must be at least 1200 messages per second per
  partition without SLO breach.
- Backlog age alert fires when the oldest unclaimed message exceeds
  90 seconds — paging, not just dashboarding.

## Measurement

Latency is measured at the sink-ack, not at enqueue-ack. A retry cycle
counts toward the latency of the original enqueue, not the retry's
enqueue time.
