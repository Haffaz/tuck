"""Local protocol contract tests. Never contacts a cloud service or reads a saved API key."""
import json, subprocess, threading, hashlib, os
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
helper = Path(os.environ.get('TUCK_TEST_HELPER', Path(__file__).resolve().parents[1] / 'src-tauri/resources/tuck-apple'))
state = {'mode':'ok', 'requests':[]}
class Handler(BaseHTTPRequestHandler):
 def log_message(self,*args): pass
 def handle_request(self):
  body = json.loads(self.rfile.read(int(self.headers.get('content-length',0))) or '{}')
  state['requests'].append({'path':self.path,'body':body,'headers':{k.lower():v for k,v in self.headers.items()}})
  if state['mode']=='auth': self.send_response(401); self.end_headers(); return
  if state['mode']=='redirect':
   self.send_response(302); self.send_header('Location', f'http://127.0.0.1:{self.server.server_port}/redirected'); self.end_headers(); return
  if self.command == 'GET': result = {'models':[{'name':'arbitrary-model'}], 'data':[{'id':'arbitrary-model'}]}
  else:
   content = state.get('content','OK')
   truncated = state['mode']=='truncated'
   if self.path.endswith('/api/chat'): result = {'done':True,'done_reason':'length' if truncated else 'stop','message':{'content':content}}
   elif self.path.endswith('/v1/messages'): result = {'stop_reason':'max_tokens' if truncated else 'end_turn','content':[{'type':'text','text':content}]}
   else: result = {'choices':[{'finish_reason':'length' if truncated else 'stop','message':{'content':content}}]}
  payload = json.dumps(result).encode(); self.send_response(200); self.send_header('Content-Type','application/json'); self.send_header('Content-Length',str(len(payload))); self.end_headers(); self.wfile.write(payload)
 do_POST=handle_request
 do_GET=handle_request
server=ThreadingHTTPServer(('127.0.0.1',0),Handler)
threading.Thread(target=server.serve_forever,daemon=True).start()
checks=[]
def call(kind,op='status',**extra):
 c={'kind':kind,'baseUrl':f'http://127.0.0.1:{server.server_port}' + ('/v1' if kind=='openai' else ''),'model':'arbitrary-model'}
 req={'model':'custom','connection':c,'apiKey':'synthetic-test-key','operation':op,**extra}
 out=subprocess.run([str(helper)],input=json.dumps(req),text=True,capture_output=True,timeout=20)
 assert out.returncode==0, out.stderr
 return json.loads(out.stdout)
def last_prompt():
 return json.dumps(state['requests'][-1]['body'])
try:
 for kind,path in [('ollama','/api/chat'),('openai','/v1/chat/completions'),('anthropic','/v1/messages')]:
  state['mode']='ok'; state['content']='OK'
  assert 'ready' in call(kind)['status']
  req=state['requests'][-1]; assert req['path']==path
  assert req['body']['model']=='arbitrary-model'
  assert req['headers'].get('x-api-key' if kind=='anthropic' else 'authorization')==('synthetic-test-key' if kind=='anthropic' else 'Bearer synthetic-test-key')
  if kind=='anthropic': assert req['body']['system']=='Reply briefly.' and req['headers']['anthropic-version']=='2023-06-01'
  assert call(kind,'models')['models']==['arbitrary-model']
  checks.append(kind+' arbitrary model, headers, models')
  for mode in ['auth','truncated','redirect']:
   state['mode']=mode; before=len(state['requests']); assert 'error' in call(kind)
   assert len(state['requests'])==before+1
   checks.append(kind+' rejects '+mode)
  state['mode']='ok'

  state['content']=json.dumps({'kind':'Bank Statement','issuer':'Chase Bank','date':'2026-02','tags':['Banking','checking','statement'],'summary':'A checking account statement.'})
  card=call(kind,'card',fileName='a.txt',text='Chase Bank checking account statement February 2026')
  assert card['kind']=='bank statement' and card['date']=='2026-02' and card['tags']==['banking','checking','statement'], card
  assert 'Chase Bank checking' in last_prompt()
  state['content']=json.dumps({'kind':'','issuer':'','date':'','tags':[],'summary':''})
  assert 'error' in call(kind,'card',fileName='a.txt',text='Unreadable')
  checks.append(kind+' card output is cleaned and empty kinds are rejected')

  state['content']=json.dumps({'name':'Bank Statements'})
  assert call(kind,'name_group',cards=[{'kind':'bank statement','summary':'Statement'}],parent='Finance')['name']=='Bank Statements'
  assert 'Finance' in last_prompt()
  state['content']=json.dumps({'name':'Finance/Banking'})
  assert 'error' in call(kind,'name_group',cards=[{'kind':'bank statement','summary':'Statement'}],parent='')
  checks.append(kind+' group names reject paths')

  state['content']='Payslips -> Work\nBank Statements -> Finance\nInvented -> Other'
  assert call(kind,'group_subjects',names=['Payslips','Bank Statements'])['subjects']=={'Payslips':'Work','Bank Statements':'Finance'}
  checks.append(kind+' subject lines keep only listed folders')

  state['content']=json.dumps({'merges':[{'from':'Banking','into':'Bank Statements'},{'from':'Invented','into':'Banking'},{'from':'Payslips','into':'Payslips'}]})
  assert call(kind,'merge_names',names=['Banking','Bank Statements','Payslips'])['merges']==[{'from':'Banking','into':'Bank Statements'}]
  checks.append(kind+' merges keep only distinct listed names')

  candidates=[{'folder':'Finance/Banking','instruction':'Bank statements.','documentTypes':['bank statement']},{'folder':'Work','instruction':'Employment.'}]
  state['content']=json.dumps({'category':'Finance/Banking'})
  assert call(kind,'choose_folder',card={'kind':'bank statement','tags':[]},candidates=candidates,rootRules='')['folder']=='Finance/Banking'
  state['content']=json.dumps({'category':'Outside'})
  assert 'error' in call(kind,'choose_folder',card={'kind':'bank statement','tags':[]},candidates=candidates,rootRules='')
  checks.append(kind+' folder choice is limited to candidates')

  state['content']=json.dumps({'instruction':'Bank statements and account letters.'})
  assert call(kind,'rule',folder='Banking',context='')['instruction']=='Bank statements and account letters.'
  checks.append(kind+' rule suggestion')

  # A service refusal must propagate instead of being converted to a card.
  state['mode']='auth';before=len(state['requests'])
  assert 'error' in call(kind,'card',fileName='a.txt',text='Monthly salary payslip')
  assert len(state['requests'])==before+1
  checks.append(kind+' card authentication failure propagates')
 # Exercise actual Keychain persistence with a disposable loopback endpoint.
 state['mode']='ok';state['content']='OK'
 base=f'http://127.0.0.1:{server.server_port}/keychain-test'
 connection={'kind':'openai','baseUrl':base,'model':'arbitrary-model'}
 account=hashlib.sha256(('openai|'+base).encode()).hexdigest()
 try:
  req={'operation':'save_key','model':'custom','connection':connection,'apiKey':'synthetic-test-key'}
  saved=subprocess.run([str(helper)],input=json.dumps(req),capture_output=True,text=True,timeout=20)
  assert 'status' in json.loads(saved.stdout)
  req={'operation':'status','model':'custom','connection':connection}
  response=subprocess.run([str(helper)],input=json.dumps(req),capture_output=True,text=True,timeout=20)
  assert 'status' in json.loads(response.stdout)
  assert state['requests'][-1]['headers']['authorization']=='Bearer synthetic-test-key'
  checks.append('Keychain save and runtime lookup without sending the key again')
 finally:
  removed=subprocess.run(['/usr/bin/security','delete-generic-password','-s','app.tuck.model-connections','-a',account],capture_output=True,text=True)
  assert removed.returncode==0, 'Could not remove synthetic Keychain item'
 print(json.dumps({'passed':len(checks),'checks':checks},indent=2))
finally: server.shutdown()
