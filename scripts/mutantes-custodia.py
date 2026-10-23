#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Produce isolated weakened test copies; never edits the original source."""
import hashlib
import json
from pathlib import Path
import sys

source, out = Path(sys.argv[1]), Path(sys.argv[2])
text = source.read_text()
out.mkdir(exist_ok=False)
cases = {
    'titular': ('if (loan->subject != subject) return UC_DENIED;', 2),
    'bytes': ('if (size > c->max_bytes - c->used_bytes) return UC_QUOTA;', 1),
    'retirada': ('if (loan->retiring) return UC_RETIRING;', 1),
    'cola': ('if (c->pending >= c->max_requests) return UC_QUOTA;', 1),
}
records = []
for name, (guard, count) in cases.items():
    if text.count(guard) != count:
        raise ValueError('mutation site changed: '+name)
    changed = text.replace(guard, '/* Laboratory mutation: '+name+' guard removed. */')
    p = out/(name+'.c')
    p.write_text(changed)
    records.append(dict(Name=name, Removed=guard, Count=count,
                        SHA256=hashlib.sha256(p.read_bytes()).hexdigest()))
(out/'mutaciones.json').write_text(json.dumps(dict(
    SourceSHA256=hashlib.sha256(source.read_bytes()).hexdigest(), Mutations=records), indent=2)+'\n')
