use std::{net::SocketAddr, sync::Arc};

use futures::future::BoxFuture;
use http_body_util::{BodyExt, combinators::BoxBody};
use hyper::{
    body::{Bytes, Incoming}, service::Service, upgrade::Upgraded, Method, Request, Response, Uri
};
use hyper_util::rt::TokioIo;
use tokio::net::TcpStream;

use crate::{
    config::config, handlers::serve_file::serve_static, http::{
        body::{empty, full, not_found}, request::ProxyRequest, response::ProxyResponse
    }
};

type ClientBuilder = hyper::client::conn::http1::Builder;

/// The `ProxyService` struct in Rust represents a proxy service with client and proxy addresses, as
/// well as a configuration server.
///
/// Properties:
///
/// * `client_addr`: The `client_addr` property in the `ProxyService` struct represents the address of
/// the client connecting to the proxy service. It is of type `SocketAddr`, which typically contains
/// information about the IP address and port number of the client.
/// * `proxy_addr`: The `proxy_addr` property in the `ProxyService` struct represents the socket address
/// of the proxy service. It specifies the network address and port number where the proxy service is
/// running and can be accessed.
/// * `config_server`: The `config_server` property in the `ProxyService` struct seems to be of type
/// `config::Server`. This property likely holds configuration information related to the server
/// settings for the proxy service. It could include details such as server address, port,
/// authentication settings, timeouts, and other server-specific configurations

pub struct ProxyService {
    // client address
    pub client_addr: SocketAddr,

    // proxy socket
    pub proxy_addr: SocketAddr,

    pub config_server: Arc<config::Server>,
}

impl ProxyService {
    fn find_matching_location(&self, path: &str) -> Option<&config::Location> {
        self.config_server
            .locations
            .iter()
            .filter(|location| path.starts_with(&location.path))
            .max_by_key(|location| location.path.len())
    }

    fn handle_location_request(
        &self,
        req: Request<Incoming>,
        location: &config::Location,
    ) -> BoxFuture<'static, Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error>> {
        if let Some(root_dir) = &location.root {
            return self.handle_static_files(req, root_dir.clone());
        }

        if let Some(proxy_target) = &location.proxy_pass {
            return self.handle_proxy_request(req, proxy_target.clone());
        }

        Box::pin(async { Ok(not_found()) })
    }

    fn handle_static_files(
        &self,
        req: Request<Incoming>,
        root_dir: String,
    ) -> BoxFuture<'static, Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error>> {
        Box::pin(async move { 
            serve_static(req, &root_dir).await 
        })
    }

    fn handle_proxy_request(
        &self,
        req: Request<Incoming>,
        proxy_target: SocketAddr,
    ) -> BoxFuture<'static, Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error>> {
        let proxy_request = ProxyRequest::new(req, self.client_addr, self.proxy_addr);
        Box::pin(proxy(proxy_request, proxy_target))
    }

}


impl Service<Request<Incoming>> for ProxyService {
    type Error = hyper::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;
    type Response = Response<BoxBody<Bytes, hyper::Error>>;

    fn call(&self, req: Request<Incoming>) -> Self::Future {
        let request_path = req.uri().path();
        
        match self.find_matching_location(request_path) {
            Some(location) => self.handle_location_request(req, location),
            None => Box::pin(async { Ok(not_found()) }),
        }
    }
}




pub async fn proxy(
    req: ProxyRequest<Incoming>,
    src: SocketAddr,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
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
            let mut resp = Response::new(full("CONNECT must be to a socket address"));
            //*resp.status_mut() = http::StatusCode::BAD_REQUEST;
            *resp.status_mut() = hyper::StatusCode::BAD_REQUEST;
            Ok(resp)
        }
    } else {
        let stream = TcpStream::connect(src).await.unwrap();

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
        let resp = sender.send_request(req.forwarded_headers()).await?;

        Ok(ProxyResponse::new(resp)
            .with_forwarded_headers()
            .map(|b| b.boxed()))
    }
}

fn host_addr(uri: &Uri) -> Option<String> {
    uri.authority().map(|auth| auth.to_string())
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
