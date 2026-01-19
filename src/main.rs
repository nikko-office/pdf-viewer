//! PDF Viewer - 高機能PDF編集アプリケーション
//!
//! 機能:
//! - 3カラムレイアウト (ファイルエクスプローラー、サムネイル、メイン編集)
//! - ページ操作 (入れ替え、削除、回転)
//! - スタンプ配置、テキスト入力
//! - PDF結合・分割

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod pdf;
mod resources;
mod ui;

use anyhow::Result;
use eframe::egui;

// 日本語フォントをバイナリに埋め込む
pub const JAPANESE_FONT: &[u8] = include_bytes!("../assets/fonts/NotoSansJP-Regular.ttf");

// スタンプ画像を埋め込む
pub const STAMP_APPROVED: &[u8] = include_bytes!("../assets/stamps/approved.png");
pub const STAMP_REJECTED: &[u8] = include_bytes!("../assets/stamps/rejected.png");
pub const STAMP_DRAFT: &[u8] = include_bytes!("../assets/stamps/draft.png");
pub const STAMP_CONFIDENTIAL: &[u8] = include_bytes!("../assets/stamps/confidential.png");

fn main() -> Result<()> {
    // ロギング初期化
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    log::info!("PDF Viewer を起動中...");

    // eframe オプション設定
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 900.0])
            .with_min_inner_size([1000.0, 600.0])
            .with_title("PDF Viewer - 高機能PDF編集"),
        ..Default::default()
    };

    // アプリケーション起動
    eframe::run_native(
        "PDF Viewer",
        options,
        Box::new(|cc| {
            // 日本語フォントを登録
            setup_fonts(&cc.egui_ctx);
            // ダークモードを初期設定
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            Ok(Box::new(app::PdfViewerApp::new(cc)))
        }),
    )
    .map_err(|e| anyhow::anyhow!("アプリケーションエラー: {}", e))
}

/// 日本語フォントを設定
fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // 日本語フォントを追加
    fonts.font_data.insert(
        "NotoSansJP".to_owned(),
        egui::FontData::from_static(JAPANESE_FONT),
    );

    // フォント優先順位を設定
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "NotoSansJP".to_owned());

    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .insert(0, "NotoSansJP".to_owned());

    ctx.set_fonts(fonts);
}
