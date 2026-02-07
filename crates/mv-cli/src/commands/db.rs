//! Database maintenance commands for MindVault CLI.

use anyhow::Result;
use std::path::Path;

use super::load_config;

const SQLITE_DB_FILE: &str = "mindvault.sqlite";

/// Run database vacuum to reclaim space.
pub async fn vacuum(config_path: &str) -> Result<()> {
    let config = load_config(config_path)?;
    let data_dir = &config.data_dir;
    let db_path = format!("{data_dir}/{SQLITE_DB_FILE}");

    if !Path::new(&db_path).exists() {
        println!("Database not found: {db_path}");
        return Ok(());
    }

    let size_before = std::fs::metadata(&db_path)?.len();

    println!("Running VACUUM on database...");
    let conn = rusqlite::Connection::open(&db_path)?;
    conn.execute("VACUUM", [])?;

    let size_after = std::fs::metadata(&db_path)?.len();
    let saved = size_before.saturating_sub(size_after);

    println!();
    println!("Vacuum complete:");
    println!("  Before: {}", format_size(size_before));
    println!("  After:  {}", format_size(size_after));
    if saved > 0 {
        println!("  Saved:  {}", format_size(saved));
    }

    Ok(())
}

/// Check database integrity.
pub async fn check(config_path: &str) -> Result<()> {
    let config = load_config(config_path)?;
    let data_dir = &config.data_dir;
    let db_path = format!("{data_dir}/{SQLITE_DB_FILE}");

    if !Path::new(&db_path).exists() {
        println!("Database not found: {db_path}");
        return Ok(());
    }

    println!("Checking database integrity...");
    let conn = rusqlite::Connection::open(&db_path)?;

    // Run integrity check
    let result: String = conn.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;

    if result == "ok" {
        println!("Database integrity: OK");
    } else {
        println!("Database integrity issues found:");
        println!("{result}");
    }

    // Check foreign keys
    let fk_result = conn
        .prepare("PRAGMA foreign_key_check")?
        .query_map([], |_row| Ok(()))?
        .count() as i64;

    if fk_result == 0 {
        println!("Foreign key integrity: OK");
    } else {
        println!("Foreign key violations: {fk_result}");
    }

    Ok(())
}

/// Analyze database for query optimization.
pub async fn analyze(config_path: &str) -> Result<()> {
    let config = load_config(config_path)?;
    let data_dir = &config.data_dir;
    let db_path = format!("{data_dir}/{SQLITE_DB_FILE}");

    if !Path::new(&db_path).exists() {
        println!("Database not found: {db_path}");
        return Ok(());
    }

    println!("Analyzing database for query optimization...");
    let conn = rusqlite::Connection::open(&db_path)?;
    conn.execute("ANALYZE", [])?;

    println!("Analysis complete. Query planner statistics updated.");

    Ok(())
}

/// Show database statistics.
pub async fn info(config_path: &str) -> Result<()> {
    let config = load_config(config_path)?;
    let data_dir = &config.data_dir;
    let db_path = format!("{data_dir}/{SQLITE_DB_FILE}");

    if !Path::new(&db_path).exists() {
        println!("Database not found: {db_path}");
        return Ok(());
    }

    let conn = rusqlite::Connection::open(&db_path)?;

    println!("Database Information");
    println!("====================");
    println!();
    println!("Path: {db_path}");
    println!("Size: {}", format_size(std::fs::metadata(&db_path)?.len()));
    println!();

    // SQLite version
    let version: String = conn.query_row("SELECT sqlite_version()", [], |row| row.get(0))?;
    println!("SQLite version: {version}");

    // Page size and count
    let page_size: i64 = conn.query_row("PRAGMA page_size", [], |row| row.get(0))?;
    let page_count: i64 = conn.query_row("PRAGMA page_count", [], |row| row.get(0))?;
    let freelist_count: i64 = conn.query_row("PRAGMA freelist_count", [], |row| row.get(0))?;

    println!("Page size: {} bytes", page_size);
    println!("Total pages: {page_count}");
    println!("Free pages: {freelist_count}");

    println!();
    println!("Tables:");

    // List tables with row counts
    let mut stmt = conn.prepare(
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name"
    )?;

    let tables: Vec<String> = stmt
        .query_map([], |row| row.get(0))?
        .filter_map(|r| r.ok())
        .collect();

    for table in tables {
        let count: i64 = conn
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap_or(0);
        println!("  {table}: {count} rows");
    }

    println!();
    println!("Indexes:");

    let mut stmt = conn.prepare(
        "SELECT name, tbl_name FROM sqlite_master WHERE type='index' AND name NOT LIKE 'sqlite_%' ORDER BY tbl_name, name"
    )?;

    let indexes: Vec<(String, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .filter_map(|r| r.ok())
        .collect();

    for (name, table) in indexes {
        println!("  {name} on {table}");
    }

    // Check for WAL mode
    let journal_mode: String = conn.query_row("PRAGMA journal_mode", [], |row| row.get(0))?;
    println!();
    println!("Journal mode: {journal_mode}");

    // Check data directory contents
    println!();
    println!("Data Directory Contents:");
    if let Ok(entries) = std::fs::read_dir(data_dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let name = entry.file_name().to_string_lossy().to_string();
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            println!("  {name}: {}", format_size(size));
        }
    }

    Ok(())
}

/// Rebuild indexes.
pub async fn reindex(config_path: &str) -> Result<()> {
    let config = load_config(config_path)?;
    let data_dir = &config.data_dir;
    let db_path = format!("{data_dir}/{SQLITE_DB_FILE}");

    if !Path::new(&db_path).exists() {
        println!("Database not found: {db_path}");
        return Ok(());
    }

    println!("Rebuilding database indexes...");
    let conn = rusqlite::Connection::open(&db_path)?;
    conn.execute("REINDEX", [])?;

    println!("Index rebuild complete.");

    Ok(())
}

fn format_size(size: u64) -> String {
    if size > 1024 * 1024 * 1024 {
        format!("{:.2} GB", size as f64 / 1024.0 / 1024.0 / 1024.0)
    } else if size > 1024 * 1024 {
        format!("{:.2} MB", size as f64 / 1024.0 / 1024.0)
    } else if size > 1024 {
        format!("{:.2} KB", size as f64 / 1024.0)
    } else {
        format!("{size} bytes")
    }
}
