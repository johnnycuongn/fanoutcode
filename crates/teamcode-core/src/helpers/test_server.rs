//! A one-shot local SSE server standing in for an OpenAI-compatible model (the harness livespec-core's own tests
//! use, which is private to that crate).

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

/// An SSE stream that answers `text` (in one chunk) and reports the given usage.
pub fn sse_body(text: &str, prompt_tokens: u64, completion_tokens: u64) -> String {
    let content = serde_json::json!({"choices": [{"index": 0, "delta": {"content": text}}]});
    let usage = serde_json::json!({"choices": [], "usage": {"prompt_tokens": prompt_tokens, "completion_tokens": completion_tokens}});
    format!("data: {content}\n\ndata: {usage}\n\ndata: [DONE]\n\n")
}

/// Serves `body` to one connection.
pub async fn sse_server(body: &str) -> (String, JoinHandle<()>) {
    sse_server_n(body, 1).await
}

/// Serves `body` to each of the next `n` connections (concurrently), then stops listening.
pub async fn sse_server_n(body: &str, n: usize) -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let body = body.to_owned();
    let server = tokio::spawn(async move {
        for _ in 0..n {
            let Ok((mut sock, _)) = listener.accept().await else {
                return;
            };
            let body = body.clone();
            tokio::spawn(async move {
                let mut buf = Vec::new();
                let mut chunk = [0u8; 4096];
                // read the headers and the Content-Length body before answering
                loop {
                    let Ok(k) = sock.read(&mut chunk).await else { return };
                    if k == 0 {
                        return;
                    }
                    buf.extend_from_slice(&chunk[..k]);
                    if let Some(end) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                        let head = String::from_utf8_lossy(&buf[..end]).to_lowercase();
                        let len = head
                            .lines()
                            .find_map(|l| l.strip_prefix("content-length:"))
                            .and_then(|v| v.trim().parse::<usize>().ok())
                            .unwrap_or(0);
                        if buf.len() >= end + 4 + len {
                            break;
                        }
                    }
                }
                let reply = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = sock.write_all(reply.as_bytes()).await;
                let _ = sock.shutdown().await;
            });
        }
    });
    (url, server)
}
