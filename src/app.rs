//! アプリケーションの状態管理

use crate::pdf::{PdfDocument, PdfOperations, Stamp, TextAnnotation};
use crate::ui::{EditorPanel, FileExplorer};
use eframe::egui::{self, Color32, TextureHandle, Vec2};
use std::path::PathBuf;
use std::process::Command;

/// アプリケーション全体の状態
pub struct PdfViewerApp {
    // UI パネル
    file_explorer: FileExplorer,
    editor_panel: EditorPanel,

    // PDF ドキュメント
    current_document: Option<PdfDocument>,
    documents: Vec<PdfDocument>,

    // 編集状態
    selected_page: usize,
    stamps: Vec<Stamp>,
    text_annotations: Vec<TextAnnotation>,

    // UI 状態
    show_split_dialog: bool,
    show_stamp_panel: bool,
    show_text_panel: bool,
    show_stamp_register_dialog: bool,
    split_start_page: String,
    split_end_page: String,

    // フォルダ内PDFサムネイル
    folder_pdfs: Vec<FolderPdfEntry>,
    selected_pdf_index: Option<usize>,
    pdf_thumbnails: Vec<Option<TextureHandle>>,
    current_folder: Option<PathBuf>,

    // カスタムスタンプ
    custom_stamps: Vec<CustomStamp>,

    // コンテキストメニュー
    context_menu_pdf: Option<(usize, egui::Pos2)>,

    // ステータスメッセージ
    status_message: String,
}

/// フォルダ内のPDFエントリ
struct FolderPdfEntry {
    path: PathBuf,
    name: String,
}

/// カスタムスタンプ
#[derive(Clone)]
pub struct CustomStamp {
    pub name: String,
    pub path: PathBuf,
}

impl PdfViewerApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            file_explorer: FileExplorer::new(),
            editor_panel: EditorPanel::new(),
            current_document: None,
            documents: Vec::new(),
            selected_page: 0,
            stamps: Vec::new(),
            text_annotations: Vec::new(),
            show_split_dialog: false,
            show_stamp_panel: false,
            show_text_panel: false,
            show_stamp_register_dialog: false,
            split_start_page: String::new(),
            split_end_page: String::new(),
            folder_pdfs: Vec::new(),
            selected_pdf_index: None,
            pdf_thumbnails: Vec::new(),
            current_folder: None,
            custom_stamps: Vec::new(),
            context_menu_pdf: None,
            status_message: "準備完了".to_string(),
        }
    }

    /// PDFファイルを開く
    pub fn open_pdf(&mut self, path: PathBuf) {
        match PdfDocument::open(&path) {
            Ok(doc) => {
                self.status_message = format!("開きました: {}", path.display());
                self.current_document = Some(doc);
                self.selected_page = 0;
                self.editor_panel.invalidate_cache();
            }
            Err(e) => {
                self.status_message = format!("エラー: {}", e);
                log::error!("PDFを開けません: {}", e);
            }
        }
    }

    /// 外部アプリでPDFを開く
    fn open_with_external(&self, path: &PathBuf) {
        #[cfg(windows)]
        {
            let _ = Command::new("cmd")
                .args(["/C", "start", "", &path.to_string_lossy()])
                .spawn();
        }
        #[cfg(target_os = "macos")]
        {
            let _ = Command::new("open").arg(path).spawn();
        }
        #[cfg(target_os = "linux")]
        {
            let _ = Command::new("xdg-open").arg(path).spawn();
        }
    }

    /// 「プログラムから開く」ダイアログを表示
    fn open_with_dialog(&self, path: &PathBuf) {
        #[cfg(windows)]
        {
            let _ = Command::new("rundll32")
                .args(["shell32.dll,OpenAs_RunDLL", &path.to_string_lossy()])
                .spawn();
        }
        #[cfg(not(windows))]
        {
            self.open_with_external(path);
        }
    }

    /// フォルダ内のPDFを更新
    pub fn update_folder_pdfs(&mut self, folder_path: &PathBuf) {
        self.folder_pdfs.clear();
        self.pdf_thumbnails.clear();
        self.selected_pdf_index = None;
        self.current_folder = Some(folder_path.clone());

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
                self.status_message = "PDFを結合しました".to_string();
                self.documents.clear();
                self.editor_panel.invalidate_cache();
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

    /// ページを回転
    fn rotate_page(&mut self, page: usize, angle: i32) {
        if let Some(ref mut doc) = self.current_document {
            if let Err(e) = PdfOperations::rotate_page(doc, page, angle) {
                self.status_message = format!("回転エラー: {}", e);
            } else {
                self.status_message = format!("ページ {} を {}° 回転しました", page + 1, angle);
                self.editor_panel.invalidate_cache();
            }
        }
    }

    /// ファイル操作を実行
    fn handle_file_operations(&mut self, 
        file_moved: Option<(PathBuf, PathBuf)>,
        file_copied: Option<(PathBuf, PathBuf)>,
        file_deleted: Option<PathBuf>
    ) {
        // ファイル移動
        if let Some((src, dest)) = file_moved {
            match std::fs::rename(&src, &dest) {
                Ok(_) => {
                    self.status_message = format!("移動しました: {}", src.display());
                    if let Some(ref folder) = self.current_folder.clone() {
                        self.update_folder_pdfs(folder);
                    }
                }
                Err(e) => {
                    self.status_message = format!("移動エラー: {}", e);
                }
            }
        }

        // ファイルコピー
        if let Some((src, dest)) = file_copied {
            if src.is_dir() {
                match copy_dir_all(&src, &dest) {
                    Ok(_) => {
                        self.status_message = format!("コピーしました: {}", src.display());
                        if let Some(ref folder) = self.current_folder.clone() {
                            self.update_folder_pdfs(folder);
                        }
                    }
                    Err(e) => {
                        self.status_message = format!("コピーエラー: {}", e);
                    }
                }
            } else {
                match std::fs::copy(&src, &dest) {
                    Ok(_) => {
                        self.status_message = format!("コピーしました: {}", src.display());
                        if let Some(ref folder) = self.current_folder.clone() {
                            self.update_folder_pdfs(folder);
                        }
                    }
                    Err(e) => {
                        self.status_message = format!("コピーエラー: {}", e);
                    }
                }
            }
        }

        // ファイル削除
        if let Some(path) = file_deleted {
            let result = if path.is_dir() {
                std::fs::remove_dir_all(&path)
            } else {
                std::fs::remove_file(&path)
            };

            match result {
                Ok(_) => {
                    self.status_message = format!("削除しました: {}", path.display());
                    if let Some(ref folder) = self.current_folder.clone() {
                        self.update_folder_pdfs(folder);
                    }
                }
                Err(e) => {
                    self.status_message = format!("削除エラー: {}", e);
                }
            }
        }
    }

    /// カスタムスタンプを登録
    fn register_custom_stamp(&mut self, path: PathBuf) {
        let name = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
        self.custom_stamps.push(CustomStamp {
            name,
            path: path.clone(),
        });
        self.status_message = format!("スタンプを登録しました: {}", path.display());
    }
}

/// ディレクトリを再帰的にコピー
fn copy_dir_all(src: &PathBuf, dest: &PathBuf) -> std::io::Result<()> {
    std::fs::create_dir_all(dest)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dest.join(entry.file_name()))?;
        } else {
            std::fs::copy(entry.path(), dest.join(entry.file_name()))?;
        }
    }
    Ok(())
}

impl eframe::App for PdfViewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // メニューバー
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("ファイル", |ui| {
                    if ui.button("📂 開く...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("PDF", &["pdf"])
                            .pick_file()
                        {
                            self.open_pdf(path);
                        }
                        ui.close_menu();
                    }
                    if ui.button("💾 保存...").clicked() {
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
                    if ui.button("➕ 結合用PDFを追加...").clicked() {
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
                    if ui.button("🔗 PDFを結合").clicked() {
                        self.merge_pdfs();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("✂ 分割...").clicked() {
                        self.show_split_dialog = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("❌ 終了").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                ui.menu_button("編集", |ui| {
                    if ui.button("🔄 90°回転").clicked() {
                        let page = self.selected_page;
                        self.rotate_page(page, 90);
                        ui.close_menu();
                    }
                    if ui.button("🔄 180°回転").clicked() {
                        let page = self.selected_page;
                        self.rotate_page(page, 180);
                        ui.close_menu();
                    }
                    if ui.button("🔄 270°回転").clicked() {
                        let page = self.selected_page;
                        self.rotate_page(page, 270);
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("✅ スタンプパネル").clicked() {
                        self.show_stamp_panel = !self.show_stamp_panel;
                        ui.close_menu();
                    }
                    if ui.button("📝 テキストパネル").clicked() {
                        self.show_text_panel = !self.show_text_panel;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("🖼 スタンプを登録...").clicked() {
                        self.show_stamp_register_dialog = true;
                        ui.close_menu();
                    }
                });

                ui.menu_button("表示", |ui| {
                    if ui.button("🌙 ダークモード").clicked() {
                        ctx.set_visuals(egui::Visuals::dark());
                        ui.close_menu();
                    }
                    if ui.button("☀ ライトモード").clicked() {
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

        // 左パネル: ファイルエクスプローラー（ツリー表示）
        egui::SidePanel::left("file_explorer")
            .default_width(250.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui.heading("📁 ファイル");
                ui.separator();
                let file_result = self.file_explorer.show(ui);
                
                // フォルダが選択された場合
                if let Some(folder_path) = file_result.selected_folder {
                    self.update_folder_pdfs(&folder_path);
                }
                
                // PDFファイルが選択された場合
                if let Some(file_path) = file_result.selected_file {
                    self.open_pdf(file_path);
                }
                
                // ファイル操作
                self.handle_file_operations(
                    file_result.file_moved,
                    file_result.file_copied,
                    file_result.file_deleted
                );
            });

        // 右パネル: プレビュー（シンプル版）
        let has_document = self.current_document.is_some();
        let page_count = self.current_document.as_ref().map(|d| d.page_count()).unwrap_or(0);
        
        egui::SidePanel::right("preview_panel")
            .default_width(500.0)
            .min_width(300.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui.heading("📄 プレビュー");
                ui.separator();

                if has_document {
                    // ツールバー
                    let mut prev_clicked = false;
                    let mut next_clicked = false;
                    let mut rotate_clicked = false;
                    
                    ui.horizontal(|ui| {
                        prev_clicked = ui.button("◀ 前").clicked() && self.selected_page > 0;
                        ui.label(format!("  {} / {}  ", self.selected_page + 1, page_count));
                        next_clicked = ui.button("次 ▶").clicked() && self.selected_page < page_count - 1;

                        ui.separator();

                        rotate_clicked = ui.button("🔄 回転").clicked();

                        ui.separator();

                        if ui.selectable_label(self.show_stamp_panel, "✅ スタンプ").clicked() {
                            self.show_stamp_panel = !self.show_stamp_panel;
                            self.show_text_panel = false;
                        }
                        if ui.selectable_label(self.show_text_panel, "📝 テキスト").clicked() {
                            self.show_text_panel = !self.show_text_panel;
                            self.show_stamp_panel = false;
                        }
                    });

                    if prev_clicked {
                        self.selected_page -= 1;
                        self.editor_panel.invalidate_cache();
                    }
                    if next_clicked {
                        self.selected_page += 1;
                        self.editor_panel.invalidate_cache();
                    }
                    if rotate_clicked {
                        let page = self.selected_page;
                        self.rotate_page(page, 90);
                    }

                    ui.separator();

                    // プレビュー（スクロール可能、全体を使用）
                    let mut new_stamp = None;
                    let mut new_text = None;
                    
                    egui::ScrollArea::both()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
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
                                new_stamp = editor_result.new_stamp;
                                new_text = editor_result.new_text;
                            }
                        });

                    if let Some(stamp) = new_stamp {
                        self.stamps.push(stamp);
                    }
                    if let Some(annotation) = new_text {
                        self.text_annotations.push(annotation);
                    }
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
                ui.heading(format!("📚 PDFファイル ({} 件)", self.folder_pdfs.len()));
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
                let mut right_clicked_pdf: Option<(usize, egui::Pos2)> = None;
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
                                    let frame_response = egui::Frame::none()
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

                                                // クリック処理
                                                if response.clicked() {
                                                    clicked_pdf = Some((*idx, path.clone()));
                                                }
                                                
                                                // 右クリック処理
                                                if response.secondary_clicked() {
                                                    if let Some(pos) = ui.input(|i| i.pointer.hover_pos()) {
                                                        right_clicked_pdf = Some((*idx, pos));
                                                    }
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

                                    // フレーム全体の右クリック
                                    if frame_response.response.secondary_clicked() {
                                        if let Some(pos) = ui.input(|i| i.pointer.hover_pos()) {
                                            right_clicked_pdf = Some((*idx, pos));
                                        }
                                    }

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

                // 右クリックでコンテキストメニュー表示
                if let Some((idx, pos)) = right_clicked_pdf {
                    self.context_menu_pdf = Some((idx, pos));
                }
            }
        });

        // PDFコンテキストメニュー
        if let Some((idx, pos)) = self.context_menu_pdf {
            let pdf_path = self.folder_pdfs.get(idx).map(|e| e.path.clone());
            
            egui::Area::new(egui::Id::new("pdf_context_menu"))
                .fixed_pos(pos)
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        ui.set_min_width(180.0);
                        
                        if let Some(ref path) = pdf_path {
                            if ui.button("📂 このアプリで開く").clicked() {
                                self.open_pdf(path.clone());
                                self.selected_pdf_index = Some(idx);
                                self.context_menu_pdf = None;
                            }
                            
                            if ui.button("🌐 外部アプリで開く").clicked() {
                                self.open_with_external(path);
                                self.context_menu_pdf = None;
                            }
                            
                            if ui.button("📋 プログラムから開く...").clicked() {
                                self.open_with_dialog(path);
                                self.context_menu_pdf = None;
                            }
                            
                            ui.separator();
                            
                            if ui.button("📄 エクスプローラーで表示").clicked() {
                                #[cfg(windows)]
                                {
                                    let _ = Command::new("explorer")
                                        .args(["/select,", &path.to_string_lossy()])
                                        .spawn();
                                }
                                self.context_menu_pdf = None;
                            }
                            
                            ui.separator();
                            
                            if ui.button("🗑 削除").clicked() {
                                if let Err(e) = std::fs::remove_file(path) {
                                    self.status_message = format!("削除エラー: {}", e);
                                } else {
                                    self.status_message = format!("削除しました: {}", path.display());
                                    if let Some(ref folder) = self.current_folder.clone() {
                                        self.update_folder_pdfs(folder);
                                    }
                                }
                                self.context_menu_pdf = None;
                            }
                        }
                    });
                });

            // メニュー外クリックで閉じる
            if ctx.input(|i| i.pointer.any_click()) {
                let pointer_pos = ctx.input(|i| i.pointer.hover_pos());
                if let Some(click_pos) = pointer_pos {
                    let menu_rect = egui::Rect::from_min_size(pos, egui::vec2(180.0, 200.0));
                    if !menu_rect.contains(click_pos) {
                        self.context_menu_pdf = None;
                    }
                }
            }
        }

        // 分割ダイアログ
        if self.show_split_dialog {
            egui::Window::new("✂ PDF分割")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    if let Some(ref doc) = self.current_document {
                        ui.label(format!("総ページ数: {}", doc.page_count()));
                        ui.separator();
                    }
                    
                    ui.horizontal(|ui| {
                        ui.label("開始ページ:");
                        ui.text_edit_singleline(&mut self.split_start_page);
                    });
                    ui.horizontal(|ui| {
                        ui.label("終了ページ:");
                        ui.text_edit_singleline(&mut self.split_end_page);
                    });
                    ui.separator();
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

        // スタンプ登録ダイアログ
        if self.show_stamp_register_dialog {
            egui::Window::new("🖼 カスタムスタンプ登録")
                .collapsible(false)
                .resizable(true)
                .show(ctx, |ui| {
                    ui.label("PNG画像ファイルを選択して、カスタムスタンプとして登録できます。");
                    ui.separator();
                    
                    // 既存のカスタムスタンプ一覧
                    if !self.custom_stamps.is_empty() {
                        ui.label(format!("登録済みスタンプ: {} 個", self.custom_stamps.len()));
                        egui::ScrollArea::vertical()
                            .max_height(150.0)
                            .show(ui, |ui| {
                                for stamp in &self.custom_stamps {
                                    ui.label(format!("• {}", stamp.name));
                                }
                            });
                        ui.separator();
                    }
                    
                    ui.horizontal(|ui| {
                        if ui.button("📂 PNG画像を追加...").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("PNG", &["png"])
                                .pick_file()
                            {
                                self.register_custom_stamp(path);
                            }
                        }
                        
                        if ui.button("閉じる").clicked() {
                            self.show_stamp_register_dialog = false;
                        }
                    });
                });
        }
    }
}
