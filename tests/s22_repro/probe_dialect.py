#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""Measure the SVG dialect of the panels, i.e. exactly what `convert/` must
translate before `svg_grid plot-grid` can consume them.

Reports, per file: element counts, transform kinds, path-command kinds, and
whether constructs the composer cannot handle are present.

Usage:
    micromamba run -n python-bio python probe_dialect.py [DIR]
"""

import collections
import os
import re
import sys
import xml.etree.ElementTree as ET

PATH_CMD = re.compile(r"[MmZzLlHhVvCcSsQqTtAa]")
CURVE_CMDS = set("CcSsQqTtAa")


def local(tag):
    return tag.split("}")[-1] if "}" in tag else tag


def walk(el):
    yield el
    for child in el:
        yield from walk(child)


def classify_transform(value):
    kinds = set()
    for part in re.findall(r"([a-zA-Z]+)\s*\(", value):
        kinds.add(part)
    return kinds


def probe(path):
    tree = ET.parse(path)
    root = tree.getroot()

    tags = collections.Counter()
    transforms = collections.Counter()
    path_cmds = collections.Counter()
    curve_pts = 0
    uses = 0
    images = 0
    clip = 0
    in_defs = 0

    def visit(el, depth_defs=False):
        nonlocal curve_pts, uses, images, clip, in_defs
        name = local(el.tag)
        tags[name] += 1
        inside_defs = depth_defs or name == "defs"
        if inside_defs and name not in ("defs", "clipPath", "style", "path"):
            in_defs += 1
        t = el.get("transform")
        if t:
            for k in classify_transform(t):
                transforms[k] += 1
        if name == "use":
            uses += 1
        if name == "image":
            images += 1
        if el.get("clip-path"):
            clip += 1
        if name == "path":
            d = el.get("d") or ""
            cmds = PATH_CMD.findall(d)
            path_cmds.update(cmds)
            curve_pts += sum(1 for c in cmds if c in CURVE_CMDS)
        for child in el:
            visit(child, inside_defs)

    visit(root)
    return dict(tags=tags, transforms=transforms, path_cmds=path_cmds,
                curve_cmds=curve_pts, uses=uses, images=images, clip=clip,
                in_defs=in_defs)


def main(directory):
    files = sorted(f for f in os.listdir(directory) if f.endswith(".svg"))
    if not files:
        print("no .svg in", directory)
        return 1
    print(f"{'file':44s} {'size':>8s} {'path':>5s} {'curve':>6s} {'use':>5s} "
          f"{'image':>6s} {'text':>5s} {'rect':>5s} {'line':>5s} {'g':>5s} "
          f"{'clip':>5s} {'defs*':>6s}")
    totals = collections.Counter()
    for f in files:
        p = probe(os.path.join(directory, f))
        t = p["tags"]
        print(f"{f:44s} {os.path.getsize(os.path.join(directory, f)):8d} "
              f"{t.get('path', 0):5d} {p['curve_cmds']:6d} {p['uses']:5d} "
              f"{p['images']:6d} {t.get('text', 0):5d} {t.get('rect', 0):5d} "
              f"{t.get('line', 0):5d} {t.get('g', 0):5d} {p['clip']:5d} "
              f"{p['in_defs']:6d}")
        totals.update(p["tags"])
        totals.update({f"curve:{k}": v for k, v in p["path_cmds"].items()})
        totals.update({f"tf:{k}": v for k, v in p["transforms"].items()})
        totals["use"] += p["uses"]
        totals["image"] += p["images"]
        totals["clip"] += p["clip"]

    print("\n--- aggregate ---")
    interesting = ["path", "text", "rect", "line", "polyline", "polygon", "circle",
                   "g", "defs", "clipPath", "use", "image", "style"]
    for k in interesting:
        if totals.get(k):
            print(f"  {k:12s} {totals[k]}")
    print("  transforms  :", {k[3:]: v for k, v in totals.items() if k.startswith("tf:")})
    print("  path cmds   :", {k[5:]: v for k, v in sorted(totals.items())
                              if k.startswith("curve:") and not k.startswith("curve:p")})
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1] if len(sys.argv) > 1 else
                  os.path.join(os.path.dirname(os.path.abspath(__file__)), "out", "panels")))
