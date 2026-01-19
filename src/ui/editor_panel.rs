//! メイン編集パネル - PDF表示、スタンプ配置、テキスト入力

use crate::pdf::{PdfDocument, Stamp, StampType, TextAnnotation};
use eframe::egui::{self, Color32, TextureHandle, Vec2};

/// エディター操作の結果
pub struct EditorResult {
    pub new_stamp: Option<Stamp>,
    pub new_text: Option<TextAnnotation>,
}

impl Default for EditorResult {
    fn default() -> Self {
        Self {
            new_stamp: None,
            new_text: None,
        }
    }
}

/// エディターパネルの状態
pub struct EditorPanel {
    // ページテクスチャのキャッシュ
    page_texture: Option<TextureHandle>,
    current_page_index: Option<usize>,

    // ズーム
    zoom: f32,

    // スタンプ配置モード
    selected_stamp_type: StampType,
    placing_stamp: bool,

    // テキスト入力
    text_input: String,
    text_font_size: f32,
    placing_text: bool,
}

impl EditorPanel {
    pub fn new() -> Self {
        Self {
            page_texture: None,
            current_page_index: None,
            zoom: 1.0,
            selected_stamp_type: StampType::Approved,
            placing_stamp: false,
            text_input: String::new(),
            text_font_size: 14.0,
            placing_text: false,
        }
    }

    /// UIを描画
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        doc: &PdfDocument,
        page_index: usize,
        stamps: &[Stamp],
        text_annotations: &[TextAnnotation],
        show_stamp_panel: bool,
        show_text_panel: bool,
    ) -> EditorResult {
        let mut result = EditorResult::default();

        // ズームコントロール
        ui.horizontal(|ui| {
            ui.label("ズーム:");
            if ui.button("−").clicked() {
                self.zoom = (self.zoom - 0.25).max(0.25);
                self.invalidate_page_cache();
            }
            ui.label(format!("{:.0}%", self.zoom * 100.0));
            if ui.button("＋").clicked() {
                self.zoom = (self.zoom + 0.25).min(4.0);
                self.invalidate_page_cache();
            }
            if ui.button("リセット").clicked() {
                self.zoom = 1.0;
                self.invalidate_page_cache();
            }
        });

        // スタンプパネル（横に展開）
        if show_stamp_panel {
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("スタンプ:");
                
                let stamp_types = [
                    (StampType::Approved, "✅承認"),
                    (StampType::Rejected, "❌却下"),
                    (StampType::Draft, "📝下書"),
                    (StampType::Confidential, "🔒機密"),
                ];

                for (stamp_type, label) in &stamp_types {
                    let selected = self.selected_stamp_type == *stamp_type;
                    if ui.selectable_label(selected, *label).clicked() {
                        self.selected_stamp_type = stamp_type.clone();
                    }
                }

                ui.separator();

                let btn_text = if self.placing_stamp { "🎯配置中" } else { "配置" };
                if ui.button(btn_text).clicked() {
                    self.placing_stamp = !self.placing_stamp;
                    self.placing_text = false;
                }
            });
        }

        // テキストパネル（横に展開）
        if show_text_panel {
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("テキスト:");
                ui.add(egui::TextEdit::singleline(&mut self.text_input).desired_width(150.0));
                ui.label("サイズ:");
                ui.add(egui::DragValue::new(&mut self.text_font_size).range(8.0..=72.0));

                let btn_text = if self.placing_text { "🎯配置中" } else { "配置" };
                if ui.button(btn_text).clicked() && !self.text_input.is_empty() {
                    self.placing_text = !self.placing_text;
                    self.placing_stamp = false;
                }
            });
        }

        ui.separator();

        // ページテクスチャを更新
        if self.current_page_index != Some(page_index) {
            self.current_page_index = Some(page_index);
            self.page_texture = None;
        }

        // ページサイズ計算
        let page_size = doc.page_size(page_index);
        let render_width = (page_size.0 * self.zoom) as u32;
        let render_height = (page_size.1 * self.zoom) as u32;

        // ページをレンダリング
        if self.page_texture.is_none() {
            if let Some(image) = doc.render_page(page_index, render_width, render_height) {
                self.page_texture = Some(ui.ctx().load_texture(
                    format!("page_{}", page_index),
                    image,
                    egui::TextureOptions::LINEAR,
                ));
            }
        }

        // ページ描画
        if let Some(ref texture) = self.page_texture {
            let size = Vec2::new(render_width as f32, render_height as f32);
            let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());

            // ページ画像描画
            ui.painter().image(
                texture.id(),
                rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );

            // 既存のスタンプを描画
            for stamp in stamps.iter().filter(|s| s.page == page_index) {
                let stamp_pos = egui::pos2(
                    rect.min.x + stamp.x * self.zoom,
                    rect.min.y + stamp.y * self.zoom,
                );
                let stamp_size = Vec2::new(stamp.width * self.zoom, stamp.height * self.zoom);
                let stamp_rect = egui::Rect::from_min_size(stamp_pos, stamp_size);

                let (bg_color, border_color) = match &stamp.stamp_type {
                    StampType::Approved => (Color32::from_rgba_unmultiplied(200, 255, 200, 180), Color32::GREEN),
                    StampType::Rejected => (Color32::from_rgba_unmultiplied(255, 200, 200, 180), Color32::RED),
                    StampType::Draft => (Color32::from_rgba_unmultiplied(255, 255, 200, 180), Color32::from_rgb(200, 150, 0)),
                    StampType::Confidential => (Color32::from_rgba_unmultiplied(200, 200, 255, 180), Color32::BLUE),
                    StampType::Custom(_) => (Color32::from_rgba_unmultiplied(220, 220, 220, 180), Color32::GRAY),
                };

                ui.painter().rect_filled(stamp_rect, 4.0, bg_color);
                ui.painter().rect_stroke(stamp_rect, 4.0, egui::Stroke::new(2.0, border_color));

                ui.painter().text(
                    stamp_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    stamp.stamp_type.label(),
                    egui::FontId::proportional(14.0 * self.zoom),
                    border_color,
                );
            }

            // 既存のテキスト注釈を描画
            for annotation in text_annotations.iter().filter(|a| a.page == page_index) {
                let text_pos = egui::pos2(
                    rect.min.x + annotation.x * self.zoom,
                    rect.min.y + annotation.y * self.zoom,
                );
                ui.painter().text(
                    text_pos,
                    egui::Align2::LEFT_TOP,
                    &annotation.text,
                    egui::FontId::proportional(annotation.font_size * self.zoom),
                    Color32::BLACK,
                );
            }

            // スタンプ配置モード
            if self.placing_stamp && response.clicked() {
                if let Some(pos) = response.interact_pointer_pos() {
                    let pdf_x = (pos.x - rect.min.x) / self.zoom;
                    let pdf_y = (pos.y - rect.min.y) / self.zoom;

                    result.new_stamp = Some(Stamp {
                        page: page_index,
                        x: pdf_x,
                        y: pdf_y,
                        width: 100.0,
                        height: 50.0,
                        stamp_type: self.selected_stamp_type.clone(),
                    });
                    self.placing_stamp = false;
                }
            }

            // テキスト配置モード
            if self.placing_text && response.clicked() {
                if let Some(pos) = response.interact_pointer_pos() {
                    let pdf_x = (pos.x - rect.min.x) / self.zoom;
                    let pdf_y = (pos.y - rect.min.y) / self.zoom;

                    result.new_text = Some(TextAnnotation {
                        page: page_index,
                        x: pdf_x,
                        y: pdf_y,
                        text: self.text_input.clone(),
                        font_size: self.text_font_size,
                    });
                    self.placing_text = false;
                    self.text_input.clear();
                }
            }

            // カーソル表示
            if self.placing_stamp || self.placing_text {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
            }
        } else {
            ui.spinner();
            ui.label("読み込み中...");
        }

        result
    }

    /// ページキャッシュを無効化
    fn invalidate_page_cache(&mut self) {
        self.page_texture = None;
    }

    /// 外部からキャッシュを無効化
    pub fn invalidate_cache(&mut self) {
        self.invalidate_page_cache();
    }
}
