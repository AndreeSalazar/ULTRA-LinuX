#!/usr/bin/env python3
"""ultra-usb supervisor on a fake sysfs: no real disks are probed or mounted."""
import importlib.machinery
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time
import unittest
from unittest.mock import patch

SOURCE = Path(__file__).resolve().parents[1]/'src/rootfs/bin/ultra-usb'
loader = importlib.machinery.SourceFileLoader('ultra_usb', str(SOURCE))
spec = importlib.util.spec_from_loader(loader.name, loader)
app = importlib.util.module_from_spec(spec); loader.exec_module(app)


class Fake:
    """Records commands; blkid answers from a table, mount can be refused."""
    def __init__(self):
        self.calls, self.props, self.refuse, self.busy = [], {}, set(), set()

    def __call__(self, *args, **kwargs):
        self.calls.append(args)
        if args[0].endswith('blkid'):
            props = self.props[args[-1].rsplit('/', 1)[1]]
            return ''.join(f'{k}={v}\n' for k, v in props.items())
        if args[0].endswith('/mount') and args[6] in self.refuse:
            raise subprocess.CalledProcessError(32, args)
        if args[0].endswith('umount') and '-l' not in args and args[-1] in self.busy:
            raise subprocess.CalledProcessError(32, args)
        return ''


class Supervisor(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        root = Path(self.temp.name)
        self.sys = root/'sys'; self.media = root/'media'; self.media.mkdir()
        self.mountinfo = root/'mountinfo'; self.mountinfo.write_text('')
        self.fake = Fake()
        self.disks = []
        self.patches = [patch.object(app, 'SYS', self.sys), patch.object(app, 'MEDIA', self.media),
                        patch.object(app, 'MOUNTINFO', self.mountinfo), patch.object(app, 'run', self.fake),
                        patch.object(app, 'usb_disks', lambda: iter(self.disks)), patch.object(app.os, 'sync', lambda: None, create=True)]
        for p in self.patches: p.start()
        self.sup = app.Supervisor()

    def tearDown(self):
        for p in reversed(self.patches): p.stop()
        self.temp.cleanup()

    def disk(self, name, partitions, majmin='8:16'):
        disk = self.sys/'block'/name; disk.mkdir(parents=True)
        (disk/'dev').write_text(majmin+'\n'); (disk/'ro').write_text('0\n')
        for index, (props, size) in enumerate(partitions, 1):
            part = disk/f'{name}{index}'; part.mkdir()
            (part/'partition').write_text(f'{index}\n'); (part/'size').write_text(f'{size}\n')
            (part/'dev').write_text(f'{majmin.split(":")[0]}:{int(majmin.split(":")[1])+index}\n')
            self.fake.props[part.name] = props
        self.disks.append(disk)
        return disk

    def mounts(self):
        return [c for c in self.fake.calls if c[0].endswith('/mount')]

    def test_vfat_stick_mounted_writable_for_user_without_exec(self):
        self.disk('sdb', [({'TYPE': 'vfat', 'UUID': 'AB12-CD34', 'LABEL': 'MI USB'}, 2048)])
        self.sup.scan()
        (call,) = self.mounts()
        opts = call[call.index('-o')+1].split(',')
        self.assertEqual(call[3], 'vfat')
        self.assertTrue({'rw', 'nosuid', 'nodev', 'noexec', 'uid=1000', 'gid=1000', 'flush'} <= set(opts))
        entry = self.sup.ours['sdb1']
        self.assertEqual((entry['nombre'], entry['ruta'], entry['bytes'], entry['escritura']),
                         ('MI_USB', str(self.media/'MI_USB'), 2048*512, True))
        self.sup.scan()
        self.assertEqual(len(self.mounts()), 1, 'no remount on later scans')

    def test_ntfs_uses_ntfs3_and_falls_back_to_read_only(self):
        self.disk('sdb', [({'TYPE': 'ntfs', 'UUID': '01D9', 'LABEL': 'Win'}, 10)])
        original = self.fake.__call__
        def refuse_rw(*args, **kw):
            if args[0].endswith('/mount') and args[args.index('-o')+1].startswith('rw'):
                self.fake.calls.append(args); raise subprocess.CalledProcessError(32, args)
            return original(*args, **kw)
        with patch.object(app, 'run', refuse_rw): self.sup.scan()
        self.assertEqual([c[3] for c in self.mounts()], ['ntfs3', 'ntfs3'])
        self.assertFalse(self.sup.ours['sdb1']['escritura'])

    def test_disk_in_use_elsewhere_is_never_touched(self):
        self.disk('sdc', [({'TYPE': 'ext4', 'UUID': 'x', 'LABEL': 'datos'}, 10)], '8:32')
        self.mountinfo.write_text('30 1 8:33 / / rw - ext4 /dev/sdc1 rw\n')
        self.sup.scan()
        self.assertEqual(self.fake.calls, [], 'not even probed')

    def test_holders_block_the_disk(self):
        disk = self.disk('sdc', [({'TYPE': 'vfat', 'UUID': 'x'}, 10)], '8:32')
        (disk/'sdc1'/'holders'/'loop0').mkdir(parents=True)
        self.sup.scan()
        self.assertEqual(self.mounts(), [])

    def test_system_labels_efi_and_unknown_filesystems_skipped(self):
        self.disk('sdb', [({'TYPE': 'ext4', 'UUID': '1', 'LABEL': 'ULTRA-DATOS'}, 10),
                          ({'TYPE': 'vfat', 'UUID': '2', 'PART_ENTRY_TYPE': app.EFI.upper()}, 10),
                          ({'TYPE': 'btrfs', 'UUID': '3'}, 10)])
        self.sup.scan(); self.sup.scan()
        self.assertEqual(self.mounts(), [])
        self.assertEqual(sum(c[0].endswith('blkid') for c in self.fake.calls), 3, 'refusals are remembered')

    def test_failed_mount_reported_once_and_folder_removed(self):
        self.disk('sdb', [({'TYPE': 'exfat', 'UUID': 'E1', 'LABEL': 'Roto'}, 10)])
        self.fake.refuse.add('/dev/sdb1')
        self.sup.scan()
        self.assertEqual(len(self.mounts()), 2, 'rw then ro')
        self.sup.scan()
        self.assertEqual(len(self.mounts()), 2, 'no retry until replugged')
        self.assertFalse((self.media/'Roto').exists())

    def test_unplug_without_eject_lazy_unmounts(self):
        disk = self.disk('sdb', [({'TYPE': 'vfat', 'UUID': 'A', 'LABEL': 'X'}, 10)])
        self.sup.scan()
        self.disks.remove(disk)
        self.sup.scan()
        self.assertIn(('/usr/bin/umount', '-l', str(self.media/'X')), self.fake.calls)
        self.assertEqual(self.sup.ours, {})

    def test_eject_busy_then_ok_and_no_remount_until_replugged(self):
        disk = self.disk('sdb', [({'TYPE': 'vfat', 'UUID': 'A', 'LABEL': 'X'}, 10)])
        self.sup.scan()
        self.fake.busy.add(str(self.media/'X'))
        with self.assertRaisesRegex(ValueError, 'en uso'): self.sup.eject('sdb1')
        self.assertIn('sdb1', self.sup.ours)
        self.fake.busy.clear()
        self.sup.eject('sdb1')
        self.sup.scan()
        self.assertEqual((self.sup.ours, self.sup.ejected), ({}, {'sdb1'}))
        self.assertEqual(len(self.mounts()), 1)
        self.disks.remove(disk); self.sup.scan()
        self.assertEqual(self.sup.ejected, set())
        self.disks.append(disk); self.sup.scan()
        self.assertEqual(len(self.mounts()), 2, 'replugged stick mounts again')

    def test_eject_only_devices_we_mounted(self):
        with self.assertRaises(ValueError): self.sup.eject('sda1')

    def test_duplicate_labels_get_distinct_folders(self):
        self.disk('sdb', [({'TYPE': 'vfat', 'UUID': 'A', 'LABEL': 'USB'}, 10)])
        self.disk('sdc', [({'TYPE': 'vfat', 'UUID': 'B', 'LABEL': 'USB'}, 10)], '8:32')
        self.sup.scan()
        self.assertEqual(sorted(e['nombre'] for e in self.sup.ours.values()), ['USB', 'USB-2'])


class Requests(unittest.TestCase):
    def write(self, doc):
        self.temp = tempfile.TemporaryDirectory(); path = Path(self.temp.name)/'p'
        path.write_text(json.dumps(doc)); os.chmod(path, 0o600)
        self.addCleanup(self.temp.cleanup)
        return path

    def decode(self, doc, now):
        path = self.write(doc)
        stat_result = os.stat(path)
        fake = os.stat_result((0o100600, *stat_result[1:4], 1000, *stat_result[5:]))
        with patch.object(app.os, 'fstat', return_value=fake):
            return app.decode_request(path, now)

    def test_valid_and_invalid_requests(self):
        good = {'accion': 'expulsar', 'id': 'a'*32, 'monotonic': 100.0, 'dispositivo': 'sdb1'}
        self.assertEqual(self.decode(good, 101)['dispositivo'], 'sdb1')
        for bad in (dict(good, dispositivo='../sda'), dict(good, dispositivo='nvme0n1p1'),
                    dict(good, accion='formatear'), dict(good, extra=1)):
            with self.assertRaises(ValueError): self.decode(bad, 101)
        with self.assertRaisesRegex(ValueError, 'caducada'): self.decode(good, 200)


if __name__ == '__main__': unittest.main(verbosity=2)
