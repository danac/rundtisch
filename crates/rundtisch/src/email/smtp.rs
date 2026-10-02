use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Tokio1Executor};

use super::{EmailSendError, EmailSender, build_plain_message, mail_from_env};

const AUTH_SMTP_URL: &str = "AUTH_SMTP_URL";
const AUTH_SMTP_USERNAME: &str = "AUTH_SMTP_USERNAME";
const AUTH_SMTP_PASSWORD: &str = "AUTH_SMTP_PASSWORD";

/// Delivers mail over SMTP with explicit STARTTLS (lettre + rustls).
///
/// Uses `tokio1-rustls`, `ring`, and `webpki-roots` so there is no native-tls /
/// OpenSSL dependency (suitable for Wasmer Edge).
#[derive(Clone)]
pub struct SmtpEmailSender {
    from: Mailbox,
    mailer: AsyncSmtpTransport<Tokio1Executor>,
}

impl std::fmt::Debug for SmtpEmailSender {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SmtpEmailSender")
            .field("from", &self.from)
            .finish_non_exhaustive()
    }
}

impl SmtpEmailSender {
    pub fn new(from: Mailbox, mailer: AsyncSmtpTransport<Tokio1Executor>) -> Self {
        Self { from, mailer }
    }

    /// Build a STARTTLS client from `AUTH_SMTP_URL`, `AUTH_SMTP_USERNAME`,
    /// `AUTH_SMTP_PASSWORD`, and optional `AUTH_MAIL_FROM`.
    pub fn from_env() -> Result<Self, EmailSendError> {
        let url = require_env(AUTH_SMTP_URL)?;
        let username = require_env(AUTH_SMTP_USERNAME)?;
        let password = require_env(AUTH_SMTP_PASSWORD)?;
        let from = mail_from_env()?;
        let (host, port) = parse_smtp_url(&url)?;

        let mut builder = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&host)
            .map_err(|err| EmailSendError::new(format!("invalid SMTP relay {host:?}: {err}")))?
            .credentials(Credentials::new(username, password));
        if let Some(port) = port {
            builder = builder.port(port);
        }
        Ok(Self::new(from, builder.build()))
    }
}

#[async_trait::async_trait]
impl EmailSender for SmtpEmailSender {
    async fn send(&self, to: &str, subject: &str, body: &str) -> Result<(), EmailSendError> {
        let message = build_plain_message(self.from.clone(), to, subject, body)?;
        self.mailer
            .send(message)
            .await
            .map(|_| ())
            .map_err(|err| EmailSendError::new(format!("SMTP send failed: {err}")))
    }
}

fn require_env(name: &str) -> Result<String, EmailSendError> {
    let value = std::env::var(name)
        .map_err(|_| EmailSendError::new(format!("missing required environment variable {name}")))?;
    if value.trim().is_empty() {
        return Err(EmailSendError::new(format!(
            "environment variable {name} must not be empty"
        )));
    }
    Ok(value)
}

/// Accept a bare hostname or an `smtp://[user@]host[:port]` URL.
fn parse_smtp_url(raw: &str) -> Result<(String, Option<u16>), EmailSendError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(EmailSendError::new(format!(
            "{AUTH_SMTP_URL} must not be empty"
        )));
    }
    let authority = match trimmed.split_once("://") {
        Some((_, rest)) => rest.split('/').next().unwrap_or(rest),
        None => trimmed,
    };
    let hostport = match authority.rsplit_once('@') {
        Some((_, hostport)) => hostport,
        None => authority,
    };
    if hostport.is_empty() {
        return Err(EmailSendError::new(format!(
            "{AUTH_SMTP_URL} must include a host ({trimmed:?})"
        )));
    }
    if let Some((host, port)) = hostport.rsplit_once(':') {
        if !host.is_empty() && !port.is_empty() && port.chars().all(|c| c.is_ascii_digit()) {
            let port: u16 = port.parse().map_err(|err| {
                EmailSendError::new(format!("invalid SMTP port in {AUTH_SMTP_URL}: {err}"))
            })?;
            return Ok((host.to_owned(), Some(port)));
        }
    }
    Ok((hostport.to_owned(), None))
}

#[cfg(test)]
mod tests {
    use super::parse_smtp_url;

    #[test]
    fn parses_bare_host() {
        assert_eq!(
            parse_smtp_url("smtp.example.com").unwrap(),
            ("smtp.example.com".to_owned(), None)
        );
    }

    #[test]
    fn parses_url_with_port() {
        assert_eq!(
            parse_smtp_url("smtp://smtp.example.com:2525").unwrap(),
            ("smtp.example.com".to_owned(), Some(2525))
        );
    }
}
