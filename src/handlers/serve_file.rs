use std::path::Path;

use http_body_util::{ combinators::BoxBody};
use hyper::{
    body::{Bytes, Incoming}, header, Request, Response
};
use mime_guess::from_path;

use crate::http::body::{full, not_found};

pub async fn server_static(
    req: Request<Incoming>,
    dir: &str,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {

    let path = req.uri().path().trim_start_matches('/'); // remove leading "/"
    let mut full_path = format!("{}/{}", dir.trim_end_matches('/'), path);

    if full_path.contains("..") {
        return Ok(not_found());
    }

    let path_obj = Path::new(&full_path);
    if path_obj.is_dir() || path_obj.extension().is_none() {
        // default to index
        full_path = format!("{}/index.html", full_path.trim_end_matches('/'));
    }


    match tokio::fs::read(&full_path).await {
        Ok(content) => {
            let mime = from_path(&full_path).first_or_octet_stream();
            Ok(Response::builder()
            .header(header::CONTENT_TYPE, mime.as_ref())
            .body(full(content))
            .unwrap())
        } 

        Err(_) => Ok(not_found()),
    }
}

