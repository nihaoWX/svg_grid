#!/usr/bin/env bash
# fontlib/setup.sh — copy the faces svg_grid needs from THIS machine into a font library.
#
# The font files themselves are NOT tracked (Arial / Times New Roman are Microsoft
# fonts — shipping them in a public repo is not allowed), so a fresh checkout, or a
# new machine, runs this once:
#
#     fontlib/setup.sh                    # copy the missing faces into the DATA ROOT
#                                         #   ($HOME/svg_grid/fontlib — where svg_grid
#                                         #    looks by default)
#     fontlib/setup.sh --dir <target>     # copy into another font library
#                                         #   (e.g. one you point --font-dir at)
#     fontlib/setup.sh --dry-run          # print what WOULD be copied, touch nothing
#
# It is idempotent: a face whose target file already holds byte-identical data is
# reported as `up-to-date` and left alone. Faces are located with `fc-match`
# (fontconfig); nothing is downloaded.
set -euo pipefail

usage() {
  echo "usage: fontlib/setup.sh [--dir <target>] [--dry-run]" >&2
  echo "  --dir <target>  font library to fill (default: \$HOME/svg_grid/fontlib," >&2
  echo "                  the data root svg_grid reads when --font-dir is not given)" >&2
  echo "  --dry-run       print what would be copied, write nothing" >&2
}

dry_run=0
dir="${HOME}/svg_grid/fontlib"
while [ $# -gt 0 ]; do
  case "$1" in
    --dry-run) dry_run=1; shift ;;
    --dir)
      [ $# -ge 2 ] || { usage; exit 2; }
      dir="$2"; shift 2 ;;
    --dir=*) dir="${1#--dir=}"; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "fontlib/setup.sh: unknown argument $1" >&2; usage; exit 2 ;;
  esac
done

if [ -z "$dir" ]; then
  echo "fontlib/setup.sh: empty --dir" >&2
  exit 2
fi
if [ "$dry_run" -eq 0 ]; then
  mkdir -p "$dir"
fi
here="$(cd "$dir" 2>/dev/null && pwd || printf '%s' "$dir")"

if ! command -v fc-match >/dev/null 2>&1; then
  echo "fontlib/setup.sh: fc-match not found — install fontconfig, or copy the .ttf files into $here by hand" >&2
  exit 1
fi

families=("Arial" "Times New Roman")
styles=("Regular" "Bold" "Italic" "Bold Italic")

copied=0
up_to_date=0
missing=0

echo "fontlib/setup.sh: target directory $here"
[ "$dry_run" -eq 1 ] && echo "fontlib/setup.sh: --dry-run, nothing will be written"

for family in "${families[@]}"; do
  for style in "${styles[@]}"; do
    label="$family $style"
    # fc-match always answers: when the requested family is absent it falls back to
    # some other font. The family/style fields are compared below, so a fallback is
    # reported as `missing` instead of being copied under a wrong name.
    info="$(fc-match -f '%{file}\t%{family}\t%{style}' "$family:style=$style" 2>/dev/null || true)"
    file=""
    got_family=""
    got_style=""
    IFS=$'\t' read -r file got_family got_style <<<"$info" || true
    first_style="${got_style%%,*}"

    if [ -z "$file" ] || [ ! -f "$file" ] || [ "$got_family" != "$family" ] || [ "$first_style" != "$style" ]; then
      echo "missing      $label — not installed on this machine (fc-match offered '$got_family $first_style')"
      missing=$((missing + 1))
      continue
    fi

    ext="${file##*.}"
    target="$here/${family// /_}-${style// /_}.$ext"
    if [ -f "$target" ] && cmp -s "$file" "$target"; then
      echo "up-to-date   $label -> $(basename "$target")"
      up_to_date=$((up_to_date + 1))
      continue
    fi
    if [ "$dry_run" -eq 1 ]; then
      echo "would copy   $label <- $file  ($(basename "$target"))"
      copied=$((copied + 1))
      continue
    fi
    cp -f "$file" "$target"
    echo "copied       $label <- $file  ($(basename "$target"), $(wc -c <"$target") bytes)"
    copied=$((copied + 1))
  done
done

echo "fontlib/setup.sh: $copied copied, $up_to_date already up-to-date, $missing missing"
if [ "$missing" -gt 0 ]; then
  echo "fontlib/setup.sh: $missing face(s) unavailable — install Arial and Times New Roman" >&2
  echo "                (Debian/Ubuntu: install ttf-mscorefonts-installer) and re-run this script." >&2
  exit 1
fi
echo "fontlib/setup.sh: fontlib is complete — check it with: svg_grid plot-grid --list-fonts"
