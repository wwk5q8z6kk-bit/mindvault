use std::path::Path;
use std::sync::Arc;

#[cfg(feature = "local-embeddings")]
use std::sync::Mutex;

use arrow_array::{Array, Float32Array, RecordBatch, RecordBatchIterator, StringArray};
use arrow_schema::{DataType, Field, Schema};
use async_trait::async_trait;
use futures::TryStreamExt;
use lancedb::query::{ExecutableQuery, QueryBase};
use uuid::Uuid;

use mv_core::*;

#[cfg(feature = "local-embeddings")]
use fastembed::{EmbeddingModel, TextEmbedding, TextInitOptions};

pub struct LanceVectorStore {
    db: lancedb::Connection,
    table_name: String,
    dimensions: usize,
}

impl LanceVectorStore {
    pub async fn open(path: &Path, dimensions: usize) -> MvResult<Self> {
        let path_str = path
            .to_str()
            .ok_or_else(|| MvError::Storage("invalid lancedb path encoding".into()))?;

        let db = lancedb::connect(path_str)
            .execute()
            .await
            .map_err(|e| MvError::Storage(format!("lancedb connect failed: {e}")))?;

        let store = Self {
            db,
            table_name: "embeddings".into(),
            dimensions,
        };

        store.ensure_table().await?;
        Ok(store)
    }

    fn schema(&self) -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("content", DataType::Utf8, false),
            Field::new(
                "vector",
                DataType::FixedSizeList(
                    Arc::new(Field::new("item", DataType::Float32, true)),
                    self.dimensions as i32,
                ),
                false,
            ),
        ]))
    }

    async fn ensure_table(&self) -> MvResult<()> {
        let tables = self
            .db
            .table_names()
            .execute()
            .await
            .map_err(|e| MvError::Storage(format!("lancedb list tables: {e}")))?;

        if !tables.contains(&self.table_name) {
            let schema = self.schema();
            let batch = RecordBatch::new_empty(schema.clone());
            let batches = RecordBatchIterator::new(vec![Ok(batch)], schema);
            self.db
                .create_table(&self.table_name, Box::new(batches))
                .execute()
                .await
                .map_err(|e| MvError::Storage(format!("lancedb create table: {e}")))?;
        }
        Ok(())
    }

    async fn get_table(&self) -> MvResult<lancedb::Table> {
        self.db
            .open_table(&self.table_name)
            .execute()
            .await
            .map_err(|e| MvError::Storage(format!("lancedb open table: {e}")))
    }
}

#[async_trait]
impl VectorStore for LanceVectorStore {
    async fn upsert(&self, id: Uuid, embedding: Vec<f32>, content: &str) -> MvResult<()> {
        if embedding.len() != self.dimensions {
            return Err(MvError::InvalidInput(format!(
                "embedding dimension mismatch: expected {}, got {}",
                self.dimensions,
                embedding.len()
            )));
        }

        // Delete existing if present, then insert
        let _ = VectorStore::delete(self, id).await;

        let table = self.get_table().await?;
        let schema = self.schema();

        let id_array = StringArray::from(vec![id.to_string()]);
        let content_array = StringArray::from(vec![content.to_string()]);

        // Build FixedSizeListArray from Float32Array
        let values = Float32Array::from(embedding);
        let field = Arc::new(Field::new("item", DataType::Float32, true));
        let vector_array = arrow_array::FixedSizeListArray::new(
            field,
            self.dimensions as i32,
            Arc::new(values) as Arc<dyn Array>,
            None,
        );

        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(id_array),
                Arc::new(content_array),
                Arc::new(vector_array),
            ],
        )
        .map_err(|e| MvError::Storage(format!("record batch error: {e}")))?;

        let batches = RecordBatchIterator::new(vec![Ok(batch)], schema);
        table
            .add(Box::new(batches))
            .execute()
            .await
            .map_err(|e| MvError::Storage(format!("lancedb upsert: {e}")))?;

        Ok(())
    }

    async fn search(
        &self,
        embedding: Vec<f32>,
        limit: usize,
        min_score: f64,
    ) -> MvResult<Vec<(Uuid, f64)>> {
        if embedding.len() != self.dimensions {
            return Err(MvError::InvalidInput(format!(
                "query embedding dimension mismatch: expected {}, got {}",
                self.dimensions,
                embedding.len()
            )));
        }

        let table = self.get_table().await?;

        let query = table
            .vector_search(embedding)
            .map_err(|e| MvError::Storage(format!("lancedb query build: {e}")))?
            .limit(limit);

        let stream = query
            .execute()
            .await
            .map_err(|e| MvError::Storage(format!("lancedb search: {e}")))?;

        let batches: Vec<RecordBatch> = stream
            .try_collect()
            .await
            .map_err(|e| MvError::Storage(format!("lancedb collect: {e}")))?;

        let mut scored = Vec::new();
        for batch in &batches {
            let id_col: Option<&StringArray> = batch
                .column_by_name("id")
                .and_then(|c: &Arc<dyn Array>| c.as_any().downcast_ref::<StringArray>());
            let dist_col: Option<&Float32Array> = batch
                .column_by_name("_distance")
                .and_then(|c: &Arc<dyn Array>| c.as_any().downcast_ref::<Float32Array>());

            if let (Some(ids), Some(distances)) = (id_col, dist_col) {
                for i in 0..ids.len() {
                    if let Ok(uuid) = Uuid::parse_str(ids.value(i)) {
                        let distance = distances.value(i) as f64;
                        let score = 1.0 / (1.0 + distance);
                        scored.push((uuid, score));
                    }
                }
            }
        }

        scored.retain(|(_, score)| *score >= min_score);
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        Ok(scored)
    }

    async fn delete(&self, id: Uuid) -> MvResult<()> {
        let table = self.get_table().await?;
        table
            .delete(&format!("id = '{}'", id))
            .await
            .map_err(|e| MvError::Storage(format!("lancedb delete: {e}")))?;
        Ok(())
    }
}

/// OpenAI-compatible embedding client.
pub struct OpenAiEmbedder {
    client: reqwest::Client,
    api_key: String,
    model: String,
    dimensions: usize,
}

impl OpenAiEmbedder {
    pub fn new(api_key: String, model: String, dimensions: usize) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            model,
            dimensions,
        }
    }

    pub fn from_env(model: String, dimensions: usize) -> MvResult<Self> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| MvError::Config("OPENAI_API_KEY not set".into()))?;
        Ok(Self::new(api_key, model, dimensions))
    }
}

/// Local embedding provider using fastembed (ONNX).
pub struct KnowledgeVaultIndexNoteEmbeddingFastembedLocalEmbedder {
    #[cfg(feature = "local-embeddings")]
    model: Arc<Mutex<TextEmbedding>>,
    model_name: String,
    dimensions: usize,
}

impl KnowledgeVaultIndexNoteEmbeddingFastembedLocalEmbedder {
    pub fn try_new(model_name: &str) -> MvResult<Self> {
        #[cfg(not(feature = "local-embeddings"))]
        {
            Err(MvError::Config(format!(
                "local_fastembed provider requires mv-storage feature 'local-embeddings' (requested model '{model_name}')"
            )))
        }

        #[cfg(feature = "local-embeddings")]
        {
            let resolved_model = fastembed_embedding_model_from_name(model_name).ok_or_else(|| {
                MvError::Config(format!(
                    "unsupported fastembed model '{model_name}'. Supported: bge-small-en-v1.5, all-minilm-l6-v2"
                ))
            })?;

            let options = TextInitOptions::new(resolved_model).with_show_download_progress(false);
            let mut embedding_model = TextEmbedding::try_new(options)
                .map_err(|err| MvError::Embedding(format!("fastembed init failed: {err}")))?;

            // Probe output dimensions once during init so vector store sizing can be aligned.
            let probe = embedding_model
                .embed(vec!["dimension probe"], Some(1))
                .map_err(|err| MvError::Embedding(format!("fastembed probe failed: {err}")))?;
            let dimensions = probe.first().map(Vec::len).ok_or_else(|| {
                MvError::Embedding("fastembed probe returned empty output".into())
            })?;

            Ok(Self {
                model: Arc::new(Mutex::new(embedding_model)),
                model_name: normalized_fastembed_model_name(model_name).to_string(),
                dimensions,
            })
        }
    }

    pub fn model_name(&self) -> &str {
        &self.model_name
    }

    pub fn dimensions(&self) -> usize {
        self.dimensions
    }
}

#[async_trait]
impl Embedder for OpenAiEmbedder {
    async fn embed(&self, text: &str) -> MvResult<Vec<f32>> {
        let results = self.embed_batch(&[text.to_string()]).await?;
        results
            .into_iter()
            .next()
            .ok_or_else(|| MvError::Embedding("empty response".into()))
    }

    async fn embed_batch(&self, texts: &[String]) -> MvResult<Vec<Vec<f32>>> {
        #[derive(serde::Serialize)]
        struct EmbedRequest<'a> {
            model: &'a str,
            input: &'a [String],
        }

        #[derive(serde::Deserialize)]
        struct EmbedResponse {
            data: Vec<EmbedData>,
        }

        #[derive(serde::Deserialize)]
        struct EmbedData {
            embedding: Vec<f32>,
        }

        let resp = self
            .client
            .post("https://api.openai.com/v1/embeddings")
            .bearer_auth(&self.api_key)
            .json(&EmbedRequest {
                model: &self.model,
                input: texts,
            })
            .send()
            .await
            .map_err(|e| MvError::Embedding(format!("request failed: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(MvError::Embedding(format!("API error {status}: {body}")));
        }

        let data: EmbedResponse = resp
            .json()
            .await
            .map_err(|e| MvError::Embedding(format!("parse error: {e}")))?;

        Ok(data.data.into_iter().map(|d| d.embedding).collect())
    }

    fn dimensions(&self) -> usize {
        self.dimensions
    }
}

#[cfg(feature = "local-embeddings")]
#[async_trait]
impl Embedder for KnowledgeVaultIndexNoteEmbeddingFastembedLocalEmbedder {
    async fn embed(&self, text: &str) -> MvResult<Vec<f32>> {
        let mut values = self.embed_batch(&[text.to_string()]).await?;
        if values.is_empty() {
            return Err(MvError::Embedding("fastembed returned empty batch".into()));
        }
        Ok(values.remove(0))
    }

    async fn embed_batch(&self, texts: &[String]) -> MvResult<Vec<Vec<f32>>> {
        let model = Arc::clone(&self.model);
        let inputs = texts.to_vec();

        tokio::task::spawn_blocking(move || {
            let mut locked = model
                .lock()
                .map_err(|err| MvError::Embedding(format!("fastembed lock poisoned: {err}")))?;
            let embeddings = locked
                .embed(inputs, Some(16))
                .map_err(|err| MvError::Embedding(format!("fastembed inference failed: {err}")))?;
            Ok(embeddings)
        })
        .await
        .map_err(|err| MvError::Embedding(format!("fastembed task join error: {err}")))?
    }

    fn dimensions(&self) -> usize {
        self.dimensions
    }
}

#[cfg(not(feature = "local-embeddings"))]
#[async_trait]
impl Embedder for KnowledgeVaultIndexNoteEmbeddingFastembedLocalEmbedder {
    async fn embed(&self, _text: &str) -> MvResult<Vec<f32>> {
        Err(MvError::Config(
            "local_fastembed provider is disabled at compile time; rebuild with feature 'local-embeddings'"
                .into(),
        ))
    }

    async fn embed_batch(&self, _texts: &[String]) -> MvResult<Vec<Vec<f32>>> {
        Err(MvError::Config(
            "local_fastembed provider is disabled at compile time; rebuild with feature 'local-embeddings'"
                .into(),
        ))
    }

    fn dimensions(&self) -> usize {
        self.dimensions
    }
}

/// No-op embedder for testing (returns zero vectors).
pub struct NoopEmbedder {
    dims: usize,
}

impl NoopEmbedder {
    pub fn new(dims: usize) -> Self {
        Self { dims }
    }
}

#[async_trait]
impl Embedder for NoopEmbedder {
    async fn embed(&self, _text: &str) -> MvResult<Vec<f32>> {
        Ok(vec![0.0; self.dims])
    }

    async fn embed_batch(&self, texts: &[String]) -> MvResult<Vec<Vec<f32>>> {
        Ok(texts.iter().map(|_| vec![0.0; self.dims]).collect())
    }

    fn dimensions(&self) -> usize {
        self.dims
    }
}

#[cfg(any(test, feature = "local-embeddings"))]
fn normalized_fastembed_model_name(model_name: &str) -> &str {
    match model_name.trim().to_ascii_lowercase().as_str() {
        "baai/bge-small-en-v1.5" | "bge-small-en-v1.5" => "bge-small-en-v1.5",
        "sentence-transformers/all-minilm-l6-v2" | "all-minilm-l6-v2" => "all-minilm-l6-v2",
        _ => model_name,
    }
}

#[cfg(any(test, feature = "local-embeddings"))]
fn fastembed_model_key_from_name(model_name: &str) -> Option<&'static str> {
    match normalized_fastembed_model_name(model_name) {
        "bge-small-en-v1.5" => Some("bge-small-en-v1.5"),
        "all-minilm-l6-v2" => Some("all-minilm-l6-v2"),
        _ => None,
    }
}

#[cfg(feature = "local-embeddings")]
fn fastembed_embedding_model_from_name(model_name: &str) -> Option<EmbeddingModel> {
    match fastembed_model_key_from_name(model_name) {
        Some("bge-small-en-v1.5") => Some(EmbeddingModel::BGESmallENV15),
        Some("all-minilm-l6-v2") => Some(EmbeddingModel::AllMiniLML6V2),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_name_normalization_works() {
        assert_eq!(
            normalized_fastembed_model_name("BAAI/bge-small-en-v1.5"),
            "bge-small-en-v1.5"
        );
        assert_eq!(
            normalized_fastembed_model_name("sentence-transformers/all-minilm-l6-v2"),
            "all-minilm-l6-v2"
        );
    }

    #[test]
    fn supported_models_map() {
        assert_eq!(
            fastembed_model_key_from_name("bge-small-en-v1.5"),
            Some("bge-small-en-v1.5")
        );
        assert_eq!(
            fastembed_model_key_from_name("all-minilm-l6-v2"),
            Some("all-minilm-l6-v2")
        );
        assert_eq!(fastembed_model_key_from_name("unsupported-model"), None);
    }

    #[cfg(not(feature = "local-embeddings"))]
    #[test]
    fn local_embedder_requires_feature_flag() {
        let err = match KnowledgeVaultIndexNoteEmbeddingFastembedLocalEmbedder::try_new(
            "bge-small-en-v1.5",
        ) {
            Ok(_) => panic!("local embedder should require feature flag in default build"),
            Err(err) => err,
        };
        match err {
            MvError::Config(message) => {
                assert!(
                    message.contains("local-embeddings"),
                    "unexpected config error: {message}"
                );
            }
            other => panic!("expected config error, got {other:?}"),
        }
    }

    #[cfg(feature = "local-embeddings")]
    #[test]
    fn local_embedder_rejects_unsupported_model_name() {
        let err =
            match KnowledgeVaultIndexNoteEmbeddingFastembedLocalEmbedder::try_new("unsupported") {
                Ok(_) => panic!("unsupported model must be rejected"),
                Err(err) => err,
            };
        match err {
            MvError::Config(message) => {
                assert!(
                    message.contains("unsupported fastembed model"),
                    "unexpected config error: {message}"
                );
            }
            other => panic!("expected config error, got {other:?}"),
        }
    }
}
