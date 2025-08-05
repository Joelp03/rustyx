use hyper::{
    header::{self, HeaderValue},
    Response,
};


/// Wrapper for an HTTP response that allows header manipulation.
pub struct ProxyResponse<T> {
    pub response: Response<T>,
}

impl<T> ProxyResponse<T> {
    /// Constructs a new `ProxyResponse`.
    pub fn new(response: Response<T>) -> Self {
        Self { response }
    }

    /// Adds proxy-related headers (e.g. `Server`) to the response.
    pub fn with_forwarded_headers(mut self) -> Response<T> {
        self.response
            .headers_mut()
            .insert(header::SERVER, HeaderValue::from_static("rustyx"));
        self.response.headers_mut().remove("x-powered-by");
        self.response
    }

}


#[cfg(test)]
mod tests {
    use http_body_util::combinators::BoxBody;
    use hyper::body::Bytes;

    use crate::http::request::empty;

    use super::*;
    

    fn dummy_response() -> Response<BoxBody<Bytes, hyper::Error>> {
        Response::builder()
            .header("x-powered-by", "Express")
            .body(empty())
            
            .unwrap()
    }

    #[test]
    fn adds_server_header() {
        let res = dummy_response();

        let proxy_resp = ProxyResponse::new(res);
        let resp_with_headers = proxy_resp.with_forwarded_headers();

        let headers = resp_with_headers.headers();

        assert_eq!(headers[header::SERVER], HeaderValue::from_static("rustyx"));
    }
}
