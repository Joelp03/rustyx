#![deny(warnings)]

use std::net::SocketAddr;

use http_body_util::{combinators::BoxBody, BodyExt, Empty, Full};
use hyper::body::Bytes;
use hyper::header::{HeaderValue,  SERVER};
use hyper::service::service_fn;
use hyper::upgrade::Upgraded;
use hyper::{ Method, Request, Response, Uri};

use hyper_util::rt::TokioIo;
use tokio::net::{TcpListener, TcpStream};

type ClientBuilder = hyper::client::conn::http1::Builder;
type ServerBuilder = hyper::server::conn::http1::Builder;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    let listener = TcpListener::bind(addr).await?;
    println!("Proxy Listening on http://{}", addr);

    loop {
        let (stream, from) = listener.accept().await?;
        let io = TokioIo::new(stream);

        tokio::task::spawn(async move {
            if let Err(err) = ServerBuilder::new()
                .preserve_header_case(true)
                .title_case_headers(true)
                .serve_connection(io, service_fn(|req| proxy(req, from)))
                .with_upgrades()
                .await
            {
                println!("Failed to serve connection: {:?}", err);
            }
        });
    }
}

async fn proxy(
    req: Request<hyper::body::Incoming>,
    from: SocketAddr,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    //println!("req: {:?}", req);

    print!("URI : {:?}", req.uri());
    println!("from: {}", from);
    if Method::CONNECT == req.method() {
        // Received an HTTP request like:
        // ```
        // CONNECT www.domain.com:443 HTTP/1.1
        // Host: www.domain.com:443
        // Proxy-Connection: Keep-Alive
        // ```
        //
        // When HTTP method is CONNECT we should return an empty body
        // then we can eventually upgrade the connection and talk a new protocol.
        //
        // Note: only after client received an empty body with STATUS_OK can the
        // connection be upgraded, so we can't return a response inside
        // `on_upgrade` future.
        if let Some(addr) = host_addr(req.uri()) {
            println!("IF PROXY");
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
        println!("Request URI: {:?}", req.headers());
        let from_port = from.port().to_string();
        let from_ip= from.ip().to_string();
        println!("client");
        println!("from port: {}, from ip: {}", from_port, from_ip);
        println!("from: {}", from);
        req.headers_mut().insert("X-Forwarded-For", HeaderValue::from_str(&from_ip).unwrap());
        req.headers_mut().insert("X-Forwarded-Port", HeaderValue::from_str(&from_port).unwrap());
        req.headers_mut().insert("X-Forwarded-Proto", HeaderValue::from_static("http"));
        let mut resp = sender.send_request(req).await?;

        resp.headers_mut().insert(SERVER, HeaderValue::from_static("Rustyx"));

        Ok(resp.map(|b| b.boxed()))
    }
}

fn host_addr(uri: &Uri) -> Option<String> {
    uri.authority().map(|auth| auth.to_string())
}

fn empty() -> BoxBody<Bytes, hyper::Error> {
    Empty::<Bytes>::new()
        .map_err(|never| match never {})
        .boxed()
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

// fn set_header_proxy(){
//         // proxy_set_header   X-Forwarded-For $proxy_add_x_forwarded_for;
//         // proxy_set_header   X-Forwarded-Host $host;
//         // proxy_set_header   X-Forwarded-Port $server_port;
//         // proxy_set_header   X-Forwarded-Proto $scheme;
// }