"""Local, synthetic helper checks. Never organizes documents or changes preferences."""
import json, subprocess, sys, time
from pathlib import Path
helper = Path(sys.argv[1])
output = Path(sys.argv[2])
rows = []
for model in ['apple', 'gemmaE2b', 'gemmaE4b']:
    for request in [
        {'operation':'status'},
        {'operation':'rule','folder':'Insurance','context':'Use clear English document names.'},
        {'operation':'categories','samples':[
            'A home insurance policy covering household property.',
            'A travel insurance certificate covering medical emergencies.',
            'An employment contract describing salary and working hours.',
            'A payslip listing monthly wages and deductions.',
            'A bank account statement listing transactions and balances.',
            'An income tax assessment listing taxes due.',
        ]},
    ]:
        started=time.monotonic()
        raw=subprocess.run([str(helper)],input=json.dumps({**request,'model':model}),capture_output=True,text=True,timeout=90,check=True)
        value=json.loads(raw.stdout)
        if 'error' in value:
            row={'model':model,'operation':request['operation'],'seconds':time.monotonic()-started,'output':value}
            rows.append(row); print(json.dumps(row),flush=True)
            continue
        if request['operation']=='status': assert 'ready' in value['status']
        elif request['operation']=='rule': assert 0<len(value['instruction'].strip())<=600
        else:
            assert 1<=len(value['rules'])<=6
            assert all(set(r)=={'folder','instruction'} for r in value['rules'])
        row={'model':model,'operation':request['operation'],'seconds':time.monotonic()-started,'output':value}
        rows.append(row)
        print(json.dumps(row),flush=True)
unknown=subprocess.run([str(helper)],input=json.dumps({'operation':'status','model':'remote-cloud'}),capture_output=True,text=True,timeout=15,check=True)
assert 'Unsupported local model' in json.loads(unknown.stdout)['error']
output.write_text(json.dumps({'cases':rows,'unknownModelRejected':True},indent=2)+'\n')

if any('error' in row['output'] for row in rows):
    raise SystemExit('One or more selected models failed; see the report.')
