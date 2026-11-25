//! Loopback HTTP server integration tests.

use cadence_studio::server::{Config, StudioServer};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

struct Response {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Response {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
}

fn start() -> StudioServer {
    StudioServer::start(Config {
        port: 0,
        token: Some("integration-token-123456".to_string()),
        open_browser: false,
        quiet: true,
    })
    .unwrap()
}

fn request(
    server: &StudioServer,
    method: &str,
    path: &str,
    body: &str,
    cookie: Option<&str>,
) -> Response {
    let mut stream = TcpStream::connect(server.addr).unwrap();
    stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    let mut head = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: {}\r\nConnection: close\r\n",
        body.len()
    );
    if let Some(cookie) = cookie {
        head.push_str(&format!("Cookie: {cookie}\r\n"));
    }
    head.push_str("\r\n");
    stream.write_all(head.as_bytes()).unwrap();
    stream.write_all(body.as_bytes()).unwrap();
    let mut bytes = Vec::new();
    stream.read_to_end(&mut bytes).unwrap();
    let split = bytes
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .unwrap();
    let header_text = String::from_utf8_lossy(&bytes[..split]).into_owned();
    let mut lines = header_text.lines();
    let status_line = lines.next().unwrap();
    let status = status_line
        .split_whitespace()
        .nth(1)
        .unwrap()
        .parse()
        .unwrap();
    let headers = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(key, value)| (key.trim().to_string(), value.trim().to_string()))
        .collect();
    Response {
        status,
        headers,
        body: bytes[split + 4..].to_vec(),
    }
}

fn token(server: &StudioServer) -> String {
    format!("cadence_token={}", server.token)
}

#[test]
fn page_load_requires_token_and_sets_secure_cookie() {
    let server = start();
    let denied = request(&server, "GET", "/", "", None);
    assert_eq!(denied.status, 403);
    let allowed = request(&server, "GET", &format!("/?token={}", server.token), "", None);
    assert_eq!(allowed.status, 200);
    assert!(allowed.text().contains("Engraving Workbench"));
    assert!(allowed
        .header("set-cookie")
        .unwrap()
        .contains("HttpOnly"));
    assert!(allowed.header("content-security-policy").unwrap().contains("default-src 'self'"));
    assert_eq!(allowed.header("x-content-type-options"), Some("nosniff"));
    assert_eq!(allowed.header("referrer-policy"), Some("no-referrer"));
}

#[test]
fn api_auth_accepts_cookie_header_bearer_and_query() {
    let server = start();
    let cookie = token(&server);
    for (path, extra) in [
        ("/api/state", None),
        (
            "/api/state",
            Some(("X-Cadence-Token", server.token.as_str())),
        ),
        (
            "/api/state",
            Some(("Authorization", "Bearer integration-token-123456")),
        ),
    ] {
        let mut stream = TcpStream::connect(server.addr).unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let mut request_text = format!(
            "GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n"
        );
        match extra {
            Some((name, value)) => request_text.push_str(&format!("{name}: {value}\r\n")),
            None => request_text.push_str(&format!("Cookie: {cookie}\r\n")),
        }
        request_text.push_str("\r\n");
        stream.write_all(request_text.as_bytes()).unwrap();
        let mut bytes = Vec::new();
        stream.read_to_end(&mut bytes).unwrap();
        assert!(String::from_utf8_lossy(&bytes).starts_with("HTTP/1.1 200"));
    }
    let query = request(&server, "GET", &format!("/api/state?token={}", server.token), "", None);
    assert_eq!(query.status, 200);
    let missing = request(&server, "GET", "/api/state", "", None);
    assert_eq!(missing.status, 401);
    assert!(missing.text().contains("access token"));
}

#[test]
fn state_edit_undo_reset_and_select_flow() {
    let server = start();
    let cookie = token(&server);
    let state = request(&server, "GET", "/api/state", "", Some(&cookie));
    assert_eq!(state.status, 200);
    assert!(state.text().contains("\"version\":1"));
    let edit = request(
        &server,
        "POST",
        "/api/edit",
        r#"{"op":"set_title","title":"Server Test"}"#,
        Some(&cookie),
    );
    assert_eq!(edit.status, 200);
    assert!(edit.text().contains("\"version\":2"));
    assert!(edit.text().contains("Server Test"));
    let undo = request(&server, "POST", "/api/undo", "{}", Some(&cookie));
    assert_eq!(undo.status, 200);
    assert!(undo.text().contains("Cadence in G"));
    let select = request(
        &server,
        "POST",
        "/api/select",
        r#"{"part":1}"#,
        Some(&cookie),
    );
    assert_eq!(select.status, 200);
    assert!(select.text().contains("\"selectedPart\":1"));
    let invalid = request(
        &server,
        "POST",
        "/api/edit",
        r#"{"op":"add_note","part":9,"pitch":"C4"}"#,
        Some(&cookie),
    );
    assert_eq!(invalid.status, 400);
    let reset = request(&server, "POST", "/api/reset", "{}", Some(&cookie));
    assert_eq!(reset.status, 200);
    assert!(reset.text().contains("Cadence in G"));
}

#[test]
fn import_endpoint_accepts_raw_musicxml() {
    let server = start();
    let cookie = token(&server);
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/fixtures/fixture_000.musicxml");
    let xml = std::fs::read_to_string(path).unwrap();
    let response = request(&server, "POST", "/api/import", &xml, Some(&cookie));
    assert_eq!(response.status, 200);
    assert!(response.text().contains("Fixture 000"));
    let broken = request(&server, "POST", "/api/import", "<score-partwise>", Some(&cookie));
    assert_eq!(broken.status, 400);
}

#[test]
fn exports_and_layout_are_authenticated_binary_endpoints() {
    let server = start();
    let cookie = token(&server);
    let svg = request(&server, "GET", "/api/layout.svg?part=all&hits=1", "", Some(&cookie));
    assert_eq!(svg.status, 200);
    assert!(svg.text().contains("data-part="));
    let pdf = request(&server, "GET", "/api/export/pdf?part=0", "", Some(&cookie));
    assert_eq!(pdf.status, 200);
    assert!(pdf.body.starts_with(b"%PDF"));
    assert!(pdf.header("content-disposition").unwrap().contains(".pdf"));
    let midi = request(&server, "GET", "/api/export/midi", "", Some(&cookie));
    assert!(midi.body.starts_with(b"MThd"));
    let unauthorized = request(&server, "GET", "/api/export/pdf", "", None);
    assert_eq!(unauthorized.status, 401);
}

#[test]
fn health_and_assets_are_available_without_data_access() {
    let server = start();
    assert_eq!(request(&server, "GET", "/healthz", "", None).status, 200);
    let css = request(&server, "GET", "/assets/app.css", "", None);
    assert_eq!(css.status, 200);
    assert!(css.text().contains("--accent"));
    let js = request(&server, "GET", "/assets/app.js", "", None);
    assert_eq!(js.status, 200);
    assert!(js.text().contains("EventSource"));
    assert_eq!(request(&server, "GET", "/missing", "", None).status, 404);
}

#[test]
fn sse_receives_an_update_after_an_edit() {
    let server = start();
    let cookie = token(&server);
    let mut stream = TcpStream::connect(server.addr).unwrap();
    stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    let request_text = format!(
        "GET /api/events HTTP/1.1\r\nHost: 127.0.0.1\r\nCookie: {cookie}\r\n\r\n"
    );
    stream.write_all(request_text.as_bytes()).unwrap();
    let mut buffer = [0u8; 2048];
    let mut received = Vec::new();
    let initial_deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        if std::time::Instant::now() > initial_deadline {
            panic!("no initial SSE event");
        }
        let read = stream.read(&mut buffer).unwrap();
        if read == 0 {
            panic!("SSE closed before initial event");
        }
        received.extend_from_slice(&buffer[..read]);
        if String::from_utf8_lossy(&received).contains("event: update") {
            break;
        }
    }
    let edit = request(
        &server,
        "POST",
        "/api/edit",
        r#"{"op":"set_tempo","part":0,"measure":0,"bpm":128}"#,
        Some(&cookie),
    );
    assert_eq!(edit.status, 200);
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        if std::time::Instant::now() > deadline {
            panic!("no SSE update after edit");
        }
        let read = stream.read(&mut buffer).unwrap();
        if read == 0 {
            break;
        }
        received.extend_from_slice(&buffer[..read]);
        let text = String::from_utf8_lossy(&received);
        if text.matches("event: update").count() >= 2 {
            return;
        }
    }
    panic!("SSE stream closed before update");
}
