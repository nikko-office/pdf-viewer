//! PDF ドキュメント管理

use anyhow::{Context, Result};
use eframe::egui;
use pdfium_render::prelude::*;
use std::path::Path;

/// PDFiumライブラリを取得
fn get_pdfium() -> Result<Pdfium> {
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
    page_rotations: Vec<i32>, // 各ページの回転角度（0, 90, 180, 270）
}

impl PdfDocument {
    /// PDFファイルを開く
    pub fn open(path: &Path) -> Result<Self> {
        let pdfium = get_pdfium()?;

        let document = pdfium
            .load_pdf_from_file(path, None)
            .context("PDFファイルを開けませんでした")?;

        let page_count = document.pages().len();

        let mut page_sizes = Vec::with_capacity(page_count as usize);
        let mut page_rotations = Vec::with_capacity(page_count as usize);
        
        for page in document.pages().iter() {
            let width = page.width().value;
            let height = page.height().value;
            page_sizes.push((width, height));
            page_rotations.push(0); // 初期回転は0度
        }

        Ok(Self {
            path: path.to_path_buf(),
            page_count: page_count as usize,
            page_sizes,
            page_rotations,
        })
    }

    /// ページ数を取得
    pub fn page_count(&self) -> usize {
        self.page_count
    }

    /// ページサイズを取得 (ポイント単位、回転考慮)
    pub fn page_size(&self, page_index: usize) -> (f32, f32) {
        let base_size = self.page_sizes
            .get(page_index)
            .copied()
            .unwrap_or((612.0, 792.0));
        
        let rotation = self.page_rotations.get(page_index).copied().unwrap_or(0);
        
        // 90度または270度の場合は幅と高さを入れ替え
        if rotation == 90 || rotation == 270 {
            (base_size.1, base_size.0)
        } else {
            base_size
        }
    }

    /// ページを回転
    pub fn rotate_page(&mut self, page_index: usize, degrees: i32) -> Result<()> {
        if page_index >= self.page_count {
            return Err(anyhow::anyhow!("無効なページ番号"));
        }
        
        // 現在の回転に追加
        let current = self.page_rotations.get(page_index).copied().unwrap_or(0);
        let new_rotation = (current + degrees) % 360;
        let new_rotation = if new_rotation < 0 { new_rotation + 360 } else { new_rotation };
        
        if page_index < self.page_rotations.len() {
            self.page_rotations[page_index] = new_rotation;
        }
        
        Ok(())
    }

    /// ページの回転角度を取得
    pub fn get_page_rotation(&self, page_index: usize) -> i32 {
        self.page_rotations.get(page_index).copied().unwrap_or(0)
    }

    /// ページをレンダリング（回転対応）
    pub fn render_page(
        &self,
        page_index: usize,
        width: u32,
        height: u32,
    ) -> Option<egui::ColorImage> {
        let pdfium = get_pdfium().ok()?;
        let document = pdfium.load_pdf_from_file(&self.path, None).ok()?;
        let page = document.pages().get(page_index as u16).ok()?;

        let rotation = self.page_rotations.get(page_index).copied().unwrap_or(0);

        // ページサイズ
        let page_width = page.width().value;
        let page_height = page.height().value;

        // 回転によるサイズ調整
        let (effective_width, effective_height) = if rotation == 90 || rotation == 270 {
            (page_height, page_width)
        } else {
            (page_width, page_height)
        };

        // スケール計算
        let scale_x = width as f32 / effective_width;
        let scale_y = height as f32 / effective_height;
        let scale = scale_x.min(scale_y);

        let render_width = (effective_width * scale) as i32;
        let render_height = (effective_height * scale) as i32;

        // 回転設定
        let rotation_setting = match rotation {
            90 => PdfPageRenderRotation::Degrees90,
            180 => PdfPageRenderRotation::Degrees180,
            270 => PdfPageRenderRotation::Degrees270,
            _ => PdfPageRenderRotation::None,
        };

        // ページをレンダリング
        let render_config = PdfRenderConfig::new()
            .set_target_width(render_width)
            .set_target_height(render_height)
            .rotate(rotation_setting, true)
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

                self.page_sizes.clear();
                self.page_rotations.resize(self.page_count, 0);
                
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
        if self.path != path {
            std::fs::copy(&self.path, path).context("PDFを保存できませんでした")?;
        }
        // 注: 回転情報は現在ファイルには保存されません
        // 実際の回転保存にはPDFiumの編集機能が必要です
        Ok(())
    }
}

impl Clone for PdfDocument {
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            page_count: self.page_count,
            page_sizes: self.page_sizes.clone(),
            page_rotations: self.page_rotations.clone(),
        }
    }
}
