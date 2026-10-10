from pathlib import Path
import json,re,statistics
ROOT=Path(__file__).parent
rubric={c['key']:c for c in json.loads((ROOT/'rubric.json').read_text())['cases']}
notes=json.loads((ROOT/'adjudications.json').read_text()) if (ROOT/'adjudications.json').exists() else {}
def norm(s):return re.sub('[^a-z0-9]','',s.lower())
rows=[json.loads(p.read_text()) for p in (ROOT/'results').glob('*.json')]
for r in rows:
 label=r.get('result',{}).get('documentType','');r['label']=label
 r['correct']=not r.get('error') and (norm(label) in {norm(s) for s in rubric[r['key']]['acceptedTypes']} or notes.get(r['key']+'|'+label,{}).get('accept',False))
summary=[]
for model in sorted({r['model'] for r in rows}):
 for mode in ['text','image']:
  for variant in ['old','new']:
   group=[r for r in rows if r['model']==model and r['mode']==mode and r['variant']==variant]
   if not group:continue
   summary.append({'model':model,'mode':mode,'variant':variant,'count':len(group),'correct':sum(r['correct'] for r in group),'developmentCorrect':sum(r['correct'] for r in group if r['split']=='development'),'developmentCount':sum(r['split']=='development' for r in group),'freshCorrect':sum(r['correct'] for r in group if r['split']=='fresh'),'freshCount':sum(r['split']=='fresh' for r in group),'errors':sum(bool(r.get('error')) for r in group),'medianSeconds':round(statistics.median(r['seconds'] for r in group if not r.get('error')),2) if any(not r.get('error') for r in group) else None})
(ROOT/'scored.json').write_text(json.dumps(rows,indent=2));(ROOT/'summary.json').write_text(json.dumps(summary,indent=2))
for s in summary:print(s['model'],s['mode'],s['variant'],str(s['correct'])+'/'+str(s['count']),'fresh',str(s['freshCorrect'])+'/'+str(s['freshCount']),'median',s['medianSeconds'])
