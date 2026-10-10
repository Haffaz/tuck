import pathlib,json,statistics
r=pathlib.Path('/private/tmp/tuck-generic-production')
traces=[json.loads(p.read_text()) for p in sorted((r/'traces').glob('*.json'))]
rows=json.loads((r/'results.json').read_text())['results']
for row in rows:
 ts=[t for t in traces if t['case']==row['id']]
 stages={op:[t for t in ts if t['request'].get('operation')==op] for op in ['language','describe','match','verify_match','name']}
 row['stages']={k:[{'seconds':t['seconds'],'output':t['response']} for t in v] for k,v in stages.items()}
 row['stage_seconds']={k:sum(t['seconds'] for t in v) for k,v in stages.items()}
traces=[t for t in traces if t['case'] in {row['id'] for row in rows}]
positive=[x for x in rows if x['expected_folder']!='Needs Review']
summary={'total':len(rows),'clear_documents':len(positive),'clear_correctly_filed':sum(x['folder_correct'] for x in positive),'clear_incorrectly_filed':sum(not x['folder_correct'] and not x['review'] for x in positive),'clear_in_fallback':sum(x['review'] for x in positive),'all_expected_destinations':sum(x['folder_correct'] for x in rows),'fallback':sum(x['review'] for x in rows),'median_seconds':statistics.median(x['seconds'] for x in rows),'max_seconds':max(x['seconds'] for x in rows),'stage_attempts':{op:sum(t['request'].get('operation')==op for t in traces) for op in ['describe','match','verify_match','name']},'stage_errors':{op:sum(t['request'].get('operation')==op and 'error' in t['response'] for t in traces) for op in ['describe','match','verify_match','name']}}
(r/'evaluation.json').write_text(json.dumps({'summary':summary,'results':rows},ensure_ascii=False,indent=2))
print(json.dumps(summary,indent=2))
