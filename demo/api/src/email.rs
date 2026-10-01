use std::io::Write;
use std::process::{Command, Stdio};

use rundtisch::{EmailSendError, EmailSender};

/// Delivers mail by piping an RFC822 message to the `sendmail -t` binary.
///
/// On Wasmer Edge, enable outbound mail with `enable_email: true` in
/// `app.yaml` and depend on `sendmail/sendmail` in `wasmer.toml`.
#[derive(Debug, Clone, Default)]
pub struct SendmailEmailSender;

impl EmailSender for SendmailEmailSender {
    fn send(&self, to: &str, subject: &str, body: &str) -> Result<(), EmailSendError> {
        let message = format!("To: {to}\nSubject: {subject}\n\n{body}");
        let mut child = Command::new("sendmail")
            .arg("-t")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|err| EmailSendError::new(format!("failed to spawn sendmail: {err}")))?;

        {
            let stdin = child
                .stdin
                .as_mut()
                .ok_or_else(|| EmailSendError::new("sendmail stdin unavailable"))?;
            stdin
                .write_all(message.as_bytes())
                .map_err(|err| EmailSendError::new(format!("failed to write sendmail stdin: {err}")))?;
        }

        let output = child
            .wait_with_output()
            .map_err(|err| EmailSendError::new(format!("failed to wait for sendmail: {err}")))?;
        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(EmailSendError::new(format!(
                "sendmail exited with {}: {}",
                output.status,
                stderr.trim()
            )))
        }
    }
}
