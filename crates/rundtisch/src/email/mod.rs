use std::fmt;
#[cfg(test)]
use std::sync::{Arc, Mutex};

#[cfg(feature = "sendmail")]
mod sendmail;
#[cfg(feature = "sendmail")]
pub use sendmail::SendmailEmailSender;

/// Error returned when outbound email cannot be delivered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmailSendError {
    message: String,
}

impl EmailSendError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for EmailSendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for EmailSendError {}

/// Outbound email transport used by handlers that need to notify users.
#[async_trait::async_trait]
pub trait EmailSender: Send + Sync {
    async fn send(&self, to: &str, subject: &str, body: &str) -> Result<(), EmailSendError>;
}

/// In-memory `EmailSender` for tests. Records each successful send.
#[cfg(test)]
#[derive(Clone, Default)]
pub struct RecordingEmailSender {
    messages: Arc<Mutex<Vec<RecordedEmail>>>,
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedEmail {
    pub to: String,
    pub subject: String,
    pub body: String,
}

#[cfg(test)]
impl RecordingEmailSender {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn messages(&self) -> Vec<RecordedEmail> {
        self.messages.lock().expect("email inbox").clone()
    }
}

#[cfg(test)]
#[async_trait::async_trait]
impl EmailSender for RecordingEmailSender {
    async fn send(&self, to: &str, subject: &str, body: &str) -> Result<(), EmailSendError> {
        self.messages
            .lock()
            .map_err(|_| EmailSendError::new("email inbox poisoned"))?
            .push(RecordedEmail {
                to: to.to_string(),
                subject: subject.to_string(),
                body: body.to_string(),
            });
        Ok(())
    }
}
