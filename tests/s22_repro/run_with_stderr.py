#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""Run compose_s22.py with every `plot-grid` / `svg_grid_convert` stderr captured
and its byte length reported, so V3 can state the *actual* stderr volume per call
(the script itself discards stderr on success)."""
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
SCRIPT = os.path.join(HERE, "compose_s22.py")

records = []
_real = subprocess.run


def run(cmd, **kw):
    res = _real(cmd, **kw)
    if cmd and os.path.basename(cmd[0]) in ("svg_grid", "svg_grid_convert"):
        err = res.stderr or ""
        if isinstance(err, bytes):
            err = err.decode("utf-8", "replace")
        records.append((os.path.basename(cmd[0]), len(err), err))
    return res


subprocess.run = run
sys.argv = [SCRIPT]
with open(SCRIPT, encoding="utf-8") as fh:
    code = compile(fh.read(), SCRIPT, "exec")
exec(code, {"__name__": "__main__", "__file__": SCRIPT})

print("\n=== CAPTURED stderr PER CALL ===")
total = 0
for name, n, err in records:
    total += n
    print(f"  {name:18s} stderr={n:4d} bytes  {err!r}")
print(f"TOTAL plot-grid/convert stderr = {total} bytes over {len(records)} calls")
