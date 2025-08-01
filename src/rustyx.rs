use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use crate::config::config::load_config;

use crate::handlers::proxy::proxy;

type ServerBuilder = hyper::server::conn::http1::Builder;



pub async fn start() -> Result<(), Box<dyn std::error::Error>> {
    let config = load_config()?;

    let listener = TcpListener::bind(config.listener).await?;
    println!("Proxy Listening on http://{}", config.listener);

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





