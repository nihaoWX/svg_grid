# 在 Windows 上安装 svg_grid

**一句话**：Windows 上**不提供安装脚本、也不预编译二进制**——用 Rust 工具链在本机编译，
然后把 `svg_grid.exe`、字体、语料、skill 放到位。全部步骤都是复制粘贴，不需要 sudo，也不需要 WSL。

> 为什么没有 `install.ps1`：`install.sh` 依赖 bash + rsync，且要同时管二进制/数据根/字体/语料/skill 五件事；
> 在 Windows 上这五件事各自只有一两条命令（下面），再维护一个分叉脚本并在另一台机器上验证，成本大于收益。

---

## 0. 前置

- Windows 10/11 x64
- **Rust**（MSVC 工具链）：<https://rustup.rs>，默认 `stable-x86_64-pc-windows-msvc`
- **MSVC 链接器**：Visual Studio Build Tools 的「使用 C++ 的桌面开发」。rustup 装 MSVC 目标时会提示
- Git（也可以直接下 zip）
- **不需要**：Python、CUDA、msys2/cygwin。依赖全是纯 Rust（resvg / ravif / fontdb），
  所以 PNG 与 **AVIF** 都不需要外部库

## 1. 编译

```powershell
git clone <repo-url>
cd <repo>\tools\svg_grid
cargo build --release
```

产物：`tools\svg_grid\target\release\svg_grid.exe`。

首次编译要联网拉 crate（这里不要加 `--offline`；`install.sh` 用的 `--offline` 只适用于依赖已在本地缓存的机器）。
编译完可以顺手跑一次单元测试（不需要语料/字体）：

```powershell
cargo test --release
```

## 2. 把 exe 放进 PATH

```powershell
New-Item -ItemType Directory -Force "$env:LOCALAPPDATA\Programs\svg_grid" | Out-Null
Copy-Item .\target\release\svg_grid.exe "$env:LOCALAPPDATA\Programs\svg_grid\"
$user = [Environment]::GetEnvironmentVariable("Path", "User")
[Environment]::SetEnvironmentVariable("Path", "$user;$env:LOCALAPPDATA\Programs\svg_grid", "User")
```

**重开一个终端**，然后：

```powershell
svg_grid --version
```

（也可以直接 `Copy-Item` 到任意已经在 PATH 里的目录，比如 `%USERPROFILE%\.local\bin`。）

## 3. 建数据根 `%USERPROFILE%\svg_grid\`

svg_grid 的字体与语料都放在**用户级数据根**：运行时按 `HOME` → `USERPROFILE` 的顺序取家目录，
再用 `<home>\svg_grid\`。它**从不读系统字体**，所以字体必须显式放进 `fontlib\`。

```powershell
New-Item -ItemType Directory -Force "$env:USERPROFILE\svg_grid\fontlib", "$env:USERPROFILE\svg_grid\fixtures" | Out-Null
```

> 用 Git Bash 时它通常会设 `HOME`。请让它等于 `%USERPROFILE%`，否则两个 shell 读到的是两个数据根：
> `setx HOME "%USERPROFILE%"`（或干脆只用 PowerShell / cmd）。

## 4. 字体：把 Arial / Times New Roman 复制进 `fontlib`

Windows 自带这两个字体，但 svg_grid 只读 `fontlib\`——**复制**（不是链接、不是指向 `C:\Windows\Fonts`）：

```powershell
$f = "$env:WINDIR\Fonts"
Copy-Item "$f\arial.ttf","$f\arialbd.ttf","$f\ariali.ttf","$f\arialbi.ttf" "$env:USERPROFILE\svg_grid\fontlib\"
Copy-Item "$f\times.ttf","$f\timesbd.ttf","$f\timesi.ttf","$f\timesbi.ttf" "$env:USERPROFILE\svg_grid\fontlib\"

svg_grid plot-grid --list-fonts
# 期望看到： Arial / Times New Roman
```

- 面板内容里的中文/日文如果要用到，把对应字体（例如 `msyh.ttc`、`simsun.ttc`）也复制进来。
- Arial / Times New Roman 是微软授权字体：**不要提交进仓库**（`fontlib\` 里只有 `README.md` 与 `setup.sh` 入库）。

## 5. 语料：给 `doctor` 自检用

```powershell
Copy-Item -Recurse -Force .\tests\fixtures\* "$env:USERPROFILE\svg_grid\fixtures\"
svg_grid doctor
# 期望最后一行： summary: 57 pass, 0 fail, 1 skip
```

不想复制也可以直接从克隆里跑：

```powershell
svg_grid doctor --fixtures <repo>\tools\svg_grid\tests\fixtures
```

## 6. 装 skill（agent 靠它发现这个工具）

组图工作流是一份 skill（`docs\general-figure-guide\`：`SKILL.md` + `references\`）。装到 harness 的 skill 目录，
**只拷这两个东西**（不要拷 `ARCHITECTURE\`、`src\`、`PROGRESS.md`），且**目录名必须等于 `SKILL.md` 里的 `name:`**
（`general-figure-guide`，不一致 harness 会静默跳过）：

```powershell
$dest = "$env:USERPROFILE\.commandcode\skills\general-figure-guide"
New-Item -ItemType Directory -Force $dest | Out-Null
Copy-Item -Recurse -Force .\docs\general-figure-guide\SKILL.md $dest\
Copy-Item -Recurse -Force .\docs\general-figure-guide\references $dest\
```

验证：`cmd skills list` 里应能看到 `general-figure-guide`。

## 7. 验收（四条，都可判定）

```powershell
svg_grid --version                                  # 1. 二进制在 PATH 上
svg_grid plot-grid --list-fonts                     # 2. 列出 Arial / Times New Roman
svg_grid doctor                                     # 3. 57 pass / 0 fail / 1 skip
svg_grid plot-grid --input .\tests\fixtures\ggplot2\bars.svg `
                   --output out.svg --png out.png --labels A     # 4. 真出一张，且 out.png 不为空白
```

## 8. 常见问题

| 症状 | 原因 / 修法 |
|---|---|
| `link.exe not found`、`LINK : fatal error LNK1181` | 缺 MSVC 链接器：装 Visual Studio Build Tools 的「使用 C++ 的桌面开发」，装完**重开终端** |
| `cargo` 不是内部或外部命令 | `%USERPROFILE%\.cargo\bin` 不在 PATH；加上后重开终端 |
| 编译报 `error: linker `cc` not found` | 当前是 GNU 目标：`rustup default stable-x86_64-pc-windows-msvc` |
| `no fonts in ...\svg_grid\fontlib` | 第 4 步没做，或拷到了别的目录；`--font-dir <dir>` 可临时指定 |
| `doctor`：找不到 fixture 目录 | 第 5 步没做；或用 `--fixtures <克隆里的 tests\fixtures>` |
| 图里中文显示成方块 | `fontlib` 里没有中文字体：把 `msyh.ttc` 等复制进去 |
| 路径含中文/空格 | 支持；在 PowerShell 里给带空格的路径加引号即可 |
| `已存在的文件` 之类的复制错误 | 加 `-Force` 覆盖（本指引里的命令都已带） |

## 9. 更新

Windows 上没有安装脚本，所以**改完代码（或改了 guide）之后手动重做第 2 步（和第 6 步）**：

```powershell
cd <repo>\tools\svg_grid
git pull
cargo build --release
Copy-Item -Force .\target\release\svg_grid.exe "$env:LOCALAPPDATA\Programs\svg_grid\"
# 改了 docs\general-figure-guide\ 就再跑一次第 6 步
```
