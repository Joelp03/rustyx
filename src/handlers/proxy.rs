use std::{net::SocketAddr, sync::Arc};

use futures::future::BoxFuture;
use http_body_util::{BodyExt, Full, combinators::BoxBody};
use hyper::{
    Method, Request, Response, Uri,
    body::{Bytes, Incoming},
    service::Service,
    upgrade::Upgraded,
};
use hyper_util::rt::TokioIo;
use tokio::net::TcpStream;

use crate::{
    config::config, handlers::serve_file::server_static, http::{
        body::{empty, full}, request::ProxyRequest, response::ProxyResponse
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

impl Service<Request<Incoming>> for ProxyService {
    type Error = hyper::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;
    type Response = Response<BoxBody<Bytes, hyper::Error>>;

    fn call(&self, req: Request<Incoming>) -> Self::Future {
        /// Finds the most specific server location match for a given path.
        ///
        /// This function iterates over the `locations` configured for the server and finds the location
        /// whose path is the longest prefix of the given `path`. It returns the `SocketAddr` for the
        /// matched location, which indicates where to proxy requests for this path.
        ///
        /// # Arguments
        ///
        /// * `config_server` - A reference to the server configuration, which includes a list of locations.
        /// * `path` - The URI path to match against the server's locations.
        ///
        /// # Returns
        ///
        /// An `Option<SocketAddr>` containing the address to proxy requests to if a match is found,
        /// or `None` if no match is found for the `path`.

        // fn match_server(config_server: &config::Server, path: &str) -> Option<SocketAddr> {
        //     config_server
        //         .locations
        //         .iter()
        //         .filter(|loc| path.starts_with(&loc.path))
        //         .max_by_key(|loc| loc.path.len())
        //         .map(|loc| loc.proxy_pass)
        // }

        fn match_location<'a>(
            config_server: &'a config::Server,
            path: &str,
        ) -> Option<&'a config::Location> {
            config_server
                .locations
                .iter()
                .filter(|loc| path.starts_with(&loc.path))
                .max_by_key(|loc| loc.path.len())
        }

        let location = match match_location(&self.config_server, &req.uri().to_string()) {
            Some(loc) => loc,
            None => {
                let mut resp = Response::new(full("Not found"));
                *resp.status_mut() = hyper::StatusCode::NOT_FOUND;
                return Box::pin(async move { Ok(resp) });
            }
        };

        if let Some(root) = &location.root {
            let root_dir = root.clone();
            return Box::pin(async move {
                server_static(req, &root_dir).await
            });
        }

        if let Some(proxy_pass) = &location.proxy_pass {
            let request = ProxyRequest::new(req, self.client_addr, self.proxy_addr);
            return Box::pin(proxy(request, proxy_pass.clone()));
        }

        let mut resp = Response::new(full("Not found"));
        *resp.status_mut() = hyper::StatusCode::NOT_FOUND;
        return Box::pin(async move { Ok(resp) });

        //  let proxy_pass = match match_server(&self.config_server, &req.uri().to_string()) {
        //     Some(url) => url,
        //     None => {
        //         let mut resp = Response::new(full("Not found"));
        //         *resp.status_mut() = hyper::StatusCode::NOT_FOUND;
        //         return Box::pin(async move { Ok(resp) });
        //     }
        // };

        // let request = ProxyRequest::new(req, self.client_addr, self.proxy_addr);

        // Box::pin(proxy(request, proxy_pass.clone()))
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
