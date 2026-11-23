import os
import sys

# Only inside a top-level evidence run: route child interpreters through the
# strict shim so a nested round cannot silently pass.
if os.environ.get("SVG_GRID_EVIDENCE_TOPLEVEL"):
    sys.executable = os.path.join(os.path.dirname(os.path.abspath(__file__)), "python3")
