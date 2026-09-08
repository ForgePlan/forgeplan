---
depth: tactical
id: EVID-169
kind: evidence
links:
- target: ADR-025
  relation: informs
- target: ADR-026
  relation: informs
status: active
title: 'ADR-025 and ADR-026 basis: vNext audit measurements plus owner approval'
---

---
assigned_number: 169
created: 2026-09-05
predicted_number: 169
slug: evid-adr-025-and-adr-026-basis-vnext-audit-measurements-plus-owner-approval
updated: 2026-09-05
---

# EVID-169: the basis for ADR-025 and ADR-026

## What this pack certifies

Both ADRs resolve blockers the vNext adversarial audit raised
(`docs/vnext/engineering-contract-layer/_audit/`), and both went through the
owner. This pack records the measured basis and the approval, so activation
does not rest on the author's own say-so.

## Measurements behind ADR-025

- ADR-001 and ADR-009 both `status: active` while asserting opposite owners
  for orchestration — read directly from frontmatter, audit finding B2.
- The playbook runtime spawns agents from the core binary
  (`agent_dispatcher.rs:69`, `claude --print`) against the declared boundary.
- The memory kind: 2 artifacts exist in this workspace, `forgeplan new
  memory` fails with "No template found for kind 'memory'", #411 shows
  mem-* ids cannot resolve for linking. Verified by execution on 0.36.0.
- `.forgeplan/state/` is gitignored — phase state never leaves the machine.

## Measurements behind ADR-026

- The audit blocked FPV-03/04/05 on storage (B3): zero mentions of a storage
  home for the four new object classes across `docs/vnext/architecture/`.
- ADR-018 already rejected a second authoritative non-markdown store; the
  journal module already exists in `forgeplan-core` for the audit stream.
- Precedent: CI results are referenced, never copied — the same rule
  generalises to receipts.

## Owner decision

2026-09-05. Decision #1 (orchestration above ForgePlan) made by the owner
directly. Proposals #2 (storage classes) and #3 (surface dispositions)
reviewed with plain-language explanations and approved: "да все понятно и
все ок - зафиксировать". This pack is that fixation.

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: audit




