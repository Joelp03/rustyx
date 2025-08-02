use std::{ net::SocketAddr};

use futures::future::BoxFuture;
use http_body_util::{BodyExt, Full, combinators::BoxBody};
use hyper::{
    body::{Bytes, Incoming}, header::{HeaderValue, SERVER}, service::Service, upgrade::Upgraded, Method, Request, Response, Uri
};
use hyper_util::rt::TokioIo;
use tokio::net::TcpStream;

use crate::http::request::{empty, ProxyRequest};


type ClientBuilder = hyper::client::conn::http1::Builder;


pub struct ProxyService {
    pub client_addr: SocketAddr,
}

impl Service<Request<Incoming>> for ProxyService {
    type Error = hyper::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;
    type Response = Response<BoxBody<Bytes, hyper::Error>>;

    fn call(&self, req: Request<Incoming>) -> Self::Future {
        Box::pin(
            proxy(
            req,
            self.client_addr,
        )
    )
    }
}

pub async fn proxy(
    req: Request<Incoming>,
    client_addr: SocketAddr,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    //println!("req: {:?}", req);

    if Method::CONNECT == req.method() {
        if let Some(addr) = host_addr(req.uri()) {
            tokio::task::spawn(async move {
                match hyper::upgrade::on(req).await {
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
            eprintln!("CONNECT host is not socket addr: {:?}", req.uri());
            let resp = Response::new(full("CONNECT must be to a socket address"));

            //*resp.status_mut() = http::StatusCode::BAD_REQUEST;

            Ok(resp)
        }
    } else {
        println!("ELSE TO PROXY");
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

        let mut req = req;
        let from_port = client_addr.port().to_string();
        let from_ip = client_addr.ip().to_string();
        println!("client");
        println!("from port: {}, from ip: {}", from_port, from_ip);
        println!("from: {}", client_addr);
        req.headers_mut()
            .insert("X-Forwarded-For", HeaderValue::from_str(&from_ip).unwrap());
        req.headers_mut().insert(
            "X-Forwarded-Port",
            HeaderValue::from_str(&from_port).unwrap(),
        );
        req.headers_mut()
            .insert("X-Forwarded-Proto", HeaderValue::from_static("http"));
        let mut resp = sender.send_request(req).await?;

        resp.headers_mut()
            .insert(SERVER, HeaderValue::from_static("Rustyx"));

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

