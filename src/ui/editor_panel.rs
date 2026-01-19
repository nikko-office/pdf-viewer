//! メイン編集パネル - PDF表示、スタンプ配置、テキスト入力

use crate::pdf::{FontType, PdfDocument, RectAnnotation, Stamp, StampType, TextAnnotation};
use eframe::egui::{self, Color32, TextureHandle, Vec2};

/// リサイズのコーナー
#[derive(Clone, Copy, PartialEq, Default)]
pub enum ResizeCorner {
    #[default]
    None,
    BottomRight,
}

/// エディター操作の結果
#[derive(Default)]
pub struct EditorResult {
    pub new_stamp: Option<Stamp>,
    pub new_text: Option<TextAnnotation>,
    pub new_rect: Option<RectAnnotation>,
    pub delete_stamp: Option<usize>,
    pub delete_text: Option<usize>,
    pub delete_rect: Option<usize>,
    pub move_stamp: Option<(usize, f32, f32)>,
    pub move_text: Option<(usize, f32, f32)>,
    pub move_rect: Option<(usize, f32, f32)>,
    pub resize_stamp: Option<(usize, f32, f32)>,  // (index, new_width, new_height)
    pub resize_text: Option<(usize, f32)>,  // (index, new_font_size)
    pub resize_rect: Option<(usize, f32, f32)>,  // (index, new_width, new_height)
    pub edit_text: Option<(usize, String, FontType, bool)>,  // (index, new_text, font_type, transparent)
    pub delete_custom_stamp: Option<usize>,
    pub register_stamp_clicked: bool,
}

/// エディターパネルの状態
pub struct EditorPanel {
    // ページテクスチャのキャッシュ
    page_texture: Option<TextureHandle>,
    current_page_index: Option<usize>,
    cached_rotation: i32,
    cached_base_size: (u32, u32),  // レンダリング時の基本サイズ

    // ズーム
    zoom: f32,

    // スタンプ配置モード
    selected_stamp_type: StampType,
    selected_custom_stamp_index: Option<usize>,
    placing_stamp: bool,

    // テキスト入力
    text_input: String,
    text_font_size: f32,
    text_font_type: FontType,
    text_transparent: bool,
    placing_text: bool,
    editing_text: bool,  // テキスト編集モード

    // 矩形配置
    placing_rect: bool,
    rect_start_pos: Option<egui::Pos2>,  // ドラッグ開始位置

    // 選択・ドラッグ
    selected_stamp_index: Option<usize>,
    selected_text_index: Option<usize>,
    selected_rect_index: Option<usize>,
    dragging: bool,
    drag_offset: Vec2,

    // リサイズ
    resizing: bool,
    resize_corner: ResizeCorner,
    resize_start_size: Vec2,
}

impl EditorPanel {
    pub fn new() -> Self {
        Self {
            page_texture: None,
            current_page_index: None,
            cached_rotation: 0,
            cached_base_size: (0, 0),
            zoom: 1.0,
            selected_stamp_type: StampType::Approved,
            selected_custom_stamp_index: None,
            placing_stamp: false,
            text_input: String::new(),
            text_font_size: 24.0,
            text_font_type: FontType::Gothic,
            text_transparent: true,
            placing_text: false,
            editing_text: false,
            placing_rect: false,
            rect_start_pos: None,
            selected_stamp_index: None,
            selected_text_index: None,
            selected_rect_index: None,
            dragging: false,
            drag_offset: Vec2::ZERO,
            resizing: false,
            resize_corner: ResizeCorner::None,
            resize_start_size: Vec2::ZERO,
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
    /// custom_stamps: (名前, テクスチャ, 幅, 高さ)
    pub fn show_with_custom_stamps(
        &mut self,
        ui: &mut egui::Ui,
        doc: &PdfDocument,
        page_index: usize,
        stamps: &[Stamp],
        text_annotations: &[TextAnnotation],
        rect_annotations: &[RectAnnotation],
        show_stamp_panel: bool,
        show_text_panel: bool,
        custom_stamps: &[(String, Option<TextureHandle>, u32, u32)],
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
                // ズームは表示スケーリングで対応、キャッシュ無効化不要
            }
            ui.label(format!("{:.0}%", self.zoom * 100.0));
            if ui.button("＋").clicked() {
                self.zoom = (self.zoom + 0.25).min(4.0);
                // ズームは表示スケーリングで対応、キャッシュ無効化不要
            }
            if ui.button("リセット").clicked() {
                self.zoom = 1.0;
                // ズームは表示スケーリングで対応、キャッシュ無効化不要
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
            } else if let Some(idx) = self.selected_rect_index {
                ui.label(format!("矩形#{} 選択中", idx + 1));
                if ui.button("🗑 削除").clicked() {
                    result.delete_rect = Some(idx);
                    self.selected_rect_index = None;
                }
                if ui.button("✕").clicked() {
                    self.selected_rect_index = None;
                }
            }
            
            ui.separator();
            
            // 矩形（白塗り）配置ボタン
            let rect_btn_text = if self.placing_rect { "🎯 矩形配置中（ドラッグで描画）" } else { "⬜ 白塗り矩形" };
            let rect_btn_color = if self.placing_rect { 
                Color32::from_rgb(50, 180, 80)
            } else { 
                Color32::from_rgb(180, 180, 180)
            };
            if ui.add(egui::Button::new(egui::RichText::new(rect_btn_text).color(Color32::BLACK)).fill(rect_btn_color)).clicked() {
                self.placing_rect = !self.placing_rect;
                self.placing_stamp = false;
                self.placing_text = false;
                self.editing_text = false;
                self.selected_stamp_index = None;
                self.selected_text_index = None;
                self.selected_rect_index = None;
                self.rect_start_pos = None;
            }
        });

        // スタンプパネル（サムネイル表示）
        if show_stamp_panel {
            ui.separator();
            
            // 配置ボタン
            ui.horizontal(|ui| {
                let btn_text = if self.placing_stamp { "🎯 配置中（クリックで解除）" } else { "📍 スタンプを配置" };
                let btn_color = if self.placing_stamp { 
                    Color32::from_rgb(50, 180, 80)  // 配置中は明るい緑
                } else { 
                    Color32::from_rgb(60, 120, 200)  // 通常は明るい青
                };
                let text_color = Color32::WHITE;
                if ui.add(egui::Button::new(egui::RichText::new(btn_text).color(text_color)).fill(btn_color)).clicked() {
                    self.placing_stamp = !self.placing_stamp;
                    self.placing_text = false;
                    self.selected_stamp_index = None;
                    self.selected_text_index = None;
                }
                
                ui.separator();
                
                if ui.add(egui::Button::new(egui::RichText::new("➕ スタンプ登録").color(Color32::WHITE)).fill(Color32::from_rgb(100, 80, 160))).clicked() {
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
                    for (i, (name, tex, w, h)) in custom_stamps.iter().enumerate() {
                        let selected = self.selected_custom_stamp_index == Some(i);
                        let frame_color = if selected { Color32::YELLOW } else { Color32::from_gray(60) };
                        
                        // サムネイルサイズを比率維持で計算
                        let aspect = *w as f32 / (*h as f32).max(1.0);
                        let (thumb_w, thumb_h) = if aspect > 1.0 {
                            (thumb_size - 8.0, (thumb_size - 8.0) / aspect)
                        } else {
                            ((thumb_size - 8.0) * aspect, thumb_size - 8.0)
                        };
                        
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
                                        ui.image((texture.id(), Vec2::new(thumb_w, thumb_h)));
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
            
            // テキスト編集モード（選択中のテキストを編集）
            if self.editing_text {
                if let Some(idx) = self.selected_text_index {
                    ui.horizontal(|ui| {
                        ui.colored_label(Color32::YELLOW, format!("📝 テキスト#{} 編集中", idx + 1));
                        if ui.button("✓ 確定").clicked() {
                            result.edit_text = Some((idx, self.text_input.clone(), self.text_font_type, self.text_transparent));
                            self.editing_text = false;
                            self.text_input.clear();
                        }
                        if ui.button("✕ キャンセル").clicked() {
                            self.editing_text = false;
                            self.text_input.clear();
                        }
                    });
                }
            }
            
            // テキスト入力欄（複数行対応）
            ui.horizontal(|ui| {
                ui.label("テキスト:");
                ui.add(
                    egui::TextEdit::multiline(&mut self.text_input)
                        .desired_width(200.0)
                        .desired_rows(2)
                        .hint_text("テキストを入力（改行可）")
                );
            });
            
            // 設定行
            ui.horizontal(|ui| {
                // フォントサイズ
                ui.label("サイズ:");
                ui.add(egui::DragValue::new(&mut self.text_font_size).range(8.0..=72.0).speed(1.0));
                
                ui.separator();
                
                // フォントタイプ
                ui.label("フォント:");
                egui::ComboBox::from_id_salt("font_type")
                    .selected_text(self.text_font_type.label())
                    .width(70.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.text_font_type, FontType::Gothic, "ゴシック");
                        ui.selectable_value(&mut self.text_font_type, FontType::Mincho, "明朝");
                    });
                
                ui.separator();
                
                // 透過設定
                ui.checkbox(&mut self.text_transparent, "透過");
            });
            
            // 操作ボタン
            ui.horizontal(|ui| {
                let btn_text = if self.placing_text { "🎯 配置中（クリックで解除）" } else { "📍 テキストを配置" };
                let btn_color = if self.placing_text { 
                    Color32::from_rgb(50, 180, 80)
                } else { 
                    Color32::from_rgb(60, 120, 200)
                };
                if ui.add(egui::Button::new(egui::RichText::new(btn_text).color(Color32::WHITE)).fill(btn_color)).clicked() && !self.text_input.is_empty() {
                    self.placing_text = !self.placing_text;
                    self.placing_stamp = false;
                    self.editing_text = false;
                    self.selected_stamp_index = None;
                    self.selected_text_index = None;
                }
                
                // 選択中のテキストを編集
                if let Some(idx) = self.selected_text_index {
                    if !self.editing_text {
                        if ui.add(egui::Button::new("✏️ 編集").fill(Color32::from_rgb(180, 140, 60))).clicked() {
                            if let Some(ann) = text_annotations.get(idx) {
                                self.text_input = ann.text.clone();
                                self.text_font_size = ann.font_size;
                                self.text_font_type = ann.font_type;
                                self.text_transparent = ann.transparent;
                                self.editing_text = true;
                                self.placing_text = false;
                            }
                        }
                    }
                }
            });
        }

        ui.separator();

        // ページサイズ計算（回転後）
        let page_size = doc.page_size(page_index);
        
        // 基本レンダリング解像度（最大800px、高解像度は不要）
        let max_render_size = 800.0;
        let scale_factor = (max_render_size / page_size.0.max(page_size.1)).min(1.0);
        let base_width = (page_size.0 * scale_factor) as u32;
        let base_height = (page_size.1 * scale_factor) as u32;
        
        // ページテクスチャを更新（ページ変更または回転変更時のみ）
        if self.current_page_index != Some(page_index) 
            || self.cached_rotation != rotation 
            || self.cached_base_size != (base_width, base_height)
        {
            self.current_page_index = Some(page_index);
            self.cached_rotation = rotation;
            self.cached_base_size = (base_width, base_height);
            self.page_texture = None;
            self.selected_stamp_index = None;
            self.selected_text_index = None;
        }

        // ページをレンダリング（基本解像度で1回だけ）
        if self.page_texture.is_none() {
            if let Some(image) = doc.render_page(page_index, base_width, base_height) {
                self.page_texture = Some(ui.ctx().load_texture(
                    format!("page_{}", page_index),
                    image,
                    egui::TextureOptions::LINEAR,  // スケーリング時に滑らかに
                ));
            }
        }

        // 表示サイズ（ズームは表示スケーリングで対応）
        let display_width = page_size.0 * self.zoom;
        let display_height = page_size.1 * self.zoom;

        // ページ描画
        if let Some(ref texture) = self.page_texture {
            let size = Vec2::new(display_width, display_height);
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
                    if let Some((_, Some(tex), _, _)) = custom_stamps.iter().find(|(n, _, _, _)| n == name) {
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

                // 選択枠とリサイズハンドル
                if is_selected {
                    ui.painter().rect_stroke(
                        stamp_rect.expand(3.0),
                        4.0,
                        egui::Stroke::new(3.0, Color32::YELLOW),
                    );
                    
                    // リサイズハンドル（右下）
                    let handle_size = 12.0;
                    let handle_rect = egui::Rect::from_min_size(
                        egui::pos2(stamp_rect.max.x - handle_size / 2.0, stamp_rect.max.y - handle_size / 2.0),
                        Vec2::splat(handle_size),
                    );
                    ui.painter().rect_filled(handle_rect, 2.0, Color32::from_rgb(60, 120, 200));
                    ui.painter().rect_stroke(handle_rect, 2.0, egui::Stroke::new(1.0, Color32::WHITE));
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
                // 複数行テキストの場合、最長行の幅と行数で計算
                let lines: Vec<&str> = annotation.text.lines().collect();
                let max_line_len = lines.iter().map(|l| l.chars().count()).max().unwrap_or(1);
                let line_count = lines.len().max(1);
                let text_width = max_line_len as f32 * annotation.font_size * 0.6;
                let text_height = annotation.font_size * line_count as f32 * 1.2;

                let (display_x, display_y) = self.pdf_to_display_pos(
                    annotation.x, annotation.y, text_width, text_height,
                    orig_w, orig_h, rotation
                );

                let text_pos = egui::pos2(
                    rect.min.x + display_x * self.zoom,
                    rect.min.y + display_y * self.zoom,
                );
                
                let is_selected = self.selected_text_index == Some(*global_idx);
                
                // フォントタイプに応じたフォント選択
                let font = match annotation.font_type {
                    FontType::Gothic => egui::FontId::proportional(annotation.font_size * self.zoom),
                    FontType::Mincho => egui::FontId::monospace(annotation.font_size * self.zoom),
                };
                
                // 複数行対応でレイアウト
                let galley = ui.painter().layout(
                    annotation.text.clone(),
                    font.clone(),
                    Color32::BLACK,
                    f32::INFINITY,
                );
                let text_rect = egui::Rect::from_min_size(text_pos, galley.size());

                // 背景（透過設定に応じて）
                if !annotation.transparent {
                    ui.painter().rect_filled(
                        text_rect.expand(4.0),
                        2.0,
                        Color32::from_rgb(255, 255, 255),
                    );
                    ui.painter().rect_stroke(
                        text_rect.expand(4.0),
                        2.0,
                        egui::Stroke::new(1.0, Color32::from_gray(180)),
                    );
                }

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
                    
                    // リサイズハンドル（右下）- フォントサイズ変更用
                    let handle_size = 10.0;
                    let handle_rect = egui::Rect::from_min_size(
                        egui::pos2(text_rect.max.x - handle_size / 2.0, text_rect.max.y - handle_size / 2.0),
                        Vec2::splat(handle_size),
                    );
                    ui.painter().rect_filled(handle_rect, 2.0, Color32::from_rgb(200, 120, 60));
                    ui.painter().rect_stroke(handle_rect, 2.0, egui::Stroke::new(1.0, Color32::WHITE));
                }

                ui.painter().galley(text_pos, galley, Color32::BLACK);
            }

            // 現在のページの矩形をフィルタ
            let page_rects: Vec<(usize, &RectAnnotation)> = rect_annotations
                .iter()
                .enumerate()
                .filter(|(_, r)| r.page == page_index)
                .collect();

            // 既存の矩形を描画（回転変換を適用）
            for (global_idx, rect_ann) in &page_rects {
                let (display_x, display_y) = self.pdf_to_display_pos(
                    rect_ann.x, rect_ann.y, rect_ann.width, rect_ann.height,
                    orig_w, orig_h, rotation
                );

                let rect_pos = egui::pos2(
                    rect.min.x + display_x * self.zoom,
                    rect.min.y + display_y * self.zoom,
                );
                let rect_size = Vec2::new(rect_ann.width * self.zoom, rect_ann.height * self.zoom);
                let display_rect = egui::Rect::from_min_size(rect_pos, rect_size);
                
                let is_selected = self.selected_rect_index == Some(*global_idx);
                
                // 矩形を描画（白塗り、枠なし）
                let fill_color = Color32::from_rgba_unmultiplied(
                    rect_ann.color[0], rect_ann.color[1], rect_ann.color[2], rect_ann.color[3]
                );
                ui.painter().rect_filled(display_rect, 0.0, fill_color);
                
                // 選択枠とリサイズハンドル
                if is_selected {
                    ui.painter().rect_stroke(
                        display_rect.expand(2.0),
                        0.0,
                        egui::Stroke::new(2.0, Color32::YELLOW),
                    );
                    
                    // リサイズハンドル（右下）
                    let handle_size = 12.0;
                    let handle_rect = egui::Rect::from_min_size(
                        egui::pos2(display_rect.max.x - handle_size / 2.0, display_rect.max.y - handle_size / 2.0),
                        Vec2::splat(handle_size),
                    );
                    ui.painter().rect_filled(handle_rect, 2.0, Color32::from_rgb(60, 120, 200));
                    ui.painter().rect_stroke(handle_rect, 2.0, egui::Stroke::new(1.0, Color32::WHITE));
                }
            }

            // クリック・ドラッグ処理
            if !self.placing_stamp && !self.placing_text && !self.placing_rect {
                if response.clicked() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        let mut found = false;
                        
                        // 矩形の選択（最前面のものから）
                        for (global_idx, rect_ann) in page_rects.iter().rev() {
                            let (display_x, display_y) = self.pdf_to_display_pos(
                                rect_ann.x, rect_ann.y, rect_ann.width, rect_ann.height,
                                orig_w, orig_h, rotation
                            );
                            let display_rect = egui::Rect::from_min_size(
                                egui::pos2(rect.min.x + display_x * self.zoom, rect.min.y + display_y * self.zoom),
                                Vec2::new(rect_ann.width * self.zoom, rect_ann.height * self.zoom),
                            );
                            if display_rect.contains(pos) {
                                self.selected_rect_index = Some(*global_idx);
                                self.selected_stamp_index = None;
                                self.selected_text_index = None;
                                found = true;
                                break;
                            }
                        }
                        
                        // スタンプの選択
                        if !found {
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
                                    self.selected_rect_index = None;
                                    found = true;
                                    break;
                                }
                            }
                        }
                        
                        // テキストの選択
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
                                    self.selected_rect_index = None;
                                    found = true;
                                    break;
                                }
                            }
                        }
                        
                        if !found {
                            self.selected_stamp_index = None;
                            self.selected_text_index = None;
                            self.selected_rect_index = None;
                        }
                    }
                }

                // ドラッグ開始
                if response.drag_started() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        let handle_size = 12.0;
                        
                        // スタンプのリサイズハンドルチェック
                        if let Some(idx) = self.selected_stamp_index {
                            if let Some(stamp) = stamps.get(idx) {
                                let (display_x, display_y) = self.pdf_to_display_pos(
                                    stamp.x, stamp.y, stamp.width, stamp.height,
                                    orig_w, orig_h, rotation
                                );
                                let stamp_rect = egui::Rect::from_min_size(
                                    egui::pos2(rect.min.x + display_x * self.zoom, rect.min.y + display_y * self.zoom),
                                    Vec2::new(stamp.width * self.zoom, stamp.height * self.zoom),
                                );
                                
                                // リサイズハンドル（右下）
                                let handle_rect = egui::Rect::from_min_size(
                                    egui::pos2(stamp_rect.max.x - handle_size / 2.0, stamp_rect.max.y - handle_size / 2.0),
                                    Vec2::splat(handle_size),
                                );
                                
                                if handle_rect.contains(pos) {
                                    // リサイズモード
                                    self.resizing = true;
                                    self.resize_corner = ResizeCorner::BottomRight;
                                    self.resize_start_size = Vec2::new(stamp.width, stamp.height);
                                    self.drag_offset = Vec2::new(pos.x - stamp_rect.max.x, pos.y - stamp_rect.max.y);
                                } else if stamp_rect.contains(pos) {
                                    // 移動モード
                                    let stamp_pos = stamp_rect.min;
                                    self.drag_offset = Vec2::new(pos.x - stamp_pos.x, pos.y - stamp_pos.y);
                                    self.dragging = true;
                                }
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
                                let font = egui::FontId::proportional(annotation.font_size * self.zoom);
                                let galley = ui.painter().layout_no_wrap(
                                    annotation.text.clone(),
                                    font,
                                    Color32::BLACK,
                                );
                                let text_rect = egui::Rect::from_min_size(text_pos, galley.size());
                                
                                // リサイズハンドル（右下）
                                let handle_rect = egui::Rect::from_min_size(
                                    egui::pos2(text_rect.max.x - handle_size / 2.0, text_rect.max.y - handle_size / 2.0),
                                    Vec2::splat(handle_size),
                                );
                                
                                if handle_rect.contains(pos) {
                                    // リサイズモード（フォントサイズ変更）
                                    self.resizing = true;
                                    self.resize_corner = ResizeCorner::BottomRight;
                                    self.resize_start_size = Vec2::new(annotation.font_size, 0.0);
                                    self.drag_offset = Vec2::new(pos.x - text_rect.max.x, pos.y - text_rect.max.y);
                                } else if text_rect.contains(pos) {
                                    // 移動モード
                                    self.drag_offset = Vec2::new(pos.x - text_pos.x, pos.y - text_pos.y);
                                    self.dragging = true;
                                }
                            }
                        } else if let Some(idx) = self.selected_rect_index {
                            if let Some(rect_ann) = rect_annotations.get(idx) {
                                let (display_x, display_y) = self.pdf_to_display_pos(
                                    rect_ann.x, rect_ann.y, rect_ann.width, rect_ann.height,
                                    orig_w, orig_h, rotation
                                );
                                let display_rect = egui::Rect::from_min_size(
                                    egui::pos2(rect.min.x + display_x * self.zoom, rect.min.y + display_y * self.zoom),
                                    Vec2::new(rect_ann.width * self.zoom, rect_ann.height * self.zoom),
                                );
                                
                                // リサイズハンドル（右下）
                                let handle_rect = egui::Rect::from_min_size(
                                    egui::pos2(display_rect.max.x - handle_size / 2.0, display_rect.max.y - handle_size / 2.0),
                                    Vec2::splat(handle_size),
                                );
                                
                                if handle_rect.contains(pos) {
                                    // リサイズモード
                                    self.resizing = true;
                                    self.resize_corner = ResizeCorner::BottomRight;
                                    self.resize_start_size = Vec2::new(rect_ann.width, rect_ann.height);
                                    self.drag_offset = Vec2::new(pos.x - display_rect.max.x, pos.y - display_rect.max.y);
                                } else if display_rect.contains(pos) {
                                    // 移動モード
                                    self.drag_offset = Vec2::new(pos.x - display_rect.min.x, pos.y - display_rect.min.y);
                                    self.dragging = true;
                                }
                            }
                        }
                    }
                }

                if response.dragged() && self.dragging {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                }
                
                if response.dragged() && self.resizing {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeNwSe);
                }

                // ドラッグ終了 - 移動
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
                        } else if let Some(idx) = self.selected_rect_index {
                            if let Some(rect_ann) = rect_annotations.get(idx) {
                                let (pdf_x, pdf_y) = self.display_to_pdf(
                                    display_x, display_y, rect_ann.width, rect_ann.height,
                                    orig_w, orig_h, rotation
                                );
                                result.move_rect = Some((idx, pdf_x, pdf_y));
                            }
                        }
                    }
                    self.dragging = false;
                }
                
                // ドラッグ終了 - リサイズ
                if response.drag_stopped() && self.resizing {
                    if let Some(pos) = ui.input(|i| i.pointer.hover_pos()) {
                        if let Some(idx) = self.selected_stamp_index {
                            if let Some(stamp) = stamps.get(idx) {
                                let (display_x, display_y) = self.pdf_to_display_pos(
                                    stamp.x, stamp.y, stamp.width, stamp.height,
                                    orig_w, orig_h, rotation
                                );
                                let stamp_min = egui::pos2(
                                    rect.min.x + display_x * self.zoom,
                                    rect.min.y + display_y * self.zoom,
                                );
                                
                                // 新しいサイズを計算（最小サイズ制限付き）
                                let new_width = ((pos.x - self.drag_offset.x - stamp_min.x) / self.zoom).max(20.0);
                                let new_height = ((pos.y - self.drag_offset.y - stamp_min.y) / self.zoom).max(20.0);
                                
                                result.resize_stamp = Some((idx, new_width, new_height));
                            }
                        } else if let Some(idx) = self.selected_text_index {
                            if let Some(annotation) = text_annotations.get(idx) {
                                let (display_x, display_y) = self.pdf_to_display_pos(
                                    annotation.x, annotation.y, 
                                    annotation.text.len() as f32 * annotation.font_size * 0.6, 
                                    annotation.font_size,
                                    orig_w, orig_h, rotation
                                );
                                let text_min = egui::pos2(
                                    rect.min.x + display_x * self.zoom,
                                    rect.min.y + display_y * self.zoom,
                                );
                                
                                // 新しいフォントサイズを計算（高さの変化量から）
                                let delta_y = (pos.y - self.drag_offset.y - text_min.y) / self.zoom;
                                let new_font_size = (delta_y).max(8.0).min(72.0);
                                
                                result.resize_text = Some((idx, new_font_size));
                            }
                        } else if let Some(idx) = self.selected_rect_index {
                            if let Some(rect_ann) = rect_annotations.get(idx) {
                                let (display_x, display_y) = self.pdf_to_display_pos(
                                    rect_ann.x, rect_ann.y, rect_ann.width, rect_ann.height,
                                    orig_w, orig_h, rotation
                                );
                                let rect_min = egui::pos2(
                                    rect.min.x + display_x * self.zoom,
                                    rect.min.y + display_y * self.zoom,
                                );
                                
                                // 新しいサイズを計算（最小サイズ制限付き）
                                let new_width = ((pos.x - self.drag_offset.x - rect_min.x) / self.zoom).max(10.0);
                                let new_height = ((pos.y - self.drag_offset.y - rect_min.y) / self.zoom).max(10.0);
                                
                                result.resize_rect = Some((idx, new_width, new_height));
                            }
                        }
                    }
                    self.resizing = false;
                    self.resize_corner = ResizeCorner::None;
                }
            }

            // スタンプ配置モード
            if self.placing_stamp {
                // カスタムスタンプの場合、元のサイズを使用（スケール調整）
                let (stamp_w, stamp_h) = if let Some(idx) = self.selected_custom_stamp_index {
                    if let Some((_, _, w, h)) = custom_stamps.get(idx) {
                        // 最大100ピクセル幅にスケーリング、比率維持
                        let max_size = 100.0;
                        let scale = max_size / (*w as f32).max(*h as f32);
                        (*w as f32 * scale, *h as f32 * scale)
                    } else {
                        (100.0, 50.0)
                    }
                } else {
                    (100.0, 50.0) // 組み込みスタンプは固定サイズ
                };

                if let Some(hover_pos) = ui.input(|i| i.pointer.hover_pos()) {
                    if rect.contains(hover_pos) {
                        let preview_w = stamp_w * self.zoom;
                        let preview_h = stamp_h * self.zoom;
                        let preview_rect = egui::Rect::from_center_size(hover_pos, Vec2::new(preview_w, preview_h));
                        
                        if let Some(idx) = self.selected_custom_stamp_index {
                            if let Some((_, Some(tex), _, _)) = custom_stamps.get(idx) {
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
                        let display_x = (pos.x - rect.min.x) / self.zoom - stamp_w / 2.0;
                        let display_y = (pos.y - rect.min.y) / self.zoom - stamp_h / 2.0;

                        let (pdf_x, pdf_y) = self.display_to_pdf(
                            display_x, display_y, stamp_w, stamp_h,
                            orig_w, orig_h, rotation
                        );

                        result.new_stamp = Some(Stamp {
                            page: page_index,
                            x: pdf_x,
                            y: pdf_y,
                            width: stamp_w,
                            height: stamp_h,
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
                            font_type: self.text_font_type,
                            transparent: self.text_transparent,
                        });
                        self.placing_text = false;
                        self.text_input.clear();
                    }
                }
            }

            // 矩形配置モード（ドラッグで描画）
            if self.placing_rect {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
                
                // ドラッグ開始
                if response.drag_started() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        if rect.contains(pos) {
                            self.rect_start_pos = Some(pos);
                        }
                    }
                }
                
                // ドラッグ中のプレビュー
                if let Some(start_pos) = self.rect_start_pos {
                    if let Some(current_pos) = ui.input(|i| i.pointer.hover_pos()) {
                        let min_x = start_pos.x.min(current_pos.x);
                        let min_y = start_pos.y.min(current_pos.y);
                        let max_x = start_pos.x.max(current_pos.x);
                        let max_y = start_pos.y.max(current_pos.y);
                        
                        let preview_rect = egui::Rect::from_min_max(
                            egui::pos2(min_x, min_y),
                            egui::pos2(max_x, max_y),
                        );
                        
                        // プレビュー描画
                        ui.painter().rect_filled(preview_rect, 0.0, Color32::from_rgba_unmultiplied(255, 255, 255, 200));
                        ui.painter().rect_stroke(preview_rect, 0.0, egui::Stroke::new(1.0, Color32::GRAY));
                    }
                }
                
                // ドラッグ終了で矩形を確定
                if response.drag_stopped() {
                    if let Some(start_pos) = self.rect_start_pos {
                        if let Some(end_pos) = ui.input(|i| i.pointer.hover_pos()) {
                            let min_x = start_pos.x.min(end_pos.x);
                            let min_y = start_pos.y.min(end_pos.y);
                            let max_x = start_pos.x.max(end_pos.x);
                            let max_y = start_pos.y.max(end_pos.y);
                            
                            let display_x = (min_x - rect.min.x) / self.zoom;
                            let display_y = (min_y - rect.min.y) / self.zoom;
                            let width = (max_x - min_x) / self.zoom;
                            let height = (max_y - min_y) / self.zoom;
                            
                            // 最小サイズチェック
                            if width > 5.0 && height > 5.0 {
                                let (pdf_x, pdf_y) = self.display_to_pdf(
                                    display_x, display_y, width, height,
                                    orig_w, orig_h, rotation
                                );
                                
                                result.new_rect = Some(RectAnnotation {
                                    page: page_index,
                                    x: pdf_x,
                                    y: pdf_y,
                                    width,
                                    height,
                                    color: [255, 255, 255, 255],  // 白色
                                });
                            }
                        }
                        self.rect_start_pos = None;
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
                } else if let Some(idx) = self.selected_rect_index {
                    result.delete_rect = Some(idx);
                    self.selected_rect_index = None;
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
