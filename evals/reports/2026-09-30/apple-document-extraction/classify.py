from pathlib import Path
import json,subprocess,time,hashlib,urllib.request,urllib.error,os,signal,socket
ROOT=Path(__file__).parent;OUT=ROOT/'results';OUT.mkdir(mode=0o700,exist_ok=True)
CASES=json.loads((ROOT/'manifest-private.json').read_text());PROMPT=(ROOT/'prompt.txt').read_text().rstrip()
SETTINGS={'temperature':0,'seed':42,'num_ctx':16384,'num_predict':700,'presence_penalty':0,'repeat_penalty':1}
SCHEMA={'type':'object','properties':{'documentType':{'type':'string'}},'required':['documentType'],'additionalProperties':False}
APPLE='/private/tmp/tuck-apple-new-prompt/apple-recognition'
def api(path,data=None):
 req=urllib.request.Request('http://127.0.0.1:11434/api/'+path,data=json.dumps(data).encode() if data is not None else None,headers={'Content-Type':'application/json'})
 with urllib.request.urlopen(req,timeout=240) as r:return json.load(r)
def call(model,user):
 start=time.monotonic()
 try:
  if model=='Apple Intelligence':
   p=subprocess.run([APPLE],input=json.dumps({'instructions':PROMPT,'prompt':user,'imagePaths':[]}),text=True,capture_output=True,timeout=180,check=True)
   r=json.loads(p.stdout)
  else:
   raw=api('chat',{'model':model,'messages':[{'role':'system','content':PROMPT},{'role':'user','content':user}],'format':SCHEMA,'stream':False,'think':False,'keep_alive':'15m','options':SETTINGS})
   r={'result':json.loads(raw['message']['content']),'error':'output_limit' if raw.get('done_reason')=='length' else None,'metrics':{k:raw.get(k) for k in ['total_duration','load_duration','prompt_eval_count','prompt_eval_duration','eval_count','eval_duration']}}
  if not r.get('error') and (set(r.get('result',{}))!={'documentType'} or not isinstance(r['result']['documentType'],str)):r['error']='schema_mismatch'
 except Exception as e:r={'error':type(e).__name__}
 r['seconds']=time.monotonic()-start
 return r
provenance={'date':time.strftime('%Y-%m-%d %H:%M:%S'),'promptHash':hashlib.sha256(PROMPT.encode()).hexdigest(),'prompt':PROMPT,'settingsGemma':SETTINGS,'schema':SCHEMA,'appleSettings':{'model':'SystemLanguageModel.default','sampling':'greedy','maxResponseTokens':700},'rubricHash':hashlib.sha256((ROOT/'rubric.json').read_bytes()).hexdigest(),'note':'39 new text classification calls using structured Apple document extraction. Previous 39 new-prompt plain text calls are historical comparators, not rerun. Same 13 source documents/pages, same prompt/schema and relaxed acceptance criteria. No translation or retry. Rendering/extraction measured separately.'}
for model in ['Apple Intelligence','gemma4:e2b-it-q4_K_M','gemma4:e4b-it-q4_K_M']:
 if model=='gemma4:e2b-it-q4_K_M':
  # Own the server lifecycle; never displace an existing listener.
  with socket.socket() as sock:
   assert sock.connect_ex(('127.0.0.1',11434))!=0,'Existing Ollama server; stop without changing it'
  log=(ROOT/'server.log').open('w');server=subprocess.Popen(['/usr/local/bin/ollama','serve'],stdout=log,stderr=log,env={**os.environ,'OLLAMA_HOST':'127.0.0.1:11434','OLLAMA_NO_CLOUD':'true','OLLAMA_NUM_PARALLEL':'1'})
  (ROOT/'server.pid').write_text(str(server.pid))
  for _ in range(100):
   try:provenance['ollamaVersion']=api('version');break
   except Exception:time.sleep(.1)
  else:server.terminate();raise RuntimeError('Server unavailable')
  provenance['models']=api('tags')
 (ROOT/'provenance.json').write_text(json.dumps(provenance,indent=2))
 try:
  if model!='Apple Intelligence':assert not api('ps')['models']
  warm=call(model,'Unreadable document.\nReturn JSON: {"documentType": "..."}');print('WARM',model,round(warm['seconds'],2),warm.get('error'),flush=True)
  for c in CASES:
   path=ROOT/'inputs'/c['key']/'text.txt'
   user='Document pages:\n'+path.read_text()+'\nReturn JSON: {"documentType": "..."}'
   r=call(model,user);r.update(model=model,variant='structured',mode='text',key=c['key'],split=c['split'],kind=c['kind'],evidenceHash=hashlib.sha256(user.encode()).hexdigest())
   dest=OUT/(model.replace(':','-').replace(' ','-')+'-'+c['key']+'.json');dest.write_text(json.dumps(r,indent=2));dest.chmod(0o600)
   print(model,c['key'],round(r['seconds'],2),'error',r.get('error'),flush=True)
 finally:
  if model!='Apple Intelligence':api('generate',{'model':model,'keep_alive':0})
  if model=='gemma4:e4b-it-q4_K_M':
   assert not api('ps')['models'];server.send_signal(signal.SIGINT);server.wait(timeout=30);log.close()
print('COMPLETE',len(list(OUT.glob('*.json'))),flush=True)
