use std::path::{Path, PathBuf};

use http_body_util::combinators::BoxBody;
use hyper::{
    Request, Response, StatusCode,
    body::{Bytes, Incoming},
    header,
};
use mime_guess::from_path;

use crate::http::body::{full, not_found};


pub async fn serve_static(
    req: Request<Incoming>,
    base_dir: &str,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    let requested_path = extract_path_from_request(&req);

    let sanitized_path = match sanitize_path(&requested_path) {
        Some(path) => path,
        None => return Ok(not_found()),
    };

    let file_path = resolve_file_path(base_dir, &sanitized_path);
    serve_file(&file_path).await
}

fn extract_path_from_request(req: &Request<Incoming>) -> String {
    req.uri().path().trim_start_matches('/').to_string()
}

fn sanitize_path(path: &str) -> Option<String> {
    // Remove leading slash and decode URL encoding
    let path = path.trim_start_matches('/');

    // Check for directory traversal attempts
    if path.contains("..") || path.contains('\0') {
        return None;
    }

    // Additional security checks
    if path.starts_with('/') || path.contains("//") {
        return None;
    }

    Some(path.to_string())
}

fn resolve_file_path(base_dir: &str, requested_path: &str) -> PathBuf {
    let base_dir = base_dir.trim_end_matches('/');
    let mut full_path = PathBuf::from(format!("{}/{}", base_dir, requested_path));

    if full_path.is_dir() || full_path.extension().is_none() {
        full_path = full_path.join("index.html");
    }

    full_path
}

async fn serve_file(
    file_path: &Path,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    match tokio::fs::read(file_path).await {
        Ok(content) => Ok(create_file_response(file_path, content)),
        Err(_) => Ok(not_found()),
    }
}

fn create_file_response(
    file_path: &Path,
    content: Vec<u8>,
) -> Response<BoxBody<Bytes, hyper::Error>> {
    let mime_type = from_path(file_path).first_or_octet_stream();

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime_type.as_ref())
        .body(full(content))
        .expect("Failed to build response")
}

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_sanitize_path() {
        // Valid paths should pass through
        assert_eq!(
            sanitize_path("normal/path/file.txt"),
            Some("normal/path/file.txt".to_string())
        );
        assert_eq!(sanitize_path("index.html"), Some("index.html".to_string()));
        assert_eq!(
            sanitize_path("assets/css/style.css"),
            Some("assets/css/style.css".to_string())
        );
        assert_eq!(sanitize_path(""), Some("".to_string()));

        // Directory traversal attempts should be rejected
        assert_eq!(sanitize_path("../etc/passwd"), None);
        assert_eq!(sanitize_path("folder/../secret"), None);
        assert_eq!(sanitize_path("../../root"), None);
        assert_eq!(sanitize_path("normal/../../../etc/passwd"), None);
        assert_eq!(sanitize_path(".."), None);
        assert_eq!(sanitize_path("../"), None);

        // Null byte injection should be rejected
        assert_eq!(sanitize_path("file\0.txt"), None);
        assert_eq!(sanitize_path("normal/path\0/file.txt"), None);
        assert_eq!(sanitize_path("\0"), None);

        // Double slashes should be rejected
        assert_eq!(sanitize_path("path//file.txt"), None);
        assert_eq!(sanitize_path("normal//path//file.txt"), None);

        // Paths starting with slash after trimming should be rejected
        // (This case is actually handled by trim_start_matches('/') first)
        assert_eq!(
            sanitize_path("/absolute/path"),
            Some("absolute/path".to_string())
        );

        // Edge cases with special characters
        assert_eq!(
            sanitize_path("file with spaces.txt"),
            Some("file with spaces.txt".to_string())
        );
        assert_eq!(
            sanitize_path("file-with-dashes_and_underscores.txt"),
            Some("file-with-dashes_and_underscores.txt".to_string())
        );
        assert_eq!(
            sanitize_path("file.with.dots.txt"),
            Some("file.with.dots.txt".to_string())
        );

        // Mixed attack attempts
        assert_eq!(sanitize_path("../folder//file\0.txt"), None);
        assert_eq!(sanitize_path("normal/../path//file.txt"), None);
    }
}
