from pathlib import Path
import json,subprocess,time,hashlib
ROOT=Path(__file__).parent
cases=json.loads((ROOT/'manifest-private.json').read_text());rows=[]
for c in cases:
 start=time.monotonic();texts=[]
 for page in c['sourcePages']:
  p=subprocess.run(['/Users/mohomedhaffaz/Projects/tuck/src-tauri/resources/tuck-reader',c['sourcePath'],str(page),'false'],capture_output=True,text=True,check=True,timeout=120)
  texts.append('Page '+str(page)+'\n'+json.loads(p.stdout)['text'])
 text='\n\n'.join(texts);elapsed=time.monotonic()-start
 rows.append({'key':c['key'],'seconds':elapsed,'characters':len(text),'textHash':hashlib.sha256(text.encode()).hexdigest(),'matchesStoredBaseline':text==Path(c['text']).read_text()})
 print(c['key'],'seconds',round(elapsed,2),'same baseline',rows[-1]['matchesStoredBaseline'],flush=True)
(ROOT/'baseline-extraction-metrics.json').write_text(json.dumps(rows,indent=2))
