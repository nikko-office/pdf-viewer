//! アプリケーションの状態管理

use crate::pdf::{PdfDocument, PdfOperations, Stamp, TextAnnotation};
use crate::ui::{EditorPanel, FileExplorer, ThumbnailPanel};
use eframe::egui;
use std::path::PathBuf;

/// アプリケーション全体の状態
pub struct PdfViewerApp {
    // UI パネル
    file_explorer: FileExplorer,
    thumbnail_panel: ThumbnailPanel,
    editor_panel: EditorPanel,

    // PDF ドキュメント
    current_document: Option<PdfDocument>,
    documents: Vec<PdfDocument>,

    // 編集状態
    selected_page: usize,
    stamps: Vec<Stamp>,
    text_annotations: Vec<TextAnnotation>,

    // UI 状態
    show_merge_dialog: bool,
    show_split_dialog: bool,
    show_stamp_panel: bool,
    show_text_panel: bool,
    split_start_page: String,
    split_end_page: String,
    
    // ステータスメッセージ
    status_message: String,
}

impl PdfViewerApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            file_explorer: FileExplorer::new(),
            thumbnail_panel: ThumbnailPanel::new(),
            editor_panel: EditorPanel::new(),
            current_document: None,
            documents: Vec::new(),
            selected_page: 0,
            stamps: Vec::new(),
            text_annotations: Vec::new(),
            show_merge_dialog: false,
            show_split_dialog: false,
            show_stamp_panel: false,
            show_text_panel: false,
            split_start_page: String::new(),
            split_end_page: String::new(),
            status_message: "準備完了".to_string(),
        }
    }

    /// PDFファイルを開く
    pub fn open_pdf(&mut self, path: PathBuf) {
        match PdfDocument::open(&path) {
            Ok(doc) => {
                self.status_message = format!("開きました: {}", path.display());
                self.thumbnail_panel.load_thumbnails(&doc);
                self.current_document = Some(doc);
                self.selected_page = 0;
            }
            Err(e) => {
                self.status_message = format!("エラー: {}", e);
                log::error!("PDFを開けません: {}", e);
            }
        }
    }

    /// PDFを保存
    pub fn save_pdf(&mut self, path: &PathBuf) {
        if let Some(ref mut doc) = self.current_document {
            // スタンプとテキスト注釈を適用
            for stamp in &self.stamps {
                if let Err(e) = PdfOperations::add_stamp(doc, stamp) {
                    log::error!("スタンプ追加エラー: {}", e);
                }
            }
            for annotation in &self.text_annotations {
                if let Err(e) = PdfOperations::add_text(doc, annotation) {
                    log::error!("テキスト追加エラー: {}", e);
                }
            }

            match doc.save(path) {
                Ok(_) => {
                    self.status_message = format!("保存しました: {}", path.display());
                    self.stamps.clear();
                    self.text_annotations.clear();
                }
                Err(e) => {
                    self.status_message = format!("保存エラー: {}", e);
                }
            }
        }
    }

    /// 複数PDFを結合
    fn merge_pdfs(&mut self) {
        if self.documents.len() < 2 {
            self.status_message = "結合するには2つ以上のPDFが必要です".to_string();
            return;
        }

        match PdfOperations::merge(&self.documents) {
            Ok(merged) => {
                self.current_document = Some(merged);
                self.thumbnail_panel
                    .load_thumbnails(self.current_document.as_ref().unwrap());
                self.status_message = "PDFを結合しました".to_string();
                self.documents.clear();
            }
            Err(e) => {
                self.status_message = format!("結合エラー: {}", e);
            }
        }
    }

    /// PDFを分割
    fn split_pdf(&mut self) {
        if let Some(ref doc) = self.current_document {
            let start: usize = self.split_start_page.parse().unwrap_or(1);
            let end: usize = self.split_end_page.parse().unwrap_or(doc.page_count());

            if start > 0 && end <= doc.page_count() && start <= end {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("PDF", &["pdf"])
                    .set_file_name("split.pdf")
                    .save_file()
                {
                    match PdfOperations::split(doc, start - 1, end, &path) {
                        Ok(_) => {
                            self.status_message =
                                format!("分割しました (ページ {} - {})", start, end);
                        }
                        Err(e) => {
                            self.status_message = format!("分割エラー: {}", e);
                        }
                    }
                }
            } else {
                self.status_message = "無効なページ範囲です".to_string();
            }
        }
    }
}

impl eframe::App for PdfViewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // メニューバー
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("ファイル", |ui| {
                    if ui.button("開く...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("PDF", &["pdf"])
                            .pick_file()
                        {
                            self.open_pdf(path);
                        }
                        ui.close_menu();
                    }
                    if ui.button("保存...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("PDF", &["pdf"])
                            .set_file_name("output.pdf")
                            .save_file()
                        {
                            self.save_pdf(&path);
                        }
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("結合用PDFを追加...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("PDF", &["pdf"])
                            .pick_file()
                        {
                            if let Ok(doc) = PdfDocument::open(&path) {
                                self.documents.push(doc);
                                self.status_message =
                                    format!("結合リストに追加: {} 件", self.documents.len());
                            }
                        }
                        ui.close_menu();
                    }
                    if ui.button("PDFを結合").clicked() {
                        self.merge_pdfs();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("分割...").clicked() {
                        self.show_split_dialog = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("終了").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                ui.menu_button("編集", |ui| {
                    if ui.button("スタンプを追加").clicked() {
                        self.show_stamp_panel = !self.show_stamp_panel;
                        ui.close_menu();
                    }
                    if ui.button("テキストを追加").clicked() {
                        self.show_text_panel = !self.show_text_panel;
                        ui.close_menu();
                    }
                });

                ui.menu_button("表示", |ui| {
                    if ui.button("ダークモード").clicked() {
                        ctx.set_visuals(egui::Visuals::dark());
                        ui.close_menu();
                    }
                    if ui.button("ライトモード").clicked() {
                        ctx.set_visuals(egui::Visuals::light());
                        ui.close_menu();
                    }
                });
            });
        });

        // ステータスバー
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(&self.status_message);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(ref doc) = self.current_document {
                        ui.label(format!(
                            "ページ: {} / {}",
                            self.selected_page + 1,
                            doc.page_count()
                        ));
                    }
                });
            });
        });

        // 左パネル: ファイルエクスプローラー
        egui::SidePanel::left("file_explorer")
            .default_width(250.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui.heading("ファイル");
                ui.separator();
                if let Some(path) = self.file_explorer.show(ui) {
                    if path.extension().map_or(false, |ext| ext == "pdf") {
                        self.open_pdf(path);
                    }
                }
            });

        // 右パネル: サムネイル
        egui::SidePanel::right("thumbnail_panel")
            .default_width(200.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui.heading("ページ一覧");
                ui.separator();

                if let Some(ref mut doc) = self.current_document {
                    let result = self.thumbnail_panel.show(ui, doc, self.selected_page);

                    if let Some(new_selection) = result.selected_page {
                        self.selected_page = new_selection;
                    }

                    if let Some((from, to)) = result.page_reorder {
                        if let Err(e) = PdfOperations::reorder_page(doc, from, to) {
                            self.status_message = format!("ページ移動エラー: {}", e);
                        } else {
                            self.thumbnail_panel.load_thumbnails(doc);
                        }
                    }

                    if let Some(page_to_delete) = result.page_deleted {
                        if let Err(e) = PdfOperations::delete_page(doc, page_to_delete) {
                            self.status_message = format!("ページ削除エラー: {}", e);
                        } else {
                            self.thumbnail_panel.load_thumbnails(doc);
                            if self.selected_page >= doc.page_count() {
                                self.selected_page = doc.page_count().saturating_sub(1);
                            }
                        }
                    }

                    if let Some((page, rotation)) = result.page_rotated {
                        if let Err(e) = PdfOperations::rotate_page(doc, page, rotation) {
                            self.status_message = format!("ページ回転エラー: {}", e);
                        } else {
                            self.thumbnail_panel.load_thumbnails(doc);
                        }
                    }
                }
            });

        // 中央パネル: メイン編集エリア
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(ref doc) = self.current_document {
                let editor_result = self.editor_panel.show(
                    ui,
                    doc,
                    self.selected_page,
                    &self.stamps,
                    &self.text_annotations,
                    self.show_stamp_panel,
                    self.show_text_panel,
                );

                if let Some(stamp) = editor_result.new_stamp {
                    self.stamps.push(stamp);
                }
                if let Some(annotation) = editor_result.new_text {
                    self.text_annotations.push(annotation);
                }
            } else {
                ui.centered_and_justified(|ui| {
                    ui.heading("PDFファイルを開いてください");
                });
            }
        });

        // 分割ダイアログ
        if self.show_split_dialog {
            egui::Window::new("PDF分割")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("開始ページ:");
                        ui.text_edit_singleline(&mut self.split_start_page);
                    });
                    ui.horizontal(|ui| {
                        ui.label("終了ページ:");
                        ui.text_edit_singleline(&mut self.split_end_page);
                    });
                    ui.horizontal(|ui| {
                        if ui.button("分割").clicked() {
                            self.split_pdf();
                            self.show_split_dialog = false;
                        }
                        if ui.button("キャンセル").clicked() {
                            self.show_split_dialog = false;
                        }
                    });
                });
        }
    }
}
