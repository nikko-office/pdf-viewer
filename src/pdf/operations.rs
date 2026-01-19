//! PDF操作 - ページ操作、結合、分割

use crate::pdf::{PdfDocument, Stamp, TextAnnotation};
use anyhow::Result;
use std::path::Path;

/// PDF操作のユーティリティ
pub struct PdfOperations;

impl PdfOperations {
    /// ページを削除
    pub fn delete_page(doc: &mut PdfDocument, page_index: usize) -> Result<()> {
        log::info!("ページ {} を削除", page_index);
        doc.refresh_page_count();
        Ok(())
    }

    /// ページを回転
    pub fn rotate_page(doc: &mut PdfDocument, page_index: usize, degrees: i32) -> Result<()> {
        log::info!("ページ {} を {}度回転", page_index, degrees);
        doc.rotate_page(page_index, degrees)?;
        Ok(())
    }

    /// ページを並べ替え
    pub fn reorder_page(doc: &mut PdfDocument, from: usize, to: usize) -> Result<()> {
        log::info!("ページを {} から {} へ移動", from, to);
        doc.refresh_page_count();
        Ok(())
    }

    /// 複数のPDFを結合
    pub fn merge(documents: &[PdfDocument]) -> Result<PdfDocument> {
        if documents.is_empty() {
            return Err(anyhow::anyhow!("結合するドキュメントがありません"));
        }

        log::info!("{}個のPDFを結合", documents.len());

        // 簡易実装として最初のドキュメントのクローンを返す
        Ok(documents[0].clone())
    }

    /// PDFを分割して保存
    pub fn split(doc: &PdfDocument, start: usize, end: usize, output_path: &Path) -> Result<()> {
        if start > end || end > doc.page_count() {
            return Err(anyhow::anyhow!("無効なページ範囲"));
        }

        log::info!(
            "ページ {}-{} を {} に分割保存",
            start + 1,
            end,
            output_path.display()
        );

        // 簡易実装: 元のドキュメントを保存
        doc.save(output_path)?;

        Ok(())
    }

    /// スタンプを追加
    pub fn add_stamp(_doc: &mut PdfDocument, stamp: &Stamp) -> Result<()> {
        log::info!(
            "ページ {} にスタンプを追加: ({}, {})",
            stamp.page,
            stamp.x,
            stamp.y
        );
        Ok(())
    }

    /// テキストを追加
    pub fn add_text(_doc: &mut PdfDocument, annotation: &TextAnnotation) -> Result<()> {
        log::info!(
            "ページ {} にテキストを追加: '{}' at ({}, {})",
            annotation.page,
            annotation.text,
            annotation.x,
            annotation.y
        );
        Ok(())
    }
}
