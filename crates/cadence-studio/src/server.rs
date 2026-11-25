//! Loopback HTTP/SSE server for the embedded Studio.

use crate::assets;
use crate::document::{Document, EditCommand};
use crate::json::{self, Value};
use cadence_core::{Error, Result};
use std::fs::File;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const MAX_HEADER_BYTES: usize = 32 * 1024;
const MAX_BODY_BYTES: usize = 8 * 1024 * 1024;
const MAX_CONNECTIONS: usize = 64;
const MAX_SUBSCRIBERS: usize = 32;

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub token: Option<String>,
    pub open_browser: bool,
    pub quiet: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            port: 0,
            token: None,
            open_browser: true,
            quiet: false,
        }
    }
}

struct Shared {
    document: Mutex<Document>,
    subscribers: Mutex<Vec<mpsc::Sender<u64>>>,
    active: AtomicUsize,
    stop: Arc<AtomicBool>,
    token: String,
}

pub struct StudioServer {
    pub addr: SocketAddr,
    pub token: String,
    pub url: String,
    stop: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
}

impl StudioServer {
    pub fn start(config: Config) -> Result<StudioServer> {
        let listener = TcpListener::bind(("127.0.0.1", config.port))
            .map_err(|error| Error::msg(format!("cannot bind loopback port: {error}")))?;
        let addr = listener
            .local_addr()
            .map_err(|error| Error::msg(format!("cannot read local address: {error}")))?;
        let token = config.token.unwrap_or_else(random_token);
        if token.len() < 8 || token.len() > 256 {
            return Err(Error::msg("auth token must be 8..=256 characters"));
        }
        let url = format!("http://127.0.0.1:{}/?token={}", addr.port(), token);
        let stop = Arc::new(AtomicBool::new(false));
        let shared = Arc::new(Shared {
            document: Mutex::new(Document::demo()),
            subscribers: Mutex::new(Vec::new()),
            active: AtomicUsize::new(0),
            stop: stop.clone(),
            token: token.clone(),
        });
        let thread_shared = shared.clone();
        let quiet = config.quiet;
        let join = thread::spawn(move || {
            for connection in listener.incoming() {
                if thread_shared.stop.load(Ordering::SeqCst) {
                    break;
                }
                let Ok(stream) = connection else {
                    continue;
                };
                let count = thread_shared.active.fetch_add(1, Ordering::SeqCst);
                if count >= MAX_CONNECTIONS {
                    thread_shared.active.fetch_sub(1, Ordering::SeqCst);
                    let mut stream = stream;
                    let _ = write_response(
                        &mut stream,
                        503,
                        "Service Unavailable",
                        &[("Content-Type", "text/plain; charset=utf-8")],
                        b"too many local connections",
                    );
                    continue;
                }
                let client_shared = thread_shared.clone();
                thread::spawn(move || {
                    let _ = catch_unwind(AssertUnwindSafe(|| {
                        let mut stream = stream;
                        if let Err(error) = handle_connection(&mut stream, &client_shared) {
                            if !quiet {
                                eprintln!("cadence: request error: {error}");
                            }
                        }
                    }));
                    client_shared.active.fetch_sub(1, Ordering::SeqCst);
                });
            }
        });
        Ok(StudioServer {
            addr,
            token,
            url,
            stop,
            join: Some(join),
        })
    }

    pub fn shutdown(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.addr);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

impl Drop for StudioServer {
    fn drop(&mut self) {
        self.shutdown();
    }
}

pub fn run() -> Result<()> {
    let port = std::env::var("CADENCE_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(0);
    let token = std::env::var("CADENCE_TOKEN").ok();
    let open_browser = std::env::var("CADENCE_NO_OPEN").ok().as_deref() != Some("1");
    let mut server = StudioServer::start(Config {
        port,
        token,
        open_browser: false,
        quiet: false,
    })?;
    println!("Cadence Studio: {}", server.url);
    if open_browser {
        open_browser_url(&server.url);
    }
    loop {
        thread::sleep(Duration::from_secs(3600));
        if server.stop.load(Ordering::SeqCst) {
            break;
        }
    }
    server.shutdown();
    Ok(())
}

fn open_browser_url(url: &str) {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(url).spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn();
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        let _ = url;
    }
}

fn random_token() -> String {
    let mut bytes = [0u8; 32];
    let from_urandom = File::open("/dev/urandom")
        .and_then(|mut file| file.read_exact(&mut bytes))
        .is_ok();
    if !from_urandom {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos() as u64)
            .unwrap_or(0x9e37_79b9);
        let mut state = nanos ^ (std::process::id() as u64) << 17;
        for byte in &mut bytes {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            *byte = state as u8;
        }
    }
    let mut token = String::with_capacity(64);
    for byte in bytes {
        token.push_str(&format!("{byte:02x}"));
    }
    token
}

struct Request {
    method: String,
    path: String,
    query: Vec<(String, String)>,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Request {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }

    fn query(&self, name: &str) -> Option<&str> {
        self.query
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }

    fn authorized(&self, token: &str) -> bool {
        if self.query("token") == Some(token) {
            return true;
        }
        if self
            .header("x-cadence-token")
            .map(|value| value == token)
            .unwrap_or(false)
        {
            return true;
        }
        if let Some(value) = self.header("authorization") {
            if let Some(rest) = value.strip_prefix("Bearer ") {
                if rest == token {
                    return true;
                }
            }
        }
        if let Some(cookie) = self.header("cookie") {
            for part in cookie.split(';') {
                let mut pair = part.trim().splitn(2, '=');
                if pair.next() == Some("cadence_token") {
                    if pair.next() == Some(token) {
                        return true;
                    }
                }
            }
        }
        false
    }
}

fn handle_connection(stream: &mut TcpStream, shared: &Shared) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(20)))?;
    stream.set_write_timeout(Some(Duration::from_secs(20)))?;
    let request = match read_request(stream) {
        Ok(Some(request)) => request,
        Ok(None) => return Ok(()),
        Err(error) => {
            let _ = write_error(stream, 400, &error.to_string(), false);
            return Ok(());
        }
    };

    if request.path == "/healthz" {
        return write_response(
            stream,
            200,
            "OK",
            &[("Content-Type", "text/plain; charset=utf-8")],
            b"ok\n",
        );
    }

    if request.method == "GET" && request.path == "/" {
        if !request.authorized(&shared.token) {
            return write_error(stream, 403, "invalid or missing access token", false);
        }
        let Some(asset) = assets::find("/") else {
            return write_error(stream, 404, "not found", false);
        };
        let cookie = format!(
            "cadence_token={}; Path=/; HttpOnly; SameSite=Strict",
            shared.token
        );
        let csp = "default-src 'self'; img-src 'self' data:; style-src 'self'; script-src 'self'; connect-src 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'";
        return write_response(
            stream,
            200,
            "OK",
            &[
                ("Content-Type", asset.content_type),
                ("Set-Cookie", &cookie),
                ("Content-Security-Policy", csp),
                ("Referrer-Policy", "no-referrer"),
                ("X-Content-Type-Options", "nosniff"),
                ("Cache-Control", "no-store"),
            ],
            asset.body.as_bytes(),
        );
    }

    if request.method == "GET" {
        if let Some(asset) = assets::find(&request.path) {
            return write_response(
                stream,
                200,
                "OK",
                &[
                    ("Content-Type", asset.content_type),
                    ("X-Content-Type-Options", "nosniff"),
                    ("Cache-Control", "no-cache"),
                ],
                asset.body.as_bytes(),
            );
        }
    }

    if !request.path.starts_with("/api/") {
        return write_error(stream, 404, "not found", false);
    }
    if !request.authorized(&shared.token) {
        return write_error(stream, 401, "missing or invalid access token", true);
    }

    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/api/state") => {
            let document = shared.document.lock().map_err(lock_error)?;
            write_json(stream, 200, &document.summary())
        }
        ("GET", "/api/event") => {
            let part = parse_index(&request, "part", 0)?;
            let measure = parse_index(&request, "measure", 0)?;
            let voice = parse_u8(&request, "voice", 1)?;
            let event = parse_index(&request, "event", 0)?;
            let document = shared.document.lock().map_err(lock_error)?;
            match document.event_json(part, measure, voice, event) {
                Ok(value) => write_json(stream, 200, &value),
                Err(error) => write_error(stream, 400, &error.to_string(), true),
            }
        }
        ("GET", "/api/layout.svg") => {
            let part = optional_part(&request)?;
            let hits = request
                .query("hits")
                .map(|value| value == "1" || value == "true")
                .unwrap_or(false);
            let document = shared.document.lock().map_err(lock_error)?;
            match document.svg(part, hits) {
                Ok(svg) => write_response(
                    stream,
                    200,
                    "OK",
                    &[("Content-Type", "image/svg+xml; charset=utf-8"), ("Cache-Control", "no-store")],
                    svg.as_bytes(),
                ),
                Err(error) => write_error(stream, 400, &error.to_string(), true),
            }
        }
        ("GET", "/api/export/svg") => {
            let part = optional_part(&request)?;
            let document = shared.document.lock().map_err(lock_error)?;
            match document.svg(part, false) {
                Ok(svg) => attachment(
                    stream,
                    "image/svg+xml; charset=utf-8",
                    "svg",
                    &document.score.metadata.title,
                    svg.as_bytes(),
                ),
                Err(error) => write_error(stream, 400, &error.to_string(), true),
            }
        }
        ("GET", "/api/export/pdf") => {
            let part = optional_part(&request)?;
            let document = shared.document.lock().map_err(lock_error)?;
            match document.pdf(part) {
                Ok(pdf) => attachment(
                    stream,
                    "application/pdf",
                    "pdf",
                    &document.score.metadata.title,
                    &pdf,
                ),
                Err(error) => write_error(stream, 400, &error.to_string(), true),
            }
        }
        ("GET", "/api/export/midi") => {
            let part = optional_part(&request)?;
            let document = shared.document.lock().map_err(lock_error)?;
            match document.midi(part) {
                Ok(midi) => attachment(
                    stream,
                    "audio/midi",
                    "mid",
                    &document.score.metadata.title,
                    &midi,
                ),
                Err(error) => write_error(stream, 400, &error.to_string(), true),
            }
        }
        ("GET", "/api/export/musicxml") => {
            let part = optional_part(&request)?;
            let document = shared.document.lock().map_err(lock_error)?;
            match document.musicxml(part) {
                Ok(xml) => attachment(
                    stream,
                    "application/vnd.recordare.musicxml+xml; charset=utf-8",
                    "musicxml",
                    &document.score.metadata.title,
                    xml.as_bytes(),
                ),
                Err(error) => write_error(stream, 400, &error.to_string(), true),
            }
        }
        ("GET", "/api/events") => handle_events(stream, shared),
        ("POST", "/api/import") => {
            let text = match std::str::from_utf8(&request.body) {
                Ok(text) => text,
                Err(_) => return write_error(stream, 400, "MusicXML body must be UTF-8", true),
            };
            let mut document = shared.document.lock().map_err(lock_error)?;
            match document.import_xml(text) {
                Ok(()) => {
                    let summary = document.summary();
                    let version = document.version;
                    drop(document);
                    notify(shared, version);
                    write_json(stream, 200, &summary)
                }
                Err(error) => write_error(stream, 400, &error.to_string(), true),
            }
        }
        ("POST", "/api/edit") => {
            let value = match json::parse(
                std::str::from_utf8(&request.body).unwrap_or(""),
            ) {
                Ok(value) => value,
                Err(error) => return write_error(stream, 400, &error.to_string(), true),
            };
            let command = match EditCommand::from_json(&value) {
                Ok(command) => command,
                Err(error) => return write_error(stream, 400, &error.to_string(), true),
            };
            let mut document = shared.document.lock().map_err(lock_error)?;
            match document.apply(command) {
                Ok(()) => {
                    let summary = document.summary();
                    let version = document.version;
                    drop(document);
                    notify(shared, version);
                    write_json(stream, 200, &summary)
                }
                Err(error) => write_error(stream, 400, &error.to_string(), true),
            }
        }
        ("POST", "/api/undo") => {
            let mut document = shared.document.lock().map_err(lock_error)?;
            match document.undo() {
                Ok(()) => {
                    let summary = document.summary();
                    let version = document.version;
                    drop(document);
                    notify(shared, version);
                    write_json(stream, 200, &summary)
                }
                Err(error) => write_error(stream, 400, &error.to_string(), true),
            }
        }
        ("POST", "/api/redo") => {
            let mut document = shared.document.lock().map_err(lock_error)?;
            match document.redo() {
                Ok(()) => {
                    let summary = document.summary();
                    let version = document.version;
                    drop(document);
                    notify(shared, version);
                    write_json(stream, 200, &summary)
                }
                Err(error) => write_error(stream, 400, &error.to_string(), true),
            }
        }
        ("POST", "/api/reset") => {
            let mut document = shared.document.lock().map_err(lock_error)?;
            *document = Document::demo();
            let summary = document.summary();
            let version = document.version;
            drop(document);
            notify(shared, version);
            write_json(stream, 200, &summary)
        }
        ("POST", "/api/select") => {
            let value = match json::parse(std::str::from_utf8(&request.body).unwrap_or("")) {
                Ok(value) => value,
                Err(error) => return write_error(stream, 400, &error.to_string(), true),
            };
            let part = value.get("part").and_then(Value::as_u64).unwrap_or(0) as usize;
            let mut document = shared.document.lock().map_err(lock_error)?;
            match document.select_part(part) {
                Ok(()) => {
                    let summary = document.summary();
                    write_json(stream, 200, &summary)
                }
                Err(error) => write_error(stream, 400, &error.to_string(), true),
            }
        }
        _ => write_error(stream, 404, "unknown endpoint", true),
    }
}

fn handle_events(stream: &mut TcpStream, shared: &Shared) -> std::io::Result<()> {
    {
        let mut subscribers = shared.subscribers.lock().map_err(lock_error)?;
        if subscribers.len() >= MAX_SUBSCRIBERS {
            return write_error(stream, 503, "too many event subscribers", true);
        }
        let (sender, receiver) = mpsc::channel();
        subscribers.push(sender);
        drop(subscribers);
        write_response_headers(
            stream,
            200,
            "OK",
            &[
                ("Content-Type", "text/event-stream; charset=utf-8"),
                ("Cache-Control", "no-cache"),
                ("X-Accel-Buffering", "no"),
            ],
        )?;
        let version = shared
            .document
            .lock()
            .map(|document| document.version)
            .unwrap_or(0);
        write!(stream, "event: update\ndata: {{\"version\":{version}}}\n\n")?;
        stream.flush()?;
        loop {
            if shared.stop.load(Ordering::SeqCst) {
                break;
            }
            match receiver.recv_timeout(Duration::from_secs(15)) {
                Ok(version) => {
                    if write!(stream, "event: update\ndata: {{\"version\":{version}}}\n\n").is_err() {
                        break;
                    }
                    if stream.flush().is_err() {
                        break;
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if write!(stream, ": keepalive\n\n").is_err() || stream.flush().is_err() {
                        break;
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    }
    Ok(())
}

fn notify(shared: &Shared, version: u64) {
    if let Ok(mut subscribers) = shared.subscribers.lock() {
        subscribers.retain(|sender| sender.send(version).is_ok());
    }
}

fn read_request(stream: &mut TcpStream) -> std::io::Result<Option<Request>> {
    let mut buffer = Vec::with_capacity(4096);
    let mut chunk = [0u8; 4096];
    let header_end;
    loop {
        let read = stream.read(&mut chunk)?;
        if read == 0 {
            if buffer.is_empty() {
                return Ok(None);
            }
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "connection closed before headers",
            ));
        }
        buffer.extend_from_slice(&chunk[..read]);
        if buffer.len() > MAX_HEADER_BYTES {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "request headers too large",
            ));
        }
        if let Some(position) = find_header_end(&buffer) {
            header_end = position;
            break;
        }
    }
    let header_text = std::str::from_utf8(&buffer[..header_end])
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "headers are not UTF-8"))?;
    let mut lines = header_text.split("\r\n");
    let request_line = lines
        .next()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "missing request line"))?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let target = parts.next().unwrap_or("").to_string();
    let version = parts.next().unwrap_or("");
    if method.is_empty() || target.is_empty() || !version.starts_with("HTTP/") {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "malformed request line",
        ));
    }
    let mut headers = Vec::new();
    for line in lines {
        if line.is_empty() {
            continue;
        }
        let (name, value) = line.split_once(':').ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "malformed header")
        })?;
        headers.push((name.trim().to_ascii_lowercase(), value.trim().to_string()));
        if headers.len() > 128 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "too many headers",
            ));
        }
    }
    if headers
        .iter()
        .any(|(name, _)| name == "transfer-encoding")
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "chunked requests are not supported",
        ));
    }
    let content_length = headers
        .iter()
        .find(|(name, _)| name == "content-length")
        .map(|(_, value)| value.parse::<usize>())
        .transpose()
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid content-length"))?
        .unwrap_or(0);
    if content_length > MAX_BODY_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "request body too large",
        ));
    }
    let body_start = header_end + 4;
    let mut body = buffer[body_start..].to_vec();
    while body.len() < content_length {
        let read = stream.read(&mut chunk)?;
        if read == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "body shorter than content-length",
            ));
        }
        body.extend_from_slice(&chunk[..read]);
        if body.len() > MAX_BODY_BYTES {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "request body too large",
            ));
        }
    }
    body.truncate(content_length);
    let (path, query_text) = match target.split_once('?') {
        Some((path, query)) => (path.to_string(), query),
        None => (target, ""),
    };
    let mut query = Vec::new();
    if !query_text.is_empty() {
        for pair in query_text.split('&') {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            query.push((percent_decode(key), percent_decode(value)));
        }
    }
    Ok(Some(Request {
        method,
        path,
        query,
        headers,
        body,
    }))
}

fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|window| window == b"\r\n\r\n")
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'%' if index + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[index + 1..index + 3]).unwrap_or("");
                if let Ok(byte) = u8::from_str_radix(hex, 16) {
                    out.push(byte);
                    index += 3;
                    continue;
                }
                out.push(bytes[index]);
                index += 1;
            }
            b'+' => {
                out.push(b' ');
                index += 1;
            }
            byte => {
                out.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn parse_index(request: &Request, name: &str, default: usize) -> std::io::Result<usize> {
    match request.query(name) {
        None => Ok(default),
        Some(value) => value
            .parse::<usize>()
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("invalid {name}"))),
    }
}

fn parse_u8(request: &Request, name: &str, default: u8) -> std::io::Result<u8> {
    match request.query(name) {
        None => Ok(default),
        Some(value) => value
            .parse::<u8>()
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("invalid {name}"))),
    }
}

fn optional_part(request: &Request) -> std::io::Result<Option<usize>> {
    match request.query("part") {
        None | Some("all") => Ok(None),
        Some(value) => value
            .parse::<usize>()
            .map(Some)
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid part")),
    }
}

fn lock_error<T>(_: T) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, "document lock poisoned")
}

fn write_json(stream: &mut TcpStream, status: u16, value: &Value) -> std::io::Result<()> {
    let body = value.stringify();
    write_response(
        stream,
        status,
        status_text(status),
        &[
            ("Content-Type", "application/json; charset=utf-8"),
            ("Cache-Control", "no-store"),
        ],
        body.as_bytes(),
    )
}

fn write_error(
    stream: &mut TcpStream,
    status: u16,
    message: &str,
    json_response: bool,
) -> std::io::Result<()> {
    if json_response {
        let value = json::object(vec![
            ("ok", json::bool(false)),
            ("error", json::string(message.to_string())),
        ]);
        write_json(stream, status, &value)
    } else {
        write_response(
            stream,
            status,
            status_text(status),
            &[("Content-Type", "text/plain; charset=utf-8")],
            message.as_bytes(),
        )
    }
}

fn attachment(
    stream: &mut TcpStream,
    content_type: &str,
    extension: &str,
    title: &str,
    body: &[u8],
) -> std::io::Result<()> {
    let filename = format!("{}.{}", safe_filename(title), extension);
    let disposition = format!("attachment; filename=\"{filename}\"");
    write_response(
        stream,
        200,
        "OK",
        &[
            ("Content-Type", content_type),
            ("Content-Disposition", &disposition),
            ("Cache-Control", "no-store"),
        ],
        body,
    )
}

fn safe_filename(title: &str) -> String {
    let mut out = String::new();
    for c in title.chars().take(64) {
        if c.is_ascii_alphanumeric() || matches!(c, '-' | '_') {
            out.push(c);
        } else if c.is_whitespace() {
            out.push('_');
        }
    }
    let out = out.trim_matches('_').to_string();
    if out.is_empty() {
        "cadence".to_string()
    } else {
        out
    }
}

fn write_response_headers(
    stream: &mut TcpStream,
    status: u16,
    reason: &str,
    headers: &[(&str, &str)],
) -> std::io::Result<()> {
    let mut head = format!("HTTP/1.1 {status} {reason}\r\n");
    for (name, value) in headers {
        head.push_str(name);
        head.push_str(": ");
        head.push_str(value);
        head.push_str("\r\n");
    }
    head.push_str("X-Content-Type-Options: nosniff\r\n");
    head.push_str("Referrer-Policy: no-referrer\r\n");
    head.push_str("Connection: close\r\n\r\n");
    stream.write_all(head.as_bytes())
}

fn write_response(
    stream: &mut TcpStream,
    status: u16,
    reason: &str,
    headers: &[(&str, &str)],
    body: &[u8],
) -> std::io::Result<()> {
    let mut all_headers: Vec<(&str, String)> = headers
        .iter()
        .map(|(name, value)| (*name, (*value).to_string()))
        .collect();
    all_headers.push(("Content-Length", body.len().to_string()));
    let mut head = format!("HTTP/1.1 {status} {reason}\r\n");
    for (name, value) in &all_headers {
        head.push_str(name);
        head.push_str(": ");
        head.push_str(value);
        head.push_str("\r\n");
    }
    head.push_str("X-Content-Type-Options: nosniff\r\n");
    head.push_str("Referrer-Policy: no-referrer\r\n");
    head.push_str("Connection: close\r\n\r\n");
    stream.write_all(head.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()
}

fn status_text(status: u16) -> &'static str {
    match status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        413 => "Payload Too Large",
        500 => "Internal Server Error",
        503 => "Service Unavailable",
        _ => "Error",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::BufRead;
    use std::io::BufReader;

    fn request_bytes(
        addr: SocketAddr,
        token: &str,
        method: &str,
        path: &str,
        body: &str,
    ) -> Vec<u8> {
        let mut stream = TcpStream::connect(addr).unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let request = format!(
            "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nCookie: cadence_token={token}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(request.as_bytes()).unwrap();
        let mut response = Vec::new();
        stream.read_to_end(&mut response).unwrap();
        response
    }

    fn request(addr: SocketAddr, token: &str, method: &str, path: &str, body: &str) -> String {
        String::from_utf8_lossy(&request_bytes(addr, token, method, path, body)).into_owned()
    }

    #[test]
    fn server_serves_state_and_rejects_missing_token() {
        let server = StudioServer::start(Config {
            port: 0,
            token: Some("test-token-123456".to_string()),
            open_browser: false,
            quiet: true,
        })
        .unwrap();
        let authorized = request(server.addr, &server.token, "GET", "/api/state", "");
        assert!(authorized.starts_with("HTTP/1.1 200"));
        assert!(authorized.contains("\"parts\""));
        let unauthorized = request(server.addr, "wrong-token", "GET", "/api/state", "");
        assert!(unauthorized.starts_with("HTTP/1.1 401"));
        let health = request(server.addr, "", "GET", "/healthz", "");
        assert!(health.starts_with("HTTP/1.1 200"));
    }

    #[test]
    fn edit_endpoint_changes_the_document() {
        let server = StudioServer::start(Config {
            port: 0,
            token: Some("edit-token-123456".to_string()),
            open_browser: false,
            quiet: true,
        })
        .unwrap();
        let body = r#"{"op":"set_tempo","part":0,"measure":0,"bpm":132}"#;
        let response = request(server.addr, &server.token, "POST", "/api/edit", body);
        assert!(response.starts_with("HTTP/1.1 200"), "{response}");
        assert!(response.contains("\"version\":2"));
        let bad = request(
            server.addr,
            &server.token,
            "POST",
            "/api/edit",
            r#"{"op":"bogus"}"#,
        );
        assert!(bad.starts_with("HTTP/1.1 400"));
    }

    #[test]
    fn export_and_layout_endpoints_return_documents() {
        let server = StudioServer::start(Config {
            port: 0,
            token: Some("export-token-123456".to_string()),
            open_browser: false,
            quiet: true,
        })
        .unwrap();
        let svg = request(server.addr, &server.token, "GET", "/api/layout.svg?part=0&hits=1", "");
        assert!(svg.contains("<svg"));
        assert!(svg.contains("data-part="));
        let pdf = request_bytes(server.addr, &server.token, "GET", "/api/export/pdf?part=0", "");
        assert!(String::from_utf8_lossy(&pdf).contains("%PDF-1.4"));
        let midi = request_bytes(server.addr, &server.token, "GET", "/api/export/midi", "");
        assert!(String::from_utf8_lossy(&midi).contains("MThd"));
        let xml = request(server.addr, &server.token, "GET", "/api/export/musicxml", "");
        assert!(xml.contains("<score-partwise"));
    }

    #[test]
    fn sse_sends_initial_version() {
        let server = StudioServer::start(Config {
            port: 0,
            token: Some("sse-token-123456".to_string()),
            open_browser: false,
            quiet: true,
        })
        .unwrap();
        let mut stream = TcpStream::connect(server.addr).unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let request = format!(
            "GET /api/events HTTP/1.1\r\nHost: 127.0.0.1\r\nCookie: cadence_token={}\r\n\r\n",
            server.token
        );
        stream.write_all(request.as_bytes()).unwrap();
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        let mut body = String::new();
        loop {
            line.clear();
            if reader.read_line(&mut line).unwrap() == 0 {
                break;
            }
            if line == "\r\n" {
                break;
            }
        }
        let mut initial = String::new();
        reader.read_line(&mut initial).unwrap();
        assert!(initial.contains("event: update"));
        body.push_str(&initial);
    }

    #[test]
    fn safe_filenames_are_bounded() {
        assert_eq!(safe_filename("Cadence in G"), "Cadence_in_G");
        assert_eq!(safe_filename("../../etc/passwd"), "etcpasswd");
        assert_eq!(safe_filename(""), "cadence");
    }

    #[test]
    fn percent_decoding_is_bounded() {
        assert_eq!(percent_decode("a%20b+c"), "a b c");
        assert_eq!(percent_decode("%zz"), "%zz");
    }
}
