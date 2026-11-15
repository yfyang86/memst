//! The OpenAI-compatible client retries what is worth retrying.

use memst_core::llm::loader::{LlmLoader, LlmProviderConfig};
use memst_core::llm::{LlmRequest, Message};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

const OK_BODY: &str = r#"{"choices":[{"index":0,"message":{"role":"assistant","content":"fine"},"finish_reason":"stop"}]}"#;

/// Serve one scripted reply per connection, then stop. `None` closes the
/// connection without answering.
fn serve(script: Vec<Option<(u16, &'static str)>>) -> (String, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/v1", listener.local_addr().unwrap());
    let seen = Arc::new(AtomicUsize::new(0));
    let counter = seen.clone();
    std::thread::spawn(move || {
        for reply in script {
            let Ok((mut stream, _)) = listener.accept() else { return };
            counter.fetch_add(1, Ordering::SeqCst);
            let mut buffer = [0u8; 8192];
            let _ = stream.read(&mut buffer);
            if let Some((status, body)) = reply {
                let _ = write!(
                    stream,
                    "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
            }
            // Dropping the stream closes the connection.
        }
    });
    (url, seen)
}

async fn ask(url: &str) -> memst_core::error::Result<String> {
    let llm = LlmLoader::load(LlmProviderConfig::openai(url, "m"))?;
    let response = llm.chat(LlmRequest::new("m").with_message(Message::user("hi"))).await?;
    Ok(response.choices[0].message.as_ref().unwrap().content.clone())
}

#[tokio::test]
async fn dropped_connections_and_server_errors_are_retried() {
    let (url, seen) = serve(vec![None, Some((503, "busy")), Some((429, "slow down")), Some((200, OK_BODY))]);
    assert_eq!(ask(&url).await.unwrap(), "fine");
    assert_eq!(seen.load(Ordering::SeqCst), 4);
}

#[tokio::test]
async fn a_bad_request_fails_at_once() {
    let (url, seen) = serve(vec![Some((401, "bad key")), Some((200, OK_BODY))]);
    let error = ask(&url).await.unwrap_err().to_string();
    assert!(error.contains("401") && error.contains("bad key"), "{error}");
    assert_eq!(seen.load(Ordering::SeqCst), 1, "no retry for a client error");
}

#[tokio::test]
async fn it_gives_up_after_a_few_attempts() {
    let (url, seen) = serve(vec![Some((500, "down")); 10]);
    assert!(ask(&url).await.unwrap_err().to_string().contains("500"));
    assert_eq!(seen.load(Ordering::SeqCst), 4);
}

#[test]
fn provider_specific_fields_are_sent_at_the_top_level() {
    let request = LlmRequest::new("m")
        .with_message(Message::user("hi"))
        .with_extra("thinking", serde_json::json!({"type": "disabled"}));
    let sent = serde_json::to_value(&request).unwrap();
    assert_eq!(sent["thinking"]["type"], "disabled");
    assert_eq!(sent["model"], "m");
    assert!(serde_json::to_value(LlmRequest::new("m")).unwrap().get("extra").is_none());
}
