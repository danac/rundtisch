// use std::process::{Command, Stdio};
//
// fn main() -> Result<(), String> {
//     let message = "To: dana.christen@gmail.com\n\
//                    Subject: Test Email 3\n\
//                    \n\
//                    Body of the email";
//
//     let status = Command::new("sh")
//         .arg("-c")
//         .arg("printf '%s' \"$1\" | sendmail -vvv -t")
//         .arg("--")
//         .arg(message)
//         .stdin(Stdio::inherit())
//         .stdout(Stdio::inherit())
//         .stderr(Stdio::inherit())
//         .status()
//         .map_err(|e| format!("failed to run sendmail: {e}"))?;
//
//     if status.success() {
//         Ok(())
//     } else {
//         Err(format!("sendmail exited with {status}"))
//     }
// }

// use std::fs::{self, File};
// use std::io::Write;
// use std::process::{Command, Stdio};
//
// fn sendmail(message: &[u8]) -> Result<(), String> {
//     let path = format!("/tmp/sendmail-{}.eml", std::process::id());
//
//     // Write the complete message and close the file before starting sendmail.
//     {
//         let mut file = File::create(&path)
//             .map_err(|e| format!("failed to create {path}: {e}"))?;
//
//         file.write_all(message)
//             .map_err(|e| format!("failed to write {path}: {e}"))?;
//     }
//
//     // Open the completed file as sendmail's stdin.
//     let input = File::open(&path)
//         .map_err(|e| format!("failed to open {path}: {e}"))?;
//
//     let result = Command::new("sendmail")
//         .arg("-vvv")
//         .arg("-t")
//         .stdin(Stdio::from(input))
//         .stdout(Stdio::inherit())
//         .stderr(Stdio::inherit())
//         .status()
//         .map_err(|e| format!("failed to spawn sendmail: {e}"));
//
//     // Remove the message regardless of whether sendmail succeeded.
//     let remove_result = fs::remove_file(&path);
//
//     // Preserve the sendmail error if there was one.
//     let status = result?;
//
//     remove_result
//         .map_err(|e| format!("sendmail exited with {status}, but failed to remove {path}: {e}"))?;
//
//     if !status.success() {
//         return Err(format!("sendmail exited with {status}"));
//     }
//
//     Ok(())
// }
//
// fn main() -> Result<(), String> {
//     let message = b"To: dana.christen@gmail.com\n\
//                     Subject: Test Email 4\n\
//                     \n\
//                     Body of the email";
//
//     sendmail(message)
// }

use std::path::PathBuf;
use std::process::Stdio;
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::fs::{self, File};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::time::{sleep, Duration};

/// Send an email using the WASIX sendmail implementation.
///
/// WASIX currently has two relevant subprocess issues:
///
/// 1. `Stdio::piped()` does not reliably deliver EOF when ChildStdin
///    is dropped.
/// 2. `Child::wait().await` does not reliably wake when a child exits.
///
/// Therefore this implementation:
///
/// - uses a regular file for stdin;
/// - uses `try_wait()` + async sleep to detect process exit.
pub async fn sendmail(message: &[u8]) -> Result<(), String> {
    let path = temporary_message_path();
    let path_disp = path.display();

    eprintln!("sendmail: creating {path_disp}");

    // ---------------------------------------------------------------
    // 1. Write the complete email to a temporary file.
    // ---------------------------------------------------------------
    {
        let mut file = File::create(&path)
            .await
            .map_err(|e| format!("failed to create {path_disp}: {e}"))?;

        file.write_all(message)
            .await
            .map_err(|e| format!("failed to write {path_disp}: {e}"))?;

        file.flush()
            .await
            .map_err(|e| format!("failed to flush {path_disp}: {e}"))?;
    }

    eprintln!("sendmail: message written");

    // ---------------------------------------------------------------
    // 2. Open the completed file for reading.
    //
    // Convert Tokio's File into std::fs::File because Stdio takes
    // ownership of the standard library file descriptor.
    // ---------------------------------------------------------------
    let input = File::open(&path)
        .await
        .map_err(|e| format!("failed to open {path_disp}: {e}"))?
        .into_std()
        .await;

    // ---------------------------------------------------------------
    // 3. Spawn sendmail.
    //
    // IMPORTANT: do not use Stdio::piped() here.
    // ---------------------------------------------------------------
    eprintln!("sendmail: spawning");

    let mut child = Command::new("sendmail")
        .arg("-vvv")
        .arg("-t")
        .stdin(Stdio::from(input))
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| format!("failed to spawn sendmail: {e}"))?;

    eprintln!("sendmail: spawned");

    // ---------------------------------------------------------------
    // 4. Wait for the process without using Child::wait().
    //
    // On WASIX, try_wait() works but wait().await does not reliably
    // receive the child-exit notification.
    // ---------------------------------------------------------------
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                eprintln!("sendmail: exited with {status}");
                break status;
            }

            Ok(None) => {
                // The child is still running.
                //
                // This is an async sleep, so the Tokio executor can
                // run other tasks while sendmail is doing its work.
                sleep(Duration::from_millis(50)).await;
                eprintln!("sleeping 50ms");
            }

            Err(error) => {
                // We can't reliably determine the child's state anymore.
                //
                // Still attempt to remove the temporary file before
                // returning the error.
                let _ = fs::remove_file(&path).await;

                return Err(format!(
                    "failed to check sendmail status: {error}"
                ));
            }
        }
    };

    // ---------------------------------------------------------------
    // 5. Remove the temporary message.
    // ---------------------------------------------------------------
    if let Err(error) = fs::remove_file(&path).await {
        return Err(format!(
            "sendmail exited with {status}, but failed to remove \
             temporary file {path_disp}: {error}"
        ));
    }

    eprintln!("sendmail: removed {path_disp}");

    // ---------------------------------------------------------------
    // 6. Check the exit status.
    // ---------------------------------------------------------------
    if !status.success() {
        return Err(format!("sendmail exited with {status}"));
    }

    eprintln!("sendmail: successfully submitted message");

    Ok(())
}

/// Generate a reasonably unique temporary filename.
///
/// If you can have many concurrent sends, including the timestamp and
/// process ID avoids collisions between normal invocations.
fn temporary_message_path() -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    std::env::temp_dir().join(format!(
        "sendmail-{}-{timestamp}.eml",
        std::process::id(),
    ))
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), String> {
    let message = b"\
To: dana.christen@gmail.com
Subject: Test Email final

Body of the email.
";

    sendmail(message).await?;

    eprintln!("main: finished");

    Ok(())
}

//let message = message.to_vec();
//
// tokio::spawn(async move {
//     if let Err(error) = sendmail(&message).await {
//         eprintln!("background sendmail failed: {error}");
//     }
// });