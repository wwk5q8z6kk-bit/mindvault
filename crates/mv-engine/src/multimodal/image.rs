//! Image processing: metadata extraction and optional LLM-based description.
//!
//! Extracts basic image metadata (dimensions from file headers, file size, format).
//! When an LLM with vision capability is available, generates a text description
//! to make images searchable via the vault's hybrid retrieval.

use async_trait::async_trait;
use mv_core::{KnowledgeNode, MvResult};
use std::path::Path;

#[cfg(feature = "image-embeddings")]
use {
    fastembed::{
        EmbeddingModel, ImageEmbedding, ImageEmbeddingModel, ImageInitOptions, TextEmbedding,
        TextInitOptions,
    },
    std::{str::FromStr, sync::{Mutex, OnceLock}},
};

use super::{ModalityProcessor, ModalityStatus, ProcessingResult};

/// Image processor that extracts metadata and generates searchable text.
pub struct ImageProcessor;

#[derive(Debug, Clone)]
pub struct ClipTag {
    pub label: String,
    pub score: f32,
}

#[cfg(feature = "image-embeddings")]
const DEFAULT_CLIP_LABELS: &[&str] = &[
    "screenshot",
    "diagram",
    "chart",
    "document",
    "receipt",
    "invoice",
    "map",
    "photo",
    "portrait",
    "landscape",
    "food",
    "animal",
    "vehicle",
    "logo",
    "code",
    "ui",
    "table",
    "handwritten note",
    "whiteboard",
    "presentation slide",
];

#[cfg(feature = "image-embeddings")]
const DEFAULT_CLIP_MODEL: &str = "Qdrant/clip-ViT-B-32-vision";

#[cfg(feature = "image-embeddings")]
const DEFAULT_CLIP_TEXT_MODEL: &str = "Qdrant/clip-ViT-B-32-text";

#[cfg(feature = "image-embeddings")]
const DEFAULT_CLIP_PROMPT: &str = "a photo of {}";

#[cfg(feature = "image-embeddings")]
const DEFAULT_CLIP_MIN_SIMILARITY: f32 = 0.24;

#[cfg(feature = "image-embeddings")]
const DEFAULT_CLIP_TOP_K: usize = 3;

#[cfg(feature = "image-embeddings")]
static CLIP_TAGGER_STATE: OnceLock<ClipTaggerState> = OnceLock::new();

#[cfg(feature = "image-embeddings")]
enum ClipTaggerState {
    Disabled(String),
    Ready(ClipTagger),
}

#[cfg(feature = "image-embeddings")]
struct ClipTagger {
    embedder: Mutex<ImageEmbedding>,
    labels: Vec<String>,
    label_embeddings: Vec<Vec<f32>>,
    min_similarity: f32,
    top_k: usize,
    image_model: String,
    text_model: String,
}

#[cfg(feature = "image-embeddings")]
impl ClipTaggerState {
    fn from_env() -> Self {
        if !env_flag("MINDVAULT_IMAGE_EMBEDDINGS") {
            return Self::Disabled("image embeddings disabled".to_string());
        }

        match ClipTagger::from_env() {
            Ok(tagger) => Self::Ready(tagger),
            Err(err) => {
                tracing::warn!(error = %err, "CLIP tagger disabled");
                Self::Disabled(err)
            }
        }
    }

    fn is_ready(&self) -> bool {
        matches!(self, Self::Ready(_))
    }
}

#[cfg(feature = "image-embeddings")]
impl ClipTagger {
    fn from_env() -> Result<Self, String> {
        let image_model_name = std::env::var("MINDVAULT_IMAGE_EMBEDDING_MODEL")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_CLIP_MODEL.to_string());
        let text_model_name = std::env::var("MINDVAULT_IMAGE_TEXT_EMBEDDING_MODEL")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_CLIP_TEXT_MODEL.to_string());

        let image_model = ImageEmbeddingModel::from_str(&image_model_name)
            .map_err(|e| format!("invalid image embedding model: {e}"))?;
        let text_model = EmbeddingModel::from_str(&text_model_name)
            .map_err(|e| format!("invalid text embedding model: {e}"))?;

        let labels = parse_label_list(
            "MINDVAULT_IMAGE_EMBEDDING_LABELS",
            DEFAULT_CLIP_LABELS,
        );
        if labels.is_empty() {
            return Err("no image labels configured".to_string());
        }

        let prompt_template = std::env::var("MINDVAULT_IMAGE_EMBEDDING_PROMPT")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_CLIP_PROMPT.to_string());

        let min_similarity = parse_env_f32("MINDVAULT_IMAGE_EMBEDDING_MIN_SIM")
            .unwrap_or(DEFAULT_CLIP_MIN_SIMILARITY);
        let top_k = parse_env_usize("MINDVAULT_IMAGE_EMBEDDING_TOP_K")
            .unwrap_or(DEFAULT_CLIP_TOP_K)
            .max(1);

        let show_download_progress =
            env_flag("MINDVAULT_IMAGE_EMBEDDING_SHOW_DOWNLOAD_PROGRESS");

        let cache_dir = std::env::var("MINDVAULT_IMAGE_EMBEDDING_CACHE_DIR")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .map(std::path::PathBuf::from);

        let mut image_options = ImageInitOptions::new(image_model).with_show_download_progress(
            show_download_progress,
        );
        if let Some(cache_dir) = cache_dir.clone() {
            image_options = image_options.with_cache_dir(cache_dir);
        }

        let embedder = ImageEmbedding::try_new(image_options)
            .map_err(|e| format!("failed to init image embedding model: {e}"))?;

        let mut text_options =
            TextInitOptions::new(text_model).with_show_download_progress(show_download_progress);
        if let Some(cache_dir) = cache_dir {
            text_options = text_options.with_cache_dir(cache_dir);
        }
        let mut text_embedder = TextEmbedding::try_new(text_options)
            .map_err(|e| format!("failed to init text embedding model: {e}"))?;

        let prompts: Vec<String> = labels
            .iter()
            .map(|label| apply_prompt(&prompt_template, label))
            .collect();
        let label_embeddings = text_embedder
            .embed(&prompts, None)
            .map_err(|e| format!("failed to embed clip labels: {e}"))?;

        Ok(Self {
            embedder: Mutex::new(embedder),
            labels,
            label_embeddings,
            min_similarity,
            top_k,
            image_model: image_model_name,
            text_model: text_model_name,
        })
    }

    fn infer_labels(&self, bytes: &[u8]) -> Result<Vec<ClipTag>, String> {
        let embeddings = {
            let mut embedder = self
                .embedder
                .lock()
                .map_err(|_| "image embedder lock poisoned".to_string())?;
            embedder
                .embed_bytes(&[bytes], None)
                .map_err(|e| format!("image embedding failed: {e}"))?
        };

        let Some(image_embedding) = embeddings.first() else {
            return Err("no image embedding generated".to_string());
        };

        let image_norm = l2_norm(image_embedding);
        if image_norm == 0.0 {
            return Err("invalid image embedding norm".to_string());
        }

        let mut scored: Vec<ClipTag> = self
            .labels
            .iter()
            .zip(self.label_embeddings.iter())
            .filter_map(|(label, emb)| {
                if emb.len() != image_embedding.len() {
                    return None;
                }
                let score = cosine_similarity(image_embedding, emb, image_norm);
                if !score.is_finite() {
                    return None;
                }
                Some(ClipTag {
                    label: label.clone(),
                    score,
                })
            })
            .collect();

        scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        Ok(scored
            .into_iter()
            .filter(|tag| tag.score >= self.min_similarity)
            .take(self.top_k)
            .collect())
    }
}

pub fn clip_tags_from_bytes(bytes: &[u8]) -> Option<Vec<ClipTag>> {
    #[cfg(feature = "image-embeddings")]
    {
        let state = CLIP_TAGGER_STATE.get_or_init(ClipTaggerState::from_env);
        match state {
            ClipTaggerState::Ready(tagger) => match tagger.infer_labels(bytes) {
                Ok(tags) if !tags.is_empty() => Some(tags),
                Ok(_) => None,
                Err(err) => {
                    tracing::warn!(error = %err, "CLIP tagger inference failed");
                    None
                }
            },
            ClipTaggerState::Disabled(_) => None,
        }
    }
    #[cfg(not(feature = "image-embeddings"))]
    {
        None
    }
}

fn clip_enabled() -> bool {
    #[cfg(feature = "image-embeddings")]
    {
        CLIP_TAGGER_STATE
            .get_or_init(ClipTaggerState::from_env)
            .is_ready()
    }
    #[cfg(not(feature = "image-embeddings"))]
    {
        false
    }
}

#[cfg(feature = "image-embeddings")]
fn clip_status_details() -> Option<(bool, Option<String>, Option<String>, usize, f32, usize, String)>
{
    let state = CLIP_TAGGER_STATE.get_or_init(ClipTaggerState::from_env);
    match state {
        ClipTaggerState::Ready(tagger) => Some((
            true,
            Some(tagger.image_model.clone()),
            Some(tagger.text_model.clone()),
            tagger.labels.len(),
            tagger.min_similarity,
            tagger.top_k,
            String::new(),
        )),
        ClipTaggerState::Disabled(reason) => Some((
            false,
            None,
            None,
            0,
            DEFAULT_CLIP_MIN_SIMILARITY,
            DEFAULT_CLIP_TOP_K,
            reason.clone(),
        )),
    }
}

fn env_flag(name: &str) -> bool {
    std::env::var(name)
        .ok()
        .map(|value| matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

#[cfg(feature = "image-embeddings")]
fn parse_env_f32(name: &str) -> Option<f32> {
    std::env::var(name).ok()?.trim().parse().ok()
}

#[cfg(feature = "image-embeddings")]
fn parse_env_usize(name: &str) -> Option<usize> {
    std::env::var(name).ok()?.trim().parse().ok()
}

#[cfg(feature = "image-embeddings")]
fn parse_label_list(name: &str, fallback: &[&str]) -> Vec<String> {
    let configured = std::env::var(name)
        .ok()
        .map(|value| {
            value
                .split(',')
                .map(|item| item.trim())
                .filter(|item| !item.is_empty())
                .map(|item| item.to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if configured.is_empty() {
        fallback.iter().map(|item| item.to_string()).collect()
    } else {
        configured
    }
}

#[cfg(feature = "image-embeddings")]
fn apply_prompt(template: &str, label: &str) -> String {
    if template.contains("{}") {
        template.replace("{}", label)
    } else {
        format!("{template} {label}")
    }
}

#[cfg(feature = "image-embeddings")]
fn l2_norm(values: &[f32]) -> f32 {
    values.iter().map(|v| v * v).sum::<f32>().sqrt()
}

#[cfg(feature = "image-embeddings")]
fn cosine_similarity(a: &[f32], b: &[f32], norm_a: f32) -> f32 {
    let dot = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum::<f32>();
    let norm_b = l2_norm(b);
    if norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}

impl ImageProcessor {
    pub fn new() -> Self {
        Self
    }
}

/// Read PNG dimensions from file header (IHDR chunk).
fn png_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    // PNG signature (8 bytes) + IHDR length (4 bytes) + "IHDR" (4 bytes) + width (4) + height (4)
    if data.len() < 24 || &data[0..8] != b"\x89PNG\r\n\x1a\n" {
        return None;
    }
    let w = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
    let h = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
    Some((w, h))
}

/// Read JPEG dimensions from file header (SOF0/SOF2 markers).
fn jpeg_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    if data.len() < 2 || data[0] != 0xFF || data[1] != 0xD8 {
        return None;
    }
    let mut i = 2;
    while i + 4 < data.len() {
        if data[i] != 0xFF {
            i += 1;
            continue;
        }
        let marker = data[i + 1];
        // SOF0 (0xC0) or SOF2 (0xC2) contain dimensions
        if marker == 0xC0 || marker == 0xC2 {
            if i + 9 < data.len() {
                let h = u16::from_be_bytes([data[i + 5], data[i + 6]]) as u32;
                let w = u16::from_be_bytes([data[i + 7], data[i + 8]]) as u32;
                return Some((w, h));
            }
        }
        if i + 3 < data.len() {
            let len = u16::from_be_bytes([data[i + 2], data[i + 3]]) as usize;
            i += 2 + len;
        } else {
            break;
        }
    }
    None
}

/// Detect format and read dimensions from file header bytes.
fn detect_image_info(data: &[u8]) -> (Option<&'static str>, Option<(u32, u32)>) {
    if data.starts_with(b"\x89PNG") {
        ("png".into(), png_dimensions(data))
    } else if data.len() >= 2 && data[0] == 0xFF && data[1] == 0xD8 {
        ("jpeg".into(), jpeg_dimensions(data))
    } else if data.starts_with(b"GIF8") {
        let dims = if data.len() >= 10 {
            let w = u16::from_le_bytes([data[6], data[7]]) as u32;
            let h = u16::from_le_bytes([data[8], data[9]]) as u32;
            Some((w, h))
        } else {
            None
        };
        ("gif".into(), dims)
    } else if data.starts_with(b"RIFF") && data.len() > 12 && &data[8..12] == b"WEBP" {
        (Some("webp"), None) // WebP dimension parsing is complex; skip for now
    } else {
        (None, None)
    }
}

#[async_trait]
impl ModalityProcessor for ImageProcessor {
    fn name(&self) -> &'static str {
        "image"
    }

    fn handles(&self) -> &[&str] {
        &[
            "image/png",
            "image/jpeg",
            "image/jpg",
            "image/gif",
            "image/webp",
            "image/svg+xml",
        ]
    }

    fn status(&self) -> ModalityStatus {
        let mut status = ModalityStatus::new(self.name(), true, self.handles())
            .with_detail("metadata_extraction", serde_json::json!(true))
            .with_detail("vision_caption", serde_json::json!(false));

        #[cfg(feature = "image-embeddings")]
        {
            if let Some((enabled, image_model, text_model, label_count, min_sim, top_k, note)) =
                clip_status_details()
            {
                status = status
                    .with_detail("clip_enabled", serde_json::json!(enabled))
                    .with_detail("clip_label_count", serde_json::json!(label_count))
                    .with_detail("clip_min_similarity", serde_json::json!(min_sim))
                    .with_detail("clip_top_k", serde_json::json!(top_k));

                if let Some(model) = image_model {
                    status = status.with_detail("clip_image_model", serde_json::json!(model));
                }
                if let Some(model) = text_model {
                    status = status.with_detail("clip_text_model", serde_json::json!(model));
                }
                if !note.is_empty() {
                    status = status.with_note(note);
                }
            }
        }

        #[cfg(not(feature = "image-embeddings"))]
        {
            status = status
                .with_detail("clip_enabled", serde_json::json!(false))
                .with_note("CLIP embeddings disabled at build time");
        }

        status
    }

    async fn process(&self, file_path: &str, node: &KnowledgeNode) -> MvResult<ProcessingResult> {
        tracing::info!(file_path, "Processing image file");

        let path = Path::new(file_path);
        let file_name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        let extension = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        // Read first 512 bytes for header analysis (enough for PNG/JPEG/GIF/WebP)
        let header_bytes = std::fs::read(file_path)
            .map(|data| data[..data.len().min(512)].to_vec())
            .unwrap_or_default();

        let (detected_format, dimensions) = detect_image_info(&header_bytes);
        let format = detected_format.unwrap_or(extension);

        let file_size = std::fs::metadata(file_path)
            .map(|m| m.len())
            .unwrap_or(0);

        // Build searchable text description
        let mut description = format!("[Image: {file_name}");
        if let Some((w, h)) = dimensions {
            description.push_str(&format!(", {w}x{h}"));
        }
        description.push_str(&format!(", {format}"));
        if file_size > 0 {
            let size_kb = file_size / 1024;
            description.push_str(&format!(", {size_kb}KB"));
        }
        description.push(']');

        // Include node title/tags as context for search
        if let Some(ref title) = node.title {
            description.push_str(&format!("\nTitle: {title}"));
        }
        if !node.tags.is_empty() {
            description.push_str(&format!("\nTags: {}", node.tags.join(", ")));
        }

        let mut result = ProcessingResult::new(description)
            .with_tag("image".to_string())
            .with_tag(format.to_string());

        // Store metadata
        result
            .metadata
            .insert("format".into(), serde_json::json!(format));
        result
            .metadata
            .insert("file_size".into(), serde_json::json!(file_size));
        if let Some((w, h)) = dimensions {
            result
                .metadata
                .insert("width".into(), serde_json::json!(w));
            result
                .metadata
                .insert("height".into(), serde_json::json!(h));
        }

        if clip_enabled() {
            if let Ok(bytes) = std::fs::read(file_path) {
                if let Some(tags) = clip_tags_from_bytes(&bytes) {
                    let label_text = tags
                        .iter()
                        .map(|tag| tag.label.as_str())
                        .collect::<Vec<_>>()
                        .join(", ");
                    if !label_text.is_empty() {
                        result
                            .metadata
                            .insert("clip_labels".into(), serde_json::json!(label_text));
                        result.text_content.push_str(&format!("\nLabels: {label_text}"));
                        for tag in tags {
                            result.suggested_tags.push(format!("image:{}", tag.label));
                        }
                    }
                }
            }
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_png_dimensions() {
        // Minimal PNG header: signature + IHDR with 100x200
        let mut data = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        data.extend_from_slice(&[0, 0, 0, 13]); // IHDR length
        data.extend_from_slice(b"IHDR");
        data.extend_from_slice(&100u32.to_be_bytes()); // width
        data.extend_from_slice(&200u32.to_be_bytes()); // height
        let (fmt, dims) = detect_image_info(&data);
        assert_eq!(fmt, Some("png"));
        assert_eq!(dims, Some((100, 200)));
    }

    #[test]
    fn detects_gif_dimensions() {
        let mut data = b"GIF89a".to_vec();
        data.extend_from_slice(&320u16.to_le_bytes());
        data.extend_from_slice(&240u16.to_le_bytes());
        let (fmt, dims) = detect_image_info(&data);
        assert_eq!(fmt, Some("gif"));
        assert_eq!(dims, Some((320, 240)));
    }
}
