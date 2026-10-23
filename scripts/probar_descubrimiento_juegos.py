#!/usr/bin/env python3
"""Disposable folder/manifest fixtures; no real accounts, games or mounts."""
import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

source = Path(__file__).resolve().parents[1]/'src/rootfs/bin/ultra-bibliotecas.py'
spec = importlib.util.spec_from_file_location('bibliotecas', source)
lib = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lib)


class Discovery(unittest.TestCase):
    def test_gog_uses_prepared_folder_without_account_writes(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            item = lib.selection(dict(uuid='1234-ABCD', relative='GOG Games', kind='folder'))
            folder = root/item['id']
            folder.mkdir()
            record = dict(item, path=str(folder), status='activa')
            with patch.object(lib, 'user_config', return_value=[item]), \
                    patch.object(lib, 'read_json', return_value=dict(folders=[record])), \
                    patch.object(lib, 'TARGETS', root), \
                    patch.object(lib, 'choose', return_value=record), \
                    patch.object(lib.subprocess, 'Popen') as launch, \
                    patch.object(lib.subprocess, 'run'):
                lib.import_gog()
                self.assertEqual(launch.call_args_list[0].args[0], ['thunar', str(folder)])
                self.assertEqual(launch.call_args_list[1].args[0],
                                 ['/opt/heroic/heroic', '--ozone-platform=wayland'])
                record['path'] = '/etc'
                with self.assertRaises(ValueError):
                    lib.import_gog()
                self.assertEqual(launch.call_count, 2)

    def test_parent_and_multiple_steam_libraries(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            for name in ('Games/Steam', 'Games/More/SteamLibrary'):
                (root/name/'steamapps').mkdir(parents=True)
            item = lib.selection(dict(uuid='1234-ABCD', relative='Games', kind='folder'))
            volumes = [dict(uuid=item['uuid'], path=str(root), status='lectura')]
            found = lib.discover_selections([item], volumes)
            self.assertEqual({r['relative'] for r in found},
                             {'Games', 'Games/Steam', 'Games/More/SteamLibrary'})
            self.assertEqual(len(lib.discover_selections(found, volumes)), 3)
            volumes[0]['status'] = 'no disponible'
            self.assertEqual(lib.discover_selections([item], volumes), [item])

    def test_no_symbolic_or_deep_search(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root/'Games/a/b/c/d/steamapps').mkdir(parents=True)
            (root/'Outside/steamapps').mkdir(parents=True)
            (root/'Games/link').symlink_to(root/'Outside', target_is_directory=True)
            item = lib.selection(dict(uuid='1234-ABCD', relative='Games', kind='folder'))
            self.assertEqual(lib.discover_selections([item], [dict(uuid=item['uuid'], path=str(root), status='lectura')]), [item])

    def test_installed_plus_update_flags_registered(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            old_targets = lib.TARGETS
            lib.TARGETS = root/'views'
            try:
                item = lib.selection(dict(uuid='1234-ABCD', relative='Games/Steam', kind='steam'))
                library = lib.TARGETS/item['id']
                (library/'steamapps/common/Test').mkdir(parents=True)
                (library/'steamapps/appmanifest_22.acf').write_text(
                    '"AppState" { "appid" "22" "name" "Test" "SizeOnDisk" "123" '
                    '"StateFlags" "6" "installdir" "Test" }')
                dest = root/'libraryfolders.vdf'
                lib.register_steam_file(dest, root/'missing', root/'Steam',
                    dict(folders=[dict(item, path=str(library), status='activa')]))
                data = lib.read_vdf(dest)['libraryfolders']
                self.assertEqual(data['1']['apps'], {'22': '123'})
                # Registration retains flags; it does not pretend an update is complete.
                self.assertEqual(lib.read_vdf(library/'steamapps/appmanifest_22.acf')['AppState']['StateFlags'], '6')
            finally:
                lib.TARGETS = old_targets


if __name__ == '__main__':
    unittest.main(verbosity=2)
