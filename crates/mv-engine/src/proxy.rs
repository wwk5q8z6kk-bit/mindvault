use std::collections::HashMap;
use std::net::IpAddr;
use std::time::Duration;

use mv_core::*;

use crate::engine::MindVaultEngine;
use crate::sanitize::OutputSanitizer;

/// Default commands allowed in exec proxy.
const DEFAULT_EXEC_ALLOWLIST: &[&str] = &[
    "gh", "git", "curl", "aws", "gcloud", "npm", "cargo",
];

/// Constitutional deny patterns checked before any policy evaluation.
/// These patterns are always blocked regardless of policy.
const CONSTITUTIONAL_DENY_PATTERNS: &[&str] = &[
    "rm -rf /",
    "dd if=/dev/zero",
    "mkfs.",
    ":(){:|:&};:",
    "chmod -R 777 /",
];

/// Private IP ranges blocked for SSRF prevention.
fn is_private_ip(addr: IpAddr) -> bool {
    match addr {
        IpAddr::V4(v4) => {
            v4.is_loopback()           // 127.0.0.0/8
            || v4.is_private()         // 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16
            || v4.is_link_local()      // 169.254.0.0/16
            || v4.is_unspecified()     // 0.0.0.0
        }
        IpAddr::V6(v6) => {
            v6.is_loopback()           // ::1
            || v6.is_unspecified()     // ::
            // fd00::/8 (unique local)
            || (v6.segments()[0] & 0xff00) == 0xfd00
            // fe80::/10 (link-local)
            || (v6.segments()[0] & 0xffc0) == 0xfe80
        }
    }
}

/// Check if a URL targets a private IP (SSRF prevention).
fn check_ssrf(url: &str) -> Result<(), String> {
    let parsed = url::Url::parse(url).map_err(|e| format!("invalid URL: {e}"))?;

    let host = parsed
        .host_str()
        .ok_or_else(|| "URL has no host".to_string())?;

    // Try parsing as IP directly
    if let Ok(ip) = host.parse::<IpAddr>() {
        if is_private_ip(ip) {
            return Err(format!("blocked: private IP address {host}"));
        }
    }

    // Check well-known private hostnames
    let host_lower = host.to_ascii_lowercase();
    if host_lower == "localhost"
        || host_lower.ends_with(".local")
        || host_lower.ends_with(".internal")
        || host_lower == "metadata.google.internal"
        || host_lower == "169.254.169.254"
    {
        return Err(format!("blocked: private hostname {host}"));
    }

    // Attempt DNS resolution to catch private IPs behind public hostnames.
    if let Ok(addrs) = std::net::ToSocketAddrs::to_socket_addrs(&(host, 80)) {
        for addr in addrs {
            if is_private_ip(addr.ip()) {
                return Err(format!(
                    "blocked: hostname {host} resolves to private IP {}",
                    addr.ip()
                ));
            }
        }
    }

    Ok(())
}

/// Check constitutional deny patterns against a request description.
fn check_constitutional_rules(description: &str) -> Result<(), String> {
    let lower = description.to_ascii_lowercase();
    for pattern in CONSTITUTIONAL_DENY_PATTERNS {
        if lower.contains(pattern) {
            return Err(format!(
                "blocked by constitutional rule: matches deny pattern '{pattern}'"
            ));
        }
    }
    Ok(())
}

/// Proxy engine for HTTP and command operations with credential injection.
pub struct ProxyEngine;

impl ProxyEngine {
    /// Execute an HTTP proxy request with credential injection.
    ///
    /// Flow: constitutional check -> SSRF check -> policy check -> inject -> execute -> sanitize -> audit
    pub async fn execute_http(
        req: &HttpProxyRequest,
        consumer: &str,
        engine: &MindVaultEngine,
    ) -> Result<HttpProxyResponse, String> {
        // 1. Constitutional rules
        let description = format!("{} {} intent={}", req.method, req.url, req.intent);
        check_constitutional_rules(&description)?;

        // 2. SSRF prevention
        check_ssrf(&req.url)?;

        // 3. Policy check
        let decision = engine
            .check_policy(&req.secret_ref, consumer)
            .await
            .map_err(|e| format!("policy check failed: {e}"))?;

        if !decision.is_allowed() {
            return Err(format!(
                "access denied: consumer '{consumer}' has no policy granting access to '{}'",
                req.secret_ref
            ));
        }

        // 4. Resolve the secret value
        let secret_value = engine
            .credential_store
            .get(&req.secret_ref)
            .map_err(|e| format!("credential lookup failed: {e}"))?
            .ok_or_else(|| format!("secret '{}' not found", req.secret_ref))?;
        let secret_str = secret_value.expose().to_string();

        // 5. Create audit entry
        let audit = ProxyAuditEntry::new(
            consumer,
            &req.secret_ref,
            "http",
            &req.url,
            &req.intent,
            format!("{} {}", req.method, req.url),
        );

        let _ = engine.log_proxy_audit(&audit).await;

        // 6. Build and send HTTP request
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .map_err(|e| format!("failed to create HTTP client: {e}"))?;

        let method = reqwest::Method::from_bytes(req.method.as_bytes())
            .map_err(|e| format!("invalid HTTP method: {e}"))?;

        let mut request_builder = client.request(method.clone(), &req.url);

        // Add user-provided headers
        for (key, value) in &req.headers {
            request_builder = request_builder.header(key.as_str(), value.as_str());
        }

        // Inject credential
        request_builder = match &req.inject_as {
            SecretInjection::BearerHeader => {
                request_builder.header("Authorization", format!("Bearer {secret_str}"))
            }
            SecretInjection::BasicAuth { username } => {
                use base64::engine::general_purpose::STANDARD;
                use base64::Engine;
                let encoded = STANDARD.encode(format!("{username}:{secret_str}"));
                request_builder.header("Authorization", format!("Basic {encoded}"))
            }
            SecretInjection::Header { name } => {
                request_builder.header(name.as_str(), secret_str.as_str())
            }
            SecretInjection::QueryParam { name } => {
                let mut url =
                    url::Url::parse(&req.url).map_err(|e| format!("invalid URL: {e}"))?;
                url.query_pairs_mut().append_pair(name, &secret_str);
                // Rebuild the request with the modified URL
                let mut rebuilt = client.request(method, url.as_str());
                for (key, value) in &req.headers {
                    rebuilt = rebuilt.header(key.as_str(), value.as_str());
                }
                rebuilt
            }
        };

        // Add body if present
        if let Some(body) = &req.body {
            request_builder = request_builder.body(body.clone());
        }

        let result = request_builder.send().await;

        match result {
            Ok(response) => {
                let status = response.status().as_u16();
                let resp_headers: HashMap<String, String> = response
                    .headers()
                    .iter()
                    .map(|(k, v)| {
                        (
                            k.as_str().to_string(),
                            v.to_str().unwrap_or("<binary>").to_string(),
                        )
                    })
                    .collect();

                let body_text = response
                    .text()
                    .await
                    .unwrap_or_else(|e| format!("<error reading body: {e}>"));

                // Sanitize output
                let secrets_for_sanitize = vec![(req.secret_ref.clone(), secret_str)];
                let (sanitized_body, was_sanitized) =
                    OutputSanitizer::sanitize(&body_text, &secrets_for_sanitize);

                // Update audit
                let _ = engine
                    .update_proxy_audit(audit.id, true, was_sanitized, None, Some(status as i32))
                    .await;

                Ok(HttpProxyResponse {
                    status,
                    headers: resp_headers,
                    body: sanitized_body,
                    sanitized: was_sanitized,
                })
            }
            Err(e) => {
                let err_msg = format!("HTTP request failed: {e}");
                let _ = engine
                    .update_proxy_audit(audit.id, false, false, Some(&err_msg), None)
                    .await;
                Err(err_msg)
            }
        }
    }

    /// Execute a command proxy request with credential env injection.
    ///
    /// Flow: constitutional check -> allowlist check -> policy check -> inject env -> run -> sanitize -> audit
    pub async fn execute_exec(
        req: &ExecProxyRequest,
        consumer: &str,
        engine: &MindVaultEngine,
    ) -> Result<ExecProxyResponse, String> {
        // 1. Constitutional rules
        let description = format!(
            "{} {} intent={}",
            req.command,
            req.args.join(" "),
            req.intent
        );
        check_constitutional_rules(&description)?;

        // 2. Command allowlist check
        if !DEFAULT_EXEC_ALLOWLIST.contains(&req.command.as_str()) {
            return Err(format!(
                "command '{}' is not in the allowed list: {:?}",
                req.command, DEFAULT_EXEC_ALLOWLIST
            ));
        }

        // 3. Policy check for each secret being injected
        let mut resolved_secrets: Vec<(String, String)> = Vec::new();
        for (env_var, secret_key) in &req.env_inject {
            let decision = engine
                .check_policy(secret_key, consumer)
                .await
                .map_err(|e| format!("policy check failed for '{secret_key}': {e}"))?;

            if !decision.is_allowed() {
                return Err(format!(
                    "access denied: consumer '{consumer}' cannot access secret '{secret_key}'"
                ));
            }

            let secret_value = engine
                .credential_store
                .get(secret_key)
                .map_err(|e| format!("credential lookup failed for '{secret_key}': {e}"))?
                .ok_or_else(|| format!("secret '{secret_key}' not found"))?;

            resolved_secrets.push((env_var.clone(), secret_value.expose().to_string()));
        }

        // Build secret ref string for audit
        let secret_refs: Vec<&str> = req.env_inject.values().map(|v| v.as_str()).collect();
        let secret_ref_str = secret_refs.join(",");

        // 4. Create audit entry
        let audit = ProxyAuditEntry::new(
            consumer,
            &secret_ref_str,
            "exec",
            &req.command,
            &req.intent,
            format!("{} {}", req.command, req.args.join(" ")),
        );

        let _ = engine.log_proxy_audit(&audit).await;

        // 5. Build and run process (using tokio::process::Command for safe arg handling)
        let mut cmd = tokio::process::Command::new(&req.command);
        cmd.args(&req.args);

        // Inject secret env vars
        for (env_var, value) in &resolved_secrets {
            cmd.env(env_var, value);
        }

        // Set working directory if specified
        if let Some(ref wd) = req.working_dir {
            cmd.current_dir(wd);
        }

        // Capture output
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let timeout = Duration::from_secs(req.timeout_seconds);

        let child = cmd
            .spawn()
            .map_err(|e| format!("failed to spawn command '{}': {e}", req.command))?;

        let output = tokio::time::timeout(timeout, child.wait_with_output())
            .await
            .map_err(|_| {
                format!(
                    "command '{}' timed out after {}s",
                    req.command, req.timeout_seconds
                )
            })?
            .map_err(|e| format!("command '{}' failed: {e}", req.command))?;

        let stdout_raw = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr_raw = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);

        // 6. Sanitize output — redact all injected secret values
        let all_secrets: Vec<(String, String)> = req
            .env_inject
            .values()
            .zip(resolved_secrets.iter().map(|(_, v)| v))
            .map(|(key, val)| (key.clone(), val.clone()))
            .collect();

        let (sanitized_stdout, stdout_redacted) =
            OutputSanitizer::sanitize(&stdout_raw, &all_secrets);
        let (sanitized_stderr, stderr_redacted) =
            OutputSanitizer::sanitize(&stderr_raw, &all_secrets);
        let was_sanitized = stdout_redacted || stderr_redacted;

        // Update audit
        let success = exit_code == 0;
        let error_msg = if !success {
            Some(format!("exit code {exit_code}"))
        } else {
            None
        };

        let _ = engine
            .update_proxy_audit(
                audit.id,
                success,
                was_sanitized,
                error_msg.as_deref(),
                Some(exit_code),
            )
            .await;

        Ok(ExecProxyResponse {
            exit_code,
            stdout: sanitized_stdout,
            stderr: sanitized_stderr,
            sanitized: was_sanitized,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ssrf_blocks_localhost() {
        assert!(check_ssrf("http://localhost:8080/api").is_err());
    }

    #[test]
    fn ssrf_blocks_private_ipv4() {
        assert!(check_ssrf("http://10.0.0.1/secret").is_err());
        assert!(check_ssrf("http://172.16.0.1/secret").is_err());
        assert!(check_ssrf("http://192.168.1.1/secret").is_err());
        assert!(check_ssrf("http://127.0.0.1:9470/api").is_err());
    }

    #[test]
    fn ssrf_blocks_ipv6_loopback() {
        assert!(check_ssrf("http://[::1]:8080/api").is_err());
    }

    #[test]
    fn ssrf_allows_public_ip() {
        assert!(check_ssrf("https://8.8.8.8/dns-query").is_ok());
    }

    #[test]
    fn ssrf_blocks_metadata_endpoints() {
        assert!(check_ssrf("http://169.254.169.254/latest/meta-data/").is_err());
        assert!(check_ssrf("http://metadata.google.internal/computeMetadata/").is_err());
    }

    #[test]
    fn constitutional_blocks_dangerous_commands() {
        assert!(check_constitutional_rules("rm -rf / --no-preserve-root").is_err());
        assert!(check_constitutional_rules("dd if=/dev/zero of=/dev/sda").is_err());
    }

    #[test]
    fn constitutional_allows_safe_commands() {
        assert!(check_constitutional_rules("git status").is_ok());
        assert!(check_constitutional_rules("cargo build").is_ok());
    }
}
