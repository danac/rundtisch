pub mod app;
pub mod email;
#[cfg(feature = "auth")]
pub mod auth;

pub use app::{AppState, SecretError};
pub use email::{EmailSendError, EmailSender};
#[cfg(feature = "sendmail")]
pub use email::SendmailEmailSender;
