import os,json,pathlib,subprocess,time
r=pathlib.Path('/private/tmp/tuck-generic-production')
repo=pathlib.Path('/Users/mohomedhaffaz/Projects/tuck')
binaries=[p for p in (repo/'src-tauri/target/debug/deps').glob('tuck_lib-*') if p.is_file() and os.access(p,os.X_OK) and '.' not in p.name]
binary=max(binaries,key=lambda p:p.stat().st_mtime)
f=json.loads((r/'fixtures.json').read_text()); rows=[]
for c in f['cases']:
 fixture=r/'current-fixture.json';fixture.write_text(json.dumps({'description':f['description'],'cases':[c]}))
 report=r/'results'/f"{c['id']}.json"
 env=dict(os.environ,TUCK_EVAL_FIXTURES=str(fixture),TUCK_EVAL_READER=str(r/'tuck-reader'),TUCK_EVAL_REPORT=str(report),TUCK_EVAL_LABEL='serial-roles-v4-category-enum',TUCK_TRACE_CASE=c['id'])
 p=subprocess.run([str(binary),'--ignored','--exact','evaluation::tests::live_apple_accuracy_eval','--nocapture'],cwd=repo,env=env,capture_output=True,text=True)
 (r/'results'/f"{c['id']}.log").write_text(p.stdout+p.stderr)
 if report.exists():
  row=json.loads(report.read_text())['results'][0];rows.append(row)
  print(json.dumps({'id':c['id'],'folder':row['decision']['folder'],'title':row['decision']['title'],'seconds':round(row['seconds'],2),'reason':row['decision']['reason']}),flush=True)
 else: print(json.dumps({'id':c['id'],'test_failed':p.returncode}),flush=True)
 (r/'results.json').write_text(json.dumps({'scope':f['description'],'results':rows},indent=2,ensure_ascii=False))
