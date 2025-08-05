use std::{net::SocketAddr};

use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use crate::{config::config::{load_config, Server}, handlers::proxy::ProxyService};


type ServerBuilder = hyper::server::conn::http1::Builder;



pub async fn start() -> Result<(), Box<dyn std::error::Error>> {
   let config = load_config()?;

   let mut tasks_set  = tokio::task::JoinSet::new();
  
    for server in config.servers {
        for listen_addr in server.listen.clone() {
            tasks_set.spawn(create_server(server.clone(), listen_addr));
        }
    }

   tasks_set.join_all().await;
   Ok(())
}

async fn create_server(server: Server, listen_addr: SocketAddr) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let listener = TcpListener::bind(listen_addr).await?;
    println!("Proxy {} listening on http://{}", server.name, listen_addr);
    
    loop {
        let (stream, client_addr) = listener.accept().await?;
        let proxy_addr = stream.local_addr()?;
        let io = TokioIo::new(stream);

         tokio::task::spawn(async move {
            if let Err(err) = ServerBuilder::new()
                .preserve_header_case(true)
                .title_case_headers(true)
                .serve_connection(io, ProxyService { 
                    client_addr,
                    proxy_addr
                })
                .with_upgrades()
                .await
            {
                println!("Failed to serve connection: {:?}", err);
            }
        });
    }
}

