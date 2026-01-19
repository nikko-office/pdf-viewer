//! サムネイルパネル - ページ一覧とドラッグ&ドロップ

use crate::pdf::PdfDocument;
use eframe::egui::{self, Color32, TextureHandle, Vec2};

/// サムネイル操作の結果
pub struct ThumbnailResult {
    pub selected_page: Option<usize>,
    pub page_reorder: Option<(usize, usize)>,
    pub page_deleted: Option<usize>,
    pub page_rotated: Option<(usize, i32)>,
}

impl Default for ThumbnailResult {
    fn default() -> Self {
        Self {
            selected_page: None,
            page_reorder: None,
            page_deleted: None,
            page_rotated: None,
        }
    }
}

/// サムネイルパネルの状態
pub struct ThumbnailPanel {
    thumbnails: Vec<Option<TextureHandle>>,
    thumbnail_size: Vec2,
    drag_state: Option<DragState>,
    context_menu_page: Option<usize>,
}

/// ドラッグ状態
struct DragState {
    from_index: usize,
    current_pos: egui::Pos2,
}

impl ThumbnailPanel {
    pub fn new() -> Self {
        Self {
            thumbnails: Vec::new(),
            thumbnail_size: Vec2::new(150.0, 200.0),
            drag_state: None,
            context_menu_page: None,
        }
    }

    /// PDFドキュメントからサムネイルをロード
    pub fn load_thumbnails(&mut self, doc: &PdfDocument) {
        self.thumbnails.clear();
        self.thumbnails.resize(doc.page_count(), None);
    }

    /// UIを描画
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        doc: &PdfDocument,
        selected_page: usize,
    ) -> ThumbnailResult {
        let mut result = ThumbnailResult::default();
        let page_count = doc.page_count();

        if page_count == 0 {
            ui.label("ページがありません");
            return result;
        }

        // サムネイル数を調整
        if self.thumbnails.len() != page_count {
            self.thumbnails.resize(page_count, None);
        }

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                for i in 0..page_count {
                    let is_selected = i == selected_page;
                    let is_being_dragged =
                        self.drag_state.as_ref().map_or(false, |d| d.from_index == i);

                    // サムネイルフレーム
                    let frame_color = if is_selected {
                        Color32::from_rgb(100, 149, 237) // コーンフラワーブルー
                    } else {
                        Color32::from_gray(60)
                    };

                    egui::Frame::none()
                        .fill(if is_being_dragged {
                            Color32::from_gray(80)
                        } else {
                            Color32::from_gray(40)
                        })
                        .stroke(egui::Stroke::new(
                            if is_selected { 3.0 } else { 1.0 },
                            frame_color,
                        ))
                        .inner_margin(4.0)
                        .outer_margin(4.0)
                        .rounding(4.0)
                        .show(ui, |ui: &mut egui::Ui| {
                            ui.vertical(|ui: &mut egui::Ui| {
                                // ページ番号
                                ui.label(
                                    egui::RichText::new(format!("ページ {}", i + 1))
                                        .size(12.0)
                                        .color(Color32::WHITE),
                                );

                                // サムネイル画像エリア
                                let (rect, response) = ui.allocate_exact_size(
                                    self.thumbnail_size,
                                    egui::Sense::click_and_drag(),
                                );

                                // サムネイル描画
                                if let Some(ref texture) = self.thumbnails[i] {
                                    ui.painter().image(
                                        texture.id(),
                                        rect,
                                        egui::Rect::from_min_max(
                                            egui::pos2(0.0, 0.0),
                                            egui::pos2(1.0, 1.0),
                                        ),
                                        Color32::WHITE,
                                    );
                                } else {
                                    // サムネイルがまだない場合はプレースホルダー
                                    ui.painter().rect_filled(rect, 0.0, Color32::from_gray(50));
                                    ui.painter().text(
                                        rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        "読込中...",
                                        egui::FontId::default(),
                                        Color32::WHITE,
                                    );

                                    // サムネイルをレンダリング
                                    if let Some(image) = doc.render_page_thumbnail(i, 150, 200) {
                                        let texture = ui.ctx().load_texture(
                                            format!("thumbnail_{}", i),
                                            image,
                                            egui::TextureOptions::LINEAR,
                                        );
                                        self.thumbnails[i] = Some(texture);
                                    }
                                }

                                // クリックでページ選択
                                if response.clicked() {
                                    result.selected_page = Some(i);
                                }

                                // 右クリックでコンテキストメニュー
                                if response.secondary_clicked() {
                                    self.context_menu_page = Some(i);
                                }

                                // ドラッグ開始
                                if response.drag_started() {
                                    self.drag_state = Some(DragState {
                                        from_index: i,
                                        current_pos: response.interact_pointer_pos().unwrap_or_default(),
                                    });
                                }

                                // ドラッグ中
                                if response.dragged() {
                                    if let Some(ref mut drag) = self.drag_state {
                                        if let Some(pos) = response.interact_pointer_pos() {
                                            drag.current_pos = pos;
                                        }
                                    }
                                }

                                // ドラッグ終了（ドロップ）
                                if response.drag_stopped() {
                                    if let Some(drag) = self.drag_state.take() {
                                        if drag.from_index != i && i < page_count {
                                            result.page_reorder = Some((drag.from_index, i));
                                        }
                                    }
                                }

                                // ドロップターゲットのハイライト
                                if let Some(ref drag) = self.drag_state {
                                    if drag.from_index != i && rect.contains(drag.current_pos) {
                                        ui.painter().rect_stroke(
                                            rect.expand(2.0),
                                            4.0,
                                            egui::Stroke::new(2.0, Color32::YELLOW),
                                        );
                                    }
                                }
                            });
                        });
                }
            });

        // コンテキストメニュー
        if let Some(page) = self.context_menu_page {
            egui::Area::new(egui::Id::new("page_context_menu"))
                .order(egui::Order::Foreground)
                .show(ui.ctx(), |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui: &mut egui::Ui| {
                        ui.set_min_width(120.0);

                        if ui.button("🔄 90°回転").clicked() {
                            result.page_rotated = Some((page, 90));
                            self.context_menu_page = None;
                        }
                        if ui.button("🔄 180°回転").clicked() {
                            result.page_rotated = Some((page, 180));
                            self.context_menu_page = None;
                        }
                        if ui.button("🔄 270°回転").clicked() {
                            result.page_rotated = Some((page, 270));
                            self.context_menu_page = None;
                        }
                        ui.separator();
                        if ui.button("🗑 ページ削除").clicked() {
                            result.page_deleted = Some(page);
                            self.context_menu_page = None;
                        }
                    });
                });

            // メニュー外クリックで閉じる
            if ui.input(|i| i.pointer.any_click()) {
                self.context_menu_page = None;
            }
        }

        result
    }
}
