//! PDF processing: text extraction via `pdftotext` CLI (poppler-utils).
//!
//! Falls back to basic metadata extraction if `pdftotext` is not available.
//! Optionally uses `tesseract` for OCR on scanned PDFs.

use async_trait::async_trait;
use mv_core::{KnowledgeNode, MvError, MvResult};
use std::process::Command;

use super::{ModalityProcessor, ModalityStatus, ProcessingResult};

/// PDF processor that extracts text content using `pdftotext` CLI.
pub struct PdfProcessor {
    pdftotext_available: bool,
    tesseract_available: bool,
    ghostscript_available: bool,
}

impl PdfProcessor {
    pub fn new() -> Self {
        let pdftotext_available = Command::new("pdftotext")
            .arg("-v")
            .output()
            .is_ok();

        let tesseract_available = Command::new("tesseract")
            .arg("--version")
            .output()
            .is_ok();

        let ghostscript_available = Command::new("gs")
            .arg("--version")
            .output()
            .is_ok();

        if pdftotext_available {
            tracing::info!("pdftotext available for PDF text extraction");
        }
        if tesseract_available {
            tracing::info!("tesseract available for OCR fallback on scanned PDFs");
        }
        if tesseract_available && !ghostscript_available {
            tracing::warn!("tesseract available but ghostscript missing; OCR conversion may fail");
        }
        if !pdftotext_available && !tesseract_available {
            tracing::warn!(
                "No PDF extraction tools available (install poppler-utils for pdftotext)"
            );
        }

        Self {
            pdftotext_available,
            tesseract_available,
            ghostscript_available,
        }
    }

    /// Extract text using pdftotext CLI.
    fn extract_with_pdftotext(&self, file_path: &str) -> Result<String, String> {
        let output = Command::new("pdftotext")
            .arg("-layout")
            .arg(file_path)
            .arg("-") // output to stdout
            .output()
            .map_err(|e| format!("failed to run pdftotext: {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("pdftotext failed: {stderr}"));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Get page count using pdfinfo CLI.
    fn get_page_count(&self, file_path: &str) -> Option<u32> {
        let output = Command::new("pdfinfo")
            .arg(file_path)
            .output()
            .ok()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if let Some(count_str) = line.strip_prefix("Pages:") {
                return count_str.trim().parse().ok();
            }
        }
        None
    }

    /// Extract text from a scanned PDF using tesseract OCR.
    /// Converts PDF to images first, then runs OCR.
    fn extract_with_ocr(&self, file_path: &str) -> Result<String, String> {
        let temp_dir = std::env::temp_dir();
        let stem = uuid::Uuid::now_v7();
        let tiff_path = temp_dir.join(format!("mv_ocr_{stem}.tiff"));

        // Convert PDF to TIFF using ghostscript (commonly available)
        let gs_result = Command::new("gs")
            .args([
                "-dNOPAUSE",
                "-dBATCH",
                "-sDEVICE=tiffg4",
                "-r300",
                &format!("-sOutputFile={}", tiff_path.display()),
                file_path,
            ])
            .output();

        match gs_result {
            Ok(output) if output.status.success() => {}
            _ => return Err("ghostscript not available or failed".to_string()),
        }

        // Run tesseract on the TIFF
        let output = Command::new("tesseract")
            .arg(&tiff_path)
            .arg("stdout")
            .output()
            .map_err(|e| format!("tesseract failed: {e}"))?;

        let _ = std::fs::remove_file(&tiff_path);

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("tesseract OCR failed: {stderr}"));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}

#[async_trait]
impl ModalityProcessor for PdfProcessor {
    fn name(&self) -> &'static str {
        "pdf"
    }

    fn handles(&self) -> &[&str] {
        &["application/pdf"]
    }

    fn status(&self) -> ModalityStatus {
        let available = self.pdftotext_available || self.tesseract_available;
        let mut status = ModalityStatus::new(self.name(), available, self.handles())
            .with_detail("pdftotext_available", serde_json::json!(self.pdftotext_available))
            .with_detail("tesseract_available", serde_json::json!(self.tesseract_available))
            .with_detail("ghostscript_available", serde_json::json!(self.ghostscript_available));

        if !available {
            status = status.with_note("No PDF extraction backend available");
        } else if self.tesseract_available && !self.ghostscript_available {
            status = status.with_note("OCR available but ghostscript missing for PDF-to-image");
        }

        status
    }

    async fn process(&self, file_path: &str, _node: &KnowledgeNode) -> MvResult<ProcessingResult> {
        tracing::info!(file_path, "Processing PDF file");

        let file_size = tokio::fs::metadata(file_path)
            .await
            .map(|m| m.len())
            .map_err(|e| MvError::Storage(format!("Failed to read PDF: {e}")))?;

        let page_count = self.get_page_count(file_path);

        // Try pdftotext first
        let text = if self.pdftotext_available {
            match self.extract_with_pdftotext(file_path) {
                Ok(text) if !text.is_empty() => text,
                Ok(_) if self.tesseract_available && self.ghostscript_available => {
                    // Empty text from pdftotext likely means scanned PDF — try OCR
                    tracing::info!("PDF appears to be scanned, attempting OCR");
                    self.extract_with_ocr(file_path).unwrap_or_else(|e| {
                        tracing::warn!(error = %e, "OCR failed");
                        format!("[Scanned PDF: {file_path} - OCR failed]")
                    })
                }
                Ok(_) if self.tesseract_available => {
                    format!("[Scanned PDF: {file_path} - install ghostscript for OCR]")
                }
                Ok(_) => format!("[Scanned PDF: {file_path} - install tesseract for OCR]"),
                Err(e) => {
                    tracing::warn!(error = %e, "pdftotext failed");
                    format!("[PDF: {file_path} - text extraction failed]")
                }
            }
        } else {
            format!("[PDF: {file_path} - install poppler-utils for text extraction]")
        };

        let is_extracted = !text.starts_with("[PDF:") && !text.starts_with("[Scanned PDF:");
        let word_count = text.split_whitespace().count();

        let mut result = ProcessingResult::new(text)
            .with_tag("pdf".to_string())
            .with_tag("document".to_string());

        if is_extracted {
            result = result.with_tag("text-extracted".to_string());
        }

        result
            .metadata
            .insert("file_size".into(), serde_json::json!(file_size));
        if let Some(pages) = page_count {
            result
                .metadata
                .insert("page_count".into(), serde_json::json!(pages));
        }
        if is_extracted {
            result
                .metadata
                .insert("word_count".into(), serde_json::json!(word_count));
        }

        Ok(result)
    }
}
