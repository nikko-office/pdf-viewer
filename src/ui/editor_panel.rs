//! メイン編集パネル - PDF表示、スタンプ配置、テキスト入力

use crate::pdf::{PdfDocument, Stamp, StampType, TextAnnotation};
use eframe::egui::{self, Color32, TextureHandle, Vec2};

/// エディター操作の結果
#[derive(Default)]
pub struct EditorResult {
    pub new_stamp: Option<Stamp>,
    pub new_text: Option<TextAnnotation>,
    pub delete_stamp: Option<usize>,
    pub delete_text: Option<usize>,
    pub move_stamp: Option<(usize, f32, f32)>,
    pub move_text: Option<(usize, f32, f32)>,
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
    selected_custom_stamp_index: Option<usize>,
    placing_stamp: bool,

    // テキスト入力
    text_input: String,
    text_font_size: f32,
    placing_text: bool,

    // 選択・ドラッグ
    selected_stamp_index: Option<usize>,
    selected_text_index: Option<usize>,
    dragging: bool,
    drag_offset: Vec2,
}

impl EditorPanel {
    pub fn new() -> Self {
        Self {
            page_texture: None,
            current_page_index: None,
            zoom: 1.0,
            selected_stamp_type: StampType::Approved,
            selected_custom_stamp_index: None,
            placing_stamp: false,
            text_input: String::new(),
            text_font_size: 14.0,
            placing_text: false,
            selected_stamp_index: None,
            selected_text_index: None,
            dragging: false,
            drag_offset: Vec2::ZERO,
        }
    }

    /// カスタムスタンプ付きでUIを描画
    pub fn show_with_custom_stamps(
        &mut self,
        ui: &mut egui::Ui,
        doc: &PdfDocument,
        page_index: usize,
        stamps: &[Stamp],
        text_annotations: &[TextAnnotation],
        show_stamp_panel: bool,
        show_text_panel: bool,
        custom_stamps: &[(String, Option<TextureHandle>)],
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
            
            ui.separator();
            
            // 選択中のアイテム情報と削除ボタン
            if let Some(idx) = self.selected_stamp_index {
                ui.label(format!("スタンプ#{} 選択中", idx + 1));
                if ui.button("🗑 削除").clicked() {
                    result.delete_stamp = Some(idx);
                    self.selected_stamp_index = None;
                }
                if ui.button("✕ 選択解除").clicked() {
                    self.selected_stamp_index = None;
                }
            } else if let Some(idx) = self.selected_text_index {
                ui.label(format!("テキスト#{} 選択中", idx + 1));
                if ui.button("🗑 削除").clicked() {
                    result.delete_text = Some(idx);
                    self.selected_text_index = None;
                }
                if ui.button("✕ 選択解除").clicked() {
                    self.selected_text_index = None;
                }
            }
        });

        // スタンプパネル
        if show_stamp_panel {
            ui.separator();
            ui.horizontal_wrapped(|ui| {
                ui.label("スタンプ:");
                
                let stamp_types = [
                    (StampType::Approved, "✅承認"),
                    (StampType::Rejected, "❌却下"),
                    (StampType::Draft, "📝下書"),
                    (StampType::Confidential, "🔒機密"),
                ];

                for (stamp_type, label) in &stamp_types {
                    let selected = self.selected_custom_stamp_index.is_none() 
                        && self.selected_stamp_type == *stamp_type;
                    if ui.selectable_label(selected, *label).clicked() {
                        self.selected_stamp_type = stamp_type.clone();
                        self.selected_custom_stamp_index = None;
                    }
                }

                for (i, (name, tex)) in custom_stamps.iter().enumerate() {
                    let selected = self.selected_custom_stamp_index == Some(i);
                    let response = ui.selectable_label(selected, format!("🖼{}", name));
                    
                    if let Some(texture) = tex {
                        response.clone().on_hover_ui(|ui| {
                            ui.image((texture.id(), Vec2::new(100.0, 100.0)));
                        });
                    }
                    
                    if response.clicked() {
                        self.selected_custom_stamp_index = Some(i);
                        self.selected_stamp_type = StampType::Custom(name.clone());
                    }
                }

                ui.separator();

                let btn_text = if self.placing_stamp { "🎯配置中" } else { "配置" };
                let btn_color = if self.placing_stamp { Color32::from_rgb(100, 200, 100) } else { Color32::GRAY };
                if ui.add(egui::Button::new(btn_text).fill(btn_color)).clicked() {
                    self.placing_stamp = !self.placing_stamp;
                    self.placing_text = false;
                    self.selected_stamp_index = None;
                    self.selected_text_index = None;
                }
            });
        }

        // テキストパネル
        if show_text_panel {
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("テキスト:");
                ui.add(egui::TextEdit::singleline(&mut self.text_input).desired_width(150.0));
                ui.label("サイズ:");
                ui.add(egui::DragValue::new(&mut self.text_font_size).range(8.0..=72.0));

                let btn_text = if self.placing_text { "🎯配置中" } else { "配置" };
                let btn_color = if self.placing_text { Color32::from_rgb(100, 200, 100) } else { Color32::GRAY };
                if ui.add(egui::Button::new(btn_text).fill(btn_color)).clicked() && !self.text_input.is_empty() {
                    self.placing_text = !self.placing_text;
                    self.placing_stamp = false;
                    self.selected_stamp_index = None;
                    self.selected_text_index = None;
                }
            });
        }

        ui.separator();
        ui.label("💡 ヒント: クリックで選択、ドラッグで移動、選択後に削除ボタンで削除");
        ui.separator();

        // ページテクスチャを更新
        if self.current_page_index != Some(page_index) {
            self.current_page_index = Some(page_index);
            self.page_texture = None;
            self.selected_stamp_index = None;
            self.selected_text_index = None;
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
            let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());

            // ページ画像描画
            ui.painter().image(
                texture.id(),
                rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );

            // 現在のページのスタンプをフィルタ
            let page_stamps: Vec<(usize, &Stamp)> = stamps
                .iter()
                .enumerate()
                .filter(|(_, s)| s.page == page_index)
                .collect();

            // 既存のスタンプを描画
            for (global_idx, stamp) in &page_stamps {
                let stamp_pos = egui::pos2(
                    rect.min.x + stamp.x * self.zoom,
                    rect.min.y + stamp.y * self.zoom,
                );
                let stamp_size = Vec2::new(stamp.width * self.zoom, stamp.height * self.zoom);
                let stamp_rect = egui::Rect::from_min_size(stamp_pos, stamp_size);

                let is_selected = self.selected_stamp_index == Some(*global_idx);

                // カスタムスタンプの場合
                if let StampType::Custom(ref name) = stamp.stamp_type {
                    if let Some((_, Some(tex))) = custom_stamps.iter().find(|(n, _)| n == name) {
                        ui.painter().image(
                            tex.id(),
                            stamp_rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            Color32::WHITE,
                        );
                    }
                } else {
                    // 組み込みスタンプ
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

                // 選択枠
                if is_selected {
                    ui.painter().rect_stroke(
                        stamp_rect.expand(3.0),
                        4.0,
                        egui::Stroke::new(3.0, Color32::YELLOW),
                    );
                }
            }

            // 現在のページのテキストをフィルタ
            let page_texts: Vec<(usize, &TextAnnotation)> = text_annotations
                .iter()
                .enumerate()
                .filter(|(_, t)| t.page == page_index)
                .collect();

            // 既存のテキスト注釈を描画
            for (global_idx, annotation) in &page_texts {
                let text_pos = egui::pos2(
                    rect.min.x + annotation.x * self.zoom,
                    rect.min.y + annotation.y * self.zoom,
                );
                
                let is_selected = self.selected_text_index == Some(*global_idx);
                
                // テキストサイズを計算（おおよそ）
                let font = egui::FontId::proportional(annotation.font_size * self.zoom);
                let galley = ui.painter().layout_no_wrap(
                    annotation.text.clone(),
                    font.clone(),
                    Color32::BLACK,
                );
                let text_rect = egui::Rect::from_min_size(text_pos, galley.size());

                // 選択時は背景を表示
                if is_selected {
                    ui.painter().rect_filled(
                        text_rect.expand(2.0),
                        2.0,
                        Color32::from_rgba_unmultiplied(255, 255, 0, 100),
                    );
                    ui.painter().rect_stroke(
                        text_rect.expand(2.0),
                        2.0,
                        egui::Stroke::new(2.0, Color32::YELLOW),
                    );
                }

                ui.painter().galley(text_pos, galley, Color32::BLACK);
            }

            // クリック・ドラッグ処理
            if !self.placing_stamp && !self.placing_text {
                // スタンプ選択チェック
                if response.clicked() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        let mut found = false;
                        
                        // スタンプをクリックしたか
                        for (global_idx, stamp) in page_stamps.iter().rev() {
                            let stamp_rect = egui::Rect::from_min_size(
                                egui::pos2(rect.min.x + stamp.x * self.zoom, rect.min.y + stamp.y * self.zoom),
                                Vec2::new(stamp.width * self.zoom, stamp.height * self.zoom),
                            );
                            if stamp_rect.contains(pos) {
                                self.selected_stamp_index = Some(*global_idx);
                                self.selected_text_index = None;
                                found = true;
                                break;
                            }
                        }
                        
                        // テキストをクリックしたか
                        if !found {
                            for (global_idx, annotation) in page_texts.iter().rev() {
                                let text_pos = egui::pos2(
                                    rect.min.x + annotation.x * self.zoom,
                                    rect.min.y + annotation.y * self.zoom,
                                );
                                let font = egui::FontId::proportional(annotation.font_size * self.zoom);
                                let galley = ui.painter().layout_no_wrap(
                                    annotation.text.clone(),
                                    font,
                                    Color32::BLACK,
                                );
                                let text_rect = egui::Rect::from_min_size(text_pos, galley.size());
                                
                                if text_rect.contains(pos) {
                                    self.selected_text_index = Some(*global_idx);
                                    self.selected_stamp_index = None;
                                    found = true;
                                    break;
                                }
                            }
                        }
                        
                        if !found {
                            self.selected_stamp_index = None;
                            self.selected_text_index = None;
                        }
                    }
                }

                // ドラッグ開始
                if response.drag_started() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        if let Some(idx) = self.selected_stamp_index {
                            if let Some(stamp) = stamps.get(idx) {
                                let stamp_pos = egui::pos2(
                                    rect.min.x + stamp.x * self.zoom,
                                    rect.min.y + stamp.y * self.zoom,
                                );
                                self.drag_offset = Vec2::new(pos.x - stamp_pos.x, pos.y - stamp_pos.y);
                                self.dragging = true;
                            }
                        } else if let Some(idx) = self.selected_text_index {
                            if let Some(annotation) = text_annotations.get(idx) {
                                let text_pos = egui::pos2(
                                    rect.min.x + annotation.x * self.zoom,
                                    rect.min.y + annotation.y * self.zoom,
                                );
                                self.drag_offset = Vec2::new(pos.x - text_pos.x, pos.y - text_pos.y);
                                self.dragging = true;
                            }
                        }
                    }
                }

                // ドラッグ中
                if response.dragged() && self.dragging {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                }

                // ドラッグ終了
                if response.drag_stopped() && self.dragging {
                    if let Some(pos) = ui.input(|i| i.pointer.hover_pos()) {
                        let new_x = (pos.x - rect.min.x - self.drag_offset.x) / self.zoom;
                        let new_y = (pos.y - rect.min.y - self.drag_offset.y) / self.zoom;
                        
                        if let Some(idx) = self.selected_stamp_index {
                            result.move_stamp = Some((idx, new_x, new_y));
                        } else if let Some(idx) = self.selected_text_index {
                            result.move_text = Some((idx, new_x, new_y));
                        }
                    }
                    self.dragging = false;
                }
            }

            // スタンプ配置モード
            if self.placing_stamp {
                if let Some(hover_pos) = ui.input(|i| i.pointer.hover_pos()) {
                    if rect.contains(hover_pos) {
                        let preview_size = Vec2::new(100.0 * self.zoom, 50.0 * self.zoom);
                        let preview_rect = egui::Rect::from_center_size(hover_pos, preview_size);
                        
                        if let Some(idx) = self.selected_custom_stamp_index {
                            if let Some((_, Some(tex))) = custom_stamps.get(idx) {
                                ui.painter().image(
                                    tex.id(),
                                    preview_rect,
                                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                                    Color32::from_rgba_unmultiplied(255, 255, 255, 150),
                                );
                            }
                        } else {
                            let (bg_color, border_color) = match &self.selected_stamp_type {
                                StampType::Approved => (Color32::from_rgba_unmultiplied(200, 255, 200, 100), Color32::from_rgba_unmultiplied(0, 200, 0, 150)),
                                StampType::Rejected => (Color32::from_rgba_unmultiplied(255, 200, 200, 100), Color32::from_rgba_unmultiplied(200, 0, 0, 150)),
                                StampType::Draft => (Color32::from_rgba_unmultiplied(255, 255, 200, 100), Color32::from_rgba_unmultiplied(200, 150, 0, 150)),
                                StampType::Confidential => (Color32::from_rgba_unmultiplied(200, 200, 255, 100), Color32::from_rgba_unmultiplied(0, 0, 200, 150)),
                                StampType::Custom(_) => (Color32::from_rgba_unmultiplied(220, 220, 220, 100), Color32::from_rgba_unmultiplied(128, 128, 128, 150)),
                            };

                            ui.painter().rect_filled(preview_rect, 4.0, bg_color);
                            ui.painter().rect_stroke(preview_rect, 4.0, egui::Stroke::new(2.0, border_color));

                            ui.painter().text(
                                preview_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                self.selected_stamp_type.label(),
                                egui::FontId::proportional(12.0 * self.zoom),
                                border_color,
                            );
                        }
                        
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
                    }
                }

                if response.clicked() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        let pdf_x = (pos.x - rect.min.x) / self.zoom - 50.0;
                        let pdf_y = (pos.y - rect.min.y) / self.zoom - 25.0;

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
            }

            // テキスト配置モード
            if self.placing_text {
                if let Some(hover_pos) = ui.input(|i| i.pointer.hover_pos()) {
                    if rect.contains(hover_pos) {
                        ui.painter().text(
                            hover_pos,
                            egui::Align2::LEFT_TOP,
                            &self.text_input,
                            egui::FontId::proportional(self.text_font_size * self.zoom),
                            Color32::from_rgba_unmultiplied(0, 0, 0, 150),
                        );
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
                    }
                }

                if response.clicked() {
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
            }

            // Deleteキーで削除
            if ui.input(|i| i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace)) {
                if let Some(idx) = self.selected_stamp_index {
                    result.delete_stamp = Some(idx);
                    self.selected_stamp_index = None;
                } else if let Some(idx) = self.selected_text_index {
                    result.delete_text = Some(idx);
                    self.selected_text_index = None;
                }
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
