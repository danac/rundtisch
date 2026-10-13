use std::time::Duration;

use bytes::Bytes;

/// Failure while downloading a remote object body (seed repair and catalog import).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchError {
    message: String,
}

impl FetchError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for FetchError {}

/// HTTP GET for seed imports. The user agent matches the previous Crockis seed
/// client so catalog hosts that reject unknown agents keep working.
#[derive(Clone, Debug)]
pub struct HttpUrlFetcher {
    client: reqwest::Client,
}

impl HttpUrlFetcher {
    pub fn new() -> Result<Self, FetchError> {
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (compatible; CrockisSeed/0.1)")
            .timeout(Duration::from_secs(60))
            .redirect(reqwest::redirect::Policy::limited(8))
            .build()
            .map_err(|err| FetchError::new(err.to_string()))?;
        Ok(Self { client })
    }

    pub fn from_env() -> Result<Self, FetchError> {
        Self::new()
    }

    pub async fn fetch(&self, url: &str, max_len: u64) -> Result<Bytes, FetchError> {
        let response = self
            .client
            .get(url)
            .header(
                reqwest::header::ACCEPT,
                "image/jpeg,image/png,image/webp,image/gif,*/*;q=0.8",
            )
            .send()
            .await
            .map_err(|err| FetchError::new(format!("{url}: {err}")))?;
        if let Some(len) = response.content_length() {
            if len > max_len {
                return Err(FetchError::new(format!(
                    "{url}: response is larger than {max_len} bytes"
                )));
            }
        }
        let status = response.status();
        if !status.is_success() {
            return Err(FetchError::new(format!("{url}: HTTP {status}")));
        }
        let bytes = response
            .bytes()
            .await
            .map_err(|err| FetchError::new(format!("{url}: {err}")))?;
        if bytes.is_empty() {
            return Err(FetchError::new(format!("{url}: empty response")));
        }
        if bytes.len() as u64 > max_len {
            return Err(FetchError::new(format!(
                "{url}: response is larger than {max_len} bytes"
            )));
        }
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn fetches_from_a_local_server() {
        let body = b"\x89PNG tiny";
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 2048];
            let _ = tokio::io::AsyncReadExt::read(&mut sock, &mut buf).await;
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            use tokio::io::AsyncWriteExt;
            sock.write_all(header.as_bytes()).await.unwrap();
            sock.write_all(body).await.unwrap();
        });
        let fetcher = HttpUrlFetcher::from_env().unwrap();
        let bytes = fetcher
            .fetch(&format!("http://127.0.0.1:{port}/tiny.png"), 1024)
            .await
            .unwrap();
        assert_eq!(&bytes[..], body);
    }
}
