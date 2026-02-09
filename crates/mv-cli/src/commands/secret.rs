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

// ---------------------------------------------------------------------------
// Policy subcommands (hit the REST API)
// ---------------------------------------------------------------------------

const BASE_URL: &str = "http://127.0.0.1:9470";

fn encode_query_value(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            _ => format!("%{:02X}", c as u32),
        })
        .collect()
}

fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("failed to create HTTP client")
}

pub async fn policy_set(
    key: &str,
    consumer: &str,
    allow: bool,
    ttl: Option<i64>,
) -> Result<()> {
    let body = serde_json::json!({
        "secret_key": key,
        "consumer": consumer,
        "allowed": allow,
        "max_ttl_seconds": ttl,
    });

    let resp = http_client()
        .post(format!("{BASE_URL}/api/v1/policies"))
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("server returned {status}: {body}");
    }

    let result: serde_json::Value = resp.json().await?;
    let id = result["id"].as_str().unwrap_or("?");
    let action = if allow { "allow" } else { "deny" };
    println!("policy {id}: {action} {consumer} -> {key}");
    if let Some(t) = ttl {
        println!("  max TTL: {t}s");
    }

    Ok(())
}

pub async fn policy_list(secret: Option<&str>, consumer: Option<&str>) -> Result<()> {
    let mut url = format!("{BASE_URL}/api/v1/policies");
    let mut params = Vec::new();
    if let Some(s) = secret {
        params.push(format!("secret_key={}", encode_query_value(s)));
    }
    if let Some(c) = consumer {
        params.push(format!("consumer={}", encode_query_value(c)));
    }
    if !params.is_empty() {
        url = format!("{}?{}", url, params.join("&"));
    }

    let resp = http_client().get(&url).send().await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("server returned {status}: {body}");
    }

    let policies: Vec<serde_json::Value> = resp.json().await?;

    if policies.is_empty() {
        println!("no policies found");
        return Ok(());
    }

    println!(
        "{:<38} {:<20} {:<20} {:<8} {}",
        "ID", "Secret", "Consumer", "Allowed", "Expires"
    );
    println!("{}", "-".repeat(110));

    for p in &policies {
        let id = p["id"].as_str().unwrap_or("?");
        let secret_key = p["secret_key"].as_str().unwrap_or("?");
        let consumer_name = p["consumer"].as_str().unwrap_or("?");
        let allowed = if p["allowed"].as_bool().unwrap_or(false) {
            "yes"
        } else {
            "no"
        };
        let expires = p["expires_at"]
            .as_str()
            .unwrap_or("never");
        println!("{:<38} {:<20} {:<20} {:<8} {}", id, secret_key, consumer_name, allowed, expires);
    }

    Ok(())
}

pub async fn policy_delete(id: &str) -> Result<()> {
    let resp = http_client()
        .delete(format!("{BASE_URL}/api/v1/policies/{id}"))
        .send()
        .await?;

    if resp.status().as_u16() == 204 {
        println!("policy {id} deleted");
    } else if resp.status().as_u16() == 404 {
        anyhow::bail!("policy {id} not found");
    } else {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("server returned {status}: {body}");
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
