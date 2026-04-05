---
title: Quick Start
description: Create your first artifact in 2 minutes
---

## Initialize Workspace

```bash
forgeplan init -y
```

## Route Your Task

```bash
forgeplan route "add user authentication"
# → Depth: Standard, Pipeline: PRD → RFC
```

## Create an Artifact

```bash
forgeplan new prd "User Authentication"
# → Created: PRD-001
```

## Validate

```bash
forgeplan validate PRD-001
# → PASS ✓
```
