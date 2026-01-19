//! リソース管理モジュール

use crate::{STAMP_APPROVED, STAMP_CONFIDENTIAL, STAMP_DRAFT, STAMP_REJECTED};
use image::{DynamicImage, ImageFormat};
use once_cell::sync::Lazy;
use std::io::Cursor;

/// 埋め込みスタンプ画像
pub static STAMPS: Lazy<StampResources> = Lazy::new(|| StampResources::load());

/// スタンプリソース
pub struct StampResources {
    pub approved: DynamicImage,
    pub rejected: DynamicImage,
    pub draft: DynamicImage,
    pub confidential: DynamicImage,
}

impl StampResources {
    fn load() -> Self {
        Self {
            approved: load_png(STAMP_APPROVED),
            rejected: load_png(STAMP_REJECTED),
            draft: load_png(STAMP_DRAFT),
            confidential: load_png(STAMP_CONFIDENTIAL),
        }
    }

    /// スタンプ名から画像を取得
    pub fn get(&self, name: &str) -> Option<&DynamicImage> {
        match name {
            "approved" => Some(&self.approved),
            "rejected" => Some(&self.rejected),
            "draft" => Some(&self.draft),
            "confidential" => Some(&self.confidential),
            _ => None,
        }
    }

    /// 利用可能なスタンプ名一覧
    pub fn names() -> &'static [&'static str] {
        &["approved", "rejected", "draft", "confidential"]
    }
}

/// PNG画像をロード
fn load_png(data: &[u8]) -> DynamicImage {
    image::load(Cursor::new(data), ImageFormat::Png).expect("Failed to load embedded PNG")
}
