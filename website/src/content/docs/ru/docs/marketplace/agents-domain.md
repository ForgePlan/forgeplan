---
title: agents-domain
description: "Агенты-специалисты по языкам и фреймворкам: TypeScript, Go, React, Next.js, Electron, встраиваемые системы, fullstack, разработка игр, мобильная разработка и WebSocket-инженерия."
---

Агенты-специалисты по языкам и фреймворкам: TypeScript, Go, React, Next.js, Electron, встраиваемые системы, fullstack, разработка игр, мобильная разработка и WebSocket-инженерия. Намеренно не осведомлены о Forgeplan - они консультируют по коду, а не по графу артефактов. Используйте, когда вопрос идиоматичен в рамках одного стека, где универсальный агент отвечает правдоподобно, но не идиоматично. Полная история в CHANGELOG.md.

**Стоимость**: средняя | **Стабильность**: стабильный | **Версия**: 1.2.1 | **Категория**: developer-tools

**Целевые платформы**: claude-code

**Требования**: Ничего, кроме Claude Code

## Агенты

| Агент | Что делает |
|---|---|
| `electron-pro` | Специалист по десктопным приложениям Electron для кроссплатформенных приложений под Windows, macOS и Linux. |
| `embedded-systems` | Инженер по прошивкам встраиваемых систем для микроконтроллеров (ARM Cortex-M, ESP32, STM32, nRF, AVR) и RTOS-окружений (FreeRTOS, Zephyr). |
| `frontend-developer` | Старший фронтенд-разработчик, специализирующийся на React 18+, Vue 3+ и Angular 17+ с TypeScript strict mode. |
| `fullstack-developer` | Разработчик end-to-end функций, охватывающий схему базы данных, REST/GraphQL API и React/Vue фронтенд в едином связном проходе. |
| `game-developer` | Старший разработчик игр, работающий с Unity (C#), Unreal (C++), Godot и кастомными движками. |
| `golang-pro` | Старший Go-разработчик (Go 1.21+), специализирующийся на идиоматичных конкурентных системах, облачных микросервисах (gRPC, REST), CLI-инструментах и Kubernetes-операторах. |
| `mobile-app-developer` | Старший мобильный разработчик для нативных iOS (SwiftUI/UIKit), Android (Jetpack Compose/Kotlin) и кроссплатформенных приложений (React Native, Flutter). |
| `nextjs-developer` | Старший Next.js 14+ разработчик, специализирующийся на App Router, серверных компонентах, серверных действиях и стратегиях рендеринга ISR/PPR. |
| `typescript-pro` | Старший TypeScript 5.0+ разработчик, специализирующийся на продвинутых возможностях системы типов (условные типы, mapped types, branded types, вариантность) и полной типобезопасности full-stack (tRPC, codegen, Zod). |
| `typescript-type-auditor` | Аудитор системы типов TypeScript, сфокусированный на корректности на уровне типов, валидации ограничений дженериков и безопасности на этапе компиляции. |
| `websocket-engineer` | Специалист по коммуникациям в реальном времени для архитектур WebSocket, Socket.IO и SSE. |

## Установка

```bash
/plugin marketplace add ForgePlan/marketplace
/plugin install agents-domain@ForgePlan-marketplace
```

## См. также

- [Обзор Marketplace](/ru/docs/marketplace/overview/)
- [Справочник команд](/ru/docs/marketplace/commands/)