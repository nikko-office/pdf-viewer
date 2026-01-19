# PDF Viewer アセットセットアップスクリプト
# 日本語フォントとスタンプ画像をダウンロード・生成します

$ErrorActionPreference = "Stop"

Write-Host "=== PDF Viewer アセットセットアップ ===" -ForegroundColor Cyan

# assets ディレクトリ作成
$dirs = @(
    "assets/fonts",
    "assets/stamps"
)

foreach ($dir in $dirs) {
    if (-not (Test-Path $dir)) {
        New-Item -ItemType Directory -Path $dir -Force | Out-Null
        Write-Host "Created directory: $dir" -ForegroundColor Green
    }
}

# 日本語フォントのダウンロード (Noto Sans JP)
$fontPath = "assets/fonts/NotoSansJP-Regular.ttf"
if (-not (Test-Path $fontPath)) {
    Write-Host "Downloading Noto Sans JP font..." -ForegroundColor Yellow
    try {
        $fontUrl = "https://github.com/google/fonts/raw/main/ofl/notosansjp/NotoSansJP-Regular.ttf"
        Invoke-WebRequest -Uri $fontUrl -OutFile $fontPath
        Write-Host "Downloaded: $fontPath" -ForegroundColor Green
    }
    catch {
        Write-Host "Failed to download font. Please download manually from:" -ForegroundColor Red
        Write-Host "  https://fonts.google.com/noto/specimen/Noto+Sans+JP" -ForegroundColor White
        
        # ダミーファイル作成 (ビルドエラー回避用)
        Write-Host "Creating placeholder font file..." -ForegroundColor Yellow
        [System.IO.File]::WriteAllBytes($fontPath, @())
    }
}
else {
    Write-Host "Font already exists: $fontPath" -ForegroundColor Gray
}

# スタンプ画像の生成 (PowerShell + .NET で簡易PNG作成)
Write-Host "Creating stamp images..." -ForegroundColor Yellow

Add-Type -AssemblyName System.Drawing

function Create-StampImage {
    param (
        [string]$text,
        [string]$outputPath,
        [System.Drawing.Color]$bgColor,
        [System.Drawing.Color]$textColor
    )

    $width = 200
    $height = 100
    $bitmap = New-Object System.Drawing.Bitmap($width, $height)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    
    # 透明背景
    $graphics.Clear([System.Drawing.Color]::Transparent)
    
    # 角丸矩形の描画
    $brush = New-Object System.Drawing.SolidBrush($bgColor)
    $pen = New-Object System.Drawing.Pen($textColor, 3)
    
    $rect = New-Object System.Drawing.Rectangle(5, 5, $width - 10, $height - 10)
    $graphics.FillRectangle($brush, $rect)
    $graphics.DrawRectangle($pen, $rect)
    
    # テキスト描画
    $font = New-Object System.Drawing.Font("Yu Gothic UI", 24, [System.Drawing.FontStyle]::Bold)
    $textBrush = New-Object System.Drawing.SolidBrush($textColor)
    $format = New-Object System.Drawing.StringFormat
    $format.Alignment = [System.Drawing.StringAlignment]::Center
    $format.LineAlignment = [System.Drawing.StringAlignment]::Center
    
    $textRect = New-Object System.Drawing.RectangleF(0, 0, $width, $height)
    $graphics.DrawString($text, $font, $textBrush, $textRect, $format)
    
    # 保存
    $bitmap.Save($outputPath, [System.Drawing.Imaging.ImageFormat]::Png)
    
    $graphics.Dispose()
    $bitmap.Dispose()
    $brush.Dispose()
    $pen.Dispose()
    $font.Dispose()
    $textBrush.Dispose()
    
    Write-Host "Created: $outputPath" -ForegroundColor Green
}

# 各スタンプを作成
$stamps = @(
    @{ Text = "承認"; Path = "assets/stamps/approved.png"; BgColor = [System.Drawing.Color]::FromArgb(180, 200, 255, 200); TextColor = [System.Drawing.Color]::FromArgb(255, 0, 128, 0) },
    @{ Text = "却下"; Path = "assets/stamps/rejected.png"; BgColor = [System.Drawing.Color]::FromArgb(180, 255, 200, 200); TextColor = [System.Drawing.Color]::FromArgb(255, 200, 0, 0) },
    @{ Text = "下書き"; Path = "assets/stamps/draft.png"; BgColor = [System.Drawing.Color]::FromArgb(180, 255, 255, 200); TextColor = [System.Drawing.Color]::FromArgb(255, 180, 180, 0) },
    @{ Text = "機密"; Path = "assets/stamps/confidential.png"; BgColor = [System.Drawing.Color]::FromArgb(180, 200, 200, 255); TextColor = [System.Drawing.Color]::FromArgb(255, 100, 0, 150) }
)

foreach ($stamp in $stamps) {
    if (-not (Test-Path $stamp.Path)) {
        Create-StampImage -text $stamp.Text -outputPath $stamp.Path -bgColor $stamp.BgColor -textColor $stamp.TextColor
    }
    else {
        Write-Host "Stamp already exists: $($stamp.Path)" -ForegroundColor Gray
    }
}

# アプリアイコン作成 (簡易版)
$icoPath = "assets/app.ico"
if (-not (Test-Path $icoPath)) {
    Write-Host "Creating app icon..." -ForegroundColor Yellow
    
    $iconSize = 64
    $iconBitmap = New-Object System.Drawing.Bitmap($iconSize, $iconSize)
    $iconGraphics = [System.Drawing.Graphics]::FromImage($iconBitmap)
    
    # グラデーション背景
    $iconGraphics.Clear([System.Drawing.Color]::FromArgb(255, 70, 130, 180))
    
    # PDFアイコン風の描画
    $font = New-Object System.Drawing.Font("Arial", 20, [System.Drawing.FontStyle]::Bold)
    $brush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::White)
    $format = New-Object System.Drawing.StringFormat
    $format.Alignment = [System.Drawing.StringAlignment]::Center
    $format.LineAlignment = [System.Drawing.StringAlignment]::Center
    
    $rect = New-Object System.Drawing.RectangleF(0, 0, $iconSize, $iconSize)
    $iconGraphics.DrawString("PDF", $font, $brush, $rect, $format)
    
    # ICO形式で保存 (BMP経由)
    $tempBmp = [System.IO.Path]::GetTempFileName() + ".bmp"
    $iconBitmap.Save($tempBmp, [System.Drawing.Imaging.ImageFormat]::Bmp)
    
    # 簡易ICO作成 (BMP埋め込み)
    try {
        $icon = [System.Drawing.Icon]::FromHandle($iconBitmap.GetHicon())
        $fs = [System.IO.File]::OpenWrite($icoPath)
        $icon.Save($fs)
        $fs.Close()
        Write-Host "Created: $icoPath" -ForegroundColor Green
    }
    catch {
        # アイコン作成に失敗した場合はPNGとして保存
        $pngPath = "assets/app.png"
        $iconBitmap.Save($pngPath, [System.Drawing.Imaging.ImageFormat]::Png)
        Write-Host "Created PNG instead: $pngPath" -ForegroundColor Yellow
    }
    
    $iconGraphics.Dispose()
    $iconBitmap.Dispose()
    
    if (Test-Path $tempBmp) { Remove-Item $tempBmp -Force }
}
else {
    Write-Host "Icon already exists: $icoPath" -ForegroundColor Gray
}

Write-Host ""
Write-Host "=== セットアップ完了 ===" -ForegroundColor Cyan
Write-Host "ビルドするには: cargo build --release" -ForegroundColor White
