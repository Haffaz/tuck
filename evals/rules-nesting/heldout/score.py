"""Score the frozen matrix without changing prompts or acceptance criteria."""
import csv, json, re, statistics
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parent
fixtures = json.loads((ROOT / 'fixtures.json').read_text())
raw = json.loads((ROOT / 'results.json').read_text())
subjects = {
    'Income': {'income', 'salary', 'salaries', 'pay', 'payslips', 'salary statements'},
    'Employment': {'employment', 'work', 'employment contracts', 'contracts'},
    'Housing': {'housing', 'home', 'tenancy', 'rental', 'rent'},
    'Insurance': {'insurance', 'insurance policies'},
}

def depth(path):
    return len(path.split('/')) if path and path.lower() not in {'needs review', 'needsreview'} else 0

def subgroup_supported(path, fixture):
    if not path or depth(path) <= 1:
        return True
    parts = path.lower().split('/')[1:]
    if fixture['id'] == 'missing-employer':
        return not any(w in path.lower() for w in ['cedar', 'northstar', 'gamma', 'example'])
    employer = fixture.get('employer')
    if employer:
        return not any(other in part for part in parts for other in {'cedar', 'northstar', 'gamma'} - {employer.lower()})
    return not any(w in path.lower() for w in ['cedar', 'northstar', 'gamma'])

def proposal_scores(run):
    proposal = run['proposal']
    assignments = {a['id']: a['category'] for a in proposal.get('assignments', [])}
    rows = []
    for i, fixture in enumerate(fixtures):
        path = assignments.get(f'd{i:04}')
        actual = path if depth(path) else None
        expected = fixture['subject']
        correct = (actual is None) if expected is None else bool(actual and actual.split('/')[0].lower() in subjects[expected])
        # A rejected proposal supplies no usable choices; it is not successful abstention.
        if not proposal['ok']:
            correct = False
        rows.append(dict(id=fixture['id'], expected=expected, actual=actual,
                         correct=correct, clear=expected is not None,
                         depth=depth(actual), supportedSubgroup=subgroup_supported(actual, fixture)))
    clear = [r for r in rows if r['clear']]
    review = [r for r in rows if not r['clear']]
    guided = [r for r in rows if r['id'] in {'cedar-may','cedar-june','northstar-salary','cedar-agreement'}]
    employer = {}
    if run['creationPreferences'] and run['depth'] >= 2:
        employer = dict(correct=sum(bool(r['actual'] and len(r['actual'].split('/'))>1 and (('northstar' if r['id']=='northstar-salary' else 'cedar') in r['actual'].lower().split('/')[1])) for r in guided), total=len(guided))
    return dict(ok=proposal['ok'], clearCorrect=sum(r['correct'] for r in clear), clearTotal=len(clear),
                reviewCorrect=sum(r['correct'] for r in review), reviewTotal=len(review),
                maxDepth=max((r['depth'] for r in rows), default=0),
                withinLimit=all(r['depth'] <= run['depth'] for r in rows),
                unsupportedSubgroups=[r for r in rows if not r['supportedSubgroup']],
                guidedEmployer=employer, rows=rows)

rows = []
for run in raw['runs']:
    clear = [d for d in run['decisions'] if d['expected'] is not None]
    unclear = [d for d in run['decisions'] if d['expected'] is None]
    named = [d for d in clear if d['decision']['confident'] and d['decision']['title']]
    title_bad = [d for d in named if ('annual bonus' in d['decision']['title'].lower()) or (run['documentInstructions'] and (not d['decision']['title'].lower().startswith('record') or bool(re.search(r'\d|march|may|june',d['decision']['title'], re.I))))]
    row = dict(id=run['id'], depth=run['depth'], documentInstructions=run['documentInstructions'], creationPreferences=run['creationPreferences'], folderRules=run['folderRules'], repeat=run['repeat'],
               proposal=proposal_scores(run), filingCorrect=sum(d['correct'] for d in run['decisions']), clearCorrect=sum(d['correct'] for d in clear), clearTotal=len(clear), reviewCorrect=sum(d['correct'] for d in unclear), reviewTotal=len(unclear),
               wrongAutomatic=sum(d['wrongAutomatic'] for d in run['decisions']), named=len(named), prefixChecks=len(named) if run['documentInstructions'] else 0,
               namingFailures=len(title_bad), keptOriginal=sum(d['decision']['confident'] and not d['decision']['title'] for d in clear),
               medianSeconds=statistics.median(d['seconds'] for d in run['decisions']), seconds=run['seconds'])
    rows.append(row)

summary = dict(runs=len(rows), filingDecisions=sum(len(r['decisions']) for r in raw['runs']),
               clearCorrect=sum(r['clearCorrect'] for r in rows), clearTotal=sum(r['clearTotal'] for r in rows),
               reviewCorrect=sum(r['reviewCorrect'] for r in rows), reviewTotal=sum(r['reviewTotal'] for r in rows),
               wrongAutomatic=sum(r['wrongAutomatic'] for r in rows),
               acceptedProposals=sum(r['proposal']['ok'] for r in rows), rejectedProposals=sum(not r['proposal']['ok'] for r in rows),
               proposalClearCorrect=sum(r['proposal']['clearCorrect'] for r in rows), proposalClearTotal=sum(r['proposal']['clearTotal'] for r in rows),
               acceptedProposalClearCorrect=sum(r['proposal']['clearCorrect'] for r in rows if r['proposal']['ok']), acceptedProposalClearTotal=sum(r['proposal']['clearTotal'] for r in rows if r['proposal']['ok']),
               depthViolations=sum(not r['proposal']['withinLimit'] for r in rows), unsupportedSubgroups=sum(len(r['proposal']['unsupportedSubgroups']) for r in rows),
               prefixChecks=sum(r['prefixChecks'] for r in rows), namingFailures=sum(r['namingFailures'] for r in rows), keptOriginal=sum(r['keptOriginal'] for r in rows),
               seconds=raw.get('seconds'), completed=raw.get('completed',False))
summary['guidedEmployerCorrect'] = sum(r['proposal']['guidedEmployer'].get('correct',0) for r in rows)
summary['guidedEmployerTotal'] = sum(r['proposal']['guidedEmployer'].get('total',0) for r in rows)
summary['wrongSubjectAutomatic'] = sum(d['wrongAutomatic'] and (d['expected'] is None or d['decision']['folder'].split('/')[0] != d['expected'].split('/')[0]) for r in raw['runs'] for d in r['decisions'])
summary['wrongBroadAutomatic'] = sum(d['wrongAutomatic'] and d['expected'] is not None and d['decision']['folder'].split('/')[0] == d['expected'].split('/')[0] and depth(d['decision']['folder']) < depth(d['expected']) for r in raw['runs'] for d in r['decisions'])

repeat_changes = []
for first in raw['runs']:
    if first['repeat'] != 1: continue
    second = next((r for r in raw['runs'] if r['repeat']==2 and r['depth']==first['depth'] and r['documentInstructions']==first['documentInstructions'] and r['creationPreferences']==first['creationPreferences'] and r['folderRules']==first['folderRules']),None)
    if not second: continue
    changed = [a['id'] for a,b in zip(first['decisions'],second['decisions']) if a['decision']['folder'] != b['decision']['folder'] or a['decision']['confident'] != b['decision']['confident']]
    repeat_changes.append(dict(first=first['id'], second=second['id'], filingDestinationChanges=changed, proposalChanged=first['proposal'].get('assignments')!=second['proposal'].get('assignments') or first['proposal']['ok']!=second['proposal']['ok']))

thresholds = dict(depthLimits=summary['depthViolations']==0, filingRecall=summary['clearCorrect'] >= .95*summary['clearTotal'], noWrongAutomatic=summary['wrongAutomatic']==0,
                  reviewSafety=summary['reviewCorrect']==summary['reviewTotal'], proposalQuality=summary['proposalClearCorrect'] >= .95*summary['proposalClearTotal'], sourceGroundedSubgroups=summary['unsupportedSubgroups']==0,
                  naming=summary['namingFailures'] == 0*summary['prefixChecks'])
thresholds['guidedEmployerGrouping'] = summary['guidedEmployerCorrect']==summary['guidedEmployerTotal']
scored = dict(summary=summary, acceptance=thresholds, runs=rows, repeatChanges=repeat_changes, edgeCases=raw['edgeCases'])
summary['proposalReviewCorrect']=sum(r['proposal']['reviewCorrect'] for r in rows)
summary['proposalReviewTotal']=sum(r['proposal']['reviewTotal'] for r in rows)
scored['acceptance']['proposalReviewSafety']=summary['proposalReviewCorrect']==summary['proposalReviewTotal']
# An explicitly absent requested grouping field must stay at the subject.
missing_rows=[a for r in rows if r['creationPreferences'] and r['depth']>1 for a in r['proposal']['rows'] if a['id']=='missing-employer']
summary['missingGroupingCorrect']=sum(bool(a['actual'] and len(a['actual'].split('/'))==1 and a['correct']) for a in missing_rows)
summary['missingGroupingTotal']=len(missing_rows)
scored['acceptance']['missingGroupingFallback']=summary['missingGroupingCorrect']==summary['missingGroupingTotal']
# A factual grouping value must not repeat as an extra detail level.
repeated_details=[]
for run in raw['runs']:
 for assignment in run.get('proposal',{}).get('assignments',[]):
  labels=assignment['category'].lower().split('/')[1:]
  if any(a==b for a,b in zip(labels,labels[1:])):repeated_details.append(assignment)
summary['repeatedDetailLabels']=len(repeated_details)
scored['acceptance']['noRepeatedDetailLabels']=not repeated_details
(ROOT/'scores.json').write_text(json.dumps(scored,indent=2))
if (ROOT/'clean-controls.json').exists():
    clean = json.loads((ROOT/'clean-controls.json').read_text())
    gold = {f['id']:f for f in fixtures}
    indices = [0,1,2,3,4,5,9]
    clean_rows = []
    for r in clean['runs']:
        correct_proposals = 0
        for a in r['proposal'].get('assignments',[]):
            fixture = fixtures[indices[int(a['id'][1:])]]
            correct_proposals += a['category'].split('/')[0].lower() in subjects[fixture['subject']]
        matches = sum(d['decision']['confident'] and d['decision']['folder']==d['proposedFolder'] for d in r['decisions'])
        wrong = [d for d in r['decisions'] if d['decision']['confident'] and d['decision']['folder']!=d['proposedFolder']]
        employer_wrong=[]
        for d in r['decisions']:
            if not d['decision']['confident']: continue
            employer=gold[d['id']].get('employer')
            actual=d['decision']['folder'].lower().split('/')[1:]
            if any(label in part for part in actual for label in {'cedar','northstar','gamma'} - ({employer.lower()} if employer else set())):
                employer_wrong.append(d)
        clean_rows.append(dict(depth=r['depth'],guided=r['guided'],proposalAccepted=r['proposal']['ok'],proposalSubjectCorrect=correct_proposals,proposalSubjectTotal=7,
                               proposalFolderMatched=matches,filingTotal=len(r['decisions']),wrongAutomatic=len(wrong),wrongEmployerAutomatic=len(employer_wrong),employerFailures=employer_wrong))
    clean_summary=dict(runs=len(clean_rows),acceptedProposals=sum(r['proposalAccepted'] for r in clean_rows),
                       proposalSubjectCorrect=sum(r['proposalSubjectCorrect'] for r in clean_rows),proposalSubjectTotal=sum(r['proposalSubjectTotal'] for r in clean_rows),
                       proposedFolderMatched=sum(r['proposalFolderMatched'] for r in clean_rows),filingTotal=sum(r['filingTotal'] for r in clean_rows),
                       wrongAutomatic=sum(r['wrongAutomatic'] for r in clean_rows),wrongEmployerAutomatic=sum(r['wrongEmployerAutomatic'] for r in clean_rows))
    (ROOT/'clean-scores.json').write_text(json.dumps(dict(summary=clean_summary,runs=clean_rows),indent=2))
with (ROOT/'matrix.csv').open('w') as f:
    fields=['id','depth','documentInstructions','creationPreferences','folderRules','repeat','proposalAccepted','proposalClearCorrect','proposalClearTotal','proposalMaxDepth','filingCorrect','clearCorrect','clearTotal','reviewCorrect','reviewTotal','wrongAutomatic','namingFailures','medianSeconds']
    writer=csv.DictWriter(f,fieldnames=fields); writer.writeheader()
    for row in rows:
        flat={k:row[k] for k in fields if k in row}
        flat.update(proposalAccepted=row['proposal']['ok'],proposalClearCorrect=row['proposal']['clearCorrect'],proposalClearTotal=row['proposal']['clearTotal'],proposalMaxDepth=row['proposal']['maxDepth'])
        writer.writerow(flat)
print(json.dumps(summary,indent=2))
