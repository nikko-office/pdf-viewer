//! ファイルエクスプローラーパネル - ツリー表示対応

use eframe::egui::{self, Color32};
use std::collections::HashSet;
use std::path::PathBuf;
use std::fs;

/// ファイルエクスプローラーの状態
pub struct FileExplorer {
    root_paths: Vec<PathBuf>,
    expanded_folders: HashSet<PathBuf>,
    selected_path: Option<PathBuf>,
    error_message: Option<String>,
    
    // ドラッグ&ドロップ
    drag_source: Option<PathBuf>,
    
    // クリップボード
    clipboard: Option<ClipboardItem>,
    
    // コンテキストメニュー
    context_menu_path: Option<PathBuf>,
    context_menu_pos: egui::Pos2,
}

/// クリップボードアイテム
#[derive(Clone)]
pub struct ClipboardItem {
    pub path: PathBuf,
    pub is_cut: bool,
}

/// ファイル操作結果
pub struct FileExplorerResult {
    pub selected_folder: Option<PathBuf>,
    pub selected_file: Option<PathBuf>,
    pub file_moved: Option<(PathBuf, PathBuf)>,
    pub file_copied: Option<(PathBuf, PathBuf)>,
    pub file_deleted: Option<PathBuf>,
}

impl Default for FileExplorerResult {
    fn default() -> Self {
        Self {
            selected_folder: None,
            selected_file: None,
            file_moved: None,
            file_copied: None,
            file_deleted: None,
        }
    }
}

impl FileExplorer {
    pub fn new() -> Self {
        let mut root_paths = Vec::new();
        
        // Windowsドライブを追加
        #[cfg(windows)]
        {
            for c in b'A'..=b'Z' {
                let drive = c as char;
                let drive_path = PathBuf::from(format!("{}:\\", drive));
                if drive_path.exists() {
                    root_paths.push(drive_path);
                }
            }
        }
        
        #[cfg(not(windows))]
        {
            root_paths.push(PathBuf::from("/"));
            if let Ok(home) = std::env::var("HOME") {
                root_paths.push(PathBuf::from(home));
            }
        }

        // ホームディレクトリを展開
        let mut expanded = HashSet::new();
        if let Some(home) = dirs::home_dir() {
            expanded.insert(home);
        }

        Self {
            root_paths,
            expanded_folders: expanded,
            selected_path: None,
            error_message: None,
            drag_source: None,
            clipboard: None,
            context_menu_path: None,
            context_menu_pos: egui::Pos2::ZERO,
        }
    }

    /// UIを描画
    pub fn show(&mut self, ui: &mut egui::Ui) -> FileExplorerResult {
        let mut result = FileExplorerResult::default();

        // ツールバー
        ui.horizontal(|ui| {
            if ui.button("🏠").on_hover_text("ホームへ").clicked() {
                if let Some(home) = dirs::home_dir() {
                    self.expanded_folders.insert(home.clone());
                    self.selected_path = Some(home.clone());
                    result.selected_folder = Some(home);
                }
            }
            if ui.button("📋").on_hover_text("貼り付け").clicked() {
                if let (Some(clip), Some(dest)) = (&self.clipboard, &self.selected_path) {
                    if dest.is_dir() {
                        let dest_path = dest.join(clip.path.file_name().unwrap_or_default());
                        if clip.is_cut {
                            result.file_moved = Some((clip.path.clone(), dest_path));
                        } else {
                            result.file_copied = Some((clip.path.clone(), dest_path));
                        }
                        self.clipboard = None;
                    }
                }
            }
        });

        ui.separator();

        // ツリー表示
        egui::ScrollArea::both()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                for root in self.root_paths.clone() {
                    self.show_tree_node(ui, &root, 0, &mut result);
                }
            });

        // コンテキストメニュー
        if let Some(path) = self.context_menu_path.clone() {
            egui::Area::new(egui::Id::new("file_context_menu"))
                .fixed_pos(self.context_menu_pos)
                .order(egui::Order::Foreground)
                .show(ui.ctx(), |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui: &mut egui::Ui| {
                        ui.set_min_width(150.0);

                        if path.is_file() {
                            if ui.button("📄 開く").clicked() {
                                result.selected_file = Some(path.clone());
                                self.context_menu_path = None;
                            }
                        }

                        if ui.button("📋 コピー").clicked() {
                            self.clipboard = Some(ClipboardItem {
                                path: path.clone(),
                                is_cut: false,
                            });
                            self.context_menu_path = None;
                        }

                        if ui.button("✂ 切り取り").clicked() {
                            self.clipboard = Some(ClipboardItem {
                                path: path.clone(),
                                is_cut: true,
                            });
                            self.context_menu_path = None;
                        }

                        if self.clipboard.is_some() && path.is_dir() {
                            if ui.button("📥 貼り付け").clicked() {
                                if let Some(clip) = &self.clipboard {
                                    let dest_path = path.join(clip.path.file_name().unwrap_or_default());
                                    if clip.is_cut {
                                        result.file_moved = Some((clip.path.clone(), dest_path));
                                    } else {
                                        result.file_copied = Some((clip.path.clone(), dest_path));
                                    }
                                    self.clipboard = None;
                                }
                                self.context_menu_path = None;
                            }
                        }

                        ui.separator();

                        if ui.button("🗑 削除").clicked() {
                            result.file_deleted = Some(path.clone());
                            self.context_menu_path = None;
                        }
                    });
                });

            // メニュー外クリックで閉じる
            if ui.input(|i| i.pointer.any_click()) && self.context_menu_path.is_some() {
                let pointer_pos = ui.input(|i| i.pointer.hover_pos());
                if let Some(pos) = pointer_pos {
                    let menu_rect = egui::Rect::from_min_size(self.context_menu_pos, egui::vec2(150.0, 200.0));
                    if !menu_rect.contains(pos) {
                        self.context_menu_path = None;
                    }
                }
            }
        }

        // エラーメッセージ
        if let Some(ref error) = self.error_message {
            ui.colored_label(Color32::RED, error);
        }

        result
    }

    /// ツリーノードを表示
    fn show_tree_node(
        &mut self,
        ui: &mut egui::Ui,
        path: &PathBuf,
        depth: usize,
        result: &mut FileExplorerResult,
    ) {
        let is_dir = path.is_dir();
        let is_expanded = self.expanded_folders.contains(path);
        let is_selected = self.selected_path.as_ref() == Some(path);

        let name = if depth == 0 {
            path.to_string_lossy().to_string()
        } else {
            path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        };

        // インデント
        let indent = depth as f32 * 16.0;

        ui.horizontal(|ui| {
            ui.add_space(indent);

            // 展開/折りたたみアイコン
            if is_dir {
                let icon = if is_expanded { "▼" } else { "▶" };
                if ui.small_button(icon).clicked() {
                    if is_expanded {
                        self.expanded_folders.remove(path);
                    } else {
                        self.expanded_folders.insert(path.clone());
                    }
                }
            } else {
                ui.add_space(20.0);
            }

            // ファイル/フォルダアイコンと名前
            let icon = if is_dir { "📁" } else { "📄" };
            let label_text = format!("{} {}", icon, name);

            let response = ui.selectable_label(is_selected, &label_text);

            // ドラッグ処理
            if response.drag_started() {
                self.drag_source = Some(path.clone());
            }

            // ドロップ処理
            if is_dir && response.hovered() && ui.input(|i| i.pointer.any_released()) {
                if let Some(source) = self.drag_source.take() {
                    if source != *path && !path.starts_with(&source) {
                        let dest = path.join(source.file_name().unwrap_or_default());
                        result.file_moved = Some((source, dest));
                    }
                }
            }

            // ドラッグ中のビジュアル
            if self.drag_source.is_some() && is_dir && response.hovered() {
                ui.painter().rect_stroke(
                    response.rect,
                    2.0,
                    egui::Stroke::new(2.0, Color32::YELLOW),
                );
            }

            // クリック処理
            if response.clicked() {
                self.selected_path = Some(path.clone());
                if is_dir {
                    result.selected_folder = Some(path.clone());
                    // ダブルクリックで展開
                } else if path.extension().map_or(false, |ext| ext.eq_ignore_ascii_case("pdf")) {
                    result.selected_file = Some(path.clone());
                }
            }

            // ダブルクリックで展開/折りたたみ
            if response.double_clicked() && is_dir {
                if is_expanded {
                    self.expanded_folders.remove(path);
                } else {
                    self.expanded_folders.insert(path.clone());
                }
            }

            // 右クリックでコンテキストメニュー
            if response.secondary_clicked() {
                self.context_menu_path = Some(path.clone());
                self.context_menu_pos = ui.input(|i| i.pointer.hover_pos().unwrap_or_default());
            }
        });

        // 子要素を表示
        if is_dir && is_expanded {
            if let Ok(entries) = fs::read_dir(path) {
                let mut dirs = Vec::new();
                let mut files = Vec::new();

                for entry in entries.flatten() {
                    let entry_path = entry.path();
                    if entry_path.is_dir() {
                        dirs.push(entry_path);
                    } else if entry_path
                        .extension()
                        .map_or(false, |ext| ext.eq_ignore_ascii_case("pdf"))
                    {
                        files.push(entry_path);
                    }
                }

                // ソート
                dirs.sort_by(|a, b| {
                    a.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_lowercase()
                        .cmp(
                            &b.file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_lowercase(),
                        )
                });
                files.sort_by(|a, b| {
                    a.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_lowercase()
                        .cmp(
                            &b.file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_lowercase(),
                        )
                });

                for dir in dirs {
                    self.show_tree_node(ui, &dir, depth + 1, result);
                }
                for file in files {
                    self.show_tree_node(ui, &file, depth + 1, result);
                }
            }
        }
    }

    /// クリップボードを取得
    pub fn get_clipboard(&self) -> Option<&ClipboardItem> {
        self.clipboard.as_ref()
    }
}

// dirs クレートがない場合のフォールバック
mod dirs {
    use std::path::PathBuf;

    pub fn home_dir() -> Option<PathBuf> {
        #[cfg(windows)]
        {
            std::env::var("USERPROFILE").ok().map(PathBuf::from)
        }
        #[cfg(not(windows))]
        {
            std::env::var("HOME").ok().map(PathBuf::from)
        }
    }
}
