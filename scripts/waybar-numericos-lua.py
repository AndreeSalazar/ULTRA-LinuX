#!/usr/bin/env python3
"""Backport numeric workspace clicks to the pinned Hyprland Lua API.

Only integer IDs enter the generated Lua. Named/special workspaces are outside
this personal desktop's numbered layout and retain their upstream behavior.
Reference: Waybar #5008 and upstream 05945748dccce28bf96d26d8f64a9e69a8dd49ba.
"""
from pathlib import Path
import sys

source=Path(sys.argv[1])/'src/modules/hyprland/workspace.cpp'
text=source.read_text()
for old,new in (
    ('"dispatch workspace " + std::to_string(id())',
     '"/dispatch hl.dsp.focus({ workspace = " + std::to_string(id()) + " })"'),
    ('"dispatch focusworkspaceoncurrentmonitor " + std::to_string(id())',
     '"/dispatch hl.dsp.focus({ workspace = " + std::to_string(id()) + ", monitor = \\\"current\\\" })"'),
):
    if text.count(old)!=1:raise SystemExit('Waybar fuente distinta; no se aplica una sustitucion ambigua')
    text=text.replace(old,new)
source.write_text(text)
print('WAYBAR-LUA: dos rutas numericas adaptadas; sin interpolar nombres de ventanas')
