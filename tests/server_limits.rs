//! Server limit and malformed-request tests.

use cadence_studio::server::{Config, StudioServer};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

fn start() -> StudioServer {
    StudioServer::start(Config {
        port: 0,
        token: Some("limits-token-123456".to_string()),
        open_browser: false,
        quiet: true,
    })
    .unwrap()
}

fn raw(server: &StudioServer, request: &[u8]) -> String {
    let mut stream = TcpStream::connect(server.addr).unwrap();
    stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    let _ = stream.write_all(request);
    let mut bytes = Vec::new();
    let _ = stream.read_to_end(&mut bytes);
    String::from_utf8_lossy(&bytes).into_owned()
}

#[test]
fn malformed_request_line_is_rejected() {
    let server = start();
    let response = raw(&server, b"nonsense\r\n\r\n");
    assert!(response.starts_with("HTTP/1.1 400"), "{response}");
    assert!(response.contains("X-Content-Type-Options: nosniff"));
}

#[test]
fn chunked_and_oversized_requests_are_rejected() {
    let server = start();
    let chunked = raw(
        &server,
        b"POST /api/import HTTP/1.1\r\nHost: x\r\nCookie: cadence_token=limits-token-123456\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n",
    );
    assert!(chunked.starts_with("HTTP/1.1 400"), "{chunked}");
    let oversized = raw(
        &server,
        b"POST /api/import HTTP/1.1\r\nHost: x\r\nCookie: cadence_token=limits-token-123456\r\nContent-Length: 999999999\r\n\r\n",
    );
    assert!(oversized.starts_with("HTTP/1.1 400"), "{oversized}");
}

#[test]
fn too_many_headers_is_rejected() {
    let server = start();
    let mut request = String::from("GET /api/state HTTP/1.1\r\nHost: x\r\n");
    for index in 0..140 {
        request.push_str(&format!("X-Test-{index}: value\r\n"));
    }
    request.push_str("\r\n");
    let response = raw(&server, request.as_bytes());
    assert!(response.starts_with("HTTP/1.1 400"), "{response}");
}

#[test]
fn invalid_utf8_import_body_is_rejected() {
    let server = start();
    let body = [0xFFu8, 0xFE, 0xFD];
    let mut request = Vec::new();
    request.extend_from_slice(b"POST /api/import HTTP/1.1\r\nHost: x\r\nCookie: cadence_token=limits-token-123456\r\nContent-Length: 3\r\n\r\n");
    request.extend_from_slice(&body);
    let response = raw(&server, &request);
    assert!(response.starts_with("HTTP/1.1 400"), "{response}");
}

#[test]
fn short_bodies_are_rejected_cleanly() {
    let server = start();
    let response = raw(
        &server,
        b"POST /api/edit HTTP/1.1\r\nHost: x\r\nCookie: cadence_token=limits-token-123456\r\nContent-Length: 100\r\n\r\n{}",
    );
    assert!(response.is_empty() || response.starts_with("HTTP/1.1 400"));
}

#[test]
fn empty_and_whitespace_bodies_are_bad_requests() {
    let server = start();
    for body in ["", "   ", "{"] {
        let request = format!(
            "POST /api/edit HTTP/1.1\r\nHost: x\r\nCookie: cadence_token=limits-token-123456\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        let response = raw(&server, request.as_bytes());
        assert!(response.starts_with("HTTP/1.1 400"), "{body:?}: {response}");
    }
}

#[test]
fn invalid_tokens_are_not_initialized() {
    match StudioServer::start(Config {
        port: 0,
        token: Some("short".to_string()),
        open_browser: false,
        quiet: true,
    }) {
        Ok(server) => {
            drop(server);
            panic!("short token was accepted");
        }
        Err(error) => assert!(error.to_string().contains("token")),
    }
}

#[test]
fn invalid_query_values_return_json_bad_requests() {
    let server = start();
    for path in [
        "/api/event?part=x&measure=0&voice=1&event=0",
        "/api/event?part=0&measure=-1&voice=1&event=0",
        "/api/event?part=0&measure=0&voice=999&event=0",
        "/api/layout.svg?part=not-a-part",
    ] {
        let request = format!(
            "GET {path} HTTP/1.1\r\nHost: x\r\nCookie: cadence_token=limits-token-123456\r\n\r\n"
        );
        let response = raw(&server, request.as_bytes());
        assert!(response.starts_with("HTTP/1.1 400"), "{path}: {response}");
        assert!(response.contains("application/json"));
    }
}

#[test]
fn unsupported_methods_return_method_not_allowed() {
    let server = start();
    let request = b"DELETE /api/state HTTP/1.1\r\nHost: x\r\nCookie: cadence_token=limits-token-123456\r\nContent-Length: 0\r\n\r\n";
    let response = raw(&server, request);
    assert!(response.starts_with("HTTP/1.1 405"), "{response}");
    assert!(response.contains("method not allowed"));
}

#[test]
fn server_survives_malformed_requests_and_still_serves_health() {
    let server = start();
    let _ = raw(&server, b"garbage\r\n\r\n");
    let _ = raw(&server, b"GET / HTTP/1.1\r\n\r\n");
    let response = raw(&server, b"GET /healthz HTTP/1.1\r\nHost: x\r\n\r\n");
    assert!(response.starts_with("HTTP/1.1 200"));
    assert!(response.contains("ok"));
}
