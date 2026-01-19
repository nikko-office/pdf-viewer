# PDF Viewer - 高機能PDF編集アプリケーション

Rustで作成された、単一EXEで動作するPDF編集アプリケーションです。

## 機能

### 3カラムレイアウト
- **左パネル**: ファイルエクスプローラー (std::fs使用)
- **中央パネル**: PDFのメイン編集画面
- **右パネル**: サムネイル一覧

### ページ操作
- ドラッグ&ドロップによるページ入れ替え
- 右クリックでページ削除・回転 (90°/180°/270°)
- ズーム機能

### 編集機能
- 透過PNGスタンプの配置 (承認/却下/下書き/機密)
- テキストボックスからの文字入力
- 日本語フォント対応

### PDF操作
- 複数PDFの結合
- 指定ページ範囲の分割・保存

## 技術スタック

- **GUI**: egui / eframe
- **PDF処理**: mupdf-rs
- **画像処理**: image crate
- **配布形態**: 単一EXE (リソース埋め込み)

## セットアップ

### 必要条件

- Rust 1.70以上
- Windows 10/11 (推奨)
- Visual Studio Build Tools (Windows)

### 日本語フォントのセットアップ

1. [Google Noto Sans JP](https://fonts.google.com/noto/specimen/Noto+Sans+JP) からフォントをダウンロード
2. `NotoSansJP-Regular.ttf` を `assets/fonts/` に配置

```bash
# または以下のコマンドでダウンロード
curl -L -o assets/fonts/NotoSansJP-Regular.ttf "https://github.com/google/fonts/raw/main/ofl/notosansjp/NotoSansJP-Regular.ttf"
```

### スタンプ画像の作成

`assets/stamps/` ディレクトリに以下の透過PNG画像を配置してください：
- `approved.png` - 承認スタンプ
- `rejected.png` - 却下スタンプ
- `draft.png` - 下書きスタンプ
- `confidential.png` - 機密スタンプ

推奨サイズ: 200x100 ピクセル

## ビルド

### 開発ビルド

```bash
cargo build
cargo run
```

### リリースビルド (最適化)

```bash
cargo build --release
```

ビルド成果物: `target/release/pdf-viewer.exe`

## Windowsスタティックリンク設定

依存DLLなしの単一EXEを生成するための設定：

### 1. `.cargo/config.toml` の設定

```toml
[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "target-feature=+crt-static"]
```

### 2. Visual Studio Build Toolsの設定

Visual Studio InstallerでC++ビルドツールをインストール：
- MSVC v143 - VS 2022 C++ x64/x86 ビルドツール
- Windows 10/11 SDK
- C++ CMake tools for Windows

### 3. MuPDFのスタティックリンク

mupdf-rs クレートは自動的にMuPDFをスタティックリンクします。
追加の設定は不要です。

### 4. 環境変数の設定 (必要な場合)

```powershell
# 64bit ビルドの場合
$env:RUSTFLAGS = "-C target-feature=+crt-static"
cargo build --release --target x86_64-pc-windows-msvc
```

## プロジェクト構造

```
pdf-viewer/
├── Cargo.toml          # 依存関係設定
├── build.rs            # ビルドスクリプト (Windows リソース)
├── README.md           # このファイル
├── src/
│   ├── main.rs         # エントリーポイント
│   ├── app.rs          # アプリケーション状態管理
│   ├── ui/
│   │   ├── mod.rs
│   │   ├── file_explorer.rs    # ファイルエクスプローラー
│   │   ├── thumbnail_panel.rs  # サムネイルパネル
│   │   └── editor_panel.rs     # メイン編集パネル
│   ├── pdf/
│   │   ├── mod.rs
│   │   ├── document.rs    # PDFドキュメント管理
│   │   ├── operations.rs  # PDF操作 (結合/分割)
│   │   └── renderer.rs    # スタンプ/テキスト定義
│   └── resources/
│       └── mod.rs         # 埋め込みリソース管理
└── assets/
    ├── fonts/
    │   └── NotoSansJP-Regular.ttf  # 日本語フォント
    ├── stamps/
    │   ├── approved.png
    │   ├── rejected.png
    │   ├── draft.png
    │   └── confidential.png
    └── app.ico            # アプリケーションアイコン
```

## 使用方法

### 基本操作

1. **PDFを開く**: メニュー「ファイル」→「開く」または左パネルでファイルをクリック
2. **ページ操作**: 右パネルのサムネイルを右クリックして削除/回転
3. **ページ入れ替え**: サムネイルをドラッグ&ドロップ
4. **スタンプ配置**: メニュー「編集」→「スタンプを追加」→ スタンプを選択 → PDF上をクリック
5. **テキスト追加**: メニュー「編集」→「テキストを追加」→ テキスト入力 → PDF上をクリック

### PDF結合

1. メニュー「ファイル」→「結合用PDFを追加」で複数のPDFを追加
2. メニュー「ファイル」→「PDFを結合」を実行

### PDF分割

1. メニュー「ファイル」→「分割」を選択
2. 開始ページと終了ページを入力
3. 「分割」ボタンで新しいPDFとして保存

## ライセンス

MIT License

## 注意事項

- MuPDFはAGPLライセンスです。商用利用の場合はライセンスを確認してください。
- 大きなPDFファイルの処理には時間がかかる場合があります。
- 日本語テキストの表示には日本語フォントの埋め込みが必要です。
