use std::path::Path;
use std::sync::Mutex;

use async_trait::async_trait;
use chrono::Utc;
use rusqlite::types::Type;
use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

use mv_core::*;

pub struct SqliteNodeStore {
    conn: Mutex<Connection>,
}

impl SqliteNodeStore {
    pub fn open(path: &Path) -> MvResult<Self> {
        let conn = Connection::open(path)
            .map_err(|e| MvError::Storage(format!("failed to open sqlite: {e}")))?;

        conn.execute_batch(
            "PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000;",
        )
        .map_err(|e| MvError::Storage(format!("pragma error: {e}")))?;

        let store = Self {
            conn: Mutex::new(conn),
        };
        store.run_migrations()?;
        Ok(store)
    }

    pub fn open_in_memory() -> MvResult<Self> {
        let conn = Connection::open_in_memory()
            .map_err(|e| MvError::Storage(format!("failed to open in-memory sqlite: {e}")))?;

        conn.execute_batch("PRAGMA foreign_keys=ON;")
            .map_err(|e| MvError::Storage(format!("pragma error: {e}")))?;

        let store = Self {
            conn: Mutex::new(conn),
        };
        store.run_migrations()?;
        Ok(store)
    }

    fn run_migrations(&self) -> MvResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let migration_001 = include_str!("../../../migrations/001_initial.sql");
        conn.execute_batch(migration_001)
            .map_err(|e| MvError::Migration(format!("migration 001 failed: {e}")))?;

        let migration_003 = include_str!("../../../migrations/003_agentic.sql");
        conn.execute_batch(migration_003)
            .map_err(|e| MvError::Migration(format!("migration 003 failed: {e}")))?;

        let migration_004 = include_str!("../../../migrations/004_exchange.sql");
        conn.execute_batch(migration_004)
            .map_err(|e| MvError::Migration(format!("migration 004 failed: {e}")))?;

        let migration_005 = include_str!("../../../migrations/005_relay_safeguards.sql");
        conn.execute_batch(migration_005)
            .map_err(|e| MvError::Migration(format!("migration 005 failed: {e}")))?;

        let migration_006 = include_str!("../../../migrations/006_feedback.sql");
        conn.execute_batch(migration_006)
            .map_err(|e| MvError::Migration(format!("migration 006 failed: {e}")))?;

        let migration_007 = include_str!("../../../migrations/007_autonomy.sql");
        conn.execute_batch(migration_007)
            .map_err(|e| MvError::Migration(format!("migration 007 failed: {e}")))?;

        let migration_008 = include_str!("../../../migrations/008_relay.sql");
        conn.execute_batch(migration_008)
            .map_err(|e| MvError::Migration(format!("migration 008 failed: {e}")))?;

        Ok(())
    }

    fn row_to_node(row: &rusqlite::Row<'_>) -> rusqlite::Result<KnowledgeNode> {
        let id_str: String = row.get(0)?;
        let kind_str: String = row.get(1)?;
        let title: Option<String> = row.get(2)?;
        let content: String = row.get(3)?;
        let source: Option<String> = row.get(4)?;
        let namespace: String = row.get(5)?;
        let importance: f64 = row.get(6)?;
        let created_at: String = row.get(7)?;
        let updated_at: String = row.get(8)?;
        let last_accessed_at: String = row.get(9)?;
        let access_count: u64 = row.get(10)?;
        let version: u32 = row.get(11)?;
        let expires_at: Option<String> = row.get(12)?;
        let metadata_json: Option<String> = row.get(13)?;

        let id = parse_uuid_str(0, &id_str)?;
        let kind: NodeKind = kind_str.parse().map_err(|err: String| {
            rusqlite::Error::FromSqlConversionFailure(
                1,
                Type::Text,
                Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, err)),
            )
        })?;
        let metadata = parse_metadata_json(metadata_json)?;

        Ok(KnowledgeNode {
            id,
            kind,
            title,
            content,
            source,
            namespace,
            tags: Vec::new(), // loaded separately
            importance,
            temporal: TemporalMeta {
                created_at: parse_dt_strict(7, &created_at)?,
                updated_at: parse_dt_strict(8, &updated_at)?,
                last_accessed_at: parse_dt_strict(9, &last_accessed_at)?,
                access_count,
                version,
                expires_at: parse_optional_dt_strict(12, expires_at)?,
            },
            metadata,
        })
    }

    fn load_tags(conn: &Connection, node_id: Uuid) -> MvResult<Vec<String>> {
        let mut stmt = conn
            .prepare("SELECT tag FROM node_tags WHERE node_id = ?1 ORDER BY tag")
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut tags = Vec::new();
        let rows = stmt
            .query_map(params![node_id.to_string()], |row| row.get(0))
            .map_err(|e| MvError::Storage(e.to_string()))?;

        for row in rows {
            tags.push(row.map_err(|e| MvError::Storage(e.to_string()))?);
        }

        Ok(tags)
    }

    fn save_tags(conn: &Connection, node_id: Uuid, tags: &[String]) -> MvResult<()> {
        conn.execute(
            "DELETE FROM node_tags WHERE node_id = ?1",
            params![node_id.to_string()],
        )
        .map_err(|e| MvError::Storage(e.to_string()))?;
        let mut stmt = conn
            .prepare("INSERT INTO node_tags (node_id, tag) VALUES (?1, ?2)")
            .map_err(|e| MvError::Storage(e.to_string()))?;
        for tag in tags {
            stmt.execute(params![node_id.to_string(), tag])
                .map_err(|e| MvError::Storage(e.to_string()))?;
        }
        Ok(())
    }

    fn log_change(
        conn: &Connection,
        node_id: Uuid,
        op: ChangeOp,
        diff: Option<&serde_json::Value>,
    ) -> MvResult<()> {
        let diff_str = diff.map(|d| serde_json::to_string(d).unwrap_or_default());
        conn.execute(
            "INSERT INTO changelog (node_id, operation, diff_json, timestamp) VALUES (?1, ?2, ?3, ?4)",
            params![
                node_id.to_string(),
                op.as_str(),
                diff_str,
                Utc::now().to_rfc3339(),
            ],
        )
        .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(())
    }
}

fn parse_uuid_str(column: usize, s: &str) -> rusqlite::Result<Uuid> {
    Uuid::parse_str(s)
        .map_err(|err| rusqlite::Error::FromSqlConversionFailure(column, Type::Text, Box::new(err)))
}

fn parse_dt_strict(column: usize, s: &str) -> rusqlite::Result<chrono::DateTime<Utc>> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|err| rusqlite::Error::FromSqlConversionFailure(column, Type::Text, Box::new(err)))
}

fn parse_optional_dt_strict(
    column: usize,
    s: Option<String>,
) -> rusqlite::Result<Option<chrono::DateTime<Utc>>> {
    match s {
        Some(value) => parse_dt_strict(column, &value).map(Some),
        None => Ok(None),
    }
}

fn parse_metadata_json(
    metadata_json: Option<String>,
) -> rusqlite::Result<std::collections::HashMap<String, serde_json::Value>> {
    match metadata_json {
        Some(raw) => serde_json::from_str(&raw).map_err(|err| {
            rusqlite::Error::FromSqlConversionFailure(13, Type::Text, Box::new(err))
        }),
        None => Ok(Default::default()),
    }
}

#[async_trait]
impl NodeStore for SqliteNodeStore {
    async fn insert(&self, node: &KnowledgeNode) -> MvResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let metadata_json = serde_json::to_string(&node.metadata)?;

        conn.execute(
            "INSERT INTO knowledge_nodes (id, kind, title, content, source, namespace, importance,
             created_at, updated_at, last_accessed_at, access_count, version, expires_at, metadata_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                node.id.to_string(),
                node.kind.as_str(),
                node.title,
                node.content,
                node.source,
                node.namespace,
                node.importance,
                node.temporal.created_at.to_rfc3339(),
                node.temporal.updated_at.to_rfc3339(),
                node.temporal.last_accessed_at.to_rfc3339(),
                node.temporal.access_count,
                node.temporal.version,
                node.temporal.expires_at.map(|dt| dt.to_rfc3339()),
                metadata_json,
            ],
        )
        .map_err(|e| MvError::Storage(format!("insert failed: {e}")))?;

        Self::save_tags(&conn, node.id, &node.tags)?;
        Self::log_change(&conn, node.id, ChangeOp::Create, None)?;
        Ok(())
    }

    async fn get(&self, id: Uuid) -> MvResult<Option<KnowledgeNode>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, kind, title, content, source, namespace, importance,
                 created_at, updated_at, last_accessed_at, access_count, version,
                 expires_at, metadata_json FROM knowledge_nodes WHERE id = ?1",
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let node = stmt
            .query_row(params![id.to_string()], Self::row_to_node)
            .optional()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        if let Some(mut node) = node {
            node.tags = Self::load_tags(&conn, node.id)?;
            Ok(Some(node))
        } else {
            Ok(None)
        }
    }

    async fn update(&self, node: &KnowledgeNode) -> MvResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let metadata_json = serde_json::to_string(&node.metadata)?;

        let rows = conn
            .execute(
                "UPDATE knowledge_nodes SET kind = ?2, title = ?3, content = ?4, source = ?5,
                 namespace = ?6, importance = ?7, updated_at = ?8, last_accessed_at = ?9,
                 access_count = ?10, version = ?11, expires_at = ?12, metadata_json = ?13
                 WHERE id = ?1",
                params![
                    node.id.to_string(),
                    node.kind.as_str(),
                    node.title,
                    node.content,
                    node.source,
                    node.namespace,
                    node.importance,
                    node.temporal.updated_at.to_rfc3339(),
                    node.temporal.last_accessed_at.to_rfc3339(),
                    node.temporal.access_count,
                    node.temporal.version,
                    node.temporal.expires_at.map(|dt| dt.to_rfc3339()),
                    metadata_json,
                ],
            )
            .map_err(|e| MvError::Storage(format!("update failed: {e}")))?;

        if rows == 0 {
            return Err(MvError::NodeNotFound(node.id));
        }

        Self::save_tags(&conn, node.id, &node.tags)?;
        Self::log_change(&conn, node.id, ChangeOp::Update, None)?;
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> MvResult<bool> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        Self::log_change(&conn, id, ChangeOp::Delete, None)?;
        let rows = conn
            .execute(
                "DELETE FROM knowledge_nodes WHERE id = ?1",
                params![id.to_string()],
            )
            .map_err(|e| MvError::Storage(format!("delete failed: {e}")))?;
        Ok(rows > 0)
    }

    async fn list(
        &self,
        filters: &QueryFilters,
        limit: usize,
        offset: usize,
    ) -> MvResult<Vec<KnowledgeNode>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut sql = String::from(
            "SELECT id, kind, title, content, source, namespace, importance,
             created_at, updated_at, last_accessed_at, access_count, version,
             expires_at, metadata_json FROM knowledge_nodes WHERE 1=1",
        );
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        let mut param_idx = 1;

        if let Some(ref ns) = filters.namespace {
            sql.push_str(&format!(" AND namespace = ?{param_idx}"));
            param_values.push(Box::new(ns.clone()));
            param_idx += 1;
        }

        if let Some(ref kinds) = filters.kinds {
            if !kinds.is_empty() {
                let placeholders: Vec<String> = kinds
                    .iter()
                    .map(|_| {
                        let p = format!("?{param_idx}");
                        param_idx += 1;
                        p
                    })
                    .collect();
                sql.push_str(&format!(" AND kind IN ({})", placeholders.join(",")));
                for k in kinds {
                    param_values.push(Box::new(k.as_str().to_string()));
                }
            }
        }

        if let Some(min_imp) = filters.min_importance {
            sql.push_str(&format!(" AND importance >= ?{param_idx}"));
            param_values.push(Box::new(min_imp));
            param_idx += 1;
        }

        if let Some(ref after) = filters.created_after {
            sql.push_str(&format!(" AND created_at >= ?{param_idx}"));
            param_values.push(Box::new(after.to_rfc3339()));
            param_idx += 1;
        }

        if let Some(ref before) = filters.created_before {
            sql.push_str(&format!(" AND created_at <= ?{param_idx}"));
            param_values.push(Box::new(before.to_rfc3339()));
            param_idx += 1;
        }

        if let Some(ref filter_tags) = filters.tags {
            if !filter_tags.is_empty() {
                let placeholders: Vec<String> = filter_tags
                    .iter()
                    .map(|_| {
                        let p = format!("?{param_idx}");
                        param_idx += 1;
                        p
                    })
                    .collect();

                sql.push_str(&format!(
                    " AND EXISTS (SELECT 1 FROM node_tags nt WHERE nt.node_id = knowledge_nodes.id AND nt.tag IN ({}))",
                    placeholders.join(",")
                ));

                for tag in filter_tags {
                    param_values.push(Box::new(tag.clone()));
                }
            }
        }

        sql.push_str(&format!(
            " ORDER BY updated_at DESC LIMIT ?{param_idx} OFFSET ?{}",
            param_idx + 1
        ));
        param_values.push(Box::new(limit as i64));
        param_values.push(Box::new(offset as i64));

        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let rows = stmt
            .query_map(params_refs.as_slice(), Self::row_to_node)
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut nodes = Vec::new();
        for row in rows {
            let mut node = row.map_err(|e| MvError::Storage(e.to_string()))?;
            node.tags = Self::load_tags(&conn, node.id)?;
            nodes.push(node);
        }

        Ok(nodes)
    }

    async fn touch(&self, id: Uuid) -> MvResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let now = Utc::now().to_rfc3339();
        let rows = conn
            .execute(
                "UPDATE knowledge_nodes SET last_accessed_at = ?2, access_count = access_count + 1 WHERE id = ?1",
                params![id.to_string(), now],
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;
        if rows == 0 {
            return Err(MvError::NodeNotFound(id));
        }
        Ok(())
    }

    async fn count(&self, filters: &QueryFilters) -> MvResult<usize> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut sql = String::from("SELECT COUNT(*) FROM knowledge_nodes WHERE 1=1");
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        let mut param_idx = 1;

        if let Some(ref ns) = filters.namespace {
            sql.push_str(&format!(" AND namespace = ?{param_idx}"));
            param_values.push(Box::new(ns.clone()));
            param_idx += 1;
        }

        if let Some(ref kinds) = filters.kinds {
            if !kinds.is_empty() {
                let placeholders: Vec<String> = kinds
                    .iter()
                    .map(|_| {
                        let p = format!("?{param_idx}");
                        param_idx += 1;
                        p
                    })
                    .collect();
                sql.push_str(&format!(" AND kind IN ({})", placeholders.join(",")));
                for k in kinds {
                    param_values.push(Box::new(k.as_str().to_string()));
                }
            }
        }
        if let Some(min_imp) = filters.min_importance {
            sql.push_str(&format!(" AND importance >= ?{param_idx}"));
            param_values.push(Box::new(min_imp));
            param_idx += 1;
        }

        if let Some(ref after) = filters.created_after {
            sql.push_str(&format!(" AND created_at >= ?{param_idx}"));
            param_values.push(Box::new(after.to_rfc3339()));
            param_idx += 1;
        }

        if let Some(ref before) = filters.created_before {
            sql.push_str(&format!(" AND created_at <= ?{param_idx}"));
            param_values.push(Box::new(before.to_rfc3339()));
            param_idx += 1;
        }

        if let Some(ref filter_tags) = filters.tags {
            if !filter_tags.is_empty() {
                let placeholders: Vec<String> = filter_tags
                    .iter()
                    .map(|_| {
                        let p = format!("?{param_idx}");
                        param_idx += 1;
                        p
                    })
                    .collect();

                sql.push_str(&format!(
                    " AND EXISTS (SELECT 1 FROM node_tags nt WHERE nt.node_id = knowledge_nodes.id AND nt.tag IN ({}))",
                    placeholders.join(",")
                ));

                for tag in filter_tags {
                    param_values.push(Box::new(tag.clone()));
                }
            }
        }

        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();

        let count: usize = conn
            .query_row(&sql, params_refs.as_slice(), |row| row.get(0))
            .map_err(|e| MvError::Storage(e.to_string()))?;

        Ok(count)
    }
}

impl SqliteNodeStore {
    /// Find a node by its `source` field (exact match).
    /// Used for dedup during imports (e.g. Obsidian vault import).
    pub async fn find_by_source(&self, source: &str) -> MvResult<Option<KnowledgeNode>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut stmt = conn
            .prepare(
                "SELECT id, kind, title, content, source, namespace, importance,
                 created_at, updated_at, last_accessed_at, access_count, version,
                 expires_at, metadata_json FROM knowledge_nodes WHERE source = ?1 LIMIT 1",
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let row = stmt
            .query_row(params![source], Self::row_to_node)
            .optional()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        match row {
            Some(mut node) => {
                node.tags = Self::load_tags(&conn, node.id)?;
                Ok(Some(node))
            }
            None => Ok(None),
        }
    }

    pub async fn insert_permission_template(&self, template: &PermissionTemplate) -> MvResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let scope_tags_json = serde_json::to_string(&template.scope_tags)?;
        let allow_kinds_json = serde_json::to_string(
            &template
                .allow_kinds
                .iter()
                .map(|kind| kind.as_str())
                .collect::<Vec<_>>(),
        )?;
        let allow_actions_json = serde_json::to_string(&template.allow_actions)?;

        conn.execute(
            "INSERT INTO permission_templates (id, name, description, tier, scope_namespace, scope_tags_json, allow_kinds_json, allow_actions_json, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                template.id.to_string(),
                template.name,
                template.description,
                template.tier.as_str(),
                template.scope_namespace,
                scope_tags_json,
                allow_kinds_json,
                allow_actions_json,
                template.created_at.to_rfc3339(),
                template.updated_at.to_rfc3339(),
            ],
        )
        .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(())
    }

    pub async fn update_permission_template(&self, template: &PermissionTemplate) -> MvResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let scope_tags_json = serde_json::to_string(&template.scope_tags)?;
        let allow_kinds_json = serde_json::to_string(
            &template
                .allow_kinds
                .iter()
                .map(|kind| kind.as_str())
                .collect::<Vec<_>>(),
        )?;
        let allow_actions_json = serde_json::to_string(&template.allow_actions)?;

        conn.execute(
            "UPDATE permission_templates SET name = ?2, description = ?3, tier = ?4, scope_namespace = ?5, scope_tags_json = ?6, allow_kinds_json = ?7, allow_actions_json = ?8, updated_at = ?9 WHERE id = ?1",
            params![
                template.id.to_string(),
                template.name,
                template.description,
                template.tier.as_str(),
                template.scope_namespace,
                scope_tags_json,
                allow_kinds_json,
                allow_actions_json,
                template.updated_at.to_rfc3339(),
            ],
        )
        .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(())
    }

    pub async fn get_permission_template(&self, id: Uuid) -> MvResult<Option<PermissionTemplate>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut stmt = conn
            .prepare(
                "SELECT id, name, description, tier, scope_namespace, scope_tags_json, allow_kinds_json, allow_actions_json, created_at, updated_at FROM permission_templates WHERE id = ?1",
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let row = stmt
            .query_row(params![id.to_string()], |row| {
                row_to_permission_template(row)
            })
            .optional()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        Ok(row)
    }

    pub async fn get_permission_template_by_name(
        &self,
        name: &str,
    ) -> MvResult<Option<PermissionTemplate>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut stmt = conn
            .prepare(
                "SELECT id, name, description, tier, scope_namespace, scope_tags_json, allow_kinds_json, allow_actions_json, created_at, updated_at FROM permission_templates WHERE lower(name) = lower(?1)",
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let row = stmt
            .query_row(params![name], |row| row_to_permission_template(row))
            .optional()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        Ok(row)
    }

    pub async fn list_permission_templates(
        &self,
        limit: usize,
        offset: usize,
    ) -> MvResult<Vec<PermissionTemplate>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut stmt = conn
            .prepare(
                "SELECT id, name, description, tier, scope_namespace, scope_tags_json, allow_kinds_json, allow_actions_json, created_at, updated_at FROM permission_templates ORDER BY created_at DESC LIMIT ?1 OFFSET ?2",
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let rows = stmt
            .query_map(params![limit as i64, offset as i64], |row| {
                row_to_permission_template(row)
            })
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut templates = Vec::new();
        for row in rows {
            templates.push(row.map_err(|e| MvError::Storage(e.to_string()))?);
        }

        Ok(templates)
    }

    pub async fn delete_permission_template(&self, id: Uuid) -> MvResult<bool> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let affected = conn
            .execute(
                "DELETE FROM permission_templates WHERE id = ?1",
                params![id.to_string()],
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;

        Ok(affected > 0)
    }

    pub async fn insert_access_key(&self, key: &AccessKey) -> MvResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        conn.execute(
            "INSERT INTO access_keys (id, name, template_id, key_hash, created_at, last_used_at, expires_at, revoked_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                key.id.to_string(),
                key.name,
                key.template_id.to_string(),
                key.key_hash,
                key.created_at.to_rfc3339(),
                key.last_used_at.map(|dt| dt.to_rfc3339()),
                key.expires_at.map(|dt| dt.to_rfc3339()),
                key.revoked_at.map(|dt| dt.to_rfc3339()),
            ],
        )
        .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(())
    }

    pub async fn list_access_keys(&self) -> MvResult<Vec<AccessKey>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut stmt = conn
            .prepare(
                "SELECT id, name, template_id, key_hash, created_at, last_used_at, expires_at, revoked_at FROM access_keys ORDER BY created_at DESC",
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| row_to_access_key(row))
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut keys = Vec::new();
        for row in rows {
            keys.push(row.map_err(|e| MvError::Storage(e.to_string()))?);
        }

        Ok(keys)
    }

    pub async fn get_access_key(&self, id: Uuid) -> MvResult<Option<AccessKey>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut stmt = conn
            .prepare(
                "SELECT id, name, template_id, key_hash, created_at, last_used_at, expires_at, revoked_at FROM access_keys WHERE id = ?1",
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let row = stmt
            .query_row(params![id.to_string()], |row| row_to_access_key(row))
            .optional()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        Ok(row)
    }

    pub async fn get_access_key_by_hash(&self, key_hash: &str) -> MvResult<Option<AccessKey>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut stmt = conn
            .prepare(
                "SELECT id, name, template_id, key_hash, created_at, last_used_at, expires_at, revoked_at FROM access_keys WHERE key_hash = ?1",
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let row = stmt
            .query_row(params![key_hash], |row| row_to_access_key(row))
            .optional()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        Ok(row)
    }

    pub async fn update_access_key_last_used(
        &self,
        id: Uuid,
        when: chrono::DateTime<chrono::Utc>,
    ) -> MvResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        conn.execute(
            "UPDATE access_keys SET last_used_at = ?2 WHERE id = ?1",
            params![id.to_string(), when.to_rfc3339()],
        )
        .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(())
    }

    pub async fn revoke_access_key(
        &self,
        id: Uuid,
        when: chrono::DateTime<chrono::Utc>,
    ) -> MvResult<bool> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let affected = conn
            .execute(
                "UPDATE access_keys SET revoked_at = ?2 WHERE id = ?1",
                params![id.to_string(), when.to_rfc3339()],
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;

        Ok(affected > 0)
    }
}

fn row_to_permission_template(row: &rusqlite::Row<'_>) -> rusqlite::Result<PermissionTemplate> {
    let id_str: String = row.get(0)?;
    let name: String = row.get(1)?;
    let description: Option<String> = row.get(2)?;
    let tier_str: String = row.get(3)?;
    let scope_namespace: Option<String> = row.get(4)?;
    let scope_tags_json: Option<String> = row.get(5)?;
    let allow_kinds_json: Option<String> = row.get(6)?;
    let allow_actions_json: Option<String> = row.get(7)?;
    let created_at: String = row.get(8)?;
    let updated_at: String = row.get(9)?;

    let id = parse_uuid_str(0, &id_str)?;
    let tier: PermissionTier = tier_str.parse().map_err(|err: String| {
        rusqlite::Error::FromSqlConversionFailure(
            3,
            Type::Text,
            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, err)),
        )
    })?;

    let scope_tags = parse_json_vec::<String>(5, scope_tags_json)?;
    let allow_kinds_raw = parse_json_vec::<String>(6, allow_kinds_json)?;
    let mut allow_kinds = Vec::new();
    for raw in allow_kinds_raw {
        let parsed: NodeKind = raw.parse().map_err(|err: String| {
            rusqlite::Error::FromSqlConversionFailure(
                6,
                Type::Text,
                Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, err)),
            )
        })?;
        allow_kinds.push(parsed);
    }
    let allow_actions = parse_json_vec::<String>(7, allow_actions_json)?;

    Ok(PermissionTemplate {
        id,
        name,
        description,
        tier,
        scope_namespace,
        scope_tags,
        allow_kinds,
        allow_actions,
        created_at: parse_dt_strict(8, &created_at)?,
        updated_at: parse_dt_strict(9, &updated_at)?,
    })
}

fn row_to_access_key(row: &rusqlite::Row<'_>) -> rusqlite::Result<AccessKey> {
    let id_str: String = row.get(0)?;
    let name: Option<String> = row.get(1)?;
    let template_id_str: String = row.get(2)?;
    let key_hash: String = row.get(3)?;
    let created_at: String = row.get(4)?;
    let last_used_at: Option<String> = row.get(5)?;
    let expires_at: Option<String> = row.get(6)?;
    let revoked_at: Option<String> = row.get(7)?;

    Ok(AccessKey {
        id: parse_uuid_str(0, &id_str)?,
        name,
        template_id: parse_uuid_str(2, &template_id_str)?,
        key_hash,
        created_at: parse_dt_strict(4, &created_at)?,
        last_used_at: parse_optional_dt_strict(5, last_used_at)?,
        expires_at: parse_optional_dt_strict(6, expires_at)?,
        revoked_at: parse_optional_dt_strict(7, revoked_at)?,
    })
}

fn parse_json_vec<T: serde::de::DeserializeOwned>(
    column: usize,
    raw: Option<String>,
) -> rusqlite::Result<Vec<T>> {
    match raw {
        Some(value) => serde_json::from_str(&value).map_err(|err| {
            rusqlite::Error::FromSqlConversionFailure(column, Type::Text, Box::new(err))
        }),
        None => Ok(Vec::new()),
    }
}

// ---------------------------------------------------------------------------
// AgenticStore Implementation
// ---------------------------------------------------------------------------

use mv_core::{
    AgenticStore, CapturedIntent, ChronicleEntry, InsightType, IntentStatus, IntentType,
    ProactiveInsight,
};

#[async_trait]
impl AgenticStore for SqliteNodeStore {
    async fn log_intent(&self, intent: &CapturedIntent) -> MvResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let params_json = serde_json::to_string(&intent.parameters)?;

        conn.execute(
            "INSERT INTO captured_intents (id, node_id, intent_type, confidence, parameters, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                intent.id.to_string(),
                intent.node_id.to_string(),
                intent.intent_type.as_str(),
                intent.confidence as f64,
                params_json,
                intent.status.as_str(),
                intent.created_at.to_rfc3339(),
                intent.updated_at.map(|dt| dt.to_rfc3339()),
            ],
        )
        .map_err(|e| MvError::Storage(format!("insert intent failed: {e}")))?;
        Ok(())
    }

    async fn get_intent(&self, id: Uuid) -> MvResult<Option<CapturedIntent>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, node_id, intent_type, confidence, parameters, status, created_at, updated_at
                 FROM captured_intents WHERE id = ?1",
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let result = stmt
            .query_row(params![id.to_string()], row_to_captured_intent)
            .optional()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(result)
    }

    async fn list_intents(
        &self,
        node_id: Option<Uuid>,
        status: Option<IntentStatus>,
        limit: usize,
        offset: usize,
    ) -> MvResult<Vec<CapturedIntent>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut sql = String::from(
            "SELECT id, node_id, intent_type, confidence, parameters, status, created_at, updated_at
             FROM captured_intents WHERE 1=1",
        );
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        let mut param_idx = 1;

        if let Some(nid) = node_id {
            sql.push_str(&format!(" AND node_id = ?{param_idx}"));
            param_values.push(Box::new(nid.to_string()));
            param_idx += 1;
        }

        if let Some(st) = status {
            sql.push_str(&format!(" AND status = ?{param_idx}"));
            param_values.push(Box::new(st.as_str().to_string()));
            param_idx += 1;
        }

        sql.push_str(&format!(
            " ORDER BY created_at DESC LIMIT ?{param_idx} OFFSET ?{}",
            param_idx + 1
        ));
        param_values.push(Box::new(limit as i64));
        param_values.push(Box::new(offset as i64));

        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let rows = stmt
            .query_map(params_refs.as_slice(), row_to_captured_intent)
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut intents = Vec::new();
        for row in rows {
            intents.push(row.map_err(|e| MvError::Storage(e.to_string()))?);
        }
        Ok(intents)
    }

    async fn update_intent_status(&self, id: Uuid, status: IntentStatus) -> MvResult<bool> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let now = Utc::now().to_rfc3339();
        let affected = conn
            .execute(
                "UPDATE captured_intents SET status = ?2, updated_at = ?3 WHERE id = ?1",
                params![id.to_string(), status.as_str(), now],
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(affected > 0)
    }

    async fn log_insight(&self, insight: &ProactiveInsight) -> MvResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let related_ids_json = serde_json::to_string(&insight.related_node_ids)?;

        conn.execute(
            "INSERT INTO proactive_insights (id, title, content, insight_type, related_node_ids, importance, created_at, dismissed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                insight.id.to_string(),
                insight.title,
                insight.content,
                insight.insight_type.as_str(),
                related_ids_json,
                insight.importance as f64,
                insight.created_at.to_rfc3339(),
                insight.dismissed_at.map(|dt| dt.to_rfc3339()),
            ],
        )
        .map_err(|e| MvError::Storage(format!("insert insight failed: {e}")))?;
        Ok(())
    }

    async fn list_insights(&self, limit: usize, offset: usize) -> MvResult<Vec<ProactiveInsight>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, title, content, insight_type, related_node_ids, importance, created_at, dismissed_at
                 FROM proactive_insights
                 WHERE dismissed_at IS NULL
                 ORDER BY created_at DESC
                 LIMIT ?1 OFFSET ?2",
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let rows = stmt
            .query_map(
                params![limit as i64, offset as i64],
                row_to_proactive_insight,
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut insights = Vec::new();
        for row in rows {
            insights.push(row.map_err(|e| MvError::Storage(e.to_string()))?);
        }
        Ok(insights)
    }

    async fn delete_insight(&self, id: Uuid) -> MvResult<bool> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let now = Utc::now().to_rfc3339();
        let affected = conn
            .execute(
                "UPDATE proactive_insights SET dismissed_at = ?2 WHERE id = ?1 AND dismissed_at IS NULL",
                params![id.to_string(), now],
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(affected > 0)
    }

    async fn log_chronicle(&self, entry: &ChronicleEntry) -> MvResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        conn.execute(
            "INSERT INTO chronicle_entries (id, node_id, step_name, logic, input_snapshot, output_snapshot, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                entry.id.to_string(),
                entry.node_id.map(|id| id.to_string()),
                entry.step_name,
                entry.logic,
                entry.input_snapshot,
                entry.output_snapshot,
                entry.timestamp.to_rfc3339(),
            ],
        )
        .map_err(|e| MvError::Storage(format!("insert chronicle failed: {e}")))?;
        Ok(())
    }

    async fn list_chronicles(
        &self,
        node_id: Option<Uuid>,
        limit: usize,
        offset: usize,
    ) -> MvResult<Vec<ChronicleEntry>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let (sql, params_box): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = if let Some(nid) =
            node_id
        {
            (
                "SELECT id, node_id, step_name, logic, input_snapshot, output_snapshot, timestamp
                 FROM chronicle_entries WHERE node_id = ?1 ORDER BY timestamp DESC LIMIT ?2 OFFSET ?3".to_string(),
                vec![
                    Box::new(nid.to_string()),
                    Box::new(limit as i64),
                    Box::new(offset as i64),
                ],
            )
        } else {
            (
                "SELECT id, node_id, step_name, logic, input_snapshot, output_snapshot, timestamp
                 FROM chronicle_entries ORDER BY timestamp DESC LIMIT ?1 OFFSET ?2"
                    .to_string(),
                vec![Box::new(limit as i64), Box::new(offset as i64)],
            )
        };

        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            params_box.iter().map(|p| p.as_ref()).collect();
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let rows = stmt
            .query_map(params_refs.as_slice(), row_to_chronicle_entry)
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut entries = Vec::new();
        for row in rows {
            entries.push(row.map_err(|e| MvError::Storage(e.to_string()))?);
        }
        Ok(entries)
    }
}

fn row_to_captured_intent(row: &rusqlite::Row<'_>) -> rusqlite::Result<CapturedIntent> {
    let id_str: String = row.get(0)?;
    let node_id_str: String = row.get(1)?;
    let intent_type_str: String = row.get(2)?;
    let confidence: f64 = row.get(3)?;
    let params_json: Option<String> = row.get(4)?;
    let status_str: String = row.get(5)?;
    let created_at: String = row.get(6)?;
    let updated_at: Option<String> = row.get(7)?;

    let id = parse_uuid_str(0, &id_str)?;
    let node_id = parse_uuid_str(1, &node_id_str)?;
    let intent_type: IntentType = intent_type_str
        .parse()
        .unwrap_or(IntentType::Custom(intent_type_str));
    let status: IntentStatus = status_str.parse().map_err(|e: String| {
        rusqlite::Error::FromSqlConversionFailure(
            5,
            Type::Text,
            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
        )
    })?;
    let parameters: serde_json::Value = params_json
        .map(|s| serde_json::from_str(&s).unwrap_or(serde_json::Value::Null))
        .unwrap_or(serde_json::Value::Null);

    Ok(CapturedIntent {
        id,
        node_id,
        intent_type,
        confidence: confidence as f32,
        parameters,
        status,
        created_at: parse_dt_strict(6, &created_at)?,
        updated_at: parse_optional_dt_strict(7, updated_at)?,
    })
}

fn row_to_proactive_insight(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProactiveInsight> {
    let id_str: String = row.get(0)?;
    let title: String = row.get(1)?;
    let content: String = row.get(2)?;
    let insight_type_str: String = row.get(3)?;
    let related_ids_json: Option<String> = row.get(4)?;
    let importance: f64 = row.get(5)?;
    let created_at: String = row.get(6)?;
    let dismissed_at: Option<String> = row.get(7)?;

    let id = parse_uuid_str(0, &id_str)?;
    let insight_type: InsightType = insight_type_str.parse().map_err(|e: String| {
        rusqlite::Error::FromSqlConversionFailure(
            3,
            Type::Text,
            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
        )
    })?;

    let related_node_ids: Vec<Uuid> = related_ids_json
        .map(|s| {
            let strs: Vec<String> = serde_json::from_str(&s).unwrap_or_default();
            strs.iter()
                .filter_map(|id_str| Uuid::parse_str(id_str).ok())
                .collect()
        })
        .unwrap_or_default();

    Ok(ProactiveInsight {
        id,
        title,
        content,
        insight_type,
        related_node_ids,
        importance: importance as f32,
        created_at: parse_dt_strict(6, &created_at)?,
        dismissed_at: parse_optional_dt_strict(7, dismissed_at)?,
    })
}

fn row_to_chronicle_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<ChronicleEntry> {
    let id_str: String = row.get(0)?;
    let node_id_str: Option<String> = row.get(1)?;
    let step_name: String = row.get(2)?;
    let logic: String = row.get(3)?;
    let input_snapshot: Option<String> = row.get(4)?;
    let output_snapshot: Option<String> = row.get(5)?;
    let timestamp: String = row.get(6)?;

    let id = parse_uuid_str(0, &id_str)?;
    let node_id = node_id_str.map(|s| Uuid::parse_str(&s).ok()).flatten();

    Ok(ChronicleEntry {
        id,
        node_id,
        step_name,
        logic,
        input_snapshot,
        output_snapshot,
        timestamp: parse_dt_strict(6, &timestamp)?,
    })
}

// ---------------------------------------------------------------------------
// ExchangeStore Implementation
// ---------------------------------------------------------------------------

use chrono::DateTime;
use mv_core::{ExchangeStore, Proposal, ProposalAction, ProposalSender, ProposalState};

fn row_to_proposal(row: &rusqlite::Row<'_>) -> rusqlite::Result<Proposal> {
    let id_str: String = row.get(0)?;
    let node_id_str: Option<String> = row.get(1)?;
    let target_node_id_str: Option<String> = row.get(2)?;
    let sender_str: String = row.get(3)?;
    let action_str: String = row.get(4)?;
    let state_str: String = row.get(5)?;
    let confidence: f64 = row.get(6)?;
    let diff_preview: Option<String> = row.get(7)?;
    let payload_json: Option<String> = row.get(8)?;
    let created_at: String = row.get(9)?;
    let updated_at: Option<String> = row.get(10)?;
    let resolved_at: Option<String> = row.get(11)?;

    let id = parse_uuid_str(0, &id_str)?;
    let node_id = node_id_str.map(|s| Uuid::parse_str(&s).ok()).flatten();
    let target_node_id = target_node_id_str
        .map(|s| Uuid::parse_str(&s).ok())
        .flatten();

    let sender: ProposalSender = sender_str.parse().map_err(|e: String| {
        rusqlite::Error::FromSqlConversionFailure(
            3,
            Type::Text,
            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
        )
    })?;
    let action: ProposalAction = action_str
        .parse()
        .unwrap_or(ProposalAction::Custom(action_str));
    let state: ProposalState = state_str.parse().map_err(|e: String| {
        rusqlite::Error::FromSqlConversionFailure(
            5,
            Type::Text,
            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
        )
    })?;

    let payload: std::collections::HashMap<String, serde_json::Value> = payload_json
        .map(|s| serde_json::from_str(&s).unwrap_or_default())
        .unwrap_or_default();

    Ok(Proposal {
        id,
        node_id,
        target_node_id,
        sender,
        action,
        state,
        confidence: confidence as f32,
        diff_preview,
        payload,
        created_at: parse_dt_strict(9, &created_at)?,
        updated_at: parse_optional_dt_strict(10, updated_at)?,
        resolved_at: parse_optional_dt_strict(11, resolved_at)?,
    })
}

#[async_trait]
impl ExchangeStore for SqliteNodeStore {
    async fn submit_proposal(&self, proposal: &Proposal) -> MvResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let payload_json = serde_json::to_string(&proposal.payload)?;

        conn.execute(
            "INSERT INTO proposals (id, node_id, target_node_id, sender, action, state, confidence, diff_preview, payload, created_at, updated_at, resolved_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                proposal.id.to_string(),
                proposal.node_id.map(|id| id.to_string()),
                proposal.target_node_id.map(|id| id.to_string()),
                proposal.sender.as_str(),
                proposal.action.as_str(),
                proposal.state.as_str(),
                proposal.confidence as f64,
                proposal.diff_preview,
                payload_json,
                proposal.created_at.to_rfc3339(),
                proposal.updated_at.map(|dt| dt.to_rfc3339()),
                proposal.resolved_at.map(|dt| dt.to_rfc3339()),
            ],
        )
        .map_err(|e| MvError::Storage(format!("insert proposal failed: {e}")))?;
        Ok(())
    }

    async fn get_proposal(&self, id: Uuid) -> MvResult<Option<Proposal>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, node_id, target_node_id, sender, action, state, confidence, diff_preview, payload, created_at, updated_at, resolved_at
                 FROM proposals WHERE id = ?1",
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let result = stmt
            .query_row(params![id.to_string()], row_to_proposal)
            .optional()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(result)
    }

    async fn list_proposals(
        &self,
        state: Option<ProposalState>,
        limit: usize,
        offset: usize,
    ) -> MvResult<Vec<Proposal>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut sql = String::from(
            "SELECT id, node_id, target_node_id, sender, action, state, confidence, diff_preview, payload, created_at, updated_at, resolved_at
             FROM proposals WHERE 1=1",
        );
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        let mut param_idx = 1;

        if let Some(st) = state {
            sql.push_str(&format!(" AND state = ?{param_idx}"));
            param_values.push(Box::new(st.as_str().to_string()));
            param_idx += 1;
        }

        sql.push_str(&format!(
            " ORDER BY created_at DESC LIMIT ?{param_idx} OFFSET ?{}",
            param_idx + 1
        ));
        param_values.push(Box::new(limit as i64));
        param_values.push(Box::new(offset as i64));

        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let rows = stmt
            .query_map(params_refs.as_slice(), row_to_proposal)
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut proposals = Vec::new();
        for row in rows {
            proposals.push(row.map_err(|e| MvError::Storage(e.to_string()))?);
        }
        Ok(proposals)
    }

    async fn resolve_proposal(&self, id: Uuid, state: ProposalState) -> MvResult<bool> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let now = Utc::now().to_rfc3339();
        let affected = conn
            .execute(
                "UPDATE proposals SET state = ?2, updated_at = ?3, resolved_at = ?3 WHERE id = ?1",
                params![id.to_string(), state.as_str(), now],
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(affected > 0)
    }

    async fn count_proposals(&self, state: Option<ProposalState>) -> MvResult<usize> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let (sql, params_box): (String, Vec<Box<dyn rusqlite::types::ToSql>>) =
            if let Some(st) = state {
                (
                    "SELECT COUNT(*) FROM proposals WHERE state = ?1".to_string(),
                    vec![Box::new(st.as_str().to_string())],
                )
            } else {
                ("SELECT COUNT(*) FROM proposals".to_string(), vec![])
            };

        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            params_box.iter().map(|p| p.as_ref()).collect();

        let count: usize = conn
            .query_row(&sql, params_refs.as_slice(), |row| row.get(0))
            .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(count)
    }

    async fn expire_proposals(&self, before: DateTime<Utc>) -> MvResult<usize> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let now = Utc::now().to_rfc3339();
        let affected = conn
            .execute(
                "UPDATE proposals SET state = 'expired', updated_at = ?1, resolved_at = ?1 WHERE state = 'pending' AND created_at < ?2",
                params![now, before.to_rfc3339()],
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(affected)
    }
}

// ---------------------------------------------------------------------------
// SafeguardStore Implementation
// ---------------------------------------------------------------------------

use mv_core::{AutoApproveRule, BlockedSender, SafeguardStore, UndoSnapshot};

fn row_to_blocked_sender(row: &rusqlite::Row<'_>) -> rusqlite::Result<BlockedSender> {
    let id_str: String = row.get(0)?;
    let sender_type: String = row.get(1)?;
    let sender_pattern: String = row.get(2)?;
    let reason: Option<String> = row.get(3)?;
    let blocked_at: String = row.get(4)?;
    let expires_at: Option<String> = row.get(5)?;

    let id = parse_uuid_str(0, &id_str)?;

    Ok(BlockedSender {
        id,
        sender_type,
        sender_pattern,
        reason,
        blocked_at: parse_dt_strict(4, &blocked_at)?,
        expires_at: parse_optional_dt_strict(5, expires_at)?,
    })
}

fn row_to_auto_approve_rule(row: &rusqlite::Row<'_>) -> rusqlite::Result<AutoApproveRule> {
    let id_str: String = row.get(0)?;
    let name: String = row.get(1)?;
    let sender_pattern: Option<String> = row.get(2)?;
    let action_types_csv: Option<String> = row.get(3)?;
    let min_confidence: f64 = row.get(4)?;
    let enabled: bool = row.get(5)?;
    let created_at: String = row.get(6)?;
    let updated_at: Option<String> = row.get(7)?;

    let id = parse_uuid_str(0, &id_str)?;
    let action_types: Vec<String> = action_types_csv
        .map(|csv| {
            csv.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();

    Ok(AutoApproveRule {
        id,
        name,
        sender_pattern,
        action_types,
        min_confidence: min_confidence as f32,
        enabled,
        created_at: parse_dt_strict(6, &created_at)?,
        updated_at: parse_optional_dt_strict(7, updated_at)?,
    })
}

fn row_to_undo_snapshot(row: &rusqlite::Row<'_>) -> rusqlite::Result<UndoSnapshot> {
    let id_str: String = row.get(0)?;
    let proposal_id_str: String = row.get(1)?;
    let snapshot_data_str: String = row.get(2)?;
    let created_at: String = row.get(3)?;
    let expires_at: String = row.get(4)?;
    let used: bool = row.get(5)?;

    let id = parse_uuid_str(0, &id_str)?;
    let proposal_id = parse_uuid_str(1, &proposal_id_str)?;
    let snapshot_data: serde_json::Value =
        serde_json::from_str(&snapshot_data_str).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(
                2,
                Type::Text,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    e.to_string(),
                )),
            )
        })?;

    Ok(UndoSnapshot {
        id,
        proposal_id,
        snapshot_data,
        created_at: parse_dt_strict(3, &created_at)?,
        expires_at: parse_dt_strict(4, &expires_at)?,
        used,
    })
}

/// Simple glob matching: supports `*` (match all), `prefix*`, and exact match.
fn safeguard_glob_match(pattern: &str, value: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if pattern.ends_with('*') {
        value.starts_with(&pattern[..pattern.len() - 1])
    } else {
        pattern == value
    }
}

#[async_trait]
impl SafeguardStore for SqliteNodeStore {
    async fn add_blocked_sender(&self, sender: &BlockedSender) -> MvResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        conn.execute(
			"INSERT INTO blocked_senders (id, sender_type, sender_pattern, reason, blocked_at, expires_at)
			 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
			params![
				sender.id.to_string(),
				sender.sender_type,
				sender.sender_pattern,
				sender.reason,
				sender.blocked_at.to_rfc3339(),
				sender.expires_at.map(|dt| dt.to_rfc3339()),
			],
		)
		.map_err(|e| MvError::Storage(format!("insert blocked_sender failed: {e}")))?;
        Ok(())
    }

    async fn remove_blocked_sender(&self, id: Uuid) -> MvResult<bool> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let affected = conn
            .execute(
                "DELETE FROM blocked_senders WHERE id = ?1",
                params![id.to_string()],
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(affected > 0)
    }

    async fn list_blocked_senders(&self) -> MvResult<Vec<BlockedSender>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, sender_type, sender_pattern, reason, blocked_at, expires_at
				 FROM blocked_senders ORDER BY blocked_at DESC",
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let rows = stmt
            .query_map([], row_to_blocked_sender)
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(|e| MvError::Storage(e.to_string()))?);
        }
        Ok(result)
    }

    async fn is_sender_blocked(&self, sender_type: &str, sender_name: &str) -> MvResult<bool> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let now = Utc::now().to_rfc3339();

        // Fetch all active rules for this sender_type (not expired)
        let mut stmt = conn
            .prepare(
                "SELECT id, sender_type, sender_pattern, reason, blocked_at, expires_at
				 FROM blocked_senders
				 WHERE sender_type = ?1 AND (expires_at IS NULL OR expires_at > ?2)",
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let rows = stmt
            .query_map(params![sender_type, now], row_to_blocked_sender)
            .map_err(|e| MvError::Storage(e.to_string()))?;

        for row in rows {
            let blocked = row.map_err(|e| MvError::Storage(e.to_string()))?;
            if safeguard_glob_match(&blocked.sender_pattern, sender_name) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    async fn add_auto_approve_rule(&self, rule: &AutoApproveRule) -> MvResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let action_types_csv = if rule.action_types.is_empty() {
            None
        } else {
            Some(rule.action_types.join(","))
        };

        conn.execute(
			"INSERT INTO auto_approve_rules (id, name, sender_pattern, action_types, min_confidence, enabled, created_at, updated_at)
			 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
			params![
				rule.id.to_string(),
				rule.name,
				rule.sender_pattern,
				action_types_csv,
				rule.min_confidence as f64,
				rule.enabled,
				rule.created_at.to_rfc3339(),
				rule.updated_at.map(|dt| dt.to_rfc3339()),
			],
		)
		.map_err(|e| MvError::Storage(format!("insert auto_approve_rule failed: {e}")))?;
        Ok(())
    }

    async fn remove_auto_approve_rule(&self, id: Uuid) -> MvResult<bool> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let affected = conn
            .execute(
                "DELETE FROM auto_approve_rules WHERE id = ?1",
                params![id.to_string()],
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(affected > 0)
    }

    async fn list_auto_approve_rules(&self) -> MvResult<Vec<AutoApproveRule>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let mut stmt = conn
			.prepare(
				"SELECT id, name, sender_pattern, action_types, min_confidence, enabled, created_at, updated_at
				 FROM auto_approve_rules ORDER BY created_at DESC",
			)
			.map_err(|e| MvError::Storage(e.to_string()))?;

        let rows = stmt
            .query_map([], row_to_auto_approve_rule)
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(|e| MvError::Storage(e.to_string()))?);
        }
        Ok(result)
    }

    async fn update_auto_approve_rule(&self, rule: &AutoApproveRule) -> MvResult<bool> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let action_types_csv = if rule.action_types.is_empty() {
            None
        } else {
            Some(rule.action_types.join(","))
        };
        let now = Utc::now().to_rfc3339();

        let affected = conn
			.execute(
				"UPDATE auto_approve_rules SET name = ?2, sender_pattern = ?3, action_types = ?4, min_confidence = ?5, enabled = ?6, updated_at = ?7 WHERE id = ?1",
				params![
					rule.id.to_string(),
					rule.name,
					rule.sender_pattern,
					action_types_csv,
					rule.min_confidence as f64,
					rule.enabled,
					now,
				],
			)
			.map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(affected > 0)
    }

    async fn save_undo_snapshot(&self, snapshot: &UndoSnapshot) -> MvResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let snapshot_json = serde_json::to_string(&snapshot.snapshot_data)?;

        conn.execute(
			"INSERT INTO proposal_undo_snapshots (id, proposal_id, snapshot_data, created_at, expires_at, used)
			 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
			params![
				snapshot.id.to_string(),
				snapshot.proposal_id.to_string(),
				snapshot_json,
				snapshot.created_at.to_rfc3339(),
				snapshot.expires_at.to_rfc3339(),
				snapshot.used,
			],
		)
		.map_err(|e| MvError::Storage(format!("insert undo_snapshot failed: {e}")))?;
        Ok(())
    }

    async fn get_undo_snapshot(&self, proposal_id: Uuid) -> MvResult<Option<UndoSnapshot>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, proposal_id, snapshot_data, created_at, expires_at, used
				 FROM proposal_undo_snapshots WHERE proposal_id = ?1",
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let result = stmt
            .query_row(params![proposal_id.to_string()], row_to_undo_snapshot)
            .optional()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(result)
    }

    async fn mark_undo_used(&self, id: Uuid) -> MvResult<bool> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let affected = conn
            .execute(
                "UPDATE proposal_undo_snapshots SET used = 1 WHERE id = ?1",
                params![id.to_string()],
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(affected > 0)
    }

    async fn cleanup_expired_snapshots(&self) -> MvResult<usize> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let now = Utc::now().to_rfc3339();
        let affected = conn
            .execute(
                "DELETE FROM proposal_undo_snapshots WHERE expires_at < ?1",
                params![now],
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(affected)
    }
}

// ---------------------------------------------------------------------------
// FeedbackStore – Agent Feedback / Reflection
// ---------------------------------------------------------------------------

fn row_to_agent_feedback(row: &rusqlite::Row<'_>) -> rusqlite::Result<AgentFeedback> {
    let id_str: String = row.get(0)?;
    let intent_id_str: Option<String> = row.get(1)?;
    let intent_type: String = row.get(2)?;
    let action: String = row.get(3)?;
    let confidence_at_time: Option<f64> = row.get(4)?;
    let user_edit_delta: Option<f64> = row.get(5)?;
    let response_time_ms: Option<i64> = row.get(6)?;
    let created_at: String = row.get(7)?;

    let id = parse_uuid_str(0, &id_str)?;
    let intent_id = intent_id_str.and_then(|s| Uuid::parse_str(&s).ok());
    let created = chrono::DateTime::parse_from_rfc3339(&created_at)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());

    Ok(AgentFeedback {
        id,
        intent_id,
        intent_type,
        action,
        confidence_at_time: confidence_at_time.map(|v| v as f32),
        user_edit_delta: user_edit_delta.map(|v| v as f32),
        response_time_ms: response_time_ms.map(|v| v as u64),
        created_at: created,
    })
}

fn row_to_confidence_override(row: &rusqlite::Row<'_>) -> rusqlite::Result<ConfidenceOverride> {
    let intent_type: String = row.get(0)?;
    let base_adjustment: f64 = row.get(1)?;
    let auto_apply_threshold: f64 = row.get(2)?;
    let suppress_below: f64 = row.get(3)?;
    let updated_at: String = row.get(4)?;

    let updated = chrono::DateTime::parse_from_rfc3339(&updated_at)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());

    Ok(ConfidenceOverride {
        intent_type,
        base_adjustment: base_adjustment as f32,
        auto_apply_threshold: auto_apply_threshold as f32,
        suppress_below: suppress_below as f32,
        updated_at: updated,
    })
}

#[async_trait]
impl FeedbackStore for SqliteNodeStore {
    async fn record_feedback(&self, fb: &AgentFeedback) -> MvResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        conn.execute(
			"INSERT INTO agent_feedback (id, intent_id, intent_type, action, confidence_at_time, user_edit_delta, response_time_ms, created_at)
			 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
			params![
				fb.id.to_string(),
				fb.intent_id.map(|id| id.to_string()),
				fb.intent_type,
				fb.action,
				fb.confidence_at_time.map(|v| v as f64),
				fb.user_edit_delta.map(|v| v as f64),
				fb.response_time_ms.map(|v| v as i64),
				fb.created_at.to_rfc3339(),
			],
		)
		.map_err(|e| MvError::Storage(format!("insert agent_feedback failed: {e}")))?;
        Ok(())
    }

    async fn list_feedback(
        &self,
        intent_type: Option<&str>,
        limit: usize,
    ) -> MvResult<Vec<AgentFeedback>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut sql = String::from(
			"SELECT id, intent_id, intent_type, action, confidence_at_time, user_edit_delta, response_time_ms, created_at
			 FROM agent_feedback WHERE 1=1",
		);
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        let mut param_idx = 1;

        if let Some(it) = intent_type {
            sql.push_str(&format!(" AND intent_type = ?{param_idx}"));
            param_values.push(Box::new(it.to_string()));
            param_idx += 1;
        }

        sql.push_str(&format!(" ORDER BY created_at DESC LIMIT ?{param_idx}"));
        param_values.push(Box::new(limit as i64));

        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let rows = stmt
            .query_map(params_refs.as_slice(), row_to_agent_feedback)
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| MvError::Storage(e.to_string()))?);
        }
        Ok(results)
    }

    async fn get_acceptance_rate(&self, intent_type: &str) -> MvResult<(usize, usize)> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let total: usize = conn
            .query_row(
                "SELECT COUNT(*) FROM agent_feedback WHERE intent_type = ?1",
                params![intent_type],
                |row| row.get(0),
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let applied: usize = conn
            .query_row(
                "SELECT COUNT(*) FROM agent_feedback WHERE intent_type = ?1 AND action = 'applied'",
                params![intent_type],
                |row| row.get(0),
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;

        Ok((applied, total))
    }

    async fn set_confidence_override(&self, override_: &ConfidenceOverride) -> MvResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        conn.execute(
			"INSERT OR REPLACE INTO agent_confidence_overrides (intent_type, base_adjustment, auto_apply_threshold, suppress_below, updated_at)
			 VALUES (?1, ?2, ?3, ?4, ?5)",
			params![
				override_.intent_type,
				override_.base_adjustment as f64,
				override_.auto_apply_threshold as f64,
				override_.suppress_below as f64,
				override_.updated_at.to_rfc3339(),
			],
		)
		.map_err(|e| MvError::Storage(format!("upsert confidence_override failed: {e}")))?;
        Ok(())
    }

    async fn get_confidence_override(
        &self,
        intent_type: &str,
    ) -> MvResult<Option<ConfidenceOverride>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let mut stmt = conn
			.prepare(
				"SELECT intent_type, base_adjustment, auto_apply_threshold, suppress_below, updated_at
				 FROM agent_confidence_overrides WHERE intent_type = ?1",
			)
			.map_err(|e| MvError::Storage(e.to_string()))?;

        let result = stmt
            .query_row(params![intent_type], row_to_confidence_override)
            .optional()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(result)
    }

    async fn list_confidence_overrides(&self) -> MvResult<Vec<ConfidenceOverride>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let mut stmt = conn
			.prepare(
				"SELECT intent_type, base_adjustment, auto_apply_threshold, suppress_below, updated_at
				 FROM agent_confidence_overrides ORDER BY intent_type",
			)
			.map_err(|e| MvError::Storage(e.to_string()))?;

        let rows = stmt
            .query_map([], row_to_confidence_override)
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| MvError::Storage(e.to_string()))?);
        }
        Ok(results)
    }
}

// ---------------------------------------------------------------------------
// AutonomyStore – Autonomy & Precision Controls (Phase 3.1)
// ---------------------------------------------------------------------------

use mv_core::{AutonomyActionLog, AutonomyDecision, AutonomyRule, AutonomyStore};

fn row_to_autonomy_rule(row: &rusqlite::Row<'_>) -> rusqlite::Result<AutonomyRule> {
    let id_str: String = row.get(0)?;
    let rule_type: String = row.get(1)?;
    let scope_key: Option<String> = row.get(2)?;
    let auto_apply_threshold: f64 = row.get(3)?;
    let max_actions_per_hour: i64 = row.get(4)?;
    let allowed_csv: Option<String> = row.get(5)?;
    let blocked_csv: Option<String> = row.get(6)?;
    let quiet_hours_start: Option<String> = row.get(7)?;
    let quiet_hours_end: Option<String> = row.get(8)?;
    let quiet_hours_timezone: String = row.get(9)?;
    let enabled: bool = row.get(10)?;
    let created_at: String = row.get(11)?;
    let updated_at: Option<String> = row.get(12)?;

    let id = parse_uuid_str(0, &id_str)?;

    let allowed_intent_types: Vec<String> = allowed_csv
        .map(|csv| {
            csv.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();
    let blocked_intent_types: Vec<String> = blocked_csv
        .map(|csv| {
            csv.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();

    Ok(AutonomyRule {
        id,
        rule_type,
        scope_key,
        auto_apply_threshold: auto_apply_threshold as f32,
        max_actions_per_hour: max_actions_per_hour as u32,
        allowed_intent_types,
        blocked_intent_types,
        quiet_hours_start,
        quiet_hours_end,
        quiet_hours_timezone,
        enabled,
        created_at: parse_dt_strict(11, &created_at)?,
        updated_at: parse_optional_dt_strict(12, updated_at)?,
    })
}

fn row_to_autonomy_action_log(row: &rusqlite::Row<'_>) -> rusqlite::Result<AutonomyActionLog> {
    let id_str: String = row.get(0)?;
    let rule_id_str: Option<String> = row.get(1)?;
    let intent_type: String = row.get(2)?;
    let decision_str: String = row.get(3)?;
    let confidence: Option<f64> = row.get(4)?;
    let reason: Option<String> = row.get(5)?;
    let created_at: String = row.get(6)?;

    let id = parse_uuid_str(0, &id_str)?;
    let rule_id = rule_id_str.and_then(|s| Uuid::parse_str(&s).ok());
    let decision: AutonomyDecision = decision_str.parse().map_err(|e: String| {
        rusqlite::Error::FromSqlConversionFailure(
            3,
            Type::Text,
            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
        )
    })?;

    Ok(AutonomyActionLog {
        id,
        rule_id,
        intent_type,
        decision,
        confidence: confidence.map(|v| v as f32),
        reason,
        created_at: parse_dt_strict(6, &created_at)?,
    })
}

#[async_trait]
impl AutonomyStore for SqliteNodeStore {
    async fn add_autonomy_rule(&self, rule: &AutonomyRule) -> MvResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let allowed_csv = if rule.allowed_intent_types.is_empty() {
            None
        } else {
            Some(rule.allowed_intent_types.join(","))
        };
        let blocked_csv = if rule.blocked_intent_types.is_empty() {
            None
        } else {
            Some(rule.blocked_intent_types.join(","))
        };

        conn.execute(
			"INSERT INTO autonomy_rules (id, rule_type, scope_key, auto_apply_threshold, max_actions_per_hour, allowed_intent_types, blocked_intent_types, quiet_hours_start, quiet_hours_end, quiet_hours_timezone, enabled, created_at, updated_at)
			 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
			params![
				rule.id.to_string(),
				rule.rule_type,
				rule.scope_key,
				rule.auto_apply_threshold as f64,
				rule.max_actions_per_hour as i64,
				allowed_csv,
				blocked_csv,
				rule.quiet_hours_start,
				rule.quiet_hours_end,
				rule.quiet_hours_timezone,
				rule.enabled,
				rule.created_at.to_rfc3339(),
				rule.updated_at.map(|dt| dt.to_rfc3339()),
			],
		)
		.map_err(|e| MvError::Storage(format!("insert autonomy_rule failed: {e}")))?;
        Ok(())
    }

    async fn get_autonomy_rule(&self, id: Uuid) -> MvResult<Option<AutonomyRule>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let mut stmt = conn
			.prepare(
				"SELECT id, rule_type, scope_key, auto_apply_threshold, max_actions_per_hour, allowed_intent_types, blocked_intent_types, quiet_hours_start, quiet_hours_end, quiet_hours_timezone, enabled, created_at, updated_at
				 FROM autonomy_rules WHERE id = ?1",
			)
			.map_err(|e| MvError::Storage(e.to_string()))?;

        let result = stmt
            .query_row(params![id.to_string()], row_to_autonomy_rule)
            .optional()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(result)
    }

    async fn list_autonomy_rules(&self) -> MvResult<Vec<AutonomyRule>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let mut stmt = conn
			.prepare(
				"SELECT id, rule_type, scope_key, auto_apply_threshold, max_actions_per_hour, allowed_intent_types, blocked_intent_types, quiet_hours_start, quiet_hours_end, quiet_hours_timezone, enabled, created_at, updated_at
				 FROM autonomy_rules ORDER BY created_at DESC",
			)
			.map_err(|e| MvError::Storage(e.to_string()))?;

        let rows = stmt
            .query_map([], row_to_autonomy_rule)
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(|e| MvError::Storage(e.to_string()))?);
        }
        Ok(result)
    }

    async fn update_autonomy_rule(&self, rule: &AutonomyRule) -> MvResult<bool> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let allowed_csv = if rule.allowed_intent_types.is_empty() {
            None
        } else {
            Some(rule.allowed_intent_types.join(","))
        };
        let blocked_csv = if rule.blocked_intent_types.is_empty() {
            None
        } else {
            Some(rule.blocked_intent_types.join(","))
        };
        let now = Utc::now().to_rfc3339();

        let affected = conn
			.execute(
				"UPDATE autonomy_rules SET rule_type = ?2, scope_key = ?3, auto_apply_threshold = ?4, max_actions_per_hour = ?5, allowed_intent_types = ?6, blocked_intent_types = ?7, quiet_hours_start = ?8, quiet_hours_end = ?9, quiet_hours_timezone = ?10, enabled = ?11, updated_at = ?12 WHERE id = ?1",
				params![
					rule.id.to_string(),
					rule.rule_type,
					rule.scope_key,
					rule.auto_apply_threshold as f64,
					rule.max_actions_per_hour as i64,
					allowed_csv,
					blocked_csv,
					rule.quiet_hours_start,
					rule.quiet_hours_end,
					rule.quiet_hours_timezone,
					rule.enabled,
					now,
				],
			)
			.map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(affected > 0)
    }

    async fn delete_autonomy_rule(&self, id: Uuid) -> MvResult<bool> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let affected = conn
            .execute(
                "DELETE FROM autonomy_rules WHERE id = ?1",
                params![id.to_string()],
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(affected > 0)
    }

    async fn log_autonomy_action(&self, log: &AutonomyActionLog) -> MvResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        conn.execute(
			"INSERT INTO autonomy_action_log (id, rule_id, intent_type, decision, confidence, reason, created_at)
			 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
			params![
				log.id.to_string(),
				log.rule_id.map(|id| id.to_string()),
				log.intent_type,
				log.decision.to_string(),
				log.confidence.map(|v| v as f64),
				log.reason,
				log.created_at.to_rfc3339(),
			],
		)
		.map_err(|e| MvError::Storage(format!("insert autonomy_action_log failed: {e}")))?;
        Ok(())
    }

    async fn count_recent_actions(
        &self,
        rule_id: Option<Uuid>,
        since: DateTime<Utc>,
    ) -> MvResult<usize> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let (sql, params_box): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = if let Some(rid) =
            rule_id
        {
            (
				"SELECT COUNT(*) FROM autonomy_action_log WHERE rule_id = ?1 AND decision = 'auto_apply' AND created_at >= ?2".to_string(),
				vec![Box::new(rid.to_string()), Box::new(since.to_rfc3339())],
			)
        } else {
            (
				"SELECT COUNT(*) FROM autonomy_action_log WHERE decision = 'auto_apply' AND created_at >= ?1".to_string(),
				vec![Box::new(since.to_rfc3339())],
			)
        };

        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            params_box.iter().map(|p| p.as_ref()).collect();

        let count: usize = conn
            .query_row(&sql, params_refs.as_slice(), |row| row.get(0))
            .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(count)
    }

    async fn list_autonomy_action_log(&self, limit: usize) -> MvResult<Vec<AutonomyActionLog>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, rule_id, intent_type, decision, confidence, reason, created_at
				 FROM autonomy_action_log ORDER BY created_at DESC LIMIT ?1",
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let rows = stmt
            .query_map(params![limit as i64], row_to_autonomy_action_log)
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(|e| MvError::Storage(e.to_string()))?);
        }
        Ok(result)
    }
}

// ---------------------------------------------------------------------------
// RelayStore – Communication Relay Network (Phase 3.2)
// ---------------------------------------------------------------------------

use mv_core::{
    ChannelType, ContentType, MessageDirection, MessageStatus, RelayChannel, RelayContact,
    RelayMessage, RelayStore, TrustLevel,
};

fn row_to_relay_contact(row: &rusqlite::Row<'_>) -> rusqlite::Result<RelayContact> {
    let id_str: String = row.get(0)?;
    let display_name: String = row.get(1)?;
    let public_key: String = row.get(2)?;
    let vault_address: Option<String> = row.get(3)?;
    let trust_level_str: String = row.get(4)?;
    let autonomy_rule_id_str: Option<String> = row.get(5)?;
    let notes: Option<String> = row.get(6)?;
    let created_at_str: String = row.get(7)?;
    let updated_at_str: Option<String> = row.get(8)?;

    let id = parse_uuid_str(0, &id_str)?;
    let trust_level: TrustLevel = trust_level_str.parse().unwrap_or(TrustLevel::RelayOnly);
    let autonomy_rule_id = autonomy_rule_id_str.and_then(|s| Uuid::parse_str(&s).ok());
    let created_at = parse_dt_strict(7, &created_at_str)?;
    let updated_at = parse_optional_dt_strict(8, updated_at_str)?;

    Ok(RelayContact {
        id,
        display_name,
        public_key,
        vault_address,
        trust_level,
        autonomy_rule_id,
        notes,
        created_at,
        updated_at,
    })
}

fn row_to_relay_channel(row: &rusqlite::Row<'_>) -> rusqlite::Result<RelayChannel> {
    let id_str: String = row.get(0)?;
    let name: Option<String> = row.get(1)?;
    let channel_type_str: String = row.get(2)?;
    let member_ids_json: String = row.get(3)?;
    let created_at_str: String = row.get(4)?;
    let updated_at_str: Option<String> = row.get(5)?;

    let id = parse_uuid_str(0, &id_str)?;
    let channel_type: ChannelType = channel_type_str.parse().unwrap_or(ChannelType::Direct);
    let member_contact_ids: Vec<Uuid> = serde_json::from_str(&member_ids_json).unwrap_or_default();
    let created_at = parse_dt_strict(4, &created_at_str)?;
    let updated_at = parse_optional_dt_strict(5, updated_at_str)?;

    Ok(RelayChannel {
        id,
        name,
        channel_type,
        member_contact_ids,
        created_at,
        updated_at,
    })
}

fn row_to_relay_message(row: &rusqlite::Row<'_>) -> rusqlite::Result<RelayMessage> {
    let id_str: String = row.get(0)?;
    let channel_id_str: String = row.get(1)?;
    let thread_id_str: Option<String> = row.get(2)?;
    let sender_contact_id_str: Option<String> = row.get(3)?;
    let recipient_contact_id_str: Option<String> = row.get(4)?;
    let direction_str: String = row.get(5)?;
    let content: String = row.get(6)?;
    let content_type_str: String = row.get(7)?;
    let status_str: String = row.get(8)?;
    let vault_node_id_str: Option<String> = row.get(9)?;
    let metadata_json: String = row.get(10)?;
    let created_at_str: String = row.get(11)?;
    let updated_at_str: Option<String> = row.get(12)?;

    let id = parse_uuid_str(0, &id_str)?;
    let channel_id = parse_uuid_str(1, &channel_id_str)?;
    let thread_id = thread_id_str.and_then(|s| Uuid::parse_str(&s).ok());
    let sender_contact_id = sender_contact_id_str.and_then(|s| Uuid::parse_str(&s).ok());
    let recipient_contact_id = recipient_contact_id_str.and_then(|s| Uuid::parse_str(&s).ok());
    let direction: MessageDirection = direction_str.parse().unwrap_or(MessageDirection::Outbound);
    let content_type: ContentType = content_type_str.parse().unwrap_or(ContentType::Text);
    let status: MessageStatus = status_str.parse().unwrap_or(MessageStatus::Pending);
    let vault_node_id = vault_node_id_str.and_then(|s| Uuid::parse_str(&s).ok());
    let metadata: std::collections::HashMap<String, serde_json::Value> =
        serde_json::from_str(&metadata_json).unwrap_or_default();
    let created_at = parse_dt_strict(11, &created_at_str)?;
    let updated_at = parse_optional_dt_strict(12, updated_at_str)?;

    Ok(RelayMessage {
        id,
        channel_id,
        thread_id,
        sender_contact_id,
        recipient_contact_id,
        direction,
        content,
        content_type,
        status,
        vault_node_id,
        metadata,
        created_at,
        updated_at,
    })
}

#[async_trait]
impl RelayStore for SqliteNodeStore {
    // ── Contacts ──────────────────────────────────────────────────────────

    async fn add_relay_contact(&self, contact: &RelayContact) -> MvResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        conn.execute(
			"INSERT INTO relay_contacts (id, display_name, public_key, vault_address, trust_level, autonomy_rule_id, notes, created_at, updated_at)
			 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
			params![
				contact.id.to_string(),
				contact.display_name,
				contact.public_key,
				contact.vault_address,
				contact.trust_level.to_string(),
				contact.autonomy_rule_id.map(|id| id.to_string()),
				contact.notes,
				contact.created_at.to_rfc3339(),
				contact.updated_at.map(|dt| dt.to_rfc3339()),
			],
		)
		.map_err(|e| MvError::Storage(format!("insert relay_contact failed: {e}")))?;
        Ok(())
    }

    async fn get_relay_contact(&self, id: Uuid) -> MvResult<Option<RelayContact>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let mut stmt = conn
			.prepare(
				"SELECT id, display_name, public_key, vault_address, trust_level, autonomy_rule_id, notes, created_at, updated_at
				 FROM relay_contacts WHERE id = ?1",
			)
			.map_err(|e| MvError::Storage(e.to_string()))?;

        let result = stmt
            .query_row(params![id.to_string()], row_to_relay_contact)
            .optional()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(result)
    }

    async fn list_relay_contacts(&self) -> MvResult<Vec<RelayContact>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let mut stmt = conn
			.prepare(
				"SELECT id, display_name, public_key, vault_address, trust_level, autonomy_rule_id, notes, created_at, updated_at
				 FROM relay_contacts ORDER BY created_at DESC",
			)
			.map_err(|e| MvError::Storage(e.to_string()))?;

        let rows = stmt
            .query_map([], row_to_relay_contact)
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(|e| MvError::Storage(e.to_string()))?);
        }
        Ok(result)
    }

    async fn update_relay_contact(&self, contact: &RelayContact) -> MvResult<bool> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let now = Utc::now().to_rfc3339();
        let affected = conn
			.execute(
				"UPDATE relay_contacts SET display_name = ?2, public_key = ?3, vault_address = ?4, trust_level = ?5, autonomy_rule_id = ?6, notes = ?7, updated_at = ?8 WHERE id = ?1",
				params![
					contact.id.to_string(),
					contact.display_name,
					contact.public_key,
					contact.vault_address,
					contact.trust_level.to_string(),
					contact.autonomy_rule_id.map(|id| id.to_string()),
					contact.notes,
					now,
				],
			)
			.map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(affected > 0)
    }

    async fn delete_relay_contact(&self, id: Uuid) -> MvResult<bool> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let affected = conn
            .execute(
                "DELETE FROM relay_contacts WHERE id = ?1",
                params![id.to_string()],
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(affected > 0)
    }

    // ── Channels ──────────────────────────────────────────────────────────

    async fn add_relay_channel(&self, channel: &RelayChannel) -> MvResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let member_ids_json = serde_json::to_string(&channel.member_contact_ids)
            .map_err(|e| MvError::Storage(e.to_string()))?;
        conn.execute(
			"INSERT INTO relay_channels (id, name, channel_type, member_contact_ids, created_at, updated_at)
			 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
			params![
				channel.id.to_string(),
				channel.name,
				channel.channel_type.to_string(),
				member_ids_json,
				channel.created_at.to_rfc3339(),
				channel.updated_at.map(|dt| dt.to_rfc3339()),
			],
		)
		.map_err(|e| MvError::Storage(format!("insert relay_channel failed: {e}")))?;
        Ok(())
    }

    async fn get_relay_channel(&self, id: Uuid) -> MvResult<Option<RelayChannel>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, channel_type, member_contact_ids, created_at, updated_at
				 FROM relay_channels WHERE id = ?1",
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let result = stmt
            .query_row(params![id.to_string()], row_to_relay_channel)
            .optional()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(result)
    }

    async fn list_relay_channels(&self) -> MvResult<Vec<RelayChannel>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, channel_type, member_contact_ids, created_at, updated_at
				 FROM relay_channels ORDER BY created_at DESC",
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let rows = stmt
            .query_map([], row_to_relay_channel)
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(|e| MvError::Storage(e.to_string()))?);
        }
        Ok(result)
    }

    async fn delete_relay_channel(&self, id: Uuid) -> MvResult<bool> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let affected = conn
            .execute(
                "DELETE FROM relay_channels WHERE id = ?1",
                params![id.to_string()],
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(affected > 0)
    }

    // ── Messages ──────────────────────────────────────────────────────────

    async fn add_relay_message(&self, message: &RelayMessage) -> MvResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let metadata_json = serde_json::to_string(&message.metadata)
            .map_err(|e| MvError::Storage(e.to_string()))?;
        conn.execute(
			"INSERT INTO relay_messages (id, channel_id, thread_id, sender_contact_id, recipient_contact_id, direction, content, content_type, status, vault_node_id, metadata, created_at, updated_at)
			 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
			params![
				message.id.to_string(),
				message.channel_id.to_string(),
				message.thread_id.map(|id| id.to_string()),
				message.sender_contact_id.map(|id| id.to_string()),
				message.recipient_contact_id.map(|id| id.to_string()),
				message.direction.to_string(),
				message.content,
				message.content_type.to_string(),
				message.status.to_string(),
				message.vault_node_id.map(|id| id.to_string()),
				metadata_json,
				message.created_at.to_rfc3339(),
				message.updated_at.map(|dt| dt.to_rfc3339()),
			],
		)
		.map_err(|e| MvError::Storage(format!("insert relay_message failed: {e}")))?;
        Ok(())
    }

    async fn get_relay_message(&self, id: Uuid) -> MvResult<Option<RelayMessage>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let mut stmt = conn
			.prepare(
				"SELECT id, channel_id, thread_id, sender_contact_id, recipient_contact_id, direction, content, content_type, status, vault_node_id, metadata, created_at, updated_at
				 FROM relay_messages WHERE id = ?1",
			)
			.map_err(|e| MvError::Storage(e.to_string()))?;

        let result = stmt
            .query_row(params![id.to_string()], row_to_relay_message)
            .optional()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(result)
    }

    async fn list_relay_messages(
        &self,
        channel_id: Uuid,
        limit: usize,
        offset: usize,
    ) -> MvResult<Vec<RelayMessage>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let mut stmt = conn
			.prepare(
				"SELECT id, channel_id, thread_id, sender_contact_id, recipient_contact_id, direction, content, content_type, status, vault_node_id, metadata, created_at, updated_at
				 FROM relay_messages WHERE channel_id = ?1 ORDER BY created_at DESC LIMIT ?2 OFFSET ?3",
			)
			.map_err(|e| MvError::Storage(e.to_string()))?;

        let rows = stmt
            .query_map(
                params![channel_id.to_string(), limit as i64, offset as i64],
                row_to_relay_message,
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(|e| MvError::Storage(e.to_string()))?);
        }
        Ok(result)
    }

    async fn update_message_status(&self, id: Uuid, status: MessageStatus) -> MvResult<bool> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let now = Utc::now().to_rfc3339();
        let affected = conn
            .execute(
                "UPDATE relay_messages SET status = ?2, updated_at = ?3 WHERE id = ?1",
                params![id.to_string(), status.to_string(), now],
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(affected > 0)
    }

    async fn list_thread_messages(
        &self,
        thread_id: Uuid,
        limit: usize,
    ) -> MvResult<Vec<RelayMessage>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;
        let mut stmt = conn
			.prepare(
				"SELECT id, channel_id, thread_id, sender_contact_id, recipient_contact_id, direction, content, content_type, status, vault_node_id, metadata, created_at, updated_at
				 FROM relay_messages WHERE thread_id = ?1 ORDER BY created_at ASC LIMIT ?2",
			)
			.map_err(|e| MvError::Storage(e.to_string()))?;

        let rows = stmt
            .query_map(
                params![thread_id.to_string(), limit as i64],
                row_to_relay_message,
            )
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(|e| MvError::Storage(e.to_string()))?);
        }
        Ok(result)
    }

    async fn count_unread_messages(&self, channel_id: Option<Uuid>) -> MvResult<usize> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MvError::Storage(e.to_string()))?;

        let (sql, params_box): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = if let Some(cid) =
            channel_id
        {
            (
				"SELECT COUNT(*) FROM relay_messages WHERE status NOT IN ('read') AND direction = 'inbound' AND channel_id = ?1".to_string(),
				vec![Box::new(cid.to_string())],
			)
        } else {
            (
				"SELECT COUNT(*) FROM relay_messages WHERE status NOT IN ('read') AND direction = 'inbound'".to_string(),
				vec![],
			)
        };

        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            params_box.iter().map(|p| p.as_ref()).collect();

        let count: usize = conn
            .query_row(&sql, params_refs.as_slice(), |row| row.get(0))
            .map_err(|e| MvError::Storage(e.to_string()))?;
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_insert_and_get() {
        let store = SqliteNodeStore::open_in_memory().unwrap();
        let node = KnowledgeNode::new(NodeKind::Fact, "Rust is fast".into())
            .with_title("Rust Speed")
            .with_tags(vec!["rust".into(), "performance".into()]);

        let id = node.id;
        store.insert(&node).await.unwrap();

        let retrieved = store.get(id).await.unwrap().unwrap();
        assert_eq!(retrieved.content, "Rust is fast");
        assert_eq!(retrieved.tags, vec!["performance", "rust"]); // sorted
        assert_eq!(retrieved.kind, NodeKind::Fact);
    }

    #[tokio::test]
    async fn test_update() {
        let store = SqliteNodeStore::open_in_memory().unwrap();
        let mut node = KnowledgeNode::new(NodeKind::Fact, "original".into());
        let id = node.id;
        store.insert(&node).await.unwrap();

        node.content = "updated".into();
        node.temporal.version = 2;
        node.temporal.updated_at = Utc::now();
        store.update(&node).await.unwrap();

        let retrieved = store.get(id).await.unwrap().unwrap();
        assert_eq!(retrieved.content, "updated");
        assert_eq!(retrieved.temporal.version, 2);
    }

    #[tokio::test]
    async fn test_delete() {
        let store = SqliteNodeStore::open_in_memory().unwrap();
        let node = KnowledgeNode::new(NodeKind::Fact, "to delete".into());
        let id = node.id;
        store.insert(&node).await.unwrap();

        assert!(store.delete(id).await.unwrap());
        assert!(store.get(id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_list_with_filters() {
        let store = SqliteNodeStore::open_in_memory().unwrap();

        let n1 = KnowledgeNode::new(NodeKind::Fact, "fact one".into())
            .with_namespace("dev")
            .with_tags(vec!["rust".into()]);
        let n2 = KnowledgeNode::new(NodeKind::Decision, "decision one".into())
            .with_namespace("dev")
            .with_tags(vec!["planning".into()]);
        let n3 = KnowledgeNode::new(NodeKind::Fact, "fact two".into())
            .with_namespace("personal")
            .with_tags(vec!["rust".into()]);

        store.insert(&n1).await.unwrap();
        store.insert(&n2).await.unwrap();
        store.insert(&n3).await.unwrap();

        let filters = QueryFilters {
            namespace: Some("dev".into()),
            ..Default::default()
        };
        let results = store.list(&filters, 10, 0).await.unwrap();
        assert_eq!(results.len(), 2);

        let filters = QueryFilters {
            kinds: Some(vec![NodeKind::Fact]),
            ..Default::default()
        };
        let results = store.list(&filters, 10, 0).await.unwrap();
        assert_eq!(results.len(), 2);

        let filters = QueryFilters {
            tags: Some(vec!["planning".into()]),
            ..Default::default()
        };
        let results = store.list(&filters, 10, 0).await.unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_touch() {
        let store = SqliteNodeStore::open_in_memory().unwrap();
        let node = KnowledgeNode::new(NodeKind::Fact, "touchable".into());
        let id = node.id;
        store.insert(&node).await.unwrap();

        store.touch(id).await.unwrap();
        let retrieved = store.get(id).await.unwrap().unwrap();
        assert_eq!(retrieved.temporal.access_count, 1);
    }

    #[tokio::test]
    async fn test_count() {
        let store = SqliteNodeStore::open_in_memory().unwrap();
        store
            .insert(&KnowledgeNode::new(NodeKind::Fact, "a".into()))
            .await
            .unwrap();
        store
            .insert(&KnowledgeNode::new(NodeKind::Fact, "b".into()))
            .await
            .unwrap();
        store
            .insert(&KnowledgeNode::new(NodeKind::Decision, "c".into()))
            .await
            .unwrap();

        let all = store.count(&QueryFilters::default()).await.unwrap();
        assert_eq!(all, 3);

        let facts = store
            .count(&QueryFilters {
                kinds: Some(vec![NodeKind::Fact]),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(facts, 2);
    }
}
