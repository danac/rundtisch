use lettre::Message;
use lettre::message::{Mailbox, header::ContentType};

use super::EmailSendError;

pub(crate) const DEFAULT_MAIL_FROM: &str = "rundtisch@localhost";
pub(crate) const AUTH_MAIL_FROM: &str = "AUTH_MAIL_FROM";

pub(crate) fn mail_from_env() -> Result<Mailbox, EmailSendError> {
    let raw = std::env::var(AUTH_MAIL_FROM).unwrap_or_else(|_| DEFAULT_MAIL_FROM.to_owned());
    raw.parse().map_err(|err| {
        EmailSendError::new(format!("invalid {AUTH_MAIL_FROM} ({raw:?}): {err}"))
    })
}

pub(crate) fn build_plain_message(
    from: Mailbox,
    to: &str,
    subject: &str,
    body: &str,
) -> Result<Message, EmailSendError> {
    let to: Mailbox = to
        .parse()
        .map_err(|err| EmailSendError::new(format!("invalid recipient {to:?}: {err}")))?;
    Message::builder()
        .from(from)
        .to(to)
        .subject(subject)
        .header(ContentType::TEXT_PLAIN)
        .body(body.to_owned())
        .map_err(|err| EmailSendError::new(format!("failed to build email: {err}")))
}
