# WorkContract Specification

## Определение

WorkContract — immutable, versioned и machine-readable проекция применимых артефактов, решений и policies для одной ограниченной единицы инженерной работы.

WorkContract не является новым вручную поддерживаемым artifact kind. Он компилируется из канонического графа.

## Обязательные разделы

- identity and version;
- source artifacts and digests;
- repository base state;
- objective and expected outcome;
- included and excluded scope;
- allowed and forbidden paths;
- applicable constraints and decisions;
- acceptance criteria with stable IDs;
- required Evidence by criterion;
- authority rules;
- external execution constraints;
- escalation and rollback rules.

## Компиляция

```bash
forgeplan contract compile PRD-073
forgeplan contract validate WC-073@3
forgeplan contract diff WC-073@2 WC-073@3
forgeplan contract export WC-073@3 --target codex
```

## Компилятор должен

1. разрешить source artifacts;
2. зафиксировать их content digest и git revision;
3. найти active decisions, которые применимы к affected paths/domain;
4. собрать constraints и non-goals;
5. нормализовать acceptance criteria;
6. определить Evidence requirements по depth и policy;
7. определить authority requirements;
8. выдать deterministic canonical representation;
9. объяснить происхождение каждого contract field;
10. отказать при unresolved contradictions.

## Contract provenance

Для каждого derived элемента сохраняется:

```text
source_artifact
source_section / field
source_digest
compiler_rule
```

## Изменение контракта

- до запуска разрешена новая version;
- после запуска scope/criteria/policy не меняются;
- repair execution может использовать тот же contract, если intent не изменился;
- change request создаёт новую contract version;
- старая version остаётся доступной для audit.

## Acceptance

WorkContract v1 готов, когда:

- deterministic compile подтверждён golden tests;
- schema round-trip проходит;
- diff показывает semantic changes;
- source provenance доступен для каждого derived field;
- CLI и MCP parity подтверждена;
- contract можно передать минимум в два разных host adapters.
