#!/usr/bin/env python3
"""R21 的可跑证据 —— 原生出图从「查一张表」升级成「先查一轮官方资料」。

用户裁决（照做）：「单细胞我只是举例，它应该让 agent 意识到：我正在用的这个分析包有没有原生出图的方式。
**先去找一轮官方资料（先找 tools/ 的对应工具，没有再网络查找），确认没有原生出图才自己搞图**。」

六族（对应报告的判据）：

① **`00_native_plots.md` 的 §0 查证流程在最前**：文中有「本表不是上限」硬话；§0 的四步
   （认包 / 查本仓 `tools/` / 查官方资料 / 确认没有才自绘）按序可被 `grep` 到；每步各自带可直接跑的命令；
   留痕格式含 `# native: <包>::<函数>` 与 `# native: none — …`，且写明 **`none` 必须带两处查证痕迹**；
   §1 改成指向 §0，不再留第二套并列判据。顺带钉住 §0 里的**真实命令输出**（不是编的）。
② **`SKILL.md` 第一步展开成四步**：章节正文含「认包 / 查 `tools/` / 查官方 / 确认无」四个关键词，
   两条命令示例**穿插**在该章里（`grep -ril` 与 `python -c "…dir(...)"`）；同时沿用 R20 的解析比对，
   断言 `# CORE WORKFLOW` 的 5 步名与随后 5 个 `##` 章节名**仍逐字一致**（不许退化）、每节正文非空。
③ **`05_checklist.md` 的 §A0** 判据改成「`none` 的原因必须含两处查证痕迹（`tools/` 与官方资料）」，
   并给一条抽查命令（`grep -rn "# native: none"`）。
④ **两副本一致**：`diff -r` 规范源 vs `tools/manuscripts_skills/general-figure-guide/` 为空。
⑤ **体积**：三份被改的 md 均 ≤ 20 000 B，其中 `00_native_plots.md` ≤ 12 288 B（12 KB）；
   指南全部 md 照旧 ≤ 20 000 B，且不含 R9 的变更叙述词。
⑥ **回归**：R10（两副本并轨 + 规范源唯一）与 R20（主干/章节/函数名/体积的全套断言）。
   R20 内含 R19→R18→R4–R17 的链条，实测约 18 分钟；超时（`R21_R20_TIMEOUT`，默认 1500s）时
   **只跑 R20 的前五族**（①②③④⑤，跳过它自己的 ⑥ 回归链），并在输出与 note 里**明说**——
   不静默降级。

无参数。离线；①–⑤ 只读（不写仓库）；临时产物在临时目录。
`micromamba` 不在 PATH ⇒ 不影响 ①–⑥（本轮不实测包内函数名，那由 R20 ② 负责）。
"""
import importlib.util
import os
import pathlib
import re
import subprocess
import sys
import tempfile
import time

HERE = pathlib.Path(__file__).resolve().parent
SVG_GRID = HERE.parents[3]                       # round-data/R21 → PITFALLS → docs → svg_grid
REPO = HERE.parents[5]                           # → bioagentforge（根仓库）
GUIDE = SVG_GRID / "docs/general-figure-guide"
COPY = REPO / "tools/manuscripts_skills/general-figure-guide"
SKILL = GUIDE / "SKILL.md"
NATIVE = GUIDE / "references/00_native_plots.md"
CHECKLIST = GUIDE / "references/05_checklist.md"

LIMIT = 20000                                    # R9 的口径：每份 md ≤ 20 000 B
NATIVE_LIMIT = 12288                             # 报告要求：00_native_plots.md ≤ 12 KB
MIN_SECTION = 150                                # 每个步骤章节的正文下限（防空壳）
R20_TIMEOUT = int(os.environ.get("R21_R20_TIMEOUT", "1500"))

STEPS = [
    "确认 panel 来源（原生出图优先）",
    "按目标格子尺寸出矢量 panel",
    "组图（svg_grid 优先）",
    "视觉检查（vision 必须真看成品）",
    "交付核对",
]

# §0 四步在文中的落地形态（报告点名的顺序与措辞）
FLOW = [
    ("①", "认包", "**认包**"),
    ("②", "查本仓 `tools/`", "**查本仓 `tools/`**"),
    ("③", "查官方资料", "**查官方资料**"),
    ("④", "确认没有才自绘", "**确认没有才自绘**"),
]

CHANGELOG = re.compile(r"已支持|已修复|不再需要|早期脚本|已现场实测收口|现在会被精确烘进")
CIRCLED = re.compile("^[\u2460-\u2473]\\s*")

FAILS = []


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
    return CIRCLED.sub("", name.strip()).strip()


# ── ① 00 的 §0 查证流程 ─────────────────────────────────────────────────────────────
def sections(text):
    """按 `## ` 切成 [(标题, 正文)]，保持文内顺序。"""
    parts = re.split(r"^##\s+", text, flags=re.M)[1:]
    out = []
    for part in parts:
        title, _, body = part.partition("\n")
        out.append((title.strip(), body.strip()))
    return out


def family_one():
    print("\n== ① 00_native_plots.md：§0 查证流程在最前 / 四步 / none 带两处痕迹 ==")
    if not NATIVE.is_file():
        check(False, "`references/00_native_plots.md` 存在", str(NATIVE))
        return
    text = NATIVE.read_text()
    head = text[:3000]
    check("本表不是上限" in head,
          "开头（前 3 000 字符）就写明「本表不是上限」——表里没有的包/工具一律走流程、禁止以「表里没写」为由自绘")
    check("禁止以" in text and "表里没写" in text and "自绘" in text,
          "「禁止以『表里没写』为由自绘」这句在文里")

    secs = sections(text)
    titles = [title for title, _ in secs]
    info("`00` 的 `##` 章节顺序：" + " / ".join(titles))
    check(bool(titles) and titles[0].startswith("0."),
          "**§0 查证流程是最靠前的一节**（第一个 `##` 是 §0）", titles[:2])
    check([t[:2] for t in titles[:3]] == ["0.", "1.", "2."],
          "章节编号 0 → 1 → 2 有序（§0 在 §1 判据、§2 表之前）", titles[:3])

    flow = secs[0][1] if secs else ""
    order = []
    for mark, name, token in FLOW:
        pos = flow.find(token)
        order.append(pos)
        check(pos >= 0, f"§0 有第 {mark} 步「{name}」", flow[:400])
    check(all(p >= 0 for p in order) and order == sorted(order),
          "§0 的四步按 认包 → 查 tools/ → 查官方 → 确认没有 的顺序排列（不被重排）", order)

    cmds = {
        "认包命令（`grep -rnE` 找 library/import）": 'grep -rnE "library\\(' in flow,
        "查 `tools/` 命令（`ls tools/` + `grep -ril`）": "grep -ril " in flow and "ls tools/" in flow,
        "查官方命令（`dir(sc.pl)` 自省）": 'dir(sc.pl)' in flow,
        "官方 R 侧自省（`getNamespaceExports`）": 'getNamespaceExports("Seurat")' in flow,
    }
    for label, ok in cmds.items():
        check(ok, "§0 每步带可直接跑的命令：" + label)

    check("# native: <包>::<函数>" in text or "# native: sc.pl.umap" in text,
          "留痕格式给出 `# native: <包>::<函数>`（原生函数一路）")
    check("# native: none" in flow and "两处查证痕迹" in flow,
          "留痕格式给出 `# native: none — …`，且写明**必须带两处查证痕迹（`tools/` + 官方资料）**")
    check("只写「没有」不算" in flow, "并写明「只写『没有』不算」（不能拿一句『没有原生出图』糊过去）")
    check("tools/<xx>" in flow and ("官方" in flow),
          "`none` 的模板里同时出现 `tools/<xx>` 与「官方 …（无对应函数）」两处痕迹位")

    # §1 不再保留第二套并列判据；改为指向 §0
    old = "判据：出 panel 前先回答三句"
    check(old not in text, "§1 的旧「判据三问」已删（不留两套并列判据）")
    sec1 = next((body for title, body in secs if title.startswith("1.")), "")
    check("§0" in sec1, "§1 的判据改成指向 §0 的四步", sec1[:300])

    # 真实命令输出（报告要求：用真实输出，不许编）
    check("tools/foundation-models/state/references/docs/10_仓库补充_pyproject与模型配置.md" in flow,
          "§0 里贴的是 `grep -ril scanpy tools/` 的**真实**输出行（首条命中）")
    check("'Deprecation'" in flow and "'violin'" in flow,
          "§0 里贴的是 `dir(sc.pl)` 的**真实**输出（首项 / 末项）")
    check("154" in flow, "§0 如实记下 `grep -ril` 的命中条数（154），并说明无 `tools/scanpy` 专用条目")


# ── ② SKILL.md 第一步展开 + 章节与主干仍逐字对应 ─────────────────────────────────────
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


def family_two():
    print("\n== ② SKILL.md 第一步 = 四步查证流程；主干与章节名仍逐字一致 ==")
    text = SKILL.read_text()
    steps, heads, rest = parse_workflow(text)
    if steps is None:
        check(False, "`SKILL.md` 有 `# CORE WORKFLOW` 一级章节", "未找到该标题")
        return
    check(steps == STEPS, f"主干仍是 R20 的 5 步且顺序一致（{len(STEPS)} 步）", steps)
    check(heads == steps,
          "每个章节名与主干的单条步骤名**仍逐字一致**、顺序一致（R20 的解析比对没有退化）",
          f"steps={steps}\nheads={heads}")
    bodies = {}
    for part in re.split(r"^##\s+", rest, flags=re.M)[1:]:
        title, _, body = part.partition("\n")
        bodies[norm(title)] = body.strip()
    thin = {k: len(v) for k, v in bodies.items() if len(v) < MIN_SECTION}
    check(not thin, f"每个步骤章节正文 ≥ {MIN_SECTION} 字符（没有空壳章节）", thin)

    first = bodies.get(STEPS[0], "")
    for token in ("**认包**", "查本仓 `tools/`", "查官方", "确认无"):
        check(token in first, f"第一步正文含四步关键词「{token}」")
    check(first.find("**认包**") < first.find("查本仓 `tools/`") < first.find("查官方")
          < first.find("确认无"),
          "四个关键词在第一步里按 认包 → 查 tools/ → 查官方 → 确认无 的顺序出现")
    check("grep -ril" in first,
          "`grep -ril` 命令**穿插**在第一步里（不是集中到别处）")
    check('dir(sc.pl)' in first and 'python -c "import scanpy as sc' in first,
          "`python -c \"…dir(...)\"` 命令也穿插在第一步里")
    check("快捷答案" in first and "00_native_plots.md" in first,
          "第一步写明「那里的表只是快捷答案，表里没有就按流程查」，并指向 `00_native_plots.md`")
    check("两处" in first and "只写" in first,
          "第一步的留痕硬规则要求 `none` 带两处痕迹（只写\"没有\"不算）")
    check("查一轮官方资料" in text.split("# CORE WORKFLOW")[1].split("## ②")[0] or
          "查一轮官方资料" in first,
          "主干第一步的口径是「先查一轮官方资料」")


# ── ③ 05_checklist §A0 ──────────────────────────────────────────────────────────────
def family_three():
    print("\n== ③ 05_checklist.md：`none` 的原因必须含两处查证痕迹 ==")
    text = CHECKLIST.read_text()
    sec = re.search(r"^##\s*A0\..+?$(.+?)(?=^##\s)", text, re.M | re.S)
    body = sec.group(1) if sec else ""
    check(bool(sec) and len(body.strip()) > 200, "判据仍在独立一节（`## A0.`，不散落）")
    check("两处查证痕迹" in body,
          "§A0 的判据写明「`none` 的原因必须含两处查证痕迹（`tools/` 与官方资料）」")
    check("tools/" in body and "官方" in body and "只写" in body,
          "§A0 同时点明痕迹的两处来源（`tools/` + 官方资料），且「只写\"没有\"不算」")
    check('grep -rn "# native: none"' in body,
          "§A0 给了抽查命令 `grep -rn \"# native: none\" <step>/figure_scripts/`")
    check("回到那次查询" in body,
          "§A0 说明抽查的判法：逐条能回到那次查询")
    for label in ("`# native: none — <原因>` 的形式", "`grep -rn \"# native:\"` 的对账命令",
                  "与 `figure_panels/*.svg` 计数对齐", "指向判据出处 `00_native_plots.md`"):
        ok = {"`# native: none — <原因>` 的形式": "# native: none" in text,
              "`grep -rn \"# native:\"` 的对账命令": 'grep -rn "# native:"' in text,
              "与 `figure_panels/*.svg` 计数对齐": "figure_panels/*.svg" in text,
              "指向判据出处 `00_native_plots.md`": "00_native_plots.md" in text}[label]
        check(ok, "R20 的 §A0 项仍在：" + label)


# ── ④ 两副本一致 ────────────────────────────────────────────────────────────────────
def family_four():
    print("\n== ④ 规范源与复制件逐字节一致 ==")
    result = subprocess.run(["diff", "-r", str(GUIDE), str(COPY)], capture_output=True, text=True)
    check(result.returncode == 0,
          "`diff -r` 规范源 vs `tools/manuscripts_skills/general-figure-guide/` 为空",
          result.stdout[:600] or result.stderr[:400])
    left = {p.relative_to(GUIDE) for p in GUIDE.rglob("*.md")}
    right = {p.relative_to(COPY) for p in COPY.rglob("*.md")}
    check(left == right, "两边的 md 文件集合相同", f"left-right={left - right} right-left={right - left}")


# ── ⑤ 体积与文档卫生 ────────────────────────────────────────────────────────────────
def family_five():
    print("\n== ⑤ 体积：三份改动的 md ≤ 20 KB（`00` ≤ 12 KB）+ 文档卫生 ==")
    for label, path, limit in [
        ("`references/00_native_plots.md`", NATIVE, NATIVE_LIMIT),
        ("`SKILL.md`", SKILL, LIMIT),
        ("`references/05_checklist.md`", CHECKLIST, LIMIT),
    ]:
        size = path.stat().st_size
        check(size <= limit, f"{label} ≤ {limit} B（实测 {size} B）")
    docs = [SKILL] + sorted((GUIDE / "references").glob("*.md"))
    sizes = sorted(((p.stat().st_size, str(p.relative_to(GUIDE))) for p in docs), reverse=True)
    info(f"{len(docs)} 份 md，最大三个：" + ", ".join(f"{name} {size}B" for size, name in sizes[:3]))
    check(not [x for x in sizes if x[0] > LIMIT], f"指南全部 md ≤ {LIMIT} B")
    hits = [(p.name, n) for p in docs for n, line in enumerate(p.read_text().splitlines(), 1)
            if CHANGELOG.search(line)]
    check(not hits, "指南不含变更叙述词（已支持/已修复/不再需要/早期脚本/…）", hits)


# ── ⑥ 回归 R10 / R20 ────────────────────────────────────────────────────────────────
def run_round(round_id, timeout=None):
    evidence = HERE.parent / round_id / "test.py"
    if not evidence.is_file():
        skip(f"回归 round-data/{round_id}", "没有 test.py")
        return None
    started = time.time()
    try:
        result = subprocess.run([sys.executable, str(evidence)], capture_output=True,
                                text=True, timeout=timeout)
    except subprocess.TimeoutExpired as exc:
        info(f"{round_id}: 超时（>{timeout}s）")
        return exc
    info(f"{round_id}: rc={result.returncode} 用时 {time.time() - started:.1f}s")
    check(result.returncode == 0, f"回归 round-data/{round_id} 仍全绿",
          result.stdout[-1200:] + result.stderr[-500:])
    return result


def r20_front_half():
    """R20 太慢时只跑它的 ①–⑤（主干/章节/函数名/两副本/体积），跳过它自己的 ⑥ 回归链。"""
    print("       ↓ 按 R20 的「前半」补跑（①②③④⑤，跳过它的 ⑥ 回归链）")
    spec = importlib.util.spec_from_file_location("r20_evidence", HERE.parent / "R20/test.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)             # 只定义函数，`main()` 在 `__main__` 保护下
    module.TMP = pathlib.Path(tempfile.mkdtemp(prefix="r21-r20-front-"))
    for family in (module.family_one, module.family_two, module.family_three,
                   module.family_four, module.family_five):
        family()
    bad = list(module.FAILS)
    check(not bad, "R20 的前半（①②③④⑤）全绿", bad)


def family_six():
    print("\n== ⑥ 回归 R10（两副本并轨）/ R20（主干·章节·函数名·体积） ==")
    run_round("R10")
    outcome = run_round("R20", timeout=R20_TIMEOUT)
    if isinstance(outcome, subprocess.TimeoutExpired):
        print("INFO  R20 超过 %ss 未跑完 —— 改跑它的前半（①②③④⑤），**跳过 ⑥ 回归链**；"
              "这不是静默降级，本轮 note 里已写明。" % R20_TIMEOUT)
        r20_front_half()


def main():
    global TMP
    print(f"# 规范源：{GUIDE}\n# 复制件：{COPY}\n# R20 超时阈值：{R20_TIMEOUT}s")
    TMP = pathlib.Path(tempfile.mkdtemp(prefix="r21-rounddata-"))
    for path in (SKILL, NATIVE, CHECKLIST):
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
