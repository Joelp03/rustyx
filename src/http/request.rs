use std::net::SocketAddr;

use http_body_util::{BodyExt, Empty, combinators::BoxBody};
use hyper::{ Request, body::Bytes};

pub struct ProxyRequest<T> {
    req: Request<T>,
    client_addr: SocketAddr,
}
pub fn empty() -> BoxBody<Bytes, hyper::Error> {
        Empty::<Bytes>::new()
            .map_err(|never| match never {})
            .boxed()
}

// proxy_set_header   X-Forwarded-For $proxy_add_x_forwarded_for;
// → Adds the original client IP address to the `X-Forwarded-For` header.
//   Useful for identifying the real IP of the user when the request is going through a proxy or load balancer.

// proxy_set_header   X-Forwarded-Host $host;
// → Sets the `X-Forwarded-Host` header to the original `Host` header sent by the client.
//   Helps the backend server know what domain was originally requested (e.g., example.com).

// proxy_set_header   X-Forwarded-Port $server_port;
// → Sets the `X-Forwarded-Port` header to the port on which the request was received by the proxy (e.g., 80 or 443).
//   Useful for backend services that need to know the original port used by the client.

// proxy_set_header   X-Forwarded-Proto $scheme;
// → Sets the `X-Forwarded-Proto` header to the original protocol (`http` or `https`) used by the client.
//   This tells the backend whether the original request was encrypted (HTTPS) or not.
// example of headers added by nginx
//   host: 'localhost',
//   'x-forwarded-for': '192.00.00.00',
//   'x-forwarded-port': '24892',
//   'x-forwarded-proto': 'http',


impl<T> ProxyRequest<T> {

    pub fn new(req: Request<T>, client_addr: SocketAddr) -> Self {
        let mut req = req;
        

        let client_ip =  client_addr.ip().to_string().parse().unwrap();
        req.headers_mut().insert("x-forwarded-for", client_ip);
        req.headers_mut().insert("x-forwarded-proto", "http".parse().unwrap());
        req.headers_mut().insert("x-forwarded-port", client_addr.port().to_string().parse().unwrap());


        Self { 
            req, 
            client_addr
         }
    }
}

// --- Sección de pruebas ---
#[cfg(test)]
mod tests {
    use super::*;
    use hyper::Uri;
    use hyper::header::HeaderValue;



    fn create_dummy_request() -> Request<BoxBody<Bytes, hyper::Error>> {
        return Request::builder()
            .uri(Uri::from_static("http://127.0.0.1:8080"))
            .body(empty())
            .unwrap();
    }

    #[test]
    fn new_proxy_request_adds_correct_headers() {
        // 1. Arrange: Set up the necessary data for the test
        let client_addr = SocketAddr::from(([127, 0, 0, 1], 8100));

        let dummy_request = create_dummy_request();

        // 2. Act: Call the method you want to test
        let proxy_req = ProxyRequest::new(dummy_request, client_addr);

        let headers = proxy_req.req.headers();

        // // Verify the values of the headers
        assert_eq!(headers["x-forwarded-for"], HeaderValue::from_static("127.0.0.1"));
        assert_eq!(headers["x-forwarded-proto"], HeaderValue::from_static("http"));
        assert_eq!(headers["x-forwarded-port"], HeaderValue::from_static("8100"));
    }
}

