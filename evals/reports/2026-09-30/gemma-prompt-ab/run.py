from pathlib import Path
import base64,hashlib,json,time,urllib.request
ROOT=Path(__file__).parent
MODELS=['gemma4:e2b-it-q4_K_M','gemma4:e4b-it-q4_K_M']
SETTINGS={'temperature':0,'seed':42,'num_ctx':16384,'num_predict':700,'presence_penalty':0,'repeat_penalty':1}
SCHEMA={'type':'object','properties':{'documentType':{'type':'string'}},'required':['documentType'],'additionalProperties':False}
PROMPTS={k:(ROOT/(k+'-prompt.txt')).read_text().rstrip() for k in ['old','new']}
CASES=json.loads((ROOT/'manifest-private.json').read_text())
OUT=ROOT/'results';OUT.mkdir(mode=0o700,exist_ok=True)
def api(path,body=None,timeout=240):
 req=urllib.request.Request('http://127.0.0.1:11434/api/'+path,data=json.dumps(body).encode() if body is not None else None,headers={'Content-Type':'application/json'})
 with urllib.request.urlopen(req,timeout=timeout) as r:return json.load(r)
def call(model,variant,user):
 req={'model':model,'messages':[{'role':'system','content':PROMPTS[variant]},user],'format':SCHEMA,'stream':False,'think':False,'keep_alive':'15m','options':SETTINGS}
 start=time.monotonic()
 try:
  raw=api('chat',req)
  response={'result':json.loads(raw['message']['content']),'error':'output_limit' if raw.get('done_reason')=='length' else None,'metrics':{k:raw.get(k) for k in ['total_duration','load_duration','prompt_eval_count','prompt_eval_duration','eval_count','eval_duration','done_reason']}}
  if set(response['result'])!={'documentType'} or not isinstance(response['result']['documentType'],str):response['error']='schema_mismatch'
 except Exception as e:response={'error':type(e).__name__}
 response['seconds']=time.monotonic()-start
 return response
for c in CASES:
 assert hashlib.sha256(Path(c['text']).read_bytes()).hexdigest()==c['textHash']
 assert hashlib.sha256(Path(c['image']).read_bytes()).hexdigest()==c['imageHash']
provenance={'started':time.strftime('%Y-%m-%d %H:%M:%S'),'models':api('tags'),'version':api('version'),'settings':SETTINGS,'schema':SCHEMA,'prompts':PROMPTS,'manifestHash':hashlib.sha256((ROOT/'manifest-private.json').read_bytes()).hexdigest(),'rubricHash':hashlib.sha256((ROOT/'rubric.json').read_bytes()).hexdigest(),'scriptHash':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'note':'104 calls: 2 models x 13 documents x 2 input modes x 2 prompts. Only system prompt varies within each pair. One document per call; IDs are runner metadata. Both prompts rerun live. No retries for better answers. Fixed accuracy rubric; one pass per condition. Inference local.'}
(ROOT/'provenance.json').write_text(json.dumps(provenance,indent=2))
for mi,model in enumerate(MODELS):
 assert not api('ps')['models'], 'Do not displace another loaded model'
 try:
  for variant in ['old','new']:
   warm=call(model,variant,{'role':'user','content':'Unreadable document.\nReturn JSON: {"documentType": "..."}'})
   print('WARM',model,variant,round(warm['seconds'],2),warm.get('error'),flush=True)
   if warm.get('error'):raise RuntimeError('Warmup failed')
  for ci,case in enumerate(CASES):
   modes=['text','image'] if ci%2==0 else ['image','text']
   for mode in modes:
    if mode=='text':prompt='Document pages:\n'+Path(case['text']).read_text()
    else:
     prompt='Document pages.'
     if case['pageCount']>1:prompt+=' Pages are arranged top-left, top-right, bottom-left, bottom-right. Empty tiles are blank.'
    user={'role':'user','content':prompt+'\nReturn JSON: {"documentType": "..."}'}
    if mode=='image':user['images']=[base64.b64encode(Path(case['image']).read_bytes()).decode()]
    evidenceHash=hashlib.sha256(json.dumps(user,sort_keys=True).encode()).hexdigest()
    order=['old','new'] if (ci+mi+(mode=='image'))%2==0 else ['new','old']
    for pi,variant in enumerate(order):
     response=call(model,variant,user)
     response.update(model=model,variant=variant,key=case['key'],split=case['split'],kind=case['kind'],mode=mode,pairPosition=pi+1,evidenceHash=evidenceHash)
     path=OUT/f"{model.replace(':','-')}-{case['key']}-{mode}-{variant}.json";path.write_text(json.dumps(response,indent=2));path.chmod(0o600)
     print(model,case['key'],mode,variant,round(response['seconds'],2),'error',response.get('error'),flush=True)
 finally:api('generate',{'model':model,'keep_alive':0})
print('COMPLETE',len(list(OUT.glob('*.json'))),flush=True)
