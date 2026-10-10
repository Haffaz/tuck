from pathlib import Path
import json,hashlib,subprocess,time,base64
ROOT=Path(__file__).parent
PREVIOUS=Path('/private/tmp/tuck-prompt-ab')
CASES=json.loads((PREVIOUS/'manifest-private.json').read_text())
PROMPT=(ROOT/'prompt.txt').read_text().rstrip()
OUT=ROOT/'results';OUT.mkdir(mode=0o700,exist_ok=True)
def call(user,images):
    payload={'instructions':PROMPT,'prompt':user,'imagePaths':images}
    start=time.monotonic()
    try:
        p=subprocess.run([str(ROOT/'apple-recognition')],input=json.dumps(payload),text=True,capture_output=True,timeout=180)
        result=json.loads(p.stdout) if p.returncode==0 else {'error':'process_failed'}
        if not result.get('error') and (set(result.get('result',{}))!={'documentType'} or not isinstance(result['result']['documentType'],str)):result['error']='schema_mismatch'
    except subprocess.TimeoutExpired:result={'error':'timeout'}
    except Exception as e:result={'error':type(e).__name__}
    result['seconds']=time.monotonic()-start
    return result
check=json.loads(subprocess.check_output([str(ROOT/'apple-recognition'),'--check']))
print('AVAILABILITY',json.dumps(check),flush=True)
assert check['available'] and check['vision']
provenance={'model':'Apple SystemLanguageModel.default','availability':check,'os':subprocess.check_output(['sw_vers'],text=True),'sampling':'greedy','maximumResponseTokens':700,'prompt':PROMPT,'promptHash':hashlib.sha256(PROMPT.encode()).hexdigest(),'rubricHash':hashlib.sha256((ROOT/'rubric.json').read_bytes()).hexdigest(),'date':time.strftime('%Y-%m-%d %H:%M:%S'),'note':'26 new Apple calls. Prior Gemma new-prompt calls reused, rescored identically. No old-prompt comparison. Exact same evidence and prompt, native guided schema per provider. No translation, retries or production pipeline.'}
(ROOT/'provenance.json').write_text(json.dumps(provenance,indent=2))
warm=call('Unreadable document.\nReturn JSON: {"documentType": "..."}',[])
print('WARM',round(warm['seconds'],2),warm.get('error'),flush=True)
for ci,c in enumerate(CASES):
    for field in ['text','image']:assert hashlib.sha256(Path(c[field]).read_bytes()).hexdigest()==c[field+'Hash']
    for mode in (['text','image'] if ci%2==0 else ['image','text']):
        if mode=='text':user='Document pages:\n'+Path(c['text']).read_text()
        else:
            user='Document pages.'
            if c['pageCount']>1:user+=' Pages are arranged top-left, top-right, bottom-left, bottom-right. Empty tiles are blank.'
        user+='\nReturn JSON: {"documentType": "..."}'
        normalized={'role':'user','content':user}
        if mode=='image':normalized['images']=[base64.b64encode(Path(c['image']).read_bytes()).decode()]
        evidenceHash=hashlib.sha256(json.dumps(normalized,sort_keys=True).encode()).hexdigest()
        for old in (PREVIOUS/'results').glob('*-'+c['key']+'-'+mode+'-new.json'):
            assert json.loads(old.read_text())['evidenceHash']==evidenceHash
        r=call(user,[c['image']] if mode=='image' else [])
        r.update(model='Apple Intelligence',variant='new',key=c['key'],split=c['split'],kind=c['kind'],mode=mode,evidenceHash=evidenceHash)
        path=OUT/(c['key']+'-'+mode+'.json');path.write_text(json.dumps(r,indent=2));path.chmod(0o600)
        print(c['key'],mode,round(r['seconds'],2),'error',r.get('error'),flush=True)
print('COMPLETE',len(list(OUT.glob('*.json'))),flush=True)
