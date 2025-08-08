use http_body_util::{combinators::BoxBody, BodyExt, Empty, Full};
use hyper::{body::Bytes, Response};

pub fn empty() -> BoxBody<Bytes, hyper::Error> {
        Empty::<Bytes>::new()
            .map_err(|never| match never {})
            .boxed()
}

pub fn full<T: Into<Bytes>>(chunk: T) -> BoxBody<Bytes, hyper::Error> {
    Full::new(chunk.into())
        .map_err(|never| match never {})
        .boxed()
}

pub fn not_found() -> Response<BoxBody<Bytes, hyper::Error>> {
    Response::builder()
        .status(hyper::StatusCode::NOT_FOUND)
        .body(full("Not Found"))
        .unwrap()
}
