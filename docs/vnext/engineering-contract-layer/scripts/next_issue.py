#!/usr/bin/env python3
from pathlib import Path
import json, subprocess, shutil
root=Path(__file__).resolve().parents[1]
data=json.loads((root/'issues/manifest.json').read_text(encoding='utf-8'))['issues']
state_path=root/'.created-issues.json'
state=json.loads(state_path.read_text(encoding='utf-8')) if state_path.exists() else {}

def closed(repo, number):
    if shutil.which('gh') is None: return False
    p=subprocess.run(['gh','issue','view',str(number),'--repo',repo,'--json','state'],capture_output=True,text=True)
    if p.returncode: return False
    try: return json.loads(p.stdout).get('state','').upper()=='CLOSED'
    except Exception: return False

completed=set()
for k,v in state.items():
    item=next((x for x in data if x['key']==k),None)
    if item and closed(item['repo'],v['number']): completed.add(k)
for item in sorted((x for x in data if x['key']!='FPV-00'), key=lambda x:(x['phase'],x['key'])):
    if item['key'] in completed: continue
    missing=[d for d in item.get('depends',[]) if d not in completed]
    if not missing:
        print(json.dumps({'next':item,'url':state.get(item['key'],{}).get('url')},ensure_ascii=False,indent=2)); break
else:
    print(json.dumps({'next':None,'message':'No unblocked issue found; check GitHub state or dependencies.'},ensure_ascii=False,indent=2))
