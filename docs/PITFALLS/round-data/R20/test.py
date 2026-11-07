#!/usr/bin/env python3
"""R20 的可跑证据 —— `general-figure-guide` 补第一条主干：**原生出图优先**。

用户裁决（照做）：「一旦使用这个 skill，agent 就会忽略那些分析原本的出图功能转而自己从 0 搭建图……
现在的改法就是：**优先用对应分析包的原生出图功能，在此基础上做视觉优化**，我只是想避免 agent 自己再造轮子。」

六族：

① **主干存在且章节同名**：`SKILL.md` 有 `# CORE WORKFLOW`，其后**每个 `##` 章节名与主干的单条步骤名逐字一致**
   （解析比对，不比人眼），顺序一致、数量都是 5，且每个章节正文非空（≥ 150 字符，防空壳）。
   顺带钉住报告要求的三处补丁：快速规则表的「panel 来源」行、文档索引的 `00_native_plots.md` 行、
   反面清单的「包有出图函数却手写 UMAP」条、以及 frontmatter `description` 的口径。
② **对照表里的函数名真的存在**：`references/00_native_plots.md` 存在且 ≤ 12 288 B（12 KB）；
   文中被**归属到具体包**的函数名逐条核对——本机已装的包（`python-bio` 的 scanpy/seaborn/squidpy、
   `r453` 的 Seurat/limma/edgeR/ComplexHeatmap/pheatmap/monocle3/CellChat/WGCNA/ggplot2/svglite）
   **实测 `hasattr` / `exists(..., asNamespace())`**；本机**没装**的包（DESeq2/fgsea/clusterProfiler/
   enrichplot/maftools/ggraph/scvelo/cnvkit）**只做静态清单检查**，且脚本断言「它们在本机确实没装」
   （装了就必须改成实测，不许拿「没装」当借口）。要求 ≥ 8 条实测，实际远多于此。
③ **`05_checklist.md` 有可判定的「来源」项**：`# native: none — <原因>`、`grep -rn "# native:"`、
   与 `figure_panels/*.svg` 计数对账、「视觉改动…不换图型」四串都在。
④ **两副本一致**：`diff -r` 规范源 vs 复制件为空。
⑤ **体积与文档卫生**：指南全部 md ≤ 20 000 B；R9 的三条口径照旧成立（无变更叙述词、`02a` 仍有
   `ARCHITECTURE/warnings.md` 指针、无 `DECISIONS.md`）；`02a` 开头有指向 `00_native_plots.md` 的指针
   且 `> **首选路线**` 一行还在（证明是「加 1 行 + 等量删」，不是覆盖式改写）。
⑥ **回归**：R8 / R10 / R19（R19 内部起 R18，R18 内部再逐轮跑 R4–R17 ⇒ R4–R19 覆盖完整）。
   本轮只改文档，故不从 R20 里重复起其余轮的进程——理由写在 `meta.toml` 的 note 里。

无参数。离线；①–⑤ 只读（不写仓库）；临时产物在临时目录。
`micromamba` 不在 PATH ⇒ ② 的实测部分 SKIP 并打印原因（静态清单部分照跑）；R 侧同理。
"""
import json
import pathlib
import re
import shutil
import subprocess
import sys
import tempfile
import time

HERE = pathlib.Path(__file__).resolve().parent
SVG_GRID = HERE.parents[3]                       # round-data/R20 → PITFALLS → docs → svg_grid
REPO = HERE.parents[5]                           # → bioagentforge（根仓库）
GUIDE = SVG_GRID / "docs/general-figure-guide"
COPY = REPO / "tools/manuscripts_skills/general-figure-guide"
SKILL = GUIDE / "SKILL.md"
NATIVE = GUIDE / "references/00_native_plots.md"
ASSEMBLY = GUIDE / "references/02a_svg_grid_assembly.md"
CHECKLIST = GUIDE / "references/05_checklist.md"

LIMIT = 20000                                    # R9 的口径：每份 md ≤ 20 000 B（从严格字节）
NATIVE_LIMIT = 12288                             # 报告要求：00_native_plots.md ≤ 12 KB
MIN_VERIFIED = 8                                 # 报告要求：至少抽查 8 个真实存在的函数名
MIN_SECTION = 150                                # 每个步骤章节的正文下限（防空壳）

MM = shutil.which("micromamba")
if not MM:
    local = pathlib.Path.home() / ".local/bin/micromamba"
    MM = str(local) if local.is_file() else None
ROUNDS = ["R8", "R10", "R19"]

# 报告点名的五步（照抄裁决里的顺序）——主干与章节都要落在这五条上。
STEPS = [
    "确认 panel 来源（原生出图优先）",
    "按目标格子尺寸出矢量 panel",
    "组图（svg_grid 优先）",
    "视觉检查（vision 必须真看成品）",
    "交付核对",
]

# 本机**没装**的包：只做静态清单检查（每个函数名都必须在下面登记，否则算「编的」）。
STATIC = {
    "DESeq2": ["plotMA", "plotPCA", "plotDispEsts"],
    "fgsea": ["plotEnrichment"],
    "clusterProfiler": ["dotplot", "cnetplot", "emapplot"],
    "enrichplot": ["gseaplot2"],
    "maftools": ["oncoplot", "rainfallPlot"],
    "ggraph": ["ggraph", "geom_edge_link", "geom_node_point", "geom_node_text"],
    "scvelo.pl": ["velocity_embedding", "velocity_embedding_grid", "velocity_embedding_stream"],
    "cnvkit": ["scatter", "heatmap", "diagram"],          # `cnvkit.py <子命令>`，不是 Python 函数
}
PY_NS = {"sc.pl", "squidpy.pl", "seaborn"}               # 可实测的 Python 命名空间
ABSENT_R = ["DESeq2", "fgsea", "clusterProfiler", "enrichplot", "maftools", "ggraph"]
ABSENT_PY = ["scvelo", "cnvkit"]

MOD_RE = re.compile(r"\b(sc\.pl|squidpy\.pl|scvelo\.pl|seaborn)\.([a-z_][a-z0-9_]*)")
PKG_RE = re.compile(r"\b([A-Za-z][A-Za-z0-9]*)::([A-Za-z_][A-Za-z0-9_.]*)")
CNV_RE = re.compile(r"cnvkit\.py (scatter|heatmap|diagram)")
BARE_RE = re.compile(r"`([a-z][A-Za-z0-9_]*)`")
CIRCLED = re.compile("^[\u2460-\u2473]\\s*")

FAILS = []
TMP = None


def check(ok, label, detail=""):
    print(("PASS  " if ok else "FAIL  ") + label)
    if not ok and detail:
        print("      " + str(detail).replace("\n", "\n      ")[:1200])
    if not ok:
        FAILS.append(label)


def skip(label, reason):
    print(f"SKIP  {label} — {reason}")


def info(text):
    print(f"INFO  {text}")


def norm(name):
    """去掉序号标记与首尾空白，用于「步骤名 == 章节名」的逐字比较。"""
    return CIRCLED.sub("", name.strip()).strip()


# ── ① 主干与章节 ────────────────────────────────────────────────────────────────────
def parse_workflow(text):
    match = re.search(r"^#\s+CORE WORKFLOW\s*$", text, re.M)
    if not match:
        return None, None, None
    body = text[match.end():]
    nxt = re.search(r"^#\s+\S", body, re.M)
    block = body[:nxt.start()] if nxt else body
    rest = body[nxt.start():] if nxt else body
    steps = [norm(one) for one in re.findall(r"^\d+\.\s+\*\*(.+?)\*\*", block, re.M)]
    heads = [norm(one) for one in re.findall(r"^##\s+(.+?)\s*$", rest, re.M)]
    return steps, heads, rest


def section_bodies(rest):
    parts = re.split(r"^##\s+", rest, flags=re.M)[1:]
    sizes = {}
    for part in parts:
        title, _, body = part.partition("\n")
        sizes[norm(title)] = len(body.strip())
    return sizes


def family_one():
    print("\n== ① SKILL.md 有 # CORE WORKFLOW，章节名与 5 步逐字一致 ==")
    text = SKILL.read_text()
    steps, heads, rest = parse_workflow(text)
    if steps is None:
        check(False, "`SKILL.md` 有 `# CORE WORKFLOW` 一级章节", "未找到该标题")
        return
    check(True, "`SKILL.md` 有 `# CORE WORKFLOW` 一级章节")
    info("主干步骤：" + " / ".join(steps))
    info("其下章节：" + " / ".join(heads))
    check(steps == STEPS, f"主干是报告要求的 5 步且顺序一致（{len(STEPS)} 步）", steps)
    check(len(heads) == len(STEPS), f"`# CORE WORKFLOW` 之后恰好 {len(STEPS)} 个 `##` 章节", heads)
    check(heads == steps,
          "每个章节名与主干的单条步骤名**逐字一致**、顺序一致（解析比对）",
          f"steps={steps}\nheads={heads}")
    bodies = section_bodies(rest)
    check(len(bodies) == len(STEPS), f"5 个步骤章节各自有正文（解析到 {len(bodies)} 节）", sorted(bodies))
    thin = {title: size for title, size in bodies.items() if size < MIN_SECTION}
    check(not thin, f"每个步骤章节正文 ≥ {MIN_SECTION} 字符（没有空壳章节）", thin)

    # 报告要求的三处补丁（同一条主干上的可见证据）
    check(bool(re.search(r"^\|\s*panel 来源\s*\|", text, re.M)),
          "快速规则表有「panel 来源」行")
    row = re.search(r"^\|\s*panel 来源\s*\|.*$", text, re.M)
    check(bool(row) and "00_native_plots.md" in row.group(0),
          "「panel 来源」那一行指向 `00_native_plots.md`", row.group(0) if row else "")
    check(bool(row) and "原生" in row.group(0) and "native:" in row.group(0),
          "「panel 来源」那一行写明「分析包的原生出图函数优先」+ 例外留痕", row.group(0) if row else "")
    index = re.search(r"^###\s*文档索引\s*$(.+?)(?=^###\s|\Z)", text, re.M | re.S)
    check(bool(index) and "references/00_native_plots.md" in index.group(1),
          "文档索引里有 `00_native_plots.md` 一行")
    bad = re.search(r"^###\s*反面清单.+?$(.+?)(?=^###\s|\Z)", text, re.M | re.S)
    check(bool(bad) and "手写 UMAP" in bad.group(1),
          "反面清单有「包明明有出图函数，却自己手写 UMAP / 热图 / 火山图」一条")
    front = text.split("---")[1] if text.startswith("---") else ""
    check("原生" in front and ("sc.pl.umap" in front or "DimPlot" in front),
          "frontmatter 的 description 体现「先原生出图再视觉优化」", front[:200])


# ── ② 对照表里的函数名 ──────────────────────────────────────────────────────────────
def attributed_tokens(text):
    """把文中「归属到具体包」的函数名抠出来：`Pkg::fn`、`sc.pl.fn`、`seaborn.fn`、`cnvkit.py <子命令>`，
    以及同一行里跟在包前缀之后的裸函数名（如 `CellChat::netVisual_circle`、`netVisual_chord_cell` 这种一行多写）。"""
    tokens = {}
    for line in text.splitlines():
        seen = None
        for match in MOD_RE.finditer(line):
            seen = match.group(1)
            tokens[(seen, match.group(2))] = line
        for match in PKG_RE.finditer(line):
            seen = match.group(1)
            fn = match.group(2)
            tokens[(seen, fn + "*" if fn.endswith("_") else fn)] = line
        if CNV_RE.search(line):
            seen = "cnvkit"
            for match in CNV_RE.finditer(line):
                tokens[("cnvkit", match.group(1))] = line
        if not seen:
            continue
        for match in BARE_RE.finditer(line):
            if match.group(1) == seen.split(".")[0]:
                continue
            tokens.setdefault((seen, match.group(1)), line)
    return tokens


PY_PROBE = """import importlib.util, json, sys
spec = json.load(open(sys.argv[1]))
absent = json.load(open(sys.argv[2]))
ns = {}
if any(name.startswith("sc.pl") for name in spec):
    import scanpy as sc
    ns["sc.pl"] = sc.pl
if any(name.startswith("squidpy.pl") for name in spec):
    import squidpy as sq
    ns["squidpy.pl"] = sq.pl
if "seaborn" in spec:
    import seaborn
    ns["seaborn"] = seaborn
for name in sorted(spec):
    module = ns[name]
    for fn in spec[name]:
        if fn.endswith("*"):
            hits = [n for n in dir(module) if n.startswith(fn[:-1])]
            print(("ok" if hits else "missing"), name, fn, len(hits))
        else:
            print(("ok" if hasattr(module, fn) else "missing"), name, fn, 1)
for pkg in absent:
    print("absent", pkg, importlib.util.find_spec(pkg) is None, 1)
"""

R_PROBE = """args <- commandArgs(trailingOnly = TRUE)
spec <- readLines(args[1], warn = FALSE)
for (line in spec) {
  bits <- strsplit(line, "\\t", fixed = TRUE)[[1]]
  pkg <- bits[1]; fn <- bits[2]
  if (grepl("\\\\*$", fn)) {
    stem <- sub("\\\\*$", "", fn)
    hits <- length(grep(paste0("^", stem), getNamespaceExports(pkg)))
    cat(if (hits > 0) "ok" else "missing", pkg, fn, hits, "\\n")
  } else {
    cat(if (exists(fn, envir = asNamespace(pkg), inherits = FALSE)) "ok" else "missing",
        pkg, fn, 1, "\\n")
  }
}
for (pkg in readLines(args[2], warn = FALSE)) {
  cat("absent", pkg, !requireNamespace(pkg, quietly = TRUE), 1, "\\n")
}
"""


def parse_probe(stdout):
    found, absent = {}, {}
    for line in stdout.splitlines():
        bits = line.split()
        if len(bits) < 4:
            continue
        state, name, fn = bits[0], bits[1], bits[2]
        if state in ("ok", "missing"):
            found[(name, fn)] = (state == "ok", int(float(bits[3])))
        elif state == "absent":
            absent[name] = bits[2].upper() == "TRUE"
    return found, absent


def family_two():
    print("\n== ② 00_native_plots.md：存在 / ≤ 12 KB / 函数名落到真实符号 ==")
    if not NATIVE.is_file():
        check(False, "`references/00_native_plots.md` 存在", str(NATIVE))
        return
    size = NATIVE.stat().st_size
    check(size <= NATIVE_LIMIT, f"`00_native_plots.md` ≤ {NATIVE_LIMIT} B（实测 {size} B）")
    text = NATIVE.read_text()
    for needle in ("判据", "对照表", "例外", "# native:", "原生出图 ≠ 最终形态"):
        check(needle in text, f"`00_native_plots.md` 含「{needle}」")
    tokens = attributed_tokens(text)
    live_side = {key for key in tokens if key[0] not in STATIC}
    static_side = {key for key in tokens if key[0] in STATIC}
    info(f"被归属到具体包的函数名共 {len(tokens)} 条：可实测 {len(live_side)} 条、"
         f"只做静态清单 {len(static_side)} 条")
    check(len(tokens) >= 40, "对照表里的函数名 ≥ 40 条（覆盖报告点名的六块）", len(tokens))

    py_spec, r_spec = {}, {}
    for name, fn in sorted(live_side):
        (py_spec if name in PY_NS else r_spec).setdefault(name, []).append(fn)
    found, absent = {}, {}
    if not MM:
        skip("函数名实测", "`micromamba` 不在 PATH")
    else:
        if py_spec:
            spec = TMP / "py_spec.json"
            spec.write_text(json.dumps(py_spec))
            gone = TMP / "py_absent.json"
            gone.write_text(json.dumps(ABSENT_PY))
            script = TMP / "probe.py"
            script.write_text(PY_PROBE)
            result = subprocess.run([MM, "run", "-n", "python-bio", "python", str(script),
                                     str(spec), str(gone)],
                                    capture_output=True, text=True, cwd=str(TMP))
            if result.returncode != 0:
                skip("Python 侧函数名实测", f"python-bio 跑不通：{result.stderr.strip()[-300:]}")
            else:
                got, got_absent = parse_probe(result.stdout)
                found.update(got)
                absent.update(got_absent)
        if r_spec:
            spec = TMP / "r_spec.tsv"
            spec.write_text("\n".join(f"{pkg}\t{fn}" for pkg in sorted(r_spec) for fn in r_spec[pkg]))
            gone = TMP / "r_absent.txt"
            gone.write_text("\n".join(ABSENT_R))
            script = TMP / "probe.R"
            script.write_text(R_PROBE)
            result = subprocess.run([MM, "run", "-n", "r453", "Rscript", "--vanilla", str(script),
                                     str(spec), str(gone)],
                                    capture_output=True, text=True, cwd=str(TMP))
            if result.returncode != 0:
                skip("R 侧函数名实测", f"r453 跑不通：{result.stderr.strip()[-300:]}")
            else:
                got, got_absent = parse_probe(result.stdout)
                found.update(got)
                absent.update(got_absent)

    if found:
        alive = sorted(f"{name}::{fn}" for (name, fn), (ok, _) in found.items() if ok)
        dead = sorted(f"{name}::{fn}" for (name, fn), (ok, _) in found.items() if not ok)
        info(f"实测存在 {len(alive)} 条：" + "、".join(alive))
        check(len(alive) >= MIN_VERIFIED,
              f"本机已装包里**实测存在**的函数名 ≥ {MIN_VERIFIED} 条（实测 {len(alive)} 条）")
        check(not dead, "被写成「原生函数」的名字没有一个落空（`hasattr`/`exists` 全成立）", dead)
        missed = sorted(f"{name}::{fn}" for (name, fn) in live_side if (name, fn) not in found)
        check(not missed, "可实测那侧没有漏检的名字（每条都真的跑过）", missed)
    else:
        skip("函数名实测结果", "探针没跑起来")

    unknown = sorted(f"{pkg}::{fn}" for (pkg, fn) in static_side if fn not in STATIC.get(pkg, []))
    info("只做静态清单（本机未装）：" +
         "、".join(sorted(f"{pkg}::{fn}" for (pkg, fn) in static_side)))
    check(not unknown, "静态清单里的名字都在脚本登记的白名单里（没有夹带没登记的名字）", unknown)
    if absent:
        still = {pkg: flag for pkg, flag in absent.items() if not flag}
        check(not still, "静态清单里的包在本机**确实没装**（装了就必须改成实测）", still)
        check(all(absent.get(pkg) for pkg in ABSENT_R if pkg in absent),
              "R 侧静态包（DESeq2/fgsea/clusterProfiler/enrichplot/maftools/ggraph）本机均未安装")
        check(all(absent.get(pkg) for pkg in ABSENT_PY if pkg in absent),
              "Py 侧静态包（scvelo/cnvkit）本机均未安装")
    else:
        skip("「本机未装」核对", "环境探针没跑起来")


# ── ③ 05_checklist 的「来源」判据 ──────────────────────────────────────────────────
def family_three():
    print("\n== ③ 05_checklist.md 有可机械判定的「原生出图 / 例外留痕」项 ==")
    text = CHECKLIST.read_text()
    for label, ok in [
        ("`# native: none — <原因>` 的形式", "# native: none" in text),
        ("`grep -rn \"# native:\"` 的对账命令", 'grep -rn "# native:"' in text),
        ("与 `figure_panels/*.svg` 计数对齐", "figure_panels/*.svg" in text),
        ("「视觉改动可列举、且没改图型」", "视觉改动" in text and "图型" in text),
        ("指向判据出处 `00_native_plots.md`", "00_native_plots.md" in text),
    ]:
        check(ok, "`05_checklist.md` 含 " + label)
    section = re.search(r"^##\s*A0\..+?$(.+?)(?=^##\s)", text, re.M | re.S)
    check(bool(section) and len(section.group(1).strip()) > 200,
          "判据集中在独立一节（`## A0.`，不散落）")


# ── ④ 两副本一致 ────────────────────────────────────────────────────────────────────
def family_four():
    print("\n== ④ 规范源与复制件逐字节一致 ==")
    result = subprocess.run(["diff", "-r", str(GUIDE), str(COPY)], capture_output=True, text=True)
    check(result.returncode == 0,
          "`diff -r` 规范源 vs `tools/manuscripts_skills/general-figure-guide/` 为空",
          result.stdout[:600] or result.stderr[:400])
    left = {p.relative_to(GUIDE) for p in GUIDE.rglob("*.md")}
    right = {p.relative_to(COPY) for p in COPY.rglob("*.md")}
    check(left == right and pathlib.PurePath("references/00_native_plots.md") in left,
          "两边的 md 文件集合相同（新增的 `00_native_plots.md` 也在复制件里）",
          f"left-right={left - right} right-left={right - left}")


# ── ⑤ 体积与文档卫生 ────────────────────────────────────────────────────────────────
CHANGELOG = re.compile(r"已支持|已修复|不再需要|早期脚本|已现场实测收口|现在会被精确烘进")


def family_five():
    print("\n== ⑤ 所有 md ≤ 20 KB + 文档卫生（R9 的三条口径） ==")
    docs = [SKILL] + sorted((GUIDE / "references").glob("*.md"))
    sizes = sorted(((p.stat().st_size, str(p.relative_to(GUIDE))) for p in docs), reverse=True)
    over = [item for item in sizes if item[0] > LIMIT]
    info(f"{len(docs)} 份 md，最大三个：" + ", ".join(f"{name} {size}B" for size, name in sizes[:3]))
    check(not over, f"指南全部 md ≤ {LIMIT} B", over)
    check(all((COPY / pathlib.Path(name)).stat().st_size == size for size, name in sizes),
          "复制件里每份 md 的体积与规范源相同（不是旧版本）")
    hits = [(p.name, n, line.strip()[:60])
            for p in docs for n, line in enumerate(p.read_text().splitlines(), 1)
            if CHANGELOG.search(line)]
    check(not hits, "指南不含变更叙述词（已支持/已修复/不再需要/早期脚本/…）", hits)

    text = ASSEMBLY.read_text()
    check("ARCHITECTURE/warnings.md" in text, "`02a` 仍给出指向 `ARCHITECTURE/warnings.md` 的指针")
    check("00_native_plots.md" in text, "`02a` 有指向 `00_native_plots.md` 的指针（panel 从哪来）")
    check("> **首选路线**" in text, "`02a` 是「加 1 行指针」，不是覆盖式改写（`> **首选路线**` 一行仍在）")
    check(not (SVG_GRID / "DECISIONS.md").exists(), "不设 `DECISIONS.md`")


# ── ⑥ 回归 ──────────────────────────────────────────────────────────────────────────
def family_six():
    print("\n== ⑥ 回归 R8 / R10 / R19（R19 内部覆盖 R4–R17） ==")
    started = time.time()
    for round_id in ROUNDS:
        evidence = HERE.parent / round_id / "test.py"
        if not evidence.is_file():
            skip(f"回归 round-data/{round_id}", "没有 test.py")
            continue
        one = time.time()
        result = subprocess.run([sys.executable, str(evidence)], capture_output=True, text=True)
        info(f"{round_id}: rc={result.returncode} 用时 {time.time() - one:.1f}s")
        check(result.returncode == 0, f"回归 round-data/{round_id} 仍全绿",
              result.stdout[-900:] + result.stderr[-400:])
    info(f"总用时 {time.time() - started:.1f}s")


def main():
    global TMP
    TMP = pathlib.Path(tempfile.mkdtemp(prefix="r20-rounddata-"))
    print(f"# 规范源：{GUIDE}\n# 复制件：{COPY}\n# micromamba：{MM}\n")
    for path in (SKILL, NATIVE, ASSEMBLY, CHECKLIST):
        if not path.is_file():
            print(f"FAIL  缺文件：{path}")
            return 1
    family_one()
    family_two()
    family_three()
    family_four()
    family_five()
    family_six()
    if FAILS:
        print(f"\n== {len(FAILS)} 条不成立 ==")
        for label in FAILS:
            print("  · " + label)
        return 1
    print("\n== 全部成立 ==")
    return 0


if __name__ == "__main__":
    sys.exit(main())
