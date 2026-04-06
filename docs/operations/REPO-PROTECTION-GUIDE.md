# Repo Protection Guide — GitHub + CI + Agentic Development

Практический гайд по настройке защиты GitHub репозитория для проектов с dev-based workflow и AI-агентами (Claude Code, Cursor, etc).

## Требования

- GitHub Free (public repo) или GitHub Team (private repo)
- `gh` CLI установлен и авторизован
- CI workflow (GitHub Actions)

## Архитектура защиты

```
┌─ 3 уровня защиты ──────────────────────────────────┐
│                                                      │
│  Layer 1: Local (агент)                              │
│    hooks → блокируют опасные команды ДО push         │
│                                                      │
│  Layer 2: CI (GitHub Actions)                        │
│    workflows → проверяют код ПОСЛЕ push              │
│                                                      │
│  Layer 3: Remote (GitHub Rulesets)                    │
│    rulesets → блокируют merge БЕЗ прохождения CI     │
│                                                      │
└──────────────────────────────────────────────────────┘
```

---

## Шаг 1: CI Workflow

Создай `.github/workflows/ci.yml` — минимальный CI для Rust проекта:

```yaml
name: CI

on:
  push:
    branches: [dev]
  pull_request:
    branches: [dev, main]

permissions:
  contents: read

env:
  CARGO_TERM_COLOR: always
  RUSTFLAGS: "-Dwarnings"

jobs:
  check:
    name: Check, Lint & Format
    runs-on: ubuntu-latest
    timeout-minutes: 10
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --all -- --check
      - run: cargo check --workspace --all-targets
      - run: cargo clippy --workspace --all-targets

  test:
    name: Tests
    runs-on: ubuntu-latest
    needs: check
    timeout-minutes: 20
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo test --workspace
```

Для Node.js/Python/Go — замени шаги на свои, но **сохрани имена jobs** (`Check, Lint & Format` и `Tests`) — они используются в rulesets.

### Адаптация для других стеков

**Node.js:**
```yaml
  check:
    name: Check, Lint & Format
    steps:
      - run: npm ci
      - run: npm run lint
      - run: npm run typecheck
  test:
    name: Tests
    steps:
      - run: npm ci
      - run: npm test
```

**Python:**
```yaml
  check:
    name: Check, Lint & Format
    steps:
      - run: pip install ruff mypy
      - run: ruff check .
      - run: mypy .
  test:
    name: Tests
    steps:
      - run: pip install -e ".[test]"
      - run: pytest
```

---

## Шаг 2: GitHub Rulesets (через API)

### Переменные

```bash
OWNER="MyOrg"          # или username
REPO="my-project"
FULL="${OWNER}/${REPO}"
```

### 2.1 Main branch protection

```bash
gh api repos/${FULL}/rulesets --method POST --input - <<'EOF'
{
  "name": "Main",
  "target": "branch",
  "enforcement": "active",
  "conditions": {
    "ref_name": {
      "include": ["refs/heads/main"],
      "exclude": []
    }
  },
  "bypass_actors": [
    {
      "actor_id": 5,
      "actor_type": "RepositoryRole",
      "bypass_mode": "always"
    }
  ],
  "rules": [
    { "type": "deletion" },
    { "type": "non_fast_forward" },
    { "type": "pull_request", "parameters": {
        "required_approving_review_count": 0,
        "dismiss_stale_reviews_on_push": false,
        "require_code_owner_review": false,
        "require_last_push_approval": false,
        "required_review_thread_resolution": false,
        "allowed_merge_methods": ["merge", "squash"]
      }
    },
    { "type": "required_status_checks", "parameters": {
        "strict_required_status_checks_policy": true,
        "do_not_enforce_on_create": true,
        "required_status_checks": [
          { "context": "Tests" },
          { "context": "Check, Lint & Format" }
        ]
      }
    }
  ]
}
EOF
```

**Параметры:**

| Параметр | Значение | Пояснение |
|----------|----------|-----------|
| `required_approving_review_count` | `0` для solo, `1` для команды | Сколько approve нужно |
| `strict_required_status_checks_policy` | `true` | PR должен быть up-to-date с main |
| `do_not_enforce_on_create` | `true` | Не блокировать создание веток |
| `bypass_actors.actor_id: 5` | Repository Admin | Экстренный bypass |
| `allowed_merge_methods` | `["merge", "squash"]` | Merge commit для releases, squash для features |

### 2.2 Dev branch protection

```bash
gh api repos/${FULL}/rulesets --method POST --input - <<'EOF'
{
  "name": "Dev",
  "target": "branch",
  "enforcement": "active",
  "conditions": {
    "ref_name": {
      "include": ["refs/heads/dev"],
      "exclude": []
    }
  },
  "bypass_actors": [
    {
      "actor_id": 5,
      "actor_type": "RepositoryRole",
      "bypass_mode": "always"
    }
  ],
  "rules": [
    { "type": "deletion" },
    { "type": "non_fast_forward" },
    { "type": "pull_request", "parameters": {
        "required_approving_review_count": 0,
        "dismiss_stale_reviews_on_push": false,
        "require_code_owner_review": false,
        "require_last_push_approval": false,
        "required_review_thread_resolution": false,
        "allowed_merge_methods": ["merge", "squash", "rebase"]
      }
    },
    { "type": "required_status_checks", "parameters": {
        "strict_required_status_checks_policy": false,
        "do_not_enforce_on_create": true,
        "required_status_checks": [
          { "context": "Tests" },
          { "context": "Check, Lint & Format" }
        ]
      }
    }
  ]
}
EOF
```

**Отличие от Main:** `strict = false` (не требует rebase перед merge), `rebase` в allowed merge methods.

### 2.3 Tag protection

```bash
gh api repos/${FULL}/rulesets --method POST --input - <<'EOF'
{
  "name": "Tags",
  "target": "tag",
  "enforcement": "active",
  "conditions": {
    "ref_name": {
      "include": ["~ALL"],
      "exclude": []
    }
  },
  "bypass_actors": [
    {
      "actor_id": 5,
      "actor_type": "RepositoryRole",
      "bypass_mode": "always"
    }
  ],
  "rules": [
    { "type": "creation" },
    { "type": "update" },
    { "type": "deletion" }
  ]
}
EOF
```

Только admin может создавать/удалять tags. Агенты не смогут случайно тегировать.

---

## Шаг 3: Security Features

```bash
# Secret scanning — блокирует push с токенами/ключами
gh api repos/${FULL} --method PATCH --input - <<'EOF'
{
  "security_and_analysis": {
    "secret_scanning": { "status": "enabled" },
    "secret_scanning_push_protection": { "status": "enabled" }
  }
}
EOF

# Dependabot alerts — CVE в зависимостях
gh api repos/${FULL}/vulnerability-alerts --method PUT
```

---

## Шаг 4: Local Hooks (для AI-агентов)

### 4.1 Safety hook — блокирует опасные команды

Создай `.claude/hooks/safety-hook.sh`:

```bash
#!/bin/bash
# PreToolUse hook — блокирует опасные bash команды
# Trigger: Bash tool calls

INPUT=$(cat)
COMMAND=$(echo "$INPUT" | jq -r '.tool_input.command // empty')

BLOCKED_PATTERNS=(
  "git push --force"
  "git push -f"
  "git reset --hard"
  "git clean -fd"
  "rm -rf /"
  "rm -rf ~"
  "cargo publish"
  "DROP TABLE"
  "DROP DATABASE"
)

for pattern in "${BLOCKED_PATTERNS[@]}"; do
  if echo "$COMMAND" | grep -qi "$pattern"; then
    echo "BLOCKED: '$pattern' is not allowed. Use PR workflow instead." >&2
    exit 2
  fi
done

exit 0
```

### 4.2 Format hook — cargo fmt перед коммитом

Создай `.claude/hooks/pre-commit-fmt.sh`:

```bash
#!/bin/bash
# PreToolUse hook — проверяет форматирование перед git commit
# Trigger: Bash tool calls containing "git commit"

INPUT=$(cat)
COMMAND=$(echo "$INPUT" | jq -r '.tool_input.command // empty')

if echo "$COMMAND" | grep -q "git commit"; then
  FMT_CHECK=$(cargo fmt -- --check 2>&1)
  if [ $? -ne 0 ]; then
    echo "BLOCKED: cargo fmt check failed. Run 'cargo fmt' first." >&2
    echo "$FMT_CHECK" >&2
    exit 2
  fi
fi

exit 0
```

### 4.3 Настройка в settings.json

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          { "type": "command", "command": ".claude/hooks/safety-hook.sh" },
          { "type": "command", "command": ".claude/hooks/pre-commit-fmt.sh" }
        ]
      }
    ]
  }
}
```

---

## Шаг 5: Верификация

### Проверить rulesets:

```bash
# Список всех rulesets
gh api repos/${FULL}/rulesets --jq '.[] | {name: .name, enforcement: .enforcement}'

# Детали конкретного ruleset (подставь ID)
gh api repos/${FULL}/rulesets/RULESET_ID
```

### Проверить security:

```bash
gh api repos/${FULL} --jq '.security_and_analysis'
```

### Тест защиты (должен fail):

```bash
# Попробуй push напрямую в main — должно быть rejected
git checkout main
echo "test" >> README.md
git add README.md && git commit -m "test direct push"
git push origin main
# Expected: rejected by ruleset
```

---

## Шаг 6: Обновление rulesets

```bash
# Получить ID рулесета
RULESET_ID=$(gh api repos/${FULL}/rulesets --jq '.[] | select(.name == "Main") | .id')

# Обновить (PUT полностью перезаписывает)
gh api repos/${FULL}/rulesets/${RULESET_ID} --method PUT --input - <<'EOF'
{ ... обновлённая конфигурация ... }
EOF
```

---

## Quick Setup Script

Одна команда для нового проекта:

```bash
#!/bin/bash
# setup-protection.sh — настраивает защиту GitHub repo
# Usage: ./setup-protection.sh MyOrg/my-project

FULL="${1:?Usage: $0 OWNER/REPO}"

echo "Setting up protection for ${FULL}..."

# 1. Main ruleset
echo "  Creating Main ruleset..."
gh api repos/${FULL}/rulesets --method POST --input - <<'MAIN'
{"name":"Main","target":"branch","enforcement":"active","conditions":{"ref_name":{"include":["refs/heads/main"],"exclude":[]}},"bypass_actors":[{"actor_id":5,"actor_type":"RepositoryRole","bypass_mode":"always"}],"rules":[{"type":"deletion"},{"type":"non_fast_forward"},{"type":"pull_request","parameters":{"required_approving_review_count":0,"dismiss_stale_reviews_on_push":false,"require_code_owner_review":false,"require_last_push_approval":false,"required_review_thread_resolution":false,"allowed_merge_methods":["merge","squash"]}},{"type":"required_status_checks","parameters":{"strict_required_status_checks_policy":true,"do_not_enforce_on_create":true,"required_status_checks":[{"context":"Tests"},{"context":"Check, Lint & Format"}]}}]}
MAIN

# 2. Dev ruleset
echo "  Creating Dev ruleset..."
gh api repos/${FULL}/rulesets --method POST --input - <<'DEV'
{"name":"Dev","target":"branch","enforcement":"active","conditions":{"ref_name":{"include":["refs/heads/dev"],"exclude":[]}},"bypass_actors":[{"actor_id":5,"actor_type":"RepositoryRole","bypass_mode":"always"}],"rules":[{"type":"deletion"},{"type":"non_fast_forward"},{"type":"pull_request","parameters":{"required_approving_review_count":0,"dismiss_stale_reviews_on_push":false,"require_code_owner_review":false,"require_last_push_approval":false,"required_review_thread_resolution":false,"allowed_merge_methods":["merge","squash","rebase"]}},{"type":"required_status_checks","parameters":{"strict_required_status_checks_policy":false,"do_not_enforce_on_create":true,"required_status_checks":[{"context":"Tests"},{"context":"Check, Lint & Format"}]}}]}
DEV

# 3. Tag protection
echo "  Creating Tags ruleset..."
gh api repos/${FULL}/rulesets --method POST --input - <<'TAGS'
{"name":"Tags","target":"tag","enforcement":"active","conditions":{"ref_name":{"include":["~ALL"],"exclude":[]}},"bypass_actors":[{"actor_id":5,"actor_type":"RepositoryRole","bypass_mode":"always"}],"rules":[{"type":"creation"},{"type":"update"},{"type":"deletion"}]}
TAGS

# 4. Security
echo "  Enabling secret scanning..."
gh api repos/${FULL} --method PATCH -f security_and_analysis[secret_scanning][status]=enabled -f security_and_analysis[secret_scanning_push_protection][status]=enabled 2>/dev/null

echo "  Enabling Dependabot..."
gh api repos/${FULL}/vulnerability-alerts --method PUT 2>/dev/null

echo "Done! Verify: gh api repos/${FULL}/rulesets"
```

---

## Матрица: что защищает от чего

| Угроза | Local Hook | CI | Ruleset | Security |
|--------|:----------:|:--:|:-------:|:--------:|
| Push в main/dev напрямую | | | ✅ | |
| Force push | ✅ | | ✅ | |
| Merge без тестов | | ✅ | ✅ | |
| Некорректное форматирование | ✅ | ✅ | | |
| Secrets в коде | | | | ✅ |
| CVE в зависимостях | | | | ✅ |
| Случайный release tag | | | ✅ | |
| `rm -rf /` | ✅ | | | |
| `cargo publish` | ✅ | | | |

## FAQ

**Q: Нужен ли мне GitHub Team/Pro?**
A: Для public repos — нет, GitHub Free достаточно. Для private repos — нужен Team (org) или Pro (personal).

**Q: Агент не может merge PR — что делать?**
A: Проверь что CI checks прошли (имена jobs должны точно совпадать с `required_status_checks.context`).

**Q: Как временно отключить protection?**
A: Используй bypass (ты admin) или измени `enforcement` на `"evaluate"` (будет предупреждать, но не блокировать).

**Q: Как добавить review requirement для команды?**
A: Измени `required_approving_review_count` на `1` (или больше) в PR rule.
