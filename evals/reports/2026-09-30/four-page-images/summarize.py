from pathlib import Path
import csv,json,statistics
ROOT=Path(__file__).parent
rows=[]
for p in (ROOT/'results').glob('*.json'):
 r=json.loads(p.read_text())
 if 'mode' in r: rows.append(r)
summary=[]
for model in ['apple','qwen3.5:4b','qwen3.5:9b']:
 for mode in ['text','separate_pages','four_up','four_up_text']:
  group=[r for r in rows if r['model']==model and r['mode']==mode]
  if not group:continue
  scores=[s for r in group for s in r['scores']]
  valid=[r['seconds'] for r in group if not r['error']]
  summary.append(dict(model=model,mode=mode,requests=len(group),documents=len(scores),correctFolders=sum(s['folderCorrect'] for s in scores),correctTypes=sum(s['typeCorrect'] for s in scores),wrongDestinations=sum(s['wrongAutomatic'] for s in scores),fallbacks=sum(s['fallback'] for s in scores),requestErrors=sum(bool(r['error']) for r in group),markersRead=sum(s['markerCorrect'] is True for s in scores),markersTotal=sum(s['markerCorrect'] is not None for s in scores),meanInputTokens=round(statistics.mean(r['metrics']['prompt_eval_count'] for r in group if r.get('metrics')),1) if any(r.get('metrics') for r in group) else None,medianSeconds=round(statistics.median(valid),2) if valid else None,totalSeconds=round(sum(r['seconds'] for r in group),2),syntheticCorrect=sum(s['folderCorrect'] for r in group if r['kind']=='synthetic' for s in r['scores']),personalCorrect=sum(s['folderCorrect'] for r in group if r['kind']=='personal' for s in r['scores'])))
(ROOT/'summary.json').write_text(json.dumps(summary,indent=2))
with (ROOT/'summary.csv').open('w') as f:
 writer=csv.DictWriter(f,fieldnames=summary[0].keys());writer.writeheader();writer.writerows(summary)
for s in summary: print(s['model'],s['mode'],f"{s['correctFolders']}/{s['documents']}",f"types {s['correctTypes']}/{s['documents']}",f"marker {s['markersRead']}/{s['markersTotal']}",s['medianSeconds'],'seconds',s['requestErrors'],'errors')
