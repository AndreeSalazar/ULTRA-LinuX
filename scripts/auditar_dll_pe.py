#!/usr/bin/env python3
"""Read PE import tables without loading or executing the image.

The overlay audit reports only same-directory DLLs provided by the verified
base, not Windows/system libraries supplied by Proton. Delay imports are
included. This is a loader preflight, not proof that a game runs.
"""
import json
from pathlib import Path
import stat
import struct
import sys


def imports(path):
    with Path(path).open('rb') as stream:
        size=stream.seek(0,2)
        if size>256*1024*1024:raise ValueError('PE too large')
        stream.seek(0);data=stream.read()
    def bounds(offset,count):
        if offset<0 or count<0 or offset+count>len(data):raise ValueError('PE bounds')
        return offset
    def u16(offset):return struct.unpack_from('<H',data,bounds(offset,2))[0]
    def u32(offset):return struct.unpack_from('<I',data,bounds(offset,4))[0]
    if data[:2]!=b'MZ':raise ValueError('MZ missing')
    pe=u32(0x3c)
    if data[bounds(pe,24):pe+4]!=b'PE\0\0':raise ValueError('PE signature')
    count=u16(pe+6);optional_size=u16(pe+20);opt=pe+24
    bounds(opt,optional_size)
    magic=u16(opt)
    if magic==0x20b:
        directory=opt+112;number=opt+108
        image_base=struct.unpack_from('<Q',data,bounds(opt+24,8))[0]
    elif magic==0x10b:
        directory=opt+96;number=opt+92;image_base=u32(opt+28)
    else:raise ValueError('PE optional header')
    if count>96 or number+4>opt+optional_size:raise ValueError('PE limits')
    directories=u32(number)
    headers=u32(opt+60)
    sections=[]
    for i in range(count):
        p=opt+optional_size+i*40;bounds(p,40)
        sections.append((u32(p+12),u32(p+16),u32(p+20)))
    def offset(rva,length):
        if 0<=rva<headers and rva+length<=headers:return bounds(rva,length)
        for start,raw_size,raw in sections:
            if start<=rva and rva+length<=start+raw_size:
                return bounds(raw+rva-start,length)
        raise ValueError('unmapped PE RVA')
    def name(rva):
        p=offset(rva,1);end=data.find(b'\0',p,min(len(data),p+261))
        if end<0:raise ValueError('PE DLL name too long')
        value=data[p:end].decode('ascii')
        if not value or '/' in value or '\\' in value or value in ('.','..'):raise ValueError('PE DLL name')
        return value
    result=set()
    for index,width in ((1,20),(13,32)):
        if directories<=index:continue
        entry=directory+index*8
        if entry+8>opt+optional_size:raise ValueError('PE directory outside optional header')
        rva=u32(entry);length=u32(entry+4)
        if not rva or not length:continue
        if length>1024*1024:raise ValueError('PE import directory too large')
        terminated=False
        for i in range(min(length//width,4096)):
            p=offset(rva+i*width,width)
            if data[p:p+width]==bytes(width):terminated=True;break
            if index==1:nrva=u32(p+12)
            else:
                attrs=u32(p);nrva=u32(p+4)
                if not attrs&1:nrva-=image_base
            result.add(name(nrva))
        if not terminated:raise ValueError('unterminated PE imports')
    return sorted(result,key=str.casefold)


def audit(base,upper):
    base=Path(base);upper=Path(upper)
    original={p.name.casefold():p for p in base.iterdir() if p.is_file() and not p.is_symlink()}
    changed={p.name.casefold():p for p in upper.iterdir()} if upper.is_dir() else {}
    todo=['cyberpunk2077.exe'];seen=set();missing=[];edges=[]
    while todo:
        key=todo.pop()
        if key in seen:continue
        if len(seen)>=512:raise ValueError('dependency graph too large')
        seen.add(key)
        path=changed.get(key,original.get(key))
        if path is None:raise ValueError('entry point absent')
        s=path.lstat()
        if not stat.S_ISREG(s.st_mode):
            if stat.S_ISCHR(s.st_mode) and s.st_rdev==0 and key in original:
                missing.append({'name':original[key].name,'reason':'whiteout'})
                path=original[key]
            else:raise ValueError('unsupported local dependency entry')
        if path.is_symlink():raise ValueError('redirected dependency')
        for dependency in imports(path):
            target=dependency.casefold()
            if target in original or target in changed:
                edges.append({'from':path.name,'to':dependency})
                todo.append(target)
    return {'audited_local_pe':len(seen),'missing_local':missing,'local_edges':edges,
            'system_libraries_not_audited':True,'gameplay_tested':False}


if __name__=='__main__':
    print(json.dumps(audit(sys.argv[1],sys.argv[2]),indent=2))
