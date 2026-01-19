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
    pub delete_custom_stamp: Option<usize>,
    pub register_stamp_clicked: bool,
}

/// エディターパネルの状態
pub struct EditorPanel {
    // ページテクスチャのキャッシュ
    page_texture: Option<TextureHandle>,
    current_page_index: Option<usize>,
    cached_rotation: i32,

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
            cached_rotation: 0,
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

    /// PDF座標から表示座標に変換（回転考慮、サイズは維持）
    fn pdf_to_display_pos(
        &self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        orig_w: f32,
        orig_h: f32,
        rotation: i32,
    ) -> (f32, f32) {
        match rotation {
            90 => {
                // 90度回転: (x, y) → (orig_h - y - height, x)
                let new_x = orig_h - y - height;
                let new_y = x;
                (new_x, new_y)
            }
            180 => {
                // 180度回転: (x, y) → (orig_w - x - width, orig_h - y - height)
                let new_x = orig_w - x - width;
                let new_y = orig_h - y - height;
                (new_x, new_y)
            }
            270 => {
                // 270度回転: (x, y) → (y, orig_w - x - width)
                let new_x = y;
                let new_y = orig_w - x - width;
                (new_x, new_y)
            }
            _ => {
                // 0度: そのまま
                (x, y)
            }
        }
    }

    /// 表示座標からPDF座標に変換（回転考慮）
    fn display_to_pdf(
        &self,
        display_x: f32,
        display_y: f32,
        width: f32,
        height: f32,
        orig_w: f32,
        orig_h: f32,
        rotation: i32,
    ) -> (f32, f32) {
        match rotation {
            90 => {
                let pdf_x = display_y;
                let pdf_y = orig_h - display_x - height;
                (pdf_x, pdf_y)
            }
            180 => {
                let pdf_x = orig_w - display_x - width;
                let pdf_y = orig_h - display_y - height;
                (pdf_x, pdf_y)
            }
            270 => {
                let pdf_x = orig_w - display_y - width;
                let pdf_y = display_x;
                (pdf_x, pdf_y)
            }
            _ => {
                (display_x, display_y)
            }
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

        // 回転情報を取得
        let rotation = doc.get_page_rotation(page_index);
        let orig_size = doc.original_page_size(page_index);
        let (orig_w, orig_h) = orig_size;

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
            ui.label(format!("回転: {}°", rotation));
            
            ui.separator();
            
            // 選択中のアイテム情報と削除ボタン
            if let Some(idx) = self.selected_stamp_index {
                ui.label(format!("スタンプ#{} 選択中", idx + 1));
                if ui.button("🗑 削除").clicked() {
                    result.delete_stamp = Some(idx);
                    self.selected_stamp_index = None;
                }
                if ui.button("✕").clicked() {
                    self.selected_stamp_index = None;
                }
            } else if let Some(idx) = self.selected_text_index {
                ui.label(format!("テキスト#{} 選択中", idx + 1));
                if ui.button("🗑 削除").clicked() {
                    result.delete_text = Some(idx);
                    self.selected_text_index = None;
                }
                if ui.button("✕").clicked() {
                    self.selected_text_index = None;
                }
            }
        });

        // スタンプパネル（サムネイル表示）
        if show_stamp_panel {
            ui.separator();
            
            // 配置ボタン
            ui.horizontal(|ui| {
                let btn_text = if self.placing_stamp { "🎯 配置中（クリックで解除）" } else { "📍 スタンプを配置" };
                let btn_color = if self.placing_stamp { Color32::from_rgb(100, 200, 100) } else { Color32::from_rgb(80, 80, 80) };
                if ui.add(egui::Button::new(btn_text).fill(btn_color)).clicked() {
                    self.placing_stamp = !self.placing_stamp;
                    self.placing_text = false;
                    self.selected_stamp_index = None;
                    self.selected_text_index = None;
                }
                
                ui.separator();
                
                if ui.button("➕ スタンプ登録").clicked() {
                    result.register_stamp_clicked = true;
                }
            });
            
            ui.add_space(4.0);
            
            // スタンプサムネイルグリッド
            egui::ScrollArea::horizontal().show(ui, |ui| {
                ui.horizontal(|ui| {
                    let thumb_size = 50.0;
                    
                    // 組み込みスタンプ
                    let stamp_types = [
                        (StampType::Approved, "✅", "承認", Color32::GREEN),
                        (StampType::Rejected, "❌", "却下", Color32::RED),
                        (StampType::Draft, "📝", "下書", Color32::from_rgb(200, 150, 0)),
                        (StampType::Confidential, "🔒", "機密", Color32::BLUE),
                    ];

                    for (stamp_type, icon, label, color) in &stamp_types {
                        let selected = self.selected_custom_stamp_index.is_none() 
                            && self.selected_stamp_type == *stamp_type;
                        
                        let frame_color = if selected { Color32::YELLOW } else { Color32::from_gray(60) };
                        
                        egui::Frame::none()
                            .fill(Color32::from_gray(40))
                            .stroke(egui::Stroke::new(if selected { 3.0 } else { 1.0 }, frame_color))
                            .rounding(4.0)
                            .inner_margin(4.0)
                            .show(ui, |ui| {
                                ui.set_width(thumb_size);
                                ui.set_height(thumb_size + 16.0);
                                
                                let response = ui.vertical_centered(|ui| {
                                    ui.label(egui::RichText::new(*icon).size(24.0));
                                    ui.label(egui::RichText::new(*label).size(10.0).color(*color));
                                });
                                
                                if response.response.clicked() {
                                    self.selected_stamp_type = stamp_type.clone();
                                    self.selected_custom_stamp_index = None;
                                }
                            });
                    }

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);

                    // カスタムスタンプ
                    for (i, (name, tex)) in custom_stamps.iter().enumerate() {
                        let selected = self.selected_custom_stamp_index == Some(i);
                        let frame_color = if selected { Color32::YELLOW } else { Color32::from_gray(60) };
                        
                        egui::Frame::none()
                            .fill(Color32::from_gray(40))
                            .stroke(egui::Stroke::new(if selected { 3.0 } else { 1.0 }, frame_color))
                            .rounding(4.0)
                            .inner_margin(4.0)
                            .show(ui, |ui| {
                                ui.set_width(thumb_size);
                                ui.set_height(thumb_size + 16.0);
                                
                                ui.vertical_centered(|ui| {
                                    if let Some(texture) = tex {
                                        ui.image((texture.id(), Vec2::new(thumb_size - 8.0, thumb_size - 8.0)));
                                    } else {
                                        ui.label(egui::RichText::new("🖼").size(24.0));
                                    }
                                    
                                    // 短い名前表示
                                    let short_name: String = name.chars().take(6).collect();
                                    ui.label(egui::RichText::new(&short_name).size(9.0));
                                });
                            })
                            .response
                            .context_menu(|ui| {
                                if ui.button("🗑 削除").clicked() {
                                    result.delete_custom_stamp = Some(i);
                                    if self.selected_custom_stamp_index == Some(i) {
                                        self.selected_custom_stamp_index = None;
                                        self.selected_stamp_type = StampType::Approved;
                                    }
                                    ui.close_menu();
                                }
                            });
                        
                        // クリックで選択
                        let last_response = ui.interact(
                            ui.min_rect(),
                            egui::Id::new(format!("custom_stamp_{}", i)),
                            egui::Sense::click(),
                        );
                        if last_response.clicked() {
                            self.selected_custom_stamp_index = Some(i);
                            self.selected_stamp_type = StampType::Custom(name.clone());
                        }
                    }
                });
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

        // ページテクスチャを更新（ページ変更または回転変更時）
        if self.current_page_index != Some(page_index) || self.cached_rotation != rotation {
            self.current_page_index = Some(page_index);
            self.cached_rotation = rotation;
            self.page_texture = None;
            self.selected_stamp_index = None;
            self.selected_text_index = None;
        }

        // ページサイズ計算（回転後）
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

            // 既存のスタンプを描画（回転変換を適用、サイズは維持）
            for (global_idx, stamp) in &page_stamps {
                // PDF座標から表示座標に変換（位置のみ、サイズは維持）
                let (display_x, display_y) = self.pdf_to_display_pos(
                    stamp.x, stamp.y, stamp.width, stamp.height,
                    orig_w, orig_h, rotation
                );

                let stamp_pos = egui::pos2(
                    rect.min.x + display_x * self.zoom,
                    rect.min.y + display_y * self.zoom,
                );
                // サイズは元のまま維持
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

            // 既存のテキスト注釈を描画（回転変換を適用）
            for (global_idx, annotation) in &page_texts {
                let text_width = annotation.text.len() as f32 * annotation.font_size * 0.6;
                let text_height = annotation.font_size;

                let (display_x, display_y) = self.pdf_to_display_pos(
                    annotation.x, annotation.y, text_width, text_height,
                    orig_w, orig_h, rotation
                );

                let text_pos = egui::pos2(
                    rect.min.x + display_x * self.zoom,
                    rect.min.y + display_y * self.zoom,
                );
                
                let is_selected = self.selected_text_index == Some(*global_idx);
                
                let font = egui::FontId::proportional(annotation.font_size * self.zoom);
                let galley = ui.painter().layout_no_wrap(
                    annotation.text.clone(),
                    font.clone(),
                    Color32::BLACK,
                );
                let text_rect = egui::Rect::from_min_size(text_pos, galley.size());

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
                if response.clicked() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        let mut found = false;
                        
                        for (global_idx, stamp) in page_stamps.iter().rev() {
                            let (display_x, display_y) = self.pdf_to_display_pos(
                                stamp.x, stamp.y, stamp.width, stamp.height,
                                orig_w, orig_h, rotation
                            );
                            let stamp_rect = egui::Rect::from_min_size(
                                egui::pos2(rect.min.x + display_x * self.zoom, rect.min.y + display_y * self.zoom),
                                Vec2::new(stamp.width * self.zoom, stamp.height * self.zoom),
                            );
                            if stamp_rect.contains(pos) {
                                self.selected_stamp_index = Some(*global_idx);
                                self.selected_text_index = None;
                                found = true;
                                break;
                            }
                        }
                        
                        if !found {
                            for (global_idx, annotation) in page_texts.iter().rev() {
                                let text_width = annotation.text.len() as f32 * annotation.font_size * 0.6;
                                let text_height = annotation.font_size;
                                let (display_x, display_y) = self.pdf_to_display_pos(
                                    annotation.x, annotation.y, text_width, text_height,
                                    orig_w, orig_h, rotation
                                );
                                let text_pos = egui::pos2(
                                    rect.min.x + display_x * self.zoom,
                                    rect.min.y + display_y * self.zoom,
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
                                let (display_x, display_y) = self.pdf_to_display_pos(
                                    stamp.x, stamp.y, stamp.width, stamp.height,
                                    orig_w, orig_h, rotation
                                );
                                let stamp_pos = egui::pos2(
                                    rect.min.x + display_x * self.zoom,
                                    rect.min.y + display_y * self.zoom,
                                );
                                self.drag_offset = Vec2::new(pos.x - stamp_pos.x, pos.y - stamp_pos.y);
                                self.dragging = true;
                            }
                        } else if let Some(idx) = self.selected_text_index {
                            if let Some(annotation) = text_annotations.get(idx) {
                                let text_width = annotation.text.len() as f32 * annotation.font_size * 0.6;
                                let text_height = annotation.font_size;
                                let (display_x, display_y) = self.pdf_to_display_pos(
                                    annotation.x, annotation.y, text_width, text_height,
                                    orig_w, orig_h, rotation
                                );
                                let text_pos = egui::pos2(
                                    rect.min.x + display_x * self.zoom,
                                    rect.min.y + display_y * self.zoom,
                                );
                                self.drag_offset = Vec2::new(pos.x - text_pos.x, pos.y - text_pos.y);
                                self.dragging = true;
                            }
                        }
                    }
                }

                if response.dragged() && self.dragging {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                }

                // ドラッグ終了
                if response.drag_stopped() && self.dragging {
                    if let Some(pos) = ui.input(|i| i.pointer.hover_pos()) {
                        let display_x = (pos.x - rect.min.x - self.drag_offset.x) / self.zoom;
                        let display_y = (pos.y - rect.min.y - self.drag_offset.y) / self.zoom;
                        
                        if let Some(idx) = self.selected_stamp_index {
                            if let Some(stamp) = stamps.get(idx) {
                                let (pdf_x, pdf_y) = self.display_to_pdf(
                                    display_x, display_y, stamp.width, stamp.height,
                                    orig_w, orig_h, rotation
                                );
                                result.move_stamp = Some((idx, pdf_x, pdf_y));
                            }
                        } else if let Some(idx) = self.selected_text_index {
                            if let Some(annotation) = text_annotations.get(idx) {
                                let text_width = annotation.text.len() as f32 * annotation.font_size * 0.6;
                                let text_height = annotation.font_size;
                                let (pdf_x, pdf_y) = self.display_to_pdf(
                                    display_x, display_y, text_width, text_height,
                                    orig_w, orig_h, rotation
                                );
                                result.move_text = Some((idx, pdf_x, pdf_y));
                            }
                        }
                    }
                    self.dragging = false;
                }
            }

            // スタンプ配置モード
            if self.placing_stamp {
                if let Some(hover_pos) = ui.input(|i| i.pointer.hover_pos()) {
                    if rect.contains(hover_pos) {
                        let preview_w = 100.0 * self.zoom;
                        let preview_h = 50.0 * self.zoom;
                        let preview_rect = egui::Rect::from_center_size(hover_pos, Vec2::new(preview_w, preview_h));
                        
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
                        let display_x = (pos.x - rect.min.x) / self.zoom - 50.0;
                        let display_y = (pos.y - rect.min.y) / self.zoom - 25.0;

                        let (pdf_x, pdf_y) = self.display_to_pdf(
                            display_x, display_y, 100.0, 50.0,
                            orig_w, orig_h, rotation
                        );

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
                        let display_x = (pos.x - rect.min.x) / self.zoom;
                        let display_y = (pos.y - rect.min.y) / self.zoom;

                        let text_width = self.text_input.len() as f32 * self.text_font_size * 0.6;
                        let text_height = self.text_font_size;

                        let (pdf_x, pdf_y) = self.display_to_pdf(
                            display_x, display_y, text_width, text_height,
                            orig_w, orig_h, rotation
                        );

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

    fn invalidate_page_cache(&mut self) {
        self.page_texture = None;
    }

    pub fn invalidate_cache(&mut self) {
        self.invalidate_page_cache();
    }
}
