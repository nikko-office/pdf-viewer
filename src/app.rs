//! アプリケーションの状態管理

use crate::pdf::{PdfDocument, PdfOperations, Stamp, TextAnnotation};
use crate::ui::{EditorPanel, FileExplorer};
use eframe::egui::{self, Color32, TextureHandle, Vec2};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;
use std::time::SystemTime;

/// アプリケーション全体の状態
pub struct PdfViewerApp {
    // UI パネル
    file_explorer: FileExplorer,
    editor_panel: EditorPanel,

    // PDF ドキュメント
    current_document: Option<PdfDocument>,
    current_pdf_path: Option<PathBuf>,
    documents: Vec<PdfDocument>,

    // 編集状態
    selected_page: usize,
    stamps: Vec<Stamp>,
    text_annotations: Vec<TextAnnotation>,
    has_unsaved_changes: bool,

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

    // カスタムスタンプ（PNG透過対応）
    custom_stamps: Vec<CustomStamp>,
    custom_stamp_textures: Vec<Option<TextureHandle>>,

    // コンテキストメニュー
    context_menu_pdf: Option<(usize, egui::Pos2)>,

    // ドラッグ中のPDFパス
    dragging_pdf: Option<PathBuf>,

    // ステータスメッセージ
    status_message: String,
}

/// フォルダ内のPDFエントリ
struct FolderPdfEntry {
    path: PathBuf,
    name: String,
    modified: SystemTime,
}

/// 注釈データ（保存用）
#[derive(Serialize, Deserialize)]
struct AnnotationData {
    stamps: Vec<Stamp>,
    texts: Vec<TextAnnotation>,
}

/// カスタムスタンプ（PNG透過対応）
#[derive(Clone)]
pub struct CustomStamp {
    pub name: String,
    pub path: PathBuf,
    pub image_data: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

impl PdfViewerApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            file_explorer: FileExplorer::new(),
            editor_panel: EditorPanel::new(),
            current_document: None,
            current_pdf_path: None,
            documents: Vec::new(),
            selected_page: 0,
            stamps: Vec::new(),
            text_annotations: Vec::new(),
            has_unsaved_changes: false,
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
            custom_stamp_textures: Vec::new(),
            context_menu_pdf: None,
            dragging_pdf: None,
            status_message: "準備完了".to_string(),
        }
    }

    /// PDFファイルを開く
    pub fn open_pdf(&mut self, path: PathBuf) {
        match PdfDocument::open(&path) {
            Ok(doc) => {
                self.current_document = Some(doc);
                self.current_pdf_path = Some(path.clone());
                self.selected_page = 0;
                self.editor_panel.invalidate_cache();
                
                // 注釈ファイルを読み込み
                self.stamps.clear();
                self.text_annotations.clear();
                self.load_annotations(&path);
                
                self.has_unsaved_changes = false;
                self.status_message = format!("開きました: {}", path.display());
            }
            Err(e) => {
                self.status_message = format!("エラー: {}", e);
                log::error!("PDFを開けません: {}", e);
            }
        }
    }

    /// 上書き保存（注釈を保存）
    fn save_current(&mut self) {
        if let Some(ref path) = self.current_pdf_path.clone() {
            self.save_annotations(&path);
            self.has_unsaved_changes = false;
            self.status_message = format!("保存しました: {}", path.display());
        } else {
            self.status_message = "保存先が指定されていません".to_string();
        }
    }

    /// 注釈ファイルのパスを取得
    fn get_annotations_path(pdf_path: &PathBuf) -> PathBuf {
        let mut path = pdf_path.clone();
        path.set_extension("annotations.json");
        path
    }

    /// 注釈を読み込み
    fn load_annotations(&mut self, pdf_path: &PathBuf) {
        let ann_path = Self::get_annotations_path(pdf_path);
        if ann_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&ann_path) {
                if let Ok(data) = serde_json::from_str::<AnnotationData>(&content) {
                    self.stamps = data.stamps;
                    self.text_annotations = data.texts;
                    self.status_message = format!("注釈を読み込みました");
                }
            }
        }
    }

    /// 注釈を保存
    fn save_annotations(&self, pdf_path: &PathBuf) {
        let ann_path = Self::get_annotations_path(pdf_path);
        let data = AnnotationData {
            stamps: self.stamps.clone(),
            texts: self.text_annotations.clone(),
        };
        if let Ok(content) = serde_json::to_string_pretty(&data) {
            if let Err(e) = std::fs::write(&ann_path, content) {
                log::error!("注釈の保存に失敗: {}", e);
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

    /// フォルダ内のPDFを更新（新しい順）
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
                    let modified = entry.metadata()
                        .and_then(|m| m.modified())
                        .unwrap_or(SystemTime::UNIX_EPOCH);
                    self.folder_pdfs.push(FolderPdfEntry { path, name, modified });
                }
            }
        }

        // 新しい順にソート（更新日時の降順）
        self.folder_pdfs.sort_by(|a, b| b.modified.cmp(&a.modified));
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
                    self.has_unsaved_changes = false;
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
            match PdfOperations::rotate_page(doc, page, angle) {
                Ok(_) => {
                    self.status_message = format!("ページ {} を {}° 回転しました", page + 1, angle);
                    self.editor_panel.invalidate_cache();
                    self.has_unsaved_changes = true;
                }
                Err(e) => {
                    self.status_message = format!("回転エラー: {}", e);
                }
            }
        }
    }

    /// ファイル操作を実行
    fn handle_file_operations(&mut self, 
        file_moved: Option<(PathBuf, PathBuf)>,
        file_copied: Option<(PathBuf, PathBuf)>,
        file_deleted: Option<PathBuf>
    ) {
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

    /// カスタムスタンプを登録（PNG透過対応）
    fn register_custom_stamp(&mut self, path: PathBuf) {
        match image::open(&path) {
            Ok(img) => {
                let rgba = img.to_rgba8();
                let (width, height) = rgba.dimensions();
                let image_data = rgba.into_raw();
                
                let name = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
                self.custom_stamps.push(CustomStamp {
                    name: name.clone(),
                    path: path.clone(),
                    image_data,
                    width,
                    height,
                });
                self.custom_stamp_textures.push(None);
                self.status_message = format!("スタンプを登録しました: {}", name);
            }
            Err(e) => {
                self.status_message = format!("画像の読み込みエラー: {}", e);
            }
        }
    }

    /// カスタムスタンプのテクスチャを取得/生成
    fn get_custom_stamp_texture(&mut self, ctx: &egui::Context, index: usize) -> Option<TextureHandle> {
        if index >= self.custom_stamps.len() {
            return None;
        }

        if self.custom_stamp_textures.get(index).and_then(|t| t.as_ref()).is_none() {
            let stamp = &self.custom_stamps[index];
            let image = egui::ColorImage::from_rgba_unmultiplied(
                [stamp.width as usize, stamp.height as usize],
                &stamp.image_data,
            );
            let texture = ctx.load_texture(
                format!("custom_stamp_{}", index),
                image,
                egui::TextureOptions::LINEAR,
            );
            if index < self.custom_stamp_textures.len() {
                self.custom_stamp_textures[index] = Some(texture);
            }
        }

        self.custom_stamp_textures.get(index).and_then(|t| t.clone())
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
                    
                    let save_enabled = self.current_pdf_path.is_some();
                    if ui.add_enabled(save_enabled, egui::Button::new("💾 上書き保存")).clicked() {
                        self.save_current();
                        ui.close_menu();
                    }
                    
                    if ui.button("📄 名前を付けて保存...").clicked() {
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
                    let has_doc = self.current_document.is_some();
                    
                    if ui.add_enabled(has_doc, egui::Button::new("🔄 90°回転")).clicked() {
                        let page = self.selected_page;
                        self.rotate_page(page, 90);
                        ui.close_menu();
                    }
                    if ui.add_enabled(has_doc, egui::Button::new("🔄 180°回転")).clicked() {
                        let page = self.selected_page;
                        self.rotate_page(page, 180);
                        ui.close_menu();
                    }
                    if ui.add_enabled(has_doc, egui::Button::new("🔄 270°回転")).clicked() {
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
                    if ui.button("🔄 フォルダを更新").clicked() {
                        if let Some(ref folder) = self.current_folder.clone() {
                            self.update_folder_pdfs(folder);
                        }
                        ui.close_menu();
                    }
                    ui.separator();
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
                // 未保存マーク
                if self.has_unsaved_changes {
                    ui.label(egui::RichText::new("●").color(Color32::RED));
                }
                ui.label(&self.status_message);
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(ref doc) = self.current_document {
                        ui.label(format!(
                            "ページ: {} / {}",
                            self.selected_page + 1,
                            doc.page_count()
                        ));
                    }
                    if !self.custom_stamps.is_empty() {
                        ui.label(format!("| スタンプ: {} 個", self.custom_stamps.len()));
                    }
                });
            });
        });

        // 左パネル: ファイルエクスプローラー
        egui::SidePanel::left("file_explorer")
            .default_width(250.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui.heading("📁 ファイル");
                ui.separator();
                let file_result = self.file_explorer.show(ui);
                
                if let Some(folder_path) = file_result.selected_folder {
                    self.update_folder_pdfs(&folder_path);
                }
                
                if let Some(file_path) = file_result.selected_file {
                    self.open_pdf(file_path);
                }
                
                self.handle_file_operations(
                    file_result.file_moved,
                    file_result.file_copied,
                    file_result.file_deleted
                );
            });

        // 右パネル: プレビュー
        let has_document = self.current_document.is_some();
        let page_count = self.current_document.as_ref().map(|d| d.page_count()).unwrap_or(0);
        
        // カスタムスタンプ情報を収集（先にテクスチャを生成）
        for i in 0..self.custom_stamps.len() {
            let _ = self.get_custom_stamp_texture(ctx, i);
        }
        
        let custom_stamp_info: Vec<(String, Option<TextureHandle>)> = self.custom_stamps
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let tex = self.custom_stamp_textures.get(i).and_then(|t| t.clone());
                (s.name.clone(), tex)
            })
            .collect();
        
        egui::SidePanel::right("preview_panel")
            .default_width(500.0)
            .min_width(300.0)
            .resizable(true)
            .show(ctx, |ui| {
                // タイトルと保存ボタン
                ui.horizontal(|ui| {
                    ui.heading("📄 プレビュー");
                    
                    if self.has_unsaved_changes {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("💾 保存").clicked() {
                                self.save_current();
                            }
                        });
                    }
                });
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

                    // プレビュー
                    let mut new_stamp = None;
                    let mut new_text = None;
                    let mut delete_stamp = None;
                    let mut delete_text = None;
                    let mut move_stamp = None;
                    let mut move_text = None;
                    let mut delete_custom_stamp = None;
                    let mut register_stamp_clicked = false;
                    
                    egui::ScrollArea::both()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            if let Some(ref doc) = self.current_document {
                                let editor_result = self.editor_panel.show_with_custom_stamps(
                                    ui,
                                    doc,
                                    self.selected_page,
                                    &self.stamps,
                                    &self.text_annotations,
                                    self.show_stamp_panel,
                                    self.show_text_panel,
                                    &custom_stamp_info,
                                );
                                new_stamp = editor_result.new_stamp;
                                new_text = editor_result.new_text;
                                delete_stamp = editor_result.delete_stamp;
                                delete_text = editor_result.delete_text;
                                move_stamp = editor_result.move_stamp;
                                move_text = editor_result.move_text;
                                delete_custom_stamp = editor_result.delete_custom_stamp;
                                register_stamp_clicked = editor_result.register_stamp_clicked;
                            }
                        });

                    // スタンプ追加
                    if let Some(stamp) = new_stamp {
                        self.stamps.push(stamp);
                        self.has_unsaved_changes = true;
                        self.status_message = "スタンプを追加しました".to_string();
                    }
                    // テキスト追加
                    if let Some(annotation) = new_text {
                        self.text_annotations.push(annotation);
                        self.has_unsaved_changes = true;
                        self.status_message = "テキストを追加しました".to_string();
                    }
                    // スタンプ削除
                    if let Some(idx) = delete_stamp {
                        if idx < self.stamps.len() {
                            self.stamps.remove(idx);
                            self.has_unsaved_changes = true;
                            self.status_message = "スタンプを削除しました".to_string();
                        }
                    }
                    // テキスト削除
                    if let Some(idx) = delete_text {
                        if idx < self.text_annotations.len() {
                            self.text_annotations.remove(idx);
                            self.has_unsaved_changes = true;
                            self.status_message = "テキストを削除しました".to_string();
                        }
                    }
                    // スタンプ移動
                    if let Some((idx, new_x, new_y)) = move_stamp {
                        if idx < self.stamps.len() {
                            self.stamps[idx].x = new_x;
                            self.stamps[idx].y = new_y;
                            self.has_unsaved_changes = true;
                        }
                    }
                    // テキスト移動
                    if let Some((idx, new_x, new_y)) = move_text {
                        if idx < self.text_annotations.len() {
                            self.text_annotations[idx].x = new_x;
                            self.text_annotations[idx].y = new_y;
                            self.has_unsaved_changes = true;
                        }
                    }
                    // カスタムスタンプ削除
                    if let Some(idx) = delete_custom_stamp {
                        if idx < self.custom_stamps.len() {
                            let name = self.custom_stamps[idx].name.clone();
                            self.custom_stamps.remove(idx);
                            if idx < self.custom_stamp_textures.len() {
                                self.custom_stamp_textures.remove(idx);
                            }
                            self.status_message = format!("スタンプ「{}」を削除しました", name);
                        }
                    }
                    // スタンプ登録ダイアログ
                    if register_stamp_clicked {
                        self.show_stamp_register_dialog = true;
                    }
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.label("PDFファイルを選択してください");
                    });
                }
            });

        // 中央パネル: フォルダ内PDFサムネイル一覧（新しい順）
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.folder_pdfs.is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.label("左側のフォルダを選択すると、PDFファイルが表示されます");
                });
            } else {
                ui.horizontal(|ui| {
                    ui.heading(format!("📚 PDFファイル ({} 件)", self.folder_pdfs.len()));
                    ui.label("- 新しい順");
                    if ui.button("🔄").on_hover_text("更新").clicked() {
                        if let Some(ref folder) = self.current_folder.clone() {
                            self.update_folder_pdfs(folder);
                        }
                    }
                });
                ui.separator();

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
                let mut drag_started_pdf: Option<PathBuf> = None;
                let mut drag_ended = false;

                // ドラッグ中のヒント表示
                if self.dragging_pdf.is_some() {
                    ui.horizontal(|ui| {
                        ui.colored_label(Color32::YELLOW, "📂 左のフォルダツリーにドロップして移動");
                    });
                }

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
                                    let is_dragging = self.dragging_pdf.as_ref() == Some(path);
                                    
                                    let frame_response = egui::Frame::none()
                                        .fill(if is_dragging {
                                            Color32::from_rgba_unmultiplied(100, 150, 200, 100)
                                        } else if *is_selected {
                                            Color32::from_rgb(70, 130, 180)
                                        } else {
                                            Color32::from_gray(45)
                                        })
                                        .stroke(egui::Stroke::new(
                                            if *is_selected { 3.0 } else { 1.0 },
                                            if is_dragging {
                                                Color32::YELLOW
                                            } else if *is_selected {
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
                                                let (rect, response) = ui.allocate_exact_size(
                                                    Vec2::new(thumb_width - 16.0, thumb_height - 50.0),
                                                    egui::Sense::click_and_drag(),
                                                );

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

                                                // ドラッグ開始
                                                if response.drag_started() {
                                                    drag_started_pdf = Some(path.clone());
                                                }
                                                
                                                // ドラッグ中
                                                if response.dragged() {
                                                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                                                }
                                                
                                                // ドラッグ終了
                                                if response.drag_stopped() {
                                                    drag_ended = true;
                                                }

                                                if response.clicked() {
                                                    clicked_pdf = Some((*idx, path.clone()));
                                                }
                                                
                                                if response.secondary_clicked() {
                                                    if let Some(pos) = ui.input(|i| i.pointer.hover_pos()) {
                                                        right_clicked_pdf = Some((*idx, pos));
                                                    }
                                                }

                                                ui.add_space(4.0);
                                                ui.label(
                                                    egui::RichText::new(name)
                                                        .size(11.0)
                                                        .color(Color32::WHITE),
                                                );
                                            });
                                        });

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
                
                // ドラッグ状態の更新
                if let Some(path) = drag_started_pdf {
                    self.dragging_pdf = Some(path);
                }
                
                // ドラッグ終了時にフォルダにドロップ
                if drag_ended {
                    if let Some(source_path) = self.dragging_pdf.take() {
                        if let Some(target_folder) = self.file_explorer.get_drop_target().cloned() {
                            let file_name = source_path.file_name().unwrap_or_default();
                            let dest_path = target_folder.join(file_name);
                            
                            if source_path != dest_path {
                                match std::fs::rename(&source_path, &dest_path) {
                                    Ok(_) => {
                                        self.status_message = format!(
                                            "移動しました: {} → {}",
                                            source_path.display(),
                                            target_folder.display()
                                        );
                                        // フォルダ内PDFリストを更新
                                        if let Some(ref folder) = self.current_folder.clone() {
                                            self.update_folder_pdfs(folder);
                                        }
                                    }
                                    Err(e) => {
                                        self.status_message = format!("移動に失敗しました: {}", e);
                                    }
                                }
                            }
                        }
                    }
                    self.file_explorer.clear_drop_target();
                }

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

                if let Some((idx, path)) = clicked_pdf {
                    self.selected_pdf_index = Some(idx);
                    self.open_pdf(path);
                }

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
            // 事前にテクスチャを準備
            let stamp_textures: Vec<(String, Option<egui::TextureId>)> = self.custom_stamps
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    let tex_id = self.custom_stamp_textures.get(i)
                        .and_then(|t| t.as_ref().map(|t| t.id()));
                    (s.name.clone(), tex_id)
                })
                .collect();
            
            let mut add_stamp_path: Option<PathBuf> = None;
            let mut close_dialog = false;
            
            egui::Window::new("🖼 カスタムスタンプ登録")
                .collapsible(false)
                .resizable(true)
                .show(ctx, |ui| {
                    ui.label("PNG画像（透過対応）を選択して、スタンプとして登録できます。");
                    ui.separator();
                    
                    if !stamp_textures.is_empty() {
                        ui.label(format!("登録済みスタンプ: {} 個", stamp_textures.len()));
                        egui::ScrollArea::vertical()
                            .max_height(200.0)
                            .show(ui, |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    for (name, tex_id) in &stamp_textures {
                                        ui.group(|ui| {
                                            ui.set_width(80.0);
                                            if let Some(id) = tex_id {
                                                ui.image((*id, Vec2::new(60.0, 60.0)));
                                            }
                                            ui.label(name);
                                        });
                                    }
                                });
                            });
                        ui.separator();
                    }
                    
                    ui.horizontal(|ui| {
                        if ui.button("📂 PNG画像を追加...").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("PNG画像", &["png"])
                                .pick_file()
                            {
                                add_stamp_path = Some(path);
                            }
                        }
                        
                        if ui.button("閉じる").clicked() {
                            close_dialog = true;
                        }
                    });
                });
            
            if let Some(path) = add_stamp_path {
                self.register_custom_stamp(path);
            }
            if close_dialog {
                self.show_stamp_register_dialog = false;
            }
        }
    }
}
