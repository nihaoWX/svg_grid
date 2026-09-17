rm(list = ls()):gc()
library(qs2)
library(cowplot)
library(ggplot2)
library(png)
library(foreach)

setwd("E:/ziliao/keyan/果子人白藜芦醇胰腺癌")

plot_dir<-"Figure"
if(!dir.exists(plot_dir)){dir.create(plot_dir)}

# ====Figure 1====

p1<-qs_read("1_数据整理/geo_data/pca.qs2")
p2<-qs_read("2_AB基因取交/Resveratrol_1,2-Diphenylethylene.qs2")
p3<-qs_read("2_DEG/DE_heatmap.qs2")
p4<-qs_read("2_DEG/DE_volcano.qs2")
pp1<-plot_grid(p1[[1]],p1[[2]],ncol=2,labels = c("A","B"),label_fontfamily = "serif",#label的设置
               label_size = 30,label_fontface = "bold",#label的设置
               label_x = -0.04,label_y = 1)

pp2<-plot_grid(p2,p4,ncol=2,labels = c("C","D"),label_fontfamily = "serif",#label的设置
               label_size = 30,label_fontface = "bold",#label的设置
               label_x = -0.04,label_y = 1)

ppp1<-plot_grid(pp1,pp2,p3,ncol = 1,rel_heights = c(0.2,0.2,0.6),
                labels = c("","","E"),label_fontfamily = "serif",#label的设置
                label_size = 30,label_fontface = "bold",#label的设置
                label_x = -0.02,label_y = 1)

ppp1 <- ggdraw(ppp1) + 
  theme(plot.margin = margin(t = 35, r = 35, b = 35, l = 35, unit = "pt"),
        plot.background = element_rect(fill = "white", color = NA))

png(file.path(plot_dir,"Figure 1.png"),width = 4000,height = 4000*(2^0.5),res = 300)
print(ppp1)
dev.off()
rm(p1,p2,p3,p4,pp1,pp2,ppp1);gc()

# ====Figure 1S====

p1<-qs_read("3_DEG与感兴趣基因集取交/A_B_DEG.qs2")
png(file.path(plot_dir,"Figure S1.png"),width = 2500,height = 2000,res = 300)
p1<-plot_grid(p1)
print(p1)
dev.off()
rm(p1);gc()

# ====Figure 2S====

p1<-qs_read("12_单细胞/1_QC/tumor_QC_violin.qs2")
p2<-qs_read("12_单细胞/1_QC/tumor_QC_scatter_metrics.qs2")
p3<-qs_read("12_单细胞/1_QC/normal_QC_violin.qs2")
p4<-qs_read("12_单细胞/1_QC/normal_QC_scatter_metrics.qs2")

p1<-plot_grid(plotlist = p1,ncol = 3)
p2<-plot_grid(plotlist = p2,ncol = 2)
p3<-plot_grid(plotlist = p3,ncol = 3)
p4<-plot_grid(plotlist = p4,ncol = 2)

pp1<-plot_grid(p1,p2,p3,p4,ncol = 1,labels = "AUTO",label_fontfamily = "serif",#label的设置
               label_size = 30,label_fontface = "bold",#label的设置
               label_x = 0,label_y = 1.02)

pp1 <- ggdraw(pp1) + 
  theme(plot.margin = margin(t = 35, r = 35, b = 35, l = 35, unit = "pt"),
        plot.background = element_rect(fill = "white", color = NA))

png(file.path(plot_dir,"Figure S2.png"),width = 4000,height = 4000*(2^0.5),res = 300)
print(pp1)
dev.off()
rm(p1,p2,p3,p4,pp1);gc()


# ====Figure S3====

p1<-qs_read("12_单细胞/2_PCA_UMAP/tumor_UMAP_before_inte.qs2")
p2<-qs_read("12_单细胞/2_PCA_UMAP/tumor_UMAP_after_inte.qs2")
p3<-qs_read("12_单细胞/2_PCA_UMAP/normal_UMAP_before_inte.qs2")
p4<-qs_read("12_单细胞/2_PCA_UMAP/normal_UMAP_after_inte.qs2")
p5<-qs_read("12_单细胞/3_marker_annotation/tumor_celltype_composition.qs2")
p6<-qs_read("12_单细胞/3_marker_annotation/normal_celltype_composition.qs2")

pp1<-plot_grid(p1[[2]],p2[[2]],ncol = 2)
pp2<-plot_grid(p3[[2]],p4[[2]],ncol = 2)
pp3<-plot_grid(p2[[1]],p4[[1]],ncol = 2)
pp4<-plot_grid(p5[[1]],p6[[1]],ncol = 2)

ppp1<-plot_grid(pp1,pp2,pp3,pp4,ncol=1,labels = "AUTO",label_fontfamily = "serif",#label的设置
                label_size = 30,label_fontface = "bold",#label的设置
                label_x = -0.02,label_y = 1.02)

ppp1 <- ggdraw(ppp1) + 
  theme(plot.margin = margin(t = 35, r = 35, b = 35, l = 35, unit = "pt"),
        plot.background = element_rect(fill = "white", color = NA))

png(file.path(plot_dir,"Figure S3.png"),width = 4000,height = 4000*(2^0.5),res = 300)
print(ppp1)
dev.off()
rm(p1,p2,p3,p4,p5,p6,pp1,pp2,pp3,pp4,ppp1);gc()

# ====Figure S4====

# 读入png转为ggplot对象
read_png_as_plot <- function(path) {
  img <- readPNG(path)
  ggdraw() + draw_image(img, x = 0, y = 0, width = 1, height = 1)
}

p1 <- read_png_as_plot("12_单细胞/1_infercnvpy/pdac_all_celltype_cnv_all_heatmap.png")
p2 <- read_png_as_plot("12_单细胞/1_infercnvpy/pdac_cnv_leiden_cnv_all_heatmap.png")
p3 <- read_png_as_plot("12_单细胞/1_infercnvpy/pdac_normal_cell_cnv_leiden_cnv_all_heatmap.png")
p4 <- read_png_as_plot("12_单细胞/1_infercnvpy/pdac_tumor_cell_cnv_leiden_cnv_all_heatmap.png")
p5 <- read_png_as_plot("12_单细胞/1_infercnvpy/cnv_umap.png")

pp1<-plot_grid(p1,p2,p3,p4,ncol = 2,labels = "AUTO",label_fontfamily = "serif",#label的设置
               label_size = 30,label_fontface = "bold",#label的设置
               label_x = -0.06,label_y = 1.02)
pp1<-plot_grid(NULL,pp1,NULL,rel_widths=c(0.08,1-0.16,0.08),ncol = 3)

ppp1<-plot_grid(pp1,p5,ncol=1,rel_heights = c(1, 1),labels = c("","E"),label_fontfamily = "serif",#label的设置
                label_size = 30,label_fontface = "bold",#label的设置
                label_x = 0.06,label_y = 1)

png(file.path(plot_dir,"Figure S4.png"), width = 4000, height = 4000*(2^0.5), res = 300)
print(ppp1)
dev.off()

rm(p1,p2,p3,p4,p5,pp1,ppp1);gc()


# ====Figure S5====

# 读入png转为ggplot对象
read_png_as_plot <- function(path) {
  img <- readPNG(path)
  ggdraw() + draw_image(img, x = 0, y = 0, width = 1, height = 1)
}

p1 <- read_png_as_plot("12_单细胞/celltype_subsets/celltypist/celltypist_tumor_immune_dotplot.png")
p2 <- read_png_as_plot("12_单细胞/celltype_subsets/celltypist/celltypist_normal_immune_dotplot.png")

pp1<-plot_grid(p1,p2,ncol = 2,labels = "AUTO",label_fontfamily = "serif",#label的设置
               label_size = 30,label_fontface = "bold",#label的设置
               label_x = 0,label_y = 1)

png(file.path(plot_dir,"Figure S5.png"), width = 4000, height = 3300, res = 300)
print(pp1)
dev.off()

rm(p1,p2,pp1);gc()

# ====Figure S6====

p1<-qs_read("12_单细胞/celltype_subsets/2_PCA_UMAP/tumor_immune_UMAP_before_inte.qs2")
p2<-qs_read("12_单细胞/celltype_subsets/2_PCA_UMAP/tumor_immune_UMAP_after_inte.qs2")
p3<-qs_read("12_单细胞/celltype_subsets/2_PCA_UMAP/tumor_non_immune_UMAP_before_inte.qs2")
p4<-qs_read("12_单细胞/celltype_subsets/2_PCA_UMAP/tumor_non_immune_UMAP_after_inte.qs2")
p5<-qs_read("12_单细胞/celltype_subsets/3_marker_annotation/tumor_immune_all_celltype_celltype_composition.qs2")
p6<-qs_read("12_单细胞/celltype_subsets/3_marker_annotation/tumor_non_immune_all_celltype_celltype_composition.qs2")

pp1<-plot_grid(p1[[2]],p2[[2]],ncol = 2)
pp2<-plot_grid(p3[[2]],p4[[2]],ncol = 2)
pp3<-plot_grid(p2[[1]],p5[[1]],ncol = 1)
pp4<-plot_grid(p4[[1]],p6[[1]],ncol = 2)

ppp1<-plot_grid(pp1,pp2,ncol=2,labels = c("A","B"),label_fontfamily = "serif",#label的设置
                label_size = 30,label_fontface = "bold",#label的设置
                label_x = -0.02,label_y = 1.06)

ppp2<-plot_grid(pp3,pp4,ncol=1,rel_heights = c(0.75,0.25),labels = c("C","D"),label_fontfamily = "serif",#label的设置
                label_size = 30,label_fontface = "bold",#label的设置
                label_x = -0.01,label_y = 1.02)

pppp1<-plot_grid(ppp1,ppp2,ncol=1,rel_heights = c(0.15,0.85))

pppp1 <- ggdraw(pppp1) + 
  theme(plot.margin = margin(t = 35, r = 35, b = 35, l = 35, unit = "pt"),
        plot.background = element_rect(fill = "white", color = NA))

png(file.path(plot_dir,"Figure S6.png"),width = 4000,height = 4000*(2^0.5),res = 300)
print(pppp1)
dev.off()
rm(p1,p2,p3,p4,p5,p6,pp1,pp2,pp3,pp4,ppp1,ppp2,pppp1);gc()


# ====Figure S7====

p1<-qs_read("12_单细胞/celltype_subsets/2_PCA_UMAP/normal_immune_UMAP_before_inte.qs2")
p2<-qs_read("12_单细胞/celltype_subsets/2_PCA_UMAP/normal_immune_UMAP_after_inte.qs2")
p3<-qs_read("12_单细胞/celltype_subsets/2_PCA_UMAP/normal_non_immune_UMAP_before_inte.qs2")
p4<-qs_read("12_单细胞/celltype_subsets/2_PCA_UMAP/normal_non_immune_UMAP_after_inte.qs2")
p5<-qs_read("12_单细胞/celltype_subsets/3_marker_annotation/normal_immune_all_celltype_celltype_composition.qs2")
p6<-qs_read("12_单细胞/celltype_subsets/3_marker_annotation/normal_non_immune_all_celltype_celltype_composition.qs2")

pp1<-plot_grid(p1[[2]],p2[[2]],ncol = 2)
pp2<-plot_grid(p3[[2]],p4[[2]],ncol = 2)
pp3<-plot_grid(p2[[1]],p5[[1]],ncol = 1)
pp4<-plot_grid(p4[[1]],p6[[1]],ncol = 2)

ppp1<-plot_grid(pp1,pp2,ncol=2,labels = c("A","B"),label_fontfamily = "serif",#label的设置
                label_size = 30,label_fontface = "bold",#label的设置
                label_x = -0.02,label_y = 1.06)

ppp2<-plot_grid(pp3,pp4,ncol=1,rel_heights = c(0.75,0.25),labels = c("C","D"),label_fontfamily = "serif",#label的设置
                label_size = 30,label_fontface = "bold",#label的设置
                label_x = -0.01,label_y = 1.02)

pppp1<-plot_grid(ppp1,ppp2,ncol=1,rel_heights = c(0.15,0.85))

pppp1 <- ggdraw(pppp1) + 
  theme(plot.margin = margin(t = 35, r = 35, b = 35, l = 35, unit = "pt"),
        plot.background = element_rect(fill = "white", color = NA))

png(file.path(plot_dir,"Figure S7.png"),width = 4000,height = 4000*(2^0.5),res = 300)
print(pppp1)
dev.off()
rm(p1,p2,p3,p4,p5,p6,pp1,pp2,pp3,pp4,ppp1,ppp2,pppp1);gc()


# ====Figure S8-S11====

plot_grid2<-function(input_dir,group,ppp_ncol,label_size,p_width,p_height,output_dir,p_name){
  p_name_list<-list.files(input_dir,pattern = paste0("^",group,".*.qs2$"))
  print(p_name_list)
  umap_groups<-p_name_list[grepl(paste0("^",group,"(.*)-umap.qs2$"),p_name_list)]
  print(umap_groups)
  pseudotime_groups<-p_name_list[grepl(paste0("^",group,"(.*)-pseudotime.qs2$"),p_name_list)]
  print(pseudotime_groups)
  id_list<-sub("^.*-(.*)-.*$", "\\1", umap_groups)
  print(id_list)
  p_group_list<-list()
  foreach(id=id_list) %do% {
    p_group_list[[id]][[1]]<-pseudotime_groups[grepl(paste0(".*-",id,"-.*"),pseudotime_groups)]
    p_group_list[[id]][[2]]<-umap_groups[grepl(paste0(".*-",id,"-.*"),umap_groups)]
  }
  pp_list<-list()
  foreach(i=names(p_group_list)) %do% {
    message("====",i,"====")
    p1<-qs_read(file.path(input_dir,p_group_list[[i]][[1]]))
    p2<-qs_read(file.path(input_dir,p_group_list[[i]][[2]]))
    p1<-ggdraw(p1)
    #pp<-plot_grid(p1,p2[[1]],p2[[2]],ncol=3,rel_widths = c(1,0.8,0.8))
    pp<-plot_grid(p1,p2[[1]],ncol=2,rel_widths = c(1,0.8))
    
    # now add the title
    title <- ggdraw() + 
      draw_label(
        paste0(i),
        fontface = 'bold',
        x = 0.5,
        hjust = 0
      ) +
      theme(
        # add margin on the left of the drawing canvas,
        # so title is aligned with left edge of first plot
        plot.margin = margin(0, 0, 0, 7)
      )
    
    pp<-plot_grid(
      title, pp,
      ncol = 1,
      # rel_heights values control vertical title margins
      rel_heights = c(0.1, 1)
    )
    
    pp_list[[i]]<-pp
  }
  ppp<-plot_grid(plotlist = pp_list,ncol=ppp_ncol,
                 labels = "AUTO",label_fontfamily = "serif",label_size = label_size,label_fontface = "bold",
                 label_x = -0.04,label_y = 1)
  ppp<- ggdraw(ppp) +
    theme(
      plot.margin = margin(20, 20, 20, 40)#上右下左
    )
  png(file=file.path(output_dir,paste0(p_name,".png")),width = p_width,height = p_height,res=300)
  print(ppp)
  dev.off()
}
#qs2文件的命名标准必须为group_细胞类型_umap(pseudotime).qs，下划线间的部分不能有下划线
input_dir<-"12_单细胞/celltype_subsets/5_monocle3"
plot_grid2(input_dir = input_dir,group = "seu1_i-",
           label_size=45,ppp_ncol=3,p_width=6000*(2^0.5),p_height = 6000,
           output_dir = plot_dir,p_name = "Figure S8")

plot_grid2(input_dir = input_dir,group = "seu1_n_i-",
           label_size=45,ppp_ncol=3,p_width=6000*(2^0.5),p_height = 6000,
           output_dir = plot_dir,p_name = "Figure S9")

plot_grid2(input_dir = input_dir,group = "seu2_i-",
           label_size=45,ppp_ncol=3,p_width=6000*(2^0.5),p_height = 6000,
           output_dir = plot_dir,p_name = "Figure S10")

plot_grid2(input_dir = input_dir,group = "seu2_n_i-",
           label_size=35,ppp_ncol=1,p_width=4000,p_height = 4000*(2^0.5),
           output_dir = plot_dir,p_name = "Figure S11")


# ====Figure S12====

input_dir<-"12_单细胞/celltype_subsets/6_sig_genes/GLM/sig_gene_time_plot"
p1<-qs_read(file.path(input_dir,"cds_res1_i_cds_res2_i-seu1_i-CD4T_seu2_i-CD4T-sig_gene_time_plot.qs2"))
p2<-qs_read(file.path(input_dir,"cds_res1_i_cds_res2_i-seu1_i-CD4T_seu2_i-CD4T-sig_gene_time_plot_umap.qs2"))
p3<-qs_read(file.path(input_dir,"cds_res1_i_cds_res2_i-seu1_i-Mono_Macro_seu2_i-Mono_Macro-sig_gene_time_plot.qs2"))
p4<-qs_read(file.path(input_dir,"cds_res1_i_cds_res2_i-seu1_i-Mono_Macro_seu2_i-Mono_Macro-sig_gene_time_plot_umap.qs2"))

plot_title<-function(p,group){

  pp<-plot_grid(p[[1]],p[[2]],ncol = 2)
  
  # now add the title
  title <- ggdraw() + 
    draw_label(
      paste0(group),
      fontface = 'bold',
      x = 0.05,
      hjust = 0
    ) +
    theme(
      # add margin on the left of the drawing canvas,
      # so title is aligned with left edge of first plot
      plot.margin = margin(0, 0, 0, 7)
    )
  
  pp<-plot_grid(
    title, pp,
    ncol = 1,
    # rel_heights values control vertical title margins
    rel_heights = c(0.1, 1)
  )
  
  return(pp)
}

pp1<-plot_title(p1,"left: tumor, right: normal, CD4T")
pp2<-plot_title(p2,"left: tumor, right: normal, CD4T")
pp3<-plot_title(p3,"left: tumor, right: normal, Mono_Macro")
pp4<-plot_title(p4,"left: tumor, right: normal, Mono_Macro")

ppp1<-plot_grid(pp1,pp2,pp3,pp4,ncol=1,labels = "AUTO",label_fontfamily = "serif",#label的设置
                label_size = 30,label_fontface = "bold",#label的设置
                label_x = -0.02,label_y = 1)

ppp1 <- ggdraw(ppp1) + 
  theme(plot.margin = margin(t = 35, r = 35, b = 35, l = 35, unit = "pt"),
        plot.background = element_rect(fill = "white", color = NA))

png(file.path(plot_dir,"Figure S12.png"),width = 4000,height = 4000*(2^0.5),res = 300)
print(ppp1)
dev.off()

rm(p1,p2,p3,p4,pp1,pp2,pp3,pp4,ppp1);gc()

# ====Figure S13====

input_dir<-"12_单细胞/celltype_subsets/6_sig_genes/GLM/sig_gene_time_plot"
p1<-qs_read(file.path(input_dir,"cds_res1_i_cds_res2_i-seu1_i-CD8T_seu2_i-CD8T-sig_gene_time_plot.qs2"))
p2<-qs_read(file.path(input_dir,"cds_res1_i_cds_res2_i-seu1_i-CD8T_seu2_i-CD8T-sig_gene_time_plot_umap.qs2"))

plot_title<-function(p,title,ncol){
  
  pp<-plot_grid(p[[1]],p[[2]],ncol = ncol)
  
  # now add the title
  title <- ggdraw() + 
    draw_label(
      paste0(title),
      fontface = 'bold',
      x = 0.05,
      hjust = 0
    ) +
    theme(
      # add margin on the left of the drawing canvas,
      # so title is aligned with left edge of first plot
      plot.margin = margin(0, 0, 0, 7)
    )
  
  pp<-plot_grid(
    title, pp,
    ncol = 1,
    # rel_heights values control vertical title margins
    rel_heights = c(0.1, 1)
  )
  
  return(pp)
}

pp1<-plot_title(p1,"left: tumor, right: normal, CD8T",2)
pp2<-plot_title(p2,"above: tumor, below: normal, CD8T",1)

ppp1<-plot_grid(pp1,pp2,ncol=1,labels = "AUTO",label_fontfamily = "serif",#label的设置
                label_size = 30,label_fontface = "bold",#label的设置
                label_x = -0.02,label_y = 1)

ppp1 <- ggdraw(ppp1) + 
  theme(plot.margin = margin(t = 35, r = 35, b = 35, l = 35, unit = "pt"),
        plot.background = element_rect(fill = "white", color = NA))

png(file.path(plot_dir,"Figure S13.png"),width = 4000,height = 4000*(2^0.5),res = 300)
print(ppp1)
dev.off()

rm(p1,p2,pp1,pp2,ppp1);gc()

# ====Figure S14====

input_dir<-"12_单细胞/celltype_subsets/6_sig_genes/spatial_autocorrelation/sig_gene_time_plot"
p1<-qs_read(file.path(input_dir,"cds_res1_n_i_cds_res2_n_i-seu1_n_i-Endothelial_seu2_n_i-Endothelial-sig_gene_time_plot.qs2"))
p2<-qs_read(file.path(input_dir,"cds_res1_n_i_cds_res2_n_i-seu1_n_i-Endothelial_seu2_n_i-Endothelial-sig_gene_time_plot_umap.qs2"))
p3<-qs_read(file.path(input_dir,"cds_res1_n_i_cds_res2_n_i-seu1_n_i-Stellate_seu2_n_i-Stellate-sig_gene_time_plot.qs2"))
p4<-qs_read(file.path(input_dir,"cds_res1_n_i_cds_res2_n_i-seu1_n_i-Stellate_seu2_n_i-Stellate-sig_gene_time_plot_umap.qs2"))

plot_title<-function(p,group){
  
  pp<-plot_grid(p[[1]],p[[2]],ncol = 2)
  
  # now add the title
  title <- ggdraw() + 
    draw_label(
      paste0(group),
      fontface = 'bold',
      x = 0.05,
      hjust = 0
    ) +
    theme(
      # add margin on the left of the drawing canvas,
      # so title is aligned with left edge of first plot
      plot.margin = margin(0, 0, 0, 7)
    )
  
  pp<-plot_grid(
    title, pp,
    ncol = 1,
    # rel_heights values control vertical title margins
    rel_heights = c(0.1, 1)
  )
  
  return(pp)
}

pp1<-plot_title(p1,"left: tumor, right: normal, Endothelial")
pp2<-plot_title(p2,"left: tumor, right: normal, Endothelial")
pp3<-plot_title(p3,"left: tumor, right: normal, Stellate")
pp4<-plot_title(p4,"left: tumor, right: normal, Stellate")

ppp1<-plot_grid(pp1,pp2,pp3,pp4,ncol=1,labels = "AUTO",label_fontfamily = "serif",#label的设置
                label_size = 30,label_fontface = "bold",#label的设置
                label_x = -0.02,label_y = 1)

ppp1 <- ggdraw(ppp1) + 
  theme(plot.margin = margin(t = 35, r = 35, b = 35, l = 35, unit = "pt"),
        plot.background = element_rect(fill = "white", color = NA))

png(file.path(plot_dir,"Figure S14.png"),width = 4000,height = 4000*(2^0.5),res = 300)
print(ppp1)
dev.off()

rm(p1,p2,p3,p4,pp1,pp2,pp3,pp4,ppp1);gc()

# ====Figure S15====

input_dir<-"13_空转/cell2location/sc_ref"

read_png_as_plot <- function(path) {
  img <- readPNG(path)
  ggdraw() + draw_image(img)
}

p1<-read_png_as_plot(file.path(input_dir,"filtered_genes.png"))
p2<-read_png_as_plot(file.path(input_dir,"train_history.png"))
p3<-read_png_as_plot(file.path(input_dir,"QC_plot_Reconstruction_accuracy.png"))
p4<-read_png_as_plot(file.path(input_dir,"QC_plot_Reference_expression_signatures.png"))

pp1<-plot_grid(p1,p2,p3,p4,ncol = 2,labels = "AUTO",label_fontfamily = "serif",#label的设置
               label_size = 30,label_fontface = "bold",#label的设置
               label_x = -0.02,label_y = 1)

pp1 <- ggdraw(pp1) + 
  theme(plot.margin = margin(t = 35, r = 35, b = 35, l = 35, unit = "pt"),
        plot.background = element_rect(fill = "white", color = NA))

png(file.path(plot_dir,"Figure S15.png"),width = 4000,height = 4000,res = 300)
print(pp1)
dev.off()

rm(p1,p2,p3,p4,pp1);gc()

# ====Figure S16====

input_dir<-"13_空转/cell2location/spa_model_res"

png_name_list<-list.files(input_dir,pattern = ".*_his_QC.png$")

read_png_as_plot <- function(path) {
  img <- readPNG(path)
  ggdraw() + draw_image(img)
}
p_add_title <- function(p, title_text, title_size = 16) {  # ← 加默认值方便复用
  title <- ggdraw() + 
    draw_label(
      title_text,
      fontface = 'bold',
      x = 0.5,
      hjust = 0,
      size = title_size    # ← 字体大小，单位是pt
    ) +
    theme(
      plot.margin = margin(0, 0, 0, 7)
    )
  
  p <- plot_grid(
    title, p,
    ncol = 1,
    rel_heights = c(0.1, 1)
  )
  
  return(p)
}

p_list<-list()
for(i in seq_along(png_name_list)){
  message("====",png_name_list[[i]],"====")
  p<-read_png_as_plot(file.path(input_dir,png_name_list[[i]]))
  
  group<-sub("^([^_]+)_.*", "\\1", png_name_list[[i]])
  
  p<-p_add_title(p,group,10)
  
  p_list[[group]]<-p
}

rm(i,p,group);gc()

pp1<-plot_grid(plotlist = p_list,ncol = 3,labels = "AUTO",label_fontfamily = "serif",#label的设置
          label_size = 15,label_fontface = "bold",#label的设置
          label_x = 0,label_y = 1)

pp1 <- ggdraw(pp1) + 
  theme(plot.margin = margin(t = 35, r = 35, b = 35, l = 35, unit = "pt"),
        plot.background = element_rect(fill = "white", color = NA))

png(file.path(plot_dir,"Figure S16.png"),width = 4000,height = 4000*(2^0.5),res = 300)
print(pp1)
dev.off()

rm(p_list,pp1,png_name_list);gc()


# ====Figure S17====

input_dir<-"13_空转/cell2location/spa_model_res"

png_name_list<-list.files(input_dir,pattern = ".*_QC2.png$")

read_png_as_plot <- function(path) {
  img <- readPNG(path)
  ggdraw() + draw_image(img)
}
p_add_title <- function(p,title_text,title_text_x=0,title_size=16) {  # ← 加默认值方便复用
  title <- ggdraw() + 
    draw_label(
      title_text,
      fontface = 'bold',
      x = title_text_x,
      hjust = 0,
      size = title_size    # ← 字体大小，单位是pt
    ) +
    theme(
      plot.margin = margin(0, 0, 0, 7)
    )
  
  p <- plot_grid(
    title, p,
    ncol = 1,
    rel_heights = c(0.1, 1)
  )
  
  return(p)
}

p_list<-list()
for(i in seq_along(png_name_list)){
  message("====",png_name_list[[i]],"====")
  p<-read_png_as_plot(file.path(input_dir,png_name_list[[i]]))
  
  group<-sub("^([^_]+)_.*", "\\1", png_name_list[[i]])
  
  p<-p_add_title(p,group,0.4,10)
  
  p_list[[group]]<-p
}

rm(i,p,group);gc()

pp1<-plot_grid(plotlist = p_list,ncol = 7,labels = "AUTO",label_fontfamily = "serif",#label的设置
               label_size = 15,label_fontface = "bold",#label的设置
               label_x = 0,label_y = 1)

ppp1<-plot_grid(NULL,pp1,NULL,ncol = 1,rel_heights = c(0.05,1-0.1,0.05))

ppp1 <- ggdraw(ppp1) + 
  theme(plot.margin = margin(t = 25, r = 25, b = 25, l = 25, unit = "pt"),
        plot.background = element_rect(fill = "white", color = NA))

png(file.path(plot_dir,"Figure S17.png"),width = 4000,height = 4000*(2^0.5),res = 300)
print(ppp1)
dev.off()

rm(p_list,pp1,ppp1,png_name_list);gc()


# ====Figure S18====

input_dir<-"13_空转/cell2location/spa_model_res"

png_name_list<-list.files(input_dir,pattern = ".*_umap_cluster.png$")

read_png_as_plot <- function(path) {
  img <- readPNG(path)
  ggdraw() + draw_image(img)
}
p_add_title <- function(p,title_text,title_text_x=0,title_size=16) {  # ← 加默认值方便复用
  title <- ggdraw() + 
    draw_label(
      title_text,
      fontface = 'bold',
      x = title_text_x,
      hjust = 0,
      size = title_size    # ← 字体大小，单位是pt
    ) +
    theme(
      plot.margin = margin(0, 0, 0, 7)
    )
  
  p <- plot_grid(
    title, p,
    ncol = 1,
    rel_heights = c(0.1, 1)
  )
  
  return(p)
}

p_list<-list()
for(i in seq_along(png_name_list)){
  message("====",png_name_list[[i]],"====")
  p<-read_png_as_plot(file.path(input_dir,png_name_list[[i]]))
  
  group<-sub("^([^_]+)_.*", "\\1", png_name_list[[i]])
  
  p<-p_add_title(p,group,0.4,10)
  
  p_list[[group]]<-p
}

rm(i,p,group);gc()

pp1<-plot_grid(plotlist = p_list,ncol = 3,labels = "AUTO",label_fontfamily = "serif",#label的设置
               label_size = 15,label_fontface = "bold",#label的设置
               label_x = 0,label_y = 1)

ppp1<-plot_grid(NULL,pp1,NULL,ncol = 1,rel_heights = c(0.05,1-0.1,0.05))

ppp1 <- ggdraw(ppp1) + 
  theme(plot.margin = margin(t = 25, r = 25, b = 25, l = 25, unit = "pt"),
        plot.background = element_rect(fill = "white", color = NA))

png(file.path(plot_dir,"Figure S18.png"),width = 4000,height = 4000*(2^0.5),res = 300)
print(ppp1)
dev.off()

rm(p_list,pp1,ppp1,png_name_list);gc()


# ====Figure S

