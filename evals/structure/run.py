"""Frozen structure checks using production grouping code; no model or document writes."""
import argparse,json,os,shutil,subprocess
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--out',type=Path,required=True);p.add_argument('--assignments',type=Path);a=p.parse_args()
here=Path(__file__).resolve().parent;repo=here.parents[1];out=a.out.resolve()
if out.exists():p.error('Use a new report directory.')
out.mkdir(parents=True);h=out/'harness';(h/'src').mkdir(parents=True)
(h/'Cargo.toml').write_text((repo/'evals/rules-nesting/Cargo.toml').read_text().replace('tuck-rules-nesting-eval','tuck-structure-eval'))
modules=['agent','apple','categories','category_evidence','classifier','collection','config','coordinator','description','files','instructions','model','provider']
(h/'src/lib.rs').write_text('#![allow(dead_code)]\n'+''.join(f'#[path={json.dumps(str(repo/f"src-tauri/src/{m}.rs"))}]\nmod {m};\n' for m in modules)+'#[cfg(test)]\nmod matrix;\n')
shutil.copy2(here/'matrix.rs',h/'src/matrix.rs');shutil.copy2(here/'fixtures.json',out/'fixtures.json')
env={**os.environ,'TUCK_STRUCTURE_REPORT':str(out/'results.json'),'TUCK_STRUCTURE_BOUNDARY_REPORT':str(out/'boundaries.json')}
if a.assignments:env.update(TUCK_STRUCTURE_INPUT=str(a.assignments.resolve()),TUCK_STRUCTURE_REPLAY_REPORT=str(out/'replay.json'))
with (out/'tests.log').open('w') as log:subprocess.run(['cargo','test','--offline','--manifest-path',str(h/'Cargo.toml'),'collection_structure','--','--nocapture'],env=env,stdout=log,stderr=subprocess.STDOUT,check=True)
rows=json.loads((out/'results.json').read_text());boundary=json.loads((out/'boundaries.json').read_text());assert all(r['passed'] for r in rows) and boundary['passed']
print(f'{len(rows)} structure cases passed in both orders; batch and merged-rule boundaries passed.')
