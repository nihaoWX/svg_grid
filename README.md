# svg_grid

是一个**纯rust写的**专门用作组图的库，它设计初衷是在生信领域（但不止于此，任何组图需求都可以尝试），与R和python接轨。

**工作流舒适**：
- 它开箱即用，只需一句话即可让agent组图/出图
- agent打基础，人精调，agent通过skill完成大部分工作，若细节不满意，除了告诉agent外还可以人工调整，svg也是一种适合人编辑的格式

# 为什么用它

R和python的组图很割裂，R的ggplot2系统和python的matplotlib不互通，现实的需求是可能会将两者混合组图，但是这带来一个问题，它们只能各自导出png组图，png在拉伸下变形，而svg格式解决了这一点。

- svg拉伸不变形：在svg_grid里，svg内的文字不会随拉伸而变形，行为很类似于在R里面用各大组图包组图，例如cowplot/patchwork等。
- svg真正实现R/python互通：它可能更有潜力，因为理论上是svg就能组图，不只是局限于R/python。
- svg是位图：那很清晰了。
- svg_grid也可以处理混合场景：例如png+svg混合组图。
- **与agent接轨**：仓库附带一整套出图+组图skill，开箱即用。并且作者正在尝试优化svg_grid让agent使用它更加节省token更加丝滑，不过作者推荐使用有视觉功能的agent。
- 低占用并且开源：你可能说我用ppt，ps，ai组图，但是这很重，svg_grid是rust，那很轻了。
- 仿照cowplot的丝滑组图体验：作者偏好cowplot的组图逻辑，因此本svg_grid的组图逻辑跟它很接近，本库开发的目的也是尝试弄一个cowplot通用版，因为cowplot只能在R使用（R的grob和ggplot2对象与python不互通）。
- svg适合人工调整：如果agent的图你怎么都不满意，svg本身也适合人工编辑，可以使用inkscape这款免费软件编辑。

# 怎么用

- 先克隆仓库
- 在win/linux/macOS编译它
- 然后阅读这个skill，agent自己会懂：svg_grid/docs/general-figure-guide。或许需要自己调整一下，因为这个skill以及svg_grid仍在开发

# 注意事项

- 你会发现这个skill它不止涵盖组图，更像是一个agent自动写手稿+出图+组图的一个工作流之中的一个skill。这一点没有错，因为我懒得修改了，后续打算开源整一套工作流程，让整个文章的分析+写作流程在agent+人的协作模式下丝滑进行。
