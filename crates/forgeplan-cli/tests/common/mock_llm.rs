//! Minimal mock OpenAI-compatible HTTP server for LLM-backed command tests.
//!
//! Serves one canned response per request on `POST /v1/chat/completions`
//! using `std::net::TcpListener`. Each test gets its own ephemeral port.
//!
//! Usage:
//!   let server = MockLlmServer::start(vec!["hello".into()]);
//!   configure_llm(&tmp, server.port(), "test-model", "test-key");
//!   // ... run forgeplan command ...
//!   assert_eq!(server.requests(), 1);
//!   server.stop();

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

/// A mock OpenAI-compatible chat completions server.
pub struct MockLlmServer {
    listener: TcpListener,
    port: u16,
    responses: Arc<Mutex<Vec<String>>>,
    request_count: Arc<AtomicU16>,
    shutdown: Arc<Mutex<bool>>,
    handle: Option<thread::JoinHandle<()>>,
}

impl MockLlmServer {
    /// Start a server that cycles through `responses` per request.
    /// If `responses` is empty, returns a fixed generic answer.
    pub fn start(responses: Vec<String>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let responses = Arc::new(Mutex::new(if responses.is_empty() {
            vec!["mock LLM response".into()]
        } else {
            responses
        }));
        let request_count = Arc::new(AtomicU16::new(0));
        let shutdown = Arc::new(Mutex::new(false));

        let resp_clone = Arc::clone(&responses);
        let count_clone = Arc::clone(&request_count);
        let count_for_loop = Arc::clone(&count_clone);
        let shutdown_clone = Arc::clone(&shutdown);
        let listener_clone = listener.try_clone().unwrap();

        let handle = thread::spawn(move || {
            for stream in listener_clone.incoming() {
                if *shutdown_clone.lock().unwrap() {
                    break;
                }
                let Ok(stream) = stream else { continue };
                Self::handle_connection(stream, &resp_clone, &count_for_loop);
            }
        });

        Self {
            listener,
            port,
            responses,
            request_count: count_clone,
            shutdown,
            handle: Some(handle),
        }
    }

    /// The port this server listens on.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// The base URL to put in `.forgeplan/config.yaml`.
    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}/v1", self.port)
    }

    /// How many HTTP requests have been served.
    pub fn requests(&self) -> u16 {
        self.request_count.load(Ordering::SeqCst)
    }

    /// Stop accepting connections.
    pub fn stop(mut self) {
        *self.shutdown.lock().unwrap() = true;
        // Unblock accept() by connecting once.
        let _ = TcpStream::connect(self.listener.local_addr().unwrap());
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }

    fn handle_connection(
        mut stream: TcpStream,
        responses: &Arc<Mutex<Vec<String>>>,
        count: &Arc<AtomicU16>,
    ) {
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        // Read headers until blank line.
        let mut content_length = 0usize;
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) | Err(_) => break,
                _ => {}
            }
            let line = line.trim();
            if line.is_empty() {
                break;
            }
            if let Some(cl) = line
                .to_lowercase()
                .strip_prefix("content-length:")
                .and_then(|v| v.trim().parse::<usize>().ok())
            {
                content_length = cl;
            }
        }
        // Read body.
        let mut body = vec![0u8; content_length];
        let _ = reader.read_exact(&mut body);

        count.fetch_add(1, Ordering::SeqCst);

        let idx = count.load(Ordering::SeqCst).saturating_sub(1) as usize;
        let resp_text = {
            let guard = responses.lock().unwrap();
            guard
                .get(idx % guard.len())
                .cloned()
                .unwrap_or_else(|| "mock LLM response".into())
        };

        let json = serde_json::json!({
            "id": "chatcmpl-mock",
            "object": "chat.completion",
            "created": 1700000000u64,
            "model": "mock-model",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": resp_text,
                },
                "finish_reason": "stop",
            }],
            "usage": { "prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15 },
        });
        let body_bytes = serde_json::to_vec(&json).unwrap();
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body_bytes.len()
        );
        let _ = stream.write_all(response.as_bytes());
        let _ = stream.write_all(&body_bytes);
        let _ = stream.flush();
    }
}

/// Write a `.forgeplan/config.yaml` pointing at the mock server.
///
/// Call after `init_workspace` so `.forgeplan/` exists.
pub fn configure_llm(tmp: &tempfile::TempDir, base_url: &str, model: &str, api_key: &str) {
    let config_path = tmp.path().join(".forgeplan").join("config.yaml");
    let existing = std::fs::read_to_string(&config_path).unwrap_or_default();
    // Replace or append the llm section. Simplest: strip any existing llm block
    // and append ours.
    let stripped = existing
        .lines()
        .skip_while(|l| !l.starts_with("llm:"))
        .skip_while(|l| l.starts_with("  ") || l.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    let header = existing
        .lines()
        .take_while(|l| !l.starts_with("llm:"))
        .collect::<Vec<_>>()
        .join("\n");
    let llm_block = format!(
        "llm:\n  provider: openai\n  model: {model}\n  api_key_env: MOCK_LLM_KEY\n  base_url: {base_url}\n  max_tokens: 4096\n  temperature: 0.7\n  reason_temperature: 0.3\n"
    );
    let written = format!("{header}\n{llm_block}{stripped}");
    std::fs::write(&config_path, &written).unwrap();
    // "LLM not configured" downstream and is hard to diagnose.
    let check = std::fs::read_to_string(&config_path).unwrap_or_default();
    assert!(
        check.contains("provider: openai"),
        "configure_llm must write the llm block; got:\n{check}"
    );
    // SAFETY: called once per test before spawning the forgeplan subprocess;
    // the subprocess reads it at process start.
    unsafe { std::env::set_var("MOCK_LLM_KEY", api_key) };
}
