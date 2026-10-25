#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Bootstrap real, corpus Native y mutaciones del verificador. Solo QEMU/Linux."""
import hashlib
import json
import os
from pathlib import Path
import resource
import subprocess
import struct
import tempfile

from seed import ROOT,HERE,OUT,build,complete_source
from verify import audit


def limits():
    resource.setrlimit(resource.RLIMIT_CPU,(15,15))
    resource.setrlimit(resource.RLIMIT_CORE,(0,0))
    resource.setrlimit(resource.RLIMIT_FSIZE,(4194304,4194304))


def run(args,timeout=45,env=None):
    return subprocess.run([str(x) for x in args],capture_output=True,timeout=timeout,preexec_fn=limits,env=env)


def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()


def compile_with(compiler,source,base,expected=0):
    binary=base.with_suffix('.elf');hexfile=base.with_suffix('.hex')
    result=run([compiler,source,binary,hexfile])
    assert result.returncode==expected,(source.name,compiler.name,result.returncode,result.stdout,result.stderr)
    if expected:
        assert not binary.exists() and not hexfile.exists(),source
        return None
    blob=binary.read_bytes();require_hex=bytes.fromhex(hexfile.read_text())
    assert blob==require_hex,'HEX-ROUNDTRIP'
    audit(blob)
    return binary


GOOD=[
    ('empty','u64 casm_main() { return 0; }',b'',0),
    ('print','u64 casm_main() { print(42); return 0; }',b'42\n',0),
    ('precedence','u64 casm_main() { print(2+3*4); print((2+3)*4); print(99/7); print(99%7); return 0; }',b'14\n20\n14\n1\n',0),
    ('bits','u64 casm_main() { print((31&7)|8); print(31^7); print(3<<4); print(128>>3); print(!0); print(!7); return 0; }',b'15\n24\n48\n16\n1\n0\n',0),
    ('comparisons','u64 casm_main() { print(2<3); print(2>3); print(3<=3); print(3>=4); print(3==3); print(3!=3); return 0; }',b'1\n0\n1\n0\n1\n0\n',0),
    ('max','u64 casm_main() { print(18446744073709551615); print(0xFFFFFFFFFFFFFFFF); return 0; }',b'18446744073709551615\n18446744073709551615\n',0),
    ('locals','u64 casm_main() { u64 a=2; u64 b=a+3; a=b*4; print(a); return 0; }',b'20\n',0),
    ('while','u64 casm_main() { u64 i=0; u64 n=0; while(i<10) { n=n+i; i=i+1; } print(n); return 0; }',b'45\n',0),
    ('break','u64 casm_main() { u64 i=0; while(1) { if(i==5) { break; } i=i+1; } print(i); return 0; }',b'5\n',0),
    ('nested-break','u64 casm_main() { u64 i=0; u64 j=0; u64 n=0; while(i<3) { j=0; while(1) { n=n+1; if(j==2) { break; } j=j+1; } i=i+1; } print(n); return 0; }',b'9\n',0),
    ('if-else','u64 casm_main() { u64 n=7; if(n<3) { print(1); } else if(n==7) { print(2); } else { print(3); } return 0; }',b'2\n',0),
    ('forward','u64 casm_main() { print(add(8,9)); return 0; } u64 add(u64 a,u64 b) { return a+b; }',b'17\n',0),
    ('eight-args','u64 sum(u64 a,u64 b,u64 c,u64 d,u64 e,u64 f,u64 g,u64 h) { return a+b+c+d+e+f+g+h; } u64 casm_main() { print(sum(1,2,3,4,5,6,7,8)); return 0; }',b'36\n',0),
    ('value-args','u64 inc(u64 a) { a=a+1; return a; } u64 casm_main() { u64 x=5; print(inc(x)); print(x); return 0; }',b'6\n5\n',0),
    ('recursion','u64 fac(u64 n) { if(n==0) { return 1; } return n*fac(n-1); } u64 casm_main() { print(fac(10)); return 0; }',b'3628800\n',0),
    ('buffers','u64 a[4]={1,2,3,4}; u64 casm_main() { u64 i=0; while(i<4) { a[i]=a[i]*2; i=i+1; } print(a[0]+a[3]); return 0; }',b'10\n',0),
    ('zero-bss','u64 a[262144]; u64 b; u64 casm_main() { print(a[262143]); print(b); return 0; }',b'0\n0\n',0),
    ('global','u64 a=18446744073709551615; u64 casm_main() { a=a-1; print(a); return 0; }',b'18446744073709551614\n',0),
    ('lhs-call','u64 a[4]; u64 twice(u64 n) { return n*2; } u64 casm_main() { a[2]=twice(9)+1; print(a[2]); return 0; }',b'19\n',0),
    ('return-status','u64 casm_main() { return 42; }',b'',42),
    ('default-return','u64 f() { print(1); } u64 casm_main() { print(f()); }',b'1\n0\n',0),
]
TRAPS=[
    ('add','print(18446744073709551615+1);',70),
    ('sub','print(0-1);',70),('mul','print(18446744073709551615*2);',70),
    ('div','print(4/0);',71),('mod','print(4%0);',71),
    ('shift-width','print(1<<64);',70),('shift-overflow','print(18446744073709551615<<1);',70),
    ('shift-right-width','print(1>>64);',70),
]
BAD=[
    ('empty','',65),('other-entry','u64 main() { return 0; }',65),
    ('unknown','u64 casm_main() { return missing(); }',69),
    ('undefined','u64 casm_main() { return x; }',69),
    ('self-init','u64 casm_main() { u64 x=x; return x; }',69),
    ('branch-local','u64 casm_main() { if(1) { u64 a=1; } return 0; }',65),
    ('duplicate-local','u64 casm_main() { u64 x=1; u64 x=2; return x; }',65),
    ('duplicate-function','u64 casm_main() { return 0; } u64 casm_main() { return 1; }',65),
    ('duplicate-param','u64 f(u64 x,u64 x) { return x; } u64 casm_main() { return 0; }',65),
    ('reserved','u64 casm_main() { u64 while=0; return 0; }',65),
    ('argc','u64 f(u64 x) { return x; } u64 casm_main() { return f(); }',65),
    ('entry-args','u64 casm_main(u64 x) { return x; }',65),
    ('bare-array','u64 a[2]; u64 casm_main() { print(a); return 0; }',65),
    ('zero-array','u64 a[0]; u64 casm_main() { return 0; }',65),
    ('large-array','u64 a[262145]; u64 casm_main() { return 0; }',65),
    ('initializer','u64 a[1]={1,2}; u64 casm_main() { return 0; }',65),
    ('break-outside','u64 casm_main() { break; return 0; }',65),
    ('pointer','u64 casm_main() { u64 *p=0; return 0; }',65),
    ('preprocessor','#include <stdio.h>\nu64 casm_main() { return 0; }',65),
    ('literal','u64 casm_main() { return 18446744073709551616; }',65),
    ('unterminated','u64 casm_main() { /*',65),
    ('shadow-buffer','u64 a[2]; u64 casm_main() { u64 a=1; print(a[0]); return 0; }',65),
    ('deep','u64 casm_main() {'+'{'*34+'return 0;'+'}'*34+'}',65),
    ('non-ascii','u64 casm_main() { return 0; }\u00ff',65),
]


def main():
    assert os.getuid()!=0,'ejecutar como usuario del taller'
    assert Path('/sys/class/dmi/id/sys_vendor').read_text().strip()=='QEMU','banco completo solo dentro de QEMU'
    assert (HERE/'compiler.casm').read_text()==complete_source(),'regenerar fuente completa'
    OUT.mkdir(parents=True,exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='native-pruebas-',dir=OUT) as tmp:
        work=Path(tmp);chains=[]
        for compiler in ['gcc','clang']:
            seed=build(compiler);one=compile_with(seed,HERE/'compiler.casm',work/(compiler+'-one'))
            two=compile_with(one,HERE/'compiler.casm',work/(compiler+'-two'))
            three=compile_with(two,HERE/'compiler.casm',work/(compiler+'-three'))
            assert one.read_bytes()==two.read_bytes()==three.read_bytes(),'SELF-HOST-DRIFT'
            if chains:assert chains[0].read_bytes()==three.read_bytes(),'CROSS-SEED-DRIFT'
            chains.append(three)
            print('GREEN [SELF-HOST]',compiler,'3 generaciones identicas',sha(three),flush=True)
        native=chains[0];results=[]
        for name,text,stdout,status in GOOD:
            source=work/(name+'.casm');source.write_text(text)
            outputs=[compile_with(c,source,work/(name+'-'+str(i))) for i,c in enumerate(chains)]
            assert outputs[0].read_bytes()==outputs[1].read_bytes(),'CORPUS-DRIFT'
            result=run([outputs[0]])
            assert (result.returncode,result.stdout)==(status,stdout),(name,result.returncode,result.stdout,result.stderr)
            results.append({'case':name,'kind':'accepted','passed':True})
        for name,body,status in TRAPS:
            source=work/(name+'.casm');source.write_text('u64 casm_main() { '+body+' return 0; }')
            binary=compile_with(native,source,work/(name+'-trap'))
            result=run([binary]);assert result.returncode==status and b'CASM trap' in result.stderr,(name,result)
            results.append({'case':name,'kind':'runtime-trap','status':status,'passed':True})
        for name,text,status in BAD:
            source=work/(name+'.casm');source.write_text(text)
            compile_with(native,source,work/(name+'-rejected'),status)
            results.append({'case':name,'kind':'source-rejected','status':status,'passed':True})
        for name,text,status in [
            ('bounds','u64 a[2]; u64 casm_main() { print(a[2]); return 0; }',72),
            ('bounds-store','u64 a[2]; u64 casm_main() { a[18446744073709551615]=1; return 0; }',72),
            ('depth','u64 f() { return f(); } u64 casm_main() { return f(); }',73)]:
            source=work/(name+'.casm');source.write_text(text);binary=compile_with(native,source,work/(name+'-checked'))
            result=run([binary]);assert result.returncode==status,(name,result)
            results.append({'case':name,'kind':'runtime-trap','status':status,'passed':True})
        # Entrada/salida de mas de un bloque y ninguna biblioteca dinamica.
        io_source=work/'io.casm';io_source.write_text('u64 a[131072]; u64 casm_main() { u64 n=input(a); output(a,n); hexout(a,n); return 0; }')
        io_binary=compile_with(native,io_source,work/'io-program')
        input_data=bytes(range(256))*511;raw=work/'input.bin';raw.write_bytes(input_data)
        raw_out=work/'output.bin';hex_out=work/'output.hex'
        result=run([io_binary,raw,raw_out,hex_out])
        assert result.returncode==0 and raw_out.read_bytes()==input_data and bytes.fromhex(hex_out.read_text())==input_data,result
        results.append({'case':'io-multiblock','kind':'io','bytes':len(input_data),'passed':True})
        existing=work/'existing.bin';existing.write_bytes(b'conservar')
        result=run([io_binary,raw,existing,work/'unused.hex'])
        assert result.returncode==74 and existing.read_bytes()==b'conservar' and not (work/'unused.hex').exists()
        results.append({'case':'io-no-overwrite','kind':'io','passed':True})
        link=work/'input-link';link.symlink_to(raw)
        result=run([io_binary,link,work/'link-output.bin',work/'link-output.hex'])
        assert result.returncode==74 and not (work/'link-output.bin').exists()
        results.append({'case':'io-no-symlink','kind':'io','passed':True})
        source=work/'byte-range.casm';source.write_text('u64 a[1]={256}; u64 casm_main() { output(a,1); return 0; }')
        binary=compile_with(native,source,work/'byte-range-program')
        result=run([binary,raw,work/'byte-range.bin',work/'byte-range.hex'])
        assert result.returncode==70,result
        results.append({'case':'io-byte-range','kind':'runtime-trap','status':70,'passed':True})
        # Mutaciones: el HEX alterado no se confia aunque proceda de nuestro compilador.
        original=native.read_bytes();mutations=[]
        for name,at,value,rule in [('machine',18,183,'ELF-CONTRACT'),('writable-code',68,7,'TEXT-RX'),('exec-stack',180,7,'STACK-NX'),('runtime-opcode',HEADER,0x90,'RUNTIME-BYTES')]:
            changed=bytearray(original);changed[at]=value
            try:audit(bytes(changed))
            except ValueError as error:
                assert str(error)==rule,(name,error)
                mutations.append({'case':name,'rule':rule})
            else:raise AssertionError('mutation accepted: '+name)
        from runtime import runtime,DATA
        rt,_=runtime();first=HEADER+len(rt)
        variants=[]
        def mutation(name,offset,data,rule,blob=original):
            changed=bytearray(blob);changed[offset:offset+len(data)]=data;variants.append((name,bytes(changed),rule))
        mutation('arity',first+3,struct.pack('<I',9),'FRAME-ARGS')
        mutation('frame',first+40,struct.pack('<I',1032),'FRAME-LOCALS')
        mutation('depth-guard',first+26,struct.pack('<i',0),'FRAME-DEPTH-GUARD')
        arithmetic=(work/'precedence-0.elf').read_bytes();at=arithmetic.index(bytes.fromhex('48 89 C2 58 48 F7 E2 0F 82'))
        mutation('overflow-guard',at+9,struct.pack('<i',0),'ARITHMETIC-GUARD',arithmetic)
        buffered=(work/'buffers-0.elf').read_bytes();at=buffered.index(bytes.fromhex('48 39 D0 0F 83'))
        mutation('bounds-guard',at+5,struct.pack('<i',0),'BUFFER-GUARD',buffered)
        simple=bytearray((work/'empty-0.elf').read_bytes());body=first+44;data_offset=struct.unpack_from('<Q',simple,128)[0]
        ret=bytes.fromhex('48 BA')+struct.pack('<Q',DATA+16)+bytes.fromhex('48 FF 0A C9 C3')
        for name,replacement,frame,rule in [
            ('return-stack',bytes.fromhex('48 B8')+bytes(8)+b'\x50'+ret,0,'RETURN-STACK'),
            ('uninitialized-local',bytes.fromhex('48 8B 85 F8 FF FF FF')+ret,8,'LOCAL-UNINITIALIZED'),
            ('saved-return-address',bytes.fromhex('48 8B 85 08 00 00 00')+ret,0,'LOCAL-RANGE')]:
            changed=simple[:body]+replacement+bytes(data_offset-body-len(replacement))+simple[data_offset:]
            struct.pack_into('<Q',changed,96,body+len(replacement));struct.pack_into('<Q',changed,104,body+len(replacement))
            struct.pack_into('<I',changed,first+40,frame)
            variants.append((name,bytes(changed),rule))
        for name,changed,rule in variants:
            try:audit(changed)
            except ValueError as error:
                assert str(error)==rule,(name,error)
                mutations.append({'case':name,'rule':rule})
            else:raise AssertionError('mutation accepted: '+name)
        # Publicar solo una generacion que acaba de pasar el verificador.
        release=OUT/'casmc-native.elf';release.write_bytes(native.read_bytes());release.chmod(0o755)
        (OUT/'casmc-native.hex').write_text('\n'.join(f'{b:02X}' for b in native.read_bytes())+'\n')
        report={'schema':1,'profile':'CASM Native 0.1 x86-64','self_hosted':True,
                'legacy_metal64_frontend_self_hosted':False,'source_sha256':sha(HERE/'compiler.casm'),
                'compiler_sha256':sha(release),'external_seeds':['gcc','clang'],'generations_per_seed':3,
                'all_generations_equal':True,'cases':results,'mutation_rejections':mutations,
                'native_audit':audit(release.read_bytes()),'empty_root_execution':'pending'}
        (OUT/'self-host.json').write_text(json.dumps(report,indent=2)+'\n')
        # El mismo compilador escrito en CASM se comprueba tambien instrumentado
        # como semilla C: no convierte los programas del usuario en C.
        for cc in ['gcc','clang']:
            seed=build(cc,True);result=compile_with(seed,HERE/'compiler.casm',work/(cc+'-asan-self'))
            assert result.read_bytes()==native.read_bytes(),'SANITIZED-SEED-DRIFT'
            for name,text,status in BAD:
                source=work/(name+'.casm')
                compile_with(seed,source,work/(cc+'-asan-'+name),status)
        # Probar el publicador con los limites que usara el usuario.
        from driver import compile_source
        published=compile_source(HERE/'ejemplo.casm')
        result=run([published/'program.elf']);assert result.stdout==b'50\n' and result.returncode==0,result
        invalid=work/'publisher-invalid.casm';invalid.write_text('u64 casm_main() { return unknown; }')
        try:compile_source(invalid)
        except ValueError as error:assert 'codigo 69' in str(error),error
        else:raise AssertionError('publisher accepted invalid source')
        report['sanitized_seeds_checked']=['gcc','clang'];report['publisher_example_passed']=True
        report['publisher_rejection_passed']=True
        (OUT/'self-host.json').write_text(json.dumps(report,indent=2)+'\n')
        print('GREEN [NATIVE]',len(results),'casos,',len(mutations),'mutaciones; self_hosted=true (perfil Native)',flush=True)


if __name__=='__main__':
    from runtime import HEADER
    main()
