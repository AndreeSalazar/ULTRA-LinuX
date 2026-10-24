#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Build original SVG icons and multiresolution XCursor assets. No downloads.
Theme name stays ULTRA-Nucleo for compatibility; colors follow BMO-X / Neon (hielo).
Safe to run again over an existing image: links are replaced."""
from pathlib import Path
import subprocess
import sys
import tempfile

root=Path(sys.argv[1]).resolve()

def enlace(link,target):
    if link.is_symlink() or link.exists(): link.unlink()
    link.symlink_to(target)

theme=root/'usr/share/icons/ULTRA-Nucleo'
icons=theme/'scalable'; icons.mkdir(parents=True,exist_ok=True)
(theme/'index.theme').write_text('[Icon Theme]\nName=ULTRA Nucleo\nComment=Neon: black, ice blue and red\nInherits=Papirus-Dark,Adwaita\nDirectories=scalable\n\n[scalable]\nSize=48\nType=Scalable\nMinSize=16\nMaxSize=256\n')
symbols={
 'utilities-terminal':'<path d="m15 18 9 6-9 6m14 1h8"/>',
 'folder':'<path d="M10 17h12l4 4h13v15H10Z"/><path d="M10 17v-4h12l4 4"/>',
 'computer':'<rect x="11" y="12" width="27" height="20" rx="2"/><path d="M24 32v6m-8 0h16"/>',
 'preferences-system':'<circle cx="24" cy="24" r="9"/><circle cx="24" cy="24" r="3"/><path d="M24 10v5m0 18v5M10 24h5m18 0h5M14 14l4 4m12 12 4 4m0-20-4 4M18 30l-4 4"/>',
 'web-browser':'<circle cx="24" cy="24" r="13"/><ellipse cx="24" cy="24" rx="6" ry="13"/><path d="M11 24h26m-23-7h20m-20 14h20"/>',
 'applications-games':'<path d="M14 18h20l6 17-5 2-8-8h-6l-8 8-5-2Z"/><path d="M15 22v8m-4-4h8m12-3h1m3 5h1"/>',
 'help-browser':'<path d="M18 18a6 6 0 1 1 9 5c-3 1-3 3-3 5m0 5v1"/>',
 'text-editor':'<path d="m15 31 16-18 5 5-16 18-7 2Z"/><path d="m28 17 5 5M12 39h25"/>',
 'audio-volume-high':'<path d="M11 20h6l8-7v23l-8-7h-6Z"/><path d="M30 19q6 5 0 10m5-15q11 10 0 20"/>',
 'video-display':'<rect x="10" y="12" width="29" height="22" rx="2"/><path d="m18 26 5-8 6 11 4-6m-9 11v5m-7 0h14"/>',
}
for name,shape in symbols.items():
    svg=f'<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 48 48"><path d="M6 3h31l8 8v29l-5 5H8l-5-5V6Z" fill="#05070b" stroke="#2a2733"/><path d="M7 5h28" stroke="#ff2a4f"/><g fill="none" stroke="#5ad8ff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">{shape}</g><path d="M9 43h14" stroke="#00f0ff"/></svg>'
    (icons/(name+'.svg')).write_text(svg)
for alias,target in {'user-home':'folder','folder-documents':'folder','folder-download':'folder',
                     'folder-games':'applications-games','brave-desktop':'web-browser',
                     'steam':'applications-games','heroic':'applications-games'}.items():
    enlace(icons/(alias+'.svg'),target+'.svg')
cursors=theme/'cursors'; cursors.mkdir(exist_ok=True)
shapes={
 'left_ptr':('<path d="M4 3v24l6-7 5 11 5-2-5-11h10Z" fill="#05070b" stroke="#5ad8ff" stroke-width="1.7"/><path d="m7 8 10 9h-5" fill="none" stroke="#ff2a4f" stroke-width="1.4"/>',(4,3)),
 'hand2':('<path d="M12 17V6a3 3 0 0 1 6 0v10-3a3 3 0 0 1 5 0 3 3 0 0 1 5 1v8c0 5-4 8-9 8-4 0-7-3-9-6l-5-7q1-5 7 0Z" fill="#05070b" stroke="#5ad8ff" stroke-width="1.6"/><path d="M16 28h8" stroke="#ff2a4f"/>',(15,4)),
 'xterm':('<path d="M10 4h12M16 4v24m-6 0h12" stroke="#05070b" stroke-width="5"/><path d="M10 4h12M16 4v24m-6 0h12" stroke="#5ad8ff" stroke-width="2"/><path d="M13 16h6" stroke="#ff2a4f" stroke-width="2"/>',(16,16)),
 'crosshair':('<path d="M16 3v9m0 8v9M3 16h9m8 0h9" stroke="#05070b" stroke-width="5"/><path d="M16 3v9m0 8v9M3 16h9m8 0h9" stroke="#5ad8ff" stroke-width="2"/><circle cx="16" cy="16" r="2" fill="#ff2a4f"/>',(16,16)),
 'watch':('<circle cx="16" cy="16" r="11" fill="#05070b" stroke="#2a2733" stroke-width="2"/><path d="M16 5a11 11 0 0 1 11 11" fill="none" stroke="#5ad8ff" stroke-width="3"/><path d="M16 10v7l5 3" fill="none" stroke="#ff2a4f" stroke-width="2"/>',(16,16)),
 'not-allowed':('<circle cx="16" cy="16" r="11" fill="#05070b" stroke="#ff2a4f" stroke-width="2"/><path d="m8 24 16-16" stroke="#5ad8ff" stroke-width="3"/>',(16,16)),
 'move':('<path d="M16 3v26M3 16h26m-18-8 5-5 5 5M11 24l5 5 5-5M8 11l-5 5 5 5m16-10 5 5-5 5" fill="none" stroke="#5ad8ff" stroke-width="2"/><circle cx="16" cy="16" r="3" fill="#ff2a4f"/>',(16,16)),
}
for name,rotation in [('sb_h_double_arrow',0),('sb_v_double_arrow',90),('size_fdiag',45),('size_bdiag',-45)]:
    path='<path d="M4 16h24m-18-6-6 6 6 6m12-12 6 6-6 6"/>'
    shapes[name]=(f'<g transform="rotate({rotation} 16 16)" fill="none" stroke="#05070b" stroke-width="5">{path}</g><g transform="rotate({rotation} 16 16)" fill="none" stroke="#5ad8ff" stroke-width="2">{path}</g>',(16,16))
with tempfile.TemporaryDirectory(prefix='ultra-cursor-') as temp:
    temp=Path(temp)
    for name,(body,hotspot) in shapes.items():
        svg=temp/(name+'.svg'); svg.write_text('<svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" viewBox="0 0 32 32">'+body+'</svg>')
        entries=[]
        for size in (24,28,32,48,64):
            png=temp/f'{name}-{size}.png'
            subprocess.run(['rsvg-convert','-w',str(size),'-h',str(size),'-o',str(png),str(svg)],check=True)
            x,y=(round(v*size/32) for v in hotspot)
            entries.append(f'{size} {x} {y} {png} 100')
        conf=temp/(name+'.conf'); conf.write_text('\n'.join(entries)+'\n')
        subprocess.run(['xcursorgen',str(conf),str(cursors/name)],check=True)
aliases={'default':'left_ptr','arrow':'left_ptr','top_left_arrow':'left_ptr','pointer':'hand2','hand1':'hand2',
 'text':'xterm','vertical-text':'xterm','wait':'watch','progress':'watch','left_ptr_watch':'watch',
 'forbidden':'not-allowed','cross':'crosshair','all-scroll':'move','fleur':'move','grab':'hand2','grabbing':'hand2',
 'ew-resize':'sb_h_double_arrow','e-resize':'sb_h_double_arrow','w-resize':'sb_h_double_arrow','col-resize':'sb_h_double_arrow',
 'ns-resize':'sb_v_double_arrow','n-resize':'sb_v_double_arrow','s-resize':'sb_v_double_arrow','row-resize':'sb_v_double_arrow',
 'nwse-resize':'size_fdiag','nw-resize':'size_fdiag','se-resize':'size_fdiag',
 'nesw-resize':'size_bdiag','ne-resize':'size_bdiag','sw-resize':'size_bdiag'}
for alias,target in aliases.items(): enlace(cursors/alias,target)
print('VERDE [TEMA]: iconos SVG originales y 11 cursores XCursor en cinco tamanos, colores Neon hielo')
