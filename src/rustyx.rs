use std::{net::SocketAddr, sync::Arc};

use crate::{
    config::{config::{load_config, Server}},
    handlers::proxy::ProxyService,
};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;

type ServerBuilder = hyper::server::conn::http1::Builder;

pub struct Master {
    
}

impl Master {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config = load_config()?;

        let mut tasks = tokio::task::JoinSet::new();

        for server in config.servers {
            let config_server = Arc::new(server); 
            for listen_addr in config_server.listen.clone() {

                tasks.spawn(Self::create_task(config_server.clone(), listen_addr));
            }
        }

        tasks.join_all().await;
        Ok(())
    }

    async fn create_task(
        server: Arc<Server>,
        listen_addr: SocketAddr,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let listener = TcpListener::bind(listen_addr).await?;
        println!("Proxy {} listening on http://{}", server.name, listen_addr);

        loop {
            let (stream, client_addr) = listener.accept().await?;
            let proxy_addr = stream.local_addr()?;
            let io = TokioIo::new(stream);

            let config_server = server.clone();

            println!("accepted connection from {:?}", client_addr);

            tokio::task::spawn(async move {
                if let Err(err) = ServerBuilder::new()
                    .preserve_header_case(true)
                    .title_case_headers(true)
                    .serve_connection(
                        io,
                        ProxyService {
                            client_addr,
                            proxy_addr,
                            config_server 
                        },
                    )
                    .with_upgrades()
                    .await
                {
                    println!("Failed to serve connection: {:?}", err);
                }
            });
        }
    }
}
