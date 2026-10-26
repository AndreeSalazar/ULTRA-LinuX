#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
import copy
import os
from pathlib import Path
import tempfile
import sys
import unittest
import revision_vigencia as v


class Vigencia(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory();self.root=Path(self.temp.name)
        (self.root/'src').mkdir();(self.root/'src/a.c').write_text('int n=42;\n');(self.root/'src/a.h').write_text('#define N 42\n')
        self.sources={'src/a.c':v.digest(self.root/'src/a.c'),'src/a.h':v.digest(self.root/'src/a.h')}
        self.config=[{'file':'src/a.c','compiler':'clang','phase':'warnings-O2','flags':['-O2']}]
        self.env={'versions':{'clang':'v1'},'binaries_and_libraries':{'/usr/bin/clang':'a'*64}}
        self.record={'schema':2,'kind':'ultra-project-review','production_certified':False,
                     'configuration_sha256':v.canonical(self.config),'environment':self.env,'engine_sha256':'engine',
                     'sources':self.sources,'dependencies':{'project:'+k:h for k,h in self.sources.items()},
                     'checks':[{'file':'src/a.c','compiler':'clang','phase':'warnings-O2','status':'clean','dependencies':['project:src/a.c','project:src/a.h']}]}
    def tearDown(self):self.temp.cleanup()
    def result(self,record=None,sources=None,config=None,env=None):
        return v.assess(self.record if record is None else record,self.root,self.sources if sources is None else sources,
                        self.config if config is None else config,self.env if env is None else env,'engine')
    def test_current(self):self.assertEqual(self.result()['state'],'revisado')
    def test_same_size_and_mtime_changed_header_is_stale(self):
        p=self.root/'src/a.h';before=p.stat();p.write_text('#define N 43\n');os.utime(p,ns=(before.st_atime_ns,before.st_mtime_ns))
        self.assertEqual(self.result()['state'],'caducado')
    def test_missing_header(self):
        (self.root/'src/a.h').unlink();self.assertEqual(self.result()['state'],'caducado')
    def test_binary_changes_without_version_change(self):
        env=copy.deepcopy(self.env);env['binaries_and_libraries']['/usr/bin/clang']='b'*64
        self.assertEqual(self.result(env=env)['state'],'caducado')
    def test_shared_library_change(self):
        env=copy.deepcopy(self.env);env['binaries_and_libraries']['/usr/lib/libLLVM.so']='b'*64
        self.assertEqual(self.result(env=env)['state'],'caducado')
    def test_changed_flags(self):
        config=copy.deepcopy(self.config);config[0]['flags']=['-O3'];self.assertEqual(self.result(config=config)['state'],'caducado')
    def test_missing_check_is_pending(self):
        r=copy.deepcopy(self.record);r['checks']=[];self.assertEqual(self.result(record=r)['state'],'pendiente')
    def test_failed_tool_is_not_clean(self):
        r=copy.deepcopy(self.record);r['checks'][0]['status']='tool_failed';self.assertEqual(self.result(record=r)['state'],'pendiente')
    def test_diagnostic_is_not_runtime_confirmation(self):
        r=copy.deepcopy(self.record);r['checks'][0]['status']='diagnostics';result=self.result(record=r)
        self.assertEqual(result['state'],'diagnostico');self.assertIs(result['units'][0]['runtime_failure_confirmed'],False)
    def test_duplicate_rows_rejected(self):
        r=copy.deepcopy(self.record);r['checks']*=2
        with self.assertRaises(ValueError):self.result(record=r)
    def test_old_schema_rejected(self):
        r=copy.deepcopy(self.record);r['schema']=1
        with self.assertRaises(ValueError):self.result(record=r)
    def test_missing_dependencies_not_green(self):
        r=copy.deepcopy(self.record);r['checks'][0]['dependencies']=[];self.assertEqual(self.result(record=r)['state'],'pendiente')
    def test_outside_dependency_rejected(self):
        r=copy.deepcopy(self.record);r['dependencies']['project:../../outside']='a'*64
        r['checks'][0]['dependencies'].append('project:../../outside');self.assertEqual(self.result(record=r)['state'],'caducado')
    @unittest.skipUnless(sys.platform.startswith('linux'),'namespace de rutas del sistema Linux')
    def test_real_system_header_content_is_rechecked(self):
        old=v.SYSTEM_ROOT;context=self.root/'system';(context/'usr/include').mkdir(parents=True)
        header=context/'usr/include/ultra-test.h';header.write_text('#define N 42\n')
        r=copy.deepcopy(self.record);r['dependencies']['system:/usr/include/ultra-test.h']=v.digest(header)
        r['checks'][0]['dependencies'].append('system:/usr/include/ultra-test.h')
        try:
            v.SYSTEM_ROOT=context;self.assertEqual(self.result(record=r)['state'],'revisado')
            info=header.stat();header.write_text('#define N 43\n');os.utime(header,ns=(info.st_atime_ns,info.st_mtime_ns))
            self.assertEqual(self.result(record=r)['state'],'caducado')
        finally:v.SYSTEM_ROOT=old
    @unittest.skipUnless(sys.platform.startswith('linux'),'namespace de rutas del compilador Linux')
    def test_make_dependency_parent_components_normalized(self):
        dep=self.root/'deps.d';dep.write_text('unit: /project/src/../src/a.c \\\n /project/src/a.h\n')
        self.assertEqual(v.read_dependencies(dep,self.root),['project:src/a.c','project:src/a.h'])
    def test_oversized_identity_file_is_rejected(self):
        with self.assertRaises(ValueError):v.digest(self.root/'src/a.c',8)
    @unittest.skipUnless(sys.platform.startswith('linux'),'FIFO Linux')
    def test_fifo_identity_is_rejected_without_waiting(self):
        fifo=self.root/'pipe';os.mkfifo(fifo)
        with self.assertRaises(ValueError):v.digest(fifo)


if __name__=='__main__':unittest.main()
