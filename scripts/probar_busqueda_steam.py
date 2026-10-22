#!/usr/bin/env python3
"""Resolve existing games in selected libraries, without Steam or personal data."""
import contextlib
import importlib.util
import io
import json
from pathlib import Path
import tempfile

source = Path(__file__).resolve().parents[1]/'src/rootfs/bin/ultra-bibliotecas.py'
spec = importlib.util.spec_from_file_location('libraries', source)
module = importlib.util.module_from_spec(spec); spec.loader.exec_module(module)
with tempfile.TemporaryDirectory() as directory:
    root = Path(directory)
    module.HOME, module.TARGETS, module.CATALOG = root/'home', root/'libraries', root/'catalog.json'
    module.user_config = lambda: []
    item = module.selection({'uuid':'1234567890ABCDEF','relative':'Steam','kind':'steam'})
    library = module.TARGETS/item['id']
    item.update(status='activa',path=str(library))
    module.CATALOG.write_text(json.dumps({'folders':[item]}))
    (library/'steamapps/common/Warframe').mkdir(parents=True)
    manifest = library/'steamapps/appmanifest_230410.acf'
    # A launcher/Steam may be updating: the resolver does not claim verification.
    manifest.write_text('"AppState" { "appid" "230410" "StateFlags" "6" "installdir" "Warframe" }')
    with contextlib.redirect_stdout(io.StringIO()) as output:
        module.find_steam_app('230410')
    assert output.getvalue().strip() == str(library)
    assert not (module.HOME/'Juegos/SteamLibrary').exists()
    for invalid in ('../230410','0;id','230410\n'):
        try: module.find_steam_app(invalid)
        except ValueError: pass
        else: raise AssertionError(invalid)
    manifest.write_text('"AppState" { "appid" "230410" "installdir" "../otro" }')
    try: module.find_steam_app('230410')
    except ValueError: pass
    else: raise AssertionError('path traversal accepted')
    manifest.unlink()
    legacy = module.HOME/'Juegos/SteamLibrary'
    (legacy/'steamapps/common/Warframe').mkdir(parents=True)
    (legacy/'steamapps/appmanifest_230410.acf').write_text('"AppState" { "appid" "230410" "installdir" "Warframe" }')
    with contextlib.redirect_stdout(io.StringIO()) as output:
        module.find_steam_app('230410')
    assert output.getvalue().strip() == str(legacy)
print('VERDE: biblioteca seleccionada sin legado, actualizacion pendiente, AppID/ruta invalidos y fallback antiguo')
