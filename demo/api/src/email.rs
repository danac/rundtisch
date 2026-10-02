use std::path::PathBuf;
use std::process::Stdio;
use std::time::{SystemTime, UNIX_EPOCH};

use rundtisch::{EmailSendError, EmailSender};
use tokio::fs::{self, File};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::time::{Duration, sleep};

/// Delivers mail by handing an RFC822 message to `sendmail -t`.
///
/// WASIX does not deliver EOF when a subprocess pipe is closed, and
/// `Child::wait` does not reliably wake. The message is written to a
/// temporary file and passed as stdin; exit is polled with `try_wait`.
///
/// On Wasmer Edge, enable outbound mail with `enable_email: true` in
/// `app.yaml` and depend on `sendmail/sendmail` in `wasmer.toml`.
#[derive(Debug, Clone, Default)]
pub struct SendmailEmailSender;

#[async_trait::async_trait]
impl EmailSender for SendmailEmailSender {
    async fn send(&self, to: &str, subject: &str, body: &str) -> Result<(), EmailSendError> {
        let message = format!("To: {to}\nSubject: {subject}\n\n{body}");
        deliver(message.as_bytes()).await
    }
}

async fn deliver(message: &[u8]) -> Result<(), EmailSendError> {
    let path = temporary_message_path();
    let path_disp = path.display().to_string();

    let write_result = async {
        let mut file = File::create(&path)
            .await
            .map_err(|err| EmailSendError::new(format!("failed to create {path_disp}: {err}")))?;
        file.write_all(message)
            .await
            .map_err(|err| EmailSendError::new(format!("failed to write {path_disp}: {err}")))?;
        file.flush()
            .await
            .map_err(|err| EmailSendError::new(format!("failed to flush {path_disp}: {err}")))?;
        Ok(())
    }
    .await;
    if let Err(err) = write_result {
        let _ = fs::remove_file(&path).await;
        return Err(err);
    }

    let input = match File::open(&path).await {
        Ok(file) => file.into_std().await,
        Err(err) => {
            let _ = fs::remove_file(&path).await;
            return Err(EmailSendError::new(format!(
                "failed to open {path_disp}: {err}"
            )));
        }
    };

    let mut child = match Command::new("sendmail")
        .arg("-t")
        .stdin(Stdio::from(input))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(err) => {
            let _ = fs::remove_file(&path).await;
            return Err(EmailSendError::new(format!(
                "failed to spawn sendmail: {err}"
            )));
        }
    };

    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => sleep(Duration::from_millis(50)).await,
            Err(err) => {
                let _ = fs::remove_file(&path).await;
                return Err(EmailSendError::new(format!(
                    "failed to check sendmail status: {err}"
                )));
            }
        }
    };

    if let Err(err) = fs::remove_file(&path).await {
        return Err(EmailSendError::new(format!(
            "sendmail exited with {status}, but failed to remove temporary file {path_disp}: {err}"
        )));
    }
    if status.success() {
        Ok(())
    } else {
        Err(EmailSendError::new(format!(
            "sendmail exited with {status}"
        )))
    }
}

fn temporary_message_path() -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::temp_dir().join(format!("sendmail-{}-{timestamp}.eml", std::process::id(),))
}
