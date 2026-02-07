use async_trait::async_trait;
use uuid::Uuid;

use crate::error::MvResult;
use crate::model::*;

/// Storage backend for knowledge nodes (metadata + content).
#[async_trait]
pub trait NodeStore: Send + Sync {
    async fn insert(&self, node: &KnowledgeNode) -> MvResult<()>;
    async fn get(&self, id: Uuid) -> MvResult<Option<KnowledgeNode>>;
    async fn update(&self, node: &KnowledgeNode) -> MvResult<()>;
    async fn delete(&self, id: Uuid) -> MvResult<bool>;
    async fn list(
        &self,
        filters: &QueryFilters,
        limit: usize,
        offset: usize,
    ) -> MvResult<Vec<KnowledgeNode>>;
    async fn touch(&self, id: Uuid) -> MvResult<()>;
    async fn count(&self, filters: &QueryFilters) -> MvResult<usize>;
}

/// Vector embedding storage + similarity search.
#[async_trait]
pub trait VectorStore: Send + Sync {
    async fn upsert(&self, id: Uuid, embedding: Vec<f32>, content: &str) -> MvResult<()>;
    async fn search(
        &self,
        embedding: Vec<f32>,
        limit: usize,
        min_score: f64,
    ) -> MvResult<Vec<(Uuid, f64)>>;
    async fn delete(&self, id: Uuid) -> MvResult<()>;
}

/// Full-text search index.
pub trait FullTextIndex: Send + Sync {
    fn index_node(&self, node: &KnowledgeNode) -> MvResult<()>;
    fn remove_node(&self, id: Uuid) -> MvResult<()>;
    fn search(&self, query: &str, limit: usize) -> MvResult<Vec<(Uuid, f64)>>;
    fn commit(&self) -> MvResult<()>;
}

/// Knowledge graph storage.
#[async_trait]
pub trait GraphStore: Send + Sync {
    async fn add_relationship(&self, rel: &Relationship) -> MvResult<()>;
    async fn get_relationship(&self, id: Uuid) -> MvResult<Option<Relationship>>;
    async fn remove_relationship(&self, id: Uuid) -> MvResult<bool>;
    async fn get_relationships_from(&self, node_id: Uuid) -> MvResult<Vec<Relationship>>;
    async fn get_relationships_to(&self, node_id: Uuid) -> MvResult<Vec<Relationship>>;
    async fn get_neighbors(&self, node_id: Uuid, depth: usize) -> MvResult<Vec<Uuid>>;
    async fn remove_node_relationships(&self, node_id: Uuid) -> MvResult<usize>;
}

/// Embedding provider.
#[async_trait]
pub trait Embedder: Send + Sync {
    async fn embed(&self, text: &str) -> MvResult<Vec<f32>>;
    async fn embed_batch(&self, texts: &[String]) -> MvResult<Vec<Vec<f32>>>;
    fn dimensions(&self) -> usize;
}

#[cfg(test)]
mod tests {
    use super::*;

    // Ensure traits are object-safe
    fn _assert_node_store_object_safe(_: &dyn NodeStore) {}
    fn _assert_vector_store_object_safe(_: &dyn VectorStore) {}
    fn _assert_full_text_index_object_safe(_: &dyn FullTextIndex) {}
    fn _assert_graph_store_object_safe(_: &dyn GraphStore) {}
    fn _assert_embedder_object_safe(_: &dyn Embedder) {}
}
