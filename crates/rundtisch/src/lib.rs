pub mod app;
#[cfg(feature = "auth")]
pub mod auth;
pub mod email;
#[cfg(feature = "storage")]
pub mod storage;

pub use app::{AppState, SecretError};
#[cfg(feature = "sendmail")]
pub use email::SendmailEmailSender;
#[cfg(feature = "smtp")]
pub use email::SmtpEmailSender;
pub use email::{EmailSendError, EmailSender};
