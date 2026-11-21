use memst_core::embed::{Embedder, HttpEmbedder};
use memst_core::llm::EmbeddingConfig;
use std::io::{Read, Write};
use std::net::TcpListener;

#[test]
fn http_embedding_works_inside_an_async_runtime() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/v1", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buffer = [0u8; 8192];
        let _ = stream.read(&mut buffer);
        let body = r#"{"data":[{"embedding":[0.1,0.2,0.3]}]}"#;
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .unwrap();
    });
    let embedder = HttpEmbedder::new(EmbeddingConfig {
        api_url: url,
        model: "m".to_string(),
        timeout: 5,
        expected_dimension: Some(3),
    })
    .unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let vectors = embedder.embed(&["hello".to_string()]).unwrap();
        assert_eq!(vectors, vec![vec![0.1, 0.2, 0.3]]);
    });
    drop(embedder);
    server.join().unwrap();
}
