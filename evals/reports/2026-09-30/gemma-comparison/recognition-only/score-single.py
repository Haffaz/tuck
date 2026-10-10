from pathlib import Path
import json,re,statistics
ROOT=Path(__file__).parent
aliases=json.loads((ROOT/'simple-aliases.json').read_text())
def norm(s):return re.sub('[^a-z0-9]','',s.lower())
rows=[json.loads(p.read_text()) for p in (ROOT/'single-results').glob('*.json')]
for r in rows:
 result=r.get('result',{})
 assert r.get('error') or set(result)=={'documentType'}, 'Unexpected model-generated field'
 key=r['case']+'/'+r['sourceId']; label=result.get('documentType','')
 r['correct']=not r.get('error') and norm(label) in {norm(s) for s in aliases[key]}
for model in sorted({r['model'] for r in rows}):
 for mode in ['text','image']:
  group=[r for r in rows if r['model']==model and r['mode']==mode]
  if group: print(model,mode,sum(r['correct'] for r in group),'/',len(group),'real',sum(r['correct'] for r in group if r['kind']=='personal'),'/',sum(r['kind']=='personal' for r in group),'seconds',round(statistics.median(r['seconds'] for r in group),2))
(ROOT/'single-scored.json').write_text(json.dumps(rows,indent=2))
