//! Embedded Studio assets. Everything is compiled into the binary; the server
//! never reads the filesystem at runtime.

pub const INDEX_HTML: &str = include_str!("../../../assets/index.html");
pub const APP_CSS: &str = include_str!("../../../assets/app.css");
pub const APP_JS: &str = include_str!("../../../assets/app.js");
pub const FAVICON_SVG: &str = include_str!("../../../assets/favicon.svg");

#[derive(Debug, Clone, Copy)]
pub struct Asset {
    pub content_type: &'static str,
    pub body: &'static str,
}

pub fn find(path: &str) -> Option<Asset> {
    let asset = match path {
        "/" | "/index.html" => Asset {
            content_type: "text/html; charset=utf-8",
            body: INDEX_HTML,
        },
        "/assets/app.css" => Asset {
            content_type: "text/css; charset=utf-8",
            body: APP_CSS,
        },
        "/assets/app.js" => Asset {
            content_type: "text/javascript; charset=utf-8",
            body: APP_JS,
        },
        "/favicon.svg" => Asset {
            content_type: "image/svg+xml; charset=utf-8",
            body: FAVICON_SVG,
        },
        _ => return None,
    };
    Some(asset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_assets_are_embedded_and_original() {
        let html = find("/").unwrap();
        assert!(html.body.contains("Cadence"));
        assert!(!html.body.contains("http://"));
        let js = find("/assets/app.js").unwrap();
        assert!(js.body.contains("EventSource"));
        assert!(!js.body.contains("https://"));
        assert!(find("/nope").is_none());
    }
}
