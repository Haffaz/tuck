from pathlib import Path
from PIL import Image
import json,re
ROOT=Path(__file__).parent;BASE=Path('/private/tmp/tuck-four-page-eval')
out=ROOT/'single-inputs';out.mkdir(mode=0o700,exist_ok=True)
manifest=[]
for case in json.loads((BASE/'manifest-private.json').read_text()):
 parts=re.split(r'(?m)^(D\d+) - page (\d+)\n',Path(case['text']).read_text())
 texts={(parts[i],int(parts[i+1])):parts[i+2].strip() for i in range(1,len(parts),3)}
 for source in case['sources']:
  key=case['id']+'-'+source['id']; pages=[p for p in case['pages'] if p['document']==source['id']]
  text='\n\n'.join('Page '+str(p['page'])+'\n'+texts[(source['id'],p['page'])] for p in pages)
  tp=out/(key+'.txt');tp.write_text(text);tp.chmod(0o600)
  if len(pages)==4:image=case['sheet']
  elif len(pages)==1:image=pages[0]['image']
  else:
   sheet=Image.new('RGB',(2480,3520),'white')
   for i,p in enumerate(pages):sheet.paste(Image.open(p['image']).convert('RGB'),((i%2)*1240,(i//2)*1760))
   image=str(out/(key+'.png'));sheet.save(image)
  expected=next(e for e in case['expected'] if e['id']==source['id'])
  manifest.append({'key':key,'case':case['id'],'sourceId':source['id'],'kind':case['kind'],'pageCount':len(pages),'text':str(tp),'image':image,'expected':expected})
(ROOT/'single-manifest-private.json').write_text(json.dumps(manifest,indent=2))
print('Prepared',len(manifest),'documents; IDs are runner metadata only')
