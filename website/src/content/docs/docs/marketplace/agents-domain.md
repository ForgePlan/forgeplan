---
title: agents-domain
description: "Domain specialist agents for languages and frameworks: TypeScript, Go, React, Next.js, Electron, embedded systems, fullstack, game development, mobile and WebSocket engineering."
---

Domain specialist agents for languages and frameworks: TypeScript, Go, React, Next.js, Electron, embedded systems, fullstack, game development, mobile and WebSocket engineering. Deliberately not forgeplan-aware - they advise on code, not on the artifact graph. Use when the question is idiomatic within one stack, where a generalist answers plausibly but not idiomatically. Full history in CHANGELOG.md.

**Cost**: medium | **Stability**: stable | **Version**: 1.2.1 | **Category**: developer-tools

**Targets**: claude-code

**Requires**: Nothing beyond Claude Code

## Agents

| Agent | What it does |
|---|---|
| `electron-pro` | Electron desktop application specialist for cross-platform apps targeting Windows, macOS, and Linux. |
| `embedded-systems` | Embedded systems firmware engineer for microcontrollers (ARM Cortex-M, ESP32, STM32, nRF, AVR) and RTOS environments (FreeRTOS, Zephyr). |
| `frontend-developer` | Senior frontend developer specializing in React 18+, Vue 3+, and Angular 17+ with TypeScript strict mode. |
| `fullstack-developer` | End-to-end feature developer spanning database schema, REST/GraphQL API, and React/Vue frontend in a single cohesive pass. |
| `game-developer` | Senior game developer covering Unity (C#), Unreal (C++), Godot, and custom engines. |
| `golang-pro` | Senior Go developer (Go 1.21+) specializing in idiomatic concurrent systems, cloud-native microservices (gRPC, REST), CLI tools, and Kubernetes operators. |
| `mobile-app-developer` | Senior mobile developer for native iOS (SwiftUI/UIKit), Android (Jetpack Compose/Kotlin), and cross-platform (React Native, Flutter). |
| `nextjs-developer` | Senior Next.js 14+ developer specializing in App Router, server components, server actions, and ISR/PPR rendering strategies. |
| `typescript-pro` | Senior TypeScript 5.0+ developer specializing in advanced type system features (conditional types, mapped types, branded types, variance) and full-stack type safety (tRPC, codegen, Zod). |
| `typescript-type-auditor` | TypeScript type system auditor focused on type-level correctness, generic constraint validation, and compile-time safety. |
| `websocket-engineer` | Real-time communication specialist for WebSocket, Socket.IO, and SSE architectures. |

## Installation

```bash
/plugin marketplace add ForgePlan/marketplace
/plugin install agents-domain@ForgePlan-marketplace
```

## See also

- [Marketplace overview](/docs/marketplace/overview/)
- [Commands Reference](/docs/marketplace/commands/)
