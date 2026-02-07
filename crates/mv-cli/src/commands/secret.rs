use anyhow::Result;
use mv_core::credentials::CredentialStore;
use std::io::{self, Write};

fn store() -> CredentialStore {
    CredentialStore::new("mindvault")
}

pub async fn set(key: &str, value: Option<&str>) -> Result<()> {
    let secret_value = match value {
        Some(v) => v.to_string(),
        None => {
            // Read from stdin (for piping) or prompt
            eprint!("Enter secret value for {key}: ");
            io::stderr().flush()?;
            let mut buf = String::new();
            io::stdin().read_line(&mut buf)?;
            buf.trim_end().to_string()
        }
    };

    if secret_value.is_empty() {
        anyhow::bail!("secret value cannot be empty");
    }

    let creds = store();
    let source = creds.set(key, &secret_value)?;
    println!("stored {key} in {source}");
    Ok(())
}

pub async fn get(key: &str) -> Result<()> {
    let creds = store();
    match creds.get(key)? {
        Some(sv) => {
            println!("{}", sv.expose());
        }
        None => {
            anyhow::bail!("{key} not found in any credential backend");
        }
    }
    Ok(())
}

pub async fn list() -> Result<()> {
    let creds = store();
    let statuses = creds.status();

    let mut found_any = false;
    for status in &statuses {
        if !status.keys.is_empty() {
            found_any = true;
        }
    }

    if !found_any {
        println!("no secrets stored");
        return Ok(());
    }

    // Collect all unique keys and their sources
    let mut entries: Vec<(String, String)> = Vec::new();
    for status in &statuses {
        for key in &status.keys {
            entries.push((key.clone(), status.name.clone()));
        }
    }

    // Deduplicate: show highest-priority source for each key
    let mut seen = std::collections::HashSet::new();
    for (key, source) in &entries {
        if seen.insert(key.clone()) {
            println!("{key}  ({source})");
        }
    }

    Ok(())
}

pub async fn delete(key: &str) -> Result<()> {
    let creds = store();
    let deleted_from = creds.delete(key)?;
    if deleted_from.is_empty() {
        println!("{key} not found in any backend");
    } else {
        let sources: Vec<String> = deleted_from.iter().map(|s| s.to_string()).collect();
        println!("deleted {key} from: {}", sources.join(", "));
    }
    Ok(())
}

pub async fn status() -> Result<()> {
    let creds = store();
    let statuses = creds.status();

    println!("{:<20} {:<10} {}", "Backend", "Available", "Keys");
    println!("{}", "─".repeat(60));

    for status in &statuses {
        let avail = if status.available { "yes" } else { "no" };
        let keys = if status.keys.is_empty() {
            "(none)".to_string()
        } else {
            status.keys.join(", ")
        };
        println!("{:<20} {:<10} {}", status.name, avail, keys);
    }

    Ok(())
}
