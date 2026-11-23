# 把独立 `convert/` crate 并入主库 `src/convert/`（R42）

## 现象 / 任务

`convert/` 一直是一个独立 crate：自己的 `Cargo.toml`、自己的 `src/main.rs`（二进制
`svg_grid_convert`）、自己的 `target/`、自己的集成测试 `convert/tests/cli.rs`。R18 起 `install.sh`
只装一个二进制（`svg_grid convert` 是子命令，转换器以库的形式编进来），但 crate 本身仍在仓库里，
于是同一份代码有两个构建单元、两套测试目标，`docs/` 与 `tests/` 里的 30 余处脚本/证据还要在一
"独立二进制"和"子命令"之间挑一个。

用户要求本轮做**纯机械搬迁**：把 `convert/` 整个并入主库，语义零变化。

## 复现（搬迁前后语义等价的判定方法）

以 base `3a8416e` 的 release 构建为对照（worktree `/tmp/r42_base`，`target/release/svg_grid`
+ `convert/target/release/svg_grid_convert`）：

```
# 1) 220 条 tests/**/*.svg 的转换产物
新: svg_grid convert --input F --output out.svg
旧: svg_grid_convert --input F --output out.svg
⇒ 逐字节相同（both ok 179 / 两边同样 41 条 rc=2 / byte-changed 0）

# 2) 58 条组合 fixture
plot-grid --input tagged.svg --width 600 --output o.svg --output-png o.png --png-dpi 300
⇒ 逐字节相同（0 differ；mathtext 两边都 rc=2）

# 3) --plan / --report
--plan stdout 逐字节相同；--report stdout 只差程序名前缀（`svg_grid_convert:` → `svg_grid convert:`）

# 4) --help
diff 只有两处：都还在描述"独立的 svg_grid_convert 二进制"
```

## 根因（结构层面）

- 转换器的库代码只依赖 `quick-xml` 与 `png`，两者主 crate 本来就依赖；独立 crate 带来的只是
  重复的依赖解析、第二个 `target/`、第二套测试目标（`convert/tests/cli.rs` 14 条）与第二份
  安装/路径故事（`STANDALONE`、`--help` 里的"独立二进制"字样、证据脚本的默认路径）。
- 主 crate 的 bin 部分（`src/cli/**`）通过 `use svg_grid::…` 访问库，因此库必须把转换器暴露成
  `pub mod convert`；而库内部原来的 `crate::…` 在模块化后必须变成 `crate::convert::…`。

## 修法

1. **移动**（`git mv`，保留历史）：
   - `convert/src/lib.rs` → `src/convert/mod.rs`；
   - 其余 27 个 `convert/src/*.rs`（含 `translate/**` 10 个）→ `src/convert/` 对应位置；
   - `convert/tests/cli.rs` → `tests/selftest/convert_cli/mod.rs`，`tests/selftest/main.rs` 加
     `mod convert_cli;`；
   - 删除 `convert/src/main.rs`、`convert/Cargo.toml`、`convert/Cargo.lock`、`convert/README.md`、
     `convert/target/`；`Cargo.toml` 删掉 `svg_grid_convert = { path = "convert" }`。
2. **模块化**：`src/lib.rs` 增加 `pub mod convert;`（`src/cli/**` 是 bin crate 的模块，只能走
   `svg_grid::convert::…`，不能写 `crate::convert`）。`src/convert/**` 内 106 处 `crate::` →
   `crate::convert::`。
3. **删 standalone 入口**：`src/convert/cli.rs` 去掉 `pub const STANDALONE`、`pub fn main`、
   `run_args`、`run_args_code`；保留 `run_args_as` / `run_args_code_as`（调用方传程序名）。
   诊断前缀因此只可能是调用方给的名字：`svg_grid convert: `。
4. **调用面**：`src/cli/convert.rs`（`svg_grid::convert::cli::{FLAGS, run_args_as}`）、
   `src/cli/doctor/run.rs`（`svg_grid::convert::Options` / `cli::run_in`，message 改 `stage 1 (convert)`）、
   `src/cli/doctor/mod.rs`（`CONVERT_STAGE = "in-process (svg_grid::convert module)"`）、
   `src/plan.rs`（`crate::convert::{…}`、`crate::convert::convert(...)`）。
5. **install.sh**：`stale_convert` 清理段保留（还装过旧版的机器要清），注释与 `--check` 的 why
   文案改成"模块"口径；`would install` 仍只有二进制 + skill 两行。
6. **引用面**：`tests/fig2_mixed/*`、`tests/s22_repro/*`、`tests/mpl_dialect/run_v1.sh`、
   `tests/rdkit_dialect/run.sh`、`tests/s22_repro/out/skilltest*/` 一律改为
   `<repo>/target/release/svg_grid convert …`；`tests/fixtures/MANIFEST.toml` 的说明行同步。
   `docs/PITFALLS/round-data/R15,R16,R17,R18,R19,R20,R25,R37,R38,R39` 的证据脚本改为指向
   `target/debug/svg_grid` + `convert` 子命令；断言强度不放宽，只把"独立二进制/前缀不同"
   这类前提断言换成搬迁后的不变量（逐轮 `modify` 记录）。

## 验证

| 项 | 结果 |
| --- | --- |
| 单 crate `cargo test` | **394** = 211 lib + 59 bin + 124 selftest，0 fail（原 273 + 121） |
| `cargo fmt --all --check` / `clippy -D warnings` | rc=0 |
| `./install.sh` | rc=0（`svg_grid 0.1.0`，fixtures 58，fonts 8） |
| `install.sh --dry-run` | 2 × `would install`（二进制 + skill） |
| `svg_grid doctor` | 57 pass / 0 fail / 1 skip（matplotlib/mathtext） |
| 220 fixture 转换 | 0 byte change，两边同样 41 条失败，幂等 0，悬空引用 0 |
| 58 组合 `--width 600` | 0 byte change，1 条两边都失败（mathtext） |
| `--plan` / `--report` | 逐字节相同 / 只差程序名前缀 |
| `svg_grid convert --help` | 2 处 hunk（见上） |
| `pitfalls-manager verify` | rc=0（38 checks） |
| 旧证据 | R8/R9/R15/R16/R17/R18/R20/R25/R28/R31/R35/R36/R37/R38/R39/R40/R41 顶层全 rc=0 |

## 遗留

- `git grep svg_grid_convert` 仍命中的是**有理由保留**的历史/说明文本：`install.sh` 的
  `stale_convert` 段（清理旧安装）、`docs/PITFALLS/round-data/*` 的历史说明、`ARCHITECTURE/*.md`、
  `PROGRESS.md`、`docs/HANDOFF.md`（后三者由 Arranger 更新）。
- `--help` 的两处差异是"旧文案描述已删除的二进制"，因此必须改；其余帮助文本一字未动。
- 转换器自身语义一行未改（本轮零行为变化），R41 遗留的 P2（混合单位 tile 继承）不在本轮范围。
