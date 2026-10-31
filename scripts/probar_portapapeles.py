#!/usr/bin/env python3
"""Disposable SQLite history: pins, byte budget, images, restart and concurrency."""
from concurrent.futures import ThreadPoolExecutor
import importlib.machinery
import importlib.util
import io
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

ROOT=Path(__file__).resolve().parents[1]
loader=importlib.machinery.SourceFileLoader('clipboard',str(ROOT/'src/rootfs/bin/ultra-portapapeles'))
spec=importlib.util.spec_from_loader(loader.name,loader);m=importlib.util.module_from_spec(spec);loader.exec_module(m)


@unittest.skipUnless(os.name=='posix','Linux private files')
class History(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory();self.home=Path(self.tmp.name).resolve();self.db=m.database(self.home)
    def tearDown(self):self.db.close();self.tmp.cleanup()
    def test_more_than_200_items_and_pagination(self):
        for n in range(250):m.store(self.db,f'copia {n}'.encode())
        first=m.list_entries(self.db);self.assertEqual(first['total'],250)
        self.assertEqual(len(first['entries']),30);self.assertTrue(first['more'])
        self.assertEqual(len(m.list_entries(self.db,page=8)['entries']),10)
    def test_pin_deduplicate_clear_and_reopen(self):
        m.store(self.db,b'conservar');entry=m.list_entries(self.db)['entries'][0]['id'];m.pin(self.db,entry)
        m.store(self.db,b'conservar');m.store(self.db,b'temporal');m.delete(self.db)
        self.assertEqual(m.list_entries(self.db)['total'],1)
        self.db.close();self.db=m.database(self.home)
        row=m.list_entries(self.db)['entries'][0]
        self.assertTrue(row['pinned']);self.assertEqual(row['id'],entry)
        m.delete(self.db,entry);self.assertEqual(m.list_entries(self.db)['pins'],1)
    def test_eviction_preserves_pins_and_bounds_database(self):
        with self.db:m.set_option(self.db,'quota_mib',1)
        m.set_page_budget(self.db)
        m.store(self.db,b'pin');entry=m.list_entries(self.db)['entries'][0]['id'];m.pin(self.db,entry)
        for n in range(100):m.store(self.db,(str(n)+'x'*20000).encode())
        self.assertTrue(self.db.execute('SELECT pinned FROM entries WHERE id=?',(entry,)).fetchone()[0])
        self.assertLessEqual(m.option(self.db,'used'),1024**2)
        path=self.home/'.local/share/ultra/portapapeles/historial.sqlite3'
        self.assertLessEqual(path.stat().st_size,1024**2)
    def test_pins_full_rolls_back_without_deleting_anything(self):
        with self.db:m.set_option(self.db,'quota_mib',1)
        m.store(self.db,b'p'*900000);entry=m.list_entries(self.db)['entries'][0]['id'];m.pin(self.db,entry)
        m.store(self.db,b'u'*40000);before=self.db.execute('SELECT id,cost FROM entries').fetchall()
        with self.assertRaises(ValueError):m.store(self.db,b'n'*1024**2)
        self.assertEqual(before,self.db.execute('SELECT id,cost FROM entries').fetchall())
        self.assertEqual(m.option(self.db,'used'),sum(r[1] for r in before))
    def test_images_are_exact_and_preview_is_bounded(self):
        from PIL import Image
        out=io.BytesIO();Image.new('RGBA',(32,24),(0,240,255,255)).save(out,format='PNG');raw=out.getvalue()
        m.store(self.db,raw);row=m.list_entries(self.db)['entries'][0]
        self.assertEqual(row['mime'],'image/png');self.assertEqual(row['thumbnail']['width'],32)
        self.assertEqual(self.db.execute('SELECT content FROM entries').fetchone()[0],raw)
        self.assertLessEqual(len(row['thumbnail']['rgba']),96*64*4)
        self.assertFalse(m.store(self.db,b'\0\xffunknown'))
    def test_sensitive_pause_and_payload_limits(self):
        self.assertFalse(m.store(self.db,b'no guardar',sensitive=True))
        with self.db:m.set_option(self.db,'paused',1)
        self.assertFalse(m.store(self.db,b'pausado'))
        with self.db:m.set_option(self.db,'paused',0)
        with self.assertRaises(ValueError):m.store(self.db,b'x'*(m.MAX_ITEM+1))
        with patch.object(m.os,'geteuid',return_value=1000),patch.dict(m.os.environ,{'CLIPBOARD_STATE':'sensitive'}),patch.object(m.sys,'argv',['clip','guardar']),patch.object(m,'database') as database:
            m.main();database.assert_not_called()
        self.assertEqual(m.list_entries(self.db)['total'],0)
    def test_sql_text_is_data_and_ids_never_recycle(self):
        data=b"'; DROP TABLE entries; -- $(touch no-ejecutar)"
        m.store(self.db,data);entry=m.list_entries(self.db)['entries'][0]['id']
        self.assertEqual(m.list_entries(self.db,"DROP TABLE")['entries'][0]['id'],entry)
        m.delete(self.db,entry);m.store(self.db,b'nuevo')
        self.assertGreater(m.list_entries(self.db)['entries'][0]['id'],entry)
        self.assertFalse((self.home/'no-ejecutar').exists())
    def test_private_files_and_reject_symlinks(self):
        folder=self.home/'.local/share/ultra/portapapeles'
        self.assertEqual(folder.stat().st_mode & 0o777,0o700)
        self.assertEqual((folder/'historial.sqlite3').stat().st_mode & 0o777,0o600)
        other=self.home/'other';other.mkdir();(other/'.local').symlink_to(folder)
        with self.assertRaises(ValueError):m.database(other)
    def test_concurrent_first_copies_are_preserved(self):
        home=self.home/'concurrent'
        def write(n):
            db=m.database(home)
            try:m.store(db,('paralelo '+str(n)).encode())
            finally:db.close()
        with ThreadPoolExecutor(max_workers=4) as pool:list(pool.map(write,range(12)))
        db=m.database(home)
        try:self.assertEqual(m.list_entries(db)['total'],12)
        finally:db.close()


if __name__=='__main__':unittest.main(verbosity=2)
