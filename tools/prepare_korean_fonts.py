"""Add immutable OFL Nanum Gothic input to a craft-fonts checkout, without altering OS fonts."""
import argparse
import base64
import hashlib
import json
import os
from pathlib import Path
import urllib.request

FONT_BLOB='2aa2b20905b4136808a0126d8e36e5c0f576715c'
LICENSE_BLOB='69ff40157d0bba4c65e4ecf77564e1f01d0b02a5'

def download_blob(sha):
    url=f'https://api.github.com/repos/google/fonts/git/blobs/{sha}'
    headers={'User-Agent':'craft-korean-font-input','Accept':'application/vnd.github+json'}
    token=os.environ.get('GITHUB_TOKEN')
    if token: headers['Authorization']='Bearer '+token
    with urllib.request.urlopen(urllib.request.Request(url,headers=headers),timeout=90) as response:
        obj=json.load(response)
    if obj.get('sha')!=sha or obj.get('encoding')!='base64':
        raise ValueError('Unexpected font blob response')
    data=base64.b64decode(obj['content'])
    digest=hashlib.sha1(b'blob '+str(len(data)).encode()+b'\0'+data).hexdigest()
    if digest!=sha or len(data)!=obj.get('size'):
        raise ValueError('Font blob checksum mismatch')
    return data

def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--directory',type=Path,required=True)
    args=parser.parse_args()
    manifest=args.directory/'fonts/manifest.txt'
    if not manifest.is_file(): raise ValueError('Expected an existing craft-fonts checkout')
    dest=args.directory/'fonts/nanum-gothic'
    dest.mkdir(parents=True,exist_ok=True)
    font=download_blob(FONT_BLOB)
    license=download_blob(LICENSE_BLOB)
    (dest/'NanumGothic-Regular.ttf').write_bytes(font)
    (dest/'OFL.txt').write_bytes(license)
    digest=hashlib.sha256(font).hexdigest()
    line=' | '.join(['Nanum Gothic','Regular','fonts/nanum-gothic/NanumGothic-Regular.ttf','Hang,Latn','OFL-1.1','fonts/nanum-gothic/OFL.txt',digest,f'https://api.github.com/repos/google/fonts/git/blobs/{FONT_BLOB}'])
    rows=manifest.read_text(encoding='utf8').splitlines()
    rows=[row for row in rows if not row.startswith('Nanum Gothic | ')]
    manifest.write_text('\n'.join(rows+[line])+'\n',encoding='utf8',newline='\n')
    print(json.dumps({'family':'Nanum Gothic','bytes':len(font),'sha256':digest}))

if __name__=='__main__': main()
