---
title: "forgeplan constellation pack"
description: "Stage 6 — build a budgeted ContextPack for a task: lexical ranking, evidence units with roles, omissions and a spend receipt"
---

## Usage

`forgeplan constellation pack [OPTIONS] <TASK>` selects lexical seeds and
their declared evidence closure. The response preserves whole artifact bodies.

| Option | Default | Description |
|---|---:|---|
| `--seed <KEY[@SOURCE]>` | none | Explicit selector; repeat for multiple seeds |
| `--budget <N>` | 8000 | Estimated tokens for the complete output, UTF-8 bytes / 4 rounded up |
| `--byte-cap <N>` | 1000000 | Exact maximum output bytes, including newline |
| `--max-depth <N>` | 4 | Relation depth, maximum 64 |
| `--max-nodes <N>` | 256 | Visited nodes, maximum 4096 |
| `--max-edges <N>` | 1024 | Traversed edges, maximum 16384 |
| `--max-millis <N>` | 5000 | Traversal time, maximum 60000 |
| `--require-complete` | false | Refuse incomplete sources, traversal or delivery |
| `--cursor <TOKEN>` | none | Continue the same selection and presentation |
| `--retrieval-mode <MODE>` | lexical | Only `lexical` is supported |
| `--mermaid` | false | Include Mermaid graph navigation |
| `--snapshot <ID>` | none | Read pinned Git revisions |
| `--allow-partial` | false | Allow an unavailable required source |
| `--json` | false | Emit compact structured JSON |

The smaller of `--byte-cap` and four times `--budget` limits each complete
response. If a whole unit cannot fit, increase both limits. Continue with the
returned `next_cursor` and unchanged options. A cursor does not extend a
traversal stopped by its limits. See the
[pack reference](/docs/mcp/constellation_pack/) for coverage fields.

## Examples

- [Reproducible Constellation walkthrough](/docs/guides/constellation/)
- [Artifact quick start](/docs/getting-started/quick-start/)

## See also

- [CLI reference](/docs/cli/)
- [Constellation reference](/docs/reference/constellation/)

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan constellation pack --help`.

```text
Stage 6 — build a budgeted ContextPack for a task: lexical ranking, evidence units with roles, omissions and a spend receipt

Usage: forgeplan constellation pack [OPTIONS] <TASK>

Arguments:
  <TASK>  What the agent must do — the ranking terms come from this text

Options:
      --budget <BUDGET>
          Estimated token allowance for the complete response (UTF-8 bytes / 4) [default: 8000]
      --byte-cap <BYTE_CAP>
          Exact maximum bytes in the complete output [default: 1000000]
      --seed <SEED>
          Explicit artifact selector, optionally qualified as KEY@source
      --max-depth <MAX_DEPTH>
          Maximum declared relation depth [default: 4]
      --max-nodes <MAX_NODES>
          Maximum visited nodes [default: 256]
      --max-edges <MAX_EDGES>
          Maximum traversed edges [default: 1024]
      --max-millis <MAX_MILLIS>
          Maximum traversal time in milliseconds [default: 5000]
      --require-complete
          Refuse incomplete sources, obligations or page delivery
      --cursor <CURSOR>
          Continue a previous response on the same basis and presentation
      --retrieval-mode <RETRIEVAL_MODE>
          Retrieval mode (only lexical is supported) [default: lexical]
      --mermaid
          Include Mermaid graph navigation
      --snapshot <SNAPSHOT>
          Pack at a recorded snapshot instead of the live sources
      --allow-partial
          Tolerate dead sources: even a required source's outage becomes a `failures` entry on a partial answer instead of an error
      --json
          Output as JSON
  -h, --help
          Print help
  -V, --version
          Print version
```
<!-- forgeplan-runtime:end -->
