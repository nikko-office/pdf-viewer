//! ファイルエクスプローラーパネル

use eframe::egui;
use std::path::{Path, PathBuf};
use std::{env, fs};

/// ファイルエクスプローラーの状態
pub struct FileExplorer {
    current_dir: PathBuf,
    entries: Vec<FileEntry>,
    error_message: Option<String>,
    last_selected_folder: Option<PathBuf>,
}

/// ファイル/ディレクトリエントリ
#[derive(Clone)]
struct FileEntry {
    name: String,
    path: PathBuf,
    is_dir: bool,
    size: u64,
}

impl FileExplorer {
    pub fn new() -> Self {
        let current_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("C:\\"));
        let mut explorer = Self {
            current_dir: current_dir.clone(),
            entries: Vec::new(),
            error_message: None,
            last_selected_folder: None,
        };
        explorer.refresh();
        explorer
    }

    /// ディレクトリ内容を更新
    fn refresh(&mut self) {
        self.entries.clear();
        self.error_message = None;

        match fs::read_dir(&self.current_dir) {
            Ok(entries) => {
                let mut dirs = Vec::new();
                let mut files = Vec::new();

                for entry in entries.flatten() {
                    if let Ok(metadata) = entry.metadata() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        let file_entry = FileEntry {
                            name,
                            path: entry.path(),
                            is_dir: metadata.is_dir(),
                            size: metadata.len(),
                        };

                        if metadata.is_dir() {
                            dirs.push(file_entry);
                        } else {
                            // PDFファイルのみ表示
                            if entry
                                .path()
                                .extension()
                                .map_or(false, |ext| ext.eq_ignore_ascii_case("pdf"))
                            {
                                files.push(file_entry);
                            }
                        }
                    }
                }

                // ディレクトリを先に、名前でソート
                dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

                self.entries = dirs;
                self.entries.extend(files);
            }
            Err(e) => {
                self.error_message = Some(format!("読み込みエラー: {}", e));
            }
        }
    }

    /// 親ディレクトリに移動
    fn go_up(&mut self) {
        if let Some(parent) = self.current_dir.parent() {
            self.current_dir = parent.to_path_buf();
            self.refresh();
        }
    }

    /// 指定ディレクトリに移動
    fn navigate_to(&mut self, path: &Path) {
        if path.is_dir() {
            self.current_dir = path.to_path_buf();
            self.refresh();
        }
    }

    /// UIを描画し、選択されたパスと種類(folder/file)を返す
    pub fn show(&mut self, ui: &mut egui::Ui) -> Option<(PathBuf, bool)> {
        let mut result = None;

        // パスナビゲーション
        ui.horizontal(|ui| {
            if ui.button("⬆").on_hover_text("上のフォルダへ").clicked() {
                self.go_up();
                result = Some((self.current_dir.clone(), true));
            }
            if ui.button("🔄").on_hover_text("更新").clicked() {
                self.refresh();
            }
            if ui.button("🏠").on_hover_text("ホームへ").clicked() {
                if let Some(home) = dirs::home_dir() {
                    self.current_dir = home.clone();
                    self.refresh();
                    result = Some((home, true));
                }
            }
        });

        // 現在のパス表示
        ui.horizontal_wrapped(|ui| {
            ui.label("📁");
            let path_str = self.current_dir.to_string_lossy();
            ui.label(egui::RichText::new(path_str.as_ref()).small());
        });

        ui.separator();

        // ドライブ選択 (Windows)
        #[cfg(windows)]
        {
            ui.horizontal_wrapped(|ui| {
                ui.label("ドライブ:");
                // A-Zの全ドライブをチェック
                for c in b'A'..=b'Z' {
                    let drive = c as char;
                    let drive_path = format!("{}:\\", drive);
                    if Path::new(&drive_path).exists() {
                        let is_current = self.current_dir.starts_with(&drive_path);
                        if ui.selectable_label(is_current, format!("{}:", drive)).clicked() {
                            self.current_dir = PathBuf::from(&drive_path);
                            self.refresh();
                            result = Some((self.current_dir.clone(), true));
                        }
                    }
                }
            });
            ui.separator();
        }

        // エラーメッセージ表示
        if let Some(ref error) = self.error_message {
            ui.colored_label(egui::Color32::RED, error);
        }

        // ファイル一覧
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                for entry in self.entries.clone() {
                    let icon = if entry.is_dir { "📁" } else { "📄" };
                    let label = format!("{} {}", icon, entry.name);

                    let is_selected = entry.is_dir
                        && self.last_selected_folder.as_ref() == Some(&entry.path);

                    let response = ui.selectable_label(is_selected, &label);

                    if response.clicked() {
                        if entry.is_dir {
                            self.last_selected_folder = Some(entry.path.clone());
                            result = Some((entry.path.clone(), true));
                        } else {
                            result = Some((entry.path.clone(), false));
                        }
                    }

                    if response.double_clicked() && entry.is_dir {
                        self.navigate_to(&entry.path);
                        result = Some((self.current_dir.clone(), true));
                    }

                    // ホバー時にファイルサイズを表示
                    if !entry.is_dir {
                        response.on_hover_text(format_size(entry.size));
                    }
                }
            });

        result
    }
}

/// ファイルサイズをフォーマット
fn format_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if size >= GB {
        format!("{:.2} GB", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.2} MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.2} KB", size as f64 / KB as f64)
    } else {
        format!("{} bytes", size)
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
