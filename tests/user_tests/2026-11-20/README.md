我从`/home/nihao/bioagentforge/research/白藜芦醇胰腺癌/steps/02_分子对接/pandamap_all`选了一些图，然后convert到这里，命令是
`svg_grid convert --input xxx.svg --output xxx.svg`
然后我通过这种命令进行组图, width是我乱填的
`svg_grid plot-grid --input xxx1.svg --input xxx2.svg --input xxx3.svg  --output xxxx.svg --ncol x --width 1000 --labels`

# 我实际体验有很多问题

## convert
- convert不应该手动完成，如果`svg_grid plot-grid`遇到需要convert，直接自动convert了
- convert把原svg损坏了，就是这个`pandamap_all`的svg，原svg外面那一圈配体是文字，convert后把这些文字变成了polygon，导致这些字在视觉上有异常

# `svg_grid plot-grid`本身缺陷

## 指定`ncol`排版异常
我运行了`svg_grid plot-grid --input pandamap_ALB_A.svg --input pandamap_ALB_B.svg --input pandamap_ALB_C.svg --input pandamap_ALB_D.svg --input pandamap_BACE1_A.svg --output ABCDE.svg --ncol 2 --width 1000 --labels`
结果发现排版异常，最后一个子图，即E，只有它被拉长了，其余4个都是正常的，这种情况下应该是全部都平均被拉长。
