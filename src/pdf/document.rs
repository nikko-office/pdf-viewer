//! PDF ドキュメント管理

use anyhow::{Context, Result};
use eframe::egui;
use pdfium_render::prelude::*;
use std::path::Path;

/// PDFiumライブラリを取得
fn get_pdfium() -> Result<Pdfium> {
    // 実行ファイルと同じディレクトリからPDFiumを読み込み
    let bindings = Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"))
        .or_else(|_| Pdfium::bind_to_system_library())
        .context("PDFiumライブラリを読み込めませんでした")?;
    Ok(Pdfium::new(bindings))
}

/// PDFドキュメントのラッパー
pub struct PdfDocument {
    path: std::path::PathBuf,
    page_count: usize,
    page_sizes: Vec<(f32, f32)>,
}

impl PdfDocument {
    /// PDFファイルを開く
    pub fn open(path: &Path) -> Result<Self> {
        let pdfium = get_pdfium()?;

        let document = pdfium
            .load_pdf_from_file(path, None)
            .context("PDFファイルを開けませんでした")?;

        let page_count = document.pages().len();

        // 各ページのサイズを取得
        let mut page_sizes = Vec::with_capacity(page_count as usize);
        for page in document.pages().iter() {
            let width = page.width().value;
            let height = page.height().value;
            page_sizes.push((width, height));
        }

        Ok(Self {
            path: path.to_path_buf(),
            page_count: page_count as usize,
            page_sizes,
        })
    }

    /// ページ数を取得
    pub fn page_count(&self) -> usize {
        self.page_count
    }

    /// ページサイズを取得 (ポイント単位)
    pub fn page_size(&self, page_index: usize) -> (f32, f32) {
        self.page_sizes
            .get(page_index)
            .copied()
            .unwrap_or((612.0, 792.0))
    }

    /// ページをレンダリング
    pub fn render_page(
        &self,
        page_index: usize,
        width: u32,
        height: u32,
    ) -> Option<egui::ColorImage> {
        let pdfium = get_pdfium().ok()?;

        let document = pdfium.load_pdf_from_file(&self.path, None).ok()?;

        let page = document.pages().get(page_index as u16).ok()?;

        // ページサイズ
        let page_width = page.width().value;
        let page_height = page.height().value;

        // スケール計算
        let scale_x = width as f32 / page_width;
        let scale_y = height as f32 / page_height;
        let scale = scale_x.min(scale_y);

        let render_width = (page_width * scale) as i32;
        let render_height = (page_height * scale) as i32;

        // ページをレンダリング
        let render_config = PdfRenderConfig::new()
            .set_target_width(render_width)
            .set_target_height(render_height)
            .render_form_data(true)
            .render_annotations(true);

        let bitmap = page.render_with_config(&render_config).ok()?;

        // egui::ColorImage に変換
        let img = bitmap.as_image();
        let rgba = img.to_rgba8();
        let (img_width, img_height) = rgba.dimensions();

        let pixels: Vec<egui::Color32> = rgba
            .pixels()
            .map(|p| egui::Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
            .collect();

        Some(egui::ColorImage {
            size: [img_width as usize, img_height as usize],
            pixels,
        })
    }

    /// サムネイル用の小さいサイズでレンダリング
    pub fn render_page_thumbnail(
        &self,
        page_index: usize,
        max_width: u32,
        max_height: u32,
    ) -> Option<egui::ColorImage> {
        let (page_w, page_h) = self.page_size(page_index);
        let scale = (max_width as f32 / page_w).min(max_height as f32 / page_h);
        let w = (page_w * scale) as u32;
        let h = (page_h * scale) as u32;
        self.render_page(page_index, w, h)
    }

    /// ファイルパスを取得
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// ページ数を更新
    pub fn refresh_page_count(&mut self) {
        if let Ok(pdfium) = get_pdfium() {
            if let Ok(document) = pdfium.load_pdf_from_file(&self.path, None) {
                self.page_count = document.pages().len() as usize;

                // ページサイズも更新
                self.page_sizes.clear();
                for page in document.pages().iter() {
                    let width = page.width().value;
                    let height = page.height().value;
                    self.page_sizes.push((width, height));
                }
            }
        }
    }

    /// PDFを保存
    pub fn save(&self, path: &Path) -> Result<()> {
        // PDFiumでの保存はより複雑なため、
        // 単純なファイルコピーを使用します
        if self.path != path {
            std::fs::copy(&self.path, path).context("PDFを保存できませんでした")?;
        }
        Ok(())
    }
}

impl Clone for PdfDocument {
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            page_count: self.page_count,
            page_sizes: self.page_sizes.clone(),
        }
    }
}
