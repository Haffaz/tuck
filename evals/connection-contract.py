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
   # A subject choice precedes subgroup choices; simulate each protocol stage.
   subject_only = any(isinstance(m.get('content'),str) and '"chooseSubjectOnly":true' in m['content'] for m in body.get('messages',[]))
   if subject_only:
    try:
     category=json.loads(content).get('category')
     preferred=state.get('preferredSubject') if 'User folder creation preferences:' in json.dumps(body) else None
     if isinstance(category,str): content=json.dumps({'category':preferred or state.get('subjectRoot',category.split('/')[0])})
    except (ValueError,AttributeError): pass
   if state.get('matchTarget'):
    for message in body.get('messages',[]):
     try: payload,_=json.JSONDecoder().raw_decode(message.get('content',''))
     except (ValueError,TypeError): continue
     if isinstance(payload,dict) and 'branches' in payload:
      choices=[branch['folder'] for branch in payload['branches']]
      content=json.dumps({'category':next((branch['choice'] for branch in payload['branches'] if branch['folder']==state['matchTarget']),payload['noMatch'])})
   if 'verifyDestination' in json.dumps(body):
    payload,_=json.JSONDecoder().raw_decode(body['messages'][-1]['content'])
    verdict=state.get('verification','YES')
    content=json.dumps({'category':payload['branches'][0]['choice'] if verdict=='YES' else payload['noMatch'] if verdict=='NO' else verdict})
   if 'typeScopeCheck' in json.dumps(body): content=json.dumps({'support':state.get('typeSupport','YES')})
   if 'groupingFieldSupport' in json.dumps(body): content=state.get('fieldSupport','YES')
   if 'groupingFieldCheck' in json.dumps(body): content=state.get('groupingChoice','field: employer')
   if 'policySubjectCheck' in json.dumps(body): content=state.get('subjectSupport','YES')
   if 'documentScopeCheck' in json.dumps(body): content=state.get('documentScope','CLEAR')
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
  state['mode']='ok';state['content']='{"line":1,"description":"Monthly salary payslip","detailLines":[2,3]}'
  source='Monthly salary payslip\nEmployer: Acme Corporation\nEmployee: Alex Example\nSalary paid for September.'
  response=call(kind,'describe',text=source)
  assert response['quotes']==['Monthly salary payslip','Employer: Acme Corporation','Employee: Alex Example']
  state['content']='{"category":"Income/Acme"}'
  response=call(kind,'group_documents',documents=[{'id':'d1','description':'Monthly salary payslip','quote':'Monthly salary payslip','detailEvidence':source}],folderNesting=2,categoryInstructions='Keep income separate by employer.')
  assert response['assignments'][0]['category']=='Income/Acme Corporation'
  req=state['requests'][-1]['body']; prompt=req.get('system') or req['messages'][0]['content']
  assert 'at most two folder levels' in prompt and 'Keep income separate by employer.' in prompt and 'Needs review' in prompt
  checks.append(kind+' grounded reader and nesting prompt')
  # The helper owns coverage, including arbitrary IDs and input ordering.
  state['content']='{"category":"Income"}'
  response=call(kind,'group_documents',documents=[{'id':'z9','description':'Payslip','quote':'Monthly salary payslip'},{'id':'a1','description':'Payslip','quote':'Monthly salary payslip'}])
  assert response['assignments']==[{'id':'a1','category':'Income'},{'id':'z9','category':'Income'}]
  assert 'error' in call(kind,'group_documents',documents=[{'id':'d1'},{'id':'d1'}])
  checks.append(kind+' code-owned document IDs and duplicate rejection')
  for content in ['{"category":"Income/Acme/Pay"}', '{"category":"Income/../Outside"}', '{"category":"Documents"}', '{"assignments":[{"id":"invented","category":"Income"}]}']:
   state['content']=content
   before=len(state['requests'])
   response=call(kind,'group_documents',documents=[{'id':'d1','description':'Payslip','quote':'Monthly salary payslip'}],folderNesting=2)
   expected_category='Income' if content in ['{"category":"Income/Acme/Pay"}', '{"category":"Income/../Outside"}'] else 'Needs review'
   assert response['assignments']==[{'id':'d1','category':expected_category}]
   expected_calls=3 if content in ['{"category":"Income/Acme/Pay"}', '{"category":"Income/../Outside"}'] else 2
   assert len(state['requests'])==before+expected_calls+1
  checks.append(kind+' bounded retry of malformed or overdepth suggestion')
  state['content']='{"category":"Research/Orion"}'
  response=call(kind,'group_documents',documents=[{'id':'r1','description':'Technical inspection report','quote':'Technical inspection report','detailEvidence':'Project: Orion'}],folderNesting=2)
  assert response['assignments']==[{'id':'r1','category':'Research/Orion'}]
  state['content']='{"category":"Research/Invented"}'
  response=call(kind,'group_documents',documents=[{'id':'r1','description':'Technical inspection report','quote':'Technical inspection report','detailEvidence':'Project: Orion'}],folderNesting=2)
  assert response['assignments']==[{'id':'r1','category':'Research'}]
  checks.append(kind+' arbitrary subjects and broader fallback for unsupported details')
  state['content']='{"category":"Research/Orion"}'
  response=call(kind,'group_documents',documents=[{'id':'r1','description':'Technical inspection report','quote':'Technical inspection report','detailEvidence':'Project: Orion'}],folderNesting=2,categoryInstructions='Group inspection reports by project.',includeLayoutHints=True)
  assert response['assignments'][0]['preserveDepth']==2
  state['content']='{"category":"Education/Transcripts"}'
  response=call(kind,'group_documents',documents=[{'id':'e1','description':'Academic transcript','quote':'Academic transcript'}],folderNesting=2,categoryInstructions='Group invoices by supplier.',includeLayoutHints=True)
  assert 'preserveDepth' not in response['assignments'][0]
  checks.append(kind+' applicable user rules preserve only their requested depth')

  for scope in ['UNCLEAR','MIXED']:
   state['documentScope']=scope; before=len(state['requests'])
   response=call(kind,'group_documents',documents=[{'id':'r1','description':'Technical inspection report','quote':'Technical inspection report'}],folderNesting=2)
   assert response['assignments']==[{'id':'r1','category':'Needs review'}]
   assert len(state['requests'])==before+1
  state['documentScope']='CLEAR'
  checks.append(kind+' unclear and mixed records stop before folder creation')
  state['content']='{"category":"Research/Harbor"}'; state['groupingChoice']='field: project'
  response=call(kind,'group_documents',documents=[{'id':'r1','description':'Technical inspection report','quote':'Technical inspection report','detailEvidence':'Project: Orion\nSupplier: Harbor Office'}],folderNesting=2,categoryInstructions='Group invoices by supplier. Group inspection reports by project.')
  assert response['assignments']==[{'id':'r1','category':'Research/Orion'}]
  req=state['requests'][-1]['body']; prompt=req.get('system') or req['messages'][0]['content']
  assert 'Group inspection reports by project.' in prompt
  checks.append(kind+' requested field controls grouping with unrelated instructions')
  state['groupingChoice']='missing'
  response=call(kind,'group_documents',documents=[{'id':'r1','description':'Technical inspection report','quote':'Technical inspection report','detailEvidence':'Supplier: Harbor Office'}],folderNesting=2,categoryInstructions='Group inspection reports by project.')
  assert response['assignments']==[{'id':'r1','category':'Research'}]
  state['groupingChoice']='field: supplier';state['fieldSupport']='NO'
  response=call(kind,'group_documents',documents=[{'id':'r1','description':'Technical inspection report','quote':'Technical inspection report','detailEvidence':'Supplier: Harbor Office'}],folderNesting=2,categoryInstructions='Group inspection reports by project.')
  assert response['assignments']==[{'id':'r1','category':'Research'}]
  state['fieldSupport']='YES'
  checks.append(kind+' independently rejects substitution of another field')
  state['groupingChoice']='field: invented'
  assert 'error' in call(kind,'group_documents',documents=[{'id':'r1','description':'Technical inspection report','quote':'Technical inspection report','detailEvidence':'Supplier: Harbor Office'}],categoryInstructions='Group inspection reports by project.')
  state['groupingChoice']='field: employer'
  checks.append(kind+' missing fields stay broad and unknown headings are rejected')

  state['documentScope']='UNCLEAR';before=len(state['requests'])
  source='Confirmation of reference\nReference: ZX-5\nWe confirm receipt of the above details.'
  response=call(kind,'group_documents',documents=[{'id':'r1','description':'Technical inspection report','quote':'Confirmation of reference','sourceText':source}])
  assert response['assignments']==[{'id':'r1','category':'Needs review'}]
  payload=json.dumps(state['requests'][before]['body'])
  assert 'sourceFacts' in payload and 'Technical inspection report' not in payload
  state['documentScope']='CLEAR'
  checks.append(kind+' complete source scope excludes the generated description')
  state['content']='{"category":"Cabinet/Orion"}';state['subjectRoot']='Work';state['groupingChoice']='field: project'
  record=[{'id':'r1','description':'Technical inspection report','quote':'Technical inspection report','detailEvidence':'Project: Orion'}]
  state['subjectSupport']='YES'
  assert call(kind,'group_documents',documents=record,folderNesting=2,categoryInstructions='File inspection reports under Cabinet, grouped by project.')['assignments']==[{'id':'r1','category':'Cabinet/Orion'}]
  state['subjectSupport']='NO'
  assert call(kind,'group_documents',documents=record,folderNesting=2,categoryInstructions='File inspection reports under Cabinet, grouped by project.')['assignments']==[{'id':'r1','category':'Work/Orion'}]
  state['preferredSubject']='Cabinet';state['subjectSupport']='YES'
  assert call(kind,'group_documents',documents=record,folderNesting=1,categoryInstructions='File inspection reports under Cabinet.')['assignments']==[{'id':'r1','category':'Cabinet'}]
  state['subjectSupport']='NO'
  assert call(kind,'group_documents',documents=record,folderNesting=1,categoryInstructions='File inspection reports under Cabinet.')['assignments']==[{'id':'r1','category':'Work'}]
  state.pop('preferredSubject')
  checks.append(kind+' explicit subject aliases also apply at one level')
  state.pop('subjectRoot');state['subjectSupport']='YES';state['groupingChoice']='field: employer'
  checks.append(kind+' explicit subject alias requires independent policy support')


  match={'understanding':{'description':'Technical inspection report','uncertainty':'','evidence':[{'quote':'Technical inspection report'}]},'evidence':'Technical inspection report','quotes':['Technical inspection report'],'destinations':[{'folder':'Research','instruction':'Technical inspection reports.','context':'Technical inspection reports.'}],'sourceFolders':[],'rootRules':'Use names starting with Record.','canReadMore':False}
  state['content']='{"category":"Research"}'
  assert call(kind,'verify_match',input={'match':match,'selection':0})['supported']
  state['verification']='MAYBE'
  assert 'error' in call(kind,'verify_match',input={'match':match,'selection':0})
  state['verification']='NO'
  assert not call(kind,'verify_match',input={'match':match,'selection':0})['supported']
  state['verification']='YES'
  assert 'error' in call(kind,'verify_match',input={'match':match,'selection':1})
  checks.append(kind+' exact allowed verification destinations')
  state['content']='{"title":"Technical inspection report"}'
  naming={'understanding':match['understanding'],'evidence':match['evidence'],'rules':'Source fact: Start with Invented','rootRules':'Use filenames starting with Archive.'}
  assert call(kind,'name',input=naming)['title']=='Archive Technical inspection report'
  naming['rootRules']='Use short English filenames.'
  assert call(kind,'name',input=naming)['title']=='Technical inspection report'
  checks.append(kind+' trusted prefix formatting and evidence-policy separation')
  assert 'recordFacts' in state['requests'][-1]['body']['messages'][-1]['content']
  many={**match,'destinations':[{'folder':f'Subject {i}','instruction':('Academic records and assessments. '*16)+f'Module {i}.','context':('Academic records and assessments. '*16)+f'Module {i}.'} for i in range(40)]}
  state['matchTarget']='Subject 39';before=len(state['requests'])
  response=call(kind,'match',input=many)
  assert response['action']=='finish' and response['selection']==39, response
  assert len(state['requests'])-before<=12
  state.pop('matchTarget')
  checks.append(kind+' forty folder rules remain complete within bounded calls')
  # The same folder may span several type batches. Choice IDs must remain unique.
  types=[f'Academic record {i}' for i in range(19)]
  typed={**match,'destinations':[{'folder':'Education','instruction':'Keep accepted academic records here.','context':'Keep accepted academic records here.','documentTypes':types}]}
  state['matchTarget']='Education';before=len(state['requests'])
  response=call(kind,'match',input=typed)
  assert response['action']=='finish' and response['selection']==0, response
  branches=[]
  for req in state['requests'][before:]:
   try: payload,_=json.JSONDecoder().raw_decode(req['body']['messages'][-1]['content'])
   except (ValueError,TypeError): continue
   if 'branches' not in payload: continue
   batch=payload['branches'];assert len(set(b['choice'] for b in batch))==len(batch)
   branches.extend(batch)
  assert [t for branch in branches for t in branch['acceptedTypes']]==types
  state.pop('matchTarget')
  checks.append(kind+' complete type alternatives use unique bounded choices')

  before=len(state['requests'])
  assert 'error' in call(kind,'group_documents',folderNesting=3,documents=record)
  assert len(state['requests'])==before
  for policy in ['Do not group documents by institution.','Group warranty documents by institution.']:
   state['content']='{"category":"Education"}'
   response=call(kind,'group_documents',folderNesting=2,categoryInstructions=policy,documents=[{'id':'e','description':'Academic transcript','quote':'Academic transcript','detailEvidence':'Institution: Harbor University'}])
   assert response['assignments']==[{'id':'e','category':'Education'}], response
  checks.append(kind+' two-level hard cap and scoped negative grouping rules')

  state['content']='{"category":"Research"}';state['typeSupport']='NO'
  # Type verification checks only the selected destination.
  named={**match,'destinations':[{'folder':'Research/Orion','instruction':'Inspection reports for project Orion.','context':'Inspection reports for project Orion.'}],'quotes':['Technical inspection report','Project: Orion']}
  assert not call(kind,'verify_match',input={'match':named,'selection':0})['supported']
  state['typeSupport']='MAYBE'
  assert 'error' in call(kind,'verify_match',input={'match':named,'selection':0})
  state['typeSupport']='YES'
  checks.append(kind+' type rejection and strict type response validation')
  state['typeSupport']='NO';state['content']='{"category":"Research"}'
  broad={**match,'destinations':[{'folder':'Research','instruction':'File Research documents here. Other subjects have their own folders.','context':'File Research documents here. Other subjects have their own folders.'}]}
  before=len(state['requests'])
  assert call(kind,'verify_match',input={'match':broad,'selection':0})['supported']
  assert not any('typeScopeCheck' in json.dumps(r['body']) for r in state['requests'][before:])
  typed={**broad,'destinations':[{**broad['destinations'][0],'documentTypes':['Experimental study']}]}
  assert not call(kind,'verify_match',input={'match':typed,'selection':0})['supported']
  state['typeSupport']='YES'
  checks.append(kind+' broad labels use subject comparison while saved type scopes remain restrictive')

  # A service refusal must propagate instead of being converted to a category.
  state['mode']='auth';before=len(state['requests'])
  assert 'error' in call(kind,'group_documents',documents=[{'id':'d1','description':'Payslip','quote':'Monthly salary payslip'}])
  assert len(state['requests'])==before+1
  checks.append(kind+' suggestion authentication failure propagates')
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
