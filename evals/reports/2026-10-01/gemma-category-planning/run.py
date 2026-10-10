import argparse,hashlib,json,os,re,statistics,subprocess,time,urllib.request
from pathlib import Path
P=argparse.ArgumentParser();P.add_argument('--repo',type=Path,default=Path('/Users/mohomedhaffaz/Projects/tuck'));P.add_argument('--output',type=Path,required=True);P.add_argument('--personal-root',type=Path);P.add_argument('--validator',type=Path,required=True);A=P.parse_args()
R=A.repo; OUT=A.output;OUT.mkdir(parents=True,exist_ok=False)
APPLE=R/'src-tauri/resources/tuck-apple'; READER=R/'src-tauri/resources/tuck-reader'
MODELS=['gemma4:e2b-it-q4_K_M','gemma4:e4b-it-q4_K_M']; OPTIONS={'temperature':0,'seed':42,'num_ctx':8192,'num_predict':1000}
def digest(b):return hashlib.sha256(b).hexdigest()
def encoded(x):return json.dumps(x,ensure_ascii=False,sort_keys=True,separators=(',',':'))
def dump(name,x):(OUT/name).write_text(json.dumps(x,ensure_ascii=False,indent=2)+'\n')
def api(endpoint,payload=None,timeout=90):
 req=urllib.request.Request('http://127.0.0.1:11434/api/'+endpoint,data=None if payload is None else encoded(payload).encode(),headers={'Content-Type':'application/json'})
 with urllib.request.urlopen(req,timeout=timeout) as f:return json.load(f)
def swift_string(raw):return json.loads('"'+raw+'"')
source=(R/'src-tauri/swift/TuckApple.swift').read_text();section=source[source.index('request["operation"] as? String == "categories"'):]
INSTRUCTIONS=swift_string(re.search(r'let instructions = "((?:\\.|[^"\\])*)"',section).group(1))
RETRY=swift_string(re.search(r'instructions \+ \(retry \? "((?:\\.|[^"\\])*)"',section).group(1))
(OUT/'instructions.txt').write_text(INSTRUCTIONS+'\n\nRetry addition:\n'+RETRY+'\n')
def validate(text):
 p=subprocess.run([str(A.validator)],input=text,text=True,capture_output=True,timeout=10)
 if p.returncode:raise ValueError('invalid_categories')
 return json.loads(p.stdout)['rules']
control=json.loads((R/'evals/reports/2026-10-01/category-specificity/controlled.json').read_text())['input']
CASES=[('controlled',control)]; before={}; cache_verified=0; extraction_checks=[]
if A.personal_root:
 root=A.personal_root.resolve(); identity='apple-on-device:roles-v5-simple:'+digest(APPLE.read_bytes()); samples=[]
 paths=[]
 for directory,dirs,names in os.walk(root,followlinks=False):
  dirs[:]=[d for d in dirs if not d.startswith('.') and not (Path(directory)/d).is_symlink()]
  for name in names:
   p=Path(directory)/name
   if not name.startswith('.') and not p.is_symlink() and p.is_file() and p.suffix.lower() in ['.pdf','.png','.jpg','.jpeg','.heic','.tiff','.tif','.txt','.md']:paths.append(p)
 for p in sorted(paths):
  h=digest(p.read_bytes());before[p]=h;c=root/'.tuck/cache'/f'{h}.json'
  if not c.is_file() or c.is_symlink():continue
  v=json.loads(c.read_text());u=v['understanding']
  if v['contentHash']!=h or v['helperHash']!=identity or u['uncertainty'] or not u['evidence']:continue
  if p.suffix.lower() in ['.txt','.md']:text=p.read_bytes().decode('utf-8',errors='replace')[:14000]
  else:
   q=subprocess.run([str(READER),str(p)],capture_output=True,text=True,timeout=65)
   if q.returncode:raise RuntimeError('Cached source extraction failed')
   text=json.loads(q.stdout)['text']
  extraction_checks.append({'sameExtraction':digest(text.encode())==v['inputHash'],'quotesStillPresent':all(e['quote'] in text for e in u['evidence'])})
  samples.append(u['description']);cache_verified+=1
 assert len(samples)==22, f'Expected the same 22 descriptions, found {len(samples)}'
 CASES.append(('personal',samples))
assert not api('ps')['models'],'Another Ollama model is active; stop rather than displacing it'
tags=api('tags');assert set(MODELS)<=set(m['name'] for m in tags['models'])
dump('provenance.json',{'scope':'Category planning only. Frozen Apple descriptions. Identical prompt and inputs; unconstrained JSON followed by the production Swift validator and one format retry. No raw PDF/image input or file operations.','models':[m for m in tags['models'] if m['name'] in MODELS],'ollama':api('version'),'settings':OPTIONS,'think':False,'format':'none, same free-JSON approach as production Apple category planner','repetitions':3,'appleHelperHash':digest(APPLE.read_bytes()),'validatorSourceHash':digest((R/'src-tauri/swift/TuckRoleValidation.swift').read_bytes()),'cases':[{'id':name,'descriptions':len(samples),'characters':sum(map(len,samples)),'inputHash':digest(encoded(samples).encode()),**({'input':samples} if name=='controlled' else {})} for name,samples in CASES],'sourceHashVerifiedCacheEntries':cache_verified,'freshExtractionMatches':sum(x['sameExtraction'] for x in extraction_checks),'freshQuotesPresent':sum(x['quotesStillPresent'] for x in extraction_checks),'inputPolicy':'Reuse frozen descriptions previously validated by Tuck; source hashes and provider identity match. Fresh OCR can vary and is outside this category-only comparison.','sourceFiles':len(before),'hardware':subprocess.check_output(['sysctl','-n','machdep.cpu.brand_string'],text=True).strip(),'memoryBytes':int(subprocess.check_output(['sysctl','-n','hw.memsize'],text=True)),'os':subprocess.check_output(['sw_vers','-productVersion'],text=True).strip()})
rows=[];warms=[]
try:
 for model in ['Apple']+MODELS:
  start=time.monotonic()
  if model=='Apple':
   subprocess.run([str(APPLE)],input=encoded({'operation':'categories','samples':['An employment contract.']}),text=True,capture_output=True,timeout=90)
   warm={'model':model,'seconds':time.monotonic()-start,'note':'Neutral one-description category warmup; OS model residency not controlled'}
  else:
   assert not api('ps')['models'],'Another Ollama model became active'
   raw=api('chat',{'model':model,'messages':[{'role':'user','content':'Reply only OK.'}],'stream':False,'think':False,'keep_alive':'5m','options':{**OPTIONS,'num_predict':8}})
   warm={'model':model,'seconds':time.monotonic()-start,'loadSeconds':raw.get('load_duration',0)/1e9,'resident':api('ps')['models']}
  warms.append(warm);dump('warmups.json',warms);print('Warmup',model,round(warm['seconds'],2),flush=True)
  try:
   for repetition in range(1,4):
    for case,samples in CASES:
     start=time.monotonic();attempts=[];rules=None;error=None
     if model=='Apple':
      p=subprocess.run([str(APPLE)],input=encoded({'operation':'categories','samples':samples}),text=True,capture_output=True,timeout=90)
      try:
       value=json.loads(p.stdout)
       if value.get('error'):raise RuntimeError(value['error'])
       rules=validate(p.stdout)
      except Exception as e:error=str(e)
     else:
      for retry in [False,True]:
       try:
        raw=api('chat',{'model':model,'messages':[{'role':'system','content':INSTRUCTIONS+(RETRY if retry else '')},{'role':'user','content':encoded(samples)}],'stream':False,'think':False,'keep_alive':'5m','options':OPTIONS},timeout=max(1,90-(time.monotonic()-start)))
        attempts.append({k:raw.get(k) for k in ['load_duration','prompt_eval_count','prompt_eval_duration','eval_count','eval_duration','done_reason']})
        if raw.get('done_reason')=='length':raise ValueError('invalid_response')
        rules=validate(raw['message']['content']);error=None;break
       except ValueError as e:
        error=str(e)
        if retry:break
       except Exception as e:error=type(e).__name__;break
     row={'model':model,'case':case,'repetition':repetition,'seconds':time.monotonic()-start,'folders':[r['folder'] for r in rules] if rules else [],'error':error,'attemptMetrics':attempts,'inputHash':digest(encoded(samples).encode())}
     if case=='controlled':row['rules']=rules
     # Never save raw private descriptions or model-generated private rule text.
     rows.append(row);dump('results.json',rows);print(encoded({k:row[k] for k in ['model','case','repetition','seconds','folders','error']}),flush=True)
  finally:
   if model!='Apple':api('generate',{'model':model,'keep_alive':0})
 for path,h in before.items():assert path.is_file() and digest(path.read_bytes())==h,'Source changed during benchmark'
 summary=[]
 for model in ['Apple']+MODELS:
  for case,_ in CASES:
   rs=[r for r in rows if r['model']==model and r['case']==case]
   summary.append({'model':model,'case':case,'valid':sum(r['error'] is None for r in rs),'runs':len(rs),'medianSeconds':statistics.median(r['seconds'] for r in rs),'distinctPlans':len({tuple(r['folders']) for r in rs}),'folders':rs[0]['folders']})
 dump('summary.json',{'results':summary,'sourcesUnchanged':True,'ollamaModelsAfter':api('ps')['models']});print('SUMMARY',encoded(summary),flush=True)
finally:
 # No source documents or app settings are mutated by this runner.
 pass
