//! ファイルエクスプローラーパネル - フォルダツリー表示

use eframe::egui::{self, Color32};
use std::collections::HashSet;
use std::path::PathBuf;
use std::fs;

/// ドライブ情報
#[derive(Clone)]
struct DriveInfo {
    path: PathBuf,
    label: String,
}

/// ファイルエクスプローラーの状態
pub struct FileExplorer {
    drives: Vec<DriveInfo>,
    expanded_folders: HashSet<PathBuf>,
    selected_path: Option<PathBuf>,
    error_message: Option<String>,
    
    // ドラッグ&ドロップのターゲット（フォルダ）
    drop_target: Option<PathBuf>,
    
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
    pub drop_target_folder: Option<PathBuf>,
}

impl Default for FileExplorerResult {
    fn default() -> Self {
        Self {
            selected_folder: None,
            selected_file: None,
            file_moved: None,
            file_copied: None,
            file_deleted: None,
            drop_target_folder: None,
        }
    }
}

impl FileExplorer {
    pub fn new() -> Self {
        let mut drives = Vec::new();
        
        // Windowsドライブを取得（名前付き）
        #[cfg(windows)]
        {
            for c in b'A'..=b'Z' {
                let drive_letter = c as char;
                let drive_path = PathBuf::from(format!("{}:\\", drive_letter));
                if drive_path.exists() {
                    let label = get_drive_label(&drive_path, drive_letter);
                    drives.push(DriveInfo {
                        path: drive_path,
                        label,
                    });
                }
            }
        }
        
        #[cfg(not(windows))]
        {
            drives.push(DriveInfo {
                path: PathBuf::from("/"),
                label: "/ (root)".to_string(),
            });
            if let Ok(home) = std::env::var("HOME") {
                drives.push(DriveInfo {
                    path: PathBuf::from(&home),
                    label: format!("🏠 Home ({})", home),
                });
            }
        }

        // ホームディレクトリを展開
        let mut expanded = HashSet::new();
        if let Some(home) = dirs::home_dir() {
            expanded.insert(home);
        }

        Self {
            drives,
            expanded_folders: expanded,
            selected_path: None,
            error_message: None,
            drop_target: None,
            clipboard: None,
            context_menu_path: None,
            context_menu_pos: egui::Pos2::ZERO,
        }
    }

    /// 現在のドロップターゲットを取得
    pub fn get_drop_target(&self) -> Option<&PathBuf> {
        self.drop_target.as_ref()
    }

    /// ドロップターゲットをクリア
    pub fn clear_drop_target(&mut self) {
        self.drop_target = None;
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
            if ui.button("🔄").on_hover_text("更新").clicked() {
                // リフレッシュ（ドライブ再スキャン）
                self.refresh_drives();
            }
        });

        ui.separator();
        ui.label("📂 フォルダ");
        ui.separator();

        // ドロップターゲットをリセット
        self.drop_target = None;

        // ツリー表示
        egui::ScrollArea::both()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                for drive in &self.drives.clone() {
                    self.show_tree_node(ui, &drive.path, &drive.label, 0, &mut result);
                }
            });

        // ドロップターゲットを結果に設定
        if let Some(ref target) = self.drop_target {
            result.drop_target_folder = Some(target.clone());
        }

        // コンテキストメニュー
        if let Some(path) = self.context_menu_path.clone() {
            egui::Area::new(egui::Id::new("folder_context_menu"))
                .fixed_pos(self.context_menu_pos)
                .order(egui::Order::Foreground)
                .show(ui.ctx(), |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui: &mut egui::Ui| {
                        ui.set_min_width(150.0);

                        if ui.button("📂 開く").clicked() {
                            self.expanded_folders.insert(path.clone());
                            self.selected_path = Some(path.clone());
                            result.selected_folder = Some(path.clone());
                            self.context_menu_path = None;
                        }

                        if self.clipboard.is_some() {
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

                        if ui.button("📋 パスをコピー").clicked() {
                            ui.output_mut(|o| o.copied_text = path.to_string_lossy().to_string());
                            self.context_menu_path = None;
                        }
                    });
                });

            // メニュー外クリックで閉じる
            if ui.input(|i| i.pointer.any_click()) && self.context_menu_path.is_some() {
                let pointer_pos = ui.input(|i| i.pointer.hover_pos());
                if let Some(pos) = pointer_pos {
                    let menu_rect = egui::Rect::from_min_size(self.context_menu_pos, egui::vec2(150.0, 150.0));
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

    /// ドライブ情報を更新
    fn refresh_drives(&mut self) {
        self.drives.clear();
        
        #[cfg(windows)]
        {
            for c in b'A'..=b'Z' {
                let drive_letter = c as char;
                let drive_path = PathBuf::from(format!("{}:\\", drive_letter));
                if drive_path.exists() {
                    let label = get_drive_label(&drive_path, drive_letter);
                    self.drives.push(DriveInfo {
                        path: drive_path,
                        label,
                    });
                }
            }
        }
    }

    /// ツリーノードを表示（フォルダのみ）
    fn show_tree_node(
        &mut self,
        ui: &mut egui::Ui,
        path: &PathBuf,
        display_name: &str,
        depth: usize,
        result: &mut FileExplorerResult,
    ) {
        let is_expanded = self.expanded_folders.contains(path);
        let is_selected = self.selected_path.as_ref() == Some(path);

        // インデント
        let indent = depth as f32 * 16.0;

        ui.horizontal(|ui| {
            ui.add_space(indent);

            // 展開/折りたたみアイコン
            let icon = if is_expanded { "▼" } else { "▶" };
            if ui.small_button(icon).clicked() {
                if is_expanded {
                    self.expanded_folders.remove(path);
                } else {
                    self.expanded_folders.insert(path.clone());
                }
            }

            // フォルダアイコンと名前
            let folder_icon = if depth == 0 { "💾" } else { "📁" };
            let label_text = format!("{} {}", folder_icon, display_name);

            let response = ui.selectable_label(is_selected, &label_text);

            // ドロップターゲット判定（ドラッグ中にホバーしている場合）
            if response.hovered() {
                // ドラッグ中かどうかを外部から判定
                self.drop_target = Some(path.clone());
                
                // ドロップターゲットのビジュアル
                ui.painter().rect_stroke(
                    response.rect,
                    2.0,
                    egui::Stroke::new(2.0, Color32::YELLOW),
                );
            }

            // クリック処理
            if response.clicked() {
                self.selected_path = Some(path.clone());
                result.selected_folder = Some(path.clone());
            }

            // ダブルクリックで展開/折りたたみ
            if response.double_clicked() {
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

        // 子フォルダを表示
        if is_expanded {
            if let Ok(entries) = fs::read_dir(path) {
                let mut subdirs: Vec<PathBuf> = entries
                    .flatten()
                    .filter(|e| e.path().is_dir())
                    .filter(|e| {
                        // 隠しフォルダを除外
                        !e.file_name().to_string_lossy().starts_with('.')
                    })
                    .map(|e| e.path())
                    .collect();

                // ソート
                subdirs.sort_by(|a, b| {
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

                for subdir in subdirs {
                    let name = subdir
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    self.show_tree_node(ui, &subdir, &name, depth + 1, result);
                }
            }
        }
    }
}

/// ドライブラベルを取得（Windows）
#[cfg(windows)]
fn get_drive_label(path: &PathBuf, letter: char) -> String {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    
    // GetVolumeInformationWを使用してボリューム名を取得
    let path_wide: Vec<u16> = OsStr::new(&path.to_string_lossy().to_string())
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    
    let mut volume_name: [u16; 261] = [0; 261];
    let mut serial_number: u32 = 0;
    let mut max_component_len: u32 = 0;
    let mut file_system_flags: u32 = 0;
    let mut file_system_name: [u16; 261] = [0; 261];
    
    let result = unsafe {
        windows::Win32::Storage::FileSystem::GetVolumeInformationW(
            windows::core::PCWSTR::from_raw(path_wide.as_ptr()),
            Some(&mut volume_name),
            Some(&mut serial_number),
            Some(&mut max_component_len),
            Some(&mut file_system_flags),
            Some(&mut file_system_name),
        )
    };
    
    if result.is_ok() {
        let name_len = volume_name.iter().position(|&c| c == 0).unwrap_or(0);
        let volume_label = String::from_utf16_lossy(&volume_name[..name_len]);
        
        if !volume_label.is_empty() {
            format!("💾 {}: [{}]", letter, volume_label)
        } else {
            // ボリューム名がない場合、ドライブタイプを判定
            let drive_type = unsafe {
                windows::Win32::Storage::FileSystem::GetDriveTypeW(
                    windows::core::PCWSTR::from_raw(path_wide.as_ptr())
                )
            };
            
            // ドライブタイプの定数値で判定
            let type_name = match drive_type {
                2 => "リムーバブル",
                3 => "ローカル",
                4 => "ネットワーク",
                5 => "CD/DVD",
                6 => "RAM",
                _ => "ドライブ",
            };
            format!("💾 {}: [{}]", letter, type_name)
        }
    } else {
        format!("💾 {}:", letter)
    }
}

#[cfg(not(windows))]
fn get_drive_label(_path: &PathBuf, _letter: char) -> String {
    String::new()
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
