---
title: agents-pro
description: "Professional specialist agents: security and prompt-injection review, PII detection, architecture and staff-level system review, the pre-activation guardian gate, ADR and specification authors, research analysts and infrastructure roles."
---

Professional specialist agents: security and prompt-injection review, PII detection, architecture and staff-level system review, the pre-activation guardian gate, ADR and specification authors, research analysts and infrastructure roles. Use when a task needs a named expert rather than a generalist - an OWASP/STRIDE pass, an architectural fitness review, a decision recorded as an ADR, or the last gate before an artifact is activated. Full history in CHANGELOG.md.

**Cost**: heavy | **Stability**: stable | **Version**: 1.30.0 | **Category**: developer-tools

**Targets**: claude-code

**Requires**: Nothing beyond Claude Code

## Agents

| Agent | What it does |
|---|---|
| `adr-architect` | ADR specialist using MADR 3.0 format. |
| `api-docs-engineer` | OpenAPI documentation specialist - generates and maintains OpenAPI 3.0 specs from existing API code, documents endpoints with examples, schemas, error responses, and authentication guides. |
| `architect-reviewer` | Architecture reviewer (Profile B gate-style). |
| `artifact-author` | Generic Profile A creator for any forgeplan artifact kind. |
| `artifact-maintainer` | Generic Profile D fallback for kind-agnostic metadata maintenance on EXISTING forgeplan artifacts. |
| `artifact-reviewer` | Generic Profile B reviewer for forgeplan artifact HEALTH audit - schema completeness, section coherence, link graph health, freshness/decay, R_eff trust. |
| `brief-intake` | First-touch intake specialist. |
| `claims-authorizer` | Claims-based authorization specialist - designs ABAC/RBAC policies, evaluates access claims, enforces fine-grained permissions, and maintains audit trails using a 5-claim model. |
| `code-analyzer` | Code quality analysis specialist performing comprehensive reviews across five domains - quality, performance, security, architecture, and technical debt. |
| `ddd-domain-expert` | Domain-Driven Design specialist for strategic and tactical domain modeling - bounded context identification, aggregate design, ubiquitous language enforcement, and context mapping. |
| `distributed-systems-expert` | Expert in distributed systems theory and practice - consensus protocols (Raft, PBFT, Paxos), CRDTs, gossip protocols, quorum management, and distributed security. |
| `documentation-engineer` | Documentation engineer - designs information architecture, creates API docs, tutorials, and reference guides with automation, search optimization, and multi-version support. |
| `evidence-gatherer` | Active evidence collector for Trust Calculus scoring. |
| `evidence-recorder` | Fallback Profile B agent for phases that produce evidence but don't fit a specialist recorder (code-reviewer / tester / security-expert / architect-reviewer / system-dev). |
| `goal-planner` | Goal-Oriented Action Planning (GOAP) specialist. |
| `guardian` | Pre-activation gate (Profile B gate-style). |
| `injection-analyst` | Prompt injection and jailbreak analyst - detects, classifies, and mitigates injection threats in LLM-powered applications using a 6-type threat taxonomy, evasion detection, and sophistication scoring. |
| `mcp-developer` | MCP developer specializing in Model Context Protocol server and client implementation - JSON-RPC 2.0, TypeScript/Python SDKs, tool/resource/prompt definitions, transport configuration (stdio/SSE), and production deployment. |
| `memory-specialist` | Vector memory and retrieval optimization specialist - HNSW indexing, vector quantization, hybrid search with RRF fusion, memory consolidation, and knowledge protection strategies. |
| `microservices-architect` | Distributed systems architect for microservice design - service boundary identification, communication patterns (sync/async), resilience patterns, observability stacks, Kubernetes deployment, and monolith-to-microservices decomposition. |
| `ml-developer` | ML developer specializing in end-to-end machine learning workflows - data preprocessing, model selection, cross-validation, hyperparameter tuning, evaluation, and deployment preparation using scikit-learn, pandas, and deep learning frameworks. |
| `pii-detector` | PII and credential detector - scans code, config files, logs, and test fixtures for sensitive data leaks using API key regexes, PII patterns, and compliance mapping (GDPR, HIPAA, PCI-DSS, SOC2). |
| `platform-engineer` | Platform engineer specializing in internal developer platforms - self-service infrastructure, GitOps workflows, golden path templates, Backstage developer portal, and developer experience optimization. |
| `prompt-engineer` | Expert prompt engineer specializing in designing, optimizing, and evaluating prompts for LLMs - prompt patterns (zero-shot, few-shot, CoT, ReAct), token optimization, safety mechanisms, and A/B testing frameworks. |
| `research-analyst` | Read-only research analyst. |
| `search-specialist` | Search specialist mastering advanced information retrieval and knowledge discovery - query formulation, source selection, Grep/Glob patterns, compound search strategies, and result curation across codebases, logs, and documentation. |
| `security-expert` | Security audit specialist. |
| `smith` | Master-orchestrator agent (Profile B-orchestrator). |
| `system-dev` | Staff/principal-level final auditor (Profile B gate-style). |
| `ui-designer` | Visual designer specializing in intuitive, accessible user interfaces - design systems, interaction patterns, visual hierarchy, responsive layouts, WCAG 2.1 AA compliance, and developer handoff. |

## Installation

```bash
/plugin marketplace add ForgePlan/marketplace
/plugin install agents-pro@ForgePlan-marketplace
```

## See also

- [Marketplace overview](/docs/marketplace/overview/)
- [Commands Reference](/docs/marketplace/commands/)
