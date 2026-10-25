#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Verificador independiente de las plantillas y del flujo de CASM Native.

Reconoce instrucciones, destinos, marcos, pila de expresiones y direcciones de
buffers; no toma los bytes del compilador como especificacion. El runtime es una
plantilla compartida auditada. No es un verificador general de ejecutables x86.
"""
import hashlib
from collections import deque
import struct

from runtime import DATA, MEMORY, HEADER, runtime

BASE=0x400000
LIMIT=262144


def require(ok,reason):
    if not ok:raise ValueError(reason)


def hx(s):return bytes.fromhex(s)


def audit(blob):
    require(HEADER<len(blob)<=LIMIT,'ELF-SIZE')
    require(blob[:16]==b'\x7fELF'+bytes([2,1,1,0])+bytes(8),'ELF-IDENT')
    fields=struct.unpack_from('<HHIQQQIHHHHHH',blob,16)
    require(fields==(2,62,1,BASE+HEADER,64,0,0,64,56,3,0,0,0),'ELF-CONTRACT')
    text,data,stack=[struct.unpack_from('<IIQQQQQQ',blob,64+i*56) for i in range(3)]
    end=text[5];data_offset=data[2];data_size=data[5]
    require(text==(1,5,0,BASE,BASE,end,end,4096),'TEXT-RX')
    require(HEADER<end<=data_offset and data_offset%4096==0,'SEGMENT-ORDER')
    require(data==(1,6,data_offset,DATA,DATA,data_size,MEMORY,4096),'DATA-RW')
    require(24<=data_size<=65536 and data_offset+data_size==len(blob),'DATA-SIZE')
    require(stack==(0x6474e551,6,0,0,0,0,0,16),'STACK-NX')
    require(blob[end:data_offset]==bytes(data_offset-end),'PADDING')
    require(blob[data_offset:data_offset+24]==bytes(24),'RUNTIME-STATE-ZERO')
    rt,labels=runtime();actual=bytearray(blob[HEADER:HEADER+len(rt)])
    main_at=labels['main_fix'];main=HEADER+main_at+4+struct.unpack_from('<i',actual,main_at)[0]
    actual[main_at:main_at+4]=bytes(4)
    require(bytes(actual)==rt,'RUNTIME-BYTES')
    targets={name:HEADER+off for name,off in labels.items()}

    def u32(p):return struct.unpack_from('<I',blob,p)[0]
    def u64(p):return struct.unpack_from('<Q',blob,p)[0]
    def relative(p):return p+4+struct.unpack_from('<i',blob,p)[0]
    def starts(p,s):return blob.startswith(hx(s),p)
    def region(base,cap=1):
        return cap>0 and cap<=262144 and base>=DATA+24 and base%8==0 and base+cap*8<=DATA+0x800000

    binary={}
    def guarded(prefix,condition,target):
        op=hx(prefix)+hx('0F '+condition)
        displacement=targets[target]  # se fija por posicion durante lectura
        return op,displacement
    # Sufijos del binario despues de mov rdx,rax; pop rax.
    for name,prefix,condition,target in [
        ('add','48 01 D0','82','overflow'),('sub','48 29 D0','82','overflow'),
        ('mul','48 F7 E2','82','overflow')]:binary[name]=guarded(prefix,condition,target)
    divisions=hx('48 89 D1 48 85 C9 0F 84')
    shifts=hx('48 89 D1 48 83 F9 40 0F 83')
    comparisons={hx('48 39 D0 0F '+x+' C0 0F B6 C0') for x in ['92','97','94','95','96','93']}
    bits={hx(x) for x in ['48 21 D0','48 09 D0','48 31 D0']}
    functions=[];instructions={};fn=None;p=HEADER+len(rt)
    while p<end:
        at=p
        if starts(p,'0F 1F 80'):
            require(p+44<=end,'FRAME-SIZE')
            argc=u32(p+3);require(argc<=8,'FRAME-ARGS')
            require(starts(p+7,'48 BA') and u64(p+9)==DATA+16,'FRAME-DEPTH-ADDRESS')
            require(starts(p+17,'48 81 3A 80 00 00 00 0F 83') and relative(p+26)==targets['depth'],'FRAME-DEPTH-GUARD')
            require(starts(p+30,'48 FF 02 55 48 89 E5 48 81 EC'),'FRAME-PROLOGUE')
            frame=u32(p+40);require(frame<=1024 and frame%8==0,'FRAME-LOCALS')
            if fn:fn['end']=p
            fn={'entry':p,'body':p+44,'argc':argc,'frame':frame,'end':None}
            functions.append(fn);p+=44;continue
        require(fn is not None,'FUNCTION-REQUIRED')
        kind='';value=None;edges=None
        if starts(p,'48 BA'):
            value=u64(p+2)
            if starts(p+10,'48 FF 0A C9 C3'):
                require(value==DATA+16,'RETURN-DEPTH');kind='return';p+=15;edges=[]
            elif starts(p+10,'48 39 D0 0F 83'):
                require(relative(p+15)==targets['bounds'],'BUFFER-GUARD')
                require(starts(p+19,'48 BA') and starts(p+29,'48 8D 04 C2'),'BUFFER-ADDRESS')
                base=u64(p+21);require(region(base,value),'BUFFER-RANGE');value=(base,value);p+=33
                if starts(p,'48 8B 00'):kind='array-load';p+=3
                elif blob[p:p+1]==b'\x50':kind='array-address-push';p+=1
                else:raise ValueError('BUFFER-USE')
            elif starts(p+10,'48 8B 02') or starts(p+10,'48 89 02'):
                require(region(value),'GLOBAL-RANGE');kind='global-load' if blob[p+11]==139 else 'global-store';p+=13
            else:raise ValueError('MEMORY-TEMPLATE')
        elif starts(p,'48 B8'):
            kind='constant';value=u64(p+2);p+=10
        elif starts(p,'48 8B 85') or starts(p,'48 89 85'):
            value=struct.unpack_from('<i',blob,p+3)[0]
            require(value%8==0 and (-fn['frame']<=value<=-8 or 16<=value<=16+8*(fn['argc']-1)),'LOCAL-RANGE')
            kind='local-load' if blob[p+1]==139 else 'local-store';p+=7
        elif starts(p,'48 89 C2 58'):
            kind='binary';p+=4;matched=False
            for prefix,target in binary.values():
                if blob.startswith(prefix,p):
                    require(relative(p+len(prefix))==target,'ARITHMETIC-GUARD');p+=len(prefix)+4;matched=True;break
            if not matched and blob.startswith(divisions,p):
                require(relative(p+8)==targets['division'],'DIVISION-GUARD')
                require(starts(p+12,'31 D2 48 F7 F1'),'DIVISION-TEMPLATE');p+=17
                if starts(p,'48 89 D0'):p+=3
                matched=True
            if not matched and blob.startswith(shifts,p):
                require(relative(p+9)==targets['overflow'],'SHIFT-WIDTH');p+=13
                if starts(p,'48 D3 E8'):p+=3
                else:
                    require(starts(p,'49 89 C0 48 D3 E0 49 89 C1 49 D3 E9 4D 39 C1 0F 85'),'SHIFT-LEFT')
                    require(relative(p+17)==targets['overflow'],'SHIFT-OVERFLOW');p+=21
                matched=True
            if not matched:
                for seq in comparisons|bits:
                    if blob.startswith(seq,p):p+=len(seq);matched=True;break
            require(matched,'BINARY-TEMPLATE')
        elif starts(p,'48 85 C0 0F 94 C0 0F B6 C0'):kind='not';p+=9
        elif starts(p,'5A 48 89 02'):kind='array-store';p+=4
        elif starts(p,'48 85 C0 0F 84'):
            kind='conditional';value=relative(p+5);p+=9;edges=[p,value]
        elif blob[p:p+1]==b'\xe9':kind='jump';value=relative(p+1);p+=5;edges=[value]
        elif blob[p:p+1]==b'\xe8':
            kind='call';target=relative(p+1);p+=5;argc=0
            if starts(p,'48 81 C4'):
                n=u32(p+3);require(0<n<=64 and n%8==0,'CALL-CLEANUP');argc=n//8;p+=7
            value=(target,argc)
        elif blob[p:p+1]==b'\x50':kind='push';p+=1
        else:raise ValueError(f'INSTRUCTION-TEMPLATE at {p:#x}')
        require(p<=end,'INSTRUCTION-SIZE')
        instructions[at]={'kind':kind,'value':value,'next':p,'edges':edges,'fn':fn}
    require(fn is not None and p==end,'CODE-END');fn['end']=end
    by_entry={f['entry']:f for f in functions}
    require(main in by_entry and by_entry[main]['argc']==0,'ENTRY-FUNCTION')
    runtime_calls={targets[n]:n for n in ['input','output','hexout','print','panic']}
    for at,ins in instructions.items():
        f=ins['fn'];kind=ins['kind']
        if kind in ('jump','conditional'):
            for target in ins['edges']:
                require(target in instructions and f['body']<=target<f['end'] and instructions[target]['fn'] is f,'BRANCH-BOUNDARY')
        if kind=='call':
            target,argc=ins['value']
            if target in by_entry:require(argc==by_entry[target]['argc'],'CALL-ARITY')
            else:
                require(target in runtime_calls,'CALL-TARGET')
                require(argc=={'input':2,'output':3,'hexout':3,'print':1,'panic':1}[runtime_calls[target]],'RUNTIME-ARITY')

    def scalar(x):return not isinstance(x,tuple)
    def join_value(a,b):
        if a==b:return a
        require(scalar(a) and scalar(b),'STACK-CAPABILITY-MERGE');return None
    visited=0;max_stack=0
    for f in functions:
        states={f['body']:((),None,0)};todo=deque([f['body']])
        while todo:
            at=todo.popleft();visited+=1;require(visited<2000000,'FLOW-BUDGET')
            require(at in instructions,'FLOW-BOUNDARY');ins=instructions[at]
            stack_values,rax,initialized=states[at];s=list(stack_values);kind=ins['kind'];value=ins['value'];terminal=False
            def need_scalar():require(scalar(rax),'SCALAR-REQUIRED')
            if kind=='constant':rax=value
            elif kind in ('local-load','local-store'):
                bit=0 if value>=16 else 1<<((-value//8)-1)
                if kind=='local-load':require(bit==0 or initialized&bit,'LOCAL-UNINITIALIZED');rax=None
                else:need_scalar();initialized|=bit
            elif kind in ('global-load','array-load'):
                if kind=='array-load':need_scalar()
                rax=None
            elif kind=='global-store':need_scalar()
            elif kind=='array-address-push':need_scalar();rax=('buffer',*value);s.append(rax)
            elif kind=='array-store':
                need_scalar();require(bool(s) and isinstance(s[-1],tuple),'BUFFER-STORE-CAPABILITY');s.pop()
            elif kind=='push':need_scalar();s.append(rax)
            elif kind=='binary':need_scalar();require(bool(s) and scalar(s[-1]),'BINARY-STACK');s.pop();rax=None
            elif kind=='not':need_scalar();rax=None
            elif kind=='conditional':need_scalar()
            elif kind=='return':need_scalar();require(not s,'RETURN-STACK');terminal=True
            elif kind=='call':
                target,argc=value;require(len(s)>=argc,'CALL-STACK')
                arguments=s[len(s)-argc:] if argc else []
                require(all(scalar(x) for x in arguments),'CALL-SCALARS')
                if argc:del s[-argc:]
                if target in runtime_calls:
                    name=runtime_calls[target]
                    if name in ('input','output','hexout'):
                        require(isinstance(arguments[0],int) and isinstance(arguments[1],int) and region(arguments[0],arguments[1]),'IO-BUFFER-RANGE')
                    if name=='panic':terminal=True
                    rax=None if name=='input' else 0
                else:rax=None
            require(len(s)<=128,'EXPRESSION-STACK');max_stack=max(max_stack,len(s))
            edges=[] if terminal else ins['edges'] if ins['edges'] is not None else [ins['next']]
            state=(tuple(s),rax,initialized)
            for target in edges:
                require(target in instructions and f['body']<=target<f['end'],'FUNCTION-FALLTHROUGH')
                if target not in states:states[target]=state;todo.append(target)
                else:
                    old=states[target];require(len(old[0])==len(s),'STACK-DEPTH-MERGE')
                    merged=(tuple(join_value(a,b) for a,b in zip(old[0],s)),join_value(old[1],rax),old[2]&initialized)
                    if merged!=old:states[target]=merged;todo.append(target)
    return {'schema':1,'profile':'CASM Native x86-64','verified':True,
            'sha256':hashlib.sha256(blob).hexdigest(),'bytes':len(blob),'functions':len(functions),
            'instruction_groups':len(instructions),'max_expression_stack':max_stack,
            'rx_text':True,'rw_data':True,'nx_stack':True,'no_dynamic_linker':True,
            'guard_scope':'closed templates, bounds, arithmetic traps, frames, calls and expression-stack flow',
            'limits':['Shared runtime template; no general x86 proof or proof of compiler source semantics.',
                      'Fixed addresses, no PIE or CET activation property. Laboratory profile.']}
