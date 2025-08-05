use std::net::SocketAddr;

use http_body_util::{BodyExt, Empty, combinators::BoxBody};
use hyper::{ body::Bytes, header::{self, HeaderValue}, Request};


pub struct ProxyRequest<T> {
    pub request: Request<T>,
    pub client_addr: SocketAddr,
    pub proxy_addr: SocketAddr,
}
pub fn empty() -> BoxBody<Bytes, hyper::Error> {
        Empty::<Bytes>::new()
            .map_err(|never| match never {})
            .boxed()
}


impl<T> ProxyRequest<T> {

    pub fn new(req: Request<T>, client_addr: SocketAddr, proxy_addr: SocketAddr) -> Self {
        Self { request: req, client_addr, proxy_addr}
    }

    /// Set the standard headers for the request to indicate that it is a forwarded request from another host.
    ///
    /// Adds the following headers:
    /// - `x-forwarded-for`: The IP of the client making the request.
    /// - `x-forwarded-port`: The port on which the request was received.
    /// - `x-forwarded-proto`: The protocol of the request (`http` or `https`).
    /// - `host`: The original host that the request was sent to, if it can be determined.
    ///
    /// These headers are useful for identifying the original client and the host that the request was sent to,
    /// even if the request goes through a proxy or load balancer.
    ///
    pub fn forwarded_headers(mut self)->Request<T> {
        let ip = self.client_addr.ip().to_string();
        let port = self.client_addr.port().to_string();
        let by = self.proxy_addr.to_string();

        let host = self.request.uri().host().map(|h| h.to_string()).unwrap_or_else(|| self.proxy_addr.ip().to_string()); 

        self.request.headers_mut().insert("x-forwarded-for", HeaderValue::from_str(&ip).unwrap());
        self.request.headers_mut().insert("x-forwarded-port", HeaderValue::from_str(&port).unwrap());
        self.request.headers_mut().insert("x-forwarded-proto", HeaderValue::from_static("http"));

        let forwarded_value = format!("by={};for={}; proto=http; host={}",by, ip, host);
        self.request.headers_mut().insert(header::FORWARDED, HeaderValue::from_str(&forwarded_value).unwrap());
 
        self.request.headers_mut().insert(header::HOST,HeaderValue::from_str(&host).unwrap());
       

        self.request
    } 

   
}


// --- Sección de pruebas ---
#[cfg(test)]
mod tests {
    use super::*;
    use hyper::{header::HeaderValue, };



    fn create_dummy_request(proxy_uri: SocketAddr) -> Request<BoxBody<Bytes, hyper::Error>> {
        let uri =  proxy_uri.to_string();
        let uri_format = format!("http://{}", uri);

        return Request::builder()
            .uri(uri_format)
            .body(empty())
            .unwrap();
    }

    #[test]
    fn proxy_request_adds_correct_headers() {
        // 1. Arrange: Set up the necessary data for the test
        let client_addr = SocketAddr::from(([127, 0, 0, 1], 5000));
        let proxy_addr = SocketAddr::from(([127, 0, 0, 1], 8080));


        let dummy_request = create_dummy_request(proxy_addr);

        // 2. Act: Call the method you want to test
        let proxy_req = ProxyRequest::new(dummy_request, client_addr, proxy_addr);

        //let headers = proxy_req.request.headers();
        let forwarded_req = proxy_req.forwarded_headers();
        let headers = forwarded_req.headers();

        let by = proxy_addr.to_string();
        let _for = client_addr.ip().to_string();
        let host = proxy_addr.ip().to_string();

        let expect_forward = format!("by={};for={}; proto={}; host={}", by, _for, "http", host);
    

        // // Verify the values of the headers
        assert_eq!(headers["x-forwarded-for"], HeaderValue::from_static("127.0.0.1"));
        assert_eq!(headers["x-forwarded-proto"], HeaderValue::from_static("http"));
        assert_eq!(headers["x-forwarded-port"], HeaderValue::from_static("5000"));
        assert_eq!(headers[header::FORWARDED], HeaderValue::from_str(&expect_forward).unwrap())

    }
}

