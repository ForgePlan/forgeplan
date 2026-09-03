# FPV-11 — [EXTENSIONS V2] Introduce extension taxonomy, manifests and generated catalog

- **Repository:** `ForgePlan/marketplace`
- **Phase:** `3`
- **Dependencies:** `FPV-02, FPV-09`
- **Summary:** Turn the Claude-first Marketplace into explicit cross-host ForgePlan Extensions.

---

## Objective

Restructure Marketplace as ForgePlan Extensions with honest maturity and capability declarations.

## Categories

- Host Integrations
- Orchestrator Adapters
- Methodology Packs
- Domain Packs
- Evidence Providers
- Migration Packs
- Visualization Extensions

## Scope

- extension manifest implementation;
- compatibility and permissions disclosure;
- owned/non-owned states;
- maturity labels;
- conformance references;
- generated root catalog/docs;
- migration of current plugins;
- explicit Claude-native vs portable surfaces;
- Smith positioned as optional router.

## Acceptance Criteria

- [ ] Every published extension has a validated manifest.
- [ ] Catalog and counts are generated from manifests.
- [ ] Each extension declares permissions and state ownership.
- [ ] Full/partial support is based on conformance results.
- [ ] Claude-specific agents/commands/hooks are clearly marked.
- [ ] `.agents/skills` remains portable baseline.
- [ ] Deprecated plugins have machine-readable replacement and sunset metadata.
- [ ] Core operation does not require Marketplace or Smith.
