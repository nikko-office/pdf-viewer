//! アプリケーションの状態管理

use crate::pdf::{PdfDocument, PdfOperations, Stamp, TextAnnotation};
use crate::ui::{EditorPanel, FileExplorer, ThumbnailPanel};
use eframe::egui::{self, Color32, TextureHandle, Vec2};
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

    // フォルダ内PDFサムネイル
    folder_pdfs: Vec<FolderPdfEntry>,
    selected_pdf_index: Option<usize>,
    pdf_thumbnails: Vec<Option<TextureHandle>>,

    // ステータスメッセージ
    status_message: String,
}

/// フォルダ内のPDFエントリ
struct FolderPdfEntry {
    path: PathBuf,
    name: String,
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
            folder_pdfs: Vec::new(),
            selected_pdf_index: None,
            pdf_thumbnails: Vec::new(),
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

    /// フォルダ内のPDFを更新
    pub fn update_folder_pdfs(&mut self, folder_path: &PathBuf) {
        self.folder_pdfs.clear();
        self.pdf_thumbnails.clear();
        self.selected_pdf_index = None;

        if let Ok(entries) = std::fs::read_dir(folder_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext.eq_ignore_ascii_case("pdf")) {
                    let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                    self.folder_pdfs.push(FolderPdfEntry { path, name });
                }
            }
        }

        // 名前でソート
        self.folder_pdfs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        self.pdf_thumbnails.resize(self.folder_pdfs.len(), None);
    }

    /// PDFを保存
    pub fn save_pdf(&mut self, path: &PathBuf) {
        if let Some(ref mut doc) = self.current_document {
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
            .default_width(220.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui.heading("ファイル");
                ui.separator();
                if let Some((path, is_folder)) = self.file_explorer.show(ui) {
                    if is_folder {
                        // フォルダが選択された場合、PDFサムネイル一覧を更新
                        self.update_folder_pdfs(&path);
                    } else if path.extension().map_or(false, |ext| ext == "pdf") {
                        self.open_pdf(path);
                    }
                }
            });

        // 右パネル: プレビュー (大きく表示)
        egui::SidePanel::right("preview_panel")
            .default_width(450.0)
            .min_width(300.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui.heading("プレビュー");
                ui.separator();

                if let Some(ref doc) = self.current_document {
                    // ツールバー
                    ui.horizontal(|ui| {
                        if ui.button("◀").clicked() && self.selected_page > 0 {
                            self.selected_page -= 1;
                            self.editor_panel.invalidate_cache();
                        }
                        ui.label(format!("{} / {}", self.selected_page + 1, doc.page_count()));
                        if ui.button("▶").clicked() && self.selected_page < doc.page_count() - 1 {
                            self.selected_page += 1;
                            self.editor_panel.invalidate_cache();
                        }

                        ui.separator();

                        // スタンプボタン
                        if ui.selectable_label(self.show_stamp_panel, "✅ 承認").clicked() {
                            self.show_stamp_panel = !self.show_stamp_panel;
                            self.show_text_panel = false;
                        }
                        if ui.selectable_label(self.show_text_panel, "📝 テキスト").clicked() {
                            self.show_text_panel = !self.show_text_panel;
                            self.show_stamp_panel = false;
                        }
                    });

                    ui.separator();

                    // プレビュー表示
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

                    // ページサムネイル (下部)
                    ui.separator();
                    ui.label("ページ一覧");
                    egui::ScrollArea::horizontal()
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let result = self.thumbnail_panel.show_horizontal(ui, doc, self.selected_page);
                                if let Some(page) = result.selected_page {
                                    self.selected_page = page;
                                    self.editor_panel.invalidate_cache();
                                }
                            });
                        });
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.label("PDFファイルを選択してください");
                    });
                }
            });

        // 中央パネル: フォルダ内PDFサムネイル一覧
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.folder_pdfs.is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.label("左側のフォルダを選択すると、PDFファイルが表示されます");
                });
            } else {
                ui.heading(format!("PDFファイル ({} 件)", self.folder_pdfs.len()));
                ui.separator();

                // サムネイルデータを事前にコピー
                let folder_pdfs: Vec<(usize, PathBuf, String, bool, Option<egui::TextureId>)> = self
                    .folder_pdfs
                    .iter()
                    .enumerate()
                    .map(|(idx, entry)| {
                        let tex_id = self.pdf_thumbnails.get(idx).and_then(|t| t.as_ref().map(|t| t.id()));
                        (idx, entry.path.clone(), entry.name.clone(), self.selected_pdf_index == Some(idx), tex_id)
                    })
                    .collect();

                let mut clicked_pdf: Option<(usize, PathBuf)> = None;
                let mut thumbnails_to_load: Vec<(usize, PathBuf)> = Vec::new();

                egui::ScrollArea::both()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        let available_width = ui.available_width();
                        let thumb_width = 180.0;
                        let thumb_height = 240.0;
                        let spacing = 10.0;
                        let columns = ((available_width - spacing) / (thumb_width + spacing)).floor() as usize;
                        let columns = columns.max(1);

                        egui::Grid::new("pdf_grid")
                            .num_columns(columns)
                            .spacing([spacing, spacing])
                            .show(ui, |ui| {
                                for (idx, path, name, is_selected, tex_id) in &folder_pdfs {
                                    egui::Frame::none()
                                        .fill(if *is_selected {
                                            Color32::from_rgb(70, 130, 180)
                                        } else {
                                            Color32::from_gray(45)
                                        })
                                        .stroke(egui::Stroke::new(
                                            if *is_selected { 3.0 } else { 1.0 },
                                            if *is_selected {
                                                Color32::from_rgb(100, 149, 237)
                                            } else {
                                                Color32::from_gray(60)
                                            },
                                        ))
                                        .rounding(4.0)
                                        .inner_margin(8.0)
                                        .show(ui, |ui: &mut egui::Ui| {
                                            ui.set_width(thumb_width);
                                            ui.set_height(thumb_height);

                                            ui.vertical_centered(|ui| {
                                                // サムネイル表示エリア
                                                let (rect, response) = ui.allocate_exact_size(
                                                    Vec2::new(thumb_width - 16.0, thumb_height - 50.0),
                                                    egui::Sense::click(),
                                                );

                                                // サムネイルを描画
                                                if let Some(texture_id) = tex_id {
                                                    ui.painter().image(
                                                        *texture_id,
                                                        rect,
                                                        egui::Rect::from_min_max(
                                                            egui::pos2(0.0, 0.0),
                                                            egui::pos2(1.0, 1.0),
                                                        ),
                                                        Color32::WHITE,
                                                    );
                                                } else {
                                                    // サムネイル生成予約
                                                    ui.painter().rect_filled(rect, 2.0, Color32::from_gray(60));
                                                    ui.painter().text(
                                                        rect.center(),
                                                        egui::Align2::CENTER_CENTER,
                                                        "PDF",
                                                        egui::FontId::proportional(24.0),
                                                        Color32::from_gray(120),
                                                    );
                                                    thumbnails_to_load.push((*idx, path.clone()));
                                                }

                                                // クリックでPDFを開く
                                                if response.clicked() {
                                                    clicked_pdf = Some((*idx, path.clone()));
                                                }

                                                // ファイル名
                                                ui.add_space(4.0);
                                                ui.label(
                                                    egui::RichText::new(name)
                                                        .size(11.0)
                                                        .color(Color32::WHITE),
                                                );
                                            });
                                        });

                                    if (idx + 1) % columns == 0 {
                                        ui.end_row();
                                    }
                                }
                            });
                    });

                // サムネイル生成（最初の数個のみ）
                for (idx, path) in thumbnails_to_load.into_iter().take(3) {
                    if let Ok(doc) = PdfDocument::open(&path) {
                        if let Some(image) = doc.render_page_thumbnail(0, 160, 200) {
                            let texture = ctx.load_texture(
                                format!("folder_pdf_{}", idx),
                                image,
                                egui::TextureOptions::LINEAR,
                            );
                            if idx < self.pdf_thumbnails.len() {
                                self.pdf_thumbnails[idx] = Some(texture);
                            }
                        }
                    }
                }

                // クリック処理
                if let Some((idx, path)) = clicked_pdf {
                    self.selected_pdf_index = Some(idx);
                    self.open_pdf(path);
                }
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
