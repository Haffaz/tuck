"""Opt-in, on-device evaluation against production modules; never moves files."""
import argparse, hashlib, json, os, shutil, subprocess
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[1]
p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--suite', choices=['core','diverse','heldout','diverse-heldout'], default='diverse')
p.add_argument('--out', type=Path, required=True, help='New report directory')
p.add_argument('--repo', type=Path, default=REPO)
p.add_argument('--resources', type=Path)
p.add_argument('--fixtures', type=Path, help='Frozen fixture JSON matching the selected suite rubric')
p.add_argument('--pilot', action='store_true', help='Only four endpoint combinations in core/heldout')
p.add_argument('--diagnostics', action='store_true', help='Additional core role traces and boundaries')
a=p.parse_args();repo=a.repo.resolve();out=a.out.resolve()
if out.exists():p.error('Use a new output directory to preserve previous measurements.')
resources=(a.resources or repo/'src-tauri/resources').resolve()
for name in ['tuck-reader','tuck-apple']:
 if not (resources/name).is_file():p.error(f'Build Tuck first; missing {resources/name}')
out.mkdir(parents=True)
suite=HERE/a.suite
shutil.copy2(a.fixtures or suite/'fixtures.json',out/'fixtures.json')
shutil.copy2(suite/'score.py',out/'score.py')
harness=out/'harness';(harness/'src').mkdir(parents=True);(harness/'resources').mkdir()
manifest=(HERE/'Cargo.toml').read_text().replace('tuck-rules-nesting-eval',f'tuck-{a.suite}-{hashlib.sha256(str(out).encode()).hexdigest()[:8]}-eval')
(harness/'Cargo.toml').write_text(manifest)
modules=['agent','apple','categories','category_evidence','classifier','collection','config','coordinator','description','files','instructions','model','provider']
source='#\u0021[allow(dead_code)]\n'+''.join(f'#[path={json.dumps(str(repo/f"src-tauri/src/{m}.rs"))}]\nmod {m};\n' for m in modules)+'#[cfg(test)]\nmod matrix;\n'
if a.suite=='core':
 source+='#[cfg(test)]\nmod edges;\n';shutil.copy2(suite/'edges.rs',harness/'src/edges.rs')
(harness/'src/lib.rs').write_text(source)
shutil.copy2(suite/'matrix.rs',harness/'src/matrix.rs')
for name in ['tuck-reader','tuck-apple']:shutil.copy2(resources/name,harness/'resources'/name)
tracked=subprocess.check_output(['git','ls-files'],cwd=repo,text=True).splitlines()
paths=[repo/f for f in tracked if f.startswith('src-tauri/src/') or f.startswith('src-tauri/swift/')]+[resources/n for n in ['tuck-reader','tuck-apple']]
provenance={'fixtureSource':str((a.fixtures or suite/'fixtures.json').resolve()),'suite':a.suite,'pilot':a.pilot,'git':subprocess.check_output(['git','status','--short','--branch'],cwd=repo,text=True),'sources':{str(f):hashlib.sha256(f.read_bytes()).hexdigest() for f in paths},'fixtures':hashlib.sha256((out/'fixtures.json').read_bytes()).hexdigest()}
(out/'provenance.json').write_text(json.dumps(provenance,indent=2))
env={**os.environ,'TUCK_MATRIX_OUT':str(out)}
if a.pilot:env['TUCK_MATRIX_PILOT']='1'
tests=['live_diverse_matrix'] if a.suite in ['diverse','diverse-heldout'] else ['live_rules_nesting_matrix']
if a.suite=='core':
 tests+=['live_rule_scope_edges','live_clean_proposal_filing_controls']
 if a.diagnostics:tests+=['live_rules_nesting_diagnostics']
for test in tests:
 with (out/f'{test}.log').open('w') as log:
  subprocess.run(['cargo','test','--offline','--manifest-path',str(harness/'Cargo.toml'),test,'--','--nocapture','--test-threads=1'],env=env,stdout=log,stderr=subprocess.STDOUT,check=True)
raw=json.loads((out/'results.json').read_text())
expected_runs=4 if a.suite in ['diverse','diverse-heldout'] or a.pilot else 20
if not raw.get('completed') or len(raw['runs'])!=expected_runs:raise SystemExit('Incomplete evaluation: check test discovery and logs.')
if a.suite=='core':
 if len(json.loads((out/'scope-edges.json').read_text()))!=4:raise SystemExit('Incomplete scope edge checks.')
 controls=json.loads((out/'clean-controls.json').read_text())
 if not controls.get('completed') or len(controls['runs'])!=4:raise SystemExit('Incomplete generated-folder checks.')
subprocess.run(['python3',str(out/'score.py')],check=True)
scores=json.loads((out/'scores.json').read_text())
print(f'Report: {out}')
if not all(scores['acceptance'].values()):raise SystemExit('Evaluation finished, but acceptance targets were not met. Inspect scores.json.')
