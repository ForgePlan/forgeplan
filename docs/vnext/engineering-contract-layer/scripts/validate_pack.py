#!/usr/bin/env python3
from pathlib import Path
import json, sys
root = Path(__file__).resolve().parents[1]
required = [
    root/'README.md', root/'issues/manifest.json', root/'issues/bodies.json',
    root/'governance/EXECUTION-ORDER.md', root/'governance/AGENTS-VNEXT.md',
    root/'prompts/PROGRAM-COORDINATOR.md', root/'prompts/ISSUE-BUILDER.md',
    root/'prompts/INDEPENDENT-VERIFIER.md'
]
missing=[str(p) for p in required if not p.exists()]
if missing:
    print('Missing files:', *missing, sep='\n- ', file=sys.stderr); raise SystemExit(1)
manifest=json.loads((root/'issues/manifest.json').read_text(encoding='utf-8'))
bodies=json.loads((root/'issues/bodies.json').read_text(encoding='utf-8'))
issues=manifest['issues']; keys=[i['key'] for i in issues]
if len(keys)!=len(set(keys)): raise SystemExit('Duplicate issue keys')
for i in issues:
    if i['key'] not in bodies: raise SystemExit(f"Missing body for {i['key']}")
    for dep in i.get('depends',[]):
        if dep not in keys: raise SystemExit(f"Unknown dependency {dep} in {i['key']}")
    repo_dir={'ForgePlan/forgeplan':'forgeplan','ForgePlan/marketplace':'marketplace','ForgePlan/forgeplan-web':'forgeplan-web'}[i['repo']]
    if not (root/'issues'/repo_dir/f"{i['key']}.md").exists(): raise SystemExit(f"Missing readable issue {i['key']}")
for schema in (root/'protocol/schemas').glob('*.json'):
    json.loads(schema.read_text(encoding='utf-8'))
print(f"OK: {len(issues)} issues, {len(list((root/'protocol/schemas').glob('*.json')))} schemas, prompts and governance present")
