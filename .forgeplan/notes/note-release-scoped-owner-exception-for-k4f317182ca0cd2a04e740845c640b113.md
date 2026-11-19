---
assigned_number: 65
author: null
created: 2026-11-14
depth: tactical
id: NOTE-065
kind: note
links:
- target: rfc-plan-cli-mcp-workspaceread-auxiliary-history
  relation: informs
parent_epic: null
predicted_number: 65
slug: note-release-scoped-owner-exception-for-k4f317182ca0cd2a04e740845c640b113
status: draft
tags: []
title: Release-scoped owner exception for deferred G1 G2 G3 conditions
updated: 2026-11-14
valid_until: null
---

---
assigned_number: 65
created: 2026-11-14
predicted_number: 65
slug: note-release-scoped-owner-exception-for-k4f317182ca0cd2a04e740845c640b113
updated: 2026-11-14
---

## Context

The owner reviewed the remaining formal conditions while C19 production verification was in progress. Actual C19 evidence is eight targeted tests and narrow Core/MCP Clippy passing; current optimized production verification is still incomplete after recorded disk failures. Test counts are not instrumental coverage.

## Content

For this release only, the owner explicitly selected `release_scoped_exception` for the enumerated G1/G2/G3 conditions: unmeasured Rust/Node coverage >=80% (including RFC-018's post-code requirement), unavailable canonical `mm-gate-failures` memory binding, and the unresolved native rule distinguishing measurement PASS from adversarial review. These conditions are deferred with an authorized exception, NOT passed.

## Decision and Boundaries

The exception does not amend a global policy, change a memory bank, establish an alias, invent a coverage percentage, erase findings, or authorize activation by force. It is not permission to push, merge, tag, publish, install, or change global configuration.

All other mandatory acceptance remains: actual CLI/MCP behavior, current-source production binaries, honest failed/skipped/unexecuted observations and final independent whole-result review. Supported product defects are not waived.

## Evidence

Direct owner form answer in this conversation: `formal_gates=release_scoped_exception`. Exact local receipt: `.local/contextpack-c7/owner-formal-release-exception.json`. This note records authorization, not a technical PASS. C19 source manifest: `f821d5a7a54cb09a931b89ecfb0f884e1d52dc1e2cac465741c338e74e1fe1de`; source-only corrective review and actual execution have separate evidence.

## Revisit Trigger

Before applying these formal conditions to any subsequent release, the policy owner must define the executable coverage denominator/tool and resolve the canonical memory binding and measurement-versus-review rule. This exception must not be inherited as a permanent default.

## Related

RFC-018; PRD-089 / RFC-017 documentation scope; EVID-196 and the historical G1/G2/G3 dispositions. This note does not rewrite their observations or lifecycle status.
