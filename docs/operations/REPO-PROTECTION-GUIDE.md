[English](REPO-PROTECTION-GUIDE.md) · [Русский](REPO-PROTECTION-GUIDE.ru.md)

# Repo Protection Guide — GitHub + CI + Agentic Development

A practical guide for setting up GitHub repository protection for projects with a dev-based workflow and AI agents (Claude Code, Cursor, etc).

> These are illustrative setup fragments for a separately authorized repository,
> not a description of ForgePlan's live rules or permission to change them.
> Do not apply the sample merge methods/review counts over this project's stricter
> policy. Native settings, hooks, rulesets and credentials remain unchanged here.
> Rules depend on plan, visibility, enforcement, selected refs, required-check
> names and bypass credentials. No live remote protection test was performed.

## Requirements

- A GitHub plan/visibility combination that supports the chosen rules and security features; public/private and personal/organization availability differ
- `gh` CLI installed and authenticated
- CI workflow (GitHub Actions)

## Protection Architecture

```
┌─ 3 layers of protection ─────────────────────────────┐
│                                                        │
│  Layer 1: Local (agent)                                │
│    hooks → block dangerous commands BEFORE push        │
│                                                        │
│  Layer 2: CI (GitHub Actions)                          │
│    workflows → verify code AFTER push                  │
│                                                        │
│  Layer 3: Remote (GitHub Rulesets)                      │
│    rulesets → block merge WITHOUT passing CI            │
│                                                        │
└────────────────────────────────────────────────────────┘
```

---

## Step 1: CI Workflow

Create `.github/workflows/ci.yml` — a minimal CI for a Rust project:

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

For Node.js/Python/Go — replace the steps with your own, but **keep the job names** (`Check, Lint & Format` and `Tests`) — they are used in rulesets.

### Adapting for Other Stacks

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

## Step 2: GitHub Rulesets (via API)

### Variables

```bash
OWNER="MyOrg"          # or username
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

**Parameters:**

| Parameter | Value | Description |
|-----------|-------|-------------|
| `required_approving_review_count` | `0` for solo, `1` for teams | How many approvals required |
| `strict_required_status_checks_policy` | `true` | PR must be up-to-date with main |
| `do_not_enforce_on_create` | `true` | Don't block branch creation |
| `bypass_actors.actor_id: 5` | Repository Admin | Emergency bypass |
| `allowed_merge_methods` | sample `["merge", "squash"]` | Generic example, not this project's merge-commit-only convention |

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

**Difference from Main:** `strict = false` (does not require rebase before merge), `rebase` in allowed merge methods.

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

These sample rules exempt the configured repository-admin role. An agent using
admin/bypass credentials can also tag; “agent” is not a GitHub permission class.
Rules must be active and applicable, and bypass possession is not authorization.

---

## Step 3: Security Features

```bash
# Push protection detects supported secret patterns; it does not recognize every secret.
gh api repos/${FULL} --method PATCH --input - <<'EOF'
{
  "security_and_analysis": {
    "secret_scanning": { "status": "enabled" },
    "secret_scanning_push_protection": { "status": "enabled" }
  }
}
EOF

# Dependabot alerts — CVEs in dependencies
gh api repos/${FULL}/vulnerability-alerts --method PUT
```

---

## Step 4: Local Hooks (for AI agents)

### 4.1 Safety hook — blocks dangerous commands

Create `.claude/hooks/safety-hook.sh`:

```bash
#!/bin/bash
# PreToolUse hook — blocks dangerous bash commands
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

### 4.2 Format hook — cargo fmt before commit

Create `.claude/hooks/pre-commit-fmt.sh`:

```bash
#!/bin/bash
# PreToolUse hook — checks formatting before git commit
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

The hook samples are literal string checks for configured `Bash` calls.
They are not a parser, sandbox or protection for all clients/equivalent commands.
Repository prohibitions remain binding when detection misses them.

### 4.3 Configuration in settings.json

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

## Step 5: Verification

### Check rulesets:

```bash
# List all rulesets
gh api repos/${FULL}/rulesets --jq '.[] | {name: .name, enforcement: .enforcement}'

# Details for a specific ruleset (substitute ID)
gh api repos/${FULL}/rulesets/RULESET_ID
```

### Check security:

```bash
gh api repos/${FULL} --jq '.security_and_analysis'
```

### Do not test by pushing a fake change to real main

The read-only inspections above establish configured rules, not a failed-push
receipt. A negative test requires separate authorization, a disposable repository
and a non-bypass identity. With the sample admin bypass, a real-main “should fail”
push can succeed and mutate protected history. Do not use it as a harmless check.

---

## Step 6: Updating Rulesets

```bash
# Get the ruleset ID
RULESET_ID=$(gh api repos/${FULL}/rulesets --jq '.[] | select(.name == "Main") | .id')

# Update (PUT completely overwrites)
gh api repos/${FULL}/rulesets/${RULESET_ID} --method PUT --input - <<'EOF'
{ ... updated configuration ... }
EOF
```

---

## Quick Setup Script

Historical setup example, not an idempotent installer. It creates new rulesets
and suppresses some errors; the final `Done!` is not proof of successful setup.
Inspect each response and the resulting rules before relying on protection.

```bash
#!/bin/bash
# setup-protection.sh — sets up GitHub repo protection
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

## Matrix: What Protects Against What

A checkmark means conditional coverage when configured and applicable, not
complete prevention. Dependabot reports known dependency advisories; it does
not itself block every vulnerable release. Secret detection covers supported
patterns. Bypass identities and disabled rules can bypass the shown barriers.

| Threat | Local Hook | CI | Ruleset | Security |
|--------|:----------:|:--:|:-------:|:--------:|
| Direct push to main/dev | | | ✅ | |
| Force push | ✅ | | ✅ | |
| Merge without tests | | ✅ | ✅ | |
| Incorrect formatting | ✅ | ✅ | | |
| Secrets in code | | | | ✅ |
| CVEs in dependencies | | | | ✅ |
| Accidental release tag | | | ✅ | |
| `rm -rf /` | ✅ | | | |
| `cargo publish` | ✅ | | | |

## FAQ

**Q: Do I need GitHub Team/Pro?**
A: For public repos — no, GitHub Free is sufficient. For private repos — you need Team (org) or Pro (personal).

**Q: The agent cannot merge a PR — what should I do?**
A: Check that CI checks passed (job names must exactly match `required_status_checks.context`).

**Q: How do I temporarily disable protection?**
A: Do not bypass or weaken rules to finish an agent task. An authorized owner
must separately approve any policy change and inspect supported enforcement
modes for that plan; `evaluate` is not universally available. Admin capability
alone is not authorization.

**Q: How do I add a review requirement for a team?**
A: Change `required_approving_review_count` to `1` (or more) in the PR rule.
