#!/usr/bin/env python3
import importlib.util
from pathlib import Path
import stat
import struct
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

spec=importlib.util.spec_from_file_location('audit',Path(__file__).with_name('auditar_dll_pe.py'))
app=importlib.util.module_from_spec(spec);spec.loader.exec_module(app)


def image(dependency=None,delay=None):
    data=bytearray(0x800);data[:2]=b'MZ'
    def w16(p,v):struct.pack_into('<H',data,p,v)
    def w32(p,v):struct.pack_into('<I',data,p,v)
    w32(0x3c,0x80);data[0x80:0x84]=b'PE\0\0'
    w16(0x86,1);w16(0x94,240);opt=0x98
    w16(opt,0x20b);struct.pack_into('<Q',data,opt+24,0x140000000)
    w32(opt+60,0x200);w32(opt+108,16)
    sec=opt+240;w32(sec+12,0x1000);w32(sec+16,0x600);w32(sec+20,0x200)
    if dependency:
        w32(opt+112+8,0x1000);w32(opt+112+12,40)
        w32(0x200+12,0x1100);raw=dependency.encode()+b'\0';data[0x300:0x300+len(raw)]=raw
    if delay:
        w32(opt+112+13*8,0x1200);w32(opt+116+13*8,64)
        w32(0x400,1);w32(0x404,0x1300);raw=delay.encode()+b'\0';data[0x500:0x500+len(raw)]=raw
    return data


class Reader(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory();self.root=Path(self.temp.name)
        self.base=self.root/'base';self.upper=self.root/'upper';self.base.mkdir();self.upper.mkdir()
    def tearDown(self):self.temp.cleanup()
    def write(self,name,dep=None,delay=None):
        path=self.base/name;path.write_bytes(image(dep,delay));return path
    def test_static_and_delay_imports(self):
        self.assertEqual(app.imports(self.write('stub.exe','first.dll','later.dll')),['first.dll','later.dll'])
    def test_transitive_graph_cycle_and_case_insensitive_local_names(self):
        self.write('Cyberpunk2077.exe','FIRST.dll');self.write('first.dll','second.dll');self.write('second.dll','first.dll')
        result=app.audit(self.base,self.upper)
        self.assertEqual(result['audited_local_pe'],3);self.assertEqual(result['missing_local'],[])
    def test_transitive_whiteout_is_reported_and_recovery_clears_graph(self):
        self.write('Cyberpunk2077.exe','first.dll');self.write('first.dll','second.dll');self.write('second.dll')
        hidden=self.upper/'second.dll';hidden.touch();original=Path.lstat
        def lstat(path,*a,**kw):
            return SimpleNamespace(st_mode=stat.S_IFCHR,st_rdev=0) if path==hidden else original(path,*a,**kw)
        with patch.object(Path,'lstat',lstat):
            self.assertEqual(app.audit(self.base,self.upper)['missing_local'],[{'name':'second.dll','reason':'whiteout'}])
        hidden.unlink();self.assertEqual(app.audit(self.base,self.upper)['missing_local'],[])
    def test_system_dependency_is_outside_scope(self):
        self.write('Cyberpunk2077.exe','kernel32.dll')
        result=app.audit(self.base,self.upper)
        self.assertEqual(result['audited_local_pe'],1);self.assertTrue(result['system_libraries_not_audited'])
    def test_truncated_pe_rejected(self):
        p=self.write('bad.exe');p.write_bytes(b'MZ')
        with self.assertRaises(ValueError):app.imports(p)
    def test_unmapped_import_rva_rejected(self):
        p=self.write('bad.exe','valid.dll');data=bytearray(p.read_bytes());struct.pack_into('<I',data,0x20c,0xffffff00);p.write_bytes(data)
        with self.assertRaises(ValueError):app.imports(p)
    def test_path_traversal_import_rejected(self):
        with self.assertRaises(ValueError):app.imports(self.write('bad.exe','../outside.dll'))
    def test_unterminated_directory_rejected(self):
        p=self.write('bad.exe','valid.dll');data=bytearray(p.read_bytes());struct.pack_into('<I',data,0x98+112+12,20);p.write_bytes(data)
        with self.assertRaises(ValueError):app.imports(p)


if __name__=='__main__':unittest.main(verbosity=2)
