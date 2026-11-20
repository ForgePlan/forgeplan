## Verdict

**Content: CONCERNS. Formal Define gate: BLOCKED.**

The material is sufficient to shape the complete product PRD **after three narrow preservation/boundary corrections below**. It preserves the mature Rust-core target rather than substituting an adapter or MVP. Missing experiment thresholds are honestly deferred to Shape with preregistration requirements; they are **not findings**.

This is one independent read-only review slice, not canonical Profile B EVID, a second review round, activation, or authorization to start PRD authoring.

**Reviewer:** `DiscoveryScopeReview`  
**Actual model:** `openai-codex/gpt-6-astra`; dispatched route `openai-codex/gpt-6-astra:high`.  
**Signature:** independent content assessment against the fingerprints below.

## Scope and provenance

Read complete D/R/A, NOTE-067, PROB-107, EPIC-009, PROB-082, ADR-025 and ADR-026; AGENTS.md239–258; and three existing vNext documents discovered through EPIC-009’s source citation.

NOTE/PROBLEM file and body hashes and all three research hashes **match the frozen handoff**. Findings describe document contents. Consequences are review inferences, not observed implementation failures.

Path abbreviations:

- **W:** `/Users/explosovebit/Work/ForgePlan-Analytical-Planning`
- **P:** `/Users/explosovebit/Work/ForgePlan`
- **N:** `W/.forgeplan/notes/note-discovery-revision-bound-engineering-k3f805772fe7840b79cb018dbdca6389f.md`
- **B:** `W/.forgeplan/problems/problem-multi-repository-change-owners-lack-a-kb79d376fabdd5d0b87be1857f2336389.md`
- **D/R/A:** the three named research files under `P/dev/`.

## Substantive findings

### S1 — Active ownership disposition is compressed too far

**Observed:** `N:73,109,134` preserves the external-orchestrator boundary but proposes extending the seven command families, including `estimate` and `calibrate`, without carrying their existing **MOVE TO EXTENSION** disposition. `W/.forgeplan/adrs/ADR-025-orchestration-sits-above-forgeplan-the-core-is-the-contract-layer.md:63,76–78` explicitly requires extension ownership and a migration RFC/deprecation window.

**Condition → consequence:** If the PRD takes F-04/F-29 as new Core responsibilities, it can reinvest in functionality already assigned elsewhere. The broad graph-versus-process summary does not settle this case: ADR-025 explicitly moves estimation despite its reading the graph.

**Minimal correction:** Add the existing disposition to the reconciliation and F-04/F-29 rows: retain the analytical outcome; Core supplies facts, extension owns estimation/calibration behavior under ADR-025. Preserve the required migration/deprecation contract or explicitly propose an independently reviewed amendment. Do not mutate ADR-025 or reduce the feature.

**Severity:** substantive boundary concern; not evidence of a current runtime violation.

### S2 — F-18 loses the on-demand language-extension proposal

**Observed:** `R:220,1277` and `D:850–852` include parsers/grammars installed on demand without rebuilding the core. `N:123` preserves navigation queries and mentions grammar licensing/sandbox verification, but gives **no disposition for on-demand language extension**. No other NOTE/PROBLEM passage restores it.

**Condition → consequence:** A PRD derived from the “complete preservation ledger” can silently omit this part of F-18 while reporting the whole row covered.

**Minimal correction:** Explicitly retain extensible/on-demand language support as a candidate under P2/P7, with WASM one mechanism rather than a selected design; evaluate compatibility, distribution, licensing, permissions and semantic capability declarations. Alternatively record a reasoned rejection. Do not equate loading a grammar with obtaining deep semantic support.

**Severity:** preservation concern, not a demand to select WASM now.

### S3 — Standalone human-readable report delivery disappears

**Observed:** `D:1734,1851` retains an autonomous report usable without a mandatory server. Historical concrete examples appear at `D:879,970` and `R:812,920`. `N:90,98` retains generated documentation, shared JSON/query results and Web/Tauri, while `N:94` provides offline evidence replay. None explicitly retains **standalone human-readable report delivery**.

**Condition → consequence:** Evidence-bundle replay and a connected UI do not necessarily satisfy sharing an understandable analysis with someone who has neither the running service nor the development environment.

**Minimal correction:** Add a P3/P7 outcome for an authorized, self-contained report carrying revision, provenance and omissions, generated from the same query model. Keep the container format open; test offline viewing and restricted-data export. No need to mandate one-file HTML or a particular renderer.

**Severity:** preservation concern.

## F-01–F-34 coverage

“Preserved” means the outcome/candidate and a proposed verification are present, **not implemented or accepted**.

| Idea | NOTE line | Assessment |
|---|---:|---|
| F-01 SQLite replacement | 106 | Preserved as separately justified candidate, not prerequisite |
| F-02 Rust codegraph | 107,88 | Preserved beyond syntax-only extraction |
| F-03 artifact↔symbol bridge | 108,86,90 | Preserved, including rename/delete/revision evidence |
| F-04 seven command families | 109 | Preserved outcome; ownership correction S1 |
| F-05 ports/adapters | 110 | Preserved without pretending production cutover exists |
| F-06 hybrid search/RRF | 111 | Preserved; present ranking correctly distinguished |
| F-07 explicit/doc-tag/diff links | 112 | Preserved; one authority and inferred-versus-approved separation |
| F-08 2D/3D | 113,98 | Preserved as working target, not discarded |
| F-09 SSE/version notification | 114 | Preserved candidate with reconnect/staleness conditions |
| F-10 shared Web/Tauri contract | 115 | Preserved; portability not assumed |
| F-11 desktop integrations | 116 | Preserved |
| F-12 qualified identity | 117 | Existing identity reused; parallel `alias::id` rejected |
| F-13 federation/ATTACH | 118 | Federation preserved; ATTACH remains mechanism candidate |
| F-14 registry/lock/bundles | 119 | Preserved portable delivery/configuration requirement |
| F-15 workspace registry | 120 | Properly reframed around existing resolver and reproducible defects |
| F-16 optional Hub | 121,98 | Preserved; neither mandatory Git host nor local prerequisite |
| F-17 MCP capability packs | 122 | Preserved candidate without invented tool-count limit |
| F-18 Reference A practices/extensions | 123 | Query practices preserved; extension disposition missing, S2 |
| F-19 four CI stages | 124,94 | Preserved with protected policy and actual execution evidence |
| F-20 command journal/undo | 125 | Preserved bounded outcome; no universal bus mandate |
| F-21 release provenance | 126 | Preserved; digest distinguished from signature |
| F-22 qualified UI chips | 127 | Preserved with existence/permission/version checks |
| F-23 legacy migration | 128,96 | Preserved dossier, alternatives, cutover and rollback |
| F-24 extraction plan | 129 | Preserved; branch-count proof correctly rejected |
| F-25 framework/reexport resolution | 130,88 | Preserved with heuristic/unknown distinctions |
| F-26 async review triage | 131,98 | Preserved; independent progress differs from merge authorization |
| F-27 Bun practices | 132,96 | Preserved with external orchestration and parity-before-refactor |
| F-28 role modes/pinning/dry-run | 133,98 | Preserved |
| F-29 effort presentation | 134 | Preserved candidate; ownership correction S1 |
| F-30 behavioral parity | 135,96 | Preserved actual harness, state/errors/effects and approved differences |
| F-31 schema/rebuild | 136,86 | Preserved; destructive reset guarantee rejected |
| F-32 broken-code partial analysis | 137,86 | Preserved stale provenance and dependent re-resolution |
| F-33 deterministic context/cache | 138 | Preserved measurable hypothesis, not guaranteed savings |
| F-34 benchmarks | 139,60 | Preserved paired task-quality/full-cost evaluation |

The ledger’s existence is not itself proof of complete preservation; S2 demonstrates a missing subproposal despite all 34 row IDs being present.

## Appendix coverage

| Appendix section | NOTE coverage | Assessment |
|---|---|---|
| 1 Mature goal before phases | 29,35,62,68,102 | Preserved |
| 2 Own analytical core and limits | 62,68,86–96 | Preserved |
| 3 Reference A usefulness/interactions | 39–45,82,90,98,123 | Preserved except standalone report, S3 |
| 4 Competitor evidence/acceptance | 41–45,82,92,94 | Preserved and correctly attributed |
| 5 Canonical/index/exchange storage | 74,86,106,108,136–137 | Preserved; detailed architecture remains pending |
| 6 Five analysis levels | 88 | Explicitly preserved |
| 7 Six test-quality distinctions | 92 | Explicitly preserved |
| 8 Task-specific reading order | 55,90,138 | Preserved, including cycles, omissions and continuation |
| 9 Legacy behavior and migration | 53,96,128–135 | Preserved |
| 10 Factual documentation | 90,141 | Observed/intended and manual ownership preserved; S3 |
| 11 Federated contracts/delivery/access | 72,94,117–121 | Preserved |
| 12 CI/evidence/trust | 92,94,124–126 | Preserved |
| 13 Working UI/triage/Tauri | 54–55,98,113–116,127,131,133 | Preserved, including accessibility and backlog visibility |
| 14 Take/reuse/reject | 62,68,86–139 | Preserved, subject to S2 |
| 15 Dependency-based stages | 84–102,141,158 | Seven areas preserved; not approved releases |
| 16 Three usefulness evaluations | 60; `B:78–85` | Preserved without invented results or thresholds |
| 17 Sources/check limits | 27–45,177–183 | Preserved; earlier ADI is not final-graph verification |

## Other assessment results

- **Problem/JTBD:** `B:31–57` marks the problem unvalidated, identifies proposed segments and separates owner intent from market prevalence. `N:58` accurately quotes D1201 and D1526. No fabricated interviews or measured losses found.
- **Mature scope:** Own Rust core, semantic producers, behavior, actual tests, legacy, documentation, reading order, federation/CI, 2D/3D, Tauri and optional Hub remain explicit. Complexity does not become an MVP exclusion.
- **Competitive claims:** Reference A code, action and published map versions are separated. Vendor measurements and prior browser interaction are attributed rather than claimed as reviewer/author execution. Superiority/adoption remain hypotheses.
- **Umbrella:** EPIC-009 remains unaccepted; PROB-082 remains a reconciliation gate. ADR-025/026 are not represented as acceptance of the whole historical program. No takeover of EPIC-010 is proposed.
- **Storage/trust:** Canonical links/history are protected from cache rebuild. CI ownership, integrity versus authentication, failed/unknown/not-run outcomes and external execution boundaries are retained.
- **Identity:** `N:71–79,117–119` and `B:71` preserve source-qualified identity and snapshots, recognize portable addressing as unresolved, and distinguish Core/Web/Marketplace references. No replacement registry is silently selected.
- **Shape unknowns:** Corpus, participants, supported configurations, thresholds, policy authority and delivery contracts have explicit follow-through. Their honest incompleteness does not require invented interviews, benchmarks or architecture decisions at Discovery.

## Formal gate limitations

Observed frontmatter still has tactical NOTE/PROBLEM depth and `N:8` has no persisted links. The handoff reports Critical calibration, canonical validation/export and four NOTE relations incomplete, plus HYP-008 `index_pending`. I did not rerun or repair these operations.

The supplied ADI receipt is author-side reasoning over an earlier published context, not independent evidence for the frozen final graph. Canonical completion, the required independent rounds and Profile B recording remain necessary.

**No files or artifacts changed. No cleanup, builds, tests, formatter, lifecycle mutation, activation or commit performed.**

## Checked SHA-256 fingerprints

### Reviewed new artifacts

| Input | File SHA-256 | Body SHA-256 |
|---|---|---|
| N | `3f89f10723c60c839bb0b5cdedcd06ea511ff1bdcfcde69880a3e801b432bd47` | `5a31e47513f1983a6c41bddee3c8ee1a67c502db8b280e6e9e61a6f35d1c6811` |
| B | `47a4b4cb13a0191b7862ec32bfa349e5e883b18170609f4da93816076d56810c` | `a9c5b11cb3782a53890d9d5eb3dddcc5c13c328a8f7c20fda2a2aeaa54356c37` |

Body hashes cover bytes after the closing outer frontmatter delimiter, as defined in the handoff.

### Research and boundary inputs

| Input | File SHA-256 |
|---|---|
| D | `de99aae985daac7d94aeefbf7109dc6e9e0593c1007e5fa8320409828401735c` |
| R | `a93c792e056620e80541746287fbb4ec2149862468df732823cd363b6ee64dba` |
| A | `fd2d1fd6a8eb84fcbaecdc23e543d07f5da9bf745bb0b7467ba8d1d93a78f1b4` |
| W EPIC-009 | `ddd38191424806b0063bc8e8e472e426bee0bdac75bf7e32523c6431104769a8` |
| W PROB-082 | `62ec19b718db8c246169a57bac0dcc188def34abad2d16dcd5b32010c81d8864` |
| W ADR-025 | `641627242bef8f45bb5e74402e003dc6edab5879b65cf1bc5d49544d5ca8768b` |
| W ADR-026 | `2a1be3cae6a1a024d4c57594349ea5fca8b2faa91850d42159f37b667564a24b` |
| P/AGENTS.md, reviewed 239–258 | `a33a05dcdec3f8ca4e83f8073df421dbeaf16e5e1de7107eac80e31ae00eb225` |
| vNext/README.md | `662f09f07fed189e1e1060d767e92a894167890912f1e4e2b95e20f2b679b857` |
| vNext/architecture/01-PRODUCT-BOUNDARY.md | `36380c00d962626def4c1e103a1e2919030ac686bc0fb028ba77ca093708ff56` |
| vNext/architecture/02-TARGET-ARCHITECTURE.md | `190a24744db4ce04c94feed689b5d4c82c788963afe92e0498d1922b8934e0b9` |

Here `vNext` means `P/docs/vnext/engineering-contract-layer`; the existing W artifact filenames are those cited in the findings and review scope.