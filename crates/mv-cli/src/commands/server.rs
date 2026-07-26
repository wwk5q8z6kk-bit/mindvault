use anyhow::Result;

pub async fn start(
    port: Option<u16>,
    grpc_port: Option<u16>,
    foreground: bool,
    config_path: &str,
) -> Result<()> {
    let runtime = super::load_runtime_config(config_path)?;

    let bind_host = runtime.server.bind_host.clone();
    let rest_port = port.unwrap_or(runtime.server.rest_port);
    let grpc_port = grpc_port.unwrap_or(runtime.server.grpc_port);

    if let Err(err) = mv_server::check_bind_safety(&bind_host) {
        eprintln!("startup safety check failed:");
        eprintln!("  {err}");
        eprintln!("hint: run `mv server preflight --config {config_path}`");
        return Err(anyhow::anyhow!("refusing to start unsafe public bind"));
    }

    let server_config = mv_server::ServerConfig {
        bind_host: bind_host.clone(),
        rest_port,
        grpc_port,
        socket_path: runtime.server.socket_path,
        cors_allowed_origins: runtime.server.cors_allowed_origins,
        engine_config: runtime.engine,
    };

    if foreground {
        println!("starting MindVault server (foreground)...");
    } else {
        println!("starting MindVault server...");
        println!("note: daemon mode is not implemented; process stays attached");
    }

    println!("  REST: http://{bind_host}:{rest_port}");
    println!("  gRPC: {bind_host}:{grpc_port}");

    mv_server::start_server(server_config)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    Ok(())
}

pub async fn preflight(config_path: &str) -> Result<()> {
    let runtime = super::load_runtime_config(config_path)?;
    let bind_host = runtime.server.bind_host.clone();

    let shared_token = has_non_empty_env("MINDVAULT_AUTH_TOKEN");
    let jwt_secret = has_non_empty_env("MINDVAULT_JWT_SECRET");
    let allow_insecure = truthy_env("MINDVAULT_ALLOW_INSECURE_BIND");

    println!("server preflight");
    println!("  bind_host: {bind_host}");
    println!("  MINDVAULT_AUTH_TOKEN set: {shared_token}");
    println!("  MINDVAULT_JWT_SECRET set: {jwt_secret}");
    println!("  MINDVAULT_ALLOW_INSECURE_BIND: {allow_insecure}");

    let bind_result = mv_server::check_bind_safety(&bind_host);
    let sealed_result = evaluate_sealed_storage_preflight(&runtime.engine);

    match &bind_result {
        Ok(()) => println!("  bind safety: PASS"),
        Err(err) => {
            println!("  bind safety: FAIL");
            println!("    {err}");
        }
    }

    match &sealed_result {
        Ok(()) => println!("  sealed storage: PASS"),
        Err(reason) => {
            println!("  sealed storage: FAIL");
            println!("    {reason}");
        }
    }

    if bind_result.is_ok() && sealed_result.is_ok() {
        println!("result: PASS");
        return Ok(());
    }

    println!("result: FAIL");
    println!("next steps:");
    if bind_result.is_err() {
        println!("  1) Set MINDVAULT_AUTH_TOKEN or MINDVAULT_JWT_SECRET");
        println!("  2) Or set MINDVAULT_ALLOW_INSECURE_BIND=true only for trusted local networks");
    }
    if sealed_result.is_err() {
        println!("  3) Run `mv keychain doctor` to inspect plaintext artifacts");
        println!("  4) Run `mv keychain migrate-sealed --passphrase <pw>` to encrypt legacy data");
    }
    Err(anyhow::anyhow!("preflight failed"))
}

fn evaluate_sealed_storage_preflight(
    config: &mv_engine::config::EngineConfig,
) -> Result<(), String> {
    let report = mv_server::scan_sealed_storage(config).map_err(|err| err.to_string())?;
    if report.is_clean() {
        return Ok(());
    }
    Err(report.summary())
}

pub async fn migrate_sealed(
    from_env: bool,
    passphrase: Option<&str>,
    config_path: &str,
) -> Result<()> {
    let config = super::load_config(config_path)?;
    if !config.sealed_mode {
        return Err(anyhow::anyhow!(
            "sealed_mode is not enabled in config; migration is only needed for sealed mode"
        ));
    }

    println!("sealed migration: initializing engine (skipping preflight)...");
    let engine = mv_engine::engine::MindVaultEngine::init(config).await?;

    let (state, _meta) = engine.keychain.vault_status().await?;
    let needs_init = matches!(state, mv_core::model::keychain::VaultState::Uninitialized);

    let password = if let Some(pw) = passphrase {
        pw.to_string()
    } else if from_env {
        std::env::var("MINDVAULT_VAULT_PASSWORD")
            .map_err(|_| anyhow::anyhow!("MINDVAULT_VAULT_PASSWORD not set"))?
    } else {
        return Err(anyhow::anyhow!(
            "provide --passphrase or --from-env to supply the vault password"
        ));
    };

    if needs_init {
        println!("sealed migration: vault not initialized, initializing...");
        engine
            .keychain
            .initialize_vault(&password, false, "migrate-sealed")
            .await
            .map_err(|e| anyhow::anyhow!("vault init failed: {e}"))?;
        println!("sealed migration: vault initialized and unsealed");
    } else {
        println!("sealed migration: unsealing vault...");
        engine
            .keychain
            .unseal(&password, "migrate-sealed")
            .await
            .map_err(|e| anyhow::anyhow!("unseal failed: {e}"))?;
        println!("sealed migration: vault unsealed");
    }

    println!("sealed migration: migrating storage (encrypting plaintext blobs, removing legacy indexes)...");
    engine
        .migrate_sealed_storage()
        .await
        .map_err(|e| anyhow::anyhow!("storage migration failed: {e}"))?;

    println!("sealed migration: rebuilding runtime indexes...");
    engine
        .rebuild_runtime_indexes()
        .await
        .map_err(|e| anyhow::anyhow!("index rebuild failed: {e}"))?;

    engine
        .keychain
        .seal("migrate-sealed")
        .await
        .map_err(|e| anyhow::anyhow!("seal failed: {e}"))?;

    println!("sealed migration: complete — vault re-sealed");
    println!("you can now start the server normally with `mv server start`");
    Ok(())
}

pub async fn stop(config_path: &str) -> Result<()> {
    let url = health_url(config_path)?;

    let client = reqwest::Client::new();
    match client.get(&url).send().await {
        Ok(_) => {
            println!("server appears to be running at {url}");
            println!("stop by terminating the server process");
            println!("  pkill -f 'mv server start'");
        }
        Err(_) => {
            println!("server is not running ({url})");
        }
    }
    Ok(())
}

pub async fn status(config_path: &str) -> Result<()> {
    let url = health_url(config_path)?;
    let diagnostics_url = embedding_diagnostics_url(config_path)?;

    let client = reqwest::Client::new();
    match client.get(&url).send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                let body: serde_json::Value = resp.json().await?;
                println!("status: running");
                println!("  endpoint: {url}");
                if let Some(count) = body.get("node_count") {
                    println!("  nodes: {count}");
                }
                if let Some(version) = body.get("version").and_then(|v| v.as_str()) {
                    println!("  version: {version}");
                }

                if let Ok(diag_resp) = client.get(&diagnostics_url).send().await {
                    if diag_resp.status().is_success() {
                        let diag: serde_json::Value = diag_resp.json().await?;
                        print_embedding_diagnostics(&diag);
                    }
                }
            } else {
                println!("status: error (HTTP {})", resp.status());
                println!("  endpoint: {url}");
            }
        }
        Err(_) => {
            println!("status: stopped");
            println!("  endpoint: {url}");
        }
    }
    Ok(())
}

fn print_embedding_diagnostics(diagnostics: &serde_json::Value) {
    let configured_provider = diagnostics
        .get("configured_provider")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let configured_model = diagnostics
        .get("configured_model")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let configured_dimensions = diagnostics
        .get("configured_dimensions")
        .and_then(|v| v.as_u64())
        .unwrap_or_default();
    let effective_provider = diagnostics
        .get("effective_provider")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let effective_model = diagnostics
        .get("effective_model")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let effective_dimensions = diagnostics
        .get("effective_dimensions")
        .and_then(|v| v.as_u64())
        .unwrap_or_default();
    let fallback_to_noop = diagnostics
        .get("fallback_to_noop")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let local_embeddings_feature_enabled = diagnostics
        .get("local_embeddings_feature_enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    println!("  embedding:");
    println!(
        "    configured: provider={configured_provider}, model={configured_model}, dims={configured_dimensions}"
    );
    println!(
        "    effective: provider={effective_provider}, model={effective_model}, dims={effective_dimensions}"
    );
    println!("    fallback_to_noop: {fallback_to_noop}");
    println!("    local_embeddings_feature_enabled: {local_embeddings_feature_enabled}");

    if let Some(reason) = diagnostics.get("reason").and_then(|v| v.as_str()) {
        println!("    reason: {reason}");
    }
}

fn health_url(config_path: &str) -> Result<String> {
    let runtime = super::load_runtime_config(config_path)?;
    let host = local_rest_host(&runtime.server.bind_host);
    Ok(format!(
        "http://{host}:{}/api/v1/health",
        runtime.server.rest_port
    ))
}

fn embedding_diagnostics_url(config_path: &str) -> Result<String> {
    let runtime = super::load_runtime_config(config_path)?;
    let host = local_rest_host(&runtime.server.bind_host);
    Ok(format!(
        "http://{host}:{}/api/v1/diagnostics/embedding",
        runtime.server.rest_port
    ))
}

fn local_rest_host(bind_host: &str) -> &str {
    if bind_host == "0.0.0.0" {
        "127.0.0.1"
    } else {
        bind_host
    }
}

fn has_non_empty_env(key: &str) -> bool {
    std::env::var(key)
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
}

fn truthy_env(key: &str) -> bool {
    let value = std::env::var(key).unwrap_or_default();
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use mv_engine::config::EngineConfig;

    fn unique_test_dir(prefix: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("mindvault-{prefix}-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create test dir");
        dir
    }

    #[test]
    fn truthy_env_parses_common_truthy_values() {
        unsafe {
            std::env::set_var("MV_TEST_BOOL", "yes");
        }
        assert!(truthy_env("MV_TEST_BOOL"));
        unsafe {
            std::env::set_var("MV_TEST_BOOL", "0");
        }
        assert!(!truthy_env("MV_TEST_BOOL"));
        unsafe {
            std::env::remove_var("MV_TEST_BOOL");
        }
        assert!(!truthy_env("MV_TEST_BOOL"));
    }

    #[test]
    fn sealed_storage_preflight_detects_legacy_artifacts() {
        let temp_dir = unique_test_dir("sealed-preflight-fail");
        std::fs::create_dir_all(temp_dir.join("tantivy")).expect("create legacy dir");

        let config = EngineConfig {
            data_dir: temp_dir.to_string_lossy().to_string(),
            ..Default::default()
        };
        let err = evaluate_sealed_storage_preflight(&config).expect_err("must fail");
        assert!(err.contains("legacy index directory present"));
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn sealed_storage_preflight_passes_clean_data_dir() {
        let temp_dir = unique_test_dir("sealed-preflight-pass");
        let config = EngineConfig {
            data_dir: temp_dir.to_string_lossy().to_string(),
            ..Default::default()
        };
        evaluate_sealed_storage_preflight(&config).expect("clean scan should pass");
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
