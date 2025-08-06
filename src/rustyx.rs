use std::{net::SocketAddr, sync::Arc, time::Duration};

use crate::config::config::{Server, load_config};
use crate::handlers::proxy::ProxyService;

use hyper_util::{rt::TokioIo, server::graceful::GracefulShutdown};
use tokio::net::TcpListener;
use tokio::signal;
use tokio::task::JoinSet;

type ServerBuilder = hyper::server::conn::http1::Builder;

pub struct Master;

impl Master {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config = load_config()?;

        let mut tasks = JoinSet::new();
 
        for server in config.servers {
            let config_server = Arc::new(server);
            for listen_addr in config_server.listen.clone() {
                tasks.spawn(Self::create_server(config_server.clone(), listen_addr));
            }
        }

        tasks.join_all().await;
        Ok(())
    }

    async fn shutdown_signal() {
        signal::ctrl_c()
            .await
            .expect("failed to install CTRL+C handler");
        eprintln!("Shutdown signal received");
    }

    async fn create_server(
        server: Arc<Server>,
        listen_addr: SocketAddr,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let listener = TcpListener::bind(listen_addr).await?;
        println!("Proxy {} listening on http://{}", server.name, listen_addr);

        let graceful = GracefulShutdown::new();
        let mut shutdown_signal = Box::pin(Self::shutdown_signal());

        loop {
            tokio::select! {
                Ok((stream, client_addr)) = listener.accept() => {
                    let proxy_addr = stream.local_addr()?;
                    let io = TokioIo::new(stream);

                    println!("accepted connection from {:?}", client_addr);

                    let config_server = server.clone();
                    let graceful_conn = graceful.watch(
                        ServerBuilder::new()
                            .preserve_header_case(true)
                            .title_case_headers(true)
                            .serve_connection(
                                io,
                                ProxyService {
                                    client_addr,
                                    proxy_addr,
                                    config_server,
                                },
                            )
                    );

                    tokio::spawn(async move {
                        if let Err(err) = graceful_conn.await {
                            eprintln!("Failed to serve connection: {:?}", err);
                        }
                    });
                },

                _ = &mut shutdown_signal => {
                    drop(listener);
                    eprintln!("Gracefully shutting down {}", server.name);
                    break;
                }
            }
        }

        const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(10);
        // waiting connections
        tokio::select! {
            _ = graceful.shutdown() => {
                eprintln!("All connections on {} closed", listen_addr);
            },
            
            _ = tokio::time::sleep(SHUTDOWN_TIMEOUT) => {
                eprintln!("Graceful shutdown timeout on {}", listen_addr);
            }
        }

        Ok(())
    }
}
