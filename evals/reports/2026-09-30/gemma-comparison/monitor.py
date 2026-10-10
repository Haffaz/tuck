import json,time,subprocess,urllib.request
from pathlib import Path
r=Path(__file__).parent
with (r/'memory-samples.jsonl').open('a') as out:
 while True:
  try:
   with urllib.request.urlopen('http://127.0.0.1:11434/api/ps',timeout=5) as response: models=json.load(response)['models']
   processes=[]
   for line in subprocess.check_output(['ps','-axo','pid=,ppid=,rss='],text=True).splitlines():
    pid,parent,rss=map(int,line.split())
    if pid==14038 or parent==14038: processes.append({'pid':pid,'parent':parent,'rssKiB':rss})
   out.write(json.dumps({'time':time.time(),'models':[m['name'] for m in models],'processes':processes})+'\n');out.flush()
  except Exception as e: out.write(json.dumps({'error':type(e).__name__})+'\n');out.flush()
  if 'COMPLETE 64' in (r/'progress.log').read_text():break
  time.sleep(2)
