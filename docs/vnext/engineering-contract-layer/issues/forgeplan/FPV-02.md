# FPV-02 — [PROTOCOL] Publish ForgePlan Protocol v1 schemas and compatibility rules

- **Repository:** `ForgePlan/forgeplan`
- **Phase:** `1`
- **Dependencies:** `FPV-01`
- **Summary:** Create versioned neutral protocol types independent of CLI, MCP and individual hosts.

---

## Objective

Create the stable protocol layer shared by ForgePlan Core, CLI/MCP, Web, host adapters and orchestrator adapters.

## Required protocol types

- ArtifactReference
- ActorIdentity
- Claim/Lease reference
- WorkContract
- ExecutionReceipt
- EvidenceBundle
- VerificationVerdict
- AuthorityPolicy
- ExternalReference
- CapabilityManifest
- ErrorEnvelope

## Scope

- JSON Schema 2020-12 definitions.
- Rust domain/DTO mapping without transport-specific fields in core types.
- Canonical JSON serialization and digest algorithm.
- semantic versioning and compatibility policy.
- fixture corpus with valid/invalid examples.
- schema package/release strategy.
- generated protocol reference docs.

## Acceptance Criteria

- [ ] Every type has a versioned schema ID.
- [ ] Unknown major versions fail closed.
- [ ] Unknown optional fields survive supported round-trips where required.
- [ ] Canonical digest is deterministic across repeated serialization.
- [ ] Rust and JSON schema validation agree on fixture corpus.
- [ ] Protocol types contain no Cursor/Codex/Claude/Paperclip-specific fields outside namespaced extensions.
- [ ] CI validates all examples and generated docs.

## Non-goals

- Implementing every adapter.
- Starting agent processes.
- Adding UI.
