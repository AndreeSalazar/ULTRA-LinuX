#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Render one static frame of the existing original city, during image build."""
import importlib.machinery
import importlib.util
from pathlib import Path
import sys

root=Path(__file__).resolve().parents[1]
stage=Path(sys.argv[1]).resolve(strict=True)
assert (stage/'ULTRA-SISTEMA').is_file() and stage!=Path('/')
source=root/'src/rootfs/bin/ultra-fondo-vivo'
if not source.is_file(): source=Path(__file__).parent/'rootfs/bin/ultra-fondo-vivo'
loader=importlib.machinery.SourceFileLoader('city',str(source))
spec=importlib.util.spec_from_loader(loader.name,loader);city=importlib.util.module_from_spec(spec);loader.exec_module(city)
for name,color in (('cyan','#00f0ff'),('violeta','#c03bff'),('verde','#3dff9a')):
    path=stage/'etc/ultra'/f'wallpaper-ciudad-{name}.png'
    city.ciudad3d(1920,1080,city.rgb(color),0).convert('RGB').save(path)
    path.chmod(0o644)
    print('FONDO-PREPARADO:',path.name,path.stat().st_size)
