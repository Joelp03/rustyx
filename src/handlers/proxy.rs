use std::{net::SocketAddr};

use futures::future::BoxFuture;
use http_body_util::{BodyExt, Full, combinators::BoxBody};
use hyper::{
    Method, Request, Response, Uri,
    body::{Bytes, Incoming},
    header::{self, HeaderValue},
    service::Service,
    upgrade::Upgraded,
};
use hyper_util::rt::TokioIo;
use tokio::net::TcpStream;

use crate::{
    http::request::{ProxyRequest, empty},
};

type ClientBuilder = hyper::client::conn::http1::Builder;

pub struct ProxyService {
    pub client_addr: SocketAddr,
    pub proxy_addr: SocketAddr,
}

impl Service<Request<Incoming>> for ProxyService {
    type Error = hyper::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;
    type Response = Response<BoxBody<Bytes, hyper::Error>>;

    fn call(&self, req: Request<Incoming>) -> Self::Future {
        let request = ProxyRequest::new(req, self.client_addr, self.proxy_addr);
        Box::pin(proxy(request))
    }
}

pub async fn proxy(
    req: ProxyRequest<Incoming>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    //println!("req: {:?}", req);

    if Method::CONNECT == req.request.method() {
        if let Some(addr) = host_addr(req.request.uri()) {
            tokio::task::spawn(async move {
                match hyper::upgrade::on(req.request).await {
                    Ok(upgraded) => {
                        if let Err(e) = tunnel(upgraded, addr).await {
                            eprintln!("server io error: {}", e);
                        };
                    }
                    Err(e) => eprintln!("upgrade error: {}", e),
                }
            });

            Ok(Response::new(empty()))
        } else {
            eprintln!("CONNECT host is not socket addr: {:?}", req.request.uri());
            let resp = Response::new(full("CONNECT must be to a socket address"));
            //*resp.status_mut() = http::StatusCode::BAD_REQUEST;
            Ok(resp)
        }
    } else {
        // Parse our URL...
        let url = "http://localhost:9000/hello".parse::<hyper::Uri>().unwrap();

        // Get the host and the port
        let host = url.host().expect("uri has no host");
        let port = url.port_u16().unwrap_or(80);

        let stream = TcpStream::connect((host, port)).await.unwrap();
        let io = TokioIo::new(stream);

        let (mut sender, conn) = ClientBuilder::new()
            .preserve_header_case(true)
            .title_case_headers(true)
            .handshake(io)
            .await?;
        tokio::task::spawn(async move {
            if let Err(err) = conn.await {
                println!("Connection failed: {:?}", err);
            }
        });

        // send request to server by proxy
        let mut resp = sender.send_request(req.forwarded_headers()).await?;

        resp.headers_mut()
            .insert(header::SERVER, HeaderValue::from_static("Rustyx"));

        Ok(resp.map(|b| b.boxed()))
    }
}

fn host_addr(uri: &Uri) -> Option<String> {
    uri.authority().map(|auth| auth.to_string())
}

fn full<T: Into<Bytes>>(chunk: T) -> BoxBody<Bytes, hyper::Error> {
    Full::new(chunk.into())
        .map_err(|never| match never {})
        .boxed()
}

// Create a TCP connection to host:port, build a tunnel between the connection and
// the upgraded connection
async fn tunnel(upgraded: Upgraded, addr: String) -> std::io::Result<()> {
    // Connect to remote server
    let mut server = TcpStream::connect(addr).await?;
    let mut upgraded = TokioIo::new(upgraded);

    // Proxying data
    let (from_client, from_server) =
        tokio::io::copy_bidirectional(&mut upgraded, &mut server).await?;

    // Print message when done
    println!(
        "client wrote {} bytes and received {} bytes",
        from_client, from_server
    );

    Ok(())
}
