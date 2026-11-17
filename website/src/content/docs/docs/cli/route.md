---
title: forgeplan route
description: "Suggest a task depth and artifact pipeline before shaping. This is a recommendation, not implementation proof, a safety waiver, or automatic artifact creation."
---

Suggest a task depth and artifact pipeline before shaping. This is a recommendation, not implementation proof, a safety waiver, or automatic artifact creation.

## Usage

```text
forgeplan route [OPTIONS] <DESCRIPTION>
```

## Arguments

```text
  <DESCRIPTION>  Task description in natural language
```

## Options

```text
      --explain        Optional: use LLM to explain the routing decision (deprecated, use --level 1)
      --level <LEVEL>  Routing level: 0 = keywords (default), 1 = LLM-classified
  -h, --help           Print help
  -V, --version        Print version
```

## Routing levels

The runtime default is automatic, despite 0.38.0 help describing Level 0 as default. `--level 0` forces keyword routing. Automatic mode tries the configured LLM and falls back to keywords when unavailable; a Deep result can trigger Level 2 reasoning. `--level 1` requests LLM classification, and runtime accepts `--level 2` for classification plus reasoning. `--explain` is the deprecated Level 1 alias.

Even an explicit Level 1 or 2 can fall back to keywords on missing configuration, credentials, provider failure, or an unparseable response. Level 2 reasoning runs only after classification is Deep/Critical. Keyless providers such as Ollama and `claude-code` do not require an API key, but still need their runtime or CLI authentication. For offline routing use `--level 0` **without** `--explain`: their combination can still invoke the legacy LLM explanation path. Output is text only; there is no JSON option.

## Examples

```bash
forgeplan route --level 0 "add rate limiting to API"
forgeplan route --level 1 "refactor the embedding pipeline"
forgeplan route "fix typo in README"
```

## Interpretation and next steps

Compare depth, pipeline, confidence, rationale, and alternatives with the actual scope and reversibility. Low confidence calls for more detail or review. LLM calls consume the provider service or vendor plan; force Level 0 for offline routing. Routing can update session state, but you create artifacts explicitly with `new`. Use `calibrate` for an existing artifact. Do not assume activation trains a router or a historical-confidence threshold controls CLI escalation.

When a workspace is available, the handler attempts to save `route_depth`, session phase `routing`, and a phase timestamp; save failures are ignored. Keyword routing can run without a workspace. An empty description is rejected.

## See also

- [`forgeplan new`](/docs/cli/new/) - create the artifact suggested by route
- [`forgeplan reason`](/docs/cli/reason/) - required for Deep/Critical depth
- [`forgeplan calibrate`](/docs/cli/calibrate/) - re-route an existing artifact
- [`forgeplan health`](/docs/cli/health/) - session start check
- [Methodology: depth calibration](/docs/methodology/overview/)
