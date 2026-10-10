import json
from pathlib import Path
root=Path(__file__).resolve().parent
fixtures=json.loads((root/'fixtures.json').read_text());raw=json.loads((root/'results.json').read_text())
subjects={'Work':{'work','projects','project notes','meeting notes','notes'},'Invoices':{'invoices','bills','supplier invoices'},'Health':{'health','medical','medical records','clinical reports','medical reports','healthcare'},'Education':{'education','academic records','transcripts','academic transcripts'},'Warranties':{'warranties','warranty','product warranties'},'Travel':{'travel','bookings','reservations'}}
summary=dict(clearCorrect=0,clearTotal=0,reviewCorrect=0,reviewTotal=0,wrongAutomatic=0,proposalClearCorrect=0,proposalClearTotal=0,proposalReviewCorrect=0,proposalReviewTotal=0,depthViolations=0,namingFailures=0,guidedDetailCorrect=0,guidedDetailTotal=0)
rows=[]
for r in raw['runs']:
 assignments={a['id']:a['category'] for a in r['proposal'].get('assignments',[])}
 row=dict(depth=r['depth'],customized=r['customized'],failures=[])
 for i,(f,d) in enumerate(zip(fixtures,r['decisions'])):
  clear=d['expected'] is not None
  summary['clearTotal' if clear else 'reviewTotal']+=1
  summary['clearCorrect' if clear else 'reviewCorrect']+=d['correct']
  summary['wrongAutomatic']+=d['wrongAutomatic']
  if not d['correct']:row['failures'].append(d)
  p=assignments.get(f'd{i:04}')
  actual=p if p and p!='Needs review' else None
  summary['proposalClearTotal' if clear else 'proposalReviewTotal']+=1
  if clear:summary['proposalClearCorrect']+=bool(r['proposal']['ok'] and actual and actual.split('/')[0].lower() in subjects[f['subject']])
  else:summary['proposalReviewCorrect']+=r['proposal']['ok'] and actual is None
  if clear and r['customized'] and r['depth']>1 and f.get('entity'):
   summary['guidedDetailTotal']+=1
   summary['guidedDetailCorrect']+=bool(actual and len(actual.split('/'))>1 and f['entity'].lower() in actual.split('/')[1].lower())
  summary['depthViolations']+=bool(actual and len(actual.split('/'))>r['depth'])
  if r['customized'] and d['decision']['confident'] and d['decision']['title']:
   summary['namingFailures']+=not d['decision']['title'].lower().startswith('record')
 rows.append(row)
acceptance=dict(filingRecall=summary['clearCorrect']>=.95*summary['clearTotal'],proposalRecall=summary['proposalClearCorrect']>=.95*summary['proposalClearTotal'],noWrongAutomatic=summary['wrongAutomatic']==0,reviewSafety=summary['reviewCorrect']==summary['reviewTotal'],proposalReviewSafety=summary['proposalReviewCorrect']==summary['proposalReviewTotal'],depthLimits=summary['depthViolations']==0,naming=summary['namingFailures']==0,guidedDetails=summary['guidedDetailCorrect']>=.95*summary['guidedDetailTotal'])
# Missing requested fields cannot be replaced by a different observed field.
missing=[]
for run in raw['runs']:
 if not run['customized'] or run['depth']<2:continue
 assignments={a['id']:a['category'] for a in run['proposal'].get('assignments',[])}
 for i,fixture in enumerate(fixtures):
  if fixture['id']=='no-project':
   value=assignments.get(f'd{i:04}','')
   missing.append(bool(value and '/' not in value and value.lower() in subjects['Work']))
summary['missingGroupingCorrect']=sum(missing)
summary['missingGroupingTotal']=len(missing)
acceptance['missingGroupingFallback']=all(missing)
# A factual grouping value must not repeat as an extra detail level.
repeated_details=[]
for run in raw['runs']:
 for assignment in run.get('proposal',{}).get('assignments',[]):
  labels=assignment['category'].lower().split('/')[1:]
  if any(a==b for a,b in zip(labels,labels[1:])):repeated_details.append(assignment)
summary['repeatedDetailLabels']=len(repeated_details)
acceptance['noRepeatedDetailLabels']=not repeated_details
(root/'scores.json').write_text(json.dumps(dict(summary=summary,acceptance=acceptance,runs=rows),indent=2));print(json.dumps(summary,indent=2))
