[English](REPO-PROTECTION-GUIDE.md) · [Русский](REPO-PROTECTION-GUIDE.ru.md)

# Repo Protection Guide — GitHub + CI + Agentic Development

A practical guide for setting up GitHub repository protection for projects with a dev-based workflow and AI agents (Claude Code, Cursor, etc).

## Requirements

- GitHub Free (public repo) or GitHub Team (private repo)
- `gh` CLI installed and authenticated
- CI workflow (GitHub Actions)

The workflow and rulesets below are templates for a new repository, not
a dump of Forgeplan's current remote settings. Forgeplan's own `ci.yml`
also checks semantic-search, embeddings, the website, and documentation
drift; see [Quality Gates](QUALITY-GATES.md). Match required check names
to the jobs actually emitted by the target repository.

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

For Node.js/Python/Go, replace the steps but keep these job names **or**
update `required_status_checks.context` to match. The stack snippets below
are step fragments; keep the runners, checkout, and runtime setup from
the full workflow and add the appropriate language setup.

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
| `allowed_merge_methods` | `["merge", "squash"]` | Merge commit for releases, squash for features |

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

With this ruleset, tag creation, updates, and deletion require the admin
bypass. This is an identity boundary, not an agent boundary: an agent
using an admin token can bypass it too. Use non-bypass credentials for agents.

---

## Step 3: Security Features

```bash
# Secret scanning — blocks push with tokens/keys
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

These sample hooks are pattern-based guardrails for Claude Code Bash
calls, not a shell sandbox or native Git hooks. Other clients, equivalent
command spellings, and admin credentials remain outside their coverage.
They require Bash and `jq`; connect them explicitly in `.claude/settings.json`.

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
  if echo "$COMMAND" | grep -Fqi -- "$pattern"; then
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

### 4.3 Configuration in settings.json

Make both scripts executable before enabling them:
```bash
chmod +x .claude/hooks/safety-hook.sh .claude/hooks/pre-commit-fmt.sh
```

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

### Test protection (should fail):

Use a disposable test repository or a temporary branch covered by an
equivalent ruleset, and authenticate with a **non-bypass** account.
An admin token is explicitly exempted by the rulesets above, so its
successful push does not show that protection failed. Do not commit a
test change to the real `main` branch just to probe rejection.

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

One command for a new project:

```bash
#!/bin/bash
# setup-protection.sh — sets up GitHub repo protection
set -euo pipefail
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
gh api "repos/${FULL}" --method PATCH -f 'security_and_analysis[secret_scanning][status]=enabled' -f 'security_and_analysis[secret_scanning_push_protection][status]=enabled'

echo "  Enabling Dependabot..."
gh api "repos/${FULL}/vulnerability-alerts" --method PUT

echo "Done! Verify: gh api repos/${FULL}/rulesets"
```

The script stops on API errors; security features depend on repository
visibility, plan, and permissions. Inspect the returned settings rather
than treating a printed completion message as proof. Running the script
again creates additional rulesets; use Step 6 to update existing ones.

---

## Matrix: What Protects Against What

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
A: Use bypass (you are admin) or change `enforcement` to `"evaluate"` (will warn but not block).

**Q: How do I add a review requirement for a team?**
A: Change `required_approving_review_count` to `1` (or more) in the PR rule.
