#!/usr/bin/env python3
"""Expand a verified, empty ext4 image into a new ordinary Windows file.
Never opens a physical device, overwrites an existing image or partitions disks.
"""
import argparse
import gzip
import hashlib
import json
import os
from pathlib import Path
import shutil


def reserve_windows(output, size):
    # CRT truncate can extend large files by issuing tiny zero writes. Use the
    # native length operation; NTFS still supplies zeroes for unwritten data.
    import ctypes
    import msvcrt
    kernel = ctypes.WinDLL('kernel32', use_last_error=True)
    kernel.SetFilePointerEx.argtypes = (ctypes.c_void_p, ctypes.c_longlong, ctypes.c_void_p, ctypes.c_ulong)
    kernel.SetFilePointerEx.restype = ctypes.c_int
    kernel.SetEndOfFile.argtypes = (ctypes.c_void_p,)
    kernel.SetEndOfFile.restype = ctypes.c_int
    handle = msvcrt.get_osfhandle(output.fileno())
    if not kernel.SetFilePointerEx(handle, size, None, 0) or not kernel.SetEndOfFile(handle):
        raise ctypes.WinError(ctypes.get_last_error())
    if not kernel.SetFilePointerEx(handle, 0, None, 0):
        raise ctypes.WinError(ctypes.get_last_error())


def main():
    if os.name != 'nt':
        raise RuntimeError('Este instalador de archivos requiere Windows')
    parser = argparse.ArgumentParser()
    parser.add_argument('--archivo', type=Path, required=True)
    parser.add_argument('--destino', type=Path, required=True)
    parser.add_argument('--sha256', required=True)
    parser.add_argument('--gzip-sha256', required=True)
    args = parser.parse_args()
    size = 16 * 1024**3
    dest = args.destino.resolve()
    if dest.name != 'ULTRA-juegos.ext4' or not dest.parent.is_dir() or dest.exists():
        raise ValueError('Se requiere un archivo nuevo llamado ULTRA-juegos.ext4')
    if shutil.disk_usage(dest.parent).free < size + 2 * 1024**3:
        raise ValueError('Se requieren 16 GiB mas 2 GiB de margen libres')
    with args.archivo.open('rb') as source:
        if hashlib.file_digest(source, 'sha256').hexdigest() != args.gzip_sha256.lower():
            raise ValueError('SHA256 del archivo comprimido distinto')
    temp = dest.with_suffix('.ext4.nuevo')
    digest = hashlib.sha256()
    total = 0
    with temp.open('xb') as output, gzip.open(args.archivo, 'rb') as source:
        reserve_windows(output, size)
        while chunk := source.read(4 * 1024**2):
            total += len(chunk)
            if total > size:
                raise ValueError('Imagen mayor de lo esperado')
            digest.update(chunk)
            if chunk.strip(b'\0'):
                output.write(chunk)
            else:
                output.seek(len(chunk), 1)
        output.flush()
        os.fsync(output.fileno())
    if total != size or digest.hexdigest() != args.sha256.lower():
        raise ValueError('Tamano o SHA256 ext4 incorrecto; no se publica la imagen')
    with temp.open('rb') as source:
        if hashlib.file_digest(source, 'sha256').hexdigest() != args.sha256.lower():
            raise ValueError('La copia escrita no coincide')
    # Windows rename refuses replacement if another process created dest meanwhile.
    if os.name != 'nt':
        raise RuntimeError('Este instalador de archivos requiere Windows')
    temp.rename(dest)
    print(json.dumps({'archivo': str(dest), 'bytes': size,
                      'sha256': digest.hexdigest(), 'particiones_modificadas': False}, indent=2))


if __name__ == '__main__':
    main()
