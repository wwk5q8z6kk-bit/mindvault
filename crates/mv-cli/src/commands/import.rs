use anyhow::{Context, Result};
use mv_core::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

pub async fn run(
    from: String,
    path: String,
    namespace: Option<String>,
    dry_run: bool,
    config_path: &str,
) -> Result<()> {
    let path = super::shellexpand(&path);
    let engine = super::load_engine(config_path).await?;

    match from.as_str() {
        "claude-memory" => import_claude_memory(&engine, &path).await,
        "markdown" | "md" => import_markdown(&engine, &path).await,
        "markdown-dir" | "md-dir" => import_markdown_dir(&engine, &path).await,
        "text" | "txt" => import_text(&engine, &path).await,
        "json" => import_json(&engine, &path).await,
        "csv" => import_csv(&engine, &path).await,
        "obsidian" => {
            let ns = namespace.as_deref().unwrap_or("imported");
            import_obsidian_vault(&engine, &path, ns, dry_run).await
        }
        _ => {
            anyhow::bail!(
                "unknown import format: {from}. Supported: claude-memory, markdown, markdown-dir, text, json, csv, obsidian"
            );
        }
    }
}

async fn import_claude_memory(
    engine: &mv_engine::engine::MindVaultEngine,
    path: &str,
) -> Result<()> {
    let content = std::fs::read_to_string(path)?;
    let mut count = 0;

    // Parse markdown sections as separate knowledge nodes
    let mut current_section = String::new();
    let mut current_title = String::new();

    for line in content.lines() {
        if line.starts_with("## ") {
            // Save previous section
            if !current_section.trim().is_empty() {
                let node = KnowledgeNode::new(NodeKind::Fact, current_section.trim().to_string())
                    .with_title(&current_title)
                    .with_source(format!("import:claude-memory:{path}"))
                    .with_namespace("imported")
                    .with_tags(vec!["imported".into(), "claude-memory".into()]);

                engine.store_node(node).await?;
                count += 1;
            }
            current_title = line.trim_start_matches('#').trim().to_string();
            current_section = String::new();
        } else {
            current_section.push_str(line);
            current_section.push('\n');
        }
    }

    // Save last section
    if !current_section.trim().is_empty() {
        let node = KnowledgeNode::new(NodeKind::Fact, current_section.trim().to_string())
            .with_title(&current_title)
            .with_source(format!("import:claude-memory:{path}"))
            .with_namespace("imported")
            .with_tags(vec!["imported".into(), "claude-memory".into()]);

        engine.store_node(node).await?;
        count += 1;
    }

    println!("imported {count} nodes from Claude memory ({path})");
    Ok(())
}

async fn import_markdown(engine: &mv_engine::engine::MindVaultEngine, path: &str) -> Result<()> {
    let content = std::fs::read_to_string(path)?;
    let filename = std::path::Path::new(path)
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("unknown");

    let node = KnowledgeNode::new(NodeKind::Fact, content)
        .with_title(filename)
        .with_source(format!("import:markdown:{path}"))
        .with_namespace("imported")
        .with_tags(vec!["imported".into(), "markdown".into()]);

    let stored = engine.store_node(node).await?;
    println!("imported: {} ({})", stored.id, filename);
    Ok(())
}

async fn import_text(engine: &mv_engine::engine::MindVaultEngine, path: &str) -> Result<()> {
    let content = std::fs::read_to_string(path)?;
    let filename = std::path::Path::new(path)
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("unknown");

    let node = KnowledgeNode::new(NodeKind::Observation, content)
        .with_title(filename)
        .with_source(format!("import:text:{path}"))
        .with_namespace("imported");

    let stored = engine.store_node(node).await?;
    println!("imported: {} ({})", stored.id, filename);
    Ok(())
}

/// Import from MindVault JSON export format.
async fn import_json(engine: &mv_engine::engine::MindVaultEngine, path: &str) -> Result<()> {
    #[derive(Deserialize)]
    struct JsonExport {
        nodes: Vec<JsonNode>,
    }

    #[derive(Deserialize)]
    struct JsonNode {
        kind: String,
        title: Option<String>,
        content: String,
        namespace: String,
        importance: f64,
        tags: Vec<String>,
        source: Option<String>,
        #[serde(default)]
        metadata: HashMap<String, serde_json::Value>,
    }

    let content = std::fs::read_to_string(path).context("Failed to read JSON file")?;
    let export: JsonExport = serde_json::from_str(&content).context("Failed to parse JSON")?;

    println!("Importing {} nodes from JSON...", export.nodes.len());

    let mut count = 0;
    for json_node in export.nodes {
        let kind = parse_node_kind(&json_node.kind);
        let source = json_node
            .source
            .unwrap_or_else(|| format!("import:json:{path}"));

        let mut node = KnowledgeNode::new(kind, json_node.content)
            .with_namespace(&json_node.namespace)
            .with_importance(json_node.importance)
            .with_tags(json_node.tags)
            .with_source(source);

        if let Some(title) = json_node.title {
            node = node.with_title(&title);
        }

        node.metadata = json_node.metadata;

        engine.store_node(node).await?;
        count += 1;
    }

    println!("Imported {count} nodes from JSON");
    Ok(())
}

/// Import from CSV format.
async fn import_csv(engine: &mv_engine::engine::MindVaultEngine, path: &str) -> Result<()> {
    let content = std::fs::read_to_string(path).context("Failed to read CSV file")?;
    let mut lines = content.lines();

    // Skip header
    let header = lines.next().context("CSV file is empty")?;
    let columns: Vec<&str> = header.split(',').collect();

    // Find column indices
    let find_col = |name: &str| columns.iter().position(|&c| c.trim() == name);
    let kind_col = find_col("kind").unwrap_or(1);
    let title_col = find_col("title").unwrap_or(2);
    let namespace_col = find_col("namespace").unwrap_or(3);
    let importance_col = find_col("importance").unwrap_or(4);
    let tags_col = find_col("tags").unwrap_or(5);
    let content_col = find_col("content").unwrap_or(7);

    println!("Importing from CSV...");

    let mut count = 0;
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }

        let fields = parse_csv_line(line);
        if fields.len() <= content_col {
            continue;
        }

        let kind_str = fields.get(kind_col).map(|s| s.as_str()).unwrap_or("fact");
        let kind = parse_node_kind(kind_str);

        let content = fields.get(content_col).cloned().unwrap_or_default();
        if content.is_empty() {
            continue;
        }

        let title = fields.get(title_col).filter(|s| !s.is_empty()).cloned();
        let namespace = fields
            .get(namespace_col)
            .filter(|s| !s.is_empty())
            .cloned()
            .unwrap_or_else(|| "imported".to_string());
        let importance: f64 = fields
            .get(importance_col)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.5);
        let tags: Vec<String> = fields
            .get(tags_col)
            .map(|s| s.split(';').map(|t| t.trim().to_string()).collect())
            .unwrap_or_default();

        let mut node = KnowledgeNode::new(kind, content)
            .with_namespace(&namespace)
            .with_importance(importance)
            .with_tags(tags)
            .with_source(format!("import:csv:{path}"));

        if let Some(t) = title {
            node = node.with_title(&t);
        }

        engine.store_node(node).await?;
        count += 1;
    }

    println!("Imported {count} nodes from CSV");
    Ok(())
}

/// Import a directory of Markdown files.
async fn import_markdown_dir(
    engine: &mv_engine::engine::MindVaultEngine,
    path: &str,
) -> Result<()> {
    let dir_path = Path::new(path);
    if !dir_path.is_dir() {
        anyhow::bail!("Path is not a directory: {path}");
    }

    println!("Importing Markdown files from: {path}");

    let mut count = 0;
    for entry in walkdir::WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let file_path = entry.path();
        if !file_path.is_file() {
            continue;
        }

        let ext = file_path.extension().and_then(|e| e.to_str());
        if ext != Some("md") && ext != Some("markdown") {
            continue;
        }

        let content = std::fs::read_to_string(file_path)?;
        let filename = file_path
            .file_stem()
            .and_then(|f| f.to_str())
            .unwrap_or("unknown");

        // Extract namespace from subdirectory
        let namespace = file_path
            .parent()
            .and_then(|p| p.strip_prefix(path).ok())
            .and_then(|p| p.to_str())
            .filter(|s| !s.is_empty())
            .unwrap_or("imported")
            .replace('/', ".");

        // Parse YAML frontmatter if present
        let (metadata, body) = parse_frontmatter(&content);

        let title = metadata
            .get("title")
            .and_then(|v| v.as_str())
            .map(String::from)
            .unwrap_or_else(|| filename.to_string());

        let tags: Vec<String> = metadata
            .get("tags")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let importance: f64 = metadata
            .get("importance")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);

        let node = KnowledgeNode::new(NodeKind::Fact, body)
            .with_title(&title)
            .with_namespace(&namespace)
            .with_importance(importance)
            .with_tags(tags)
            .with_source(format!("import:markdown-dir:{}", file_path.display()));

        engine.store_node(node).await?;
        count += 1;
    }

    println!("Imported {count} Markdown files");
    Ok(())
}

/// Import from Obsidian vault using the engine-level importer.
async fn import_obsidian_vault(
    engine: &mv_engine::engine::MindVaultEngine,
    path: &str,
    namespace: &str,
    dry_run: bool,
) -> Result<()> {
    let vault_path = Path::new(path);

    if dry_run {
        println!("DRY RUN: Scanning Obsidian vault at: {path}");
    } else {
        println!("Importing Obsidian vault from: {path}");
    }

    let stats =
        mv_engine::import::obsidian::import_obsidian_vault(vault_path, namespace, engine, dry_run)
            .await?;

    if dry_run {
        println!("\n--- Dry Run Results ---");
        println!("Files scanned:           {}", stats.files_scanned);
        println!("Would create:            {}", stats.nodes_created);
        println!("Would update:            {}", stats.nodes_updated);
        println!("Already up to date:      {}", stats.nodes_skipped);
        println!("Wikilink relationships:  {}", stats.relationships_created);
    } else {
        println!("\n--- Import Results ---");
        println!("Files scanned:           {}", stats.files_scanned);
        println!("Nodes created:           {}", stats.nodes_created);
        println!("Nodes updated:           {}", stats.nodes_updated);
        println!("Nodes skipped (up-to-date): {}", stats.nodes_skipped);
        println!("Relationships created:   {}", stats.relationships_created);
    }

    if !stats.errors.is_empty() {
        println!("\nErrors ({}):", stats.errors.len());
        for err in &stats.errors {
            println!("  - {err}");
        }
    }

    Ok(())
}

fn parse_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        if in_quotes {
            if c == '"' {
                if i + 1 < chars.len() && chars[i + 1] == '"' {
                    current.push('"');
                    i += 1;
                } else {
                    in_quotes = false;
                }
            } else {
                current.push(c);
            }
        } else if c == '"' {
            in_quotes = true;
        } else if c == ',' {
            fields.push(current.clone());
            current.clear();
        } else {
            current.push(c);
        }

        i += 1;
    }

    fields.push(current);
    fields
}

fn parse_frontmatter(content: &str) -> (HashMap<String, serde_json::Value>, String) {
    let mut metadata = HashMap::new();

    if !content.starts_with("---") {
        return (metadata, content.to_string());
    }

    let parts: Vec<&str> = content.splitn(3, "---").collect();
    if parts.len() < 3 {
        return (metadata, content.to_string());
    }

    let frontmatter = parts[1].trim();
    let body = parts[2].trim();

    // Simple YAML parsing for common fields
    for line in frontmatter.lines() {
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let value = value.trim().trim_matches('"');

            if key == "tags" {
                // Handle tags: [tag1, tag2] or tags:\n- tag1\n- tag2
                if value.starts_with('[') {
                    let tags: Vec<serde_json::Value> = value
                        .trim_start_matches('[')
                        .trim_end_matches(']')
                        .split(',')
                        .map(|s| serde_json::Value::String(s.trim().to_string()))
                        .collect();
                    metadata.insert(key.to_string(), serde_json::Value::Array(tags));
                }
            } else if let Ok(num) = value.parse::<f64>() {
                metadata.insert(key.to_string(), serde_json::Value::from(num));
            } else {
                metadata.insert(
                    key.to_string(),
                    serde_json::Value::String(value.to_string()),
                );
            }
        }
    }

    (metadata, body.to_string())
}

fn parse_node_kind(kind: &str) -> NodeKind {
    let normalized = kind.trim().to_ascii_lowercase().replace('-', "_");
    match normalized.as_str() {
        // Legacy values from older exports.
        "belief" => NodeKind::Fact,
        "goal" => NodeKind::Task,
        "episode" => NodeKind::Event,
        _ => normalized.parse::<NodeKind>().unwrap_or(NodeKind::Fact),
    }
}
