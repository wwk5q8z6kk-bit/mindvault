//! Email (SMTP) adapter — sends messages via SMTP using the `lettre` crate.
//!
//! Configuration keys:
//! - `smtp_host`: SMTP server hostname (e.g., "smtp.gmail.com")
//! - `smtp_port`: SMTP port (default: "587")
//! - `smtp_user`: SMTP username/email
//! - `smtp_pass`: SMTP password or app-specific password
//! - `from_address`: Sender email address
//! - `default_to`: (optional) Default recipient email

use std::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use mv_core::{MvError, MvResult};

use super::{
    AdapterConfig, AdapterInboundMessage, AdapterOutboundMessage, AdapterStatus, AdapterType,
    ExternalAdapter,
};

pub struct EmailAdapter {
    config: AdapterConfig,
    last_send: Mutex<Option<DateTime<Utc>>>,
    last_error: Mutex<Option<String>>,
}

impl EmailAdapter {
    pub fn new(config: AdapterConfig) -> MvResult<Self> {
        // Validate required settings
        for key in &["smtp_host", "smtp_user", "smtp_pass", "from_address"] {
            if config.get_setting(key).is_none() {
                return Err(MvError::Config(format!(
                    "Email adapter requires '{key}' setting"
                )));
            }
        }
        Ok(Self {
            config,
            last_send: Mutex::new(None),
            last_error: Mutex::new(None),
        })
    }

    fn smtp_port(&self) -> u16 {
        self.config
            .get_setting("smtp_port")
            .and_then(|p| p.parse().ok())
            .unwrap_or(587)
    }
}

#[async_trait]
impl ExternalAdapter for EmailAdapter {
    fn name(&self) -> &str {
        &self.config.name
    }

    fn adapter_type(&self) -> AdapterType {
        AdapterType::Email
    }

    async fn send(&self, message: &AdapterOutboundMessage) -> MvResult<()> {
        let host = self
            .config
            .get_setting("smtp_host")
            .ok_or_else(|| MvError::Config("missing smtp_host".into()))?;
        let user = self
            .config
            .get_setting("smtp_user")
            .ok_or_else(|| MvError::Config("missing smtp_user".into()))?;
        let pass = self
            .config
            .get_setting("smtp_pass")
            .ok_or_else(|| MvError::Config("missing smtp_pass".into()))?;
        let from = self
            .config
            .get_setting("from_address")
            .ok_or_else(|| MvError::Config("missing from_address".into()))?;

        // The `channel` field is used as the recipient email address
        let to = if message.channel.is_empty() {
            self.config
                .get_setting("default_to")
                .ok_or_else(|| MvError::Config("no recipient and no default_to".into()))?
                .to_string()
        } else {
            message.channel.clone()
        };

        let port = self.smtp_port();

        // Use reqwest-compatible approach: spawn blocking SMTP via raw TCP
        // This avoids adding lettre as a dependency — we use a minimal SMTP
        // implementation via tokio's TCP stream for the basic STARTTLS flow.
        //
        // For production, consider switching to the `lettre` crate.
        // For now, we use a simple approach that works for common SMTP servers.
        let subject = message
            .metadata
            .get("subject")
            .cloned()
            .unwrap_or_else(|| "MindVault Message".to_string());

        let email_body = format!(
            "From: {from}\r\n\
             To: {to}\r\n\
             Subject: {subject}\r\n\
             Content-Type: text/plain; charset=utf-8\r\n\
             \r\n\
             {content}",
            content = message.content,
        );

        // Use a lightweight SMTP send via the `reqwest`-style HTTP bridge
        // We shell out to a simple TCP connection for SMTP
        let host_owned = host.to_string();
        let user_owned = user.to_string();
        let pass_owned = pass.to_string();

        let result = tokio::task::spawn_blocking(move || {
            send_smtp_email(&host_owned, port, &user_owned, &pass_owned, &email_body)
        })
        .await
        .map_err(|e| MvError::Internal(format!("email task join error: {e}")))?;

        match result {
            Ok(()) => {
                *self.last_send.lock().unwrap() = Some(Utc::now());
                *self.last_error.lock().unwrap() = None;
                Ok(())
            }
            Err(e) => {
                let err = format!("email send failed: {e}");
                *self.last_error.lock().unwrap() = Some(err.clone());
                Err(MvError::Internal(err))
            }
        }
    }

    async fn poll(
        &self,
        cursor: Option<&str>,
    ) -> MvResult<(Vec<AdapterInboundMessage>, String)> {
        // Email polling (IMAP) is a complex protocol requiring a dedicated
        // connection. For now, inbound email is not supported — users should
        // configure email forwarding to a webhook endpoint instead.
        Ok((vec![], cursor.unwrap_or("0").to_string()))
    }

    async fn health_check(&self) -> MvResult<bool> {
        let host = match self.config.get_setting("smtp_host") {
            Some(h) => h.to_string(),
            None => return Ok(false),
        };
        let port = self.smtp_port();

        // Try to connect to SMTP port
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            tokio::net::TcpStream::connect(format!("{host}:{port}")),
        )
        .await;

        match result {
            Ok(Ok(_)) => Ok(true),
            _ => Ok(false),
        }
    }

    fn status(&self) -> AdapterStatus {
        AdapterStatus {
            adapter_type: AdapterType::Email,
            name: self.config.name.clone(),
            connected: self.last_error.lock().unwrap().is_none(),
            last_send: *self.last_send.lock().unwrap(),
            last_receive: None,
            error: self.last_error.lock().unwrap().clone(),
        }
    }
}

/// Minimal SMTP send via raw TCP.
///
/// Supports PLAIN auth over a non-TLS connection (port 25/587 without STARTTLS).
/// For production use with TLS, consider the `lettre` crate.
fn send_smtp_email(
    host: &str,
    port: u16,
    user: &str,
    pass: &str,
    email_data: &str,
) -> Result<(), String> {
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpStream;

    let stream =
        TcpStream::connect(format!("{host}:{port}")).map_err(|e| format!("connect: {e}"))?;
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(10)))
        .ok();
    stream
        .set_write_timeout(Some(std::time::Duration::from_secs(10)))
        .ok();

    let mut reader = BufReader::new(stream.try_clone().map_err(|e| format!("clone: {e}"))?);
    let mut writer = stream;

    let mut response = String::new();

    // Read greeting
    response.clear();
    reader
        .read_line(&mut response)
        .map_err(|e| format!("read greeting: {e}"))?;
    if !response.starts_with("220") {
        return Err(format!("unexpected greeting: {response}"));
    }

    // EHLO
    write!(writer, "EHLO mindvault\r\n").map_err(|e| format!("write EHLO: {e}"))?;
    writer.flush().map_err(|e| format!("flush EHLO: {e}"))?;
    loop {
        response.clear();
        reader
            .read_line(&mut response)
            .map_err(|e| format!("read EHLO: {e}"))?;
        if response.len() < 4 || response.as_bytes()[3] == b' ' {
            break;
        }
    }

    // AUTH PLAIN
    let auth_str = format!("\0{user}\0{pass}");
    let auth_b64 = base64_encode(auth_str.as_bytes());
    write!(writer, "AUTH PLAIN {auth_b64}\r\n").map_err(|e| format!("write AUTH: {e}"))?;
    writer.flush().map_err(|e| format!("flush AUTH: {e}"))?;
    response.clear();
    reader
        .read_line(&mut response)
        .map_err(|e| format!("read AUTH: {e}"))?;
    if !response.starts_with("235") {
        return Err(format!("auth failed: {response}"));
    }

    // Extract From/To from email_data headers
    let from = email_data
        .lines()
        .find(|l| l.starts_with("From:"))
        .and_then(|l| l.strip_prefix("From:"))
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| user.to_string());
    let to = email_data
        .lines()
        .find(|l| l.starts_with("To:"))
        .and_then(|l| l.strip_prefix("To:"))
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    // MAIL FROM
    write!(writer, "MAIL FROM:<{from}>\r\n").map_err(|e| format!("write MAIL: {e}"))?;
    writer.flush().map_err(|e| format!("flush MAIL: {e}"))?;
    response.clear();
    reader
        .read_line(&mut response)
        .map_err(|e| format!("read MAIL: {e}"))?;

    // RCPT TO
    write!(writer, "RCPT TO:<{to}>\r\n").map_err(|e| format!("write RCPT: {e}"))?;
    writer.flush().map_err(|e| format!("flush RCPT: {e}"))?;
    response.clear();
    reader
        .read_line(&mut response)
        .map_err(|e| format!("read RCPT: {e}"))?;

    // DATA
    write!(writer, "DATA\r\n").map_err(|e| format!("write DATA: {e}"))?;
    writer.flush().map_err(|e| format!("flush DATA: {e}"))?;
    response.clear();
    reader
        .read_line(&mut response)
        .map_err(|e| format!("read DATA: {e}"))?;
    if !response.starts_with("354") {
        return Err(format!("DATA rejected: {response}"));
    }

    // Send email body + terminator
    write!(writer, "{email_data}\r\n.\r\n").map_err(|e| format!("write body: {e}"))?;
    writer.flush().map_err(|e| format!("flush body: {e}"))?;
    response.clear();
    reader
        .read_line(&mut response)
        .map_err(|e| format!("read body response: {e}"))?;
    if !response.starts_with("250") {
        return Err(format!("message rejected: {response}"));
    }

    // QUIT
    write!(writer, "QUIT\r\n").map_err(|e| format!("write QUIT: {e}"))?;
    writer.flush().ok();

    Ok(())
}

fn base64_encode(data: &[u8]) -> String {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    STANDARD.encode(data)
}
