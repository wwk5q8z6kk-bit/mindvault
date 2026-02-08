//! Multi-modal processing pipeline.
//! Dispatches to modality-specific processors based on content type.

use async_trait::async_trait;
use mv_core::{KnowledgeNode, MvResult};
use serde::Serialize;
use std::collections::HashMap;

/// A processor for a specific modality (audio, image, PDF, etc.)
#[async_trait]
pub trait ModalityProcessor: Send + Sync {
    /// Human-readable name for this processor.
    fn name(&self) -> &'static str;

    /// The content type this processor handles (e.g. "audio/wav", "image/png", "application/pdf")
    fn handles(&self) -> &[&str];

    /// Process a file and return extracted text content and metadata
    async fn process(&self, file_path: &str, node: &KnowledgeNode) -> MvResult<ProcessingResult>;

    /// Status/introspection for diagnostics endpoints.
    fn status(&self) -> ModalityStatus {
        ModalityStatus::new(self.name(), true, self.handles())
    }
}

/// Result of processing a multi-modal input
#[derive(Debug, Clone)]
pub struct ProcessingResult {
    /// Extracted text content (transcription, OCR, etc.)
    pub text_content: String,
    /// Additional tags to add to the node
    pub suggested_tags: Vec<String>,
    /// Summary/description of the content
    pub summary: Option<String>,
    /// Processing metadata (e.g. duration, dimensions, page count)
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModalityStatus {
    pub name: String,
    pub available: bool,
    pub supported_types: Vec<String>,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub details: HashMap<String, serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl ModalityStatus {
    pub fn new(name: &str, available: bool, supported: &[&str]) -> Self {
        Self {
            name: name.to_string(),
            available,
            supported_types: supported.iter().map(|s| s.to_string()).collect(),
            details: HashMap::new(),
            note: None,
        }
    }

    pub fn with_detail(mut self, key: &str, value: serde_json::Value) -> Self {
        self.details.insert(key.to_string(), value);
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }
}

impl ProcessingResult {
    pub fn new(text_content: String) -> Self {
        Self {
            text_content,
            suggested_tags: Vec::new(),
            summary: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn with_summary(mut self, summary: String) -> Self {
        self.summary = Some(summary);
        self
    }

    pub fn with_tag(mut self, tag: String) -> Self {
        self.suggested_tags.push(tag);
        self
    }
}

/// Pipeline that dispatches to registered modality processors.
pub struct MultiModalPipeline {
    processors: Vec<Box<dyn ModalityProcessor>>,
}

impl MultiModalPipeline {
    pub fn new() -> Self {
        Self {
            processors: Vec::new(),
        }
    }

    pub fn register(&mut self, processor: Box<dyn ModalityProcessor>) {
        self.processors.push(processor);
    }

    /// Find the first processor that handles this content type and process the file.
    pub async fn process(
        &self,
        content_type: &str,
        file_path: &str,
        node: &KnowledgeNode,
    ) -> MvResult<Option<ProcessingResult>> {
        for processor in &self.processors {
            if processor.handles().iter().any(|&ct| ct == content_type) {
                let result = processor.process(file_path, node).await?;
                return Ok(Some(result));
            }
        }
        Ok(None)
    }

    /// Check if any processor can handle this content type.
    pub fn can_process(&self, content_type: &str) -> bool {
        self.processors
            .iter()
            .any(|p| p.handles().iter().any(|&ct| ct == content_type))
    }

    /// Return all content types that are supported by registered processors.
    pub fn supported_types(&self) -> Vec<&str> {
        self.processors
            .iter()
            .flat_map(|p| p.handles().iter().copied())
            .collect()
    }

    /// Return status information for each registered processor.
    pub fn status(&self) -> Vec<ModalityStatus> {
        self.processors.iter().map(|p| p.status()).collect()
    }
}

pub mod audio;
pub mod image;
pub mod pdf;
