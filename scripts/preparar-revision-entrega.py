#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Instala la fotografia revisada y su acceso en un stage de ULTRA, no en discos."""
from pathlib import Path
import argparse
import csv
import hashlib
import importlib.util
import json
import shutil

ROOT=Path(__file__).resolve().parents[1]


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('stage',type=Path);a=p.parse_args()
    stage=a.stage.resolve(strict=True)
    if not (stage/'ULTRA-SISTEMA').is_file() or stage==Path('/'):
        raise SystemExit('Stage ULTRA no identificado')
    spec=importlib.util.spec_from_file_location('revision',ROOT/'scripts/revisar-proyecto.py')
    review=importlib.util.module_from_spec(spec);spec.loader.exec_module(review)
    review.validate()
    record=review.OUT/'current.json';raw=record.read_bytes();identifier=hashlib.sha256(raw).hexdigest()[:16]
    target=stage/'usr/share/ultra/proyecto-revisado'
    target.mkdir(parents=True,exist_ok=True)
    for path in review.inputs():
        dest=target/path.relative_to(ROOT);dest.parent.mkdir(parents=True,exist_ok=True)
        shutil.copyfile(path,dest);dest.chmod(0o644)
    (target/'build/revision-proyecto').mkdir(parents=True,exist_ok=True)
    shutil.copyfile(record,target/'build/revision-proyecto/current.json')
    llvm=ROOT/'build/llvm-ub/current.json'
    if llvm.is_file():
        (target/'build/llvm-ub').mkdir(parents=True,exist_ok=True)
        shutil.copyfile(llvm,target/'build/llvm-ub/current.json')
    (target/'REVISION-ID').write_text(identifier+'\n')
    (target/'ALCANCE.txt').write_text('Fuentes C propias seleccionadas de ULTRA y CASM.\nNo incluye kernel, juegos ni fuentes de paquetes externos.\nRevisar estaticamente no certifica ausencia de UB.\n')
    with (target/'INVENTARIO.csv').open('w',newline='') as f:
        w=csv.DictWriter(f,fieldnames=['file','category']);w.writeheader();w.writerows(review.inventory())
    helper=stage/'usr/local/bin/ultra-revisar-proyecto';helper.parent.mkdir(parents=True,exist_ok=True)
    shutil.copyfile(ROOT/'src/rootfs/bin/ultra-revisar-proyecto',helper);helper.chmod(0o755)
    for directory in ['usr/share/applications','etc/ultra']:
        dest=stage/directory/'ultra-revision.desktop';dest.parent.mkdir(parents=True,exist_ok=True)
        shutil.copyfile(ROOT/'src/rootfs/desktop/ultra-revision.desktop',dest)
    shutil.copyfile(ROOT/'scripts/probar_revision_instalada.py',stage/'usr/share/ultra/probar_revision_instalada.py')
    print('REVISION-ENTREGADA',identifier)


if __name__=='__main__':main()
