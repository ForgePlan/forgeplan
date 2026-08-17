# FPV-10 — [DOCS] Rebuild product positioning, documentation IA and docs-as-code gates

- **Repository:** `ForgePlan/forgeplan`
- **Phase:** `1`
- **Dependencies:** `FPV-01`
- **Summary:** Align site, README and docs around contract/verification positioning and eliminate drift.

---

## Objective

Make ForgePlan understandable in under one minute and keep all public documentation consistent with shipped capabilities.

## Canonical messaging

- Category: repository-native engineering contract and verification layer for AI coding agents.
- Hero: From engineering intent to verified change.
- Promise: Give every agent a contract. Require proof.
- Cycle: DEFINE → CONTRACT → EXECUTE ANYWHERE → VERIFY → ACCEPT.

## Scope

- README and website information architecture.
- What ForgePlan is / is not.
- Solo, Multi-agent and Autonomous use cases.
- Product family: Protocol, Core, Web, Extensions.
- Integration documentation template.
- Current/planned feature separation.
- generated CLI/MCP/schema/plugin references.
- cross-repo link/version/count/example tests.

## Acceptance Criteria

- [ ] README, website, Core docs, Marketplace and Web use one canonical product definition.
- [ ] Static MCP/CLI/plugin counts are removed or generated.
- [ ] Methodology acronyms are not required to understand the homepage.
- [ ] One end-to-end contract/execution/evidence example appears on the site.
- [ ] Documentation IA follows Start → Concepts → Protocol → Hosts → Orchestrators → Components → Methodologies → Reference → Operations.
- [ ] All code examples are smoke/schema tested in CI.
- [ ] EN/RU structural parity check exists.
- [ ] Old version references and deprecated install paths are detected.

## Important

Do not advertise unshipped Protocol v1 features as current until their release flags and compatibility pages say so.
