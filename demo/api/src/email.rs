use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Instant;

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
        let started = Instant::now();
        eprintln!("sendmail: spawning");
        let mut child = Command::new("sendmail")
            .arg("-t")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            // Do not buffer diagnostics until the process exits: a stalled
            // process needs to remain observable in Wasmer's runtime logs.
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|err| {
                eprintln!(
                    "sendmail: spawn failed elapsed_ms={} error={err}",
                    started.elapsed().as_millis()
                );
                EmailSendError::new(format!("failed to spawn sendmail: {err}"))
            })?;
        eprintln!(
            "sendmail: spawned elapsed_ms={}",
            started.elapsed().as_millis()
        );

        let mut stdin = child.stdin.take().ok_or_else(|| {
            eprintln!(
                "sendmail: stdin unavailable elapsed_ms={}",
                started.elapsed().as_millis()
            );
            EmailSendError::new("sendmail stdin unavailable")
        })?;
        stdin.write_all(message.as_bytes()).map_err(|err| {
            eprintln!(
                "sendmail: stdin write failed elapsed_ms={} error={err}",
                started.elapsed().as_millis()
            );
            EmailSendError::new(format!("failed to write sendmail stdin: {err}"))
        })?;
        // sendmail reads until EOF. Close the pipe explicitly before waiting.
        drop(stdin);
        eprintln!(
            "sendmail: stdin closed elapsed_ms={}",
            started.elapsed().as_millis()
        );

        let status = child.wait().map_err(|err| {
            eprintln!(
                "sendmail: wait failed elapsed_ms={} error={err}",
                started.elapsed().as_millis()
            );
            EmailSendError::new(format!("failed to wait for sendmail: {err}"))
        })?;
        eprintln!(
            "sendmail: exited status={status} elapsed_ms={}",
            started.elapsed().as_millis()
        );
        if status.success() {
            Ok(())
        } else {
            Err(EmailSendError::new(format!(
                "sendmail exited with {status}"
            )))
        }
    }
}
