#!/usr/bin/env python
# -*- coding: utf-8 -*-
r"""Generate a genuine RDKit `MolDraw2DSVG` panel.

RDKit's vector backend writes *unit-bearing* lengths: the root is
`width='300px' height='200px'` and every bond path carries `stroke-width:2.0px`
inside its `style`. Those lengths are numerically the user units (the root size
matches the `viewBox`), but a composer that only parses plain numbers rejected the
whole panel until the converter learned to strip the `px` suffix at a 1:1 viewport.

  python make_rdkit.py out.svg
"""

import sys

from rdkit import Chem
from rdkit.Chem.Draw import rdMolDraw2D


def main():
    out = sys.argv[1]
    mol = Chem.MolFromSmiles("Cc1ccc(C(=O)O)cc1")  # p-toluic acid
    drawer = rdMolDraw2D.MolDraw2DSVG(300, 200)
    rdMolDraw2D.PrepareAndDrawMolecule(drawer, mol)
    drawer.FinishDrawing()
    svg = drawer.GetDrawingText()
    with open(out, "w") as handle:
        handle.write(svg)
    print(f"wrote {out}: {len(svg)} bytes, {svg.count('px')} `px` occurrence(s)")


if __name__ == "__main__":
    main()
