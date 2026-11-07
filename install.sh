#!/usr/bin/env bash
# install.sh — the one command that puts svg_grid on this machine.
#
# Contract: ARCHITECTURE/install.md. Two artifacts, both idempotent, no symlinks:
#   1) the ONE binary `svg_grid` — release-built from this crate (`convert/` is compiled
#      INTO it as a library, so `svg_grid convert …` is a subcommand, not a second
#      executable), copied into <prefix> (default ~/.local/bin);
#   2) the skill that ships with it — the using side only (`SKILL.md` + `references/`),
#      copied into the harness' skill dir: `~/.commandcode/skills/general-figure-guide/`
#      by default, `<dir>/.commandcode/skills/…` with `--project <dir>` (copies, never
#      symlinks: an external user has no dev tree to link back into).
# Data (fixture corpus + font library) goes to `$HOME/svg_grid/` — the same data root a
# `cargo run` uses. Re-run this after every change to the code OR the skill.
#
#   install.sh [--prefix <dir>] [--project <dir>] [--no-skill] [--no-fixtures]
#              [--no-fonts] [--no-build] [--dry-run] [--check] [--help]
#
# Output is English, one fact per line. A failure exits non-zero with one sentence plus
# a `Fix:` line. `--dry-run` writes nothing (R15 asserts exactly that).
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
main_crate="$here"
main_bin="$main_crate/target/release/svg_grid"
fixtures_src="$here/tests/fixtures"
font_script="$here/fontlib/setup.sh"
# The skill shipped with the tool: using side only. Whitelist, never the whole dir —
# ARCHITECTURE/, src/, docs/ would lead a using agent into development docs.
skill_src="$here/docs/general-figure-guide"
skill_files="SKILL.md references"
# The converter used to be installed next to svg_grid; it is built in now, so an
# older copy left in <prefix> would shadow nothing but confuse `command -v`. It is
# removed once and said out loud (never silently).
stale_convert="svg_grid_convert"

usage() {
  cat <<'EOF'
usage: install.sh [--prefix <dir>] [--project <dir>] [--no-skill] [--no-fixtures]
                  [--no-fonts] [--no-build] [--dry-run] [--check] [--help]

Build svg_grid (release), copy that ONE binary into <prefix>, install the skill
(general-figure-guide: SKILL.md + references/ only) into the harness' skill dir, then
sync the fixture corpus and the font library into the data root $HOME/svg_grid. The
dialect converter is compiled into svg_grid (`svg_grid convert …`), so no second
executable is installed — an older `svg_grid_convert` left in <prefix> by a previous
install is removed and reported. Safe to re-run: the binary is overwritten, identical
skill files and font files are left alone.

  --prefix <dir>   where the binary goes (default: $HOME/.local/bin)
  --project <dir>  install the skill into <dir>/.commandcode/skills/ instead of the
                   global $HOME/.commandcode/skills (the binary still goes to <prefix>)
  --no-skill       skip the skill step (binary + data only)
  --no-fixtures    skip the fixture step (no corpus is installed; `doctor` then needs
                   --fixtures or $SVG_GRID_FIXTURES)
  --no-fonts       skip the font step (install the binary only)
  --no-build       skip the build (use the release binary already in target/release)
  --dry-run        print what would be done, write nothing
  --check          read-only: verify the installed binary and skill, write nothing;
                   exits 2 with a `fix:` line per item that is not satisfied
  -h, --help       print this text and exit 0
EOF
}

die() { # die <one-line failure> <fix>
  printf 'install.sh: %s\n' "$1" >&2
  printf 'Fix: %s\n' "$2" >&2
  exit 1
}

canon() { # absolute path, no trailing slashes; the path need not exist
  local path="$1"
  case "$path" in
    '~') path="$HOME" ;;
    '~/'*) path="$HOME/${path#'~/'}" ;;
  esac
  case "$path" in
    /*) ;;
    *) path="$PWD/$path" ;;
  esac
  if command -v realpath >/dev/null 2>&1; then
    realpath -m -- "$path"
  else
    printf '%s\n' "${path%"${path##*[!/]}"}"
  fi
}

count_faces() { # <dir> -> font files svg_grid would load (recursive, like fontdb)
  if [ -d "$1" ]; then
    find "$1" -type f \( -name '*.ttf' -o -name '*.otf' -o -name '*.ttc' \) 2>/dev/null |
      wc -l | tr -d ' '
  else
    printf '0'
  fi
}

count_svgs() { # <dir> -> corpus files (`doctor` registers *.svg)
  if [ -d "$1" ]; then
    find "$1" -type f -name '*.svg' 2>/dev/null | wc -l | tr -d ' '
  else
    printf '0'
  fi
}

font_counts() { # stdin: setup.sh output -> "copied up-to-date missing" (empty if absent)
  sed -n 's/^.*: \([0-9][0-9]*\) copied, \([0-9][0-9]*\) already up-to-date, \([0-9][0-9]*\) missing$/\1 \2 \3/p' |
    tail -n 1
}

on_path() { # <dir> -> success when the directory is one of $PATH's entries
  local want entry
  want="$(canon "$1")"
  local IFS=:
  for entry in $PATH; do
    [ -n "$entry" ] || continue
    if [ "$(canon "$entry")" = "$want" ]; then
      return 0
    fi
  done
  return 1
}

# ── arguments ───────────────────────────────────────────────────────────────────────
[ -n "${HOME:-}" ] || die "HOME is not set" "export HOME=<your home directory>, then re-run install.sh"
prefix="$HOME/.local/bin"
do_build=1
do_fixtures=1
do_fonts=1
do_skill=1
project=""
check=0
dry_run=0
while [ $# -gt 0 ]; do
  case "$1" in
    --prefix)
      [ $# -ge 2 ] || { printf 'install.sh: missing value for --prefix\n' >&2; usage >&2; exit 2; }
      prefix="$2"; shift 2 ;;
    --prefix=*) prefix="${1#--prefix=}"; shift ;;
    --project)
      [ $# -ge 2 ] || { printf 'install.sh: missing value for --project\n' >&2; usage >&2; exit 2; }
      project="$2"; shift 2 ;;
    --project=*) project="${1#--project=}"; shift ;;
    --no-build) do_build=0; shift ;;
    --no-fixtures) do_fixtures=0; shift ;;
    --no-fonts) do_fonts=0; shift ;;
    --no-skill) do_skill=0; shift ;;
    --check) check=1; shift ;;
    --dry-run) dry_run=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) printf 'install.sh: unknown argument %s\n' "$1" >&2; usage >&2; exit 2 ;;
  esac
done

[ -n "$prefix" ] || { printf 'install.sh: empty --prefix\n' >&2; exit 2; }
prefix="$(canon "$prefix")"
data_root="$(canon "$HOME/svg_grid/fontlib")"
fixtures_dst="$(canon "$HOME/svg_grid/fixtures")"

# ── skill paths (source is fixed; destination depends on --project) ─────────────────
skill_name=""
skill_dest=""
skills_root=""
skill_scope=""
if [ "$do_skill" -eq 1 ]; then
  [ -f "$skill_src/SKILL.md" ] ||
    die "skill step: $skill_src/SKILL.md is missing" \
      "restore tools/svg_grid/docs/general-figure-guide (it is tracked in git)"
  [ -d "$skill_src/references" ] ||
    die "skill step: $skill_src/references/ is missing" \
      "restore tools/svg_grid/docs/general-figure-guide/references (it is tracked in git)"
  skill_name="$(sed -n 's/^name:[[:space:]]*//p' "$skill_src/SKILL.md" | head -n 1 | sed 's/[[:space:]]*$//')"
  [ -n "$skill_name" ] ||
    die "skill step: $skill_src/SKILL.md has no 'name:' in its frontmatter" \
      "add 'name: <skill name>' to the frontmatter; the installed dir name must equal it"
  if [ -n "$project" ]; then
    [ -d "$project" ] ||
      die "--project $project is not an existing directory" \
        "pass a real project dir, e.g. --project tools/svg_grid"
    project="$(canon "$project")"
    skills_root="$project/.commandcode/skills"
    skill_scope="project"
  else
    skills_root="$HOME/.commandcode/skills"
    skill_scope="global"
  fi
  skill_dest="$skills_root/$skill_name"
fi

sha_of() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum -- "$1" | cut -d' ' -f1
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 -- "$1" | cut -d' ' -f1
  else
    cksum -- "$1" | cut -d' ' -f1
  fi
}

# Copy the whitelist (SKILL.md + references/) into <dir> — never the whole source dir.
stage_skill() {
  for entry in $skill_files; do
    cp -Rp -- "$skill_src/$entry" "$1/" ||
      die "cannot copy $skill_src/$entry" "check that $1 is writable"
  done
}

# ── --check: read-only verification, never builds, never writes ─────────────────────
if [ "$check" -eq 1 ]; then
  bad=0
  fail_item() {
    bad=$((bad + 1))
    printf 'FAIL: %s\n' "$1"
    shift
    for line in "$@"; do printf '  %s\n' "$line"; done
  }

  if [ ! -e "$prefix/svg_grid" ]; then
    fail_item "no binary at $prefix/svg_grid" \
      "why:  the skill drives this command; without it nothing runs" \
      "fix:  bash $here/install.sh"
  elif [ ! -x "$prefix/svg_grid" ]; then
    fail_item "binary $prefix/svg_grid is not executable" "fix:  bash $here/install.sh"
  elif [ ! -f "$main_bin" ]; then
    fail_item "no build artifact at $main_bin — cannot tell whether the installed binary is current" \
      "why:  --check never builds; it compares against the last release build" \
      "fix:  bash $here/install.sh    (builds first, then installs)"
  elif [ "$(sha_of "$main_bin")" != "$(sha_of "$prefix/svg_grid")" ]; then
    fail_item "binary $prefix/svg_grid is stale (installed != build artifact $main_bin)" \
      "why:  src/ or convert/ changed after the last install" \
      "fix:  bash $here/install.sh"
  else
    printf 'OK: binary    %s  (== build artifact)\n' "$prefix/svg_grid"
  fi

  if [ "$do_skill" -eq 0 ]; then
    printf 'note: skill     skipped (--no-skill)\n'
  elif [ -L "$skill_dest" ]; then
    fail_item "skill $skill_dest is a symlink" \
      "why:  installs are copies — a link breaks once the dev tree moves or is absent" \
      "fix:  rm -f $skill_dest && bash $here/install.sh"
  elif [ ! -d "$skill_dest" ]; then
    fail_item "no skill installed at $skill_dest" \
      "why:  the harness scans $skills_root and will not find $skill_name" \
      "fix:  bash $here/install.sh"
  else
    stage="$(mktemp -d)" || die "mktemp -d failed" "check that the temp dir is writable"
    stage_skill "$stage"
    if diff -r -- "$stage" "$skill_dest" >/dev/null 2>&1; then
      printf 'OK: skill     %s  (byte-identical to SKILL.md + references/)\n' "$skill_dest"
    else
      extra="$( (cd "$skill_dest" && find . -type f -print) | while IFS= read -r rel; do
                  [ -e "$stage/$rel" ] || printf '%s ' "${rel#./}"; done)"
      differ="$( (cd "$stage" && find . -type f -print) | while IFS= read -r rel; do
                  if [ ! -e "$skill_dest/$rel" ]; then printf '%s (missing) ' "${rel#./}"
                  elif ! cmp -s -- "$skill_dest/$rel" "$stage/$rel"; then printf '%s ' "${rel#./}"; fi
                done)"
      if [ -n "$extra" ]; then
        fail_item "skill $skill_dest ships files it must not have: $extra" \
          "why:  the installed skill is SKILL.md + references/ only; dev-side files (ARCHITECTURE*/, src/, docs/, PROGRESS.md) lead a using agent into dev docs" \
          "fix:  bash $here/install.sh    (replaces the whole dir)"
      fi
      if [ -n "$differ" ]; then
        fail_item "skill $skill_dest is not byte-identical to the source: $differ" \
          "why:  the installed skill is a copy; edits to SKILL.md / references/ do not propagate" \
          "fix:  bash $here/install.sh"
      fi
      [ -n "$extra$differ" ] || fail_item "skill $skill_dest differs from the source" \
        "fix:  bash $here/install.sh"
    fi
    rm -rf -- "$stage"
  fi

  if [ "$do_skill" -eq 1 ]; then
    case "$skill_dest" in
      "$skills_root/$skill_name") printf 'OK: location  %s  (%s)\n' "$skills_root" "$skill_scope" ;;
      *) fail_item "$skill_dest is not $skills_root/$skill_name" \
           "why:  a harness only discovers skills at <root>/.commandcode/skills/<SKILL.md name>" ;;
    esac
  fi

  [ "$bad" -eq 0 ] || exit 2
  exit 0
fi

# ── 1. build the one crate in release (convert is a library dependency) ─────────────
if [ "$do_build" -eq 0 ]; then
  [ "$dry_run" -eq 0 ] || printf 'dry-run: would skip the build (--no-build)\n'
else
  command -v cargo >/dev/null 2>&1 ||
    die "cargo not found (needed to build the release binary)" \
      "install Rust (https://rustup.rs) or source ~/.cargo/env, then re-run install.sh"
  if [ "$dry_run" -eq 1 ]; then
    printf 'dry-run: would build svg_grid in %s (cargo build --release --offline; the convert crate is compiled in)\n' "$main_crate"
  else
    build_log=""
    if ! build_log="$(cd "$main_crate" && cargo build --release --offline 2>&1)"; then
      printf '%s\n' "$build_log" >&2
      die "build failed in $main_crate (cargo build --release --offline)" \
        "fix the compile errors above, then re-run install.sh"
    fi
  fi
fi

# The binary must exist before anything is copied.
if [ ! -f "$main_bin" ]; then
  if [ "$do_build" -eq 1 ] && [ "$dry_run" -eq 0 ]; then
    die "the build produced no binary at $main_bin" "run cargo build --release --offline in $here and re-run install.sh"
  fi
  if [ "$do_build" -eq 0 ]; then
    die "no release binary at $main_bin (--no-build)" "drop --no-build so install.sh builds it"
  fi
fi

# ── 2. install the one binary (and drop a stale svg_grid_convert) ────────────────────
stale_line=""
if [ "$dry_run" -eq 1 ]; then
  printf 'dry-run: would install svg_grid -> %s/svg_grid\n' "$prefix"
  if [ -e "$prefix/$stale_convert" ]; then
    printf 'dry-run: would remove the stale %s/%s (the converter is compiled into svg_grid)\n' \
      "$prefix" "$stale_convert"
  fi
else
  mkdir -p -- "$prefix" ||
    die "cannot create the install directory $prefix" "pass a writable --prefix, or create $prefix yourself"
  cp -f -- "$main_bin" "$prefix/svg_grid" ||
    die "cannot copy $main_bin into $prefix" "check that $prefix is writable and not read-only"
  chmod 0755 "$prefix/svg_grid" ||
    die "cannot make $prefix/svg_grid executable" "check the permissions of $prefix"
  if [ -e "$prefix/$stale_convert" ]; then
    rm -f -- "$prefix/$stale_convert" ||
      die "cannot remove the stale $prefix/$stale_convert" "remove it yourself, then re-run install.sh"
    stale_line="removed stale: $prefix/$stale_convert (svg_grid convert replaces it)"
  fi
fi

# ── 3. fixture corpus (the data root `doctor` reads by default) ─────────────────────
fixtures_line=""
if [ "$do_fixtures" -eq 0 ]; then
  [ "$dry_run" -eq 0 ] || printf 'dry-run: would skip the fixture step (--no-fixtures)\n'
  fixtures_line="fixtures: skipped (--no-fixtures)"
else
  [ -d "$fixtures_src" ] ||
    die "fixture step: $fixtures_src is missing" "restore tools/svg_grid/tests/fixtures (it is tracked in git)"
  command -v rsync >/dev/null 2>&1 ||
    die "rsync not found (needed to install the fixture corpus)" \
      "install rsync (Debian/Ubuntu: apt-get install rsync), then re-run install.sh"
  if [ "$dry_run" -eq 1 ]; then
    printf 'dry-run: would sync %s files from %s -> %s (rsync -a --delete)\n' \
      "$(count_svgs "$fixtures_src")" "$fixtures_src" "$fixtures_dst"
  else
    mkdir -p -- "$fixtures_dst" ||
      die "cannot create the fixture directory $fixtures_dst" "check that $HOME/svg_grid is writable"
    rsync -a --delete -- "$fixtures_src/" "$fixtures_dst/" ||
      die "fixture step: cannot sync $fixtures_src into $fixtures_dst" \
        "check that $HOME/svg_grid is writable and rsync is complete"
  fi
  fixtures_line="fixtures: $(count_svgs "$fixtures_dst") files in $fixtures_dst"
fi

# ── 4. font library (the data root svg_grid reads by default) ───────────────────────
font_line=""
if [ "$do_fonts" -eq 0 ]; then
  [ "$dry_run" -eq 0 ] || printf 'dry-run: would skip the font step (--no-fonts)\n'
  font_line="fonts: skipped (--no-fonts)"
else
  [ -f "$font_script" ] ||
    die "font step: $font_script is missing" "restore tools/svg_grid/fontlib/setup.sh (it is tracked in git)"
  font_args=(--dir "$data_root")
  if [ "$dry_run" -eq 1 ]; then
    font_args+=(--dry-run)
  fi
  font_log=""
  if ! font_log="$(bash "$font_script" "${font_args[@]}" 2>&1)"; then
    printf '%s\n' "$font_log" >&2
    if [ "$dry_run" -eq 1 ]; then
      die "font step: the font library would stay incomplete (faces listed above)" \
        "install Arial and Times New Roman (Debian/Ubuntu: ttf-mscorefonts-installer), then re-run install.sh"
    fi
    die "font step: could not fill $data_root (faces listed above)" \
      "install Arial and Times New Roman (Debian/Ubuntu: ttf-mscorefonts-installer), then re-run install.sh"
  fi
  counts="$(printf '%s\n' "$font_log" | font_counts)"
  if [ -n "$counts" ]; then
    read -r copied up_to_date missing <<<"$counts"
  else
    copied="0"; up_to_date="$(count_faces "$data_root")"; missing="0"
  fi
  faces="$(count_faces "$data_root")"
  if [ "$dry_run" -eq 1 ]; then
    printf 'dry-run: fontlib %s would hold %s faces (would copy %s, already up-to-date %s, missing %s)\n' \
      "$data_root" "$((faces + copied))" "$copied" "$up_to_date" "$missing"
  fi
  font_line="fonts: $faces faces in $data_root (copied $copied, already up-to-date $up_to_date)"
fi

# ── 5. the skill shipped with the tool (whitelist copy into the harness skill dir) ───
skill_line=""
if [ "$do_skill" -eq 0 ]; then
  [ "$dry_run" -eq 0 ] || printf 'dry-run: would skip the skill step (--no-skill)\n'
  skill_line="skill: skipped (--no-skill)"
elif [ "$dry_run" -eq 1 ]; then
  printf 'dry-run: would install skill %s -> %s (SKILL.md + references/, %s)\n' \
    "$skill_name" "$skill_dest" "$skill_scope"
else
  mkdir -p -- "$skills_root" ||
    die "cannot create the skill dir $skills_root" "check that its parent directory is writable"
  stage_new="$(mktemp -d "$skills_root/.staging-$skill_name.XXXXXX")" ||
    die "cannot stage the skill inside $skills_root" "check that $skills_root is writable"
  stage_skill "$stage_new"
  if [ -d "$skill_dest" ] && diff -r -- "$stage_new" "$skill_dest" >/dev/null 2>&1; then
    rm -rf -- "$stage_new"
    skill_state="up to date"
  else
    old=""
    if [ -e "$skill_dest" ]; then
      old="$skills_root/.old-$skill_name.$$"
      rm -rf -- "$old" 2>/dev/null || true
      mv -- "$skill_dest" "$old" ||
        die "cannot move the old skill $skill_dest out of the way" "check the permissions of $skills_root"
    fi
    if ! mv -- "$stage_new" "$skill_dest"; then
      [ -z "$old" ] || mv -- "$old" "$skill_dest" 2>/dev/null || true
      die "cannot move the new skill into $skill_dest" "check that $skills_root is on a writable filesystem"
    fi
    [ -z "$old" ] || rm -rf -- "$old"
    skill_state="installed"
  fi
  skill_line="skill: $skill_name -> $skill_dest ($skill_state, $(find "$skill_dest" -type f | wc -l | tr -d ' ') files, $skill_scope)"
fi

# ── 6. report ────────────────────────────────────────────────────────────────────────
if on_path "$prefix"; then
  path_line="PATH: $prefix is on PATH"
else
  path_line="PATH: $prefix is not on PATH - add it with: export PATH=\"$prefix:\$PATH\""
fi

if [ "$dry_run" -eq 1 ]; then
  printf '%s\n' "$path_line"
  exit 0
fi
version="$("$prefix/svg_grid" --version)" ||
  die "the installed binary does not run: $prefix/svg_grid" "check it with: $prefix/svg_grid --version"
printf '%s\n' "$version"
printf 'installed: %s (svg_grid)\n' "$prefix"
if [ -n "$stale_line" ]; then
  printf '%s\n' "$stale_line"
fi
printf '%s\n' "$skill_line"
printf '%s\n' "$fixtures_line"
printf '%s\n' "$font_line"
printf '%s\n' "$path_line"
