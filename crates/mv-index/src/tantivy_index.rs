use std::path::Path;
use std::sync::RwLock;

use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::{Directory, Index, IndexReader, IndexWriter, ReloadPolicy};
use uuid::Uuid;

use mv_core::*;

pub struct TantivyFullTextIndex {
    index: Index,
    writer: RwLock<IndexWriter>,
    reader: IndexReader,
    #[allow(dead_code)]
    schema: Schema,
    // Field handles
    f_id: Field,
    f_title: Field,
    f_content: Field,
    f_tags: Field,
    f_kind: Field,
    f_namespace: Field,
}

impl TantivyFullTextIndex {
    pub fn open(path: &Path) -> MvResult<Self> {
        std::fs::create_dir_all(path)
            .map_err(|e| MvError::Index(format!("create index dir: {e}")))?;

        let dir = tantivy::directory::MmapDirectory::open(path)
            .map_err(|e| MvError::Index(format!("mmap dir: {e}")))?;

        Self::open_with_dir(dir)
    }

    pub fn open_in_memory() -> MvResult<Self> {
        Self::open_with_dir(tantivy::directory::RamDirectory::create())
    }

    fn open_with_dir<D: Directory + 'static>(dir: D) -> MvResult<Self> {
        let mut schema_builder = Schema::builder();
        let f_id = schema_builder.add_text_field("id", STRING | STORED);
        let f_title = schema_builder.add_text_field("title", TEXT);
        let f_content = schema_builder.add_text_field("content", TEXT);
        let f_tags = schema_builder.add_text_field("tags", TEXT);
        let f_kind = schema_builder.add_text_field("kind", STRING);
        let f_namespace = schema_builder.add_text_field("namespace", STRING);
        let schema = schema_builder.build();

        let index = Index::open_or_create(dir, schema.clone())
            .map_err(|e| MvError::Index(format!("open index: {e}")))?;

        let writer = index
            .writer(50_000_000) // 50MB heap
            .map_err(|e| MvError::Index(format!("create writer: {e}")))?;

        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .map_err(|e| MvError::Index(format!("create reader: {e}")))?;

        Ok(Self {
            index,
            writer: RwLock::new(writer),
            reader,
            schema,
            f_id,
            f_title,
            f_content,
            f_tags,
            f_kind,
            f_namespace,
        })
    }
}

impl FullTextIndex for TantivyFullTextIndex {
    fn index_node(&self, node: &KnowledgeNode) -> MvResult<()> {
        let writer = self
            .writer
            .write()
            .map_err(|e| MvError::Index(e.to_string()))?;

        // Remove existing document with same ID
        let id_term = tantivy::Term::from_field_text(self.f_id, &node.id.to_string());
        writer.delete_term(id_term);

        let mut doc = tantivy::TantivyDocument::new();
        doc.add_text(self.f_id, node.id.to_string());
        if let Some(ref title) = node.title {
            doc.add_text(self.f_title, title);
        }
        let attachment_search_blob = node
            .metadata
            .get("attachment_search_text")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        if attachment_search_blob.trim().is_empty() {
            doc.add_text(self.f_content, &node.content);
        } else {
            let mut combined_content =
                String::with_capacity(node.content.len() + attachment_search_blob.len() + 1);
            combined_content.push_str(&node.content);
            combined_content.push('\n');
            combined_content.push_str(attachment_search_blob);
            doc.add_text(self.f_content, &combined_content);
        }
        doc.add_text(self.f_tags, node.tags.join(" "));
        doc.add_text(self.f_kind, node.kind.as_str());
        doc.add_text(self.f_namespace, &node.namespace);

        writer
            .add_document(doc)
            .map_err(|e| MvError::Index(e.to_string()))?;
        Ok(())
    }

    fn remove_node(&self, id: Uuid) -> MvResult<()> {
        let writer = self
            .writer
            .write()
            .map_err(|e| MvError::Index(e.to_string()))?;
        let id_term = tantivy::Term::from_field_text(self.f_id, &id.to_string());
        writer.delete_term(id_term);
        Ok(())
    }

    fn search(&self, query: &str, limit: usize) -> MvResult<Vec<(Uuid, f64)>> {
        let searcher = self.reader.searcher();

        let query_parser =
            QueryParser::for_index(&self.index, vec![self.f_title, self.f_content, self.f_tags]);

        let parsed = query_parser
            .parse_query(query)
            .map_err(|e| MvError::Index(format!("parse query: {e}")))?;

        let top_docs = searcher
            .search(&parsed, &TopDocs::with_limit(limit))
            .map_err(|e| MvError::Index(format!("search: {e}")))?;

        let mut results = Vec::new();
        for (score, doc_address) in top_docs {
            let doc: tantivy::TantivyDocument = searcher
                .doc(doc_address)
                .map_err(|e| MvError::Index(format!("fetch doc: {e}")))?;

            if let Some(id_value) = doc.get_first(self.f_id) {
                if let Some(id_str) = id_value.as_str() {
                    if let Ok(uuid) = Uuid::parse_str(id_str) {
                        results.push((uuid, score as f64));
                    }
                }
            }
        }

        Ok(results)
    }

    fn commit(&self) -> MvResult<()> {
        let mut writer = self
            .writer
            .write()
            .map_err(|e| MvError::Index(e.to_string()))?;
        writer
            .commit()
            .map_err(|e| MvError::Index(format!("commit: {e}")))?;
        self.reader
            .reload()
            .map_err(|e| MvError::Index(format!("reload: {e}")))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_and_search() {
        let idx = TantivyFullTextIndex::open_in_memory().unwrap();

        let node = KnowledgeNode::new(
            NodeKind::Fact,
            "Rust is a systems programming language".into(),
        )
        .with_title("About Rust")
        .with_tags(vec!["rust".into(), "programming".into()]);

        idx.index_node(&node).unwrap();
        idx.commit().unwrap();

        let results = idx.search("rust programming", 5).unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].0, node.id);
    }

    #[test]
    fn test_remove_node() {
        let idx = TantivyFullTextIndex::open_in_memory().unwrap();

        let node = KnowledgeNode::new(NodeKind::Fact, "temporary fact".into());
        let id = node.id;

        idx.index_node(&node).unwrap();
        idx.commit().unwrap();

        let results = idx.search("temporary", 5).unwrap();
        assert!(!results.is_empty());

        idx.remove_node(id).unwrap();
        idx.commit().unwrap();

        let results = idx.search("temporary", 5).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_attachment_search_blob_is_indexed() {
        let idx = TantivyFullTextIndex::open_in_memory().unwrap();
        let mut node = KnowledgeNode::new(NodeKind::Fact, "Sprint plan".into())
            .with_title("Planning")
            .with_tags(vec!["ops".into()]);
        node.metadata.insert(
            "attachment_search_text".into(),
            "invoice reference 2026".into(),
        );

        idx.index_node(&node).unwrap();
        idx.commit().unwrap();

        let results = idx.search("invoice 2026", 5).unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].0, node.id);
    }
}
