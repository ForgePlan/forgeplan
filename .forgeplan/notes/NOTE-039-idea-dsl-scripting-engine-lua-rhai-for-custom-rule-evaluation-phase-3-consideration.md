---
depth: tactical
id: NOTE-039
kind: note
links:
- target: RFC-001
  relation: informs
status: draft
title: 'Idea: DSL scripting engine (Lua/Rhai) for custom rule evaluation — Phase 3+ consideration'
---

## DSL Scripting Engine for Rule Evaluation

### Тезис
Заменить YAML rule conditions на embedded scripting language (Lua через mlua или Rhai) для полной flexibility в custom правилах.

### Зачем
Phase 2 rule engine использует structured YAML conditions (expressions, ranges, links_missing). Но пользователь может захотеть:
- Сложную логику: "если PRD active > 30 дней и нет RFC И R_eff < 0.5 — escalate"
- Cross-artifact queries: "все PRD в этом Epic у которых нет evidence"
- Custom scoring формулы: пользователь определяет свой R_eff вариант
- Pipeline hooks: "при activate PRD — автоматически создать RFC stub"

### Варианты
1. **Lua (mlua crate)** — зрелый, быстрый, маленький runtime (~200KB), широко используется в играх/конфигах. Минус: ещё один язык для пользователя.
2. **Rhai (rhai crate)** — Rust-native scripting, синтаксис похож на Rust. Минус: менее зрелый, больше runtime.
3. **WASM plugins** — пользователь компилирует правило в WASM. Максимальная flexibility, но высокий порог входа.
4. **Starlark (starlark-rs)** — Python-like (используется в Bazel/Buck). Знакомый синтаксис для большинства.

### Effort: 2-3 дня на интеграцию + sandboxing + примеры
### Prerequisite: Phase 2 rule engine (structured YAML) должен работать первым — DSL = расширение, не замена.

### Решение: отложить до Phase 3+. Сначала YAML rules покажут что пользователям реально нужно.

