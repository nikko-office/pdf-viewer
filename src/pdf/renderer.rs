//! スタンプとテキスト注釈の定義

use serde::{Deserialize, Serialize};

/// スタンプの種類
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StampType {
    Approved,
    Rejected,
    Draft,
    Confidential,
}

impl StampType {
    /// スタンプ名を取得
    pub fn name(&self) -> &'static str {
        match self {
            StampType::Approved => "approved",
            StampType::Rejected => "rejected",
            StampType::Draft => "draft",
            StampType::Confidential => "confidential",
        }
    }

    /// 日本語ラベル
    pub fn label(&self) -> &'static str {
        match self {
            StampType::Approved => "承認",
            StampType::Rejected => "却下",
            StampType::Draft => "下書き",
            StampType::Confidential => "機密",
        }
    }

    /// 全スタンプタイプ
    pub fn all() -> &'static [StampType] {
        &[
            StampType::Approved,
            StampType::Rejected,
            StampType::Draft,
            StampType::Confidential,
        ]
    }
}

/// PDFに配置するスタンプ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stamp {
    /// ページ番号 (0-indexed)
    pub page: usize,
    /// X座標 (PDFポイント)
    pub x: f32,
    /// Y座標 (PDFポイント)
    pub y: f32,
    /// 幅 (PDFポイント)
    pub width: f32,
    /// 高さ (PDFポイント)
    pub height: f32,
    /// スタンプタイプ
    pub stamp_type: StampType,
}

impl Stamp {
    /// 新しいスタンプを作成
    pub fn new(page: usize, x: f32, y: f32, stamp_type: StampType) -> Self {
        Self {
            page,
            x,
            y,
            width: 100.0,
            height: 50.0,
            stamp_type,
        }
    }

    /// スタンプの矩形を取得
    pub fn rect(&self) -> (f32, f32, f32, f32) {
        (self.x, self.y, self.x + self.width, self.y + self.height)
    }
}

/// PDFに追加するテキスト注釈
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextAnnotation {
    /// ページ番号 (0-indexed)
    pub page: usize,
    /// X座標 (PDFポイント)
    pub x: f32,
    /// Y座標 (PDFポイント)
    pub y: f32,
    /// テキスト内容
    pub text: String,
    /// フォントサイズ (ポイント)
    pub font_size: f32,
}

impl TextAnnotation {
    /// 新しいテキスト注釈を作成
    pub fn new(page: usize, x: f32, y: f32, text: String, font_size: f32) -> Self {
        Self {
            page,
            x,
            y,
            text,
            font_size,
        }
    }
}
