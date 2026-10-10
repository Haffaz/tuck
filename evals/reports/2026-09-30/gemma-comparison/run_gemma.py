import hashlib,importlib.util,json,subprocess,time
from pathlib import Path
ROOT=Path(__file__).parent
BASE=Path('/private/tmp/tuck-four-page-eval')
spec=importlib.util.spec_from_file_location('baseline',BASE/'run.py'); baseline=importlib.util.module_from_spec(spec);spec.loader.exec_module(baseline)
MODELS=['gemma4:e2b-it-q4_K_M','gemma4:e4b-it-q4_K_M']
OUT=ROOT/'results';OUT.mkdir(exist_ok=True)
cases=json.loads((BASE/'manifest-private.json').read_text())
original=json.loads((BASE/'results/provenance.json').read_text())
assert hashlib.sha256((BASE/'manifest-private.json').read_bytes()).hexdigest()==original['manifestHash']
for case in cases:
 for p in case['pages']:
  assert hashlib.sha256(Path(p['image']).read_bytes()).hexdigest()==p['sha256']
def state():
 return {'models':baseline.api('/api/ps')['models'],'swap':subprocess.check_output(['sysctl','vm.swapusage'],text=True).strip()}
provenance={'baselineManifestHash':original['manifestHash'],'baselineScriptHash':hashlib.sha256((BASE/'run.py').read_bytes()).hexdigest(),'scriptHash':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'models':baseline.api('/api/tags'),'ollamaVersion':baseline.api('/api/version'),'settings':original['settings'],'rules':baseline.RULES,'types':baseline.TYPES,'started':time.strftime('%Y-%m-%d %H:%M:%S'),'os':subprocess.check_output(['sw_vers'],text=True),'hardware':{'memoryBytes':subprocess.check_output(['sysctl','-n','hw.memsize'],text=True).strip(),'cpu':subprocess.check_output(['sysctl','-n','machdep.cpu.brand_string'],text=True).strip()},'before':state(),'note':'Same frozen inputs, rules, schema and decoding as prior Qwen benchmark. Single-pass inference only; no file operations. Model allocation is API-reported, not whole-system peak RAM. Conditions rotate; caches may affect timing. Temperature 0 is for comparison and differs from Google default recommendation.'}
(ROOT/'provenance.json').write_text(json.dumps(provenance,indent=2))
for model in MODELS:
 assert not baseline.api('/api/ps')['models'], 'Do not displace another loaded model'
 info=baseline.api('/api/show',{'model':model})
 (ROOT/(model.replace(':','-')+'-info.json')).write_text(json.dumps({k:info.get(k) for k in ['details','capabilities','model_info','projector_info','parameters']},indent=2))
 warm=dict(instructions=baseline.INSTRUCTIONS,prompt='Classify document D1. Text: Unreadable. Use Needs Review and Unknown. No marker.',imagePaths=[],ids=['D1'],folders=list(baseline.RULES),types=baseline.TYPES)
 start=time.monotonic();response=baseline.call(model,warm)
 (ROOT/(model.replace(':','-')+'-cold-start.json')).write_text(json.dumps({'seconds':response['seconds'],'error':response.get('error'),'metrics':response.get('metrics'),'state':state()},indent=2))
 print('COLD',model,round(response['seconds'],2),response.get('error'),flush=True)
 try:
  if response.get('error'): raise RuntimeError('Neutral warmup failed: '+response['error'])
  for ci,case in enumerate(cases):
   for offset in range(4):
    mode=baseline.MODES[(ci+offset)%4]
    inp=baseline.make_input(case,mode)
    before=state(); response=baseline.call(model,inp); after=state()
    scores=baseline.score(case,response)
    # Do not save document text, filenames, freeform markers or raw model responses.
    clean={'model':model,'case':case['id'],'kind':case['kind'],'mode':mode,'seconds':response['seconds'],'imagesSent':response.get('imagesSent',len(inp['imagePaths'])),'error':response.get('error'),'metrics':response.get('metrics'),'scores':scores,'requestHash':hashlib.sha256(json.dumps(inp,sort_keys=True).encode()).hexdigest(),'runtimeBefore':before,'runtimeAfter':after}
    (OUT/f"{model.replace(':','-')}-{case['id']}-{mode}.json").write_text(json.dumps(clean,indent=2))
    print(model,case['id'],mode,round(clean['seconds'],2),'folders',sum(s['folderCorrect'] for s in scores),'/',len(scores),'types',sum(s['typeCorrect'] for s in scores),'error',clean['error'],flush=True)
 finally:
  baseline.api('/api/generate',{'model':model,'keep_alive':0})
print('COMPLETE',len(list(OUT.glob('*.json'))),flush=True)
(ROOT/'after.json').write_text(json.dumps(state(),indent=2))
