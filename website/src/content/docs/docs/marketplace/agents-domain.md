---
title: agents-domain
description: "Domain specialist agents for languages and frameworks: TypeScript, Go, React, Next.js, Electron, embedded systems, fullstack, game development, mobile and WebSocket engineering."
---

**Catalog description:** Domain specialist agents for languages and frameworks: TypeScript, Go, React, Next.js, Electron, embedded systems, fullstack, game development, mobile and WebSocket engineering. Deliberately not forgeplan-aware - they advise on code, not on the artifact graph. Use when the question is idiomatic within one stack, where a generalist answers plausibly but not idiomatically. Full history in CHANGELOG.md.

**Cost**: medium | **Stability**: stable | **Version**: 1.2.1 | **Category**: developer-tools

**Targets**: claude-code

**Requires**: No additional requirements declared in the catalog

**Packaged surface**: 0 command files, 11 agent files, 0 skills

## Agents

Claude Code scoped names are shown below. Its [agent loader](https://code.claude.com/docs/en/plugins/components#agents) scans `agents/` recursively, keeps subfolder segments and applies frontmatter `name` to the final segment. Other runtimes may use different names. Descriptions are packaged metadata except for curated corrections to documented runtime behavior; runtime boundaries, where provided, take precedence over optimistic metadata wording.

| Agent | Source path | What it does |
|---|---|---|
| `agents-domain:electron-pro` | `agents/electron-pro.md` | Electron desktop application specialist for cross-platform apps targeting Windows, macOS, and Linux. Use when building or hardening Electron apps - security model (contextIsolation, IPC, preload), native OS integration, auto-update, and distribution pipelines. Hand off to `code-reviewer` for security audit of IPC and preload scripts. |
| `agents-domain:embedded-systems` | `agents/embedded-systems.md` | Embedded systems firmware engineer for microcontrollers (ARM Cortex-M, ESP32, STM32, nRF, AVR) and RTOS environments (FreeRTOS, Zephyr). Use when developing firmware, HAL layers, communication protocols, power management, or debugging with JTAG/SWD. Hand off to `code-reviewer` for interrupt safety and memory allocation review. |
| `agents-domain:frontend-developer` | `agents/frontend-developer.md` | Senior frontend developer specializing in React 18+, Vue 3+, and Angular 17+ with TypeScript strict mode. Use when building UI components, implementing accessibility (WCAG 2.1 AA), optimizing Web Vitals, or setting up frontend testing strategy. Hand off to `typescript-pro` for deep type system work or `laws-of-ux` agent for UX review. |
| `agents-domain:fullstack-developer` | `agents/fullstack-developer.md` | End-to-end feature developer spanning database schema, REST/GraphQL API, and React/Vue frontend in a single cohesive pass. Use when implementing full features with shared types, auth flows, testing strategy across all layers, and deployment pipelines. Hand off to `typescript-pro` for type safety deep-dives or `security-expert` for auth/authz review. |
| `agents-domain:game-developer` | `agents/game-developer.md` | Senior game developer covering Unity (C#), Unreal (C++), Godot, and custom engines. Use when implementing game architecture (ECS, scene management), graphics pipelines, physics, AI (A*, behavior trees), multiplayer networking, or optimizing for mobile/console/VR targets. Hand off to `performance-engineer` for frame-rate profiling or `code-reviewer` for engine-level code review. |
| `agents-domain:golang-pro` | `agents/golang-pro.md` | Senior Go developer (Go 1.21+) specializing in idiomatic concurrent systems, cloud-native microservices (gRPC, REST), CLI tools, and Kubernetes operators. Use when building or refactoring Go code - goroutine lifecycle, error handling, pprof profiling, testing with race detector. Hand off to `security-expert` for TLS/auth review or `code-reviewer` for idiomatic Go review. |
| `agents-domain:mobile-app-developer` | `agents/mobile-app-developer.md` | Senior mobile developer for native iOS (SwiftUI/UIKit), Android (Jetpack Compose/Kotlin), and cross-platform (React Native, Flutter). Use when building or optimizing mobile apps - platform HIG compliance, offline-first architecture, push notifications, device APIs, CI/CD with Fastlane. Hand off to `security-expert` for Keychain/EncryptedSharedPreferences and certificate pinning review. |
| `agents-domain:nextjs-developer` | `agents/nextjs-developer.md` | Senior Next.js 14+ developer specializing in App Router, server components, server actions, and ISR/PPR rendering strategies. Use when building full-stack Next.js apps - data fetching patterns, SEO with Metadata API, performance (Core Web Vitals), and Vercel/Docker deployment. Hand off to `typescript-pro` for complex type work or `security-expert` for auth middleware and API route security. |
| `agents-domain:typescript-pro` | `agents/typescript-pro.md` | Senior TypeScript 5.0+ developer specializing in advanced type system features (conditional types, mapped types, branded types, variance) and full-stack type safety (tRPC, codegen, Zod). Use when building or refactoring TypeScript codebases, configuring strict tsconfig, or optimizing build performance in monorepos. Hand off to `typescript-type-auditor` for a dedicated type-coverage audit pass. |
| `agents-domain:typescript-type-auditor` | `agents/typescript-type-auditor.md` | TypeScript type system auditor focused on type-level correctness, generic constraint validation, and compile-time safety. Use when auditing an existing codebase for implicit `any`, unsafe assertions (`as unknown as T`), incomplete discriminated unions, or variance issues. Complements `typescript-pro` - dispatch after implementation to verify type hygiene. Hand off findings to `code-reviewer` for prioritization. |
| `agents-domain:websocket-engineer` | `agents/websocket-engineer.md` | Real-time communication specialist for WebSocket, Socket.IO, and SSE architectures. Use when designing or implementing bidirectional messaging - connection lifecycle, authentication during handshake, reconnection with exponential backoff, horizontal scaling via Redis Pub/Sub, and load testing. Hand off to `security-expert` for JWT handshake and message validation review. |

## Installation

```bash
/plugin marketplace add ForgePlan/marketplace
/plugin install agents-domain@ForgePlan-marketplace
```

## See also

- [Marketplace overview](/docs/marketplace/overview/)
- [Commands Reference](/docs/marketplace/commands/)
