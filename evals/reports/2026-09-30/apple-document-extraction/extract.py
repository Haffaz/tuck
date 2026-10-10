from pathlib import Path
import json,subprocess,time,hashlib,html
ROOT=Path(__file__).parent
cases=json.loads((ROOT/'manifest-private.json').read_text())
OUT=ROOT/'inputs';OUT.mkdir(mode=0o700,exist_ok=True)
def overlap(a,b):
 x,y,w,h=a;u,v,s,t=b
 return max(0,min(x+w,u+s)-max(x,u))*max(0,min(y+h,v+t)-max(y,v))/max(w*h,1e-9)
def serialize(doc):
 blocks=[];regions=[]
 for table in doc['tables']:
  parts=['<table>'];seen=set()
  for row in table['rows']:
   cells=[]
   for c in row:
    key=(c['rowStart'],c['colStart'])
    if key in seen:continue
    seen.add(key)
    attrs=''
    if c['rowEnd']>c['rowStart']:attrs+=' rowspan="'+str(c['rowEnd']-c['rowStart']+1)+'"'
    if c['colEnd']>c['colStart']:attrs+=' colspan="'+str(c['colEnd']-c['colStart']+1)+'"'
    cells.append('<td'+attrs+'>'+html.escape(c['text']).replace('\n','<br>')+'</td>')
   parts.append('<tr>'+''.join(cells)+'</tr>')
  parts.append('</table>');blocks.append((table['bounds'],'\n'.join(parts)));regions.append(table['bounds'])
 for li in doc['lists']:
  blocks.append((li['bounds'],'\n'.join((i['marker'] or '-')+' '+i['text'] for i in li['items'])));regions.append(li['bounds'])
 for p in doc['paragraphs']:
  if any(overlap(p['bounds'],b)>0.8 for b in regions):continue
  text=p['text']
  if doc['title'] and text.strip()==doc['title'].strip():text='# '+text
  blocks.append((p['bounds'],text))
 # Simple geometric ordering for this experimental adapter. Native paragraphs retain content.
 blocks.sort(key=lambda b:(-(b[0][1]+b[0][3]),b[0][0]))
 return '\n\n'.join(text for _,text in blocks)
check=subprocess.run([str(ROOT/'document-reader'),'--check'],capture_output=True,text=True,check=True)
(ROOT/'capabilities.json').write_text(check.stdout)
metrics=[]
for c in cases:
 key=c['key'];folder=OUT/key;folder.mkdir(mode=0o700,exist_ok=True)
 start=time.monotonic()
 try:
  run=subprocess.run([str(ROOT/'document-reader')],input=json.dumps({'path':c['sourcePath'],'pages':c['sourcePages']}),text=True,capture_output=True,timeout=600,check=True)
  raw=json.loads(run.stdout)
 except Exception as e:raw={'error':type(e).__name__}
 elapsed=time.monotonic()-start
 (folder/'structured-private.json').write_text(json.dumps(raw,indent=2))
 if raw.get('error'):
  metrics.append({'key':key,'error':raw['error'],'seconds':elapsed});print(key,'ERROR',raw['error'],flush=True);continue
 texts=[]
 for p in raw['pages']:
  content='\n\n'.join(serialize(d) for d in p['documents'])
  if not content.strip():content=p['transcript']
  # Match original evidence's page wrappers instead of adding new identity hints.
  texts.append(f"Page {p['page']}\n[Document has {raw['pageCount']} pages.]\n\n[Page {p['page']}]\n"+content)
 text='\n\n'.join(texts);tp=folder/'text.txt';tp.write_text(text)
 old=Path(c['text']).read_text()
 metric={'key':key,'seconds':elapsed,'pages':len(raw['pages']),'tables':sum(p['tables'] for p in raw['pages']),'lists':sum(p['lists'] for p in raw['pages']),'paragraphs':sum(p['paragraphs'] for p in raw['pages']),'characters':len(text),'oldCharacters':len(old),'textHash':hashlib.sha256(text.encode()).hexdigest(),'sourceHash':hashlib.sha256(Path(c['sourcePath']).read_bytes()).hexdigest(),'pageSeconds':[p['seconds'] for p in raw['pages']]}
 metrics.append(metric)
 print(key,'pages',metric['pages'],'tables',metric['tables'],'lists',metric['lists'],'seconds',round(elapsed,2),'chars',len(text),flush=True)
 (ROOT/'extraction-metrics.json').write_text(json.dumps(metrics,indent=2))
(ROOT/'extraction-metrics.json').write_text(json.dumps(metrics,indent=2))
print('EXTRACTION COMPLETE',len(metrics),flush=True)
